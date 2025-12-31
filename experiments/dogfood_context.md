# Context Injection for Dogfooding Experiment

Copy this into a new Claude Code session's CLAUDE.md or system prompt to test context injection.

---

# CRA Development Context

You're working on CRA (Context Registry for Agents), a Rust project.

## System Overview

CRA has three protocols:
- **CARP**: Permissions and policies (what can agents do)
- **TRACE**: Audit trail (what did agents do)
- **Context Registry**: Guidance injection (what do agents need to know)

Core principle: "If it wasn't emitted by the runtime, it didn't happen."

## Directory Structure

```
cra-core/src/
├── carp/           # CARP protocol
│   └── resolver.rs # Main entry point
├── trace/          # TRACE protocol
│   ├── event.rs    # ★ TRACEEvent, compute_hash(), EventType
│   ├── collector.rs # Event emission, deferred mode
│   └── chain.rs    # Hash chain verification
├── context/        # Context Registry
│   ├── registry.rs # Storage and querying
│   └── matcher.rs  # Condition matching
└── atlas/          # Atlas manifests
```

## Critical Rules

### Hash Computation
**NEVER reimplement hash logic.** Use `TRACEEvent::compute_hash()` in trace/event.rs.

```rust
// CORRECT
let hash = event.compute_hash();

// WRONG - breaks chain
let hash = sha256(serde_json::to_string(&event)?);
```

Use `canonical_json()` for deterministic key ordering, not `serde_json::to_string()`.

### Adding Event Types
In trace/event.rs, follow the existing pattern:

```rust
pub enum EventType {
    #[serde(rename = "session.started")]
    SessionStarted,
    // Add new types here with snake_case serde rename
}
```

Also update the `Display` impl and `as_str()` method.

### Testing
```bash
cargo test --lib          # All unit tests (117+)
cargo test trace::        # Trace module
cargo test chain          # Chain verification
```

Always verify chain integrity after changes:
```rust
let verification = resolver.verify_chain(&session_id)?;
assert!(verification.is_valid);
```

## Checkpoints

Before modifying trace/event.rs:
- [ ] Have you read the existing compute_hash() implementation?

After any trace changes:
- [ ] Run `cargo test trace::`
- [ ] Verify chain integrity passes

Before committing:
- [ ] All tests pass (`cargo test`)
- [ ] No new warnings (`cargo check`)

## Documentation

- `/CLAUDE.md` - Quick developer reference
- `/docs/VISION.md` - System architecture vision
- `/docs/TESTING_STANDARDS.md` - Test patterns
