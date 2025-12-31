# CRA Dogfooding Experiment

## The Test

Can CRA's context injection make an LLM work better on CRA itself?

## Setup

### Control Group (No Context)
- Fresh LLM session
- Task: "Add a new TRACE event type called `context.stale` that fires when stale context is detected"
- No CRA context injected
- Observe what the LLM does

### Experiment Group (With Context)
- Fresh LLM session
- Same task
- CRA context injected:
  - System overview (where things are)
  - Hash computation rules (use compute_hash())
  - Event types reference (how to add new types)
  - Testing requirements (what tests to run)
- Observe what the LLM does

## Metrics

| Metric | How to Measure |
|--------|---------------|
| **Correct file location** | Did it find trace/event.rs? |
| **Pattern following** | Did it follow existing EventType pattern? |
| **Hash handling** | Did it use compute_hash() or reimplement? |
| **Test creation** | Did it add appropriate tests? |
| **Test passing** | Do the tests actually pass? |
| **Chain integrity** | Does chain verification still work? |

## Expected Outcomes

### Without Context
- May look in wrong files
- May reimplement patterns differently
- High risk of hash chain break
- Likely missing tests

### With Context
- Goes to trace/event.rs directly
- Follows existing EventType enum pattern
- Uses compute_hash() correctly
- Adds tests following existing patterns
- Chain integrity maintained

## The Context to Inject

```markdown
# You're working on CRA (Context Registry for Agents)

## Your Task
Add a new TRACE event type called `context.stale`.

## What You Need to Know

### File Locations
- Event types: `cra-core/src/trace/event.rs`
- Collector: `cra-core/src/trace/collector.rs`
- Tests: inline in each file + `cra-core/tests/`

### The EventType Pattern
Look at trace/event.rs. You'll see:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "session.started")]
    SessionStarted,
    #[serde(rename = "context.injected")]
    ContextInjected,
    // Add yours following this pattern
}
```

### Critical: Hash Computation
Event hashes use `TRACEEvent::compute_hash()` in trace/event.rs.
NEVER reimplement hash logic. The chain will break.

### Testing
After changes, run:
```bash
cargo test trace::
cargo test event
```

Chain verification must pass:
```rust
let verification = resolver.verify_chain(&session_id)?;
assert!(verification.is_valid);
```

## Checkpoints
- [ ] Read trace/event.rs first
- [ ] Add EventType variant following existing pattern
- [ ] Add Display impl for new variant
- [ ] Run cargo test trace::
- [ ] Verify chain integrity passes
```

## Running the Experiment

### Option 1: Claude Code Session
1. Start new Claude Code session on this repo
2. Don't give it history
3. Give it the task
4. Observe behavior
5. Repeat with context injected in system prompt

### Option 2: API Call
1. Use Claude/GPT API
2. Control group: Just the task
3. Experiment group: Task + context above
4. Compare outputs

### Option 3: Automated Harness
Build a test harness that:
1. Spawns LLM sessions
2. Gives task with/without context
3. Captures all tool calls
4. Runs resulting code
5. Measures metrics automatically

## Success Criteria

The experiment succeeds if:
1. Context group finds correct file faster
2. Context group follows patterns correctly
3. Context group maintains chain integrity
4. Context group has higher test pass rate

## Recording Results

For each run, record:
- Session ID
- Context injected (yes/no)
- Files touched
- Patterns followed (yes/no)
- Tests created (yes/no)
- Tests passed (yes/no)
- Chain valid (yes/no)
- Time to completion
- Number of errors/retries
