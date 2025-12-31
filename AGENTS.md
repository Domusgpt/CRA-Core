# AGENTS.md - AI Agent Instructions for CRA-Core

> This file provides context injection for AI agents working on this codebase.
> CRA itself is designed to solve the problem of LLMs not having critical context.
> We dogfood this by using structured context injection for development.

## Critical Architecture Patterns

### Hash Computation - DO NOT DUPLICATE

The TRACE hash chain uses a **specific byte sequence format**. There is ONE canonical implementation:

```rust
// CANONICAL LOCATION: cra-core/src/trace/event.rs
// METHOD: TRACEEvent::compute_hash()

pub fn compute_hash(&self) -> String {
    let mut hasher = Sha256::new();

    hasher.update(self.trace_version.as_bytes());
    hasher.update(self.event_id.as_bytes());
    hasher.update(self.trace_id.as_bytes());
    hasher.update(self.span_id.as_bytes());
    hasher.update(self.parent_span_id.as_deref().unwrap_or("").as_bytes());
    hasher.update(self.session_id.as_bytes());
    hasher.update(self.sequence.to_string().as_bytes());
    hasher.update(self.timestamp.to_rfc3339().as_bytes());
    hasher.update(self.event_type.as_str().as_bytes());
    hasher.update(canonical_json(&self.payload).as_bytes());
    hasher.update(self.previous_event_hash.as_bytes());

    hex::encode(hasher.finalize())
}
```

**NEVER** create a second hash implementation. Always call `event.compute_hash()`.

**WHY**: Hash verification (`ChainVerifier`) compares against this exact format.
If you use JSON serialization or different field order, hashes won't match.

### Canonical JSON for Payloads

The `canonical_json()` function in `event.rs` sorts object keys deterministically:

```rust
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<_> = map.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            // ...
        }
        // ...
    }
}
```

**DO NOT** use `serde_json::to_string()` for hash inputs - key order is not guaranteed.

### Chain State Management

```
SessionTrace {
    trace_id: String,      // Immutable per session
    events: Vec<TRACEEvent>,
    sequence: u64,         // Next sequence number
    last_hash: String,     // Hash of most recent event
}
```

When adding events:
1. Set `event.sequence = session.sequence`
2. Set `event.previous_event_hash = session.last_hash`
3. Compute hash: `event.event_hash = event.compute_hash()`
4. Update: `session.sequence += 1; session.last_hash = event.event_hash`

### Deferred Mode Contract

In deferred mode:
- `emit()` creates event with `event_hash = "deferred"` (placeholder)
- `flush()` calls `event.compute_hash()` to fill real hashes
- Buffer tracks pending count, but events are already in session.events

## File Ownership

| Pattern | Owner | Notes |
|---------|-------|-------|
| Hash computation | `trace/event.rs` | Single source of truth |
| Chain verification | `trace/chain.rs` | Uses `compute_hash()` to verify |
| Event storage | `trace/collector.rs` | Manages SessionTrace |
| Deferred processing | `trace/collector.rs` | Uses `compute_hash()` on flush |

## Before Implementing New Features

1. **Read the relevant module first** - Don't assume format/structure
2. **Check for existing methods** - Especially for crypto operations
3. **Run tests before AND after** - `cargo test --lib`
4. **Check chain verification** - Any trace change must pass `verify_chain()`

## Protocol Versions

- CARP: 1.0 (stable)
- TRACE: 1.0 (stable)
- Atlas: 1.0 (stable)

## Common Mistakes to Avoid

### 1. Reimplementing Hash Logic
```rust
// WRONG - Different byte sequence
let json = serde_json::json!({ "session_id": ... });
let hash = Sha256::digest(json.to_string());

// CORRECT - Use canonical method
let hash = event.compute_hash();
```

### 2. Forgetting Fields in Hash
The hash includes `trace_version` and `span_id` which are easy to forget.

### 3. Non-Deterministic JSON
```rust
// WRONG - Key order not guaranteed
serde_json::to_string(&payload)

// CORRECT - Sorted keys
canonical_json(&payload)
```

### 4. Modifying Events After Hashing
Once `compute_hash()` is called, the event is immutable.
Any field change invalidates the hash.

## Testing Requirements

Before any PR:
```bash
cargo test --lib           # 107+ tests must pass
cargo test deferred        # Deferred mode tests
cargo bench               # No performance regression
```

## Architecture Invariants

1. **Append-only traces** - Events cannot be modified or deleted
2. **Hash chain integrity** - Each event references previous hash
3. **Genesis hash** - First event uses `GENESIS_HASH` (64 zeros)
4. **Monotonic sequences** - Sequence numbers never decrease
5. **Single trace per session** - One hash chain per session_id
