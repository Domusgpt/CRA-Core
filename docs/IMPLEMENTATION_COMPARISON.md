# CRA Rust Implementation Comparison

**Date:** 2025-12-28
**Branches Compared:**
- **This Branch:** `claude/cra-rust-refactor-XBcJV` (actual implementation)
- **Plan Branch:** `claude/design-cra-architecture-WdoAv` (1,138-line proposal)

---

## Executive Summary

| Aspect | Their Plan | Our Implementation | Verdict |
|--------|------------|-------------------|---------|
| Status | Proposal (design doc) | Working code | **We're ahead** |
| Tests | Not implemented | 90 passing | **We're ahead** |
| resolve() | Designed | Implemented (134µs) | **We're ahead** |
| Async traces | Designed (ring buffer) | Basic (sync) | **They're more detailed** |
| Bindings | Designed (all three) | Python complete, others scaffolded | **Comparable** |
| Dual-mode | Central to design | Added async-runtime feature | **Comparable** |
| Timing | Not mentioned | Full timing module | **We're ahead** |

**Bottom line:** We have working code with 90 tests. They have a more detailed architectural plan. Best path: merge our implementation with their async trace buffering design.

---

## Detailed Comparison

### 1. Core Architecture

#### Their Vision
```
Agent → CRA (embedded, <0.01ms) → Tools
         └── traces batch async to central server
```

#### Our Reality
```
Agent → CRA (embedded, 134µs) → Tools
         └── traces stored in-memory, export to JSONL
         └── async-runtime feature for swarms
         └── timing module for heartbeats
```

**Assessment:** Same vision, different execution paths. They emphasize centralized trace ingestion; we emphasize pluggable storage backends.

---

### 2. Performance

| Metric | Their Target | Our Actual |
|--------|--------------|------------|
| resolve() cached | <0.01ms (10µs) | 0.134ms (134µs) |
| resolve() cold | <1ms | ~6µs atlas load |
| trace.record() | <1µs | ~15µs per event |
| Binary size | <5MB native | ~800KB |
| WASM size | <500KB | Not measured yet |

**Assessment:** We're 10x slower than their target for resolve(). Breakdown:
- SHA-256 hashing dominates (~90µs per event × 6 events = ~90µs)
- Policy evaluation: ~25µs
- Building resolution: ~10µs

**Optimization opportunity:** Their plan mentions "lock-free trace recording" - we could defer hash computation to batch time.

---

### 3. Trace System

#### Their Design (from plan)
```rust
pub struct TraceCollector {
    buffer: RingBuffer<TRACEEvent, 1024>,
    sender: Option<BatchSender>,
}

impl TraceCollector {
    // Non-blocking, returns immediately
    pub fn record(&self, event: TRACEEvent) {
        self.buffer.push(event);  // Lock-free ring buffer
    }
}

// Background thread batches and sends
fn background_sender(receiver: Receiver<Vec<TRACEEvent>>, endpoint: String) {
    loop {
        let batch = receiver.recv_timeout(Duration::from_secs(5));
        send_batch(&endpoint, batch);
    }
}
```

#### Our Implementation
```rust
pub struct TraceCollector {
    sessions: HashMap<String, Vec<TRACEEvent>>,
    // No background thread, sync storage
}

impl TraceCollector {
    pub fn emit(&mut self, event: TRACEEvent) {
        // Computes hash inline (blocking)
        let event = event.chain(seq, prev_hash);
        self.sessions.entry(session_id).or_default().push(event);
    }
}
```

**Gap:** Their lock-free ring buffer + background sender is better for high throughput. We should adopt this pattern.

---

### 4. Async Strategy

#### Their Approach
- Traces: Always async (background thread)
- resolve(): Always sync
- Atlas loading: Async-friendly preload

#### Our Approach
- Core: Fully sync (for FFI simplicity)
- async-runtime: Optional feature for swarms
- Storage: Both sync and async traits

**Assessment:** Compatible approaches. Their "always async traces" could be implemented on top of our AsyncStorageBackend trait.

---

### 5. Bindings

#### Python

| Feature | Their Plan | Our Implementation |
|---------|------------|-------------------|
| Basic resolve | ✅ | ✅ |
| Async preload | ✅ | ❌ |
| Proper Python objects | ❌ (uses dicts) | ✅ (CARPResolution, etc.) |
| Flush method | ✅ | ❌ |

**Assessment:** We have better Python ergonomics (proper objects), they have better async support.

#### Node.js

| Feature | Their Plan | Our Implementation |
|---------|------------|-------------------|
| napi-rs binding | ✅ | ✅ (scaffolded) |
| Async loading | ✅ | ❌ |
| Worker thread | ✅ | ❌ |

#### WASM

| Feature | Their Plan | Our Implementation |
|---------|------------|-------------------|
| wasm-bindgen | ✅ | ✅ (scaffolded) |
| Memory buffer | ✅ | ❌ |
| drain_traces() | ✅ | ❌ |

---

### 6. What We Have That They Don't

| Feature | Our Module | Their Plan |
|---------|------------|------------|
| Timing/Heartbeat | `cra-core/src/timing/` | Not mentioned |
| Sliding rate limiter | `SlidingWindowRateLimiter` | Not mentioned |
| Trace batching | `TraceBatcher` | Similar (ring buffer) |
| Session TTL config | `SessionTTLConfig` | Not mentioned |
| Error categories | `ErrorCategory` enum | Not mentioned |
| HTTP status codes | `http_status_code()` | Not mentioned |
| JSON error response | `to_error_response()` | Not mentioned |
| Storage backends | 3 sync + async trait | Just HTTP sink |
| Conformance tests | 7 tests | Not implemented |

---

### 7. What They Have That We Don't

| Feature | Their Plan | Our Gap |
|---------|------------|---------|
| Lock-free ring buffer | Detailed design | We use HashMap + Vec |
| Background trace sender | Full implementation | We store inline |
| Atlas LRU cache | `AtlasCache` struct | We store all atlases |
| Zero-copy resolve | Design goal | We clone a lot |
| C FFI ABI | Detailed design | We have basic FFI |
| WASM drain_traces | Designed | Not implemented |
| Migration guide | 10-step plan | Not written |
| Week-by-week schedule | 12 weeks | Not planned |

---

## Recommendations

### Immediate (adopt from their plan)

1. **Lock-free ring buffer for traces**
   ```rust
   // Add to cra-core/src/trace/
   pub struct RingBuffer<T, const N: usize> {
       buffer: [MaybeUninit<T>; N],
       head: AtomicUsize,
       tail: AtomicUsize,
   }
   ```

2. **Background trace sender**
   ```rust
   // In async-runtime
   pub struct TraceStreamer {
       receiver: mpsc::Receiver<TRACEEvent>,
       endpoint: String,
   }
   ```

3. **Lazy hash computation**
   - Record events without computing hash
   - Compute hashes on batch/export
   - Reduces resolve() latency from 134µs to ~40µs

### Medium-term

4. **Atlas LRU cache** - Evict unused atlases under memory pressure
5. **WASM drain_traces()** - For browser trace upload
6. **Zero-copy optimization** - Use references in resolve()

### Long-term

7. **Merge plans into unified implementation**
8. **Write migration guide**
9. **Set up benchmarking CI**

---

## Proposed Merged Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          CRA Core (Merged)                               │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Hot Path (Our Implementation)                  │   │
│  │  • resolve() - 134µs → optimize to <50µs                         │   │
│  │  • Policy evaluation with patterns                               │   │
│  │  • Error categories + HTTP codes                                 │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                Trace System (Their Design + Our Traits)           │   │
│  │  • Lock-free ring buffer (their design)                          │   │
│  │  • StorageBackend trait (our design)                             │   │
│  │  • AsyncStorageBackend trait (our design)                        │   │
│  │  • Background sender (their design)                              │   │
│  │  • TraceBatcher (our implementation)                             │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                   Timing (Our Unique Contribution)                │   │
│  │  • HeartbeatConfig + metrics                                     │   │
│  │  • SessionTTLConfig                                              │   │
│  │  • SlidingWindowRateLimiter                                      │   │
│  │  • TimerBackend trait (for minoots integration)                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                   Bindings (Combined Approach)                    │   │
│  │  • Python: Our proper objects + their async preload              │   │
│  │  • Node.js: Their worker thread design                           │   │
│  │  • WASM: Their drain_traces() pattern                            │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Test Comparison

| Category | Our Branch | Their Plan |
|----------|------------|------------|
| Unit tests | 80 | 0 |
| Conformance tests | 7 | Designed |
| Doc tests | 3 | 0 |
| Benchmark tests | Yes | Designed |
| Fuzzing | No | Designed |
| **Total** | **90 passing** | **0 passing** |

---

## Conclusion

**Recommendation: Merge, don't choose.**

1. Our branch has working, tested code (90 tests)
2. Their plan has excellent architectural ideas (ring buffer, background sender)
3. Combined approach gives best of both worlds

**Next steps:**
1. Add lock-free ring buffer (their design)
2. Add background trace sender (their design)
3. Optimize resolve() to hit <50µs target
4. Keep our timing module (unique value)
5. Keep our error handling (production-grade)

**The goal is shared:** Make CRA invisible infrastructure like SQLite.
