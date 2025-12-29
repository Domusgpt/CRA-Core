# CRA Context System Design

## The Problem With Current Approaches

Every existing context injection system has the same flaws:

| System | Flaw |
|--------|------|
| RAG | Retrieves chunks, not understanding. LLM gets fragments. |
| Keyword triggers | "hash" matches everything with "hash" - no intent |
| Static docs | Written for humans, stale immediately |
| System prompts | One-size-fits-all, no task awareness |
| MCP/tools | Good for actions, terrible for guidance |

**The fundamental mistake**: Treating LLMs like search engines that need "relevant documents" instead of colleagues that need "the right guidance at the right time."

---

## Design Principles

### 1. Context is a CONVERSATION, not a DOCUMENT

Wrong:
```
Here is documentation about hash computation:
[wall of text]
```

Right:
```
You're about to work on hash computation. Let me tell you what matters:

The hash logic lives in ONE place: trace/event.rs, lines 45-80.
DO NOT create a second implementation - I've seen this break the chain before.

The key insight is that JSON key order matters. Use canonical_json(), not serde_json::to_string().

If you want to verify your changes work: cargo test hash && cargo test chain

Questions I'd ask if I were you:
- Have you read trace/event.rs yet?
- Are you adding to compute_hash() or calling it?
```

### 2. Task-Phase Awareness

Context needs change based on WHERE you are in a task:

| Phase | What LLM Needs |
|-------|---------------|
| **Starting** | Overview, file locations, "where do I even begin" |
| **Exploring** | Relationships, dependencies, "how does X connect to Y" |
| **Implementing** | Exact patterns, examples, "show me the right way" |
| **Stuck** | Common mistakes, debugging, "what usually goes wrong" |
| **Verifying** | Test commands, invariants, "how do I know it works" |
| **Reviewing** | Checklist, edge cases, "what might I have missed" |

The system should detect phase from:
- Tool call patterns (lots of reads = exploring, edits = implementing)
- Explicit signals ("I'm stuck", "let me verify")
- Time since last progress

### 3. Layered Information

Every context should have three layers:

```
┌─────────────────────────────────────────────────────────────┐
│ LAYER 1: THE HEADLINE (always included, ~50 words)         │
│                                                             │
│ "Hash computation is in trace/event.rs. Never reimplement. │
│  Use compute_hash(). Key gotcha: canonical_json() for      │
│  deterministic key order."                                  │
└─────────────────────────────────────────────────────────────┘
                            ↓ if relevant
┌─────────────────────────────────────────────────────────────┐
│ LAYER 2: THE FULL CONTEXT (~500 words)                     │
│                                                             │
│ Complete explanation with examples, anti-patterns,          │
│ history of why it's this way, what breaks if violated.      │
└─────────────────────────────────────────────────────────────┘
                            ↓ if going deep
┌─────────────────────────────────────────────────────────────┐
│ LAYER 3: THE SOURCE                                         │
│                                                             │
│ "Read these files directly:                                 │
│  - trace/event.rs (lines 45-80) - the implementation       │
│  - trace/chain.rs - how verification uses it               │
│  - tests/trace_test.rs - see it in action"                 │
└─────────────────────────────────────────────────────────────┘
```

### 4. Checkpoint Rules

Context should include WHEN to re-check:

```yaml
checkpoints:
  before_modify:
    - "Before editing any file in trace/, re-read this context"
    - "Before creating any new hash function, STOP and ask why"

  after_action:
    - "After any edit to event.rs, run: cargo test hash"
    - "After completing implementation, run: cargo test chain"

  periodic:
    - "Every 5 edits, verify chain still passes"

  on_error:
    - "If tests fail with 'hash mismatch', you likely used wrong JSON serializer"
    - "If chain verification fails, check previous_event_hash linkage"
```

### 5. Living Context (Staleness Tracking)

Context knows when it might be outdated:

```yaml
context_block:
  id: hash-computation
  content: "..."

  sources:
    - file: trace/event.rs
      lines: 45-80
      hash_at_creation: "abc123"  # If file changes, context is stale

    - file: trace/chain.rs
      hash_at_creation: "def456"

  last_verified: "2024-01-15"

  staleness_check:
    trigger: "on_file_change"
    action: "flag_for_review"  # or "regenerate_from_archivist"
```

---

## The Format

NOT JSON. Context is delivered as natural language with structure:

```markdown
# Context: Hash Computation

## The Headline
Hash computation lives in trace/event.rs:45-80. Never reimplement.
Use compute_hash(). Use canonical_json() for deterministic serialization.

## What You Need to Know

[Full explanation in conversational tone]

## Examples

### Right Way
```rust
let hash = event.compute_hash();  // Call existing function
```

### Wrong Way (DON'T DO THIS)
```rust
let mut hasher = Sha256::new();
hasher.update(serde_json::to_string(&payload)?);  // BREAKS CHAIN
```

## Checkpoints
- [ ] Before modifying: Have you read trace/event.rs?
- [ ] After modifying: Run `cargo test hash && cargo test chain`
- [ ] If tests fail: Check canonical_json() vs serde_json

## Source Files
When you need the actual code:
- `trace/event.rs` lines 45-80 - The implementation
- `trace/chain.rs` - Verification logic
- Run `cargo test hash` to see tests

## Last Updated
This context describes code as of commit abc123.
If trace/event.rs has changed since, this may be stale.
```

---

## Trigger System

### Beyond Keywords

Current: `keywords: ["hash", "sha256"]` → dumb string matching

Better: **Intent Classification**

```yaml
triggers:
  intent_patterns:
    - intent: "modifying_hash_logic"
      signals:
        - goal_contains: ["hash", "compute", "sha256"]
        - files_touched: ["trace/event.rs", "trace/chain.rs"]
        - actions_requested: ["code.modify"]
      confidence_threshold: 0.7

    - intent: "debugging_chain_failure"
      signals:
        - error_contains: ["chain", "verification", "hash mismatch"]
        - recent_test_failure: true
      confidence_threshold: 0.5

  phase_detection:
    exploring:
      - "many Read calls, few Edit calls"
      - "questions in goal: 'how', 'where', 'what'"

    implementing:
      - "Edit calls increasing"
      - "goal contains: 'add', 'modify', 'implement'"

    stuck:
      - "repeated similar actions"
      - "error messages in recent output"
      - "goal contains: 'not working', 'failing', 'stuck'"
```

### Archivist Integration

For complex queries, the Archivist LLM synthesizes context:

```
Agent Goal: "I need to add a new hash verification mode that's faster"

Archivist receives:
- The goal
- Current file tree
- Recent agent actions
- Embeddings of all source files

Archivist responds (to be injected as context):
"You want faster hash verification. Here's what you need to know:

Current verification in trace/chain.rs is sequential - it walks the chain
event by event. This is O(n) where n = events.

Options for faster:
1. Parallel verification - verify chunks independently, merge results
2. Merkle tree - would require restructuring the chain
3. Sampling - verify random subset (trades accuracy for speed)

The constraint you'll hit: previous_event_hash linkage means events
MUST be verified in order unless you restructure.

I'd suggest starting with parallel chunk verification. Read trace/chain.rs
first to understand the current approach.

Files to read:
- trace/chain.rs - current verification
- trace/buffer.rs - has parallel primitives you could reuse

Want me to show you the current verify function?"
```

---

## Implementation Path

### Phase 1: Better Static Context (Now)
- Rewrite context blocks to use the layered format
- Add checkpoint rules
- Add source file references with line numbers
- Track file hashes for staleness

### Phase 2: Phase-Aware Injection (Next)
- Detect task phase from tool call patterns
- Inject different layers based on phase
- Add periodic checkpoints

### Phase 3: Archivist Integration (Future)
- LLM that maintains codebase understanding
- Synthesizes context for novel queries
- Can answer follow-up questions
- Regenerates stale context from source

---

## Success Metrics

How do we know if context is working?

| Metric | How to Measure |
|--------|---------------|
| **Mistake Prevention** | Compare error rate with/without context on same tasks |
| **Time to Completion** | Does context speed up task completion? |
| **Follow-through** | Do agents actually use checkpoint rules? |
| **Staleness** | How often is injected context out of date? |
| **Relevance** | Is injected context actually used in agent's work? |

---

## Open Questions

1. **How much context is too much?** Token budgets matter.
2. **How to detect phase reliably?** Current heuristics are weak.
3. **Archivist cost?** Running an LLM for every context query is expensive.
4. **Feedback loop?** How does the system learn what context helped?
