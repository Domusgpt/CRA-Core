/**
 * CRA Runtime
 *
 * The authoritative resolver and policy engine.
 * All context resolution and action permissions flow through here.
 */

import type {
  CARPRequest,
  CARPResolution,
  CARPDecision,
  CARPActionRequest,
  CARPExecutionResult,
  CARPError,
  ContextBlock,
  ActionPermission,
  ActionDenial,
  Evidence,
  PolicyApplication,
} from '@cra/protocol';
import {
  createResolution,
  createError,
  validateRequest,
  isResolutionExpired,
  generateId,
  getTimestamp,
  CARP_VERSION,
} from '@cra/protocol';
import { TRACECollector } from '@cra/trace';
import { AtlasLoader, LoadedAtlas } from '@cra/atlas';

// =============================================================================
// Types
// =============================================================================

export interface RuntimeConfig {
  /** Session ID */
  session_id?: string;
  /** Trace output directory */
  trace_dir?: string;
  /** Enable file-based trace output */
  trace_to_file?: boolean;
  /** Atlas search paths */
  atlas_paths?: string[];
  /** Default TTL for resolutions (seconds) */
  default_ttl_seconds?: number;
  /** Maximum context tokens */
  max_context_tokens?: number;
  /** Maximum actions per resolution */
  max_actions_per_resolution?: number;
}

export interface RuntimeStats {
  resolutions_total: number;
  resolutions_allowed: number;
  resolutions_denied: number;
  actions_executed: number;
  actions_failed: number;
  atlases_loaded: number;
  session_start: string;
  uptime_ms: number;
}

// =============================================================================
// CRA Runtime
// =============================================================================

export class CRARuntime {
  private readonly config: Required<RuntimeConfig>;
  private readonly atlasLoader: AtlasLoader;
  private readonly trace: TRACECollector;
  private readonly loadedAtlases: Map<string, LoadedAtlas> = new Map();
  private readonly resolutionCache: Map<string, CARPResolution> = new Map();
  private readonly stats: RuntimeStats;
  private readonly sessionStart: Date;

  constructor(config: RuntimeConfig = {}) {
    this.sessionStart = new Date();
    this.config = {
      session_id: config.session_id ?? generateId(),
      trace_dir: config.trace_dir ?? './traces',
      trace_to_file: config.trace_to_file ?? false,
      atlas_paths: config.atlas_paths ?? ['./atlases'],
      default_ttl_seconds: config.default_ttl_seconds ?? 300,
      max_context_tokens: config.max_context_tokens ?? 8192,
      max_actions_per_resolution: config.max_actions_per_resolution ?? 50,
    };

    this.atlasLoader = new AtlasLoader({ validate: true });

    this.trace = new TRACECollector({
      session_id: this.config.session_id,
      output_dir: this.config.trace_dir,
      file_output: this.config.trace_to_file,
      source: {
        component: 'cra.runtime',
        version: '0.1.0',
      },
    });

    this.stats = {
      resolutions_total: 0,
      resolutions_allowed: 0,
      resolutions_denied: 0,
      actions_executed: 0,
      actions_failed: 0,
      atlases_loaded: 0,
      session_start: this.sessionStart.toISOString(),
      uptime_ms: 0,
    };

    // Record session start event
    this.trace.record('session.started', {
      session_id: this.config.session_id,
      config: this.config,
    });
  }

  /**
   * Get the session ID
   */
  getSessionId(): string {
    return this.config.session_id;
  }

  /**
   * Get the TRACE collector
   */
  getTrace(): TRACECollector {
    return this.trace;
  }

  /**
   * Load an atlas
   */
  async loadAtlas(source: string): Promise<LoadedAtlas> {
    const span = this.trace.startSpan('atlas.load', {
      attributes: { source },
    });

    try {
      this.trace.record('atlas.load.started', { source }, { span_id: span.span_id });

      const atlas = await this.atlasLoader.load(source);
      this.loadedAtlases.set(atlas.ref, atlas);
      this.stats.atlases_loaded++;

      this.trace.record(
        'atlas.load.completed',
        {
          atlas_ref: atlas.ref,
          domains: atlas.manifest.domains.map(d => d.id),
          context_packs: atlas.manifest.context_packs.length,
          actions: atlas.manifest.actions.length,
          policies: atlas.manifest.policies.length,
        },
        { span_id: span.span_id }
      );

      this.trace.endSpan(span.span_id, 'ok');
      return atlas;
    } catch (error) {
      this.trace.record(
        'atlas.load.failed',
        { source, error: String(error) },
        { span_id: span.span_id, severity: 'error' }
      );
      this.trace.endSpan(span.span_id, 'error', { message: String(error) });
      throw error;
    }
  }

  /**
   * Resolve a CARP request
   */
  async resolve(request: CARPRequest): Promise<CARPResolution | CARPError> {
    const span = this.trace.startSpan('carp.resolve', {
      attributes: {
        request_id: request.request_id,
        operation: request.operation,
        goal: request.task?.goal,
      },
    });

    try {
      // Record request received
      this.trace.record(
        'carp.request.received',
        {
          request_id: request.request_id,
          operation: request.operation,
          goal: request.task?.goal,
          goal_hash: request.task?.goal_hash,
          risk_tier: request.task?.risk_tier,
        },
        { span_id: span.span_id }
      );

      // Validate request
      const validation = validateRequest(request);
      if (!validation.valid) {
        const error = createError(
          request.request_id,
          'INVALID_REQUEST',
          `Request validation failed: ${validation.errors.join(', ')}`,
          { details: { errors: validation.errors } }
        );
        this.trace.endSpan(span.span_id, 'error', { message: 'Validation failed' });
        return error;
      }

      this.trace.record('carp.request.validated', { request_id: request.request_id }, { span_id: span.span_id });

      // Check resolution cache
      const cacheKey = this.getResolutionCacheKey(request);
      const cached = this.resolutionCache.get(cacheKey);
      if (cached && !isResolutionExpired(cached)) {
        this.trace.record(
          'carp.resolution.cache_hit',
          { resolution_id: cached.resolution_id },
          { span_id: span.span_id }
        );
        this.trace.endSpan(span.span_id, 'ok');
        return cached;
      }

      // Start resolution
      this.trace.record('carp.resolution.started', { request_id: request.request_id }, { span_id: span.span_id });
      this.stats.resolutions_total++;

      // Get applicable atlases
      const atlases = this.getApplicableAtlases(request);
      if (atlases.length === 0) {
        const error = createError(
          request.request_id,
          'ATLAS_NOT_FOUND',
          'No applicable atlases found',
          { trace_id: this.trace.getTraceId() }
        );
        this.stats.resolutions_denied++;
        this.trace.endSpan(span.span_id, 'error', { message: 'No atlases' });
        return error;
      }

      for (const atlas of atlases) {
        this.trace.record(
          'carp.atlas.loaded',
          { atlas_ref: atlas.ref },
          { span_id: span.span_id }
        );
      }

      // Assemble context blocks
      const contextBlocks = this.assembleContext(request, atlases, span.span_id);

      // Resolve actions
      const { allowed, denied } = this.resolveActions(request, atlases, span.span_id);

      // Evaluate policies
      const policyResults = this.evaluatePolicies(request, atlases, span.span_id);

      // Gather evidence
      const evidence = this.gatherEvidence(atlases, span.span_id);

      // Determine decision
      const decision = this.determineDecision(request, policyResults, allowed, denied);

      // Update stats
      if (decision.type === 'allow' || decision.type === 'allow_with_constraints') {
        this.stats.resolutions_allowed++;
      } else if (decision.type === 'deny') {
        this.stats.resolutions_denied++;
      }

      // Create resolution
      const resolution = createResolution(request, decision, {
        context_blocks: contextBlocks,
        allowed_actions: allowed,
        denied_actions: denied,
        policies_applied: policyResults.applications,
        evidence,
        ttl_seconds: this.config.default_ttl_seconds,
        trace_id: this.trace.getTraceId(),
        span_id: span.span_id,
      });

      // Update telemetry link
      resolution.telemetry_link.events_emitted = this.trace.getEvents().length;

      // Cache resolution
      this.resolutionCache.set(cacheKey, resolution);

      // Record completion
      this.trace.record(
        'carp.resolution.completed',
        {
          request_id: request.request_id,
          resolution_id: resolution.resolution_id,
          decision_type: decision.type,
          context_blocks_count: contextBlocks.length,
          allowed_actions_count: allowed.length,
          denied_actions_count: denied.length,
          policies_applied_count: policyResults.applications.length,
        },
        { span_id: span.span_id }
      );

      this.trace.endSpan(span.span_id, 'ok');
      return resolution;
    } catch (error) {
      this.trace.record(
        'error.internal',
        { request_id: request.request_id, error: String(error) },
        { span_id: span.span_id, severity: 'error' }
      );
      this.trace.endSpan(span.span_id, 'error', { message: String(error) });

      return createError(
        request.request_id,
        'INTERNAL_ERROR',
        `Resolution failed: ${error}`,
        { trace_id: this.trace.getTraceId() }
      );
    }
  }

  /**
   * Execute a CARP action
   */
  async execute(request: CARPActionRequest): Promise<CARPExecutionResult | CARPError> {
    const span = this.trace.startSpan('carp.execute', {
      attributes: {
        action_id: request.action.action_id,
        action_type: request.action.action_type,
      },
    });

    try {
      this.trace.record(
        'carp.action.requested',
        {
          action_id: request.action.action_id,
          action_type: request.action.action_type,
          resolution_id: request.action.resolution_id,
        },
        { span_id: span.span_id }
      );

      // Validate resolution exists and is valid
      const resolution = this.findResolution(request.action.resolution_id);
      if (!resolution) {
        const error = createError(
          request.request_id,
          'RESOLUTION_NOT_FOUND',
          `Resolution ${request.action.resolution_id} not found`,
          { trace_id: this.trace.getTraceId() }
        );
        this.trace.endSpan(span.span_id, 'error', { message: 'Resolution not found' });
        return error;
      }

      if (isResolutionExpired(resolution)) {
        const error = createError(
          request.request_id,
          'RESOLUTION_EXPIRED',
          `Resolution ${request.action.resolution_id} has expired`,
          { trace_id: this.trace.getTraceId() }
        );
        this.trace.endSpan(span.span_id, 'error', { message: 'Resolution expired' });
        return error;
      }

      // Check action is permitted
      const permission = resolution.allowed_actions.find(
        a => a.action_id === request.action.action_id
      );

      if (!permission) {
        this.trace.record(
          'carp.action.denied',
          { action_id: request.action.action_id, reason: 'Not in allowed_actions' },
          { span_id: span.span_id, severity: 'warn' }
        );
        this.stats.actions_failed++;

        const error = createError(
          request.request_id,
          'ACTION_NOT_PERMITTED',
          `Action ${request.action.action_id} is not permitted`,
          { trace_id: this.trace.getTraceId() }
        );
        this.trace.endSpan(span.span_id, 'error', { message: 'Action not permitted' });
        return error;
      }

      // Check approval requirement
      if (permission.requires_approval) {
        this.trace.record(
          'carp.action.approval.pending',
          { action_id: request.action.action_id },
          { span_id: span.span_id }
        );

        // For now, auto-approve (in production, this would wait for approval)
        this.trace.record(
          'carp.action.approved',
          { action_id: request.action.action_id, approver: 'auto' },
          { span_id: span.span_id }
        );
      }

      // Execute action
      this.trace.record(
        'carp.action.started',
        {
          action_id: request.action.action_id,
          action_type: request.action.action_type,
          parameters: request.action.parameters,
        },
        { span_id: span.span_id }
      );

      const startTime = Date.now();

      // Simulate action execution (in production, this would call the actual action)
      const result = await this.executeAction(request, permission);

      const duration = Date.now() - startTime;
      this.stats.actions_executed++;

      this.trace.record(
        'carp.action.completed',
        {
          action_id: request.action.action_id,
          status: result.status,
          duration_ms: duration,
        },
        { span_id: span.span_id }
      );

      this.trace.endSpan(span.span_id, 'ok');
      return result;
    } catch (error) {
      this.stats.actions_failed++;
      this.trace.record(
        'carp.action.failed',
        { action_id: request.action.action_id, error: String(error) },
        { span_id: span.span_id, severity: 'error' }
      );
      this.trace.endSpan(span.span_id, 'error', { message: String(error) });

      return createError(
        request.request_id,
        'EXECUTION_FAILED',
        `Action execution failed: ${error}`,
        { trace_id: this.trace.getTraceId() }
      );
    }
  }

  /**
   * Get runtime statistics
   */
  getStats(): RuntimeStats {
    return {
      ...this.stats,
      uptime_ms: Date.now() - this.sessionStart.getTime(),
    };
  }

  /**
   * Shutdown the runtime
   */
  async shutdown(): Promise<void> {
    this.trace.record('session.ended', {
      session_id: this.config.session_id,
      stats: this.getStats(),
    });

    await this.trace.close();
  }

  // =========================================================================
  // Private Methods
  // =========================================================================

  private getResolutionCacheKey(request: CARPRequest): string {
    const parts = [
      request.task?.goal_hash ?? '',
      request.requester.agent_id,
      JSON.stringify(request.scope ?? {}),
    ];
    return parts.join(':');
  }

  private getApplicableAtlases(request: CARPRequest): LoadedAtlas[] {
    let atlases = Array.from(this.loadedAtlases.values());

    // Filter by scope if specified
    if (request.scope?.atlases?.length) {
      atlases = atlases.filter(a =>
        request.scope!.atlases!.some(ref => a.ref.startsWith(ref))
      );
    }

    // Filter by domains if hints provided
    if (request.task?.context_hints?.length) {
      atlases = atlases.filter(a =>
        a.manifest.domains.some(d =>
          request.task!.context_hints!.includes(d.id)
        )
      );
    }

    return atlases;
  }

  private assembleContext(
    request: CARPRequest,
    atlases: LoadedAtlas[],
    spanId: string
  ): ContextBlock[] {
    const allBlocks: ContextBlock[] = [];
    const maxTokens = request.scope?.max_context_tokens ?? this.config.max_context_tokens;
    let totalTokens = 0;

    this.trace.record(
      'carp.context.selected',
      { atlases: atlases.map(a => a.ref), max_tokens: maxTokens },
      { span_id: spanId }
    );

    for (const atlas of atlases) {
      const blocks = this.atlasLoader.getContextBlocks(atlas, {
        domains: request.task?.context_hints,
        max_tokens: maxTokens - totalTokens,
      });

      for (const block of blocks) {
        if (totalTokens + block.token_count <= maxTokens) {
          allBlocks.push(block);
          totalTokens += block.token_count;
        }
      }
    }

    this.trace.record(
      'carp.context.assembled',
      {
        block_count: allBlocks.length,
        total_tokens: totalTokens,
      },
      { span_id: spanId }
    );

    return allBlocks;
  }

  private resolveActions(
    request: CARPRequest,
    atlases: LoadedAtlas[],
    spanId: string
  ): { allowed: ActionPermission[]; denied: ActionDenial[] } {
    const allowed: ActionPermission[] = [];
    const denied: ActionDenial[] = [];
    const maxActions = request.scope?.max_actions ?? this.config.max_actions_per_resolution;

    for (const atlas of atlases) {
      const permissions = this.atlasLoader.getActionPermissions(atlas, {
        domains: request.task?.context_hints,
        risk_tier: request.task?.risk_tier,
        action_types: request.scope?.actions,
      });

      for (const permission of permissions) {
        if (allowed.length < maxActions) {
          allowed.push(permission);
        }
      }
    }

    this.trace.record(
      'carp.actions.resolved',
      {
        allowed_count: allowed.length,
        denied_count: denied.length,
      },
      { span_id: spanId }
    );

    return { allowed, denied };
  }

  private evaluatePolicies(
    request: CARPRequest,
    atlases: LoadedAtlas[],
    spanId: string
  ): {
    allowed: boolean;
    requires_approval: boolean;
    applications: PolicyApplication[];
  } {
    const applications: PolicyApplication[] = [];
    let allowed = true;
    let requiresApproval = false;

    this.trace.record('carp.policy.evaluation.started', {}, { span_id: spanId });

    for (const atlas of atlases) {
      const result = this.atlasLoader.evaluatePolicies(atlas, {
        risk_tier: request.task?.risk_tier,
        domain: request.task?.context_hints?.[0],
      });

      if (!result.allowed) {
        allowed = false;
      }
      if (result.requires_approval) {
        requiresApproval = true;
      }

      for (const match of result.matched_rules) {
        this.trace.record(
          'carp.policy.rule.matched',
          { policy_id: match.policy_id, rule_id: match.rule_id, effect: match.effect },
          { span_id: spanId }
        );

        // Find or create application record
        let app = applications.find(a => a.policy_id === match.policy_id);
        if (!app) {
          const policy = atlas.manifest.policies.find(p => p.id === match.policy_id);
          app = {
            policy_id: match.policy_id,
            policy_name: policy?.name ?? match.policy_id,
            policy_version: policy?.version ?? '1.0',
            atlas_ref: atlas.ref,
            rules_evaluated: 0,
            rules_matched: 0,
            effects: [],
            evaluation_time_ms: 0,
          };
          applications.push(app);
        }
        app.rules_matched++;
        app.effects.push({
          rule_id: match.rule_id,
          effect: match.effect as PolicyApplication['effects'][0]['effect'],
          target: 'request',
        });
      }
    }

    this.trace.record(
      'carp.policy.evaluation.completed',
      {
        allowed,
        requires_approval: requiresApproval,
        policies_evaluated: applications.length,
      },
      { span_id: spanId }
    );

    return { allowed, requires_approval: requiresApproval, applications };
  }

  private gatherEvidence(atlases: LoadedAtlas[], spanId: string): Evidence[] {
    const evidence: Evidence[] = [];

    for (const atlas of atlases) {
      // Add atlas itself as evidence
      evidence.push({
        evidence_id: generateId(),
        type: 'documentation',
        source: `atlas:${atlas.ref}`,
        atlas_ref: atlas.ref,
        content_hash: '',
        created_at: atlas.loaded_at,
        verified: true,
        verification_method: 'atlas_validation',
      });
    }

    this.trace.record(
      'carp.evidence.gathered',
      { evidence_count: evidence.length },
      { span_id: spanId }
    );

    return evidence;
  }

  private determineDecision(
    request: CARPRequest,
    policyResults: { allowed: boolean; requires_approval: boolean },
    allowed: ActionPermission[],
    _denied: ActionDenial[]
  ): CARPDecision {
    if (!policyResults.allowed) {
      return {
        type: 'deny',
        reason: 'Policy evaluation denied the request',
        policy_refs: [],
      };
    }

    if (policyResults.requires_approval) {
      return {
        type: 'requires_approval',
        approvers: [{ id: 'admin', type: 'role', name: 'Administrator' }],
        approval_timeout_seconds: 300,
      };
    }

    if (allowed.length === 0 && request.task?.context_hints?.length) {
      return {
        type: 'insufficient_context',
        missing_domains: request.task.context_hints,
        suggestion: 'Load additional atlases for the requested domains',
      };
    }

    // Check for high-risk tier requiring constraints
    if (request.task?.risk_tier === 'high' || request.task?.risk_tier === 'critical') {
      return {
        type: 'allow_with_constraints',
        constraints: [
          {
            id: generateId(),
            type: 'audit_required',
            description: 'All actions must be logged for audit',
            params: {},
            enforcement: 'hard',
          },
        ],
      };
    }

    return { type: 'allow' };
  }

  private findResolution(resolutionId: string): CARPResolution | undefined {
    for (const resolution of this.resolutionCache.values()) {
      if (resolution.resolution_id === resolutionId) {
        return resolution;
      }
    }
    return undefined;
  }

  private async executeAction(
    request: CARPActionRequest,
    permission: ActionPermission
  ): Promise<CARPExecutionResult> {
    // Simulate action execution
    // In production, this would dispatch to the appropriate handler
    return {
      carp_version: CARP_VERSION,
      request_id: request.request_id,
      execution_id: generateId(),
      timestamp: getTimestamp(),
      status: 'success',
      result: {
        output: { message: `Action ${permission.action_type} executed successfully` },
        output_hash: generateId(),
        output_type: 'application/json',
      },
      metrics: {
        duration_ms: Math.floor(Math.random() * 100) + 10,
      },
      telemetry_link: {
        trace_id: this.trace.getTraceId(),
        span_id: generateId(),
        events_emitted: 0,
      },
    };
  }
}
