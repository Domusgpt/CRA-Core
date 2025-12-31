# CRA Self-Governance: Eating Our Own Dog Food

## Overview

CRA (Context Registry for Agents) uses its own context injection system to govern development of CRA itself. This is the prime example of how context injection works and prevents common mistakes.

## The Problem This Solves

During CRA development, I (an LLM) made a critical mistake:
- I reimplemented hash computation with `serde_json::to_string()` instead of reading `trace/event.rs` and using `canonical_json()`
- This caused chain verification failures
- The fix was simply calling `event.compute_hash()`

**The solution**: Use CRA's Context Registry to inject the correct patterns whenever an LLM mentions "hash" in their goal.

## How It Works

### 1. The Self-Governance Atlas

Located at `atlases/cra-development.json`, this atlas contains:

```
dev.cra.self-governance
├── 13 context_blocks with keywords
├── 3 policies (deny, requires_approval)
├── 12 actions (trace.*, carp.*, context.*, atlas.*)
└── 4 capabilities
```

### 2. Context Blocks with Keywords

Each context block has keywords that trigger injection:

```json
{
  "context_id": "hash-computation-rule",
  "priority": 200,
  "content": "CRITICAL: Use TRACEEvent::compute_hash()...",
  "keywords": ["hash", "compute_hash", "sha256", "chain", "verification"]
}
```

### 3. The Injection Flow

```
LLM Goal: "I need to modify the hash computation"
    │
    ▼
┌───────────────────────────────────────┐
│         CARPRequest.goal              │
│  "modify the hash computation"        │
└───────────────────────────────────────┘
    │
    │ Resolver.resolve()
    ▼
┌───────────────────────────────────────┐
│         ContextRegistry.query()       │
│  Finds keywords: "hash", "computation"│
└───────────────────────────────────────┘
    │
    │ ContextMatcher.evaluate()
    ▼
┌───────────────────────────────────────┐
│    Matched Context Blocks             │
│  1. read-before-modify (250)          │
│  2. hash-computation-rule (200)       │
│  3. chain-invariants (190)            │
│  4. event-types-reference (130)       │
│  5. module-boundaries (105)           │
└───────────────────────────────────────┘
    │
    │ CARPResolution.context_blocks
    ▼
┌───────────────────────────────────────┐
│    LLM Receives Context               │
│  "CRITICAL: Use compute_hash()..."    │
│  "NEVER reimplement hash logic..."    │
└───────────────────────────────────────┘
```

### 4. TRACE Audit Trail

Every injection emits a `context.injected` event:

```json
{
  "event_type": "context.injected",
  "payload": {
    "context_id": "hash-computation-rule",
    "source_atlas": "dev.cra.self-governance",
    "priority": 200,
    "match_score": 220,
    "token_estimate": 236
  }
}
```

This creates a cryptographic audit trail of what context was provided.

## Running the Self-Governance Tests

```bash
# Run the self-governance tests with output
cargo test --test self_governance -- --nocapture

# Run a specific test
cargo test --test self_governance test_hash_context_injection -- --nocapture
```

### Test Output Example

```
=== Multi-Context Injection Test ===
Goal: I need to modify the hash chain verification in the trace module

Injected 7 context blocks:

--- read-before-modify (priority: 250) ---
# MANDATORY: Read Before Modify
**You MUST read any file before modifying it.**
...

--- hash-computation-rule (priority: 200) ---
# CRITICAL: Hash Computation
**NEVER reimplement hash logic. Use TRACEEvent::compute_hash()...**
...

Chain verification: VALID
```

## Context Block Reference

| Context ID | Priority | Keywords | When Injected |
|------------|----------|----------|---------------|
| `read-before-modify` | 250 | read, before, modify, first | Any high-risk operation |
| `hash-computation-rule` | 200 | hash, compute_hash, sha256 | Hash-related work |
| `chain-invariants` | 190 | chain, invariant, verify | Chain verification work |
| `deferred-mode-pattern` | 180 | deferred, flush, buffer | Deferred mode work |
| `context-registry-usage` | 170 | context, registry, inject | Context system work |
| `context-matcher-conditions` | 160 | matcher, conditions, score | Matcher work |
| `carp-resolver-flow` | 150 | resolver, resolve, carp | Resolver work |
| `atlas-manifest-structure` | 140 | atlas, manifest, schema | Atlas work |
| `event-types-reference` | 130 | event, EventType, trace | Event handling |
| `testing-requirements` | 120 | test, testing, cargo | Testing work |
| `error-handling-patterns` | 110 | error, CRAError, Result | Error handling |
| `module-boundaries` | 105 | module, structure, directory | Understanding codebase |
| `cra-architecture-overview` | 100 | architecture, overview, cra | General questions |

## Meta: Context About Context

The most elegant aspect is when an LLM asks about the context system itself:

```
Goal: "I want to add a new matching condition to the context registry"
    │
    ▼
Injected: context-registry-usage, context-matcher-conditions
    │
    ▼
LLM receives documentation about how to modify the very system
that's providing the documentation!
```

## Using This Pattern in Your Projects

1. **Create an atlas** for your project with context_blocks
2. **Add keywords** that trigger relevant context
3. **Set priorities** (higher = more important)
4. **Load the atlas** when your LLM agent starts
5. **Include goal** in CARPRequest
6. **Use context_blocks** from CARPResolution

Example:

```rust
let mut resolver = Resolver::new();
resolver.load_atlas(my_project_atlas)?;

let session_id = resolver.create_session("my-agent", "Project development")?;

let request = CARPRequest::new(
    session_id,
    "my-agent".to_string(),
    "I need to modify the authentication system".to_string(), // Goal with keywords
);

let resolution = resolver.resolve(&request)?;

// resolution.context_blocks now contains relevant context
// like "auth-security-rules", "session-handling", etc.
for block in &resolution.context_blocks {
    println!("Context: {} - {}", block.block_id, block.name);
    // Inject block.content into LLM's system prompt
}
```

## The Key Insight

**Context injection prevents mistakes by providing relevant information at the right time.**

Instead of hoping an LLM remembers that `compute_hash()` exists, we automatically inject that knowledge whenever the goal mentions "hash". The LLM can't forget because the context is always provided when needed.

This is the "C" in CRA - **Context** as a first-class governance mechanism.
