# Hash Pattern Reference

When working with TRACE event hashes, ALWAYS use the canonical implementation.

## The One True Hash

Location: `cra-core/src/trace/event.rs`
Method: `TRACEEvent::compute_hash()`

```rust
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

## Rules

1. **NEVER** create a second hash implementation
2. **ALWAYS** call `event.compute_hash()`
3. **NEVER** use `serde_json::to_string()` for hash inputs
4. **ALWAYS** use `canonical_json()` for payloads (sorted keys)

## Why This Matters

Chain verification in `chain.rs` calls `compute_hash()` and compares.
If you compute differently, verification fails.

## Deferred Mode

In deferred mode, events get `event_hash = "deferred"` placeholder.
On `flush()`, we call `event.compute_hash()` - NOT a reimplementation.
