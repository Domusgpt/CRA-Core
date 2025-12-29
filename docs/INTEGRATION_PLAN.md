# CRA Real-World Integration Plan

## The Problem With My Previous "Test"

I manually wrote different prompts for two agents. That tests nothing about CRA - it just tests whether more info helps (obviously yes).

**The REAL test**: Does CRA's Context Registry correctly identify and inject the right context for a given goal, and does that help the agent?

---

## What End-to-End Actually Looks Like

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           AGENT WORKFLOW                                 │
└─────────────────────────────────────────────────────────────────────────┘

1. Agent starts with a GOAL
   "I need to add a new TRACE event type for stale context detection"

                                    ↓

2. Agent (or framework) calls CRA
   ┌─────────────────────────────────────────────────────────────────────┐
   │ POST /resolve                                                        │
   │ {                                                                    │
   │   "agent_id": "claude-session-xyz",                                  │
   │   "goal": "add a new TRACE event type for stale context detection",  │
   │   "context_hints": ["trace", "event"]                                │
   │ }                                                                    │
   └─────────────────────────────────────────────────────────────────────┘

                                    ↓

3. CRA processes the request
   ┌─────────────────────────────────────────────────────────────────────┐
   │ Resolver.resolve():                                                  │
   │   - Evaluates policies → what actions allowed                        │
   │   - Queries ContextRegistry with goal                                │
   │   - Matches keywords: "TRACE", "event", "type"                       │
   │   - Selects relevant context blocks:                                 │
   │     * event-types-reference (priority 130)                           │
   │     * hash-computation-rule (priority 200)                           │
   │     * module-boundaries (priority 105)                               │
   │   - Renders context for LLM consumption                              │
   │   - Emits TRACE events for audit                                     │
   └─────────────────────────────────────────────────────────────────────┘

                                    ↓

4. CRA returns resolution WITH context
   ┌─────────────────────────────────────────────────────────────────────┐
   │ {                                                                    │
   │   "decision": "allow",                                               │
   │   "allowed_actions": [...],                                          │
   │   "context_blocks": [                                                │
   │     {                                                                │
   │       "block_id": "hash-computation-rule",                           │
   │       "headline": "NEVER reimplement hash. Use compute_hash().",     │
   │       "full_context": "...",                                         │
   │       "source_refs": [{"path": "trace/event.rs", "lines": [45,80]}]  │
   │     },                                                               │
   │     ...                                                              │
   │   ],                                                                 │
   │   "rendered_context": "## Context for your task\n\n..."              │
   │ }                                                                    │
   └─────────────────────────────────────────────────────────────────────┘

                                    ↓

5. Agent receives context as part of its working context
   The framework injects resolution.rendered_context into the agent's
   system prompt or context window.

                                    ↓

6. Agent works on the task WITH the injected context
   - Knows to go to trace/event.rs
   - Knows to use compute_hash()
   - Follows EventType pattern

                                    ↓

7. TRACE records everything
   - What context was injected
   - What actions the agent took
   - Whether checkpoints were followed
   - Final outcome
```

---

## What We Need to Build

### Phase 1: Context Rendering

The CRA resolution needs to RENDER context in a way an LLM can consume.

```rust
impl CARPResolution {
    /// Render all context blocks into a single LLM-ready string
    pub fn render_context(&self, phase: TaskPhase) -> String {
        let mut output = String::new();
        output.push_str("# Context for Your Task\n\n");

        for block in &self.context_blocks {
            output.push_str(&block.render_for_phase(phase));
            output.push_str("\n---\n");
        }

        output
    }
}
```

### Phase 2: Agent Integration Point

How does an agent actually CALL CRA? Options:

| Integration | Pros | Cons |
|-------------|------|------|
| **MCP Server** | Standard, works with Claude/GPT | Requires running server |
| **HTTP API** | Simple, universal | Requires running server |
| **CLI** | Easy to test | Not for production |
| **Claude Code Hook** | Direct integration | Specific to Claude Code |
| **Library** | Embedded, fast | Tight coupling |

**For dogfooding**: Start with CLI that outputs rendered context.

```bash
$ cra-context "I need to add a new TRACE event type"

# Context for Your Task

## Hash Computation Rule
NEVER reimplement hash logic. Use compute_hash() in trace/event.rs.
...

## Event Types Reference
Event types are defined in trace/event.rs. Follow the pattern:
...
```

### Phase 3: Test Harness

A harness that:
1. Takes a task description
2. Calls CRA to get context
3. Spawns an agent WITH that context
4. Runs the agent on the task
5. Evaluates the result

```rust
struct DogfoodTest {
    task: String,
    expected_files: Vec<String>,
    expected_patterns: Vec<String>,
    forbidden_patterns: Vec<String>,  // e.g., "sha256::new" outside compute_hash
}

impl DogfoodTest {
    async fn run(&self) -> TestResult {
        // 1. Get context from CRA
        let resolution = cra.resolve(&self.task)?;
        let context = resolution.render_context(TaskPhase::Starting);

        // 2. Spawn agent with context
        let agent = spawn_agent_with_context(&context);

        // 3. Give task to agent
        let result = agent.execute(&self.task).await?;

        // 4. Evaluate
        let files_touched = result.files_modified();
        let code_written = result.code_changes();

        TestResult {
            correct_files: self.expected_files.iter()
                .all(|f| files_touched.contains(f)),
            patterns_followed: self.expected_patterns.iter()
                .all(|p| code_written.contains(p)),
            forbidden_avoided: self.forbidden_patterns.iter()
                .all(|p| !code_written.contains(p)),
            tests_pass: result.run_tests()?,
        }
    }
}
```

### Phase 4: Real Dogfood

Use CRA to govern development of CRA itself:

1. **Session Start Hook**: When Claude Code opens CRA repo, call CRA
2. **Goal Detection**: Extract goal from first message
3. **Context Injection**: Add rendered context to system prompt
4. **Checkpoint Monitoring**: Watch for trigger conditions
5. **Audit Logging**: Record what context was used, outcome

---

## Development Roadmap

### Week 1: Foundation
- [ ] Add `render_context()` to CARPResolution
- [ ] Add `rendered_context` field to resolution
- [ ] Create CLI tool: `cra-context <goal>`
- [ ] Verify context blocks render correctly

### Week 2: Integration
- [ ] Create MCP server wrapper
- [ ] Test with Claude Desktop / Claude Code
- [ ] Add session-start hook for Claude Code
- [ ] Verify context reaches agent

### Week 3: Testing
- [ ] Build test harness
- [ ] Define dogfood test cases
- [ ] Run A/B tests (with/without context)
- [ ] Measure and iterate

### Week 4: Feedback Loop
- [ ] Add TRACE events for context usage
- [ ] Analyze which context correlates with success
- [ ] Refine context blocks based on data
- [ ] Document learnings

---

## Success Metrics

| Metric | How to Measure | Target |
|--------|---------------|--------|
| Context Relevance | % of injected context used by agent | >80% |
| Error Prevention | Mistakes avoided due to context | Measurable reduction |
| Task Completion | Successfully completed tasks | Higher with context |
| Time to Completion | Time from start to finish | Faster with context |
| Chain Integrity | Hash chain remains valid | 100% |

---

## The Real Dogfood Test

**Setup**:
1. New Claude Code session on CRA repo
2. CRA session-start hook fires
3. Agent's goal extracted from first message
4. CRA resolves goal → injects context
5. Context appears in agent's context window

**Task**: "Modify the context matching algorithm to use embeddings"

**What CRA Should Inject**:
- Context Registry architecture
- Current matcher implementation
- Where embeddings would integrate
- What NOT to break (hash chain, etc.)

**What We Observe**:
- Did agent find the right files?
- Did agent understand the architecture?
- Did agent preserve invariants?
- Did tests pass?

**Comparison**:
- Run same task WITHOUT CRA context injection
- Compare outcomes
- Quantify the difference

This is the REAL test. Not manually writing prompts - actually using the system.
