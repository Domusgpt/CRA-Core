/**
 * CARP Protocol utilities
 */

import { createHash } from 'crypto';
import { v7 as uuidv7 } from 'uuid';
import type {
  CARPRequest,
  CARPResolution,
  CARPVersion,
  RequesterInfo,
  TaskSpec,
  CARPDecision,
  ContextBlock,
  ActionPermission,
  CARPError,
  CARPErrorCode,
} from './types.js';

/** Current CARP protocol version */
export const CARP_VERSION: CARPVersion = '1.0';

/**
 * Generate a UUIDv7 (time-ordered)
 */
export function generateId(): string {
  return uuidv7();
}

/**
 * Get current ISO 8601 timestamp with timezone
 */
export function getTimestamp(): string {
  return new Date().toISOString();
}

/**
 * Compute SHA-256 hash of content
 */
export function computeHash(content: string): string {
  return createHash('sha256').update(content).digest('hex');
}

/**
 * Compute goal hash for caching
 */
export function computeGoalHash(goal: string): string {
  return computeHash(goal.trim().toLowerCase());
}

/**
 * Create a new CARP request
 */
export function createRequest(
  operation: 'resolve' | 'execute' | 'validate',
  requester: RequesterInfo,
  options: {
    task?: TaskSpec;
    action?: CARPRequest['action'];
    scope?: CARPRequest['scope'];
    telemetry?: CARPRequest['telemetry'];
  } = {}
): CARPRequest {
  const request: CARPRequest = {
    carp_version: CARP_VERSION,
    request_id: generateId(),
    timestamp: getTimestamp(),
    operation,
    requester,
    ...options,
  };

  // Add goal hash if task provided
  if (request.task?.goal && !request.task.goal_hash) {
    request.task.goal_hash = computeGoalHash(request.task.goal);
  }

  return request;
}

/**
 * Create a CARP resolution
 */
export function createResolution(
  request: CARPRequest,
  decision: CARPDecision,
  options: {
    context_blocks?: ContextBlock[];
    allowed_actions?: ActionPermission[];
    denied_actions?: CARPResolution['denied_actions'];
    policies_applied?: CARPResolution['policies_applied'];
    evidence?: CARPResolution['evidence'];
    ttl_seconds?: number;
    trace_id?: string;
    span_id?: string;
  } = {}
): CARPResolution {
  const now = new Date();
  const ttlSeconds = options.ttl_seconds ?? 300; // 5 minutes default

  return {
    carp_version: CARP_VERSION,
    request_id: request.request_id,
    resolution_id: generateId(),
    timestamp: getTimestamp(),
    decision,
    context_blocks: options.context_blocks ?? [],
    allowed_actions: options.allowed_actions ?? [],
    denied_actions: options.denied_actions ?? [],
    policies_applied: options.policies_applied ?? [],
    evidence: options.evidence ?? [],
    ttl: {
      context_expires_at: new Date(now.getTime() + ttlSeconds * 1000).toISOString(),
      resolution_expires_at: new Date(now.getTime() + ttlSeconds * 1000).toISOString(),
      refresh_hint_seconds: Math.floor(ttlSeconds * 0.8),
    },
    telemetry_link: {
      trace_id: options.trace_id ?? generateId(),
      span_id: options.span_id ?? generateId(),
      events_emitted: 0,
    },
  };
}

/**
 * Create a CARP error response
 */
export function createError(
  requestId: string,
  code: CARPErrorCode,
  message: string,
  options: {
    details?: Record<string, unknown>;
    trace_id?: string;
    retriable?: boolean;
    retry_after_seconds?: number;
  } = {}
): CARPError {
  return {
    carp_version: CARP_VERSION,
    request_id: requestId,
    timestamp: getTimestamp(),
    error: {
      code,
      message,
      details: options.details,
      trace_id: options.trace_id,
    },
    retry: options.retriable !== undefined
      ? {
          retriable: options.retriable,
          retry_after_seconds: options.retry_after_seconds,
          max_retries: options.retriable ? 3 : 0,
        }
      : undefined,
  };
}

/**
 * Create a context block
 */
export function createContextBlock(
  atlasRef: string,
  packRef: string,
  domain: string,
  content: string,
  options: {
    content_type?: ContextBlock['content_type'];
    ttl_seconds?: number;
    tags?: string[];
    priority?: number;
    evidence_refs?: string[];
  } = {}
): ContextBlock {
  return {
    block_id: generateId(),
    content_hash: computeHash(content),
    atlas_ref: atlasRef,
    pack_ref: packRef,
    domain,
    content_type: options.content_type ?? 'markdown',
    content,
    token_count: estimateTokens(content),
    ttl_seconds: options.ttl_seconds ?? 300,
    tags: options.tags ?? [],
    priority: options.priority ?? 0,
    evidence_refs: options.evidence_refs ?? [],
  };
}

/**
 * Estimate token count for text (rough approximation)
 */
export function estimateTokens(text: string): number {
  // Rough estimate: ~4 characters per token
  return Math.ceil(text.length / 4);
}

/**
 * Validate a CARP request structure
 */
export function validateRequest(request: unknown): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if (!request || typeof request !== 'object') {
    return { valid: false, errors: ['Request must be an object'] };
  }

  const req = request as Record<string, unknown>;

  // Check version
  if (req.carp_version !== CARP_VERSION) {
    errors.push(`Invalid carp_version: expected ${CARP_VERSION}`);
  }

  // Check required fields
  if (!req.request_id || typeof req.request_id !== 'string') {
    errors.push('Missing or invalid request_id');
  }

  if (!req.timestamp || typeof req.timestamp !== 'string') {
    errors.push('Missing or invalid timestamp');
  }

  if (!['resolve', 'execute', 'validate'].includes(req.operation as string)) {
    errors.push('Invalid operation');
  }

  if (!req.requester || typeof req.requester !== 'object') {
    errors.push('Missing or invalid requester');
  } else {
    const requester = req.requester as Record<string, unknown>;
    if (!requester.agent_id) errors.push('Missing requester.agent_id');
    if (!requester.session_id) errors.push('Missing requester.session_id');
  }

  // Operation-specific validation
  if (req.operation === 'resolve' && !req.task) {
    errors.push('Resolve operation requires task');
  }

  if ((req.operation === 'execute' || req.operation === 'validate') && !req.action) {
    errors.push(`${req.operation} operation requires action`);
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Check if a resolution is expired
 */
export function isResolutionExpired(resolution: CARPResolution): boolean {
  return new Date(resolution.ttl.resolution_expires_at) < new Date();
}

/**
 * Check if context is expired
 */
export function isContextExpired(resolution: CARPResolution): boolean {
  return new Date(resolution.ttl.context_expires_at) < new Date();
}

/**
 * Get decision type string
 */
export function getDecisionType(decision: CARPDecision): string {
  return decision.type;
}

/**
 * Check if decision allows action
 */
export function isAllowDecision(decision: CARPDecision): boolean {
  return decision.type === 'allow' || decision.type === 'allow_with_constraints';
}
