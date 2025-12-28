# CRA Rust Core - Main Development Branch Prompt

**To:** Claude session on `claude/cra-rust-refactor-XBcJV`
**From:** Coordination across all CRA branches
**Date:** 2025-12-28
**Subject:** You are now the MAIN development branch

---

## Your Role

**You are the main CRA development branch.** Your working Rust implementation (90 tests) is the foundation. Other branches contribute designs and specs that you integrate.

---

## Development Session Logging

**IMPORTANT:** Document all development sessions with timestamps.

Create/update `docs/DEV_LOG.md` with entries like:

```markdown
## 2025-12-28 - Session: [Brief Description]

### Changes Made
- ...

### Decisions
- ...

### Next Steps
- ...

### Branch State
- Tests: X passing
- resolve() latency: Xµs
```

---

## What to Pull From Other Branches

### 1. From `claude/plan-cra-platform-WoXIo` - Protocol Specs

```bash
git fetch origin claude/plan-cra-platform-WoXIo

# Pull the specs directory (JSON schemas, conformance tests)
git checkout origin/claude/plan-cra-platform-WoXIo -- specs/
```

Contains:
- `specs/PROTOCOL.md` - Master specification
- `specs/schemas/*.json` - JSON Schema validation
- `specs/conformance/` - Golden trace tests
- `specs/openapi.yaml` - HTTP API spec

### 2. From `claude/design-cra-architecture-WdoAv` - Architecture Designs

```bash
git fetch origin claude/design-cra-architecture-WdoAv

# Review (don't pull directly - adopt the designs):
git show origin/claude/design-cra-architecture-WdoAv:docs/RUST_INFRASTRUCTURE_PLAN.md
```

**Key designs to adopt:**

#### A. Async Ring Buffer for Traces

Your current trace collector is sync. Adopt this pattern:

```rust
// cra-core/src/trace/ring_buffer.rs

use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem::MaybeUninit;

/// Lock-free ring buffer for non-blocking trace recording
pub struct RingBuffer<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T: Clone, const N: usize> RingBuffer<T, N> {
    /// Push without blocking - returns false if full
    pub fn try_push(&self, item: T) -> bool {
        // Lock-free implementation
    }

    /// Drain all items for batch processing
    pub fn drain(&self) -> Vec<T> {
        // ...
    }
}
```

#### B. Background Trace Sender

```rust
// cra-core/src/trace/sender.rs

pub struct BackgroundSender {
    sender: mpsc::Sender<TraceCommand>,
    handle: Option<JoinHandle<()>>,
}

enum TraceCommand {
    Record(TRACEEvent),
    Flush,
    Shutdown,
}

impl BackgroundSender {
    pub fn new(sink: Box<dyn TraceSink>) -> Self {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            let mut buffer = Vec::with_capacity(100);
            loop {
                match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(TraceCommand::Record(event)) => {
                        buffer.push(event);
                        if buffer.len() >= 100 {
                            sink.write_batch(&buffer);
                            buffer.clear();
                        }
                    }
                    Ok(TraceCommand::Flush) => {
                        sink.write_batch(&buffer);
                        buffer.clear();
                    }
                    Ok(TraceCommand::Shutdown) => break,
                    Err(_) => {
                        // Timeout - flush if we have events
                        if !buffer.is_empty() {
                            sink.write_batch(&buffer);
                            buffer.clear();
                        }
                    }
                }
            }
        });

        Self { sender: tx, handle: Some(handle) }
    }

    /// Non-blocking record - returns immediately
    pub fn record(&self, event: TRACEEvent) {
        let _ = self.sender.send(TraceCommand::Record(event));
    }
}
```

#### C. Lazy Hash Computation

Your current implementation computes hashes inline (adds ~90µs). Defer to batch time:

```rust
// Record without hash
pub fn record_lazy(&self, event_type: EventType, payload: Value) {
    self.buffer.push(UnhashedEvent { event_type, payload, timestamp: now() });
}

// Compute hashes when batching
pub fn finalize_batch(&self) -> Vec<TRACEEvent> {
    let mut prev_hash = self.last_hash.clone();
    self.buffer.drain(..).map(|e| {
        let event = e.with_hash(&prev_hash);
        prev_hash = event.event_hash.clone();
        event
    }).collect()
}
```

This should reduce resolve() from 134µs → <50µs.

---

## Repository Structure (Target)

```
CRA-Core/
├── Cargo.toml                 # Workspace root
├── specs/                     # FROM plan branch
│   ├── PROTOCOL.md
│   ├── schemas/
│   ├── conformance/
│   └── openapi.yaml
│
├── cra-core/                  # YOUR implementation
│   ├── src/
│   │   ├── carp/              # CARP resolver
│   │   ├── trace/             # TRACE collector + ring buffer
│   │   ├── atlas/             # Atlas loader
│   │   ├── timing/            # YOUR timing module (keep!)
│   │   ├── storage/           # YOUR storage backends (keep!)
│   │   └── error.rs           # YOUR error handling (keep!)
│   ├── benches/
│   └── tests/
│
├── cra-server/                # HTTP server
├── cra-python/                # PyO3 bindings
├── cra-node/                  # napi-rs bindings
├── cra-wasm/                  # WASM bindings
│
├── docs/
│   ├── DEV_LOG.md             # Session log (NEW - maintain this!)
│   ├── ARCHITECTURE.md
│   └── API.md
│
└── examples/
```

---

## What to Keep (Your Unique Contributions)

**DO NOT remove these - they're valuable:**

1. **Timing module** (`cra-core/src/timing/`)
   - HeartbeatConfig
   - SessionTTLConfig
   - SlidingWindowRateLimiter
   - TimerBackend trait

2. **Storage backends** (`cra-core/src/storage/`)
   - InMemoryStorage
   - FileStorage
   - AsyncStorageBackend trait

3. **Error handling** (`cra-core/src/error.rs`)
   - ErrorCategory enum
   - HTTP status codes
   - JSON error responses

4. **Python bindings** with proper objects
   - CARPResolution class
   - AllowedAction class
   - TRACEEvent class

---

## Priority Tasks

### Immediate
1. [ ] Pull `specs/` from plan branch
2. [ ] Add `docs/DEV_LOG.md` and start logging
3. [ ] Implement ring buffer for traces
4. [ ] Add background sender thread

### Short-term
5. [ ] Implement lazy hash computation
6. [ ] Run conformance tests from `specs/conformance/`
7. [ ] Optimize resolve() to <50µs
8. [ ] Complete WASM bindings with `drain_traces()`

### Medium-term
9. [ ] Add HTTP sink for centralized trace collection
10. [ ] Complete Node.js bindings
11. [ ] Add benchmarking CI
12. [ ] Write migration guide for TypeScript users

---

## Reference Branches

| Branch | Use For |
|--------|---------|
| `claude/plan-cra-platform-WoXIo` | `specs/`, Python reference, MINOOTS integration |
| `claude/design-cra-architecture-WdoAv` | Architecture docs, TypeScript SDK (future) |

These branches will be maintained as references. When you need:
- Protocol clarification → check plan branch's `specs/`
- Architecture rationale → check design branch's docs
- TypeScript patterns → check design branch's `packages/`

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| resolve() latency | 134µs | <50µs |
| trace.record() | ~15µs (blocking) | <1µs (non-blocking) |
| Tests passing | 90 | 100+ |
| Conformance tests | 7 | All from specs/ |
| Binary size | ~800KB | <1MB |
| WASM size | ? | <500KB |

---

## Commands to Get Started

```bash
# Ensure you're on the rust-refactor branch
git checkout claude/cra-rust-refactor-XBcJV

# Pull specs from plan branch
git fetch origin claude/plan-cra-platform-WoXIo
git checkout origin/claude/plan-cra-platform-WoXIo -- specs/

# Commit the specs
git add specs/
git commit -m "Add protocol specs from plan branch"

# Create dev log
cat > docs/DEV_LOG.md << 'EOF'
# CRA Development Log

## 2025-12-28 - Branch designated as main development

### Status
- Received specs/ from plan branch
- 90 tests passing
- resolve() at 134µs

### Next Steps
- Implement async ring buffer
- Add background trace sender
- Optimize to <50µs
EOF

git add docs/DEV_LOG.md
git commit -m "Add development log, designate as main branch"
git push
```

---

## Summary

**You are the main branch now.** Your Rust implementation is the foundation. Integrate:
- `specs/` from plan branch (pull it)
- Async trace designs from design branch (implement them)
- Keep your timing, storage, and error handling (they're good)

Document everything with timestamps. Other branches are now references.

Build CRA as invisible infrastructure. 🦀
