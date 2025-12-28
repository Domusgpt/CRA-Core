# CRA Rust Infrastructure Plan

**Vision:** CRA as invisible infrastructure — like SQLite, embedded everywhere, just how things work.

**Date:** 2025-12-28
**Branch:** `claude/design-cra-architecture-WdoAv`
**Status:** Proposal for unified Rust core

---

## Table of Contents

1. [The Infrastructure Vision](#1-the-infrastructure-vision)
2. [Why Current Approaches Fall Short](#2-why-current-approaches-fall-short)
3. [The Dual-Mode Architecture](#3-the-dual-mode-architecture)
4. [Rust Core Design](#4-rust-core-design)
5. [Async Trace Buffering](#5-async-trace-buffering)
6. [Universal Embedding](#6-universal-embedding)
7. [Migration Path](#7-migration-path)
8. [Implementation Phases](#8-implementation-phases)
9. [Success Criteria](#9-success-criteria)

---

## 1. The Infrastructure Vision

### The Goal: Invisible Governance

CRA should be like **SQLite** — you don't think about it, it's just there:

```
Today (CRA as a service):
┌─────────────┐      "Can I do X?"     ┌─────────────┐
│    Agent    │ ─────────────────────▶ │  CRA Server │
│             │ ◀───────────────────── │  (remote)   │
└─────────────┘      50-100ms          └─────────────┘

   Problem: Every governance check is a visible "tool call"
   Problem: Adds latency to every agent action
   Problem: LLM sees CRA as a dependency to work around


Tomorrow (CRA as infrastructure):
┌─────────────────────────────────────────────────────┐
│                  Agent Runtime                       │
│                                                      │
│    Agent Logic ──▶ CRA (embedded) ──▶ Tools         │
│                     │                                │
│                     └── <0.001ms, invisible          │
│                                                      │
└─────────────────────────────────────────────────────┘

   Solution: Governance is instant, in-process
   Solution: LLM never sees CRA (not a "tool")
   Solution: Just how the runtime works
```

### The SQLite Analogy

| SQLite | CRA |
|--------|-----|
| Embedded database | Embedded governance |
| No server needed | No server needed (for resolution) |
| Zero configuration | Zero configuration |
| Works everywhere | Works everywhere |
| ~500KB footprint | Target: <1MB footprint |
| Used by billions | Target: Every AI agent |

### What "Infrastructure" Means

1. **Invisible** — Agents don't know they're being governed
2. **Instant** — Sub-millisecond, never the bottleneck
3. **Universal** — Same code runs in Python, Node, Browser, Edge
4. **Reliable** — Memory-safe, can't crash the host
5. **Embeddable** — Library, not a service (for the hot path)

---

## 2. Why Current Approaches Fall Short

### Python Implementation (Other Branch)

```python
# Current: HTTP call for every resolution
resolution = await cra_client.resolve(request)  # 50-100ms network
```

**Problems:**
- Network latency on every resolution
- Requires running a separate service
- Can't embed in browsers or edge
- Python GIL limits concurrency

### TypeScript Implementation (Our Branch)

```typescript
// Current: In-process but JS runtime overhead
const resolution = await runtime.resolve(request);  // 1-5ms
```

**Problems:**
- JavaScript runtime overhead
- Can't embed in Python (without HTTP)
- V8/Node.js memory footprint
- Not truly universal

### What Both Miss: The Latency Math

```
Agent Task: "Help customer with refund"
├── LLM call #1: Understand request (500ms)
├── CRA resolve: Check permissions (50ms)  ← HTTP
├── Tool call: Look up order (200ms)
├── LLM call #2: Analyze order (500ms)
├── CRA resolve: Check refund policy (50ms)  ← HTTP
├── Tool call: Process refund (300ms)
├── LLM call #3: Confirm to user (500ms)
└── Total CRA overhead: 100ms (5% of task)

At scale: 1M tasks/day × 100ms = 27 CPU-hours wasted on governance latency
```

**With embedded Rust:**
```
CRA resolve: Check permissions (0.01ms)  ← In-process
CRA resolve: Check refund policy (0.01ms)  ← In-process
Total CRA overhead: 0.02ms (0.001% of task)
```

---

## 3. The Dual-Mode Architecture

### The Critical Insight

**Not all operations are equal:**

| Operation | Frequency | Latency Tolerance | Mode |
|-----------|-----------|-------------------|------|
| Resolution | 10-50/task | <1ms required | EMBEDDED |
| Trace Record | 10-50/task | Seconds OK | ASYNC |
| Trace Query | Rare | Seconds OK | HTTP |
| Atlas Load | Once/session | Seconds OK | HTTP or Local |

### The Pattern

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CRA DUAL-MODE                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  HOT PATH (Embedded)                 AUDIT PATH (Centralized)       │
│  ════════════════════                ═════════════════════════      │
│                                                                      │
│  ┌─────────────────────┐             ┌─────────────────────┐        │
│  │   CRA Core (Rust)   │             │   TRACE Ingest      │        │
│  │                     │             │   Service           │        │
│  │  • resolve()        │   batch     │                     │        │
│  │  • evaluate_policy()│ ─────────▶  │  • Receive batches  │        │
│  │  • get_context()    │   async     │  • Validate chains  │        │
│  │                     │   HTTP      │  • Store to DB      │        │
│  │  Runs IN-PROCESS    │             │  • Index for query  │        │
│  │  <0.01ms latency    │             │                     │        │
│  └─────────────────────┘             └──────────┬──────────┘        │
│           │                                     │                    │
│           │                                     ▼                    │
│           │                          ┌─────────────────────┐        │
│           │                          │   TRACE Storage     │        │
│           │                          │   (PostgreSQL/S3)   │        │
│           │                          └─────────────────────┘        │
│           │                                     │                    │
│           ▼                                     ▼                    │
│  ┌─────────────────────┐             ┌─────────────────────┐        │
│  │   Agent executes    │             │   Audit Dashboard   │        │
│  │   with governance   │             │   Replay, Query     │        │
│  └─────────────────────┘             └─────────────────────┘        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Why This Matters for Agents

```
WITHOUT dual-mode:
Agent → HTTP to CRA → Wait → Response → Continue
        └── 50ms ──┘

WITH dual-mode:
Agent → cra.resolve() → Continue    (traces batch in background)
        └── 0.01ms ──┘

The agent never waits for audit logging.
The agent never makes network calls for permission checks.
Governance becomes invisible.
```

---

## 4. Rust Core Design

### Design Principles

1. **Zero-copy where possible** — Avoid allocations in hot path
2. **No async in resolve()** — Sync function, predictable latency
3. **Lock-free trace recording** — Non-blocking event capture
4. **Minimal dependencies** — Small binary, fast compile
5. **C ABI exports** — Universal FFI compatibility

### Crate Structure

```
cra-rust/
├── Cargo.toml                      # Workspace
│
├── cra-core/                       # Main library (what gets embedded)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # Public API
│       │
│       ├── protocol/               # Types (zero deps, no_std compatible)
│       │   ├── mod.rs
│       │   ├── carp.rs             # CARPRequest, CARPResolution
│       │   ├── trace.rs            # TRACEEvent
│       │   └── atlas.rs            # AtlasManifest
│       │
│       ├── engine/                 # Resolution engine
│       │   ├── mod.rs
│       │   ├── resolver.rs         # resolve() - THE HOT PATH
│       │   ├── context.rs          # Context selection
│       │   └── policy.rs           # Policy evaluation
│       │
│       ├── atlas/                  # Atlas management
│       │   ├── mod.rs
│       │   ├── loader.rs           # Load from file/memory
│       │   ├── cache.rs            # LRU cache
│       │   └── validator.rs        # Schema validation
│       │
│       ├── trace/                  # Trace collection
│       │   ├── mod.rs
│       │   ├── collector.rs        # Event collection
│       │   ├── buffer.rs           # Ring buffer
│       │   ├── chain.rs            # Hash chain
│       │   └── sink.rs             # Output destinations
│       │
│       └── ffi/                    # Foreign function interface
│           ├── mod.rs
│           └── c_api.rs            # C ABI for any language
│
├── cra-server/                     # HTTP server (optional component)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── routes/
│       └── middleware/
│
├── cra-ingest/                     # TRACE ingest service
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── receiver.rs
│       └── storage.rs
│
└── bindings/                       # Language bindings
    ├── python/                     # PyO3
    ├── node/                       # napi-rs
    └── wasm/                       # wasm-bindgen
```

### The Hot Path: resolve()

```rust
// cra-core/src/engine/resolver.rs

use crate::protocol::{CARPRequest, CARPResolution, Decision};
use crate::atlas::AtlasCache;
use crate::trace::TraceCollector;

/// The CRA resolver - this is the HOT PATH
///
/// Design constraints:
/// - MUST be synchronous (no async)
/// - MUST complete in <1ms for cached atlases
/// - MUST NOT allocate in common case
/// - MUST NOT block on I/O
pub struct Resolver {
    atlases: AtlasCache,
    trace: TraceCollector,
}

impl Resolver {
    /// Create a new resolver
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            atlases: AtlasCache::new(config.cache_size),
            trace: TraceCollector::new(config.trace),
        }
    }

    /// Resolve a CARP request - THE HOT PATH
    ///
    /// This function:
    /// - Looks up cached atlases (O(1) hash lookup)
    /// - Evaluates policies (O(n) where n = rules, typically <100)
    /// - Selects context blocks (O(m) where m = blocks, typically <50)
    /// - Records trace event (non-blocking queue push)
    ///
    /// Target latency: <0.01ms for cached atlas
    #[inline]
    pub fn resolve(&self, request: &CARPRequest) -> CARPResolution {
        // 1. Record request received (non-blocking)
        self.trace.record_request(request);

        // 2. Get relevant atlases from cache
        let atlases = self.atlases.get_for_request(request);

        // 3. Evaluate policies (deny first, then allow)
        let decision = self.evaluate_policies(request, &atlases);

        // 4. If allowed, select context and actions
        let (context_blocks, allowed_actions) = match &decision {
            Decision::Allow | Decision::Partial { .. } => {
                self.select_context_and_actions(request, &atlases)
            }
            _ => (vec![], vec![]),
        };

        // 5. Build resolution
        let resolution = CARPResolution {
            resolution_id: generate_id(),
            request_id: request.request_id.clone(),
            timestamp: now_iso8601(),
            decision,
            context_blocks,
            allowed_actions,
            denied_actions: vec![], // TODO: populate
            constraints: vec![],
            ttl_seconds: 300,
            trace_id: self.trace.current_trace_id(),
        };

        // 6. Record resolution (non-blocking)
        self.trace.record_resolution(&resolution);

        resolution
    }

    /// Pre-load atlases into cache
    ///
    /// Call this at startup or session init.
    /// This CAN be async since it's not in the hot path.
    pub async fn preload(&self, atlas_ids: &[String]) -> Result<(), AtlasError> {
        for id in atlas_ids {
            let atlas = self.load_atlas(id).await?;
            self.atlases.insert(atlas);
        }
        Ok(())
    }

    #[inline]
    fn evaluate_policies(&self, request: &CARPRequest, atlases: &[&Atlas]) -> Decision {
        // Policy evaluation order:
        // 1. Explicit deny rules (highest priority)
        // 2. Require approval rules
        // 3. Rate limit rules
        // 4. Allow rules
        // 5. Default deny

        for atlas in atlases {
            for policy in &atlas.policies {
                if policy.matches(request) {
                    match policy.effect {
                        Effect::Deny => return Decision::Deny {
                            reason: policy.reason.clone(),
                        },
                        Effect::RequireApproval => return Decision::RequiresApproval {
                            approver: policy.approver.clone(),
                            timeout_seconds: policy.timeout,
                        },
                        _ => continue,
                    }
                }
            }
        }

        // Check for any allow
        for atlas in atlases {
            for policy in &atlas.policies {
                if policy.matches(request) && policy.effect == Effect::Allow {
                    return Decision::Allow;
                }
            }
        }

        // Default: deny
        Decision::Deny {
            reason: "No matching allow policy".into(),
        }
    }

    #[inline]
    fn select_context_and_actions(
        &self,
        request: &CARPRequest,
        atlases: &[&Atlas],
    ) -> (Vec<ContextBlock>, Vec<ActionPermission>) {
        let mut contexts = Vec::with_capacity(16);
        let mut actions = Vec::with_capacity(32);

        for atlas in atlases {
            // Select relevant context blocks
            for block in &atlas.context_packs {
                if self.context_matches(request, block) {
                    contexts.push(block.to_context_block());
                }
            }

            // Select permitted actions
            for action in &atlas.actions {
                if self.action_permitted(request, action) {
                    actions.push(action.to_permission());
                }
            }
        }

        // Sort by priority
        contexts.sort_by_key(|c| std::cmp::Reverse(c.priority));

        (contexts, actions)
    }
}
```

### Memory Layout Optimization

```rust
// Optimize for cache locality in hot path

#[repr(C)]
pub struct CARPRequest {
    // Hot fields first (accessed in every resolve)
    pub request_id: CompactString,      // 24 bytes, inline small strings
    pub operation: Operation,            // 1 byte
    pub _pad1: [u8; 7],                  // alignment
    pub requester: Requester,            // 48 bytes
    pub task: Task,                      // 64 bytes

    // Cold fields (rarely accessed)
    pub atlas_ids: SmallVec<[String; 4]>, // Usually 1-2 atlases
    pub context: Option<Box<HashMap<String, Value>>>, // Heap if present
}

// Total hot path: ~144 bytes, fits in 2-3 cache lines
```

---

## 5. Async Trace Buffering

### The Design

Trace recording MUST NOT block the hot path:

```rust
// cra-core/src/trace/collector.rs

use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;

/// Non-blocking trace collector
///
/// Events are pushed to a lock-free queue and processed
/// by a background thread. The hot path never waits.
pub struct TraceCollector {
    sender: Sender<TraceCommand>,
    trace_id: String,
    sequence: AtomicU64,
    previous_hash: AtomicPtr<String>,
}

enum TraceCommand {
    Record(TRACEEvent),
    Flush,
    SetSink(Box<dyn TraceSink>),
    Shutdown,
}

impl TraceCollector {
    pub fn new(config: TraceConfig) -> Self {
        let (sender, receiver) = channel();

        // Spawn background thread for trace processing
        let buffer_size = config.buffer_size;
        let flush_interval = config.flush_interval;

        thread::spawn(move || {
            Self::background_loop(receiver, buffer_size, flush_interval);
        });

        Self {
            sender,
            trace_id: generate_trace_id(),
            sequence: AtomicU64::new(0),
            previous_hash: AtomicPtr::new(Box::into_raw(Box::new("genesis".to_string()))),
        }
    }

    /// Record an event - NON-BLOCKING
    ///
    /// This pushes to an unbounded channel and returns immediately.
    /// The background thread handles serialization and batching.
    #[inline]
    pub fn record<T: Serialize>(&self, event_type: TRACEEventType, payload: &T) {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);

        // Get previous hash (lock-free read)
        let prev_hash_ptr = self.previous_hash.load(Ordering::Acquire);
        let prev_hash = unsafe { &*prev_hash_ptr };

        // Compute new hash
        let payload_bytes = serde_json::to_vec(payload).unwrap();
        let event_hash = compute_hash(&event_type, &payload_bytes, prev_hash);

        // Update previous hash (lock-free swap)
        let new_hash = Box::into_raw(Box::new(event_hash.clone()));
        self.previous_hash.store(new_hash, Ordering::Release);

        // Build event
        let event = TRACEEvent {
            trace_version: "1.0".into(),
            event_id: generate_event_id(),
            trace_id: self.trace_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: None,
            session_id: self.trace_id.clone(),
            sequence: seq,
            timestamp: now_iso8601(),
            event_type,
            payload: serde_json::from_slice(&payload_bytes).unwrap(),
            event_hash,
            previous_event_hash: prev_hash.clone(),
        };

        // Non-blocking send (unbounded channel)
        let _ = self.sender.send(TraceCommand::Record(event));
    }

    /// Force flush - use at session end
    pub fn flush(&self) {
        let _ = self.sender.send(TraceCommand::Flush);
    }

    fn background_loop(
        receiver: Receiver<TraceCommand>,
        buffer_size: usize,
        flush_interval: Duration,
    ) {
        let mut buffer = Vec::with_capacity(buffer_size);
        let mut sink: Box<dyn TraceSink> = Box::new(NoopSink);
        let mut last_flush = Instant::now();

        loop {
            // Non-blocking receive with timeout
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(TraceCommand::Record(event)) => {
                    buffer.push(event);

                    // Flush if buffer full
                    if buffer.len() >= buffer_size {
                        sink.write_batch(&buffer);
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
                Ok(TraceCommand::Flush) => {
                    if !buffer.is_empty() {
                        sink.write_batch(&buffer);
                        buffer.clear();
                    }
                    last_flush = Instant::now();
                }
                Ok(TraceCommand::SetSink(new_sink)) => {
                    // Flush old sink first
                    if !buffer.is_empty() {
                        sink.write_batch(&buffer);
                        buffer.clear();
                    }
                    sink = new_sink;
                }
                Ok(TraceCommand::Shutdown) => {
                    // Final flush
                    if !buffer.is_empty() {
                        sink.write_batch(&buffer);
                    }
                    break;
                }
                Err(_) => {
                    // Timeout - check if we should flush based on time
                    if last_flush.elapsed() >= flush_interval && !buffer.is_empty() {
                        sink.write_batch(&buffer);
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }
}

/// Trait for trace output destinations
pub trait TraceSink: Send + 'static {
    fn write_batch(&self, events: &[TRACEEvent]);
}

/// HTTP sink - sends batches to ingest service
pub struct HttpSink {
    endpoint: String,
    client: reqwest::blocking::Client,
    compression: bool,
}

impl TraceSink for HttpSink {
    fn write_batch(&self, events: &[TRACEEvent]) {
        if events.is_empty() {
            return;
        }

        let payload = serde_json::to_vec(events).unwrap();

        let body = if self.compression {
            zstd::encode_all(&payload[..], 3).unwrap()
        } else {
            payload
        };

        // Fire and forget - don't block on response
        let _ = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Content-Encoding", if self.compression { "zstd" } else { "identity" })
            .body(body)
            .send();
    }
}

/// File sink - append to local file (for development)
pub struct FileSink {
    path: PathBuf,
}

impl TraceSink for FileSink {
    fn write_batch(&self, events: &[TRACEEvent]) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .unwrap();

        for event in events {
            serde_json::to_writer(&mut file, event).unwrap();
            writeln!(file).unwrap();
        }
    }
}
```

### Trace Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                      TRACE DATA FLOW                                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   Hot Path (sync)              Background Thread         Remote      │
│   ══════════════               ═════════════════         ══════      │
│                                                                      │
│   resolve()                                                          │
│      │                                                               │
│      ├─▶ trace.record()                                              │
│      │      │                                                        │
│      │      └─▶ channel.send() ─────▶ ┌──────────────┐              │
│      │           (non-blocking)       │ Ring Buffer  │              │
│      │                                │              │              │
│      │                                │ [e1,e2,e3..] │              │
│      │                                └──────┬───────┘              │
│      │                                       │                       │
│      │                                       │ (buffer full OR       │
│      │                                       │  timer elapsed)       │
│      │                                       ▼                       │
│      │                                ┌──────────────┐              │
│      │                                │ Serialize +  │              │
│      │                                │ Compress     │              │
│      │                                └──────┬───────┘              │
│      │                                       │                       │
│      │                                       │ HTTP POST             │
│      │                                       ▼                       │
│      │                                ┌──────────────┐   ┌────────┐ │
│      │                                │ TraceSink    │──▶│ Ingest │ │
│      │                                │ (HTTP/File)  │   │ Service│ │
│      │                                └──────────────┘   └────────┘ │
│      │                                                               │
│      ▼                                                               │
│   return resolution                                                  │
│   (agent continues immediately)                                      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘

Timeline:
─────────────────────────────────────────────────────────────────────▶
  0μs        10μs                      100ms                    5s
   │          │                          │                       │
   │          │                          │                       │
   resolve()  │                          │                       │
   starts     │                          │                       │
              │                          │                       │
              resolve()                  │                       │
              returns                    │                       │
              (agent continues)          │                       │
                                         │                       │
                                         buffer                  │
                                         flushes                 │
                                         (async)                 │
                                                                 │
                                                                 traces
                                                                 arrive at
                                                                 ingest
```

---

## 6. Universal Embedding

### Target Platforms

| Platform | Binding | Method | Size Target |
|----------|---------|--------|-------------|
| Native (Linux/macOS/Win) | Direct | Rust binary | <5MB |
| Python | PyO3 | Native extension | <3MB wheel |
| Node.js | napi-rs | Native addon | <3MB |
| Browser | wasm-bindgen | WASM module | <500KB |
| Cloudflare Workers | wasm32-unknown | WASM | <500KB |
| Deno | wasm-bindgen | WASM | <500KB |
| Go | cgo | C FFI | Linked |
| Ruby | FFI gem | C FFI | Linked |
| Any language | C ABI | dlopen/LoadLibrary | <5MB |

### Python Binding (PyO3)

```rust
// bindings/python/src/lib.rs

use pyo3::prelude::*;
use cra_core::{Resolver, ResolverConfig};

#[pyclass]
struct CRAResolver {
    inner: Resolver,
}

#[pymethods]
impl CRAResolver {
    #[new]
    #[pyo3(signature = (cache_size=100, trace_endpoint=None))]
    fn new(cache_size: usize, trace_endpoint: Option<String>) -> PyResult<Self> {
        let config = ResolverConfig {
            cache_size,
            trace: TraceConfig {
                sink: trace_endpoint.map(|e| TraceSinkConfig::Http { endpoint: e })
                    .unwrap_or(TraceSinkConfig::Noop),
                ..Default::default()
            },
        };

        Ok(Self {
            inner: Resolver::new(config),
        })
    }

    /// Resolve a CARP request
    ///
    /// This is the main entry point. It's synchronous and fast (<1ms).
    fn resolve(&self, request: &PyDict, py: Python<'_>) -> PyResult<PyObject> {
        // Convert Python dict to Rust struct
        let request: CARPRequest = depythonize(request)?;

        // Call Rust resolver (fast!)
        let resolution = self.inner.resolve(&request);

        // Convert back to Python dict
        pythonize(py, &resolution)
    }

    /// Preload atlases (async-friendly)
    fn preload_atlas<'py>(&self, py: Python<'py>, atlas_path: &str) -> PyResult<&'py PyAny> {
        let path = atlas_path.to_string();
        let resolver = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            resolver.preload_from_file(&path).await?;
            Ok(())
        })
    }

    /// Flush pending traces
    fn flush(&self) {
        self.inner.trace().flush();
    }
}

#[pymodule]
fn cra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<CRAResolver>()?;
    Ok(())
}
```

**Python Usage:**
```python
from cra import CRAResolver

# Create resolver (starts background trace thread)
resolver = CRAResolver(
    cache_size=100,
    trace_endpoint="http://localhost:8421/ingest"
)

# Preload atlas at startup
await resolver.preload_atlas("./atlases/customer-support/atlas.json")

# Resolve - THIS IS FAST (<0.01ms)
resolution = resolver.resolve({
    "carp_version": "1.0",
    "request_id": "req-123",
    "operation": "resolve",
    "requester": {"agent_id": "support-bot", "session_id": "sess-456"},
    "task": {"goal": "Process refund for order #789"}
})

if resolution["decision"]["type"] == "allow":
    # Agent can proceed
    for action in resolution["allowed_actions"]:
        print(f"Can do: {action['action_id']}")

# At session end
resolver.flush()
```

### WASM Binding

```rust
// bindings/wasm/src/lib.rs

use wasm_bindgen::prelude::*;
use cra_core::{Resolver, ResolverConfig};

#[wasm_bindgen]
pub struct WasmResolver {
    inner: Resolver,
}

#[wasm_bindgen]
impl WasmResolver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // WASM uses in-memory trace buffer (no HTTP from browser)
        let config = ResolverConfig {
            cache_size: 50,  // Smaller for browser
            trace: TraceConfig {
                sink: TraceSinkConfig::Memory,
                buffer_size: 100,
                ..Default::default()
            },
        };

        Self {
            inner: Resolver::new(config),
        }
    }

    /// Load atlas from JSON string
    #[wasm_bindgen]
    pub fn load_atlas(&mut self, json: &str) -> Result<(), JsError> {
        let atlas: AtlasManifest = serde_json::from_str(json)?;
        self.inner.atlases().insert(atlas);
        Ok(())
    }

    /// Resolve a request
    #[wasm_bindgen]
    pub fn resolve(&self, request: JsValue) -> Result<JsValue, JsError> {
        let request: CARPRequest = serde_wasm_bindgen::from_value(request)?;
        let resolution = self.inner.resolve(&request);
        serde_wasm_bindgen::to_value(&resolution).map_err(Into::into)
    }

    /// Get buffered traces (for sending to server)
    #[wasm_bindgen]
    pub fn drain_traces(&self) -> Result<JsValue, JsError> {
        let events = self.inner.trace().drain();
        serde_wasm_bindgen::to_value(&events).map_err(Into::into)
    }
}
```

**Browser Usage:**
```typescript
import init, { WasmResolver } from '@cra/wasm';

// Initialize WASM module
await init();

// Create resolver
const resolver = new WasmResolver();

// Load atlas (fetched via HTTP)
const atlasJson = await fetch('/atlases/support.json').then(r => r.text());
resolver.load_atlas(atlasJson);

// Resolve - runs entirely client-side, instant
const resolution = resolver.resolve({
  carp_version: '1.0',
  request_id: crypto.randomUUID(),
  operation: 'resolve',
  requester: { agent_id: 'browser-agent', session_id: sessionId },
  task: { goal: 'Help user with question' },
});

// Periodically send traces to server
setInterval(() => {
  const traces = resolver.drain_traces();
  if (traces.length > 0) {
    fetch('/api/traces', {
      method: 'POST',
      body: JSON.stringify(traces),
    });
  }
}, 5000);
```

---

## 7. Migration Path

### From TypeScript (Our Branch)

```
Current TypeScript                    Target Rust
═══════════════════                   ════════════

packages/protocol/     ──────────▶    cra-core/src/protocol/
  (types only)                        (same types in Rust)

packages/runtime/      ──────────▶    cra-core/src/engine/
  CRARuntime class                    Resolver struct

packages/trace/        ──────────▶    cra-core/src/trace/
  TRACECollector                      TraceCollector

packages/atlas/        ──────────▶    cra-core/src/atlas/
  AtlasLoader                         AtlasCache + Loader

packages/server/       ──────────▶    cra-server/
  Express routes                      Axum routes

packages/cli/          ──────────▶    cra-cli/ (Rust)
  Commander.js                        OR keep TS, call Rust

@cra/client (NEW)      ◀──────────    Wraps Rust via WASM/napi
  TypeScript SDK
```

### Migration Steps

**Phase 1: Core in Rust**
```
1. Implement cra-core with same types as packages/protocol
2. Implement resolve() matching CRARuntime.resolve()
3. Implement TraceCollector matching TRACECollector
4. Write conformance tests against specs/
```

**Phase 2: Bindings**
```
5. Create Python binding (PyO3)
6. Create Node.js binding (napi-rs)
7. Create WASM binding (wasm-bindgen)
8. Test bindings against TypeScript test suite
```

**Phase 3: SDK Refactor**
```
9. Create @cra/client that loads WASM (browser) or native (Node)
10. Deprecate packages/runtime (replaced by Rust)
11. Keep packages/cli (calls Rust underneath)
12. Keep packages/ui (unchanged)
```

**Phase 4: Server**
```
13. Implement cra-server in Rust (optional)
14. OR keep packages/server calling Rust core
15. Implement cra-ingest for centralized traces
```

---

## 8. Implementation Phases

### Phase 1: Foundation (Week 1-2)

| Task | Output |
|------|--------|
| Set up Rust workspace | `Cargo.toml`, toolchain |
| Implement protocol types | `cra-core/src/protocol/` |
| Implement hash chain | `cra-core/src/trace/chain.rs` |
| Basic test suite | `tests/` |
| CI/CD | GitHub Actions |

**Deliverable:** Types compile, hash chain works

### Phase 2: Engine (Week 3-4)

| Task | Output |
|------|--------|
| Atlas loader | `cra-core/src/atlas/` |
| LRU cache | `cra-core/src/atlas/cache.rs` |
| Policy evaluator | `cra-core/src/engine/policy.rs` |
| Resolver | `cra-core/src/engine/resolver.rs` |
| Integration tests | Compare to TypeScript output |

**Deliverable:** `resolve()` works, matches TypeScript behavior

### Phase 3: Trace Collector (Week 5-6)

| Task | Output |
|------|--------|
| Ring buffer | `cra-core/src/trace/buffer.rs` |
| Background thread | `cra-core/src/trace/collector.rs` |
| HTTP sink | `cra-core/src/trace/sink.rs` |
| File sink | Development mode |
| Non-blocking tests | Verify no hot-path blocking |

**Deliverable:** Traces collect async, don't block resolve()

### Phase 4: Bindings (Week 7-8)

| Task | Output |
|------|--------|
| C FFI | `cra-core/src/ffi/c_api.rs` |
| Python binding | `bindings/python/` |
| Node.js binding | `bindings/node/` |
| WASM binding | `bindings/wasm/` |
| Binding tests | All pass conformance |

**Deliverable:** Works from Python, Node, Browser

### Phase 5: Integration (Week 9-10)

| Task | Output |
|------|--------|
| @cra/client SDK | TypeScript wrapper |
| WASM loader | Browser integration |
| Native loader | Node.js integration |
| Migration guide | Documentation |

**Deliverable:** TypeScript code uses Rust underneath

### Phase 6: Production (Week 11-12)

| Task | Output |
|------|--------|
| Benchmarks | <0.01ms resolution |
| Fuzzing | Security testing |
| Documentation | Complete API docs |
| Release | v1.0.0 |

**Deliverable:** Production-ready Rust core

---

## 9. Success Criteria

### Performance

| Metric | Target | How to Measure |
|--------|--------|----------------|
| resolve() latency (cached) | <0.01ms | Benchmark, p99 |
| resolve() latency (cold) | <1ms | First call after preload |
| trace.record() latency | <1μs | Must not block |
| Memory per session | <1MB | Measure with 1000 sessions |
| Binary size (native) | <5MB | `ls -la target/release/` |
| WASM size | <500KB | `wc -c pkg/*.wasm` |

### Correctness

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Conformance tests | 100% pass | `specs/conformance/` |
| Hash chain validity | 100% | Chain verification |
| Policy evaluation | Match Python | Compare outputs |
| TypeScript parity | 100% | Same test suite |

### Compatibility

| Platform | Binding | Status |
|----------|---------|--------|
| Python 3.9+ | PyO3 | Required |
| Node.js 18+ | napi-rs | Required |
| Browser (Chrome, FF, Safari) | WASM | Required |
| Cloudflare Workers | WASM | Required |
| Deno | WASM | Nice to have |
| Go | cgo | Nice to have |

---

## Summary

This plan transforms CRA from "a service agents call" into "invisible infrastructure agents run within."

**Key principles:**
1. **Dual-mode:** Resolution is embedded, traces are centralized
2. **Non-blocking:** Hot path never waits for I/O
3. **Universal:** Same Rust core runs everywhere
4. **Invisible:** Governance happens without agents knowing

**The end state:**
```python
# This "just works" - no HTTP calls, no visible governance
from cra import CRAResolver

resolver = CRAResolver()
resolution = resolver.resolve(request)  # <0.01ms, in-process

# Traces flow to central storage automatically
# Agent never waits, never sees CRA as a "tool"
# Governance is infrastructure, not a feature
```

This is how CRA becomes the SQLite of AI governance.
