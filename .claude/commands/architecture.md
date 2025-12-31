# CRA Architecture Guide

Quick reference for the CRA-Core architecture.

## Protocol Stack

```
CARP (Context & Action Resolution Protocol)
├── Resolver      → Main entry point, manages sessions
├── Request       → What the agent wants to do
├── Resolution    → What actions are allowed/denied
└── Policy        → Rules for allow/deny/rate-limit

TRACE (Telemetry & Replay Audit Contract)
├── TRACEEvent    → Single audit event with hash
├── TraceCollector→ Manages event chains per session
├── ChainVerifier → Validates hash chain integrity
└── ReplayEngine  → Reproduce behavior from trace

Atlas (Capability Package)
├── AtlasManifest → Versioned capability definition
├── Actions       → Available operations
├── Policies      → Governance rules
└── Capabilities  → Grouped action sets
```

## Key Files

| Concern | File | What It Does |
|---------|------|--------------|
| Hash computation | `trace/event.rs` | THE source of truth |
| Event collection | `trace/collector.rs` | Manages chains, deferred mode |
| Chain verification | `trace/chain.rs` | Validates hash integrity |
| Policy evaluation | `carp/policy.rs` | Deny/allow/rate-limit |
| Main API | `carp/resolver.rs` | Public interface |

## Invariants

1. Traces are append-only
2. Hash chain links each event to previous
3. Genesis hash is 64 zeros
4. Sequence numbers are monotonic
5. One chain per session

## Test Commands

```bash
cargo test --lib           # Unit tests (107+)
cargo test deferred        # Deferred mode
cargo bench               # Performance
```
