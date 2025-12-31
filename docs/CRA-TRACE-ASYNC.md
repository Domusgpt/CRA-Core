# TRACE Async Design

## Default: Async, Non-Blocking

Most CRAs are low governance. TRACE shouldn't slow agents down.

**Default behavior:**
- Events queued, not immediately written
- Hash chain built in background
- Agent continues without waiting
- Cached results accepted

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     ASYNC TRACE FLOW                                 │
│                                                                      │
│  Agent Action                                                        │
│       │                                                              │
│       ▼                                                              │
│  ┌─────────────────┐                                                │
│  │  Wrapper        │                                                │
│  │  Captures Event │                                                │
│  └────────┬────────┘                                                │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐         ┌─────────────────┐                   │
│  │  Event Queue    │────────▶│  Async Processor│                   │
│  │  (in memory)    │         │  (background)   │                   │
│  └─────────────────┘         └────────┬────────┘                   │
│           │                           │                              │
│           ▼                           ▼                              │
│  Agent continues              ┌─────────────────┐                   │
│  (not blocked)                │  TRACE Store    │                   │
│                               │  + Hash Chain   │                   │
│                               └─────────────────┘                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Event Queue

### Structure

```rust
pub struct TraceQueue {
    /// Pending events (not yet written)
    pending: VecDeque<TraceEvent>,

    /// Maximum queue size before force flush
    max_size: usize,

    /// Last known chain hash (for linking)
    last_hash: String,

    /// Flush interval
    flush_interval: Duration,
}
```

### Queue Behavior

| Condition | Action |
|-----------|--------|
| Event added | Queue it, continue |
| Queue full | Force flush (async) |
| Interval elapsed | Background flush |
| Session end | Sync flush (wait) |
| Sync required | Immediate flush + wait |

---

## Async Processor

Runs in background, processes queued events:

```rust
impl AsyncTraceProcessor {
    pub async fn run(&mut self) {
        loop {
            // Wait for events or interval
            tokio::select! {
                event = self.queue.recv() => {
                    self.buffer.push(event);
                    if self.buffer.len() >= self.batch_size {
                        self.flush().await;
                    }
                }
                _ = tokio::time::sleep(self.flush_interval) => {
                    if !self.buffer.is_empty() {
                        self.flush().await;
                    }
                }
            }
        }
    }

    async fn flush(&mut self) {
        // Compute hashes for batch
        let mut prev_hash = self.last_hash.clone();
        for event in &mut self.buffer {
            event.previous_hash = prev_hash.clone();
            event.hash = event.compute_hash();
            prev_hash = event.hash.clone();
        }

        // Write to storage
        self.storage.write_batch(&self.buffer).await;

        // Update state
        self.last_hash = prev_hash;
        self.buffer.clear();
    }
}
```

---

## When Sync Is Required

Some events need immediate confirmation:

### 1. Policy Checks

```rust
// Atlas requires sync for policy checks
if atlas.trace_config.sync_required_for.contains("policy_check") {
    let decision = trace.record_sync(PolicyCheckEvent { ... }).await;
    if decision.denied {
        return Err(PolicyDenied(decision.reason));
    }
}
```

### 2. High-Risk Actions

```rust
// High-risk actions wait for confirmation
if action.risk_tier >= RiskTier::High {
    trace.record_sync(action_event).await?;
}
```

### 3. Session Boundaries

```rust
// Session end always flushes
pub async fn end_session(&mut self) -> SessionSummary {
    self.queue.flush_sync().await;  // Wait for all events
    SessionSummary { ... }
}
```

---

## Atlas Configuration

```json
{
  "trace_config": {
    "mode": "async",

    "queue": {
      "max_size": 100,
      "flush_interval_ms": 5000,
      "batch_size": 10
    },

    "sync_required_for": [
      "policy_check",
      "session_end",
      "wrapper_construction"
    ],

    "cache": {
      "enabled": true,
      "context_ttl_seconds": 300,
      "policy_ttl_seconds": 60
    }
  }
}
```

---

## Caching

### What Gets Cached

| Item | Cache Key | TTL | Invalidation |
|------|-----------|-----|--------------|
| Context blocks | `atlas_id:context_id` | 5 min default | Atlas update |
| Policy decisions | `action:params_hash` | 1 min default | Policy change |
| Session state | `session_id` | Session lifetime | Session end |
| Atlas manifest | `atlas_id:version` | 1 hour default | Version bump |

### Cache Structure

```rust
pub struct TraceCache {
    /// Cached context blocks
    contexts: HashMap<String, CachedContext>,

    /// Cached policy decisions
    policies: HashMap<String, CachedDecision>,

    /// Session state
    sessions: HashMap<String, SessionState>,
}

pub struct CachedContext {
    content: String,
    fetched_at: Instant,
    ttl: Duration,
    hash: String,  // For verification
}

impl TraceCache {
    pub fn get_context(&self, key: &str) -> Option<&CachedContext> {
        self.contexts.get(key).filter(|c| !c.is_expired())
    }

    pub fn get_policy(&self, action: &str, params_hash: &str) -> Option<&CachedDecision> {
        let key = format!("{}:{}", action, params_hash);
        self.policies.get(&key).filter(|p| !p.is_expired())
    }
}
```

### Cache Flow

```
Agent needs context
        │
        ▼
┌─────────────────┐
│ Check cache     │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
 HIT ▼      MISS ▼
┌─────────┐  ┌─────────────┐
│ Return  │  │ Fetch from  │
│ cached  │  │ CRA-Core    │
└─────────┘  └──────┬──────┘
                    │
                    ▼
             ┌─────────────┐
             │ Cache result│
             │ Return      │
             └─────────────┘
```

---

## Hash Chain With Async

### Challenge

Async events might arrive out of order. Hash chain needs order.

### Solution: Sequence Numbers

```rust
pub struct TraceEvent {
    pub session_id: String,
    pub sequence: u64,         // Monotonic within session
    pub timestamp: DateTime,
    pub event_type: String,
    pub payload: Value,
    pub previous_hash: String, // Links to sequence - 1
    pub hash: String,
}
```

### Ordering

1. Wrapper assigns sequence number when capturing
2. Queue sorts by sequence before flush
3. Hash chain built in sequence order
4. Out-of-order arrivals wait or error

```rust
impl TraceQueue {
    pub fn add(&mut self, mut event: TraceEvent) {
        event.sequence = self.next_sequence();
        self.pending.push_back(event);
    }

    pub fn prepare_flush(&mut self) -> Vec<TraceEvent> {
        // Sort by sequence
        let mut events: Vec<_> = self.pending.drain(..).collect();
        events.sort_by_key(|e| e.sequence);
        events
    }
}
```

---

## Failure Handling

### Queue Overflow

If queue fills up faster than flush:

```rust
impl TraceQueue {
    pub fn add(&mut self, event: TraceEvent) -> Result<(), QueueFull> {
        if self.pending.len() >= self.max_size {
            // Force sync flush
            self.flush_sync()?;
        }
        self.pending.push_back(event);
        Ok(())
    }
}
```

### Write Failures

If storage write fails:

1. Keep events in queue
2. Retry with backoff
3. If persistent failure, alert custodian
4. Events not lost until queue overflow

### Recovery

On restart:
1. Load last known hash from storage
2. Resume queue from that point
3. Check for orphaned events
4. Reconcile chain if needed

---

## Performance

### Async Benefits

| Metric | Sync | Async |
|--------|------|-------|
| Event capture latency | 50-100ms | <1ms |
| Agent blocked | Yes | No |
| Throughput | Limited by storage | Limited by queue |
| Batch efficiency | 1 event/write | N events/write |

### Tuning

```json
{
  "trace_config": {
    "queue": {
      "max_size": 1000,        // More buffer = less pressure
      "flush_interval_ms": 10000,  // Less frequent = more batching
      "batch_size": 50         // Bigger batches = better I/O
    }
  }
}
```

---

## Sync Mode When Needed

For high-governance atlases:

```json
{
  "trace_config": {
    "mode": "sync",  // Everything waits
    "timeout_ms": 5000
  }
}
```

Or hybrid:

```json
{
  "trace_config": {
    "mode": "async",
    "sync_required_for": [
      "production_*",
      "delete_*",
      "policy_*"
    ]
  }
}
```

---

## Summary

| Default | Behavior |
|---------|----------|
| Mode | Async |
| Events | Queued, batched |
| Agent blocked | No |
| Cache | Enabled |
| Hash chain | Built in background |
| Sync triggers | Policy check, session end, high-risk |

Most CRAs: Async, cached, non-blocking.
High-governance CRAs: Configure sync for what matters.
