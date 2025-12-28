# CRA Rust Implementation - Development Journal

**Started:** 2025-12-28
**Author:** Claude (Opus 4.5)
**Branch:** `claude/cra-rust-refactor-XBcJV`

This journal documents every decision, rationale, and implementation detail for the CRA Rust core. This implementation is being built from scratch to later compare against other approaches.

---

## Table of Contents

1. [Project Goals](#1-project-goals)
2. [Why Rust](#2-why-rust)
3. [Architecture Decisions](#3-architecture-decisions)
4. [Implementation Log](#4-implementation-log)
5. [Trade-offs Made](#5-trade-offs-made)
6. [What I Would Do Differently](#6-what-i-would-do-differently)
7. [Comparison Notes](#7-comparison-notes)

---

## 1. Project Goals

### Primary Objective

Create a **universal CRA core** that can be embedded in any language runtime without network overhead.

### Success Criteria

| Metric | Target | Rationale |
|--------|--------|-----------|
| Resolution latency | <0.01ms | Must be invisible to LLM latency |
| Binary size | <1MB | Embeddable in mobile/edge |
| WASM size | <500KB | Browser-viable |
| Conformance | 100% | Protocol compliance non-negotiable |
| Test coverage | >80% | Production reliability |

### Non-Goals (For Now)

- HTTP server (thin wrapper can be added later)
- Database persistence (pluggable backends later)
- Authentication (handled by embedding application)

---

## 2. Why Rust

### The Core Problem

The Python and TypeScript implementations require:
```
Agent → HTTP → CRA Server → HTTP → Agent
         ↑                    ↑
      5-50ms              5-50ms
```

This adds 10-100ms to every resolution, and makes CRA visible as an "external tool" to LLMs.

### Rust Solution

```
Agent Process
┌─────────────────────────────────┐
│  Agent Logic                    │
│       ↓                         │
│  CRA Core (Rust, in-process)    │  ← 0.01ms, invisible
│       ↓                         │
│  Continue execution             │
└─────────────────────────────────┘
```

### Why Not Other Languages?

| Language | Problem |
|----------|---------|
| C/C++ | Memory safety issues unacceptable in governance layer |
| Go | GC pauses, larger binary, worse FFI story |
| Zig | Too immature, smaller ecosystem |
| Rust | Memory safe, zero-cost abstractions, excellent FFI |

### Rust Advantages Used

1. **Zero-cost abstractions** - Iterators, pattern matching compile to optimal code
2. **Ownership model** - No GC pauses, predictable latency
3. **Excellent FFI** - PyO3, napi-rs, wasm-bindgen are mature
4. **Cargo ecosystem** - Easy dependency management, cross-compilation

---

## 3. Architecture Decisions

### 3.1 Workspace Structure

**Decision:** Cargo workspace with multiple crates

```
CRA-Core/
├── Cargo.toml          # Workspace root
├── cra-core/           # Core library (cdylib + rlib)
├── cra-python/         # PyO3 bindings
├── cra-node/           # napi-rs bindings
└── cra-wasm/           # wasm-bindgen bindings
```

**Rationale:**
- Separation of concerns - core logic vs bindings
- Independent versioning possible
- Parallel compilation
- Clear dependency direction (bindings depend on core)

**Alternative Considered:** Single crate with feature flags
- Rejected: More complex conditional compilation, harder to maintain

### 3.2 Error Handling

**Decision:** Custom `CRAError` enum with `thiserror`

```rust
#[derive(Debug, thiserror::Error)]
pub enum CRAError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },
    // ...
}
```

**Rationale:**
- Type-safe error handling
- Error codes for FFI compatibility
- `is_recoverable()` method for retry logic
- Structured errors (not just strings)

**Alternative Considered:** `anyhow::Error`
- Rejected: Need structured errors for FFI, not just messages

### 3.3 Serialization

**Decision:** `serde` with `serde_json`

**Rationale:**
- Industry standard for Rust JSON
- Derive macros reduce boilerplate
- Rename attributes for protocol compliance (`#[serde(rename = "carp_version")]`)
- Optional fields handled cleanly

### 3.4 Hash Chain Implementation

**Decision:** SHA-256 via `sha2` crate, computed over concatenated fields

```rust
pub fn compute_hash(&self) -> String {
    let mut hasher = Sha256::new();
    hasher.update(self.trace_version.as_bytes());
    hasher.update(self.event_id.as_bytes());
    // ... all fields in protocol-defined order
    hasher.update(self.previous_event_hash.as_bytes());
    hex::encode(hasher.finalize())
}
```

**Rationale:**
- SHA-256 specified in protocol
- Deterministic ordering crucial for verification
- Hex encoding for human readability
- `sha2` is pure Rust, no OpenSSL dependency

**Critical Detail:** Fields MUST be hashed in exact order specified in `specs/PROTOCOL.md`:
```
trace_version || event_id || trace_id || span_id || parent_span_id ||
session_id || sequence || timestamp || event_type || canonical_json(payload) ||
previous_event_hash
```

### 3.5 Policy Evaluation Order

**Decision:** Strict ordering enforced in `PolicyEvaluator::evaluate()`

```rust
pub fn evaluate(&mut self, action_id: &str) -> PolicyResult {
    // Phase 1: Deny policies (immediate rejection)
    for policy in self.policies.iter().filter(|p| p.policy_type == PolicyType::Deny) {
        if matches_action(&policy.actions, action_id) {
            return PolicyResult::Deny { ... };
        }
    }

    // Phase 2: Approval policies
    // Phase 3: Rate limit policies
    // Phase 4: Allow policies
    // Phase 5: Default allow
}
```

**Rationale:**
- Protocol specifies: deny → requires_approval → rate_limit → allow
- Security principle: deny rules always win
- Rate limits checked after approval (approved actions can still be rate-limited)
- Default allow matches protocol ("If no policy matches, action is allowed")

### 3.6 Pattern Matching for Actions

**Decision:** Simple glob-style patterns, not regex

```rust
fn pattern_matches(pattern: &str, action_id: &str) -> bool {
    if pattern == "*" { return true; }
    if pattern == action_id { return true; }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return action_id.starts_with(prefix)
            && action_id[prefix.len()..].starts_with('.');
    }
    // ...
}
```

**Supported Patterns:**
- `*` - matches everything
- `ticket.get` - exact match
- `ticket.*` - prefix match (ticket.get, ticket.create, etc.)
- `*.delete` - suffix match (ticket.delete, user.delete, etc.)

**Rationale:**
- Simple, predictable behavior
- No regex compilation overhead
- Matches protocol specification
- Easy to reason about for policy authors

**Alternative Considered:** Full regex
- Rejected: Overkill, potential ReDoS, harder to audit policies

### 3.7 Session State Management

**Decision:** In-memory `HashMap<String, Session>` in `Resolver`

```rust
pub struct Resolver {
    sessions: HashMap<String, Session>,
    // ...
}
```

**Rationale:**
- Simple for MVP
- Fast O(1) lookup
- Session lifecycle is request-scoped anyway
- Persistence is pluggable concern (not core responsibility)

**Trade-off:** No persistence across restarts
- Acceptable: Embedding application can handle persistence if needed
- Future: Add `StorageBackend` trait for pluggable persistence

### 3.8 Trace Collection

**Decision:** Per-session event vectors with optional callback

```rust
pub struct TraceCollector {
    sessions: HashMap<String, Vec<TRACEEvent>>,
    on_emit: Option<Box<dyn Fn(&TRACEEvent) + Send + Sync>>,
}
```

**Rationale:**
- Events grouped by session for easy retrieval
- Callback allows streaming to external systems
- In-memory storage sufficient for session lifetime
- Export to JSONL for persistence

**Callback Pattern:**
```rust
collector.set_callback(|event| {
    // Stream to Kafka, write to file, etc.
    println!("{}", serde_json::to_string(event).unwrap());
});
```

### 3.9 FFI Design

**Decision:** C-style API with opaque pointers

```c
// Opaque handle
typedef struct CRAResolver CRAResolver;

// Lifecycle
CRAResolver* cra_resolver_new(void);
void cra_resolver_free(CRAResolver* resolver);

// Operations return JSON strings
char* cra_resolver_resolve(CRAResolver* resolver, const char* request_json);
void cra_free_string(char* s);

// Error handling
const char* cra_get_last_error(void);
```

**Rationale:**
- Maximum compatibility (any language can call C)
- Opaque pointers hide implementation details
- JSON strings for complex data (avoid struct ABI issues)
- Thread-local error storage avoids callback complexity

**Memory Contract:**
- All returned strings MUST be freed with `cra_free_string()`
- `CRAResolver*` MUST be freed with `cra_resolver_free()`
- Input strings must be valid UTF-8, null-terminated

---

## 4. Implementation Log

### Entry 1: Initial Setup (2025-12-28)

**What I Did:**
1. Created Cargo workspace with 4 crates
2. Set up shared dependencies in workspace `Cargo.toml`
3. Configured each binding crate (PyO3, napi-rs, wasm-bindgen)

**Key Files Created:**
- `Cargo.toml` (workspace)
- `cra-core/Cargo.toml`
- `cra-python/Cargo.toml`
- `cra-node/Cargo.toml`
- `cra-wasm/Cargo.toml`

**Decisions Made:**
- Used `cdylib` + `rlib` crate types for core (library + dynamic)
- Pinned dependency versions for reproducibility
- Added `criterion` for benchmarking

### Entry 2: Core Types (2025-12-28)

**What I Did:**
1. Implemented `CARPRequest` and `CARPResolution` types
2. Implemented `TRACEEvent` with hash computation
3. Implemented `AtlasManifest` with validation

**Challenges:**
- Ensuring JSON serialization matches protocol exactly
- Getting serde rename attributes right for snake_case ↔ camelCase

**Solution:**
```rust
#[derive(Serialize, Deserialize)]
pub struct CARPRequest {
    #[serde(default = "default_carp_version")]
    pub carp_version: String,
    // ...
}
```

### Entry 3: CARP Resolver (2025-12-28)

**What I Did:**
1. Implemented `Resolver` with session management
2. Implemented `PolicyEvaluator` with correct ordering
3. Connected TRACE emission to resolution flow

**Key Insight:**
The resolver is the orchestrator - it coordinates:
- Atlas loading
- Session lifecycle
- Policy evaluation
- Trace emission

Each `resolve()` call:
1. Validates request
2. Checks session exists and is active
3. Loads relevant actions from atlases
4. Evaluates each action against policies
5. Emits TRACE events
6. Returns resolution

### Entry 4: TRACE Chain Verification (2025-12-28)

**What I Did:**
1. Implemented `ChainVerifier` for integrity checking
2. Implemented `ReplayEngine` for deterministic replay
3. Added chain extension verification

**Critical Bug Fixed:**
Initial test used `create_test_chain()` twice, but events have random UUIDs and timestamps. Changed to clone:
```rust
let chain_a = create_test_chain();
let chain_b = chain_a.clone();  // NOT create_test_chain() again!
```

### Entry 5: FFI Layer (2025-12-28)

**What I Did:**
1. Created C API in `ffi/mod.rs`
2. Used `Box::into_raw` / `Box::from_raw` for pointer management
3. Implemented thread-local error storage

**Pattern Used:**
```rust
#[no_mangle]
pub extern "C" fn cra_resolver_new() -> *mut Resolver {
    Box::into_raw(Box::new(Resolver::new()))
}

#[no_mangle]
pub unsafe extern "C" fn cra_resolver_free(resolver: *mut Resolver) {
    if !resolver.is_null() {
        drop(Box::from_raw(resolver));
    }
}
```

### Entry 6: Fixing Borrow Checker Issues (2025-12-28)

**Problem:**
```rust
// This doesn't compile!
for policy in self.policies.iter() {
    if self.check_rate_limit(action_id) {  // Mutable borrow
        // ...
    }
}
```

**Solution:**
Extract to free functions that don't borrow `self`:
```rust
fn matches_action(patterns: &[String], action_id: &str) -> bool {
    patterns.iter().any(|p| pattern_matches(p, action_id))
}

// Now in the loop:
for policy in self.policies.iter() {
    if matches_action(&policy.actions, action_id) {
        // ...
    }
}
```

**Lesson:** Rust's borrow checker forces better design - separating pure functions from stateful methods.

### Entry 7: Test Suite (2025-12-28)

**What I Did:**
1. Added unit tests to each module
2. Added integration tests in `lib.rs`
3. Achieved 68 passing tests

**Test Categories:**
- `atlas::*` - 16 tests
- `carp::*` - 18 tests
- `trace::*` - 24 tests
- `ffi::*` - 4 tests
- Integration - 6 tests

---

## 5. Trade-offs Made

### 5.1 In-Memory Only

**Trade-off:** No persistence, sessions lost on restart

**Why:**
- Keeps core simple and fast
- Persistence is application-specific concern
- Can add `StorageBackend` trait later

**Mitigation:** Export functions for JSONL traces

### 5.2 Single-Threaded Resolver

**Trade-off:** `Resolver` is not `Send + Sync`

**Why:**
- Simpler implementation
- Most embeddings are single-threaded per instance
- Avoids mutex overhead

**Mitigation:** Create one `Resolver` per thread if needed

### 5.3 No Async

**Trade-off:** All operations are synchronous

**Why:**
- Core operations are CPU-bound, not I/O-bound
- Simpler FFI (async across FFI is complex)
- Sub-millisecond latency doesn't need async

**Mitigation:** Wrapper can spawn to thread pool if needed

### 5.4 String-Heavy API

**Trade-off:** UUIDs stored as `String`, not `Uuid`

**Why:**
- Simpler serialization
- FFI friendliness
- Protocol uses string representation anyway

**Cost:** ~40 bytes per ID instead of 16

### 5.5 Clone for Policy Evaluation

**Trade-off:** Clone rate limit policies to avoid borrow issues

```rust
let rate_limit_matches: Vec<_> = self.policies
    .iter()
    .filter(|p| p.policy_type == PolicyType::RateLimit)
    .cloned()  // Clone here
    .collect();
```

**Why:** Avoids complex lifetime annotations

**Cost:** Small allocation per resolution (policies are small)

---

## 6. What I Would Do Differently

### 6.1 Start with Conformance Tests

I implemented the logic first, then found the conformance tests. Better approach:

1. Parse `specs/conformance/golden/*` first
2. Write failing tests against golden traces
3. Implement until tests pass

### 6.2 More Granular Modules

Current `carp/resolver.rs` is 400+ lines. Could split:
- `session.rs` - Session management
- `execution.rs` - Action execution
- `resolution.rs` - Resolution building

### 6.3 Builder Pattern Everywhere

Used builders for `CARPResolution` but not consistently. Should use for:
- `CARPRequest`
- `TRACEEvent`
- `AtlasManifest`

### 6.4 Property-Based Testing

Current tests are example-based. Could add:
```rust
#[quickcheck]
fn hash_chain_always_verifiable(events: Vec<TRACEEvent>) -> bool {
    let chain = build_chain(events);
    ChainVerifier::verify(&chain).is_valid
}
```

---

## 7. Comparison Notes

*This section will be updated after comparing with other implementations.*

### Expected Differences from Python Branch

| Aspect | Python | Rust (Expected) |
|--------|--------|-----------------|
| Startup time | ~500ms (interpreter) | ~1ms |
| Resolution latency | ~5ms (HTTP) | ~0.01ms |
| Memory per session | ~10KB | ~1KB |
| Binary size | N/A (needs Python) | ~800KB |

### Expected Differences from Other Rust Attempts

*To be filled in after comparison*

---

## Appendix: Key Code Patterns

### Pattern 1: Default Values in Serde

```rust
fn default_carp_version() -> String {
    "1.0".to_string()
}

#[derive(Deserialize)]
pub struct CARPRequest {
    #[serde(default = "default_carp_version")]
    pub carp_version: String,
}
```

### Pattern 2: Builder Pattern

```rust
impl CARPResolution {
    pub fn builder(session_id: String) -> CARPResolutionBuilder {
        CARPResolutionBuilder::new(session_id)
    }
}

// Usage
let resolution = CARPResolution::builder(session_id)
    .decision(Decision::Allow)
    .allowed_actions(actions)
    .build();
```

### Pattern 3: Safe FFI

```rust
#[no_mangle]
pub extern "C" fn cra_some_function(input: *const c_char) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        let input_str = unsafe { CStr::from_ptr(input) }.to_str()?;
        // ... do work ...
        Ok(output)
    });

    match result {
        Ok(Ok(s)) => CString::new(s).unwrap().into_raw(),
        Ok(Err(e)) => { set_error(e.to_string()); std::ptr::null_mut() }
        Err(_) => { set_error("panic".into()); std::ptr::null_mut() }
    }
}
```

### Pattern 4: Hash Chain Linking

```rust
impl TRACEEvent {
    pub fn chain(mut self, sequence: u64, previous_hash: String) -> Self {
        self.sequence = sequence;
        self.previous_event_hash = previous_hash;
        self.event_hash = self.compute_hash();
        self
    }
}

// Usage
let event2 = TRACEEvent::new(...)
    .chain(1, event1.event_hash.clone());
```

---

## 8. Benchmark Results (2025-12-28)

Benchmarks run on Linux with Criterion:

| Operation | Time | Notes |
|-----------|------|-------|
| `resolver_new` | 28.5 ns | Empty resolver creation |
| `atlas_load` | 6.7 µs | Parse and load 5-action atlas |
| `session_create` | 28 µs | Create session + emit TRACE event |
| `resolve` | 134 µs | Full resolution with 5 actions |
| `execute` | 57 µs | Execute single action |
| `verify_chain` (100 events) | 1.47 ms | SHA-256 verify 100 events |

### Analysis

The `resolve` operation at 134 µs (0.134 ms) is higher than the 0.01ms target. Breakdown:

1. **Policy evaluation**: ~5-10 µs per action × 5 actions = ~25-50 µs
2. **TRACE event emission**: ~6 events × ~15 µs = ~90 µs
3. **Resolution building**: ~10 µs

The TRACE overhead dominates. This is acceptable because:
- TRACE is cryptographic (SHA-256 per event)
- Most of the time is hash computation, not logic
- 134 µs is still 50-500x faster than HTTP (~5-50ms)

### Optimization Opportunities (Not Implemented)

1. **Lazy hash computation** - Compute hashes on export, not emit
2. **Batch events** - Buffer events, hash in batch
3. **Parallel hashing** - Use rayon for chain verification
4. **Arena allocation** - Reduce allocations per resolution

---

## 9. Conformance Test Results (2025-12-28)

Created `cra-core/tests/conformance.rs` with 7 tests against golden traces:

| Test | Status | What It Verifies |
|------|--------|------------------|
| `conformance_simple_resolve_decision` | ✅ PASS | Decision type matches expected |
| `conformance_simple_resolve_trace_events` | ✅ PASS | Event types in correct order |
| `conformance_simple_resolve_trace_payloads` | ✅ PASS | Event payloads contain required fields |
| `conformance_hash_chain_integrity` | ✅ PASS | Chain verification succeeds |
| `conformance_policy_deny_takes_precedence` | ✅ PASS | Deny policies override allow |
| `conformance_genesis_event_hash` | ✅ PASS | Genesis hash is 64 zeros |
| `conformance_sequence_monotonic` | ✅ PASS | Sequence numbers increase by 1 |

### Key Conformance Guarantees

1. **Protocol-defined ordering**: deny → approval → rate_limit → allow
2. **Hash chain**: SHA-256 computed over exact field order from spec
3. **Genesis convention**: `previous_event_hash = "00...00"` (64 zeros)
4. **Event types**: Match `specs/PROTOCOL.md` definitions
5. **Decision types**: `allow`, `deny`, `partial`, `requires_approval`

---

## 10. Test Summary

**Total: 76 tests passing**

| Category | Count | Location |
|----------|-------|----------|
| Unit tests | 68 | `cra-core/src/**/*.rs` |
| Conformance tests | 7 | `cra-core/tests/conformance.rs` |
| Doc tests | 1 | `cra-core/src/lib.rs` |

---

## 11. Response to "Dual Mode Architecture" Question (2025-12-28)

### The Question

The other branches mentioned I "didn't account for the dual mode architecture" they have.

### What They Mean

The Python implementation has two modes:

1. **Server Mode** - FastAPI HTTP server (`cra/runtime/server.py`)
   - REST API endpoints for resolve, execute, trace
   - WebSocket/SSE for trace streaming
   - PostgreSQL for persistence

2. **Middleware Mode** - In-process integration (`cra/middleware/`)
   - LangChainMiddleware
   - OpenAIMiddleware
   - Direct function calls, no HTTP

### My Position: We Don't Need Server Mode in Rust Core

**Reasoning:**

1. **The whole point of Rust is eliminating HTTP overhead**
   ```
   Python Server Mode:
   Agent → HTTP → FastAPI → Python Core → HTTP → Agent
                    ↑                       ↑
                 5-50ms                  5-50ms

   Rust Embedded Mode:
   Agent → Rust Core (in-process) → Agent
                ↑
            0.134ms
   ```
   Adding an HTTP server to Rust would defeat the purpose.

2. **Server mode is a THIN WRAPPER, not core logic**
   - All business logic is in `cra-core`
   - HTTP is just serialization + routing
   - The wrapper should be separate from core

3. **Separation of concerns**
   - `cra-core` = Pure logic (no I/O, no networking)
   - `cra-server` = Optional HTTP wrapper
   - `cra-python` = Python bindings (can use FastAPI if needed)

4. **Users who need server mode can**:
   - Use Python: `from cra import Resolver` + FastAPI (uses Rust via PyO3)
   - Use Node.js: `require('@cra/core')` + Express (uses Rust via napi-rs)
   - Use Rust: Add `axum` or `actix-web` wrapper (20 lines of code)

### What I Will Add

To address their concern without polluting the core:

1. **Example server wrapper** in `examples/http_server.rs` using `axum`
2. **Optional `cra-server` crate** if there's demand
3. **Documentation** showing both embedded and server patterns

### Why This Is The Right Approach

```
Their Architecture:                Our Architecture:
┌─────────────────────────┐       ┌─────────────────────────┐
│   Python Monolith       │       │   Rust Core (pure)      │
│   ┌─────────────────┐   │       │   • CARP                │
│   │  Core Logic     │   │       │   • TRACE               │
│   ├─────────────────┤   │       │   • Atlas               │
│   │  HTTP Server    │   │       │   • Policy              │
│   ├─────────────────┤   │       └───────────┬─────────────┘
│   │  PostgreSQL     │   │                   │
│   ├─────────────────┤   │       ┌───────────┴─────────────┐
│   │  Auth (JWT)     │   │       │   Wrapper Layer         │
│   └─────────────────┘   │       │   (Optional, Separate)  │
└─────────────────────────┘       │   • HTTP (axum)         │
                                  │   • Storage (sqlx)      │
Coupled: Everything in one        │   • Auth (tower)        │
                                  └─────────────────────────┘
                                  Decoupled: Mix and match
```

### The SQLite Analogy

SQLite is embedded, not a server. If you want a server, you use:
- SQLite + your own HTTP layer
- Or switch to PostgreSQL

Similarly:
- CRA Core (embedded) + your own HTTP layer
- Or use the Python implementation if you want batteries-included

### Conclusion

**Do we need dual mode? Yes, but not in core.**

- ✅ Embedded mode is in `cra-core`
- ✅ Server mode is achievable via thin wrappers
- ❌ Server mode should NOT be baked into the core library

I'll add example code showing how to wrap the core with HTTP.

---

## 12. Enhanced Python Bindings (2025-12-28)

### What Changed

Rewrote Python bindings to return **proper Python objects** instead of just JSON strings.

### Before (JSON-only)
```python
resolution = resolver.resolve(session_id, agent_id, goal)
# resolution is a JSON string - need to parse it
data = json.loads(resolution)
```

### After (Python objects)
```python
resolution = resolver.resolve(session_id, agent_id, goal)

# Proper Python objects with attributes
print(resolution.decision)           # "allow", "deny", "partial"
print(resolution.is_allowed)         # True/False property
print(resolution.allowed_actions)    # List of AllowedAction objects

# Access action details
for action in resolution.allowed_actions:
    print(f"{action.action_id}: {action.description}")
    print(f"Risk: {action.risk_tier}")

# Check specific action
if resolution.is_action_allowed("data.get"):
    print("data.get is available")

# Trace events as objects
for event in resolver.get_trace_events(session_id):
    print(f"{event.event_type}: seq={event.sequence}")
    payload = event.get_payload_dict()  # Python dict

# Chain verification with proper boolean
if resolver.verify_chain(session_id):
    print("Chain is valid")
```

### New Python Classes

| Class | Purpose |
|-------|---------|
| `Resolver` | Main interface (unchanged) |
| `CARPResolution` | Resolution with properties |
| `AllowedAction` | Action with attributes |
| `DeniedAction` | Denied action with reason |
| `TRACEEvent` | Event with payload access |
| `ChainVerification` | Verification with bool support |

### Key Improvements

1. **Type safety**: Python objects have proper attributes
2. **IDE support**: Autocomplete works with classes
3. **Pythonic**: `if verification:` works (implements `__bool__`)
4. **Repr**: `print(action)` shows useful info
5. **Backwards compatible**: `resolve_json()` still available

---

## 13. Pluggable Storage Backend (2025-12-28)

### Motivation

The original implementation stored all traces in-memory within the `TraceCollector`. This had limitations:

1. No persistence across restarts
2. No way to swap storage for different environments
3. Testing required managing the collector's internal state

### Solution: Storage Trait

Created a pluggable `StorageBackend` trait in `cra-core/src/storage/mod.rs`:

```rust
pub trait StorageBackend: Send + Sync {
    fn store_event(&self, event: &TRACEEvent) -> Result<()>;
    fn get_events(&self, session_id: &str) -> Result<Vec<TRACEEvent>>;
    fn get_events_by_type(&self, session_id: &str, event_type: &str) -> Result<Vec<TRACEEvent>>;
    fn get_last_events(&self, session_id: &str, n: usize) -> Result<Vec<TRACEEvent>>;
    fn get_event_count(&self, session_id: &str) -> Result<usize>;
    fn delete_session(&self, session_id: &str) -> Result<()>;
    fn health_check(&self) -> Result<()>;
    fn name(&self) -> &'static str;
}
```

### Implementations

Three storage backends included out of the box:

| Backend | Use Case | Characteristics |
|---------|----------|-----------------|
| `InMemoryStorage` | Default, testing | Fast, thread-safe via RwLock, no persistence |
| `FileStorage` | Development, debugging | JSONL files per session, human-readable |
| `NullStorage` | Testing, high-performance | Discards all events, zero overhead |

### Example Usage

```rust
use cra_core::storage::{StorageBackend, InMemoryStorage, FileStorage};

// Default in-memory storage
let storage = InMemoryStorage::new();

// File-based storage
let storage = FileStorage::new("/var/cra/traces")?;

// Custom backend (e.g., PostgreSQL)
struct PostgresStorage { pool: PgPool }
impl StorageBackend for PostgresStorage {
    // ... implement trait methods
}
```

### Error Handling

Added `StorageLocked` error variant for RwLock poisoning:

```rust
#[error("Storage backend lock poisoned")]
StorageLocked,
```

This is a recoverable error - retry is possible after short delay.

### Design Decisions

1. **`&self` instead of `&mut self`**: All methods take `&self` to allow interior mutability patterns (RwLock, Mutex). This enables sharing storage across threads.

2. **Per-session organization**: Storage is organized by session_id. This maps naturally to the TRACE spec where each session has an independent hash chain.

3. **JSONL for FileStorage**: Newline-delimited JSON is easy to parse, append-only friendly, and human-readable for debugging.

4. **`Send + Sync` bound**: Required for thread-safe storage sharing. All provided implementations satisfy this.

### Test Results

```
test storage::tests::test_in_memory_storage ... ok
test storage::tests::test_file_storage ... ok
test storage::tests::test_null_storage ... ok
```

**Total: 80 tests passing** (71 unit + 7 conformance + 2 doc tests)

### Future Work

The storage trait is designed for extension. Planned backends:

- `SqliteStorage` - Embedded database for single-node deployments
- `PostgresStorage` - Shared storage for distributed deployments
- `RedisStorage` - Fast ephemeral storage with TTL support

These will be in separate crates (`cra-storage-sqlite`, etc.) to avoid adding dependencies to core.

---

*Journal continues as development progresses...*
