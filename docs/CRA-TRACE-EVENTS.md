# CRA TRACE Events Specification

## Overview

TRACE (Telemetry & Replay Audit Contract) records agent activity in a tamper-evident hash chain. This document specifies all event types, their fields, and async/sync behavior.

---

## Event Structure

### Base Event

Every TRACE event has this structure:

```typescript
interface TRACEEvent {
  // Identity
  event_id: string;           // UUID, unique per event
  trace_id: string;           // Spans entire operation chain
  span_id: string;            // This specific span
  parent_span_id?: string;    // Parent span (if nested)
  session_id: string;         // Agent session

  // Ordering
  sequence: number;           // Monotonic within session
  timestamp: string;          // ISO 8601 with timezone

  // Content
  event_type: string;         // From EventType enum
  payload: Record<string, unknown>;  // Type-specific data

  // Chain
  previous_hash: string;      // Hash of previous event
  hash: string;               // This event's hash
}
```

### Hash Computation

```rust
fn compute_hash(event: &TRACEEvent) -> String {
    let input = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        TRACE_VERSION,
        event.event_id,
        event.trace_id,
        event.span_id,
        event.parent_span_id.unwrap_or(""),
        event.session_id,
        event.sequence,
        event.timestamp,
        event.event_type,
        canonical_json(&event.payload),
        event.previous_hash,
    );
    sha256(input)
}
```

---

## Event Types

### Session Events

#### `session_started`

**When:** Session begins
**Sync:** No (async)

```typescript
{
  event_type: "session_started",
  payload: {
    agent_type: string,           // "claude-code", "openai-agent", etc.
    agent_version?: string,
    wrapper_version: string,
    atlas_ids: string[],          // Atlases loaded
    initial_contexts: string[],   // Context IDs injected
    intent?: string,              // Stated session intent
    environment: {
      platform: string,
      runtime?: string,
    }
  }
}
```

#### `session_ended`

**When:** Session terminates
**Sync:** Yes (must flush before exit)

```typescript
{
  event_type: "session_ended",
  payload: {
    duration_ms: number,
    event_count: number,
    actions_taken: number,
    contexts_injected: number,
    final_status: "completed" | "error" | "timeout" | "cancelled",
    summary?: string,
  }
}
```

---

### Wrapper Events

#### `wrapper_constructed`

**When:** Agent finishes building wrapper
**Sync:** Yes (must confirm before proceeding)

```typescript
{
  event_type: "wrapper_constructed",
  payload: {
    wrapper_version: string,
    components: string[],         // ["ioHooks", "traceQueue", "contextCache"]
    plugins: string[],            // Loaded plugins
    transport: string,            // "mcp", "rest", "direct"
    cache_backend: string,        // "memory", "file", etc.
    construction_hash: string,    // Proof of construction
    capabilities: {
      intercept_input: boolean,
      intercept_output: boolean,
      intercept_actions: boolean,
    }
  }
}
```

#### `wrapper_verified`

**When:** CRA verifies wrapper construction
**Sync:** Yes

```typescript
{
  event_type: "wrapper_verified",
  payload: {
    verification_passed: boolean,
    tests_run: string[],
    failures?: string[],
  }
}
```

---

### Context Events

#### `context_requested`

**When:** Agent or checkpoint requests context
**Sync:** No

```typescript
{
  event_type: "context_requested",
  payload: {
    request_type: "explicit" | "checkpoint" | "keyword",
    query?: string,
    hints?: string[],
    checkpoint_type?: string,
    keywords_matched?: string[],
  }
}
```

#### `context_injected`

**When:** Context is provided to agent
**Sync:** No

```typescript
{
  event_type: "context_injected",
  payload: {
    context_ids: string[],
    atlas_ids: string[],
    total_size_bytes: number,
    trigger: "always" | "on_match" | "on_demand" | "risk_based",
    checkpoint_type?: string,
    cache_hit: boolean,
  }
}
```

#### `context_feedback`

**When:** Agent provides feedback on context usefulness
**Sync:** No

```typescript
{
  event_type: "context_feedback",
  payload: {
    context_id: string,
    helpful: boolean,
    reason?: string,
    usage_outcome?: "applied" | "ignored" | "partial",
  }
}
```

---

### Action Events

#### `action_attempted`

**When:** Agent attempts an action
**Sync:** Depends on risk tier

```typescript
{
  event_type: "action_attempted",
  payload: {
    action_type: string,
    action_params: Record<string, unknown>,
    risk_tier: "low" | "medium" | "high" | "critical",
    policy_checked: boolean,
    context_injected?: string[],  // Contexts provided for this action
  }
}
```

#### `action_completed`

**When:** Action finishes (success or failure)
**Sync:** No

```typescript
{
  event_type: "action_completed",
  payload: {
    action_type: string,
    success: boolean,
    duration_ms: number,
    result_summary?: string,
    error?: {
      code: string,
      message: string,
    }
  }
}
```

#### `action_blocked`

**When:** Policy blocks an action
**Sync:** Yes (agent must know)

```typescript
{
  event_type: "action_blocked",
  payload: {
    action_type: string,
    action_params: Record<string, unknown>,
    policy_id: string,
    reason: string,
    override_available: boolean,
  }
}
```

---

### Policy Events

#### `policy_checked`

**When:** Policy evaluation occurs
**Sync:** Yes (for blocking policies)

```typescript
{
  event_type: "policy_checked",
  payload: {
    policy_id: string,
    action_type: string,
    decision: "allow" | "deny" | "require_approval",
    conditions_evaluated: number,
    matching_rule?: string,
    context_used?: string[],
  }
}
```

#### `policy_override`

**When:** Blocked action is manually approved
**Sync:** Yes

```typescript
{
  event_type: "policy_override",
  payload: {
    policy_id: string,
    action_type: string,
    approver: string,             // Who approved
    approval_method: string,      // How they approved
    justification?: string,
  }
}
```

---

### Checkpoint Events

#### `checkpoint_triggered`

**When:** A checkpoint activates
**Sync:** No (unless checkpoint requires sync)

```typescript
{
  event_type: "checkpoint_triggered",
  payload: {
    checkpoint_type: string,      // From CheckpointType enum
    trigger_condition: string,
    priority: number,
    input_hash?: string,          // Hash of triggering input
  }
}
```

#### `checkpoint_completed`

**When:** Checkpoint handler finishes
**Sync:** No

```typescript
{
  event_type: "checkpoint_completed",
  payload: {
    checkpoint_type: string,
    duration_ms: number,
    contexts_injected: string[],
    action_taken?: string,        // "continue" | "block" | "modify"
    block_reason?: string,
  }
}
```

---

### Risk Events

#### `risk_detected`

**When:** Risk tier threshold exceeded
**Sync:** Yes (for high/critical)

```typescript
{
  event_type: "risk_detected",
  payload: {
    risk_tier: "high" | "critical",
    trigger: string,              // What triggered detection
    action_type?: string,
    recommended_action: "proceed" | "verify" | "block",
  }
}
```

#### `risk_verified`

**When:** High-risk action verified and approved
**Sync:** Yes

```typescript
{
  event_type: "risk_verified",
  payload: {
    risk_tier: string,
    verification_method: string,
    verified_by?: string,
    action_type: string,
  }
}
```

---

### Error Events

#### `error_occurred`

**When:** Error during agent operation
**Sync:** No

```typescript
{
  event_type: "error_occurred",
  payload: {
    error_type: string,
    error_code?: string,
    message: string,
    recoverable: boolean,
    context_provided?: string[],  // Error-handling context
    stack_trace?: string,
  }
}
```

#### `error_recovered`

**When:** Error was handled/recovered
**Sync:** No

```typescript
{
  event_type: "error_recovered",
  payload: {
    original_error_id: string,    // Links to error_occurred event
    recovery_method: string,
    success: boolean,
  }
}
```

---

### Input/Output Events

#### `input_received`

**When:** Agent receives input
**Sync:** No

```typescript
{
  event_type: "input_received",
  payload: {
    input_hash: string,           // Hash of input, not content
    input_size_bytes: number,
    source: "user" | "system" | "other_agent",
    checkpoints_triggered: string[],
  }
}
```

#### `output_produced`

**When:** Agent produces output
**Sync:** No

```typescript
{
  event_type: "output_produced",
  payload: {
    output_hash: string,          // Hash of output, not content
    output_size_bytes: number,
    output_type: "text" | "action" | "structured",
    contains_action: boolean,
  }
}
```

---

### Plugin Events

#### `plugin_loaded`

**When:** Plugin initializes
**Sync:** No

```typescript
{
  event_type: "plugin_loaded",
  payload: {
    plugin_name: string,
    plugin_version: string,
    capabilities: string[],
  }
}
```

#### `plugin_event`

**When:** Plugin records custom event
**Sync:** Depends on plugin config

```typescript
{
  event_type: "plugin_event",
  payload: {
    plugin_name: string,
    custom_type: string,          // Plugin-defined type
    data: Record<string, unknown>,
  }
}
```

---

## Sync vs Async

### Default: Async

Most events are queued and processed asynchronously:

```
Event captured → Queue → Background flush → Storage
      ↓
Agent continues immediately
```

### When Sync Is Required

| Event Type | Sync | Reason |
|------------|------|--------|
| `session_started` | No | Can proceed immediately |
| `session_ended` | Yes | Must flush before exit |
| `wrapper_constructed` | Yes | Must verify before proceeding |
| `wrapper_verified` | Yes | Agent needs result |
| `context_injected` | No | Already done |
| `action_attempted` (low risk) | No | Non-blocking |
| `action_attempted` (high risk) | Yes | Must check policy |
| `action_blocked` | Yes | Agent must know |
| `policy_checked` | Yes* | Only if blocking |
| `risk_detected` (high/critical) | Yes | Must verify |
| All others | No | Async by default |

### Configuring Sync

Atlas can specify sync requirements:

```json
{
  "trace_config": {
    "sync_required_for": [
      "action_attempted:high",
      "action_attempted:critical",
      "policy_checked",
      "risk_detected",
      "session_ended"
    ]
  }
}
```

---

## Event Relationships

### Span Relationships

Events can be nested using span IDs:

```
session_started (span: A)
├── input_received (span: B, parent: A)
│   ├── checkpoint_triggered (span: C, parent: B)
│   └── context_injected (span: D, parent: B)
├── action_attempted (span: E, parent: A)
│   ├── policy_checked (span: F, parent: E)
│   └── action_completed (span: G, parent: E)
└── session_ended (span: H, parent: A)
```

### Trace ID

All events in a logical operation share a `trace_id`:

```
User request → Agent processing → Multiple actions → Response
     └──────────── All share same trace_id ───────────┘
```

---

## Storage Considerations

### What to Store

| Level | Events | Use Case |
|-------|--------|----------|
| Minimal | session_*, action_blocked, error_occurred | Basic audit |
| Standard | + action_*, context_*, policy_* | Full audit |
| Verbose | + input_*, output_*, checkpoint_* | Debugging |
| Complete | All events | Compliance, forensics |

### Retention

Atlas can configure retention:

```json
{
  "trace_config": {
    "retention": {
      "default_days": 90,
      "by_event_type": {
        "session_*": 365,
        "action_blocked": 365,
        "policy_override": 730
      }
    }
  }
}
```

---

## Replay

TRACE enables session replay:

```typescript
interface ReplaySession {
  // Load events for session
  load(sessionId: string): TRACEEvent[];

  // Verify chain integrity
  verifyChain(): VerificationResult;

  // Step through events
  step(callback: (event: TRACEEvent) => void): void;

  // Jump to specific sequence
  seek(sequence: number): TRACEEvent;

  // Filter by type
  filter(types: string[]): TRACEEvent[];
}
```

---

## Extensibility

### Custom Event Types

Plugins can register custom events:

```typescript
cra.registerEventType('custom_analysis', {
  description: 'Custom analysis event',
  sync: false,

  schema: {
    analysis_type: 'string',
    score: 'number',
    details: 'object',
  },

  validate: (payload) => {
    return payload.score >= 0 && payload.score <= 1;
  },
});
```

### Event Hooks

Plugins can hook into events:

```typescript
cra.onEvent('action_attempted', async (event) => {
  // Custom processing
  await customAuditLog(event);
});

cra.beforeEvent('session_ended', async (event) => {
  // Modify before recording
  event.payload.custom_metric = calculateMetric();
  return event;
});
```

---

## Summary

| Category | Events | Default Sync |
|----------|--------|--------------|
| Session | session_started, session_ended | No, Yes |
| Wrapper | wrapper_constructed, wrapper_verified | Yes, Yes |
| Context | context_requested, context_injected, context_feedback | No |
| Action | action_attempted, action_completed, action_blocked | Depends, No, Yes |
| Policy | policy_checked, policy_override | Yes, Yes |
| Checkpoint | checkpoint_triggered, checkpoint_completed | No |
| Risk | risk_detected, risk_verified | Yes |
| Error | error_occurred, error_recovered | No |
| I/O | input_received, output_produced | No |
| Plugin | plugin_loaded, plugin_event | No |

All events are hash-chained. Most are async. Sync only when the agent needs to know or wait.
