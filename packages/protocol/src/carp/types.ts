/**
 * CARP/1.0 - Context & Action Resolution Protocol
 *
 * Core type definitions for the protocol contract between
 * acting agents (Requesters) and context authorities (Resolvers).
 */

// =============================================================================
// Core Types
// =============================================================================

export type RiskTier = 'low' | 'medium' | 'high' | 'critical';

export type CARPVersion = '1.0';

export type CARPOperation = 'resolve' | 'execute' | 'validate';

// =============================================================================
// CARP Request
// =============================================================================

export interface RequesterInfo {
  /** Unique agent identifier */
  agent_id: string;
  /** Agent type (e.g., "claude", "gpt-4", "custom") */
  agent_type?: string;
  /** Session correlation ID */
  session_id: string;
  /** Bearer token or API key */
  auth_token?: string;
  /** Additional metadata */
  metadata?: Record<string, string>;
}

export interface TaskConstraint {
  type: string;
  value: unknown;
}

export interface TaskSpec {
  /** Natural language goal description */
  goal: string;
  /** SHA-256 hash of goal for caching */
  goal_hash?: string;
  /** Declared risk level */
  risk_tier?: RiskTier;
  /** Requested context domains/topics */
  context_hints?: string[];
  /** Task constraints */
  constraints?: TaskConstraint[];
  /** For subtask chaining */
  parent_task_id?: string;
}

export interface ActionSpec {
  /** Action ID from prior resolution */
  action_id: string;
  /** Action type identifier */
  action_type: string;
  /** Action parameters */
  parameters: Record<string, unknown>;
  /** Reference to authorizing resolution */
  resolution_id: string;
}

export interface RequestScope {
  /** Limit to specific atlases */
  atlases?: string[];
  /** Limit to specific domains */
  domains?: string[];
  /** Limit action types */
  actions?: string[];
  /** Context size limit */
  max_context_tokens?: number;
  /** Action count limit */
  max_actions?: number;
}

export interface TelemetryOptions {
  /** Existing trace to join */
  trace_id?: string;
  /** Parent span */
  parent_span_id?: string;
  /** Sampling rate (0.0 - 1.0) */
  sampling_rate?: number;
}

export interface CARPRequest {
  /** Protocol version */
  carp_version: CARPVersion;
  /** Client-generated request ID (UUIDv7) */
  request_id: string;
  /** ISO 8601 timestamp with timezone */
  timestamp: string;
  /** Operation type */
  operation: CARPOperation;
  /** Requester identity */
  requester: RequesterInfo;
  /** Task specification (for resolve) */
  task?: TaskSpec;
  /** Action specification (for execute/validate) */
  action?: ActionSpec;
  /** Scoping constraints */
  scope?: RequestScope;
  /** Telemetry options */
  telemetry?: TelemetryOptions;
}

// =============================================================================
// CARP Resolution
// =============================================================================

export interface Constraint {
  id: string;
  type: ConstraintType;
  description: string;
  params: Record<string, unknown>;
  enforcement: 'hard' | 'soft';
}

export type ConstraintType =
  | 'rate_limit'
  | 'time_window'
  | 'parameter_restriction'
  | 'output_filter'
  | 'approval_required'
  | 'audit_required'
  | 'sandbox'
  | 'custom';

export interface Approver {
  id: string;
  type: 'user' | 'role' | 'system';
  name?: string;
}

// Decision types
export interface AllowDecision {
  type: 'allow';
}

export interface AllowWithConstraintsDecision {
  type: 'allow_with_constraints';
  constraints: Constraint[];
}

export interface DenyDecision {
  type: 'deny';
  reason: string;
  policy_refs: string[];
  remediation?: string;
}

export interface RequiresApprovalDecision {
  type: 'requires_approval';
  approvers: Approver[];
  approval_timeout_seconds: number;
  approval_url?: string;
}

export interface InsufficientContextDecision {
  type: 'insufficient_context';
  missing_domains: string[];
  missing_atlases?: string[];
  suggestion?: string;
}

export interface PartialDecision {
  type: 'partial';
  reason: string;
  allowed_subset: string[];
  denied_subset: string[];
}

export type CARPDecision =
  | AllowDecision
  | AllowWithConstraintsDecision
  | DenyDecision
  | RequiresApprovalDecision
  | InsufficientContextDecision
  | PartialDecision;

// Context blocks
export interface Redaction {
  original_hash: string;
  redacted_fields: string[];
  reason: string;
  policy_ref: string;
}

export interface ContextBlock {
  /** Unique within resolution */
  block_id: string;
  /** SHA-256 of content */
  content_hash: string;
  /** Atlas ID + version */
  atlas_ref: string;
  /** Context pack ID */
  pack_ref: string;
  /** Domain classification */
  domain: string;
  /** Content type */
  content_type: 'markdown' | 'json' | 'yaml' | 'text';
  /** The actual context */
  content: string;
  /** Estimated tokens */
  token_count: number;
  /** How long this context is valid */
  ttl_seconds: number;
  /** Classification level */
  classification?: string;
  /** Applied redactions */
  redactions?: Redaction[];
  /** IDs of supporting evidence */
  evidence_refs: string[];
  /** Tags for categorization */
  tags: string[];
  /** Priority (higher = more important) */
  priority: number;
}

// Action permissions
export interface ParameterConstraint {
  parameter: string;
  constraint_type: 'enum' | 'range' | 'pattern' | 'max_length' | 'required' | 'forbidden';
  value: unknown;
  message: string;
}

export interface ActionExample {
  description: string;
  parameters: Record<string, unknown>;
  expected_outcome?: string;
}

export interface RateLimit {
  requests: number;
  window_seconds: number;
  scope: 'action' | 'session' | 'agent' | 'global';
  current_usage?: number;
  resets_at?: string;
}

export interface ActionPermission {
  /** Unique within resolution */
  action_id: string;
  /** Type identifier (e.g., "api.github.create_issue") */
  action_type: string;
  /** Human-readable name */
  name: string;
  /** What this action does */
  description: string;
  /** JSON Schema for parameters */
  schema: Record<string, unknown>;
  /** Usage examples */
  examples?: ActionExample[];
  /** General constraints */
  constraints: Constraint[];
  /** Parameter-specific constraints */
  parameter_constraints?: ParameterConstraint[];
  /** Approval requirement */
  requires_approval: boolean;
  /** Approval type if required */
  approval_type?: 'sync' | 'async';
  /** Risk level */
  risk_tier: RiskTier;
  /** Rate limiting */
  rate_limit?: RateLimit;
  /** Source atlas */
  atlas_ref: string;
  /** Supporting evidence */
  evidence_refs: string[];
  /** Validity period */
  valid_until: string;
}

export interface ActionDenial {
  action_type: string;
  reason: string;
  policy_refs: string[];
  permanent: boolean;
  remediation?: string;
  alternative?: string;
}

// Evidence
export type EvidenceType =
  | 'documentation'
  | 'api_spec'
  | 'example'
  | 'test_result'
  | 'policy'
  | 'changelog'
  | 'external'
  | 'user_provided';

export interface Evidence {
  evidence_id: string;
  type: EvidenceType;
  source: string;
  source_url?: string;
  atlas_ref?: string;
  content_hash: string;
  content_preview?: string;
  full_content_ref?: string;
  created_at: string;
  verified: boolean;
  verification_method?: string;
}

// Policy application
export interface PolicyEffect {
  rule_id: string;
  effect: 'allow' | 'deny' | 'require_approval' | 'redact' | 'constrain';
  target: string;
  reason?: string;
}

export interface PolicyApplication {
  policy_id: string;
  policy_name: string;
  policy_version: string;
  atlas_ref: string;
  rules_evaluated: number;
  rules_matched: number;
  effects: PolicyEffect[];
  evaluation_time_ms: number;
}

export interface Warning {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface TelemetryLink {
  trace_id: string;
  span_id: string;
  events_emitted: number;
}

export interface TTL {
  context_expires_at: string;
  resolution_expires_at: string;
  refresh_hint_seconds?: number;
}

export interface CARPResolution {
  /** Protocol version */
  carp_version: CARPVersion;
  /** Echoes request ID */
  request_id: string;
  /** Server-generated resolution ID (UUIDv7) */
  resolution_id: string;
  /** ISO 8601 timestamp */
  timestamp: string;
  /** Resolution decision */
  decision: CARPDecision;
  /** Assembled context blocks */
  context_blocks: ContextBlock[];
  /** Permitted actions */
  allowed_actions: ActionPermission[];
  /** Explicitly denied actions */
  denied_actions: ActionDenial[];
  /** Policies that were applied */
  policies_applied: PolicyApplication[];
  /** Supporting evidence */
  evidence: Evidence[];
  /** Time bounds */
  ttl: TTL;
  /** Telemetry linkage */
  telemetry_link: TelemetryLink;
  /** Non-fatal warnings */
  warnings?: Warning[];
}

// =============================================================================
// CARP Action Request
// =============================================================================

export interface ExecutionOptions {
  timeout_ms?: number;
  dry_run?: boolean;
  capture_output?: boolean;
  sandbox?: boolean;
}

export interface CARPActionRequest {
  carp_version: CARPVersion;
  request_id: string;
  timestamp: string;
  operation: 'execute';
  requester: RequesterInfo;
  action: {
    action_id: string;
    resolution_id: string;
    action_type: string;
    parameters: Record<string, unknown>;
  };
  execution_options?: ExecutionOptions;
  telemetry?: TelemetryOptions;
}

// =============================================================================
// CARP Execution Result
// =============================================================================

export type ExecutionStatus =
  | 'success'
  | 'partial_success'
  | 'failed'
  | 'timeout'
  | 'cancelled'
  | 'pending_approval';

export interface SideEffect {
  type: string;
  target: string;
  reversible: boolean;
  details?: Record<string, unknown>;
}

export interface ExecutionError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  retriable: boolean;
}

export interface ExecutionMetrics {
  duration_ms: number;
  tokens_used?: number;
  api_calls?: number;
}

export interface CARPExecutionResult {
  carp_version: CARPVersion;
  request_id: string;
  execution_id: string;
  timestamp: string;
  status: ExecutionStatus;
  result?: {
    output: unknown;
    output_hash: string;
    output_type: string;
  };
  error?: ExecutionError;
  metrics: ExecutionMetrics;
  side_effects?: SideEffect[];
  telemetry_link: TelemetryLink;
}

// =============================================================================
// CARP Errors
// =============================================================================

export type CARPErrorCode =
  // Request errors
  | 'INVALID_REQUEST'
  | 'INVALID_VERSION'
  | 'MISSING_FIELD'
  | 'INVALID_FORMAT'
  // Auth errors
  | 'UNAUTHORIZED'
  | 'FORBIDDEN'
  | 'TOKEN_EXPIRED'
  // Resolution errors
  | 'ATLAS_NOT_FOUND'
  | 'DOMAIN_NOT_FOUND'
  | 'RESOLUTION_EXPIRED'
  | 'RESOLUTION_NOT_FOUND'
  // Execution errors
  | 'ACTION_NOT_PERMITTED'
  | 'ACTION_DENIED'
  | 'CONSTRAINT_VIOLATED'
  | 'EXECUTION_FAILED'
  | 'TIMEOUT'
  // Rate limiting
  | 'RATE_LIMITED'
  // System errors
  | 'INTERNAL_ERROR'
  | 'SERVICE_UNAVAILABLE';

export interface CARPError {
  carp_version: CARPVersion;
  request_id: string;
  timestamp: string;
  error: {
    code: CARPErrorCode;
    message: string;
    details?: Record<string, unknown>;
    trace_id?: string;
  };
  retry?: {
    retriable: boolean;
    retry_after_seconds?: number;
    max_retries?: number;
  };
}
