# CRA Testing Standards

## Overview

This document defines testing standards for CRA development, including test categories, coverage requirements, and the analysis framework for continuous improvement.

## Current Test Status

| Category | Tests | Status |
|----------|-------|--------|
| **Library Tests** | 117 | ✅ Passing |
| **Conformance Tests** | 12 | ✅ Passing |
| **Integration Tests** | 7 | ✅ Passing |
| **Total** | 136 | ✅ All Green |

### Test Distribution by Module

| Module | Test Count | Coverage Area |
|--------|------------|---------------|
| `atlas::*` | 16 | Manifest parsing, loading, validation |
| `carp::*` | 16 | Request/resolution, policy, resolver |
| `context::*` | 8 | Registry, matcher, injection |
| `trace::*` | 27 | Events, chain, collector, buffer |
| `timing::*` | 10 | Timers, rate limiting, batching |
| `error::*` | 7 | Error handling, categories |
| `storage::*` | 3 | Storage backends |
| `ffi::*` | 3 | FFI bindings |
| `conformance` | 12 | Protocol compliance, golden traces |
| `self_governance` | 7 | Context injection demo |

---

## Test Categories

### 1. Unit Tests (`--lib`)

Test individual functions and methods in isolation.

```bash
# Run all unit tests
cargo test --lib

# Run specific module tests
cargo test atlas::
cargo test trace::
cargo test context::
```

**Requirements:**
- Each public function should have at least one test
- Edge cases must be covered (empty input, max values, errors)
- No external dependencies (mock where needed)

### 2. Integration Tests (`tests/`)

Test components working together.

```bash
# Run all integration tests
cargo test --test self_governance

# Run with output
cargo test --test self_governance -- --nocapture
```

**Current Integration Tests:**

| Test | Purpose |
|------|---------|
| `test_hash_context_injection` | Verifies hash rules are injected when goal mentions "hash" |
| `test_deferred_mode_context_injection` | Verifies deferred mode docs are injected |
| `test_architecture_context_injection` | Verifies architecture overview is injected |
| `test_multi_context_injection` | Complex goals trigger multiple context blocks |
| `test_context_priority_ordering` | Higher priority blocks come first |
| `test_context_injection_audit_trail` | TRACE events are emitted for injections |
| `test_meta_context_injection` | Context about context system (META!) |

### 3. Conformance Tests (`tests/conformance.rs`)

Test protocol compliance.

```bash
cargo test --test conformance
```

**Conformance Areas:**
- CARP request/response format
- TRACE event structure
- Hash chain integrity
- Atlas manifest schema

### 4. Benchmark Tests (`benches/`)

Performance testing.

```bash
cargo bench --bench resolver_bench
```

---

## Test Requirements by Risk Level

### Critical (MUST test)
- Hash computation (`TRACEEvent::compute_hash()`)
- Chain verification
- Policy evaluation
- Context injection

### High (SHOULD test)
- Session management
- Action execution
- Event emission
- Deferred mode

### Medium (RECOMMENDED)
- Serialization/deserialization
- Error handling
- Builder patterns

### Low (OPTIONAL)
- Display implementations
- Debug formatting

---

## Running Tests

### Quick Check
```bash
# Just compilation
cargo check

# Fast tests
cargo test --lib
```

### Full Suite
```bash
# All tests with output
cargo test -- --nocapture

# Specific category
cargo test hash        # Hash-related
cargo test chain       # Chain verification
cargo test context     # Context injection
cargo test deferred    # Deferred mode
cargo test resolver    # Resolver tests
```

### Before Commit
```bash
# MANDATORY before any commit
cargo test --lib
cargo test --test self_governance
```

---

## Test Patterns

### Pattern 1: Setup-Execute-Verify

```rust
#[test]
fn test_context_injection() {
    // Setup
    let mut resolver = Resolver::new();
    resolver.load_atlas(create_test_atlas()).unwrap();
    let session_id = resolver.create_session("agent", "goal").unwrap();

    // Execute
    let request = CARPRequest::new(session_id, "agent", "goal with hash");
    let resolution = resolver.resolve(&request).unwrap();

    // Verify
    assert!(!resolution.context_blocks.is_empty());
    assert!(resolution.context_blocks.iter().any(|b| b.block_id == "expected"));
}
```

### Pattern 2: Chain Integrity Check

```rust
#[test]
fn test_chain_after_operation() {
    let mut resolver = Resolver::new();
    // ... perform operations ...

    // ALWAYS verify chain
    let verification = resolver.verify_chain(&session_id).unwrap();
    assert!(verification.is_valid, "Chain must be valid");
}
```

### Pattern 3: TRACE Event Verification

```rust
#[test]
fn test_events_emitted() {
    // ... perform operations ...

    let trace = resolver.get_trace(&session_id).unwrap();
    let events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == EventType::ContextInjected)
        .collect();

    assert!(!events.is_empty(), "Should emit context.injected events");
}
```

---

## Test Invariants

### Hash Chain Invariants (NEVER violate)

1. Each event's `previous_event_hash` equals prior event's `event_hash`
2. First event uses GENESIS_HASH (64 zeros)
3. Sequence numbers are monotonic (0, 1, 2...)
4. Hash = f(version, event_id, trace_id, ..., canonical_json(payload), previous_hash)

### Context Injection Invariants

1. Only matching contexts are injected
2. Higher priority blocks appear first
3. Each injection emits `context.injected` event
4. Content is never modified during injection

### Resolution Invariants

1. Allowed actions pass policy evaluation
2. Denied actions have policy_id and reason
3. context_blocks contains injected context
4. ttl_seconds is set

---

## Adding New Tests

### Checklist

- [ ] Test name is descriptive (`test_<what>_<condition>_<expectation>`)
- [ ] Setup is minimal and clear
- [ ] Single assertion per test (when possible)
- [ ] Chain verification for any TRACE operations
- [ ] No external dependencies
- [ ] Clean up any created resources

### Template

```rust
#[test]
fn test_feature_scenario_expectation() {
    // Arrange
    let mut resolver = Resolver::new();
    let atlas = create_test_atlas_with_context();
    resolver.load_atlas(atlas).unwrap();

    // Act
    let session_id = resolver.create_session("test", "test goal").unwrap();
    let result = resolver.some_operation(&session_id);

    // Assert
    assert!(result.is_ok());

    // Verify chain integrity (if applicable)
    let verification = resolver.verify_chain(&session_id).unwrap();
    assert!(verification.is_valid);
}
```

---

## Continuous Integration

### Pre-commit Checks
```bash
cargo check
cargo test --lib
cargo clippy -- -D warnings
```

### PR Checks
```bash
cargo test
cargo test --test conformance
cargo bench --bench resolver_bench -- --noplot
```

### Release Checks
```bash
cargo test --release
cargo test --test conformance --release
```

---

## Test Failure Analysis

### If Unit Tests Fail

1. Check the specific assertion that failed
2. Read the test to understand expected behavior
3. Check if recent changes broke the invariant

### If Chain Tests Fail

**Common Causes:**
1. Reimplemented hash logic (use `compute_hash()`)
2. Used `serde_json::to_string()` instead of `canonical_json()`
3. Changed hash component order
4. Modified sequence numbering

**Fix:** Read `trace/event.rs` and use existing implementations.

### If Context Tests Fail

**Common Causes:**
1. Keywords not matching goal text
2. Priority ordering wrong
3. Conditions not evaluated correctly

**Fix:** Check `context/matcher.rs` evaluate logic.

---

## Metrics & Goals

### Current Metrics
- **Test Count:** 124
- **Pass Rate:** 100%
- **Module Coverage:** All modules have tests

### Goals
- [ ] 150+ tests (add edge case coverage)
- [ ] Property-based testing for serialization
- [ ] Fuzzing for parser robustness
- [ ] Performance regression tests

---

## Quick Reference

```bash
# Check everything compiles
cargo check

# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_context_injection

# Run module tests
cargo test context::

# Run integration tests
cargo test --test self_governance

# Run benchmarks
cargo bench

# Before commit
cargo test --lib && cargo test --test self_governance
```
