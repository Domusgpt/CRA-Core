# CRA Unified Architecture

**Date:** 2025-12-28
**Purpose:** Holistic design integrating lock-free traces, async processing, and minoots timer system

---

## Design Principles

1. **Hot path is sacred** - resolve() must be <10µs, no blocking operations
2. **Defer expensive work** - hash computation, I/O happen in background
3. **Timers are first-class** - not an afterthought, integrated from the start
4. **Async is optional** - core is sync for FFI simplicity, async wraps it

---

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Application Layer                                   │
│  Python (PyO3) │ Node.js (napi-rs) │ WASM (wasm-bindgen) │ Rust Native      │
└────────────────┬────────────────────┬───────────────────┬───────────────────┘
                 │                    │                   │
                 ▼                    ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CRA Public API (cra-core/src/lib.rs)                      │
│                                                                              │
│  Resolver::resolve()    - sync, <10µs target                                │
│  Resolver::execute()    - sync, validates and traces                        │
│  AsyncRuntime           - async wrapper for swarms (feature-gated)          │
└────────────────┬────────────────────────────────────────────────────────────┘
                 │
     ┌───────────┴───────────────────────────┐
     ▼                                       ▼
┌────────────────────┐          ┌─────────────────────────────────────────────┐
│   Policy Engine    │          │              Trace Pipeline                  │
│                    │          │                                              │
│ • Pattern matching │          │  ┌─────────────────────────────────────┐    │
│ • Deny/Allow/Rate  │   push   │  │      RingBuffer<RawEvent, 4096>     │    │
│ • Constraints      │─────────▶│  │  • Lock-free (crossbeam)            │    │
│ • ~25µs eval       │          │  │  • No hash computation              │    │
└────────────────────┘          │  │  • Backpressure aware               │    │
                                │  └──────────────────┬──────────────────┘    │
                                │                     │ drain (background)    │
                                │                     ▼                       │
                                │  ┌─────────────────────────────────────┐    │
                                │  │         TraceProcessor              │    │
                                │  │  • Worker thread(s)                 │    │
                                │  │  • Batch hash computation           │    │
                                │  │  • Chain events                     │    │
                                │  │  • Forward to storage               │    │
                                │  └──────────────────┬──────────────────┘    │
                                │                     │                       │
                                │                     ▼                       │
                                │  ┌─────────────────────────────────────┐    │
                                │  │        Storage Backends             │    │
                                │  │  • InMemoryStorage (tests)          │    │
                                │  │  • FileStorage (JSONL)              │    │
                                │  │  • AsyncStorageBackend (DB/Kafka)   │    │
                                │  └─────────────────────────────────────┘    │
                                └─────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                        Timer System (minoots integration)                    │
│                                                                              │
│  TimerBackend trait                                                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐              │
│  │ MinootsBackend  │  │ TokioBackend    │  │ StdBackend      │              │
│  │ (Horology Kern) │  │ (tokio::time)   │  │ (std::thread)   │              │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘              │
│           │                    │                    │                        │
│           └────────────────────┴────────────────────┘                        │
│                                │                                             │
│                                ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      TimerManager                                   │    │
│  │  • Heartbeat emission (configurable interval)                       │    │
│  │  • Session TTL management (idle timeout, max lifetime)              │    │
│  │  • Rate limit window tracking (sliding window reset)                │    │
│  │  • Trace batch flush scheduling                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Key Data Structures

### RawEvent (unhashed)

```rust
/// Event before hash computation - used in ring buffer
pub struct RawEvent {
    pub session_id: String,
    pub trace_id: String,
    pub event_type: EventType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}
```

### ChainedEvent (after processing)

```rust
/// Fully processed event with hash chain
pub struct TRACEEvent {
    // ... all existing fields ...
    pub sequence: u64,
    pub event_hash: String,
    pub previous_event_hash: String,
}
```

### RingBuffer

```rust
/// Lock-free ring buffer for trace events
pub struct TraceRingBuffer {
    buffer: crossbeam::queue::ArrayQueue<RawEvent>,
    dropped: AtomicU64,  // Counter for monitoring
}

impl TraceRingBuffer {
    /// Push event - never blocks, may drop if full
    pub fn push(&self, event: RawEvent) -> bool;

    /// Drain up to N events for processing
    pub fn drain(&self, max: usize) -> Vec<RawEvent>;

    /// Check if buffer is getting full (for backpressure)
    pub fn pressure(&self) -> f32;
}
```

### TraceProcessor

```rust
/// Background worker for hash computation and storage
pub struct TraceProcessor {
    buffer: Arc<TraceRingBuffer>,
    storage: Arc<dyn StorageBackend>,
    chains: RwLock<HashMap<String, ChainState>>,
}

struct ChainState {
    sequence: u64,
    last_hash: String,
}

impl TraceProcessor {
    /// Run the processor (call in background thread)
    pub fn run(&self, shutdown: Receiver<()>);

    /// Process a batch of events
    fn process_batch(&self, events: Vec<RawEvent>);
}
```

---

## Timer Integration with minoots

The timing module becomes central, not peripheral:

```rust
/// Timer manager that coordinates all CRA timing needs
pub struct TimerManager<B: TimerBackend> {
    backend: B,
    heartbeat_config: HeartbeatConfig,
    session_ttl_config: SessionTTLConfig,
    trace_flush_interval: Duration,
}

impl<B: TimerBackend> TimerManager<B> {
    /// Start all managed timers
    pub fn start(&self, handler: impl TimerHandler) -> Result<()> {
        // Schedule heartbeat
        self.backend.schedule_repeating(
            "cra:heartbeat",
            self.heartbeat_config.interval,
            TimerEvent::Heartbeat { session_id: "*".into() },
        )?;

        // Schedule trace flush
        self.backend.schedule_repeating(
            "cra:trace-flush",
            self.trace_flush_interval,
            TimerEvent::TraceBatchFlush,
        )?;

        Ok(())
    }

    /// Track a new session for TTL management
    pub fn track_session(&self, session_id: &str) -> Result<()>;

    /// Reset session idle timer (call on activity)
    pub fn touch_session(&self, session_id: &str) -> Result<()>;
}
```

### minoots Horology Kernel Integration

```rust
/// TimerBackend implementation using minoots Horology Kernel
pub struct MinootsTimerBackend {
    horology: HorologyClient,  // From minoots-timer-system
}

impl TimerBackend for MinootsTimerBackend {
    fn schedule_once(&self, id: &str, delay: Duration, event: TimerEvent) -> Result<()> {
        // Use Horology Kernel's precise timing
        self.horology.schedule(HorologyTimer {
            id: id.to_string(),
            delay_ms: delay.as_millis() as u64,
            callback_data: serde_json::to_value(&event)?,
            repeat: false,
        })
    }

    fn schedule_repeating(&self, id: &str, interval: Duration, event: TimerEvent) -> Result<()> {
        self.horology.schedule(HorologyTimer {
            id: id.to_string(),
            delay_ms: interval.as_millis() as u64,
            callback_data: serde_json::to_value(&event)?,
            repeat: true,
        })
    }

    // ... other methods
}
```

---

## Performance Targets

| Operation | Current | Target | How |
|-----------|---------|--------|-----|
| resolve() | 134µs | <10µs | Defer hash to background |
| trace.emit() | 15µs | <1µs | Lock-free ring buffer push |
| Session creation | ~50µs | ~20µs | Lazy initialization |
| Hash computation | 15µs/event | Same, but background | Batch processing |

---

## Module Organization

```
cra-core/src/
├── lib.rs                 # Public API re-exports
├── carp/
│   ├── mod.rs
│   ├── resolver.rs        # Core resolver (sync, fast)
│   ├── request.rs
│   ├── resolution.rs
│   └── policy.rs
├── trace/
│   ├── mod.rs
│   ├── event.rs           # TRACEEvent, EventType
│   ├── raw.rs             # NEW: RawEvent (unhashed)
│   ├── buffer.rs          # NEW: Lock-free ring buffer
│   ├── processor.rs       # NEW: Background processor
│   ├── collector.rs       # Updated: Uses ring buffer
│   ├── chain.rs
│   └── replay.rs
├── timing/
│   ├── mod.rs             # TimerBackend trait, configs
│   ├── manager.rs         # NEW: TimerManager
│   ├── backends/
│   │   ├── mod.rs
│   │   ├── minoots.rs     # NEW: MinootsTimerBackend
│   │   ├── tokio.rs       # NEW: TokioTimerBackend
│   │   └── std.rs         # NEW: StdTimerBackend
│   ├── heartbeat.rs
│   ├── rate_limit.rs
│   └── session_ttl.rs
├── storage/
│   ├── mod.rs
│   ├── memory.rs
│   ├── file.rs
│   └── null.rs
├── runtime/               # Feature: async-runtime
│   ├── mod.rs
│   └── async_storage.rs
├── atlas/
│   └── ...
└── error.rs
```

---

## Implementation Order

1. **Phase 1: Lock-free buffer** (immediate performance win)
   - Add `crossbeam` dependency
   - Implement `RawEvent` and `TraceRingBuffer`
   - Update `TraceCollector` to use ring buffer
   - Add `TraceProcessor` background worker

2. **Phase 2: Timer integration**
   - Add `TimerManager`
   - Implement `TokioTimerBackend` (for async-runtime feature)
   - Implement `StdTimerBackend` (for sync usage)
   - Add minoots integration scaffold

3. **Phase 3: Storage unification**
   - Update `TraceProcessor` to use `StorageBackend` trait
   - Add batched writes for efficiency
   - Integrate with `AsyncStorageBackend` for async-runtime

4. **Phase 4: AsyncRuntime updates**
   - Update `AsyncRuntime` to use new trace system
   - Add proper backpressure handling
   - Integrate with `TimerManager`

---

## Backward Compatibility

- `TraceCollector::emit()` signature unchanged (internally uses ring buffer)
- `Resolver::resolve()` signature unchanged (faster)
- All existing tests continue to work
- New functionality is additive

---

## Dependencies

```toml
[dependencies]
crossbeam = "0.8"       # Lock-free data structures

[features]
async-runtime = ["tokio", "async-trait", "parking_lot", "num_cpus"]
minoots = []            # Enable minoots timer backend
```

---

## Conclusion

This architecture:
1. Makes hot path sacred (<10µs resolve target)
2. Integrates minoots timing as first-class citizen
3. Keeps sync core for FFI simplicity
4. Adds async as optional wrapper layer
5. Unifies trace processing in background worker
