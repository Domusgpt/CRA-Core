# CRA Vision Document

## What Is This System?

CRA is a **governance layer for AI agents**. It answers three questions:

1. **What can the agent do?** → CARP (permissions, policies)
2. **What did the agent do?** → TRACE (audit trail, proof)
3. **What does the agent need to know?** → Context Registry

The third question is the hardest and most important.

---

## The Problem We're Solving

When an AI agent works on a task, it needs context to do the job well. Currently:

| Approach | Problem |
|----------|---------|
| System prompts | Static, one-size-fits-all, gets stale |
| RAG | Retrieves fragments, no understanding of task |
| Documentation | Written for humans, not LLMs |
| Examples | Good but not adaptive to situation |

**The real problem**: No existing system gives an LLM the right information at the right time in the right format.

---

## The Vision

### Context is a Conversation

The context system should behave like a knowledgeable colleague sitting next to the agent:

- **Before starting**: "Here's what you're about to work on. The critical thing to know is..."
- **During work**: "Heads up, you're about to touch a sensitive file. Remember..."
- **When stuck**: "I've seen this error before. Usually it means..."
- **Before finishing**: "Before you're done, make sure you've..."

Not a wall of documentation. A conversation.

### Three Layers of Information

Every piece of context has three layers:

```
┌────────────────────────────────────────────────────────────────┐
│ LAYER 1: THE HEADLINE                                          │
│ "Hash computation lives in trace/event.rs. Never reimplement.  │
│ Use compute_hash()."                                           │
│ (~50 words, ALWAYS delivered)                                  │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│ LAYER 2: THE FULL CONTEXT                                      │
│ Why it matters, examples of right/wrong, common mistakes,      │
│ history of how it got this way, what breaks if violated.       │
│ (Delivered when task is relevant)                              │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│ LAYER 3: THE SOURCE                                            │
│ "Read trace/event.rs lines 45-80 for the implementation."      │
│ (Delivered when agent needs actual code)                       │
└────────────────────────────────────────────────────────────────┘
```

### Phase Awareness

An agent goes through phases. Context needs change:

| Phase | What Agent Needs |
|-------|-----------------|
| **Starting** | Overview. What is this system? Where do I look? |
| **Exploring** | Relationships. How does X connect to Y? |
| **Implementing** | Specifics. Show me the right pattern. |
| **Stuck** | Debugging. What usually goes wrong here? |
| **Verifying** | Checklists. How do I know it works? |
| **Reviewing** | Completeness. What might I have missed? |

The system detects phase from:
- Tool usage patterns (reads → exploring, edits → implementing)
- Explicit signals ("I'm stuck", "let me verify")
- Error presence (test failures → stuck)

### Checkpoints

Context includes rules for when to re-check:

```
BEFORE modifying trace/event.rs:
  → Re-read hash computation context
  → Verify you've read the existing implementation

AFTER any edit:
  → Run: cargo test hash

IF tests fail with "hash mismatch":
  → You probably used serde_json instead of canonical_json

BEFORE completing:
  → Verify chain integrity passes
  → Run: cargo test chain
```

These aren't suggestions. They're rules the system enforces.

### Staleness

Context knows when it might be outdated:

```yaml
context:
  id: hash-computation
  describes_files:
    - path: trace/event.rs
      lines: 45-80
      hash_when_written: abc123

  last_verified: 2024-01-15
```

When `trace/event.rs` changes, the context is marked stale. Options:
1. Flag to human to update
2. Regenerate from source (if archivist available)
3. Warn agent that context may be outdated

### The Archivist (Future)

For complex queries, an LLM that deeply understands the codebase:

```
Agent: "I need to add a faster hash verification mode"

Archivist:
"You want faster verification. Here's what I know:

Current approach (trace/chain.rs) is sequential O(n).
The constraint is that events link via previous_event_hash,
so you can't verify out of order without restructuring.

Options:
1. Parallel chunk verification (verify chunks, merge)
2. Sampling (verify subset, trade accuracy)
3. Merkle tree (major restructure)

I'd start with parallel chunks. The buffer.rs module
has concurrent primitives you could reuse.

Read chain.rs lines 50-100 for the current verify loop."
```

Not fragments. Understanding.

---

## How It Fits Together

```
┌─────────────────────────────────────────────────────────────────────┐
│                            ATLAS                                     │
│  Defines: actions, policies, capabilities, AND context blocks        │
└─────────────────────────────────────────────────────────────────────┘
                                 ↓ loaded into
┌─────────────────────────────────────────────────────────────────────┐
│                           RESOLVER                                   │
│  - Evaluates policies (what's allowed)                              │
│  - Queries ContextRegistry (what's relevant)                         │
│  - Detects task phase (when in the work)                            │
│  - Renders context for LLM (how to present)                         │
└─────────────────────────────────────────────────────────────────────┘
                                 ↓ produces
┌─────────────────────────────────────────────────────────────────────┐
│                         CARP RESOLUTION                              │
│  - allowed_actions: what agent can do                               │
│  - denied_actions: what agent cannot do                              │
│  - context_blocks: what agent needs to know (layered, phase-aware)   │
│  - checkpoints: when to re-check                                    │
└─────────────────────────────────────────────────────────────────────┘
                                 ↓ recorded by
┌─────────────────────────────────────────────────────────────────────┐
│                            TRACE                                     │
│  - context.injected: what context was given                         │
│  - checkpoint.triggered: when reminders fired                        │
│  - task.phase.changed: when phase was detected                      │
│  - (feedback loop: which context correlated with success?)           │
└─────────────────────────────────────────────────────────────────────┘
```

The key insight: **TRACE closes the loop**. We can analyze:
- Which context was given for successful tasks?
- Which checkpoints prevented mistakes?
- Which phases got stuck most often?

This data improves context selection over time.

---

## What Makes This Better

| Feature | Current Systems | CRA Vision |
|---------|----------------|------------|
| **Timing** | All upfront | Phase-aware, delivered when needed |
| **Format** | Human docs | LLM-native, conversational |
| **Depth** | One level | Three layers (headline → full → source) |
| **Reminders** | None | Checkpoint rules |
| **Freshness** | Unknown | Staleness tracking |
| **Learning** | None | TRACE enables feedback loop |
| **Synthesis** | Fragments | Archivist understands holistically |

---

## Open Questions

1. **How to detect phase reliably?** Heuristics are weak.
2. **Token budget?** Can't inject infinite context.
3. **Archivist cost?** LLM for every query is expensive.
4. **Feedback signal?** How to know if context helped?
5. **Staleness regeneration?** Who updates stale context?

---

## Next Steps

1. **Design the data model** - What does a ContextBlock actually contain?
2. **Design phase detection** - What signals indicate phase?
3. **Design checkpoint system** - How are rules specified and enforced?
4. **Design staleness tracking** - How to track and handle stale context?
5. **Prototype without archivist** - What can we do with static context + good structure?

---

## The Test

If this system works, an AI agent working on CRA should:

1. **Get hash warning automatically** when goal mentions "hash"
2. **See different detail** based on phase (starting vs implementing)
3. **Receive checkpoint reminders** before modifying critical files
4. **Be warned** if context might be stale
5. **Leave a TRACE** that shows what context was used

If an agent that received CRA context makes fewer mistakes than one without, we've succeeded.
