# Performance vs. Consistency Trade-offs in CRA Trace System

## Current State

### Architecture Now
```
Resolver::resolve()
    │
    ├── Policy evaluation (~25µs)
    │
    └── For each TRACE event:
            ├── Create TRACEEvent
            ├── Compute SHA-256 hash (~15µs)  ← BLOCKING
            ├── Chain to previous event
            └── Store in HashMap
                    │
                    └── Total: ~127µs per resolve()
```

### What We Built (Infrastructure Ready, Not Wired)
```
                              ┌─────────────────────────┐
Resolver::resolve()           │   TraceRingBuffer       │
    │                         │   (lock-free, 4096)     │
    ├── Policy eval (~25µs)   │                         │
    │                         │   TraceProcessor        │
    └── Push RawEvent ────────│   (background thread)   │
        (<1µs, no hash)       │   - Computes hashes     │
                              │   - Chains events       │
                              │   - Stores to backend   │
                              └─────────────────────────┘
```

## The Core Trade-off

### Option A: Immediate Consistency (Current)
**How it works:** Hash computed inline, events immediately queryable

```rust
// Current behavior
resolver.resolve(&request)?;           // ~127µs, blocks for hash
let trace = resolver.get_trace(&session)?;  // Immediately available
let valid = resolver.verify_chain(&session)?; // Works immediately
```

**Pros:**
- Simple mental model
- Events always consistent
- Chain verification always works
- No "eventual consistency" surprises
- Easy debugging - events are there when you expect

**Cons:**
- 127µs per resolve() - too slow for high-frequency use
- Hash computation on hot path
- Can't scale to 100K+ resolutions/second

---

### Option B: Deferred Processing (New Infrastructure)
**How it works:** Events pushed to buffer immediately, hashed in background

```rust
// Deferred behavior
resolver.resolve(&request)?;           // <10µs, returns before hash
let trace = resolver.get_trace(&session)?;  // May be incomplete!
let valid = resolver.verify_chain(&session)?; // May fail - events still processing
```

**Pros:**
- Target <10µs resolve() achievable
- Non-blocking hot path
- Scales to millions of resolutions/second
- Background work doesn't affect latency

**Cons:**
- Events not immediately queryable
- Chain verification requires waiting
- "Eventual consistency" model
- More complex error handling
- Harder to debug timing issues

---

## The Fundamental Question

**What guarantees does your use case need?**

### Use Case 1: Real-time Audit Display
```
Agent resolves → UI shows trace immediately
```
**Requires:** Immediate consistency (Option A)
**Acceptable latency:** Higher is OK if events appear instantly

### Use Case 2: High-frequency Agent Swarm
```
1000 agents × 100 resolutions/second = 100,000 res/sec
```
**Requires:** Deferred processing (Option B)
**Acceptable latency:** Must be <10µs, events can lag

### Use Case 3: Hybrid
```
Fast resolution, but wait before displaying trace
```
**Requires:** Deferred processing + explicit sync point
**Pattern:** resolve() fast, then `await_trace_sync()` before display

---

## Implementation Paths

### Path 1: Keep Current (Recommended for v1.0)
- resolve() stays at ~127µs
- All infrastructure ready for future optimization
- Simple, predictable behavior
- Good enough for most single-agent use cases

**Changes needed:** None - it works today

---

### Path 2: Opt-in Deferred Mode
Add a configuration flag to enable deferred processing:

```rust
// New API
let resolver = Resolver::new()
    .with_deferred_tracing(true);  // Enable background processing

// resolve() now returns before events are hashed
resolver.resolve(&request)?;  // <10µs

// Must explicitly wait for trace to be ready
resolver.await_trace_ready(&session_id)?;  // Blocks until processed

// Or check if ready
if resolver.is_trace_ready(&session_id) {
    let trace = resolver.get_trace(&session_id)?;
}
```

**Changes needed:**
1. Add `deferred_tracing: bool` to Resolver config
2. Modify `TraceCollector::emit()` to push to RingBuffer when deferred
3. Add `await_trace_ready()` and `is_trace_ready()` methods
4. Start background TraceProcessor when deferred mode enabled
5. Update `get_trace()` to either wait or return partial results

---

### Path 3: Fully Async (AsyncRuntime Only)
Keep sync Resolver as-is, but AsyncRuntime uses deferred:

```rust
// Sync - immediate consistency
let resolver = Resolver::new();
resolver.resolve(&request)?;  // ~127µs, events ready

// Async - deferred consistency
let runtime = AsyncRuntime::new(config).await?;
runtime.resolve(&request).await?;  // <10µs, events may lag
runtime.flush_traces().await?;     // Ensure all events processed
```

**Changes needed:**
1. AsyncRuntime::resolve() pushes RawEvents to buffer
2. Background tokio task processes events
3. Add `flush_traces()` for explicit sync
4. Storage gets events from processor, not resolver

---

## What Changes to Wire Deferred Mode

### Step 1: Modify TraceCollector
```rust
pub struct TraceCollector {
    // Existing
    sessions: HashMap<String, SessionTrace>,

    // New
    buffer: Option<Arc<TraceRingBuffer>>,
    deferred: bool,
}

impl TraceCollector {
    pub fn emit(...) -> Result<&TRACEEvent> {
        if self.deferred {
            // Push to buffer without hash
            let raw = RawEvent::new(session_id, trace_id, event_type, payload);
            self.buffer.as_ref().unwrap().push(raw);
            // Return placeholder or nothing
        } else {
            // Current behavior - compute hash inline
        }
    }
}
```

### Step 2: Add Sync Points to Resolver
```rust
impl Resolver {
    /// Wait for all pending trace events to be processed
    pub fn await_trace_ready(&self, session_id: &str) -> Result<()> {
        // Spin/wait until processor has drained buffer for this session
    }

    /// Check if trace is complete
    pub fn is_trace_ready(&self, session_id: &str) -> bool {
        // Check if any events for this session are still in buffer
    }
}
```

### Step 3: Start Processor
```rust
impl Resolver {
    pub fn with_deferred_tracing(mut self, enabled: bool) -> Self {
        if enabled {
            self.trace_collector.enable_deferred(buffer.clone());
            // Start background processor
            let processor = TraceProcessor::new(buffer, storage, config);
            self.processor_handle = Some(processor.start());
        }
        self
    }
}
```

---

## Impact on Existing Code

### Breaking Changes (if we switch to deferred by default)
1. `get_trace()` may return incomplete results
2. `verify_chain()` may fail for in-flight events
3. Tests that check trace immediately after resolve() may fail

### Non-Breaking (if opt-in)
1. Default behavior unchanged
2. New methods added for deferred mode
3. Existing code works as before

---

## Recommendation

**For now (v1.0):** Keep immediate consistency as default

**For v1.1 (performance release):**
1. Add `with_deferred_tracing(bool)` configuration
2. Default to `false` (current behavior)
3. Document the trade-offs clearly
4. Let users opt-in when they need performance

**For swarm use cases:**
- Use AsyncRuntime which already has the buffer integrated
- Add `flush_traces()` method for explicit sync

---

## Questions to Decide

1. **Should `get_trace()` block until ready, or return partial?**
   - Block: Simpler API, hides complexity
   - Partial: More control, can show "processing..." UI

2. **Should there be a timeout on awaiting trace?**
   - Yes: Prevents hangs if processor is stuck
   - No: Simpler, let caller decide timeout

3. **How to handle buffer overflow (backpressure)?**
   - Drop oldest events: Lose data
   - Block resolve(): Defeats purpose
   - Return error: User decides

4. **Should deferred mode be per-session or global?**
   - Per-session: More flexibility
   - Global: Simpler implementation

---

## Performance Targets

| Mode | resolve() | get_trace() | verify_chain() |
|------|-----------|-------------|----------------|
| Immediate | ~127µs | <1µs | ~14µs/event |
| Deferred | <10µs | may wait | may wait |
| Deferred + await | <10µs + wait | <1µs | ~14µs/event |

The "wait" is bounded by:
- Buffer drain rate (~50ms batches)
- Hash computation (~15µs/event)
- Storage write time (varies)

For a typical resolve() with 6 events:
- Processing time: ~90µs (6 × 15µs hashing)
- Max wait: ~50ms (one batch interval)
- Expected wait: ~25ms average
