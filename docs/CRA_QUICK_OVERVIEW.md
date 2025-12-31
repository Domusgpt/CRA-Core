# CRA: Quick Overview

## What is CRA?

**Context Registry for Agents** - A governance layer that provides:
- **CARP**: Policy evaluation (what actions are allowed)
- **TRACE**: Cryptographic audit trail (proof of what happened)
- **Atlas**: Versioned packages (context + actions + policies together)

## Where CRA Fits

```
┌─────────────────────────────────────────────────────┐
│ APPLICATION:  LangChain, CrewAI, Custom Agents      │
└───────────────────────┬─────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────┐
│ CAPABILITY:   Claude Skills, OpenAI Function Calls  │
└───────────────────────┬─────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────┐
│ PROTOCOL:     MCP (Model Context Protocol)          │
└───────────────────────┬─────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────┐
│ GOVERNANCE:   CRA  ← You are here                   │
│               Proof + Policy + Audit                │
└─────────────────────────────────────────────────────┘
```

## CRA vs Others

| System | What It Does | What's Missing |
|--------|--------------|----------------|
| **Claude Skills** | Teach agents how to work | No audit trail, no proof |
| **OpenAI Agents** | Orchestrate multi-agent workflows | Tracing for debug only, not proof |
| **MCP** | Connect agents to external systems | No governance layer |
| **CRA** | **Prove what happened + Govern what's allowed** | - |

## Core Principle

> **"If it wasn't emitted by the runtime, it didn't happen."**

Every action produces a TRACE event. Events form a hash chain. Tampering breaks the chain.

## What CRA Actually Outputs

When you call `resolver.resolve(request)`:

```rust
CARPResolution {
    decision: Allow | Deny,
    allowed_actions: vec!["code.write", "file.read"],
    context_blocks: vec![
        InjectedContext {
            block_id: "vib3-essential-facts",
            priority: 350,
            content: "# VIB3+ Essential Facts\n\n## Working Systems..."
        }
    ],
    policies_applied: vec!["default-allow"],
}
```

And these TRACE events are recorded:
```
[seq 0] session.started           hash: a7b3c2...
[seq 1] carp.request.received     hash: f4e2d1...
[seq 2] context.injected          hash: 9c8b7a...  ← Proof of guidance
[seq 3] carp.resolution.completed hash: 2d3e4f...
```

## The Actual CRA Effect

**Without CRA**: Agent receives goal, no context, figures it out alone.

**With CRA**: Agent receives goal + relevant context blocks + cryptographic proof of what was provided.

The `context.injected` events in TRACE are the key:
- They prove exactly what guidance the agent received
- They're cryptographically linked (hash chain)
- They can't be forged or modified after the fact

## When to Use CRA

- **Enterprise AI**: Audit trail for compliance
- **High-stakes agents**: Proof of correct operation
- **Multi-agent systems**: Governance across handoffs
- **Regulated industries**: Legal evidence of agent behavior

## Quick Start

```rust
let mut resolver = Resolver::new();
resolver.load_atlas(atlas)?;

let session = resolver.create_session("agent-id", "description")?;
let request = CARPRequest::new(session, agent_id, goal);
let resolution = resolver.resolve(&request)?;

// resolution.context_blocks = what agent should see
// resolver.get_trace(&session) = cryptographic proof
// resolver.verify_chain(&session) = integrity check
```

---

*For full analysis, see [CRA_LANDSCAPE_ANALYSIS.md](./CRA_LANDSCAPE_ANALYSIS.md)*
