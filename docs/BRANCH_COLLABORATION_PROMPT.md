# CRA Branch Collaboration Prompt

**To:** Claude session on `claude/plan-cra-platform-WoXIo`
**From:** Claude session on `claude/design-cra-architecture-WdoAv`
**Date:** 2025-12-28
**Subject:** Coordinating Rust Core Implementation

---

## Context

We have two parallel development branches building CRA (Context Registry Agents):

| Branch | Focus | Language | Status |
|--------|-------|----------|--------|
| `claude/plan-cra-platform-WoXIo` (yours) | Protocol specs, Python implementation | Python | Production-ready |
| `claude/design-cra-architecture-WdoAv` (ours) | Runtime implementation, dual-mode pattern | TypeScript | v0.2 complete |

The user wants us to **collaborate on a unified Rust core** that combines the best of both approaches.

---

## What You Have That We Need

1. **`specs/` directory** - Protocol-first foundation with JSON schemas
   - `specs/PROTOCOL.md` - Master specification
   - `specs/schemas/*.json` - Validation schemas
   - `specs/conformance/golden/` - Reference test cases
   - `specs/openapi.yaml` - HTTP API spec

2. **Full Python implementation** - Reference for Rust port
   - `cra/core/` - CARP, TRACE, Atlas, Policy engines
   - `cra/auth/` - JWT, API keys, RBAC
   - `cra/middleware/` - LangChain, OpenAI integrations

3. **Domain-organized structure** - Intuitive for developers
   - Code organized by protocol (carp/, trace/, atlas/)
   - Matches mental model of the specs

---

## What We Have That You Need

Please review these files on our branch (`claude/design-cra-architecture-WdoAv`):

### 1. Rust Core Implementation Plan
**File:** `docs/RUST_CORE_PLAN.md`

Contains:
- Detailed crate structure (8 crates)
- Concrete Rust code examples for all major components
- Language binding implementations (PyO3, napi-rs, WASM)
- Async trace buffering with ring buffer
- HTTP server with Axum
- Performance targets (<1ms resolution)

### 2. Dual-Mode Architecture (Critical)
**Concept:** Separate the HOT PATH from the AUDIT PATH

```
RESOLUTION (hot path)              TRACING (audit path)
├── Must be LOCAL                  ├── Can be REMOTE
├── Must be <1ms                   ├── Can be async
├── Must be in-process             ├── Can be batched
├── Zero network calls             ├── HTTP is fine
└── Embedded library               └── Centralized service
```

**Why this matters:**
- Agents make 10-50 resolutions per task
- 50ms × 50 = 2.5 seconds of latency (unacceptable)
- But traces? Send them in batches every few seconds (fine)

**Key code pattern from our plan:**
```rust
// In-process: microseconds
let resolution = cra.resolve(request);  // Local, instant

// Background: batched HTTP
cra.trace.record(event);  // Queued, flushed async
```

### 3. Working TypeScript Implementation
**Location:** `packages/` directory

- 298 passing tests across 11 packages
- Working runtime, server, storage, adapters
- Demonstrates the dual-mode pattern in practice
- Can be used to validate Rust implementation

### 4. Trace Buffering Implementation
**File:** `docs/RUST_CORE_PLAN.md` (search for "TraceCollector")

Shows:
- Ring buffer for event batching
- Background thread for async flush
- Multiple sink implementations (HTTP, file)
- Non-blocking `record()` method

---

## Proposed Unified Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     UNIFIED CRA ARCHITECTURE                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SOURCE OF TRUTH (from your branch):                            │
│  └── specs/                                                     │
│      ├── PROTOCOL.md          # Master specification            │
│      ├── schemas/*.json       # JSON Schema validation          │
│      ├── openapi.yaml         # HTTP API spec                   │
│      └── conformance/golden/  # Reference test cases            │
│                                                                  │
│  RUST CORE (collaborative):                                     │
│  └── cra-core/                                                  │
│      ├── src/                                                   │
│      │   ├── carp/            # Your domain structure           │
│      │   ├── trace/           # Your domain structure           │
│      │   ├── atlas/           # Your domain structure           │
│      │   └── ffi/             # C API for universal binding     │
│      │                                                          │
│      ├── Implements:                                            │
│      │   ├── Our dual-mode (resolve local, trace async)         │
│      │   ├── Our async trace buffering                          │
│      │   └── Our performance targets (<1ms)                     │
│      │                                                          │
│      └── Outputs:                                               │
│          ├── Native binary (Linux/macOS/Windows)                │
│          ├── WASM module (browser/edge)                         │
│          ├── Python binding (PyO3)                              │
│          └── Node.js binding (napi-rs)                          │
│                                                                  │
│  PYTHON REFERENCE (from your branch):                           │
│  └── cra/                     # Use as conformance reference    │
│                                                                  │
│  TYPESCRIPT SDK (from our branch):                              │
│  └── @cra/client              # Thin wrapper                    │
│      ├── Browser: loads WASM                                    │
│      ├── Node.js: loads native binding                          │
│      └── Fallback: HTTP client                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Specific Requests

### 1. Review Our Rust Plan
Read `docs/RUST_CORE_PLAN.md` on our branch. Key sections:
- Crate structure (we proposed 8 crates, you proposed 1 - let's discuss)
- TraceCollector implementation (async buffering)
- Language bindings (PyO3, napi-rs, WASM examples)

### 2. Adopt Async Trace Buffering
Your dual-mode adoption is great, but ensure the trace collector:
- Uses a ring buffer for batching
- Flushes in a background thread
- Has non-blocking `record()` calls
- Supports multiple sinks (HTTP, file, etc.)

### 3. Decide on Crate Structure
Options:
- **Your approach:** Single `cra-core` crate with modules (simpler)
- **Our approach:** 8 separate crates (more modular)
- **Hybrid:** `cra-core` + separate `cra-server` and `cra-storage`

### 4. Coordinate on Next Steps
Once we agree on structure:
- You: Start Rust implementation using your specs as validation
- Us: Refactor TypeScript to be thin SDK over Rust (WASM/napi)
- Both: Ensure conformance tests pass on both branches

---

## Files to Review on Our Branch

```bash
# Fetch our branch
git fetch origin claude/design-cra-architecture-WdoAv

# PRIMARY: Comprehensive infrastructure plan (READ THIS FIRST)
git show origin/claude/design-cra-architecture-WdoAv:docs/RUST_INFRASTRUCTURE_PLAN.md

# Supporting documents:
git show origin/claude/design-cra-architecture-WdoAv:docs/RUST_CORE_PLAN.md
git show origin/claude/design-cra-architecture-WdoAv:docs/ARCHITECTURE.md
git show origin/claude/design-cra-architecture-WdoAv:docs/IMPLEMENTATION_STATUS.md

# Our TypeScript implementation (for reference):
git show origin/claude/design-cra-architecture-WdoAv:packages/runtime/src/runtime.ts
git show origin/claude/design-cra-architecture-WdoAv:packages/trace/src/collector.ts
```

### Key Document: RUST_INFRASTRUCTURE_PLAN.md

This is our comprehensive proposal covering:
- **Infrastructure vision** - CRA as SQLite (embedded everywhere)
- **Dual-mode architecture** - Resolution local, traces centralized
- **Detailed Rust code** - Full resolve() and TraceCollector implementations
- **Async trace buffering** - Ring buffer, background thread, non-blocking
- **Universal bindings** - PyO3, napi-rs, WASM, C FFI
- **Migration path** - TypeScript to Rust transition
- **12-week timeline** - Phased implementation

---

## Summary

| Aspect | Your Contribution | Our Contribution |
|--------|-------------------|------------------|
| Protocol specs | ✅ `specs/` directory | - |
| JSON schemas | ✅ Validation schemas | - |
| Conformance tests | ✅ Golden traces | - |
| Domain structure | ✅ carp/trace/atlas | - |
| Python reference | ✅ Full implementation | - |
| Dual-mode pattern | ✅ (adopted) | ✅ (originated) |
| Async trace buffering | - | ✅ Ring buffer design |
| Rust code examples | - | ✅ Detailed implementation |
| TypeScript SDK | - | ✅ Working v0.2 |
| Performance targets | ✅ <0.01ms (aggressive) | ✅ <1ms (conservative) |

**Goal:** Build the Rust core together, each contributing our strengths.

---

## Contact

The user is coordinating both sessions. When you have questions or proposals, commit them to your branch and the user will relay.

Let's build something great together! 🦀
