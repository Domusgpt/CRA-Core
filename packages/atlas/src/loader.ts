/**
 * Atlas Loader
 *
 * Loads, validates, and manages Atlas packages.
 */

import * as fs from 'fs';
import * as path from 'path';
import { parse as parseYaml } from 'yaml';
import type {
  AtlasManifest,
  LoadedAtlas,
  ValidationResult,
  ValidationError,
  PolicyCondition,
  Platform,
} from './types.js';
import type { ContextBlock, ActionPermission, RiskTier } from '@cra/protocol';
import { computeHash, generateId, getTimestamp, estimateTokens } from '@cra/protocol';

// =============================================================================
// Atlas Loader
// =============================================================================

export interface AtlasLoaderOptions {
  /** Cache TTL in ms */
  cache_ttl_ms?: number;
  /** Validate on load */
  validate?: boolean;
}

export class AtlasLoader {
  private cache: Map<string, { atlas: LoadedAtlas; expires_at: number }> = new Map();
  private cacheTtl: number;
  private validateOnLoad: boolean;

  constructor(options: AtlasLoaderOptions = {}) {
    this.cacheTtl = options.cache_ttl_ms ?? 10 * 60 * 1000; // 10 minutes
    this.validateOnLoad = options.validate ?? true;
  }

  /**
   * Load an atlas from a directory or manifest
   */
  async load(source: string | AtlasManifest): Promise<LoadedAtlas> {
    let manifest: AtlasManifest;
    let baseDir: string;

    if (typeof source === 'string') {
      // Load from directory
      baseDir = source;
      manifest = await this.loadManifest(source);
    } else {
      // Use provided manifest
      manifest = source;
      baseDir = '.';
    }

    const ref = `${manifest.metadata.id}@${manifest.metadata.version}`;

    // Check cache
    const cached = this.cache.get(ref);
    if (cached && cached.expires_at > Date.now()) {
      return cached.atlas;
    }

    // Validate if enabled
    if (this.validateOnLoad) {
      const validation = this.validate(manifest);
      if (!validation.valid) {
        throw new Error(
          `Atlas validation failed: ${validation.errors.map(e => e.message).join(', ')}`
        );
      }
    }

    // Load context content
    const contextContent = new Map<string, string>();
    for (const pack of manifest.context_packs) {
      const contentPath = path.join(baseDir, pack.source);
      try {
        const content = await fs.promises.readFile(contentPath, 'utf-8');
        contextContent.set(pack.id, content);
      } catch (error) {
        console.warn(`Failed to load context pack ${pack.id}: ${error}`);
      }
    }

    // Load adapter configs
    const adapterConfigs = new Map<Platform, unknown>();
    for (const adapter of manifest.adapters) {
      const configPath = path.join(baseDir, adapter.config_file);
      try {
        const configContent = await fs.promises.readFile(configPath, 'utf-8');
        const config = configPath.endsWith('.yaml') || configPath.endsWith('.yml')
          ? parseYaml(configContent)
          : JSON.parse(configContent);
        adapterConfigs.set(adapter.platform, config);
      } catch (error) {
        console.warn(`Failed to load adapter config for ${adapter.platform}: ${error}`);
      }
    }

    const atlas: LoadedAtlas = {
      manifest,
      ref,
      base_dir: baseDir,
      context_content: contextContent,
      adapter_configs: adapterConfigs,
      loaded_at: getTimestamp(),
    };

    // Cache
    this.cache.set(ref, {
      atlas,
      expires_at: Date.now() + this.cacheTtl,
    });

    return atlas;
  }

  /**
   * Load manifest from directory
   */
  private async loadManifest(dir: string): Promise<AtlasManifest> {
    // Try atlas.json first, then atlas.yaml
    const jsonPath = path.join(dir, 'atlas.json');
    const yamlPath = path.join(dir, 'atlas.yaml');

    try {
      const content = await fs.promises.readFile(jsonPath, 'utf-8');
      return JSON.parse(content);
    } catch {
      try {
        const content = await fs.promises.readFile(yamlPath, 'utf-8');
        return parseYaml(content) as AtlasManifest;
      } catch {
        throw new Error(`No atlas.json or atlas.yaml found in ${dir}`);
      }
    }
  }

  /**
   * Validate an atlas manifest
   */
  validate(manifest: AtlasManifest): ValidationResult {
    const errors: ValidationError[] = [];
    const warnings: ValidationError[] = [];

    // Check version
    if (manifest.atlas_version !== '0.1') {
      errors.push({
        path: 'atlas_version',
        message: `Unsupported atlas version: ${manifest.atlas_version}`,
        severity: 'error',
      });
    }

    // Check metadata
    if (!manifest.metadata?.id) {
      errors.push({ path: 'metadata.id', message: 'Missing atlas ID', severity: 'error' });
    }
    if (!manifest.metadata?.version) {
      errors.push({ path: 'metadata.version', message: 'Missing version', severity: 'error' });
    }
    if (!manifest.metadata?.name) {
      errors.push({ path: 'metadata.name', message: 'Missing name', severity: 'error' });
    }

    // Check domains
    const domainIds = new Set<string>();
    for (const domain of manifest.domains ?? []) {
      if (domainIds.has(domain.id)) {
        errors.push({
          path: `domains.${domain.id}`,
          message: `Duplicate domain ID: ${domain.id}`,
          severity: 'error',
        });
      }
      domainIds.add(domain.id);
    }

    // Check context packs reference valid domains
    for (const pack of manifest.context_packs ?? []) {
      if (!domainIds.has(pack.domain)) {
        errors.push({
          path: `context_packs.${pack.id}.domain`,
          message: `Unknown domain: ${pack.domain}`,
          severity: 'error',
        });
      }
    }

    // Check actions reference valid domains
    for (const action of manifest.actions ?? []) {
      if (!domainIds.has(action.domain)) {
        errors.push({
          path: `actions.${action.id}.domain`,
          message: `Unknown domain: ${action.domain}`,
          severity: 'error',
        });
      }
    }

    // Check policies
    for (const policy of manifest.policies ?? []) {
      for (const rule of policy.rules ?? []) {
        if (!this.validateCondition(rule.condition)) {
          warnings.push({
            path: `policies.${policy.id}.rules.${rule.id}`,
            message: 'Policy condition may be invalid',
            severity: 'warning',
          });
        }
      }
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings,
    };
  }

  /**
   * Validate a policy condition
   */
  private validateCondition(condition: PolicyCondition): boolean {
    if (!condition.type) return false;

    if (condition.type === 'all' || condition.type === 'any') {
      if (!condition.conditions?.length) return false;
      return condition.conditions.every(c => this.validateCondition(c));
    }

    return true;
  }

  /**
   * Get context blocks for a query
   */
  getContextBlocks(
    atlas: LoadedAtlas,
    options: {
      domains?: string[];
      tags?: string[];
      max_tokens?: number;
    } = {}
  ): ContextBlock[] {
    const blocks: ContextBlock[] = [];
    let totalTokens = 0;
    const maxTokens = options.max_tokens ?? 8192;

    // Filter and sort context packs
    let packs = [...atlas.manifest.context_packs];

    if (options.domains?.length) {
      packs = packs.filter(p => options.domains!.includes(p.domain));
    }

    if (options.tags?.length) {
      packs = packs.filter(p => p.tags.some(t => options.tags!.includes(t)));
    }

    // Sort by priority (descending)
    packs.sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));

    // Build blocks
    for (const pack of packs) {
      const content = atlas.context_content.get(pack.id);
      if (!content) continue;

      const tokens = estimateTokens(content);
      if (totalTokens + tokens > maxTokens) break;

      blocks.push({
        block_id: generateId(),
        content_hash: computeHash(content),
        atlas_ref: atlas.ref,
        pack_ref: pack.id,
        domain: pack.domain,
        content_type: (pack.format ?? 'markdown') as ContextBlock['content_type'],
        content,
        token_count: tokens,
        ttl_seconds: pack.ttl_seconds,
        tags: pack.tags,
        priority: pack.priority ?? 0,
        evidence_refs: pack.evidence_sources.map(() => generateId()),
      });

      totalTokens += tokens;
    }

    return blocks;
  }

  /**
   * Get action permissions for a domain/risk tier
   */
  getActionPermissions(
    atlas: LoadedAtlas,
    options: {
      domains?: string[];
      risk_tier?: RiskTier;
      action_types?: string[];
    } = {}
  ): ActionPermission[] {
    let actions = [...atlas.manifest.actions];

    // Filter by domain
    if (options.domains?.length) {
      actions = actions.filter(a => options.domains!.includes(a.domain));
    }

    // Filter by risk tier
    if (options.risk_tier) {
      const riskOrder: RiskTier[] = ['low', 'medium', 'high', 'critical'];
      const maxRiskIndex = riskOrder.indexOf(options.risk_tier);
      actions = actions.filter(a => riskOrder.indexOf(a.risk_tier) <= maxRiskIndex);
    }

    // Filter by action types
    if (options.action_types?.length) {
      actions = actions.filter(a => options.action_types!.includes(a.type));
    }

    // Convert to permissions
    const now = new Date();
    const validUntil = new Date(now.getTime() + 5 * 60 * 1000).toISOString(); // 5 minutes

    return actions.map(action => ({
      action_id: generateId(),
      action_type: action.type,
      name: action.name,
      description: action.description,
      schema: action.schema,
      examples: action.examples,
      constraints: [],
      requires_approval: action.risk_tier === 'high' || action.risk_tier === 'critical',
      risk_tier: action.risk_tier,
      atlas_ref: atlas.ref,
      evidence_refs: [],
      valid_until: validUntil,
    }));
  }

  /**
   * Evaluate policies for a request
   */
  evaluatePolicies(
    atlas: LoadedAtlas,
    context: {
      risk_tier?: RiskTier;
      action_type?: string;
      domain?: string;
      requester?: Record<string, unknown>;
    }
  ): {
    allowed: boolean;
    requires_approval: boolean;
    matched_rules: { policy_id: string; rule_id: string; effect: string }[];
    redactions: string[];
  } {
    const matchedRules: { policy_id: string; rule_id: string; effect: string }[] = [];
    let allowed = true;
    let requiresApproval = false;
    const redactions: string[] = [];

    for (const policy of atlas.manifest.policies) {
      // Sort rules by priority (descending)
      const sortedRules = [...policy.rules].sort((a, b) => b.priority - a.priority);

      for (const rule of sortedRules) {
        if (this.evaluateCondition(rule.condition, context)) {
          matchedRules.push({
            policy_id: policy.id,
            rule_id: rule.id,
            effect: rule.effect,
          });

          switch (rule.effect) {
            case 'deny':
              allowed = false;
              break;
            case 'require_approval':
              requiresApproval = true;
              break;
            case 'redact':
              redactions.push(rule.message ?? 'Redacted by policy');
              break;
          }
        }
      }
    }

    return { allowed, requires_approval: requiresApproval, matched_rules: matchedRules, redactions };
  }

  /**
   * Evaluate a policy condition
   */
  private evaluateCondition(
    condition: PolicyCondition,
    context: Record<string, unknown>
  ): boolean {
    switch (condition.type) {
      case 'all':
        return condition.conditions?.every(c => this.evaluateCondition(c, context)) ?? false;

      case 'any':
        return condition.conditions?.some(c => this.evaluateCondition(c, context)) ?? false;

      case 'risk_tier':
        return this.compareValue(context.risk_tier, condition.operator ?? 'eq', condition.value);

      case 'action_type':
        return this.compareValue(context.action_type, condition.operator ?? 'eq', condition.value);

      case 'domain':
        return this.compareValue(context.domain, condition.operator ?? 'eq', condition.value);

      case 'requester':
        // Check requester attributes
        const requester = context.requester as Record<string, unknown> | undefined;
        if (!requester) return false;
        const [key] = Object.keys(condition.value as Record<string, unknown>);
        return this.compareValue(
          requester[key],
          condition.operator ?? 'eq',
          (condition.value as Record<string, unknown>)[key]
        );

      default:
        return false;
    }
  }

  /**
   * Compare values with operator
   */
  private compareValue(actual: unknown, operator: string, expected: unknown): boolean {
    switch (operator) {
      case 'eq':
        return actual === expected;
      case 'neq':
        return actual !== expected;
      case 'in':
        return Array.isArray(expected) && expected.includes(actual);
      case 'not_in':
        return Array.isArray(expected) && !expected.includes(actual);
      case 'gt':
        return typeof actual === 'number' && typeof expected === 'number' && actual > expected;
      case 'lt':
        return typeof actual === 'number' && typeof expected === 'number' && actual < expected;
      case 'matches':
        return typeof actual === 'string' && typeof expected === 'string' &&
          new RegExp(expected).test(actual);
      default:
        return false;
    }
  }

  /**
   * Clear cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Remove expired cache entries
   */
  pruneCache(): number {
    const now = Date.now();
    let removed = 0;
    for (const [key, value] of this.cache.entries()) {
      if (value.expires_at <= now) {
        this.cache.delete(key);
        removed++;
      }
    }
    return removed;
  }
}
