/**
 * TRACE/1.0 - Telemetry & Replay Artifact Contract Envelope
 *
 * Core type definitions for the runtime-emitted telemetry system.
 * Rule: If it wasn't emitted by the runtime, it didn't happen.
 */

// =============================================================================
// Core Types
// =============================================================================

export type TRACEVersion = '1.0';

export type Severity = 'debug' | 'info' | 'warn' | 'error';

// =============================================================================
// Event Types
// =============================================================================

export type TRACEEventType =
  // Session lifecycle
  | 'session.started'
  | 'session.ended'
  | 'session.error'

  // CARP resolution
  | 'carp.request.received'
  | 'carp.request.validated'
  | 'carp.resolution.started'
  | 'carp.atlas.loaded'
  | 'carp.context.selected'
  | 'carp.context.assembled'
  | 'carp.policy.evaluation.started'
  | 'carp.policy.rule.matched'
  | 'carp.policy.evaluation.completed'
  | 'carp.actions.resolved'
  | 'carp.evidence.gathered'
  | 'carp.resolution.completed'
  | 'carp.resolution.cached'
  | 'carp.resolution.cache_hit'

  // CARP execution
  | 'carp.action.requested'
  | 'carp.action.validated'
  | 'carp.action.approved'
  | 'carp.action.approval.pending'
  | 'carp.action.approval.timeout'
  | 'carp.action.denied'
  | 'carp.action.started'
  | 'carp.action.completed'
  | 'carp.action.failed'
  | 'carp.action.side_effect'

  // Atlas operations
  | 'atlas.load.started'
  | 'atlas.load.completed'
  | 'atlas.load.failed'
  | 'atlas.validation.started'
  | 'atlas.validation.completed'
  | 'atlas.validation.failed'
  | 'atlas.cache.hit'
  | 'atlas.cache.miss'

  // Adapter operations
  | 'adapter.tool.generated'
  | 'adapter.prompt.generated'
  | 'adapter.call.received'
  | 'adapter.call.translated'
  | 'adapter.call.forwarded'
  | 'adapter.response.received'

  // System events
  | 'system.startup'
  | 'system.shutdown'
  | 'system.config.loaded'
  | 'system.health.check'

  // Error events
  | 'error.validation'
  | 'error.auth'
  | 'error.policy'
  | 'error.execution'
  | 'error.internal'

  // Custom events (extensible)
  | `custom.${string}`;

// =============================================================================
// Artifact Types
// =============================================================================

export type ArtifactType =
  | 'request'
  | 'resolution'
  | 'context_block'
  | 'action_input'
  | 'action_output'
  | 'error_detail'
  | 'evidence'
  | 'policy'
  | 'custom';

export interface ArtifactReference {
  /** Unique artifact ID (UUIDv7) */
  artifact_id: string;
  /** Artifact type classification */
  type: ArtifactType;
  /** Human-readable name */
  name: string;
  /** SHA-256 hash of content */
  content_hash: string;
  /** Size in bytes */
  size_bytes: number;
  /** MIME type */
  mime_type: string;
  /** Storage location */
  storage: 'inline' | 'external';
  /** Base64 content for small artifacts (<4KB) */
  inline_content?: string;
  /** Storage path/URL for large artifacts */
  external_ref?: string;
  /** Creation timestamp */
  created_at: string;
  /** Optional expiration */
  expires_at?: string;
}

// =============================================================================
// Event Source
// =============================================================================

export interface EventSource {
  /** Component name (e.g., "carp.resolver", "trace.collector") */
  component: string;
  /** Component version */
  version: string;
  /** Instance ID for distributed systems */
  instance_id?: string;
}

// =============================================================================
// TRACE Event
// =============================================================================

export interface TRACEEvent {
  /** Protocol version */
  trace_version: TRACEVersion;

  /** Unique event ID (UUIDv7, time-ordered) */
  event_id: string;
  /** Monotonic sequence number within session */
  sequence: number;
  /** ISO 8601 timestamp with microseconds */
  timestamp: string;

  /** Root trace identifier */
  trace_id: string;
  /** Current span identifier */
  span_id: string;
  /** Parent span (for nesting) */
  parent_span_id?: string;
  /** Session identifier */
  session_id: string;

  /** Event classification */
  event_type: TRACEEventType;
  /** Severity level */
  severity: Severity;

  /** Event-specific payload */
  payload: Record<string, unknown>;

  /** Large objects stored separately */
  artifacts?: ArtifactReference[];

  /** SHA-256 of previous event (chain) */
  previous_event_hash?: string;
  /** SHA-256 of this event (excluding this field) */
  event_hash: string;

  /** Event source information */
  source: EventSource;

  /** Arbitrary tags for filtering */
  tags?: Record<string, string>;
}

// =============================================================================
// Span Types
// =============================================================================

export type SpanStatus = 'in_progress' | 'ok' | 'error' | 'timeout' | 'cancelled';

export type SpanKind = 'internal' | 'client' | 'server';

export interface SpanEvent {
  name: string;
  timestamp: string;
  attributes?: Record<string, unknown>;
}

export interface SpanLink {
  trace_id: string;
  span_id: string;
  relationship: 'caused_by' | 'follows_from' | 'child_of';
}

export interface Span {
  /** Span ID (UUIDv7) */
  span_id: string;
  /** Root trace ID */
  trace_id: string;
  /** Parent span (null for root) */
  parent_span_id?: string;
  /** Span name (e.g., "carp.resolve") */
  name: string;
  /** Span kind */
  kind: SpanKind;
  /** Start timestamp */
  started_at: string;
  /** End timestamp (null if in progress) */
  ended_at?: string;
  /** Current status */
  status: SpanStatus;
  /** Status message */
  status_message?: string;
  /** Span attributes */
  attributes: Record<string, unknown>;
  /** Events within span */
  events: SpanEvent[];
  /** Links to other traces */
  links?: SpanLink[];
}

// =============================================================================
// Replay Types
// =============================================================================

export type ReplayMode = 'full' | 'fast_forward' | 'step';

export interface ReplayRequest {
  /** Path to trace file */
  trace_file: string;
  /** Replay mode */
  mode: ReplayMode;
  /** Speed multiplier (1.0 = real-time) */
  speed?: number;
  /** Event ID to start from */
  start_at?: string;
  /** Event ID to stop at */
  stop_at?: string;
  /** Event filtering */
  filter?: {
    event_types?: string[];
    spans?: string[];
  };
}

export interface ReplayEvent {
  /** Original event */
  original_event: TRACEEvent;
  /** Replay timestamp */
  replay_timestamp: string;
  /** Time since last event (ms) */
  time_delta_ms: number;
  /** Current position */
  sequence_position: number;
  /** Total events in trace */
  total_events: number;
}

// =============================================================================
// Diff Types
// =============================================================================

export type DifferenceType = 'added' | 'removed' | 'modified';

export type DifferenceSeverity = 'info' | 'warning' | 'error';

export type TraceCompatibility = 'identical' | 'compatible' | 'breaking';

export interface TraceDifference {
  /** Type of difference */
  type: DifferenceType;
  /** JSON path to difference */
  path: string;
  /** Expected value */
  expected?: unknown;
  /** Actual value */
  actual?: unknown;
  /** Severity */
  severity: DifferenceSeverity;
  /** Explanation */
  message: string;
}

export interface TraceDiff {
  summary: {
    events_added: number;
    events_removed: number;
    events_modified: number;
    artifacts_changed: number;
  };
  differences: TraceDifference[];
  compatibility: TraceCompatibility;
}

// =============================================================================
// Golden Trace Types
// =============================================================================

export interface GoldenAssertion {
  /** JSON path */
  path: string;
  /** Comparison operator */
  operator: 'eq' | 'neq' | 'exists' | 'not_exists' | 'matches';
  /** Expected value */
  value?: unknown;
  /** Assertion message */
  message: string;
}

export interface GoldenTraceTest {
  /** Test name */
  name: string;
  /** Test description */
  description: string;
  /** Path to expected trace */
  golden_trace: string;
  /** Input to reproduce */
  input: Record<string, unknown>;
  /** Comparison settings */
  comparison: {
    /** Fields to ignore */
    ignore_fields: string[];
    /** Event types to ignore */
    ignore_event_types: string[];
    /** Allow additional events */
    allow_additional_events: boolean;
    /** Artifact comparison mode */
    artifact_comparison: 'hash' | 'content' | 'skip';
  };
  /** Custom assertions */
  assertions?: GoldenAssertion[];
}

export interface GoldenTraceResult {
  /** Test name */
  name: string;
  /** Pass/fail status */
  passed: boolean;
  /** Duration in ms */
  duration_ms: number;
  /** Diff results */
  diff?: TraceDiff;
  /** Assertion results */
  assertion_results?: {
    assertion: GoldenAssertion;
    passed: boolean;
    actual?: unknown;
  }[];
  /** Error if failed */
  error?: string;
}

// =============================================================================
// Filter Types
// =============================================================================

export interface TraceFilter {
  /** Time range start */
  from?: string;
  /** Time range end */
  to?: string;
  /** Event type patterns (glob) */
  event_types?: string[];
  /** Minimum severity */
  severity?: Severity[];
  /** Specific spans */
  spans?: string[];
  /** Partial payload match */
  payload_match?: Record<string, unknown>;
  /** Maximum results */
  limit?: number;
  /** Offset for pagination */
  offset?: number;
}

// =============================================================================
// Session Types
// =============================================================================

export interface SessionInfo {
  session_id: string;
  agent_id: string;
  agent_type?: string;
  started_at: string;
  ended_at?: string;
  event_count: number;
  trace_ids: string[];
  status: 'active' | 'ended' | 'error';
}

// =============================================================================
// Retention Types
// =============================================================================

export type RetentionConditionType = 'all' | 'severity' | 'event_type' | 'custom';

export interface RetentionCondition {
  type: RetentionConditionType;
  min_severity?: Severity;
  patterns?: string[];
  expression?: string;
}

export interface RetentionPolicy {
  name: string;
  condition: RetentionCondition;
  retention_days: number;
  archive: boolean;
  archive_location?: string;
}
