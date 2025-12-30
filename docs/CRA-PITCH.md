# CRA - Context Registry for Agents

## Why It Exists

AI agents are proliferating. They write code, manage systems, interact with APIs, and make decisions. But they operate blind:

- **No shared knowledge** - Each agent starts from scratch, making the same mistakes
- **No audit trail** - When something goes wrong, no one knows what the agent did or why
- **No governance** - Policies exist in documentation agents may or may not read
- **No accountability** - Agents can't prove what they knew or agreed to

CRA exists to solve this. It provides a governance and context layer that agents actually use—because it helps them, not hinders them.

---

## What It Facilitates

### 1. Domain Knowledge That Actually Gets Used

Atlases are packages of context—the things agents need to know to work in a domain. Not generic documentation. Specific, actionable knowledge:

- How to use internal APIs correctly
- What mistakes to avoid
- What policies apply
- What tools are available

CRA injects this context at the right moments, so agents don't have to remember to look it up.

### 2. Audit Trails That Build Themselves

TRACE records what happened—every action, every context received, every decision. Hash-chained so it can't be tampered with. Async so it doesn't slow anything down.

When something goes wrong, you know exactly what the agent did and what information it had.

### 3. Governance That Doesn't Get In The Way

Policies are enforced, but only when they need to be. Most operations are async, cached, non-blocking. Agents don't avoid CRA because CRA doesn't slow them down.

High-risk actions can require sync verification. Everything else just flows.

### 4. Identity and Consent That Are Provable

Agents build their own wrapper during onboarding. This is their consent—they actively construct the integration. Their identity is verified by the wrapper's construction hash.

No "I didn't know" or "I didn't agree." The build process proves both.

---

## How It Fits

### The Agentic Coordination Landscape

```
┌─────────────────────────────────────────────────────────────────────┐
│                     AGENTIC SYSTEMS                                  │
│                                                                      │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │
│  │ Agent         │  │ Agent         │  │ Agent         │           │
│  │ Frameworks    │  │ Orchestration │  │ Tool Systems  │           │
│  │               │  │               │  │               │           │
│  │ LangChain     │  │ AutoGPT       │  │ MCP           │           │
│  │ CrewAI        │  │ BabyAGI       │  │ OpenAI Tools  │           │
│  │ etc.          │  │ etc.          │  │ etc.          │           │
│  └───────┬───────┘  └───────┬───────┘  └───────┬───────┘           │
│          │                  │                  │                    │
│          └──────────────────┼──────────────────┘                    │
│                             │                                       │
│                             ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                         CRA                                  │   │
│  │                                                              │   │
│  │  Context Layer: What should agents know?                    │   │
│  │  Governance Layer: What are agents allowed to do?           │   │
│  │  Audit Layer: What did agents actually do?                  │   │
│  │                                                              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                             │                                       │
│                             ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    EXECUTION                                 │   │
│  │                                                              │   │
│  │  APIs, Databases, File Systems, External Services           │   │
│  │                                                              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### CRA's Role

CRA is not an agent framework. It's not an orchestrator. It's not a tool system.

CRA is the **governance and context layer** that sits between agents and their execution environment.

| System Type | What It Does | CRA's Relationship |
|-------------|--------------|-------------------|
| Agent Frameworks | Define how agents work | CRA provides context/governance TO agents |
| Orchestration | Coordinate multiple agents | CRA governs each agent in the orchestra |
| Tool Systems (MCP) | Give agents capabilities | CRA works through MCP to reach agents |
| Execution | Where work happens | CRA audits what agents do here |

### Complementary, Not Competing

CRA works WITH other systems:

- **MCP**: CRA exposes its tools through MCP. The wrapper construction happens via MCP.
- **Agent Frameworks**: CRA provides context that frameworks inject into agents.
- **Orchestration**: Each agent in an orchestrated system can have its own CRA wrapper.
- **Tool Systems**: CRA can govern which tools agents use and how.

---

## The Core Protocol

### CARP - Context & Action Resolution Protocol

What context is relevant? What actions are allowed?

CARP takes a request (goal, context, action) and returns:
- Matched context blocks
- Policy decisions (allow, deny, require approval)
- Relevant governance rules

### TRACE - Telemetry & Replay Audit Contract

What happened?

TRACE records events in a hash chain:
- What context was provided
- What actions were taken
- What decisions were made
- When, in what order, by whom

Async by default. Tamper-evident always.

### Atlas - Versioned Context Packages

What should agents know?

Atlases are JSON packages containing:
- Context blocks (domain knowledge)
- Policies (governance rules)
- Actions (what's possible)
- Steward config (access, delivery, integrations)

---

## Why This Design

### Agent-Centric

Agents build their own wrapper. They understand what they're agreeing to because they constructed it. They own their integration.

### Lean By Default

Async TRACE. Cached context. Checkpoint-based injection. Most agents barely notice CRA is there—they just get helpful context and have an audit trail.

### Governance When Needed

Low-governance use cases get low overhead. High-governance use cases can add sync verification, blocking policies, real-time checks. It scales with need.

### Steward-Owned

Atlas creators control their content. Access rules. Delivery modes. External integrations. Alerts on usage. The IP stays with the creator.

---

## Summary

**CRA exists because agents need:**
- Context they actually use
- Audit trails that build themselves
- Governance that doesn't slow them down
- Identity and consent that are provable

**CRA facilitates:**
- Domain knowledge injection at the right moments
- Tamper-evident audit trails
- Policy enforcement when required
- Agent onboarding through wrapper construction

**CRA fits with other systems by:**
- Working through MCP to reach agents
- Providing context to agent frameworks
- Governing agents in orchestrated systems
- Auditing execution without blocking it

**The result:** Agents that know what they should know, do what they're allowed to do, and leave a trail of what they did.
