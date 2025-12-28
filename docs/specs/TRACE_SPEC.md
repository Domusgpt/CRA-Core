# TRACE/1.0 Specification

## Telemetry & Replay Artifact Contract Envelope

**Version**: 1.0
**Status**: Draft
**Last Updated**: 2025-01-01

---

## 1. Introduction

TRACE (Telemetry & Replay Artifact Contract Envelope) defines an append-only, runtime-emitted telemetry system for CRA operations. It provides the foundation for audit, replay, regression testing, and compliance verification.

### 1.1 Core Principle

> **If it wasn't emitted by the runtime, it didn't happen.**

LLM narration is non-authoritative. Only TRACE events constitute proof of execution.

### 1.2 Design Goals

1. **Immutability**: Events cannot be modified after emission
2. **Ordering**: Events have guaranteed sequence within a session
3. **Integrity**: Hash chains detect tampering
4. **Replayability**: Traces can reconstruct execution flow
5. **Diffability**: Traces can be compared for regression testing
6. **Streaming**: Events are emitted in real-time (JSONL format)
7. **Audit-Grade**: Suitable for compliance and forensic analysis

---

## 2. Event Model

### 2.1 TRACEEvent Structure

```typescript
interface TRACEEvent {
  // Protocol identification
  trace_version: "1.0";

  // Event identification
  event_id: string;                // UUIDv7 (time-ordered)
  sequence: number;                // Monotonic within session (starts at 1)
  timestamp: string;               // ISO 8601 with microseconds

  // Correlation
  trace_id: string;                // Root trace identifier
  span_id: string;                 // Current span
  parent_span_id?: string;         // Parent span (for nesting)
  session_id: string;              // Session identifier

  // Event classification
  event_type: TRACEEventType;
  severity: "debug" | "info" | "warn" | "error";

  // Event data
  payload: Record<string, unknown>;

  // Artifacts (large objects stored separately)
  artifacts?: ArtifactReference[];

  // Integrity
  previous_event_hash?: string;    // SHA-256 of previous event
  event_hash: string;              // SHA-256 of this event (excluding this field)

  // Metadata
  source: {
    component: string;             // e.g., "carp.resolver", "trace.collector"
    version: string;               // Component version
    instance_id?: string;          // For distributed systems
  };

  tags?: Record<string, string>;   // Arbitrary tags for filtering
}
```

### 2.2 Event Types

```typescript
type TRACEEventType =
  // Session lifecycle
  | "session.started"
  | "session.ended"
  | "session.error"

  // CARP resolution
  | "carp.request.received"
  | "carp.request.validated"
  | "carp.resolution.started"
  | "carp.atlas.loaded"
  | "carp.context.selected"
  | "carp.context.assembled"
  | "carp.policy.evaluation.started"
  | "carp.policy.rule.matched"
  | "carp.policy.evaluation.completed"
  | "carp.actions.resolved"
  | "carp.evidence.gathered"
  | "carp.resolution.completed"
  | "carp.resolution.cached"
  | "carp.resolution.cache_hit"

  // CARP execution
  | "carp.action.requested"
  | "carp.action.validated"
  | "carp.action.approved"
  | "carp.action.approval.pending"
  | "carp.action.approval.timeout"
  | "carp.action.denied"
  | "carp.action.started"
  | "carp.action.completed"
  | "carp.action.failed"
  | "carp.action.side_effect"

  // Atlas operations
  | "atlas.load.started"
  | "atlas.load.completed"
  | "atlas.load.failed"
  | "atlas.validation.started"
  | "atlas.validation.completed"
  | "atlas.validation.failed"
  | "atlas.cache.hit"
  | "atlas.cache.miss"

  // Adapter operations
  | "adapter.tool.generated"
  | "adapter.prompt.generated"
  | "adapter.call.received"
  | "adapter.call.translated"
  | "adapter.call.forwarded"
  | "adapter.response.received"

  // System events
  | "system.startup"
  | "system.shutdown"
  | "system.config.loaded"
  | "system.health.check"

  // Error events
  | "error.validation"
  | "error.auth"
  | "error.policy"
  | "error.execution"
  | "error.internal"

  // Custom events
  | `custom.${string}`;
```

### 2.3 Artifact Reference

```typescript
interface ArtifactReference {
  artifact_id: string;             // UUIDv7
  type: ArtifactType;
  name: string;                    // Human-readable name

  // Content info
  content_hash: string;            // SHA-256
  size_bytes: number;
  mime_type: string;

  // Storage
  storage: "inline" | "external";
  inline_content?: string;         // Base64 for small artifacts (<4KB)
  external_ref?: string;           // Storage path/URL for large artifacts

  // Metadata
  created_at: string;
  expires_at?: string;             // Optional expiration
}

type ArtifactType =
  | "request"                      // CARP request
  | "resolution"                   // CARP resolution
  | "context_block"                // Context content
  | "action_input"                 // Action parameters
  | "action_output"                // Action result
  | "error_detail"                 // Error information
  | "evidence"                     // Supporting evidence
  | "policy"                       // Policy document
  | "custom";
```

---

## 3. Span Model

Spans provide hierarchical structure to traces.

### 3.1 Span Structure

```typescript
interface Span {
  span_id: string;                 // UUIDv7
  trace_id: string;                // Root trace
  parent_span_id?: string;         // Parent (null for root)

  name: string;                    // e.g., "carp.resolve"
  kind: "internal" | "client" | "server";

  started_at: string;              // ISO 8601
  ended_at?: string;               // ISO 8601 (null if in progress)

  status: "in_progress" | "ok" | "error" | "timeout" | "cancelled";
  status_message?: string;

  attributes: Record<string, unknown>;

  events: SpanEvent[];             // Lightweight events within span
  links?: SpanLink[];              // Links to other traces
}

interface SpanEvent {
  name: string;
  timestamp: string;
  attributes?: Record<string, unknown>;
}

interface SpanLink {
  trace_id: string;
  span_id: string;
  relationship: "caused_by" | "follows_from" | "child_of";
}
```

### 3.2 Span Hierarchy Example

```
trace_id: abc123
│
├── span: session.main
│   ├── span: carp.resolve
│   │   ├── span: atlas.load
│   │   ├── span: policy.evaluate
│   │   └── span: context.assemble
│   │
│   └── span: carp.execute
│       ├── span: action.validate
│       └── span: action.run
```

---

## 4. Integrity Model

### 4.1 Event Hashing

Each event is hashed to enable tamper detection:

```typescript
function computeEventHash(event: TRACEEvent): string {
  // Create canonical form (sorted keys, no event_hash field)
  const canonical = canonicalize({
    ...event,
    event_hash: undefined
  });

  return sha256(canonical);
}
```

### 4.2 Hash Chain

Events form a chain via `previous_event_hash`:

```
Event 1: hash=H1, prev=null
Event 2: hash=H2, prev=H1
Event 3: hash=H3, prev=H2
...
```

Verification:
```typescript
function verifyChain(events: TRACEEvent[]): boolean {
  for (let i = 1; i < events.length; i++) {
    if (events[i].previous_event_hash !== events[i-1].event_hash) {
      return false;
    }
    if (computeEventHash(events[i]) !== events[i].event_hash) {
      return false;
    }
  }
  return true;
}
```

### 4.3 Artifact Integrity

Artifact content is verified by hash:

```typescript
function verifyArtifact(artifact: ArtifactReference, content: Buffer): boolean {
  return sha256(content) === artifact.content_hash;
}
```

---

## 5. Wire Format

### 5.1 JSONL Streaming

TRACE events are streamed as newline-delimited JSON (JSONL):

```jsonl
{"trace_version":"1.0","event_id":"0192...","event_type":"session.started",...}
{"trace_version":"1.0","event_id":"0192...","event_type":"carp.request.received",...}
{"trace_version":"1.0","event_id":"0192...","event_type":"carp.resolution.completed",...}
```

### 5.2 File Storage

Trace files use `.trace.jsonl` extension:

```
traces/
  2025-01-01T10-30-00-abc123.trace.jsonl
  2025-01-01T11-45-00-def456.trace.jsonl
```

### 5.3 Artifact Storage

Large artifacts stored separately:

```
artifacts/
  0192-xxxx-artifact.json
  0192-yyyy-artifact.bin
```

Referenced in events via `external_ref`.

---

## 6. Streaming Protocol

### 6.1 WebSocket Stream

```typescript
// Client connects
ws://host/trace/stream?session_id=xxx

// Server sends events
{"type":"event","data":{...TRACEEvent...}}
{"type":"event","data":{...TRACEEvent...}}
{"type":"heartbeat","timestamp":"..."}

// Client can send control messages
{"type":"pause"}
{"type":"resume"}
{"type":"filter","event_types":["carp.*"]}
```

### 6.2 SSE Stream

```http
GET /trace/stream?session_id=xxx HTTP/1.1
Accept: text/event-stream

event: trace
data: {...TRACEEvent...}

event: trace
data: {...TRACEEvent...}

event: heartbeat
data: {"timestamp":"..."}
```

---

## 7. Replay Semantics

### 7.1 Replay Request

```typescript
interface ReplayRequest {
  trace_file: string;              // Path to trace file
  mode: "full" | "fast_forward" | "step";
  speed?: number;                  // 1.0 = real-time, 2.0 = 2x, etc.
  start_at?: string;               // Event ID to start from
  stop_at?: string;                // Event ID to stop at
  filter?: {
    event_types?: string[];        // Only these types
    spans?: string[];              // Only these spans
  };
}
```

### 7.2 Replay Output

```typescript
interface ReplayEvent {
  original_event: TRACEEvent;
  replay_timestamp: string;
  time_delta_ms: number;           // Time since last event
  sequence_position: number;       // Current position
  total_events: number;
}
```

---

## 8. Diff Semantics

### 8.1 Trace Comparison

```typescript
interface TraceDiff {
  summary: {
    events_added: number;
    events_removed: number;
    events_modified: number;
    artifacts_changed: number;
  };

  differences: TraceDifference[];

  compatibility: "identical" | "compatible" | "breaking";
}

interface TraceDifference {
  type: "added" | "removed" | "modified";
  path: string;                    // JSON path to difference
  expected?: unknown;
  actual?: unknown;
  severity: "info" | "warning" | "error";
  message: string;
}
```

### 8.2 Golden Trace Comparison

For regression testing:

```typescript
interface GoldenTraceTest {
  name: string;
  description: string;
  golden_trace: string;            // Path to expected trace
  input: CARPRequest;              // Input to reproduce

  comparison: {
    ignore_fields: string[];       // Fields to ignore (e.g., timestamps)
    ignore_event_types: string[];  // Event types to ignore
    allow_additional_events: boolean;
    artifact_comparison: "hash" | "content" | "skip";
  };

  assertions?: GoldenAssertion[];
}

interface GoldenAssertion {
  path: string;                    // JSON path
  operator: "eq" | "neq" | "exists" | "not_exists" | "matches";
  value?: unknown;
  message: string;
}
```

---

## 9. Event Payloads

### 9.1 Session Events

```typescript
// session.started
{
  agent_id: string;
  agent_type: string;
  session_config: Record<string, unknown>;
  environment: {
    platform: string;
    version: string;
    atlases_loaded: string[];
  };
}

// session.ended
{
  duration_ms: number;
  events_emitted: number;
  resolutions_completed: number;
  actions_executed: number;
  errors_occurred: number;
}
```

### 9.2 CARP Events

```typescript
// carp.request.received
{
  request_id: string;
  operation: string;
  goal?: string;
  goal_hash?: string;
  risk_tier?: string;
}

// carp.resolution.completed
{
  request_id: string;
  resolution_id: string;
  decision_type: string;
  context_blocks_count: number;
  allowed_actions_count: number;
  denied_actions_count: number;
  policies_applied_count: number;
  duration_ms: number;
}

// carp.action.completed
{
  action_id: string;
  action_type: string;
  status: string;
  duration_ms: number;
  output_hash?: string;
  side_effects_count: number;
}
```

### 9.3 Error Events

```typescript
// error.*
{
  error_code: string;
  error_message: string;
  error_details?: Record<string, unknown>;
  stack_trace?: string;            // Only in debug mode
  recovery_attempted: boolean;
  recovery_successful?: boolean;
}
```

---

## 10. Filtering and Querying

### 10.1 Filter Syntax

```typescript
interface TraceFilter {
  // Time range
  from?: string;                   // ISO 8601
  to?: string;                     // ISO 8601

  // Event selection
  event_types?: string[];          // Glob patterns: "carp.*"
  severity?: string[];             // Minimum severity
  spans?: string[];                // Specific spans

  // Content filtering
  payload_match?: Record<string, unknown>;  // Partial match

  // Limits
  limit?: number;
  offset?: number;
}
```

### 10.2 Query Examples

```typescript
// All CARP errors in last hour
{
  from: "2025-01-01T09:00:00Z",
  event_types: ["carp.*.failed", "carp.*.denied", "error.policy"],
  severity: ["error"]
}

// Resolution timeline for specific request
{
  spans: ["0192-xxx-resolve"],
  event_types: ["carp.resolution.*"]
}
```

---

## 11. Retention and Archival

### 11.1 Retention Policies

```typescript
interface RetentionPolicy {
  name: string;
  condition: RetentionCondition;
  retention_days: number;
  archive: boolean;                // Archive before delete
  archive_location?: string;
}

type RetentionCondition =
  | { type: "all" }
  | { type: "severity"; min: string }
  | { type: "event_type"; patterns: string[] }
  | { type: "custom"; expression: string };
```

### 11.2 Archival Format

Archived traces use compressed JSONL:

```
archive/
  2025-01/
    traces-2025-01-01.trace.jsonl.gz
    artifacts-2025-01-01.tar.gz
```

---

## 12. Conformance Requirements

### 12.1 MUST Requirements

1. MUST use UUIDv7 for event_id (time-ordered)
2. MUST include all required fields in TRACEEvent
3. MUST compute SHA-256 hashes correctly
4. MUST maintain hash chain integrity
5. MUST use monotonic sequence numbers within session
6. MUST emit events in real-time (not batched)
7. MUST preserve event ordering

### 12.2 SHOULD Requirements

1. SHOULD stream events via JSONL
2. SHOULD support WebSocket streaming
3. SHOULD implement replay functionality
4. SHOULD implement diff functionality
5. SHOULD compress archived traces

### 12.3 MAY Requirements

1. MAY implement custom event types
2. MAY add custom metadata fields
3. MAY implement distributed tracing integration

---

## 13. Security Considerations

### 13.1 Sensitive Data

- NEVER log raw credentials/tokens in payloads
- Use artifact redaction for sensitive outputs
- Apply policy-based redaction rules

### 13.2 Integrity Protection

- Store hash chain root in secure location
- Sign trace files for tamper evidence
- Use append-only storage where possible

### 13.3 Access Control

- Trace access requires appropriate scopes
- Filter events based on viewer permissions
- Audit trace access itself
