# CRA in the AI Agent Landscape

## Context Registry for Agents: Where It Fits

**Version:** 1.0
**Date:** December 2025
**Author:** CRA Development Team

---

## Executive Summary

CRA (Context Registry for Agents) occupies a unique position in the AI agent ecosystem. While other systems focus on **teaching agents** (Claude Skills) or **giving agents tools** (OpenAI Agents SDK, MCP), CRA focuses on **governance and proof**: ensuring agents operate within policy boundaries and providing cryptographic evidence of what actually happened.

| System | Primary Purpose |
|--------|-----------------|
| **Claude Skills** | Teach agents HOW to work (context injection) |
| **OpenAI Agents SDK** | Give agents TOOLS to work (orchestration) |
| **MCP** | Connect agents to SYSTEMS (protocol) |
| **CRA** | **PROVE what agents did + GOVERN what they can do** |

CRA complements all of these - it's the governance layer that wraps around skills, tools, and MCP connections.

---

## Table of Contents

1. [The AI Agent Ecosystem (2025)](#the-ai-agent-ecosystem-2025)
2. [Claude Skills: Deep Dive](#claude-skills-deep-dive)
3. [OpenAI Agents SDK: Deep Dive](#openai-agents-sdk-deep-dive)
4. [CRA Architecture](#cra-architecture)
5. [Comparative Analysis](#comparative-analysis)
6. [Data Format Analysis](#data-format-analysis)
7. [Integration Patterns](#integration-patterns)
8. [Recommendations](#recommendations)

---

## The AI Agent Ecosystem (2025)

### The Three Layers of Agent Infrastructure

```
┌─────────────────────────────────────────────────────────────────┐
│                    APPLICATION LAYER                             │
│  LangChain │ CrewAI │ AutoGen │ LangGraph │ Custom Agents       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CAPABILITY LAYER                              │
│  Claude Skills │ OpenAI Function Calling │ Agent Skills Standard │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    INFRASTRUCTURE LAYER                          │
│  MCP (Model Context Protocol) │ Tool APIs │ Database Connectors │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GOVERNANCE LAYER  ← CRA                       │
│  CARP (Policy) │ TRACE (Audit) │ Atlas (Packages)               │
└─────────────────────────────────────────────────────────────────┘
```

### Key Industry Developments (2024-2025)

| Date | Event | Significance |
|------|-------|--------------|
| Nov 2024 | MCP Released | Anthropic releases Model Context Protocol |
| Dec 2024 | Agent Skills Standard | Cross-platform skill format launched |
| Mar 2025 | OpenAI Agents SDK | Production-ready multi-agent framework |
| Nov 2025 | MCP donated to Linux Foundation | Industry-wide adoption signal |
| Dec 2025 | Skills in Codex | OpenAI adopts Agent Skills standard |

### The Governance Gap

Most agent systems focus on capabilities but lack:
- **Cryptographic proof** of what agents actually did
- **Formal policy evaluation** beyond simple allow/deny lists
- **Audit trails** that can't be tampered with
- **Context governance** - what information agents receive

**This is CRA's territory.**

---

## Claude Skills: Deep Dive

### What Are Skills?

Claude Skills are modular, reusable capabilities that extend Claude through **structured context injection**. A skill is fundamentally a markdown file that teaches Claude how to do something specific.

### Architecture: Meta-Tool Pattern

```
┌─────────────────────────────────────────────────────────────────┐
│                     SKILL INVOCATION                             │
├─────────────────────────────────────────────────────────────────┤
│  1. Agent reasoning determines skill is relevant                │
│  2. Skill tool loads SKILL.md content                           │
│  3. Instructions injected as new user message                   │
│  4. Execution context modified (allowed tools, model)           │
│  5. Conversation continues with enriched environment            │
└─────────────────────────────────────────────────────────────────┘
```

**Key insight**: There is NO algorithmic routing. Claude's language model makes the selection decision during its forward pass. No regex, no keyword matching, no ML-based intent detection.

### Skill File Structure

```
my-skill/
├── SKILL.md          # Required: instructions + YAML frontmatter
├── REFERENCE.md      # Optional: loaded on demand
├── examples/         # Optional: loaded on demand
└── scripts/          # Optional: executed, not loaded into context
```

### SKILL.md Format

```yaml
---
name: ticket-support
description: Handle customer support ticket operations
allowed-tools: Read, Write, Bash
version: 1.0.0
---

# Ticket Support

## Overview
Instructions for working with tickets...

## Examples
[examples here]
```

### Three-Tier Loading (Progressive Disclosure)

| Tier | What's Loaded | When | Token Cost |
|------|---------------|------|------------|
| **1** | Name + Description only | Always (startup) | ~few dozen tokens |
| **2** | Full SKILL.md body | When skill activated | Full content |
| **3** | Reference files | When explicitly accessed | Only what's read |

This is a critical design pattern that CRA should adopt.

### Governance in Skills

| Mechanism | Description |
|-----------|-------------|
| `allowed-tools` | Restricts which tools Claude can use |
| Permission prompts | State-changing operations require approval |
| Enterprise controls | Centralized skill assignment, auditing |

**Limitation**: No formal policy language, no cryptographic audit trail.

---

## OpenAI Agents SDK: Deep Dive

### Core Primitives

The SDK is built around **four core primitives**:

1. **Agents**: LLMs equipped with instructions and tools
2. **Handoffs**: Enable agents to delegate to other agents
3. **Guardrails**: Validate agent inputs and outputs
4. **Sessions**: Maintain conversation history

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     OPENAI AGENTS SDK                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────┐    handoff()    ┌─────────┐                       │
│  │ Agent A │ ───────────────▶ │ Agent B │                       │
│  └────┬────┘                  └────┬────┘                       │
│       │                            │                            │
│       ▼                            ▼                            │
│  ┌─────────┐                  ┌─────────┐                       │
│  │ Tools   │                  │ Tools   │                       │
│  └─────────┘                  └─────────┘                       │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                      SESSION                              │  │
│  │  Maintains conversation history across agents             │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                     GUARDRAILS                            │  │
│  │  Input validation │ Output validation │ PII masking       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Responses API vs Chat Completions

| Feature | Responses API (New) | Chat Completions (Legacy) |
|---------|---------------------|---------------------------|
| State management | Server-side | Manual |
| Tool calling | Built-in loop | Single request |
| Reasoning preservation | Yes | No |
| Performance | 3% better (SWE-bench) | Baseline |
| Cost | 40-80% better cache | Baseline |

### Tracing (Observability)

OpenAI Agents SDK includes **built-in tracing**:
- Automatic: LLM generations, tool calls, handoffs, guardrails
- Custom: User-defined spans and events
- Integrations: Datadog, Langfuse, LangSmith, etc.

**Limitation**: Tracing is for observability/debugging, not cryptographic proof.

### Guardrails

```python
@input_guardrail
async def validate_input(ctx, agent, input):
    # Check for PII, jailbreaks, etc.
    return GuardrailResult(...)

@output_guardrail
async def validate_output(ctx, agent, output):
    # Validate response format, content
    return GuardrailResult(...)
```

**Limitation**: Validates I/O but no formal policy language.

---

## CRA Architecture

### The Three Pillars

```
┌─────────────────────────────────────────────────────────────────┐
│                           CRA                                    │
├─────────────────┬─────────────────┬─────────────────────────────┤
│      CARP       │      TRACE      │         ATLAS               │
│  Context &      │  Telemetry &    │   Versioned packages        │
│  Action         │  Replay Audit   │   defining capabilities     │
│  Resolution     │  Contract       │   and policies              │
│  Protocol       │                 │                             │
├─────────────────┼─────────────────┼─────────────────────────────┤
│ • Policy eval   │ • Hash chain    │ • Context blocks            │
│ • Action allow/ │ • Cryptographic │ • Actions + schemas         │
│   deny          │   proof         │ • Policy definitions        │
│ • Context       │ • Immutable     │ • Capability groups         │
│   injection     │   audit log     │ • Dependencies              │
└─────────────────┴─────────────────┴─────────────────────────────┘
```

### Core Principle

> **"If it wasn't emitted by the runtime, it didn't happen."**

TRACE events form an immutable audit log with cryptographic integrity. Each event's hash depends on all previous events - tampering with any event breaks the chain.

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      ATLAS PACKAGE                               │
│  manifest.json                                                  │
│  ├─ context_blocks (inline)                                     │
│  ├─ context_packs (file references)                             │
│  ├─ actions (with schemas)                                      │
│  ├─ policies (allow/deny/rate_limit/requires_approval)          │
│  └─ capabilities (action groupings)                             │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                    load_atlas()
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CARP RESOLVER                                 │
│                                                                 │
│  1. Register context in ContextRegistry                         │
│  2. Index by keywords for matching                              │
│  3. Store policies for evaluation                               │
│                                                                 │
│  On resolve(request):                                           │
│  ├─ Evaluate policies → allowed/denied actions                  │
│  ├─ Query matching contexts by goal keywords                    │
│  ├─ Emit TRACE events for each step                             │
│  └─ Return CARPResolution                                       │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    TRACE CHAIN                                   │
│                                                                 │
│  Event 0: session.started                                       │
│      hash = f(GENESIS, payload, timestamp, ...)                 │
│                                                                 │
│  Event 1: carp.request.received                                 │
│      hash = f(event_0.hash, payload, timestamp, ...)            │
│                                                                 │
│  Event 2: policy.evaluated                                      │
│      hash = f(event_1.hash, payload, timestamp, ...)            │
│                                                                 │
│  Event 3: context.injected                                      │
│      hash = f(event_2.hash, payload, timestamp, ...)            │
│                                                                 │
│  ...chain continues, each event linked to previous...           │
│                                                                 │
│  Verification: ChainVerifier.verify() proves integrity          │
└─────────────────────────────────────────────────────────────────┘
```

### Policy Types

| Type | Description | Example |
|------|-------------|---------|
| `allow` | Explicitly permit actions | `allow: ["ticket.get", "ticket.list"]` |
| `deny` | Block actions with reason | `deny: ["ticket.delete"], reason: "Audit required"` |
| `rate_limit` | Throttle action frequency | `max_calls: 100, window_seconds: 60` |
| `requires_approval` | Human-in-the-loop | `actions: ["payment.*"]` |
| `budget` | Cost/resource limits | `max_cost: 1000, currency: "USD"` |

### Context Injection

Current matching logic:
1. Keywords in goal text trigger inclusion
2. `context_hints` from request boost matching
3. `risk_tiers` filter by request risk level
4. `inject_when` matches action patterns
5. Priority determines injection order

**Gap identified**: No "always inject" mode for essential context.

---

## Comparative Analysis

### Feature Matrix

| Feature | Claude Skills | OpenAI Agents | CRA |
|---------|--------------|---------------|-----|
| **Context Injection** | ✅ Progressive | ✅ Instructions | ✅ Keyword-matched |
| **Tool Restrictions** | ✅ allowed-tools | ✅ Guardrails | ✅ CARP policies |
| **Audit Trail** | ❌ None | ⚠️ Observability | ✅ Cryptographic |
| **Policy Language** | ❌ Simple list | ❌ Code-based | ✅ Formal types |
| **Multi-Agent** | ⚠️ Subagents | ✅ Handoffs | ⚠️ Session-based |
| **Versioning** | ✅ version field | ❌ None | ✅ Semver |
| **Dependencies** | ❌ None | ❌ None | ✅ Atlas deps |
| **Progressive Loading** | ✅ 3-tier | ❌ Upfront | ❌ Upfront |

### What CRA Does Better

#### 1. Cryptographic Proof (TRACE)

```
Claude Skills:  No audit trail
OpenAI Agents:  Tracing for debugging
CRA:            Hash-chained events - tamper-evident proof
```

Enterprise needs **proof**, not just logs. TRACE provides:
- Immutable event chain
- Cryptographic verification
- Replay capability
- Legal/compliance evidence

#### 2. Formal Policy Evaluation (CARP)

```
Claude Skills:  allowed-tools: ["Read", "Write"]
OpenAI Agents:  @guardrail decorators
CRA:            {
                  "policy_type": "rate_limit",
                  "actions": ["api.*"],
                  "max_calls": 100,
                  "window_seconds": 60
                }
```

CRA policies are:
- Declarative (not code)
- Composable
- Auditable
- Version-controlled

#### 3. Context + Governance Together

```
Claude Skills:  Skills are separate from governance
OpenAI Agents:  Tools + guardrails are separate
CRA:            Atlas = context + actions + policies in ONE package
```

An Atlas defines:
- What context to inject
- What actions are available
- What policies govern those actions
- All versioned together

### What CRA Should Adopt

#### 1. Progressive Disclosure (From Skills)

```rust
pub enum InjectMode {
    Always,      // Essential facts - always inject
    OnMatch,     // Current behavior - keyword match
    OnDemand,    // Only if explicitly requested
}
```

#### 2. Markdown + YAML Frontmatter (From Skills)

```markdown
---
context_id: vib3-essential-facts
priority: 350
inject_mode: always
---

# VIB3+ Essential Facts

## Working Systems
- Faceted (WORKING)
- Quantum (WORKING)
- Holographic (WORKING)

## NOT Working
- Polychora - PLACEHOLDER, do not use
```

#### 3. Sources Field (Missing)

```json
{
  "sources": {
    "repositories": ["https://github.com/org/repo"],
    "documentation": "https://docs.example.com",
    "demo": "https://demo.example.com"
  }
}
```

---

## Data Format Analysis

### Format Comparison

| Format | Token Efficiency | LLM Accuracy | Human Readable | Rust Support |
|--------|-----------------|--------------|----------------|--------------|
| **JSON** | Baseline | 69.7% | Medium | ✅ Excellent |
| **YAML** | +20-40% savings | 69.0% | ✅ Best | ✅ Good |
| **TOON** | +30-60% savings | 73.9% | Medium | ✅ Good |
| **JSON5** | +5-10% savings | Mixed | Good | ✅ Good |
| **TOML** | Similar to YAML | N/A | Good | ✅ Excellent |
| **CSON** | Unknown | Poor | Good | ❌ None |
| **I-JSON** | Same as JSON | Same | Same | ⚠️ Partial |

### Token Efficiency Benchmark

```
Format          Tokens    Accuracy   Acc%/1K Tokens
─────────────────────────────────────────────────────
TOON            2,744     73.9%      26.9  ⭐ Best for LLM
JSON Compact    3,081     70.7%      22.9
YAML            3,719     69.0%      18.6
JSON            4,545     69.7%      15.3
XML             5,167     67.1%      13.0
```

### Format Recommendations by Use Case

| Use Case | Recommended | Rationale |
|----------|-------------|-----------|
| **Atlas Manifests** | JSON or YAML | Tooling, validation, human editing |
| **Context Content** | Markdown | Human-readable, LLM-native |
| **LLM Prompt Data (tabular)** | TOON | 30-60% token savings |
| **LLM Prompt Data (nested)** | JSON | Better structure preservation |
| **TRACE Events** | JSON | Canonical serialization for hashing |
| **API Communication** | JSON | Universal standard |

### Why JSON Remains Primary for CRA

1. **Canonical serialization**: Required for hash computation
2. **Schema validation**: JSON Schema is mature
3. **Universal tooling**: Every language supports JSON
4. **LLM compatibility**: Models reliably generate JSON with schemas
5. **Serde integration**: Excellent Rust support

### Recommended Hybrid Approach

```
Atlas Package Structure:
├── manifest.json          # JSON: tooling, validation
├── context/
│   ├── ESSENTIAL.md       # Markdown: always-inject content
│   ├── workflows/
│   │   ├── embed.md       # Markdown: workflow guides
│   │   └── customize.md
│   └── reference/
│       └── params.yaml    # YAML: structured reference data
└── examples/
    └── data.toon          # TOON: large tabular examples
```

---

## Integration Patterns

### CRA + Claude Skills

```
┌─────────────────────────────────────────────────────────────────┐
│                     CLAUDE SKILL                                 │
│                                                                 │
│  SKILL.md defines HOW to work                                   │
│  ├─ Instructions                                                │
│  ├─ Examples                                                    │
│  └─ Procedures                                                  │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           │ Skill invokes actions
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CRA GOVERNANCE                               │
│                                                                 │
│  Atlas defines WHAT is allowed                                  │
│  ├─ CARP: Evaluate policy before action                         │
│  ├─ TRACE: Record action execution                              │
│  └─ Verify: Prove what happened                                 │
└─────────────────────────────────────────────────────────────────┘
```

### CRA + OpenAI Agents

```
┌─────────────────────────────────────────────────────────────────┐
│                    OPENAI AGENT                                  │
│                                                                 │
│  Agent orchestrates tools and handoffs                          │
│  ├─ Function calling                                            │
│  ├─ Handoffs between agents                                     │
│  └─ Session management                                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           │ Before each tool call
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CRA GOVERNANCE                               │
│                                                                 │
│  CARP.resolve(action, context)                                  │
│  ├─ Check policy: allowed/denied/rate_limited                   │
│  ├─ Inject relevant context                                     │
│  └─ Emit TRACE event                                            │
└─────────────────────────────────────────────────────────────────┘
```

### CRA + MCP

```
┌─────────────────────────────────────────────────────────────────┐
│                    MCP SERVER                                    │
│                                                                 │
│  Provides tools and resources                                   │
│  ├─ Tool definitions                                            │
│  ├─ Resource access                                             │
│  └─ Prompts                                                     │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           │ Tool invocation
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CRA WRAPPER                                  │
│                                                                 │
│  Governance layer around MCP                                    │
│  ├─ Policy check before tool execution                          │
│  ├─ Context injection for tool use                              │
│  ├─ TRACE event for each MCP call                               │
│  └─ Chain verification for audit                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Recommendations

### For CRA v1.x (Near-term)

1. **Add `inject_mode` to context blocks**
   ```rust
   pub enum InjectMode {
       Always,    // Essential context
       OnMatch,   // Keyword matching (current)
       OnDemand,  // Explicit request only
   }
   ```

2. **Add `sources` to manifest**
   ```rust
   pub struct AtlasSources {
       pub repositories: Vec<String>,
       pub documentation: Option<String>,
       pub demo: Option<String>,
   }
   ```

3. **Support Markdown context files**
   - Parse YAML frontmatter for metadata
   - Body becomes context content
   - More readable than JSON for documentation

4. **Keep JSON as primary manifest format**
   - Required for canonical serialization (TRACE hashing)
   - Best tooling and validation support
   - Consider YAML as optional input format

### For CRA v2.0 (Future)

1. **Agent Skills compatibility**
   - Support `.cra/skills/` directory
   - Parse SKILL.md format
   - Map to Atlas structure

2. **TOON for large context data**
   - Use for tabular policy tables
   - Use for example datasets
   - 30-60% token savings

3. **Progressive loading**
   - Tier 1: Manifest metadata only
   - Tier 2: Context on session start
   - Tier 3: Reference files on demand

4. **MCP integration**
   - CRA as MCP server wrapper
   - Governance for any MCP tool
   - TRACE events for MCP calls

### Summary: CRA's Unique Value

| What Others Do | What CRA Adds |
|----------------|---------------|
| Skills teach agents | **Prove** agents followed the teaching |
| Tools give capabilities | **Govern** which capabilities are allowed |
| MCP connects to systems | **Audit** every system interaction |
| Guardrails validate I/O | **Policy language** for complex rules |
| Tracing for debugging | **Cryptographic proof** for compliance |

**CRA is the governance and audit layer the AI agent ecosystem is missing.**

---

## Appendix: Research Sources

### Claude Skills
- [Agent Skills - Claude Code Docs](https://code.claude.com/docs/en/skills)
- [Introducing Agent Skills | Claude](https://claude.com/blog/skills)
- [Equipping agents for the real world](https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills)
- [Claude Skills: Technical Deep-Dive](https://medium.com/data-science-collective/claude-skills-a-technical-deep-dive-into-context-injection-architecture)

### OpenAI Agents SDK
- [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/)
- [New tools for building agents | OpenAI](https://openai.com/index/new-tools-for-building-agents/)
- [Function calling | OpenAI API](https://platform.openai.com/docs/guides/function-calling)
- [Responses API](https://openai.com/index/new-tools-and-features-in-the-responses-api/)

### Data Formats
- [TOON Specification](https://github.com/toon-format/toon)
- [JSON5 Specification](https://spec.json5.org/)
- [RFC 7493 - I-JSON](https://datatracker.ietf.org/doc/html/rfc7493)
- [LLM Output Formats Comparison](https://medium.com/@michael.hannecke/beyond-json-picking-the-right-format-for-llm-pipelines)

### Industry Context
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Agent Skills Standard](https://agentskills.io/)
- [AI Agent Governance (2025)](https://dextralabs.com/blog/agentic-ai-safety-playbook-guardrails-permissions-auditability/)

---

*This document is part of the CRA-Core repository. For implementation details, see the source code in `cra-core/src/`.*
