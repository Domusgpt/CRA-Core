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

---

## What CRA Does

In plain terms:

**1. Gives agents the right knowledge at the right time**

You have an agent working with your internal API. Instead of hoping it read the docs, CRA injects the relevant context when the agent needs it. Not everything—just what's relevant to what it's doing right now.

**2. Records everything without slowing anything down**

Every action the agent takes gets logged in a tamper-proof chain. But it happens in the background. The agent doesn't wait. You get a complete audit trail without the overhead.

**3. Enforces rules only when it matters**

Most operations just flow. But when the agent tries to do something risky—delete production data, access sensitive systems—CRA can require verification. Governance scales with risk.

**4. Proves what agents knew and agreed to**

The agent builds its own integration wrapper. That construction is recorded. You can prove exactly what context the agent received, what rules it agreed to, and when.

---

## How It Solves Issues

### Issue: Agents Make Preventable Mistakes

They don't read documentation. They hallucinate APIs. They make the same errors repeatedly.

**CRA Solution:** Context injection at checkpoints. The agent doesn't need to remember to look things up—relevant knowledge appears when needed.

### Issue: No One Knows What Agents Did

Something went wrong. The agent made changes. But there's no log of its reasoning, what it tried, what failed.

**CRA Solution:** TRACE records everything. Hash-chained so it can't be altered. When you need to know what happened, you have cryptographic proof.

### Issue: Governance Slows Everything Down

Every action requires approval. Agents wait for verification. Work grinds to a halt.

**CRA Solution:** Async by default. Most operations are non-blocking. Only high-risk actions require sync verification. Governance overhead matches governance need.

### Issue: Agents Claim Ignorance

"I didn't know that was against policy." "I wasn't told about that API."

**CRA Solution:** The agent builds its own wrapper. Its agreement is recorded. The context it received is hashed. No plausible deniability.

### Issue: One-Size-Fits-All Doesn't Work

Different agents, different environments, different needs. Generic solutions don't fit.

**CRA Solution:** Agent builds a tailored wrapper. Stewards configure atlases for their domain. Custodians add local restrictions. Everything is customized to context.

---

## Where It Fits in the Landscape

### The Current Stack

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   MODELS          FRAMEWORKS        TOOLS           EXECUTION        │
│   ──────          ──────────        ─────           ─────────        │
│   GPT-4           LangChain         MCP             APIs             │
│   Claude          CrewAI            Function Call   Databases        │
│   Llama           AutoGen           Tool Use        File Systems     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### What's Missing

- **Context layer**: How do agents know domain-specific things?
- **Governance layer**: How do we enforce policies without blocking?
- **Audit layer**: How do we know what happened?

### Where CRA Fits

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   MODELS          FRAMEWORKS        TOOLS           EXECUTION        │
│   ──────          ──────────        ─────           ─────────        │
│   GPT-4           LangChain         MCP             APIs             │
│   Claude          CrewAI            Function Call   Databases        │
│   Llama           AutoGen           Tool Use        File Systems     │
│                                                                      │
│          └──────────────┬───────────────┘                           │
│                         │                                            │
│                         ▼                                            │
│          ┌─────────────────────────────┐                            │
│          │            CRA              │                            │
│          │                             │                            │
│          │  Context    Governance      │                            │
│          │  (CARP)     (Policies)      │                            │
│          │                             │                            │
│          │  Audit      Identity        │                            │
│          │  (TRACE)    (Wrapper)       │                            │
│          │                             │                            │
│          └─────────────────────────────┘                            │
│                         │                                            │
│                         ▼                                            │
│                    EXECUTION                                         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

CRA is the layer between agent decisions and execution. It's not replacing frameworks or tools—it's adding what they don't have.

---

## How It Works (Simple Explanation)

### Step 1: Agent Connects

An agent needs to work with your system. It connects to CRA.

### Step 2: CRA Detects

CRA figures out what kind of agent this is—model, capabilities, environment. No interrogation, just detection.

### Step 3: Agent Builds Wrapper

CRA guides the agent to build its own integration. The agent creates hooks that:
- Let CRA see what it's doing
- Let CRA inject helpful context
- Send activity logs to the audit trail

By building this, the agent learns how the system works.

### Step 4: Agent Works

The agent does its job. Mostly, it doesn't notice CRA:
- When it hits a checkpoint (keyword match, risky action), context appears
- Its actions get logged in the background
- Policies are checked only when they need to be

### Step 5: Audit Trail Exists

Everything the agent did is recorded. What context it got. What it tried. What succeeded. Hash-chained and provable.

```
That's it:
Connect → Build Wrapper → Work → Audit Trail Exists
```

---

## Novel / Unique Capabilities

### 1. Agent-Built Integration

**What's unique:** The agent constructs its own wrapper.

**Why it matters:**
- Agent understands the integration (built it)
- Agent consented (active construction, not passive agreement)
- Agent's identity is cryptographically tied to its wrapper
- No generic integration—tailored to each agent

**Nothing else does this.** Other systems inject context or enforce rules on agents. CRA has the agent build its own governance.

### 2. Async-First Audit

**What's unique:** TRACE is non-blocking by default.

**Why it matters:**
- Agents don't wait for logging
- Audit happens in background
- But still tamper-proof (hash chain)
- Scales from lightweight logging to full governance

**Most audit systems block.** CRA logs everything without slowing anything.

### 3. Checkpoint-Based Context

**What's unique:** Context injection happens at defined moments, not constantly.

**Why it matters:**
- Not overwhelming the agent with info
- Not adding latency to every operation
- Context appears when relevant
- Configurable per atlas

**Other context systems are all-or-nothing.** CRA injects surgically.

### 4. Steward-Owned Atlases

**What's unique:** Domain experts create and control context packages.

**Why it matters:**
- The VIB3 team writes the VIB3 atlas
- The security team writes the security atlas
- Each steward controls access, delivery, integrations
- Potential for marketplace of expert knowledge

**Context doesn't have to be generic.** Specialists create packages for their domain.

### 5. Proof of Knowledge

**What's unique:** Cryptographic proof of what context an agent received.

**Why it matters:**
- Agent can't claim it didn't know
- Auditors can verify what information was available
- Liability is clear
- Training data for improvement

**No other system proves what agents knew.** CRA records it in the hash chain.

### 6. Governance That Scales

**What's unique:** From zero governance to full control, configured per atlas.

**Why it matters:**
- Internal tool atlas: just logging
- Production system atlas: full verification
- Same system handles both
- Add governance without changing architecture

**Most governance is all-or-nothing.** CRA lets you dial it to what you need.

---

## The Utility

**For Developers:** Your agents know how to use your internal systems. They don't hallucinate your API. You have a log of what they did.

**For Enterprises:** Governance without grinding to a halt. Audit trails that build themselves. Proof of compliance.

**For Domain Experts:** Package your knowledge into an atlas. Agents that use it work correctly. You control access and get usage data.

**For Agent Builders:** Drop-in governance layer. Works with your framework. Agents build their own integration. Minimal overhead.

---

## Summary

| Aspect | CRA's Approach |
|--------|----------------|
| **Context** | Injected at checkpoints, not constantly |
| **Audit** | Async, non-blocking, but tamper-proof |
| **Governance** | Scales with need, not one-size-fits-all |
| **Identity** | Proven by wrapper construction |
| **Integration** | Agent builds it, tailored to each |
| **Ownership** | Stewards control their atlases |

**CRA is the missing layer between agent intelligence and execution—context, governance, and audit that actually works at scale.**
