# CRA Rust Core Implementation Plan

**Version:** 0.1 Draft
**Date:** 2025-12-28
**Status:** Proposal

---

## Executive Summary

This plan defines a **dual-mode CRA architecture** with a Rust core that can be:
1. **Embedded** directly in agent processes (zero-latency resolution)
2. **Accessed via HTTP** for simple integration and centralized deployments

The key insight: **Resolution is local, Traces are centralized.**

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CRA Ecosystem                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                      Rust Core (cra-core)                          │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │ │
│  │  │ cra-protocol │ │ cra-engine   │ │ cra-policy   │ │ cra-trace  │ │ │
│  │  │ (types)      │ │ (resolver)   │ │ (evaluator)  │ │ (buffer)   │ │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐               │ │
│  │  │ cra-atlas    │ │ cra-storage  │ │ cra-crypto   │               │ │
│  │  │ (loader)     │ │ (abstract)   │ │ (hash chain) │               │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘               │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                    │                                     │
│            ┌───────────────────────┼───────────────────────┐            │
│            ▼                       ▼                       ▼            │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐      │
│  │   Native Binary  │  │   WASM Module    │  │  Language Binds  │      │
│  │   (cra-server)   │  │   (cra.wasm)     │  │  (PyO3, napi)    │      │
│  │                  │  │                  │  │                  │      │
│  │  • HTTP API      │  │  • Browser       │  │  • Python pkg    │      │
│  │  • CLI           │  │  • CF Workers    │  │  • npm package   │      │
│  │  • Standalone    │  │  • Deno/Bun      │  │  • In-process    │      │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘      │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
          ┌─────────────────┐             ┌─────────────────┐
          │  Embedded Mode  │             │   HTTP Mode     │
          │  (in-process)   │             │   (remote)      │
          │                 │             │                 │
          │  Resolve: <1ms  │             │  Resolve: ~50ms │
          │  Trace: queued  │             │  Trace: direct  │
          └─────────────────┘             └─────────────────┘
                    │                               │
                    └───────────────┬───────────────┘
                                    ▼
                          ┌─────────────────┐
                          │  TRACE Ingest   │
                          │  Service        │
                          │                 │
                          │  • Async batch  │
                          │  • Compression  │
                          │  • Dedup        │
                          └────────┬────────┘
                                   ▼
                          ┌─────────────────┐
                          │  TRACE Storage  │
                          │  (Postgres/S3)  │
                          └─────────────────┘
```

---

## Crate Structure

```
cra-core/
├── Cargo.toml                    # Workspace root
├── rust-toolchain.toml           # Pin Rust version
├── deny.toml                     # cargo-deny config
│
├── crates/
│   ├── cra-protocol/             # Core types (no deps)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── carp.rs           # CARP request/response types
│   │       ├── trace.rs          # TRACE event types
│   │       ├── atlas.rs          # Atlas manifest types
│   │       ├── policy.rs         # Policy rule types
│   │       └── error.rs          # Error types
│   │
│   ├── cra-crypto/               # Cryptographic primitives
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── hash.rs           # SHA-256 hashing
│   │       ├── chain.rs          # Hash chain for TRACE
│   │       └── signature.rs      # Optional signing
│   │
│   ├── cra-atlas/                # Atlas loading and caching
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loader.rs         # Load from file/URL
│   │       ├── cache.rs          # In-memory LRU cache
│   │       ├── validator.rs      # Schema validation
│   │       └── registry.rs       # Atlas registry client
│   │
│   ├── cra-policy/               # Policy evaluation engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs         # Rule evaluation
│   │       ├── conditions.rs     # Condition matchers
│   │       ├── effects.rs        # Allow/deny/require_approval
│   │       └── context.rs        # Evaluation context
│   │
│   ├── cra-engine/               # CARP resolution engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── resolver.rs       # Main resolution logic
│   │       ├── session.rs        # Session management
│   │       ├── context.rs        # Context block selection
│   │       └── action.rs         # Action permission logic
│   │
│   ├── cra-trace/                # TRACE collection and buffering
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── collector.rs      # Event collection
│   │       ├── buffer.rs         # Ring buffer for batching
│   │       ├── serializer.rs     # JSON/MessagePack output
│   │       └── sink.rs           # Sink trait (file, HTTP, etc.)
│   │
│   ├── cra-storage/              # Storage abstraction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs         # Storage traits
│   │       ├── memory.rs         # In-memory (testing)
│   │       ├── sqlite.rs         # SQLite (embedded)
│   │       └── postgres.rs       # PostgreSQL (production)
│   │
│   ├── cra-server/               # HTTP server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs           # Binary entry point
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── health.rs     # GET /health
│   │       │   ├── resolve.rs    # POST /v1/resolve
│   │       │   ├── execute.rs    # POST /v1/execute
│   │       │   ├── sessions.rs   # Session management
│   │       │   └── traces.rs     # TRACE endpoints
│   │       ├── middleware/
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs       # API key / JWT
│   │       │   ├── rate_limit.rs # Rate limiting
│   │       │   └── tracing.rs    # Request tracing
│   │       └── websocket.rs      # WebSocket for streaming
│   │
│   └── cra-cli/                  # CLI application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── serve.rs      # cra serve
│           │   ├── resolve.rs    # cra resolve
│           │   ├── trace.rs      # cra trace
│           │   ├── atlas.rs      # cra atlas
│           │   └── doctor.rs     # cra doctor
│           └── output.rs         # Human/JSON output
│
├── bindings/
│   ├── python/                   # PyO3 bindings
│   │   ├── Cargo.toml
│   │   ├── pyproject.toml        # maturin config
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── resolver.rs       # Python CRAResolver class
│   │       └── types.rs          # Type conversions
│   │
│   ├── node/                     # napi-rs bindings
│   │   ├── Cargo.toml
│   │   ├── package.json
│   │   └── src/
│   │       ├── lib.rs
│   │       └── resolver.rs
│   │
│   └── wasm/                     # WASM bindings
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── glue.rs           # wasm-bindgen glue
│
├── sdks/                         # Thin client SDKs
│   ├── typescript/               # @cra/client
│   │   ├── package.json
│   │   └── src/
│   │       ├── index.ts
│   │       ├── client.ts         # HTTP client
│   │       ├── embedded.ts       # WASM loader
│   │       └── types.ts          # Generated types
│   │
│   └── python/                   # cra-client
│       ├── pyproject.toml
│       └── cra_client/
│           ├── __init__.py
│           ├── client.py         # HTTP client
│           └── embedded.py       # Native extension loader
│
└── tests/
    ├── conformance/              # CARP/TRACE conformance
    ├── integration/              # Integration tests
    └── benchmarks/               # Performance benchmarks
```

---

## Core Crate Details

### cra-protocol (Zero Dependencies)

The foundational types, designed to compile to any target:

```rust
// crates/cra-protocol/src/carp.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPRequest {
    pub carp_version: String,
    pub request_id: String,
    pub timestamp: String,
    pub operation: Operation,
    pub requester: Requester,
    pub task: Task,
    #[serde(default)]
    pub atlas_ids: Vec<String>,
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Resolve,
    Execute,
    Validate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requester {
    pub agent_id: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_tier: Option<RiskTier>,
    #[serde(default)]
    pub context_hints: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPResolution {
    pub resolution_id: String,
    pub request_id: String,
    pub timestamp: String,
    pub decision: Decision,
    pub context_blocks: Vec<ContextBlock>,
    pub allowed_actions: Vec<ActionPermission>,
    pub denied_actions: Vec<DeniedAction>,
    pub constraints: Vec<Constraint>,
    pub ttl_seconds: u32,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny { reason: String },
    RequiresApproval { approver: String, timeout_seconds: u32 },
    Partial { reason: String },
}
```

```rust
// crates/cra-protocol/src/trace.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TRACEEvent {
    pub trace_version: String,
    pub event_id: String,
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub session_id: String,
    pub sequence: u64,
    pub timestamp: String,
    pub event_type: TRACEEventType,
    pub payload: serde_json::Value,
    pub event_hash: String,
    pub previous_event_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TRACEEventType {
    // Session lifecycle
    SessionStarted,
    SessionEnded,

    // CARP events
    CarpRequestReceived,
    CarpResolutionCompleted,

    // Action events
    ActionRequested,
    ActionApproved,
    ActionDenied,
    ActionExecuted,
    ActionFailed,

    // Context events
    ContextInjected,
    ContextExpired,

    // Policy events
    PolicyEvaluated,
    PolicyViolated,

    // Custom
    Custom(String),
}
```

### cra-engine (The Resolver)

```rust
// crates/cra-engine/src/resolver.rs

use cra_protocol::{CARPRequest, CARPResolution, Decision};
use cra_atlas::AtlasCache;
use cra_policy::PolicyEngine;
use cra_trace::TraceCollector;

pub struct CRAResolver {
    atlas_cache: AtlasCache,
    policy_engine: PolicyEngine,
    trace_collector: TraceCollector,
}

impl CRAResolver {
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            atlas_cache: AtlasCache::new(config.cache_size),
            policy_engine: PolicyEngine::new(),
            trace_collector: TraceCollector::new(config.trace_config),
        }
    }

    /// Resolve a CARP request - this is the hot path
    /// Designed to be <1ms for cached atlases
    pub fn resolve(&self, request: &CARPRequest) -> Result<CARPResolution, ResolveError> {
        let span = self.trace_collector.start_span("carp.resolve");

        // 1. Load relevant atlases (from cache)
        let atlases = self.atlas_cache.get_many(&request.atlas_ids)?;

        // 2. Select context blocks based on task
        let context_blocks = self.select_context(&request.task, &atlases);

        // 3. Determine allowed actions
        let (allowed, denied) = self.evaluate_actions(&request, &atlases);

        // 4. Evaluate policies
        let decision = self.policy_engine.evaluate(&request, &allowed)?;

        // 5. Build resolution
        let resolution = CARPResolution {
            resolution_id: generate_id(),
            request_id: request.request_id.clone(),
            timestamp: now_iso8601(),
            decision,
            context_blocks,
            allowed_actions: allowed,
            denied_actions: denied,
            constraints: vec![],
            ttl_seconds: 300,
            trace_id: span.trace_id().to_string(),
        };

        // 6. Record trace event (non-blocking)
        self.trace_collector.record(TRACEEventType::CarpResolutionCompleted, &resolution);

        span.end();
        Ok(resolution)
    }

    /// Pre-load atlases for faster resolution
    pub async fn preload_atlases(&self, atlas_ids: &[String]) -> Result<(), AtlasError> {
        self.atlas_cache.preload(atlas_ids).await
    }
}

// Configuration
pub struct ResolverConfig {
    pub cache_size: usize,
    pub trace_config: TraceConfig,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            cache_size: 100,
            trace_config: TraceConfig::default(),
        }
    }
}
```

### cra-trace (Async Buffered Collection)

```rust
// crates/cra-trace/src/collector.rs

use std::sync::mpsc::{channel, Sender};
use std::thread;

pub struct TraceCollector {
    sender: Sender<TraceCommand>,
    session_id: String,
    sequence: AtomicU64,
    previous_hash: Mutex<String>,
}

enum TraceCommand {
    Record(TRACEEvent),
    Flush,
    Shutdown,
}

impl TraceCollector {
    pub fn new(config: TraceConfig) -> Self {
        let (sender, receiver) = channel();

        // Background thread for async trace handling
        thread::spawn(move || {
            let mut buffer = TraceBuffer::new(config.buffer_size);
            let sink = config.create_sink();

            loop {
                match receiver.recv() {
                    Ok(TraceCommand::Record(event)) => {
                        buffer.push(event);
                        if buffer.should_flush() {
                            sink.write_batch(buffer.drain());
                        }
                    }
                    Ok(TraceCommand::Flush) => {
                        sink.write_batch(buffer.drain());
                    }
                    Ok(TraceCommand::Shutdown) | Err(_) => {
                        sink.write_batch(buffer.drain());
                        break;
                    }
                }
            }
        });

        Self {
            sender,
            session_id: generate_session_id(),
            sequence: AtomicU64::new(0),
            previous_hash: Mutex::new("genesis".to_string()),
        }
    }

    /// Record an event - non-blocking, returns immediately
    pub fn record<T: Serialize>(&self, event_type: TRACEEventType, payload: &T) {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        let payload_json = serde_json::to_value(payload).unwrap();

        let mut prev_hash = self.previous_hash.lock().unwrap();
        let event_hash = compute_hash(&event_type, &payload_json, &prev_hash);

        let event = TRACEEvent {
            trace_version: "1.0".to_string(),
            event_id: generate_id(),
            trace_id: self.session_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: None,
            session_id: self.session_id.clone(),
            sequence,
            timestamp: now_iso8601(),
            event_type,
            payload: payload_json,
            event_hash: event_hash.clone(),
            previous_event_hash: prev_hash.clone(),
        };

        *prev_hash = event_hash;
        drop(prev_hash);

        let _ = self.sender.send(TraceCommand::Record(event));
    }

    /// Force flush buffered events
    pub fn flush(&self) {
        let _ = self.sender.send(TraceCommand::Flush);
    }
}

// Sink implementations
pub trait TraceSink: Send + 'static {
    fn write_batch(&self, events: Vec<TRACEEvent>);
}

pub struct HttpTraceSink {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl TraceSink for HttpTraceSink {
    fn write_batch(&self, events: Vec<TRACEEvent>) {
        if events.is_empty() { return; }

        // Compress and send
        let payload = serde_json::to_vec(&events).unwrap();
        let compressed = zstd::encode_all(&payload[..], 3).unwrap();

        let _ = self.client
            .post(&self.endpoint)
            .header("Content-Encoding", "zstd")
            .body(compressed)
            .send();
    }
}

pub struct FileTraceSink {
    path: PathBuf,
}

impl TraceSink for FileTraceSink {
    fn write_batch(&self, events: Vec<TRACEEvent>) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .unwrap();

        for event in events {
            serde_json::to_writer(&mut file, &event).unwrap();
            writeln!(file).unwrap();
        }
    }
}
```

---

## Binding Layer Examples

### Python (PyO3)

```rust
// bindings/python/src/lib.rs

use pyo3::prelude::*;
use cra_engine::CRAResolver;
use cra_protocol::{CARPRequest, CARPResolution};

#[pyclass]
struct PyCRAResolver {
    inner: CRAResolver,
}

#[pymethods]
impl PyCRAResolver {
    #[new]
    fn new(config: Option<PyResolverConfig>) -> PyResult<Self> {
        let config = config.map(Into::into).unwrap_or_default();
        Ok(Self {
            inner: CRAResolver::new(config),
        })
    }

    /// Resolve a CARP request
    /// Returns resolution dict, raises CRAError on failure
    fn resolve(&self, request: &PyDict) -> PyResult<PyObject> {
        let request: CARPRequest = pythonize::depythonize(request)?;
        let resolution = self.inner.resolve(&request)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Python::with_gil(|py| {
            pythonize::pythonize(py, &resolution)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Preload atlases for faster resolution
    fn preload_atlases<'py>(&self, py: Python<'py>, atlas_ids: Vec<String>) -> PyResult<&'py PyAny> {
        let resolver = self.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            resolver.preload_atlases(&atlas_ids).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// Flush pending trace events
    fn flush_traces(&self) {
        self.inner.trace_collector().flush();
    }
}

#[pymodule]
fn cra_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyCRAResolver>()?;
    Ok(())
}
```

**Python Usage:**
```python
from cra_core import PyCRAResolver

# Create resolver (loads native Rust library)
resolver = PyCRAResolver()

# Preload atlases
await resolver.preload_atlases(["com.example.github-ops"])

# Resolve - executes in <1ms
resolution = resolver.resolve({
    "carp_version": "1.0",
    "request_id": "req-123",
    "timestamp": "2025-01-01T00:00:00Z",
    "operation": "resolve",
    "requester": {
        "agent_id": "my-agent",
        "session_id": "sess-456"
    },
    "task": {
        "goal": "Create a GitHub issue",
        "risk_tier": "low"
    },
    "atlas_ids": ["com.example.github-ops"]
})

print(resolution["allowed_actions"])
```

### Node.js (napi-rs)

```rust
// bindings/node/src/lib.rs

use napi::bindgen_prelude::*;
use napi_derive::napi;
use cra_engine::CRAResolver;

#[napi]
pub struct NodeCRAResolver {
    inner: CRAResolver,
}

#[napi]
impl NodeCRAResolver {
    #[napi(constructor)]
    pub fn new(config: Option<Object>) -> Result<Self> {
        let config = config.map(|c| parse_config(c)).transpose()?.unwrap_or_default();
        Ok(Self {
            inner: CRAResolver::new(config),
        })
    }

    #[napi]
    pub fn resolve(&self, request: Object) -> Result<Object> {
        let request: CARPRequest = serde_json::from_value(
            object_to_value(request)?
        )?;

        let resolution = self.inner.resolve(&request)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        value_to_object(serde_json::to_value(&resolution)?)
    }

    #[napi]
    pub async fn preload_atlases(&self, atlas_ids: Vec<String>) -> Result<()> {
        self.inner.preload_atlases(&atlas_ids).await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn flush_traces(&self) {
        self.inner.trace_collector().flush();
    }
}
```

**Node.js Usage:**
```typescript
import { NodeCRAResolver } from '@cra/core';

const resolver = new NodeCRAResolver();

// Preload
await resolver.preloadAtlases(['com.example.github-ops']);

// Resolve - <1ms
const resolution = resolver.resolve({
  carp_version: '1.0',
  request_id: 'req-123',
  timestamp: new Date().toISOString(),
  operation: 'resolve',
  requester: {
    agent_id: 'my-agent',
    session_id: 'sess-456',
  },
  task: {
    goal: 'Create a GitHub issue',
    risk_tier: 'low',
  },
  atlas_ids: ['com.example.github-ops'],
});

console.log(resolution.allowed_actions);
```

### WASM

```rust
// bindings/wasm/src/lib.rs

use wasm_bindgen::prelude::*;
use cra_engine::CRAResolver;

#[wasm_bindgen]
pub struct WasmCRAResolver {
    inner: CRAResolver,
}

#[wasm_bindgen]
impl WasmCRAResolver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CRAResolver::new(ResolverConfig::default()),
        }
    }

    #[wasm_bindgen]
    pub fn resolve(&self, request: JsValue) -> Result<JsValue, JsError> {
        let request: CARPRequest = serde_wasm_bindgen::from_value(request)?;
        let resolution = self.inner.resolve(&request)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&resolution)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn load_atlas(&mut self, atlas_json: &str) -> Result<(), JsError> {
        let atlas: AtlasManifest = serde_json::from_str(atlas_json)?;
        self.inner.atlas_cache().insert(atlas);
        Ok(())
    }
}
```

**Browser Usage:**
```typescript
import init, { WasmCRAResolver } from '@cra/wasm';

await init();

const resolver = new WasmCRAResolver();

// Load atlas (fetched separately)
const atlasJson = await fetch('/atlases/github-ops.json').then(r => r.text());
resolver.load_atlas(atlasJson);

// Resolve in browser - no network call
const resolution = resolver.resolve({
  carp_version: '1.0',
  // ...
});
```

---

## HTTP Server (for HTTP-only mode)

```rust
// crates/cra-server/src/main.rs

use axum::{
    routing::{get, post},
    Router, Json, Extension,
};
use std::sync::Arc;
use cra_engine::CRAResolver;

#[tokio::main]
async fn main() {
    let config = load_config();
    let resolver = Arc::new(CRAResolver::new(config.resolver));

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/resolve", post(resolve))
        .route("/v1/execute", post(execute))
        .route("/v1/sessions", post(create_session))
        .route("/v1/sessions/:id", get(get_session))
        .route("/v1/traces", get(list_traces))
        .route("/v1/traces/:id/stream", get(stream_traces))
        .layer(Extension(resolver))
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("CRA server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn resolve(
    Extension(resolver): Extension<Arc<CRAResolver>>,
    Json(request): Json<CARPRequest>,
) -> Result<Json<CARPResolution>, AppError> {
    let resolution = resolver.resolve(&request)?;
    Ok(Json(resolution))
}
```

---

## TRACE Ingest Service

Separate service for centralized trace collection:

```rust
// services/trace-ingest/src/main.rs

use axum::{routing::post, Router, Json};
use tokio_postgres::Client;

#[tokio::main]
async fn main() {
    let db = connect_postgres().await;

    let app = Router::new()
        .route("/ingest", post(ingest_traces))
        .route("/ingest/batch", post(ingest_batch))
        .layer(Extension(db));

    // Listen on internal port
    axum::Server::bind(&"0.0.0.0:8421".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ingest_batch(
    Extension(db): Extension<Client>,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    // Decompress
    let decompressed = zstd::decode_all(&body[..])?;
    let events: Vec<TRACEEvent> = serde_json::from_slice(&decompressed)?;

    // Validate hash chain
    validate_chain(&events)?;

    // Batch insert
    let stmt = db.prepare(
        "INSERT INTO traces (event_id, trace_id, session_id, event_type, payload, timestamp, event_hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    ).await?;

    for event in events {
        db.execute(&stmt, &[
            &event.event_id,
            &event.trace_id,
            &event.session_id,
            &event.event_type.to_string(),
            &event.payload,
            &event.timestamp,
            &event.event_hash,
        ]).await?;
    }

    Ok(StatusCode::ACCEPTED)
}
```

---

## Implementation Phases

### Phase 0: Foundation (Week 1-2)
- [ ] Set up Rust workspace with Cargo
- [ ] Implement `cra-protocol` with all types
- [ ] Implement `cra-crypto` hash chain
- [ ] Basic test suite
- [ ] CI/CD with GitHub Actions

### Phase 1: Core Engine (Week 3-4)
- [ ] Implement `cra-atlas` loader and cache
- [ ] Implement `cra-policy` rule engine
- [ ] Implement `cra-engine` resolver
- [ ] Implement `cra-trace` collector with buffering
- [ ] Integration tests

### Phase 2: HTTP Server (Week 5-6)
- [ ] Implement `cra-server` with Axum
- [ ] Add authentication middleware
- [ ] Add rate limiting
- [ ] WebSocket trace streaming
- [ ] OpenAPI spec generation

### Phase 3: Language Bindings (Week 7-8)
- [ ] Python bindings with PyO3 + maturin
- [ ] Node.js bindings with napi-rs
- [ ] WASM bindings with wasm-bindgen
- [ ] Publish to PyPI, npm

### Phase 4: TRACE Ingest (Week 9-10)
- [ ] Implement trace ingest service
- [ ] PostgreSQL storage
- [ ] S3 archival
- [ ] Replay API

### Phase 5: Production Hardening (Week 11-12)
- [ ] Performance benchmarks
- [ ] Fuzzing with cargo-fuzz
- [ ] Security audit
- [ ] Documentation
- [ ] Release v1.0

---

## Performance Targets

| Operation | Target | Method |
|-----------|--------|--------|
| Resolve (cached atlas) | <1ms | In-memory LRU cache |
| Resolve (cold atlas) | <50ms | Async preloading |
| Trace record | <10μs | Lock-free queue |
| Trace flush | Async | Background thread |
| HTTP resolve | <5ms | Axum + Tokio |
| WASM resolve | <2ms | No FFI overhead |

---

## Build Targets

```bash
# Native binaries
cargo build --release -p cra-server
cargo build --release -p cra-cli

# Python wheel
cd bindings/python && maturin build --release

# Node.js addon
cd bindings/node && npm run build

# WASM
cd bindings/wasm && wasm-pack build --target web

# Docker
docker build -t cra-server .
```

---

## Migration Path from TypeScript

The existing TypeScript implementation can be:

1. **Replaced** - Use Rust core, deprecate TS runtime
2. **Wrapped** - TS SDK calls Rust via napi-rs
3. **Parallel** - Keep TS for HTTP mode, Rust for embedded

**Recommended:** Option 2 - Keep TypeScript SDK as the user-facing API, but use Rust under the hood for performance-critical paths.

```typescript
// @cra/client - TypeScript SDK
import { CRAClient } from '@cra/client';

// Automatically uses:
// - WASM in browser
// - Native addon in Node.js
// - HTTP fallback if neither available
const client = await CRAClient.create({
  mode: 'auto', // 'embedded' | 'http' | 'auto'
  atlases: ['com.example.github-ops'],
});

const resolution = await client.resolve(request);
```

---

## Open Questions

1. **Async runtime:** Tokio vs async-std vs smol?
2. **Serialization:** serde_json vs simd-json for hot path?
3. **HTTP framework:** Axum vs Actix-web?
4. **WASM size budget:** How small must the WASM bundle be?
5. **Python GIL:** Release GIL during resolution?

---

## Conclusion

This plan enables CRA to be:
- **Embedded** for zero-latency agent integration
- **Served** for simple HTTP-based usage
- **Universal** via WASM for browser/edge
- **Interoperable** with Python/Node.js ecosystems

The Rust core provides the performance and safety guarantees needed for production agent infrastructure, while language bindings and HTTP APIs maintain developer experience.
