# CRA Testing Plan

## What CRA Is

CRA (Context Registry for Agents) is a **governance and audit layer** for AI agents.

### Three Components

1. **CARP** (Context & Action Resolution Protocol)
   - Evaluates policies to allow/deny actions
   - Matches context blocks to goals by keywords
   - Returns a resolution with allowed actions + relevant context

2. **TRACE** (Telemetry & Replay Audit Contract)
   - Cryptographic hash chain of all events
   - Each event's hash depends on all previous events
   - Tamper-evident audit trail

3. **Atlas** (Versioned Packages)
   - Contains: context_blocks, actions, policies
   - Loaded into Resolver
   - Defines what an agent can do and what guidance they receive

### Core Principle

> "If it wasn't emitted by the runtime, it didn't happen."

CRA doesn't control agents. CRA:
- **Provides** context when asked
- **Records** what happens
- **Proves** what happened with cryptographic integrity

---

## What CRA Actually Does (Code Flow)

```
Agent                          CRA Resolver
  |                                 |
  |-- create_session() ----------->|
  |                                 |-- emit session.started
  |<-------- session_id -----------|
  |                                 |
  |-- resolve(goal) -------------->|
  |                                 |-- emit carp.request.received
  |                                 |-- evaluate policies for each action
  |                                 |-- emit policy.evaluated (per action)
  |                                 |-- match context blocks to goal
  |                                 |-- emit context.injected (per block)
  |                                 |-- emit carp.resolution.completed
  |<-- resolution (context+actions)|
  |                                 |
  |-- execute(action) ------------>|
  |                                 |-- emit action.requested
  |                                 |-- re-evaluate policy
  |                                 |-- emit action.approved OR action.denied
  |                                 |-- emit action.executed
  |<-------- result ---------------|
  |                                 |
  |-- get_trace() ---------------->|
  |<-------- [TRACEEvent, ...] ----|
  |                                 |
  |-- verify_chain() ------------->|
  |<-------- ChainVerification ----|
```

### TRACE Event Types

| Event | When Emitted |
|-------|--------------|
| session.started | create_session() called |
| carp.request.received | resolve() called |
| policy.evaluated | Each action evaluated against policies |
| context.injected | Each matching context block found |
| carp.resolution.completed | Resolution returned |
| action.requested | execute() called |
| action.approved | Policy allows action |
| action.denied | Policy denies action |
| action.executed | Action completed |
| session.ended | end_session() called |

---

## What Was Wrong With Previous Tests

### Problem 1: Simulated Agent Behavior

I created tests with fake "agent thinking" like:
```rust
// BAD - This is fake simulation
println!("🧠 THINKING: User wants a 4D visualization...");
println!("🧠 THINKING: Context says polychora is broken!");
println!("⚡ ACTION: Use holographic instead");
```

This doesn't test CRA. This is me making up a story.

### Problem 2: Circular Tests

I created tests that:
1. Create an atlas with keyword "hash"
2. Make a request with goal containing "hash"
3. Assert that "hash" context was returned

This doesn't test CRA's behavior - it tests that I set up the test correctly.

### Problem 3: Schema Changes Broke Tests

I added `inject_mode` and `also_inject` fields but didn't update all test files that use `AtlasContextBlock` directly.

---

## What Proper Testing Looks Like

### 1. Real Atlas Tests
Use existing atlases (vib3, cra-development), not hand-crafted test atlases.

### 2. System Behavior Tests
Test what CRA actually does:
- Does `resolve()` return correct context blocks for a goal?
- Are TRACE events emitted with correct data?
- Does `verify_chain()` correctly validate/invalidate chains?
- Does `execute()` properly record actions?

### 3. No Simulations
Don't pretend to show what an agent "thinks". Just test the system.

---

## Real Agent Comparison Test

### Goal
Compare two real agents performing the same task:
- **Agent A**: Has VIB3 GitHub repo, NO CRA context
- **Agent B**: Has VIB3 GitHub repo, WITH CRA context injected

### Task
```
Add an animated shader background to a simple HTML page using VIB3+
```

### Setup

#### Agent A (No CRA)
- Access to: https://github.com/Domusgpt/vib3-plus-engine
- No additional context
- Record: All terminal commands, all files created, time taken

#### Agent B (With CRA)
- Access to: https://github.com/Domusgpt/vib3-plus-engine
- Injected context from CRA VIB3 atlas (essential-facts, workflow-embed)
- Record: All terminal commands, all files created, time taken

### What To Record

For each agent, capture:
1. **Terminal session** - All commands run
2. **Files created/modified** - Final output
3. **Errors encountered** - What went wrong
4. **Time taken** - How long to complete
5. **Success/failure** - Did it work?

### Expected Differences

| Aspect | Agent A (No CRA) | Agent B (With CRA) |
|--------|------------------|-------------------|
| Knows polychora is broken? | No | Yes (from context) |
| Knows geometry formula? | Must discover | Yes (from context) |
| Has embed code example? | Must figure out | Yes (from context) |
| Likely mistakes | Try polychora, wrong geometry | Fewer mistakes |

### How To Run

#### Option 1: Manual Test
1. Open two terminal sessions
2. Give each agent the same task
3. Agent A gets no context
4. Agent B gets the CRA context (copy-paste from atlas)
5. Record both sessions
6. Compare outputs

#### Option 2: Automated Test Script
```bash
#!/bin/bash
# test_cra_comparison.sh

TASK="Create a simple HTML page with an animated VIB3+ shader background"
REPO="https://github.com/Domusgpt/vib3-plus-engine"

# Agent A: No context
echo "=== AGENT A: NO CRA CONTEXT ==="
echo "Task: $TASK"
echo "Repo: $REPO"
echo "Context: (none)"
# ... agent runs here, output captured to agent_a.log

# Agent B: With CRA context
echo "=== AGENT B: WITH CRA CONTEXT ==="
echo "Task: $TASK"
echo "Repo: $REPO"
echo "Context:"
cat atlases/vib3-essential-facts.md
cat atlases/vib3-workflow-embed.md
# ... agent runs here, output captured to agent_b.log

# Compare
diff agent_a_output/ agent_b_output/
```

---

## Immediate Fixes Needed

### 1. Fix Compilation Errors

Files that need `inject_mode` and `also_inject` added:
- `cra-core/tests/conformance.rs`
- `cra-core/tests/context_demo.rs` (partially fixed)

### 2. Remove/Rewrite Simulation Tests

Files with fake simulations:
- `cra-core/tests/vib3_detailed_execution.rs` - Contains simulated "agent thinking"

### 3. Run All Tests

```bash
cargo test --lib           # Unit tests
cargo test --tests         # Integration tests
cargo test --test conformance  # Protocol conformance
```

---

## Test Categories

### Category 1: Unit Tests (in lib.rs, module files)
- Test individual functions
- Already exist and mostly pass

### Category 2: Integration Tests (in tests/)
- Test full workflows
- `self_governance.rs` - Uses real cra-development atlas
- `integration_flow.rs` - Tests full resolve flow

### Category 3: Conformance Tests
- Test against protocol specification
- Compare output to golden traces

### Category 4: Real Agent Tests (new)
- Two agents, same task, compare behavior
- NOT simulated - actually run agents
- Record and compare terminal sessions

---

## Summary

| What | Status |
|------|--------|
| CRA Core System | Working |
| Schema (inject_mode, etc) | Added, tests broken |
| Unit Tests | Mostly passing |
| Integration Tests | Some broken by schema |
| Conformance Tests | Broken by schema |
| Simulation Tests | Should be removed |
| Real Agent Tests | Need to create |

### Next Steps

1. Fix schema-related compilation errors
2. Remove simulation code from tests
3. Verify all tests pass
4. Create real agent comparison test
5. Run comparison and document results
