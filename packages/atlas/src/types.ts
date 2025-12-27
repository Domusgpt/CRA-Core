/**
 * Atlas Format Types
 *
 * Atlases are creator-published packages of domain expertise,
 * policies, adapters, tests, and prompt/tool artifacts.
 */

import type { RiskTier } from '@cra/protocol';

// =============================================================================
// Atlas Manifest
// =============================================================================

export interface AtlasManifest {
  /** Atlas format version */
  atlas_version: '0.1';

  /** Metadata */
  metadata: AtlasMetadata;

  /** Domain definitions */
  domains: Domain[];

  /** Context packs */
  context_packs: ContextPack[];

  /** Policies */
  policies: Policy[];

  /** Action definitions */
  actions: ActionDefinition[];

  /** Platform adapters */
  adapters: AdapterConfig[];

  /** Test suites */
  tests: TestSuite[];

  /** Dependencies on other atlases */
  dependencies?: AtlasDependency[];
}

// =============================================================================
// Metadata
// =============================================================================

export interface AtlasMetadata {
  /** Unique identifier (reverse-dns style, e.g., "com.example.github-ops") */
  id: string;
  /** Human-readable name */
  name: string;
  /** SemVer version */
  version: string;
  /** Description */
  description: string;
  /** Authors */
  authors: Author[];
  /** License information */
  license: LicenseInfo;
  /** Keywords for discovery */
  keywords: string[];
  /** Homepage URL */
  homepage?: string;
  /** Repository URL */
  repository?: string;
}

export interface Author {
  name: string;
  email?: string;
  url?: string;
}

export interface LicenseInfo {
  /** License type */
  type: 'free' | 'paid' | 'subscription' | 'usage' | 'enterprise';
  /** SPDX identifier for open source */
  spdx?: string;
  /** Terms URL */
  terms_url?: string;
  /** Price information */
  price?: PriceInfo;
}

export interface PriceInfo {
  model: 'one_time' | 'monthly' | 'yearly' | 'per_resolution' | 'per_action';
  amount_cents: number;
  currency: string;
}

// =============================================================================
// Domains
// =============================================================================

export interface Domain {
  /** Domain ID */
  id: string;
  /** Human-readable name */
  name: string;
  /** Description */
  description: string;
  /** Default risk tier */
  risk_tier: RiskTier;
}

// =============================================================================
// Context Packs
// =============================================================================

export interface ContextPack {
  /** Pack ID */
  id: string;
  /** Associated domain */
  domain: string;
  /** Path to content file (relative to atlas root) */
  source: string;
  /** Content format */
  format?: 'markdown' | 'json' | 'yaml' | 'text';
  /** TTL in seconds */
  ttl_seconds: number;
  /** Tags for filtering */
  tags: string[];
  /** Evidence sources */
  evidence_sources: string[];
  /** Priority (higher = more important) */
  priority?: number;
}

// =============================================================================
// Policies
// =============================================================================

export interface Policy {
  /** Policy ID */
  id: string;
  /** Human-readable name */
  name: string;
  /** Version */
  version: string;
  /** Description */
  description?: string;
  /** Rules */
  rules: PolicyRule[];
}

export interface PolicyRule {
  /** Rule ID */
  id: string;
  /** Description */
  description: string;
  /** Condition for this rule */
  condition: PolicyCondition;
  /** Effect when condition matches */
  effect: 'allow' | 'deny' | 'require_approval' | 'redact';
  /** Priority (higher = evaluated first) */
  priority: number;
  /** Optional message */
  message?: string;
}

export interface PolicyCondition {
  /** Condition type */
  type: 'risk_tier' | 'action_type' | 'domain' | 'requester' | 'time' | 'rate' | 'custom' | 'all' | 'any';
  /** Comparison operator */
  operator?: 'eq' | 'neq' | 'in' | 'not_in' | 'gt' | 'lt' | 'matches';
  /** Comparison value */
  value?: unknown;
  /** Sub-conditions for all/any */
  conditions?: PolicyCondition[];
}

// =============================================================================
// Actions
// =============================================================================

export interface ActionDefinition {
  /** Action ID */
  id: string;
  /** Action type (e.g., "api.github.create_issue") */
  type: string;
  /** Human-readable name */
  name: string;
  /** Description */
  description: string;
  /** Associated domain */
  domain: string;
  /** JSON Schema for parameters */
  schema: Record<string, unknown>;
  /** Usage examples */
  examples: ActionExample[];
  /** Risk tier */
  risk_tier: RiskTier;
}

export interface ActionExample {
  description: string;
  parameters: Record<string, unknown>;
  expected_outcome?: string;
}

// =============================================================================
// Adapters
// =============================================================================

export type Platform = 'openai' | 'claude' | 'google_adk' | 'mcp';

export interface AdapterConfig {
  /** Target platform */
  platform: Platform;
  /** Path to platform-specific config file */
  config_file: string;
  /** Tool mappings */
  tool_mappings?: ToolMapping[];
}

export interface ToolMapping {
  /** Action ID in atlas */
  action_id: string;
  /** Tool name on platform */
  platform_tool_name: string;
  /** Parameter transformations */
  parameter_transforms?: Record<string, string>;
}

// =============================================================================
// Tests
// =============================================================================

export type TestType = 'unit' | 'integration' | 'conformance' | 'golden_trace';

export interface TestSuite {
  /** Suite ID */
  id: string;
  /** Suite name */
  name: string;
  /** Test type */
  type: TestType;
  /** Test file paths */
  test_files: string[];
}

// =============================================================================
// Dependencies
// =============================================================================

export interface AtlasDependency {
  /** Atlas ID */
  atlas_id: string;
  /** SemVer range */
  version_range: string;
}

// =============================================================================
// Loaded Atlas
// =============================================================================

export interface LoadedAtlas {
  /** Manifest */
  manifest: AtlasManifest;
  /** Atlas reference string (id@version) */
  ref: string;
  /** Base directory */
  base_dir: string;
  /** Loaded context content (pack_id -> content) */
  context_content: Map<string, string>;
  /** Loaded adapter configs (platform -> config) */
  adapter_configs: Map<Platform, unknown>;
  /** Load timestamp */
  loaded_at: string;
}

// =============================================================================
// Validation
// =============================================================================

export interface ValidationError {
  path: string;
  message: string;
  severity: 'error' | 'warning';
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationError[];
}
