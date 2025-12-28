# CRA Rust Core Refactor - Session Prompt

## Mission

Refactor CRA (Context Registry Agents) into a **protocol-first Rust core** with language bindings for Python, TypeScript, and WASM. The goal is infrastructure-level governance for AI agents that can be embedded anywhere without tool-use overhead.

---

## Context: What Exists

This repository contains multiple implementation attempts across branches:

### Branch: `claude/plan-cra-platform-WoXIo` (Python - Most Complete)

**111 files** - Production-ready Python implementation:

```
cra/
├── core/           # CARP resolver, TRACE collector, Atlas loader, policies
├── runtime/        # FastAPI server, services, PostgreSQL storage
├── cli/            # Typer-based CLI with all commands
├── adapters/       # OpenAI, Anthropic, Google ADK, MCP adapters
├── templates/      # LangChain, CrewAI, OpenAI GPT generators
├── auth/           # JWT, API keys, RBAC (5 roles)
├── middleware/     # Framework integration (LangChain, OpenAI)
├── observability/  # OpenTelemetry, SIEM exporters
└── config/         # Pydantic settings

specs/              # PROTOCOL-FIRST FOUNDATION (use this!)
├── PROTOCOL.md                    # Master spec (CARP/1.0, TRACE/1.0, Atlas/1.0)
├── openapi.yaml                   # HTTP API spec
├── schemas/
│   ├── carp-request.schema.json
│   ├── carp-resolution.schema.json
│   ├── trace-event.schema.json
│   └── atlas-manifest.schema.json
└── conformance/
    ├── CONFORMANCE.md             # Test requirements
    └── golden/                    # Reference test cases

docs/
├── ARCHITECTURE.md
├── API.md
├── CLI.md
├── ATLASES.md
├── DEPLOYMENT.md
├── INTEGRATION.md
└── TYPESCRIPT_SDK_PLAN.md
```

### Branch: `claude/design-cra-architecture-WdoAv` (TypeScript)

**~60 files** - Node.js monorepo with packages:

```
packages/
├── protocol/    # CARP/TRACE TypeScript types
├── trace/       # Trace collector
├── atlas/       # Atlas loader
├── runtime/     # Core runtime
├── adapters/    # OpenAI, Claude, MCP
├── cli/         # CLI tool
├── mcp/         # MCP server (started)
└── otel/        # OpenTelemetry bridge (started)
```

Has 76 passing tests. Good patterns for MCP and type definitions.

### Branches: `2025-12-27/22-*-codex` (Python MVP)

Early Python prototypes with good documentation specs in `docs/`:
- `CARP_1_0.md`, `TRACE_1_0.md` - Protocol specs
- `EXECUTIVE_BRIEF.md` - Vision document
- `CONFORMANCE_TESTS.md` - Test requirements

---

## Target Architecture: Rust Core + Bindings

```
┌─────────────────────────────────────────────────────────────────┐
│                        CRA Infrastructure                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    cra-core (Rust)                         │ │
│  │                                                            │ │
│  │  • CARP Engine (resolution, policies, constraints)        │ │
│  │  • TRACE Collector (hash chain, events, replay)           │ │
│  │  • Atlas Loader (manifest parsing, validation)            │ │
│  │  • Policy Engine (deny lists, rate limits, approvals)     │ │
│  │  • ~50KB binary, minimal dependencies                     │ │
│  │                                                            │ │
│  └───────────────────────────┬────────────────────────────────┘ │
│                              │                                   │
│         ┌────────────────────┼────────────────────┐             │
│         │                    │                    │             │
│         ▼                    ▼                    ▼             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │ cra-python  │     │  cra-node   │     │  cra-wasm   │       │
│  │ (PyO3 FFI)  │     │ (napi-rs)   │     │             │       │
│  │             │     │             │     │ • Browsers  │       │
│  │ • LangChain │     │ • MCP       │     │ • Edge      │       │
│  │ • CrewAI    │     │ • VS Code   │     │ • Deno      │       │
│  │ • FastAPI   │     │ • Dashboard │     │             │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Rust Core

1. **Embedded everywhere** - Native bindings to Python, Node.js, WASM
2. **Zero tool-use cost** - In-process calls, not HTTP requests
3. **OS-level integration** - Can run as system daemon
4. **Infrastructure status** - Like SQLite, embedded in everything
5. **Performance** - ~0.001ms resolution vs ~5-50ms HTTP
6. **Memory safety** - No runtime errors in governance layer

---

## Implementation Plan

### Phase 1: Rust Core (`cra-core`)

```
cra-core/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library root, public API
│   ├── carp/
│   │   ├── mod.rs
│   │   ├── request.rs      # CARPRequest struct
│   │   ├── resolution.rs   # CARPResolution struct
│   │   ├── resolver.rs     # resolve() implementation
│   │   └── policy.rs       # Policy evaluation engine
│   ├── trace/
│   │   ├── mod.rs
│   │   ├── event.rs        # TRACEEvent struct
│   │   ├── collector.rs    # Event collection
│   │   ├── chain.rs        # SHA-256 hash chain
│   │   └── replay.rs       # Deterministic replay
│   ├── atlas/
│   │   ├── mod.rs
│   │   ├── manifest.rs     # AtlasManifest struct
│   │   ├── loader.rs       # Load from disk/memory
│   │   └── validator.rs    # JSON Schema validation
│   └── ffi/
│       ├── mod.rs
│       └── c_api.rs        # C ABI for any language
└── tests/
    └── conformance/        # Run against specs/conformance/
```

**Key Rust crates:**
- `serde` + `serde_json` - Serialization
- `jsonschema` - Schema validation
- `sha2` - Hash chain
- `uuid` - UUIDv7 generation
- `chrono` - Timestamps

**Public API:**
```rust
pub struct Resolver { ... }

impl Resolver {
    pub fn new() -> Self;
    pub fn load_atlas(&mut self, manifest: &str) -> Result<(), Error>;
    pub fn resolve(&self, request: &CARPRequest) -> Result<CARPResolution, Error>;
    pub fn execute(&mut self, action_id: &str, params: Value) -> Result<Value, Error>;
    pub fn get_trace(&self, session_id: &str) -> Vec<TRACEEvent>;
    pub fn verify_chain(&self, session_id: &str) -> Result<bool, Error>;
}
```

### Phase 2: Python Binding (`cra-python`)

```
cra-python/
├── Cargo.toml              # PyO3 config
├── src/
│   └── lib.rs              # Python bindings
├── python/
│   └── cra/
│       ├── __init__.py
│       ├── resolver.py     # Pythonic wrapper
│       └── middleware/
│           ├── langchain.py
│           └── crewai.py
└── pyproject.toml
```

**Usage:**
```python
from cra import Resolver

resolver = Resolver()
resolver.load_atlas("./atlas.json")

# In-process, ~0.001ms
resolution = resolver.resolve(goal="Help user", agent_id="my-agent")

# LangChain integration
from cra.middleware import LangChainMiddleware
middleware = LangChainMiddleware(resolver)
tools = middleware.get_tools(goal="Customer support")
```

### Phase 3: Node.js Binding (`cra-node`)

```
cra-node/
├── Cargo.toml              # napi-rs config
├── src/
│   └── lib.rs              # Node.js bindings
├── npm/
│   ├── package.json
│   └── index.d.ts          # TypeScript types
└── __tests__/
```

**Usage:**
```typescript
import { Resolver } from '@cra/core';

const resolver = new Resolver();
await resolver.loadAtlas('./atlas.json');

// In-process, ~0.001ms
const resolution = resolver.resolve({
  goal: 'Help user',
  agentId: 'my-agent',
});
```

### Phase 4: WASM Build (`cra-wasm`)

```
cra-wasm/
├── Cargo.toml              # wasm-bindgen config
├── src/
│   └── lib.rs              # WASM bindings
└── pkg/                    # Generated npm package
```

**Usage (browser):**
```typescript
import init, { Resolver } from '@cra/wasm';

await init();
const resolver = new Resolver();
// Runs entirely client-side
```

### Phase 5: Language-Specific Features

**Python (`cra-python`):**
- FastAPI server wrapper
- LangChain/CrewAI middleware
- Pydantic model generation from schemas

**TypeScript (`cra-node`):**
- MCP server
- VS Code extension
- React dashboard

---

## Conformance Requirements

The Rust implementation MUST pass all tests in `specs/conformance/`:

1. **Schema Validation** - All JSON validates against `specs/schemas/*.json`
2. **Hash Chain** - SHA-256 computation matches reference
3. **Policy Evaluation** - Deny > Approval > Rate Limit > Allow
4. **Golden Traces** - Output matches `specs/conformance/golden/`
5. **Replay Determinism** - Same input → same output

Run conformance:
```bash
cargo test --features conformance
```

---

## Key Files to Reference

| File | Purpose |
|------|---------|
| `specs/PROTOCOL.md` | Master protocol specification |
| `specs/schemas/*.json` | JSON Schema definitions |
| `specs/conformance/CONFORMANCE.md` | Test requirements |
| `specs/openapi.yaml` | HTTP API spec (for server mode) |
| `cra/core/carp.py` | Python CARP implementation to port |
| `cra/core/trace.py` | Python TRACE implementation to port |
| `cra/core/atlas.py` | Python Atlas implementation to port |
| `cra/core/policy.py` | Python policy engine to port |

---

## Success Criteria

1. **Rust core passes all conformance tests**
2. **Python binding works with existing middleware** (LangChain, CrewAI)
3. **Node.js binding works with MCP**
4. **WASM builds and runs in browser**
5. **Performance: <0.01ms resolution time**
6. **Binary size: <1MB (without debug symbols)**

---

## Commands to Start

```bash
# Create Rust workspace
mkdir cra-rust && cd cra-rust
cargo new cra-core --lib
cargo new cra-python --lib
cargo new cra-node --lib
cargo new cra-wasm --lib

# Copy protocol specs
cp -r ../specs .

# Start implementing
# Reference: specs/PROTOCOL.md, specs/schemas/
```

---

## Notes

- **Protocol is source of truth** - specs/ directory defines everything
- **Python code is reference** - Port logic from cra/core/*.py
- **TypeScript has good types** - Reference packages/protocol/src/
- **Don't break existing Python** - Binding should be drop-in replacement
- **Conformance tests are mandatory** - No shortcuts

---

*This prompt provides context for refactoring CRA into a protocol-first Rust core with universal bindings.*
