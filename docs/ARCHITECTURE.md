# CRA-Core Rust Architecture

## Overview

CRA-Core is the universal Rust implementation of the Context Registry for Agents (CRA) system. It provides a high-performance, embeddable core that can be used across multiple platforms through language-specific bindings.

### Design Goals

1. **Universal Core**: Single source of truth for CRA logic, deployable everywhere
2. **Zero-Copy Performance**: <0.01ms resolution latency through careful memory management
3. **Minimal Footprint**: <1MB binary size for embedded deployment
4. **Platform Agnostic**: Compile to native (via FFI), Python, Node.js, and WebAssembly
5. **Cryptographic Integrity**: SHA-256 hash chains for tamper-evident audit trails

---

## Workspace Structure

```
CRA-Core/
├── Cargo.toml              # Workspace root
├── cra-core/               # Core library
│   ├── src/
│   │   ├── lib.rs          # Public API exports
│   │   ├── error.rs        # Error types
│   │   ├── carp/           # CARP protocol implementation
│   │   ├── trace/          # TRACE protocol implementation
│   │   ├── atlas/          # Atlas package system
│   │   └── ffi/            # C FFI bindings
│   └── benches/            # Performance benchmarks
├── cra-python/             # PyO3 Python bindings
├── cra-node/               # napi-rs Node.js bindings
├── cra-wasm/               # wasm-bindgen WebAssembly bindings
├── specs/                  # Protocol specifications
│   ├── schemas/            # JSON Schema definitions
│   └── conformance/        # Conformance test suite
└── docs/                   # Documentation
```

---

## Core Modules

### 1. CARP Module (`cra-core/src/carp/`)

The Context & Action Resolution Protocol determines what context and actions are available to an agent.

#### 1.1 Request (`request.rs`)

```rust
pub struct CARPRequest {
    pub carp_version: String,      // "1.0"
    pub request_id: String,        // UUID
    pub session_id: String,        // Active session reference
    pub agent_id: String,          // Requesting agent identifier
    pub goal: String,              // Natural language goal description
    pub context: Option<Value>,    // Additional context (optional)
    pub requested_actions: Vec<String>,  // Specific actions requested
    pub risk_tier: RiskTier,       // low, medium, high, critical
    pub timestamp: DateTime<Utc>,  // ISO 8601 timestamp
}
```

**Risk Tiers:**
- `Low`: Read-only operations, no external effects
- `Medium`: Modifications with undo capability
- `High`: Irreversible changes, external API calls
- `Critical`: Financial transactions, PII access, security-sensitive

#### 1.2 Resolution (`resolution.rs`)

```rust
pub struct CARPResolution {
    pub carp_version: String,
    pub resolution_id: String,
    pub session_id: String,
    pub trace_id: String,          // Links to TRACE events
    pub decision: Decision,        // Allow, Deny, Partial, AllowWithConstraints
    pub allowed_actions: Vec<AllowedAction>,
    pub denied_actions: Vec<DeniedAction>,
    pub context_blocks: Vec<ContextBlock>,
    pub constraints: Vec<Constraint>,
    pub expires_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

pub enum Decision {
    Allow,                 // All requested actions permitted
    Deny,                  // All actions denied
    Partial,               // Some actions allowed, some denied
    AllowWithConstraints,  // Allowed with restrictions
}
```

#### 1.3 Policy Evaluator (`policy.rs`)

The policy engine evaluates actions against loaded policies in a strict order:

```
┌─────────────────────────────────────────────────────────┐
│                    Policy Evaluation                     │
├─────────────────────────────────────────────────────────┤
│  1. DENY policies      → Immediate rejection            │
│  2. APPROVAL policies  → Requires human approval        │
│  3. RATE_LIMIT policies→ Check quota, throttle if needed│
│  4. ALLOW policies     → Explicit allowance             │
│  5. No match           → Default allow                  │
└─────────────────────────────────────────────────────────┘
```

**Pattern Matching:**
```rust
fn pattern_matches(pattern: &str, action_id: &str) -> bool {
    // Exact: "ticket.get" matches "ticket.get"
    // Wildcard suffix: "ticket.*" matches "ticket.get", "ticket.create"
    // Wildcard prefix: "*.delete" matches "ticket.delete", "user.delete"
    // Full wildcard: "*" matches everything
}
```

**Rate Limiting:**
```rust
struct RateLimitState {
    count: u64,           // Current call count in window
    window_start: Instant,// When current window started
    max_calls: u64,       // Maximum allowed calls
    window_seconds: u64,  // Window duration
}
```

#### 1.4 Resolver (`resolver.rs`)

The main orchestrator managing sessions, atlases, and resolutions:

```rust
pub struct Resolver {
    atlases: HashMap<String, AtlasManifest>,
    sessions: HashMap<String, Session>,
    policy_evaluator: PolicyEvaluator,
    trace_collector: TraceCollector,
    default_ttl: u64,
}

impl Resolver {
    pub fn new() -> Self;
    pub fn load_atlas(&mut self, atlas: AtlasManifest) -> Result<()>;
    pub fn create_session(&mut self, agent_id: &str, goal: &str) -> Result<String>;
    pub fn resolve(&mut self, request: &CARPRequest) -> Result<CARPResolution>;
    pub fn execute(&mut self, session_id: &str, action_id: &str, params: Value) -> Result<Value>;
    pub fn end_session(&mut self, session_id: &str) -> Result<()>;
    pub fn verify_chain(&self, session_id: &str) -> Result<ChainVerification>;
}
```

---

### 2. TRACE Module (`cra-core/src/trace/`)

The Telemetry & Replay Audit Contract provides cryptographic proof of what happened.

#### 2.1 Event (`event.rs`)

```rust
pub struct TRACEEvent {
    pub trace_version: String,       // "1.0"
    pub event_id: String,            // Unique event UUID
    pub trace_id: String,            // Groups related events
    pub span_id: String,             // Operation span
    pub parent_span_id: Option<String>,
    pub session_id: String,
    pub sequence: u64,               // Monotonically increasing
    pub timestamp: DateTime<Utc>,    // Microsecond precision
    pub event_type: EventType,
    pub payload: Value,              // Event-specific data
    pub event_hash: String,          // SHA-256 of this event
    pub previous_event_hash: String, // Hash chain link
}
```

**Event Types:**
```rust
pub enum EventType {
    SessionStarted,
    SessionEnded,
    CARPRequestReceived,
    CARPResolutionCompleted,
    PolicyEvaluated,
    ActionExecuted,
    ActionFailed,
    ConstraintViolation,
    ApprovalRequested,
    ApprovalReceived,
    Custom(String),
}
```

#### 2.2 Hash Chain

Each event's hash is computed over:
```
SHA-256(
    trace_version ||
    event_id ||
    trace_id ||
    span_id ||
    parent_span_id ||
    session_id ||
    sequence ||
    timestamp ||
    event_type ||
    canonical_json(payload) ||
    previous_event_hash
)
```

**Genesis Event:**
- First event uses `previous_event_hash = "0000...0000"` (64 zeros)
- This establishes the chain anchor

```
┌──────────┐    ┌──────────┐    ┌──────────┐
│ Event 0  │───▶│ Event 1  │───▶│ Event 2  │
│ Genesis  │    │          │    │          │
│ prev: 00 │    │ prev: H0 │    │ prev: H1 │
│ hash: H0 │    │ hash: H1 │    │ hash: H2 │
└──────────┘    └──────────┘    └──────────┘
```

#### 2.3 Chain Verifier (`chain.rs`)

```rust
pub struct ChainVerifier;

impl ChainVerifier {
    /// Verify entire chain integrity
    pub fn verify(events: &[TRACEEvent]) -> ChainVerification;

    /// Find where two chains diverge
    pub fn find_divergence(a: &[TRACEEvent], b: &[TRACEEvent]) -> Option<usize>;

    /// Verify chain extension is valid
    pub fn verify_extension(base: &[TRACEEvent], extension: &[TRACEEvent]) -> bool;
}

pub struct ChainVerification {
    pub is_valid: bool,
    pub event_count: usize,
    pub first_event_id: Option<String>,
    pub last_event_id: Option<String>,
    pub error_type: Option<ChainErrorType>,
    pub error_index: Option<usize>,
}
```

#### 2.4 Replay Engine (`replay.rs`)

Deterministic replay for debugging and auditing:

```rust
pub struct ReplayEngine {
    speed: f64,           // 1.0 = real-time
    pause_on_error: bool,
}

impl ReplayEngine {
    pub fn replay(&self, events: &[TRACEEvent]) -> Result<ReplayResult>;
    pub fn diff(&self, a: &[TRACEEvent], b: &[TRACEEvent]) -> TraceDiff;
}

pub struct ReplayResult {
    pub success: bool,
    pub events_replayed: usize,
    pub errors: Vec<ReplayError>,
    pub stats: ReplayStats,
}
```

---

### 3. Atlas Module (`cra-core/src/atlas/`)

Atlas packages contain domain-specific context, actions, and policies.

#### 3.1 Manifest (`manifest.rs`)

```rust
pub struct AtlasManifest {
    pub atlas_version: String,    // "1.0"
    pub atlas_id: String,         // Reverse domain: "com.company.domain"
    pub name: String,
    pub description: String,
    pub version: String,          // SemVer
    pub author: Option<String>,
    pub license: Option<String>,
    pub dependencies: Vec<AtlasDependency>,
    pub capabilities: Vec<String>,
    pub actions: Vec<AtlasAction>,
    pub policies: Vec<AtlasPolicy>,
    pub context: Option<Value>,
}

pub struct AtlasAction {
    pub action_id: String,        // "ticket.create"
    pub name: String,
    pub description: String,
    pub capability: String,       // Required capability
    pub parameters_schema: Option<Value>,  // JSON Schema
    pub returns_schema: Option<Value>,
    pub risk_tier: String,
    pub examples: Vec<ActionExample>,
}

pub struct AtlasPolicy {
    pub policy_id: String,
    pub policy_type: PolicyType,  // allow, deny, rate_limit, requires_approval
    pub actions: Vec<String>,     // Patterns to match
    pub conditions: Option<Value>,
    pub parameters: Option<Value>,
}
```

#### 3.2 Loader (`loader.rs`)

```rust
pub struct AtlasLoader {
    atlases: HashMap<String, LoadedAtlas>,
    search_paths: Vec<PathBuf>,
}

impl AtlasLoader {
    pub fn load_from_json(&mut self, json: &str) -> Result<String>;
    pub fn load_from_file(&mut self, path: &Path) -> Result<String>;
    pub fn load_from_directory(&mut self, path: &Path) -> Result<String>;
}
```

#### 3.3 Validator (`validator.rs`)

Comprehensive validation including:
- Schema validation (JSON Schema draft-07)
- Semantic validation (policy references valid actions)
- Dependency resolution
- Cross-reference checking

```rust
pub struct AtlasValidator;

impl AtlasValidator {
    pub fn validate(manifest: &AtlasManifest) -> ValidationResult;
    pub fn validate_policy(policy: &AtlasPolicy, manifest: &AtlasManifest) -> Vec<ValidationFinding>;
}
```

---

### 4. FFI Module (`cra-core/src/ffi/`)

C-compatible API for language bindings:

```c
// Lifecycle
CRAResolver* cra_resolver_new(void);
void cra_resolver_free(CRAResolver* resolver);

// Atlas Management
int32_t cra_resolver_load_atlas(CRAResolver* resolver, const char* json);

// Session Management
char* cra_resolver_create_session(CRAResolver* resolver,
                                   const char* agent_id,
                                   const char* goal);
int32_t cra_resolver_end_session(CRAResolver* resolver,
                                  const char* session_id);

// Resolution
char* cra_resolver_resolve(CRAResolver* resolver, const char* request_json);

// Error Handling
const char* cra_get_last_error(void);
void cra_free_string(char* s);

// Version
const char* cra_version(void);
const char* cra_carp_version(void);
const char* cra_trace_version(void);
```

**Memory Management:**
- All returned strings are heap-allocated
- Caller must free with `cra_free_string()`
- Thread-local error storage via `cra_get_last_error()`

---

## Language Bindings

### Python (cra-python)

```python
from cra import Resolver, CARPRequest

resolver = Resolver()
resolver.load_atlas_json(atlas_json)

session_id = resolver.create_session("agent-1", "Help with tickets")

request = CARPRequest(
    session_id=session_id,
    agent_id="agent-1",
    goal="Create a support ticket"
)
resolution = resolver.resolve(request)

print(f"Decision: {resolution.decision}")
for action in resolution.allowed_actions:
    print(f"  - {action.action_id}: {action.description}")
```

### Node.js (cra-node)

```javascript
const { Resolver, CARPRequest } = require('cra-node');

const resolver = new Resolver();
resolver.loadAtlasJson(atlasJson);

const sessionId = resolver.createSession('agent-1', 'Help with tickets');

const request = new CARPRequest({
    sessionId,
    agentId: 'agent-1',
    goal: 'Create a support ticket'
});
const resolution = resolver.resolve(request);

console.log(`Decision: ${resolution.decision}`);
```

### WebAssembly (cra-wasm)

```javascript
import init, { Resolver, CARPRequest } from 'cra-wasm';

await init();

const resolver = new Resolver();
resolver.loadAtlasJson(atlasJson);

const sessionId = resolver.createSession('agent-1', 'Help with tickets');
const resolution = resolver.resolve(requestJson);
```

---

## Data Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Agent     │────▶│   CARP      │────▶│   Atlas     │
│             │     │   Request   │     │   Lookup    │
└─────────────┘     └─────────────┘     └─────────────┘
                           │                   │
                           ▼                   ▼
                    ┌─────────────┐     ┌─────────────┐
                    │   Policy    │◀────│   Actions   │
                    │   Evaluate  │     │   Found     │
                    └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐     ┌─────────────┐
                    │   CARP      │────▶│   TRACE     │
                    │  Resolution │     │   Events    │
                    └─────────────┘     └─────────────┘
                           │                   │
                           ▼                   ▼
                    ┌─────────────┐     ┌─────────────┐
                    │   Agent     │     │   Hash      │
                    │   Executes  │     │   Chain     │
                    └─────────────┘     └─────────────┘
```

---

## Performance Characteristics

### Benchmarks (on Apple M1)

| Operation | Latency | Notes |
|-----------|---------|-------|
| Policy Lookup | ~50μs | O(n) policies, O(1) pattern cache |
| Hash Computation | ~2μs | SHA-256 single event |
| Chain Verification | ~100μs/event | Linear scan |
| Resolution (5 actions) | ~200μs | Including TRACE emit |

### Memory Usage

| Component | Size |
|-----------|------|
| Resolver (empty) | ~1KB |
| Per Session | ~500B |
| Per Event | ~300B |
| Per Atlas | Variable (typically 5-50KB) |

### Binary Sizes

| Target | Size |
|--------|------|
| Native (release, stripped) | ~800KB |
| WASM (optimized) | ~400KB |
| Python wheel | ~2MB (includes Python overhead) |

---

## Security Considerations

1. **Hash Chain Integrity**: Any modification to events is detectable through hash verification
2. **No Secrets in Events**: Payload should not contain credentials; use references instead
3. **Policy Isolation**: Each session has isolated policy evaluation state
4. **Rate Limiting**: Built-in protection against excessive action execution
5. **Input Validation**: All JSON inputs validated against schemas

---

## Testing

### Unit Tests (68 total)

```bash
cargo test --package cra-core
```

Categories:
- `atlas::*` - 16 tests (manifest, loader, validator)
- `carp::*` - 18 tests (request, resolution, resolver, policy)
- `trace::*` - 24 tests (event, collector, chain, replay)
- `ffi::*` - 4 tests (C API)
- Integration - 6 tests (full workflows)

### Conformance Tests

Located in `specs/conformance/`:
- Golden traces for deterministic verification
- Schema validation tests
- Cross-implementation compatibility tests

### Benchmarks

```bash
cargo bench --package cra-core
```

---

## Error Handling

```rust
pub enum CRAError {
    // CARP Errors
    InvalidCARPRequest { reason: String },
    InvalidCARPResolution { reason: String },

    // Session Errors
    SessionNotFound { session_id: String },
    SessionAlreadyExists { session_id: String },
    SessionAlreadyEnded { session_id: String },

    // Atlas Errors
    AtlasNotFound { atlas_id: String },
    InvalidAtlas { reason: String },
    AtlasValidationFailed { errors: Vec<String> },

    // Policy Errors
    PolicyNotFound { policy_id: String },
    PolicyEvaluationFailed { reason: String },

    // Trace Errors
    InvalidTraceEvent { reason: String },
    ChainVerificationFailed { reason: String },

    // I/O Errors
    IoError { message: String },
    JsonError { message: String },
}
```

Each error has:
- `error_code()` - Unique string identifier
- `is_recoverable()` - Whether operation can be retried

---

## Future Work

1. **Async Support**: `async/await` for I/O-bound operations
2. **Persistence**: SQLite/RocksDB for trace storage
3. **Streaming**: Real-time event streaming via WebSocket
4. **Encryption**: At-rest encryption for sensitive payloads
5. **Compression**: LZ4 compression for trace storage
6. **Clustering**: Distributed trace aggregation
