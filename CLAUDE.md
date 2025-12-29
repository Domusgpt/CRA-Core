# CRA-Core Development Context

## What This Project Is

CRA (Context Registry for Agents) is a governance layer for AI agents providing:
- **CARP**: Context & Action Resolution Protocol - what actions are allowed
- **TRACE**: Telemetry & Replay Audit Contract - cryptographic proof of what happened
- **Atlas**: Versioned packages defining agent capabilities and policies

Core principle: **"If it wasn't emitted by the runtime, it didn't happen."**

## Before You Write Code

### For ANY trace/event changes:
1. Read `cra-core/src/trace/event.rs` first
2. Use `TRACEEvent::compute_hash()` - never reimplement
3. Use `canonical_json()` for payload hashing - never `serde_json::to_string()`

### For hash chain operations:
1. Read `cra-core/src/trace/chain.rs`
2. Understand: hash = f(trace_version, event_id, trace_id, span_id, parent_span_id, session_id, sequence, timestamp, event_type, canonical_payload, previous_hash)

### For deferred mode:
1. Read `cra-core/src/trace/collector.rs`
2. Placeholder hash is literal string `"deferred"`
3. `flush()` calls `compute_hash()` - it doesn't reimplement

## Quick Reference

```
cra-core/src/
├── carp/           # CARP protocol
│   ├── resolver.rs # Main entry point - Resolver struct
│   ├── request.rs  # CARPRequest
│   ├── resolution.rs # CARPResolution
│   └── policy.rs   # Policy evaluation
├── trace/          # TRACE protocol
│   ├── event.rs    # TRACEEvent + compute_hash() [CANONICAL]
│   ├── collector.rs # TraceCollector + deferred mode
│   ├── chain.rs    # ChainVerifier
│   └── buffer.rs   # Lock-free ring buffer
├── atlas/          # Atlas manifests
├── timing/         # Timer backends
└── storage/        # Storage backends
```

## Running Tests

```bash
cargo test --lib                    # All unit tests
cargo test deferred                 # Deferred mode specifically
cargo test --test conformance       # Protocol conformance
cargo bench --bench resolver_bench  # Performance
```

## Current Branch

Development happens on `claude/cra-rust-refactor-*` branches.
Check git status before starting work.
