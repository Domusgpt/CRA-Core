# CRA: Context Registry Agents — Complete System Overview & Rust Refactor Guide

## Table of Contents

1. [The Problem](#1-the-problem)
2. [The Solution: CRA](#2-the-solution-cra)
3. [Core Protocols](#3-core-protocols)
4. [The Atlas System](#4-the-atlas-system)
5. [Platform Vision](#5-platform-vision)
6. [Current Implementation Status](#6-current-implementation-status)
7. [Future Roadmap](#7-future-roadmap)
8. [Why Rust Core](#8-why-rust-core)
9. [Rust Refactor Implementation Plan](#9-rust-refactor-implementation-plan)
10. [Reference Materials](#10-reference-materials)

---

## 1. The Problem

### AI Agents Are Ungoverned

LLMs and AI agents routinely:

- **Invent APIs and tools** — Hallucinate endpoints, parameters, and workflows that don't exist
- **Misuse proprietary systems** — Call internal tools incorrectly, violate business rules
- **Provide no proof of execution** — No audit trail, no verification, no accountability
- **Exceed authorized scope** — Access data and perform actions beyond their intended purpose
- **Bypass security controls** — No consistent authz/authn layer across agent frameworks

### This Breaks Everything

| Stakeholder | Problem |
|-------------|---------|
| **Security** | No audit trail, no access control, agents as attack vectors |
| **Compliance** | Can't prove what happened, no SOC2/HIPAA/GDPR evidence |
| **Operations** | Agents break production systems, no rate limiting |
| **Developers** | Every framework has different tool formats, no portability |
| **Business** | Can't trust agents with real systems, limits adoption |

### The Root Cause

**LLM output is treated as authoritative.** When an agent says "I called the API and got X", there's no verification. The agent's word is taken as truth.

---

## 2. The Solution: CRA

### Core Principle

> **If it wasn't emitted by the runtime, it didn't happen.**

CRA (Context Registry Agents) inverts the authority model:

```
Traditional:                          CRA:
┌─────────┐                          ┌─────────┐
│   LLM   │ ◀── "Trust me, I        │   LLM   │ ◀── Advisory only
│         │      did the thing"      │         │
└────┬────┘                          └────┬────┘
     │                                    │
     ▼                                    ▼
┌─────────┐                          ┌─────────┐
│  Tools  │ ◀── Direct access        │   CRA   │ ◀── Runtime authority
│         │     (unmonitored)        │ Runtime │     (all calls logged)
└─────────┘                          └────┬────┘
                                          │
                                          ▼
                                     ┌─────────┐
                                     │  Tools  │ ◀── Governed access
                                     │         │     (policy-checked)
                                     └─────────┘
```

### What CRA Does

1. **Curates Context** — Provides only the information the agent needs for its task
2. **Enforces Policies** — Validates every action against governance rules
3. **Proves Execution** — Emits cryptographically-linked audit events
4. **Enables Replay** — Any session can be deterministically replayed
5. **Unifies Platforms** — Single source of truth, adapters for every LLM vendor

### The Runtime Is Authoritative

CRA is not a library the agent uses — it's the layer the agent operates within. The agent proposes actions; CRA decides what's allowed and records what happens.

---

## 3. Core Protocols

CRA defines two complementary protocols:

### CARP/1.0 — Context & Action Resolution Protocol

**Purpose:** Determine what context and actions are available for a given goal.

```
Agent                                CRA Runtime
  │                                      │
  │  "I need to help user with X"        │
  │  ────────── CARPRequest ──────────►  │
  │                                      │
  │                              ┌───────┴───────┐
  │                              │ Load Atlases  │
  │                              │ Eval Policies │
  │                              │ Build Context │
  │                              └───────┬───────┘
  │                                      │
  │  ◀─────── CARPResolution ──────────  │
  │  • Context blocks to inject          │
  │  • Allowed actions (tools)           │
  │  • Denied actions (with reasons)     │
  │  • Active constraints                │
  │                                      │
```

**Key Concepts:**

| Concept | Description |
|---------|-------------|
| **Request** | Agent's goal, identity, and context hints |
| **Resolution** | What the agent may know and do |
| **Decision** | allow / deny / partial / requires_approval |
| **TTL** | Resolution expires, must re-resolve for fresh context |
| **Trace ID** | Links resolution to audit events |

**Resolution Flow:**

1. Agent submits goal to CARP
2. Runtime loads relevant Atlas(es)
3. Runtime evaluates policies (deny → approval → rate limit → allow)
4. Runtime assembles context blocks with priority ordering
5. Runtime returns resolution with allowed actions
6. Resolution has TTL — agent must re-resolve when expired

### TRACE/1.0 — Telemetry & Replay Audit Contract for Execution

**Purpose:** Prove what actually happened with cryptographic integrity.

```
┌─────────────────────────────────────────────────────────────────┐
│                         TRACE Stream                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Event 0          Event 1          Event 2          Event 3     │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌────────┐  │
│  │ session  │────▶│ carp.req │────▶│ action   │────▶│ action │  │
│  │ started  │     │ received │     │ executed │     │ failed │  │
│  │          │     │          │     │          │     │        │  │
│  │ hash: A  │     │ hash: B  │     │ hash: C  │     │ hash: D│  │
│  │ prev: 0  │     │ prev: A  │     │ prev: B  │     │ prev: C│  │
│  └──────────┘     └──────────┘     └──────────┘     └────────┘  │
│                                                                  │
│  Chain: 0 ──▶ A ──▶ B ──▶ C ──▶ D                               │
│  Tamper-evident: changing any event breaks the chain            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Key Properties:**

| Property | Description |
|----------|-------------|
| **Append-Only** | Events can only be added, never modified |
| **Hash Chain** | Each event includes hash of previous event |
| **Tamper-Evident** | Any modification breaks chain verification |
| **Replayable** | Given TRACE + Atlas, can reproduce exact behavior |
| **Diffable** | Compare traces to detect behavioral changes |

**Event Types:**

```
session.started       — Session created
session.ended         — Session completed
carp.request.received — CARP request received
carp.resolution.completed — Resolution computed
action.requested      — Agent requested action
action.approved       — Action passed policy check
action.denied         — Action blocked by policy
action.executed       — Action executed successfully
action.failed         — Action execution failed
policy.evaluated      — Policy rule checked
policy.violated       — Policy violation detected
context.injected      — Context block added to agent
```

### Protocol Relationship

```
Atlas/1.0 ────────────▶ CARP/1.0 ────────────▶ TRACE/1.0
(defines what's        (resolves what's       (records what
 available)             allowed)                happened)
```

---

## 4. The Atlas System

### What Is an Atlas?

An **Atlas** is a versioned package containing everything needed to govern agent behavior in a domain:

```
customer-support-atlas/
├── atlas.json              # Manifest (identity, version, metadata)
├── context/                # Knowledge documents
│   ├── policies.md         # Company policies
│   ├── procedures.md       # Support procedures
│   └── faq.md              # Common questions
├── policies/               # Governance rules
│   ├── deny-refunds.json   # Block refund actions for certain tiers
│   └── rate-limits.json    # API rate limiting
├── actions/                # Available tools
│   ├── ticket.json         # Ticket operations
│   └── customer.json       # Customer lookup
└── adapters/               # Platform-specific formats
    ├── openai.json         # OpenAI function calling
    ├── anthropic.json      # Claude tool format
    └── mcp.json            # MCP server config
```

### Atlas Manifest

```json
{
  "atlas_version": "1.0",
  "atlas_id": "com.acme.customer-support",
  "version": "2.1.0",
  "name": "Customer Support Atlas",
  "description": "Context and tools for customer support agents",
  "domains": ["support", "crm", "ticketing"],

  "capabilities": [
    {
      "capability_id": "ticket.read",
      "name": "Read Tickets",
      "actions": ["ticket.get", "ticket.list", "ticket.search"]
    },
    {
      "capability_id": "ticket.write",
      "name": "Modify Tickets",
      "actions": ["ticket.create", "ticket.update", "ticket.close"]
    }
  ],

  "policies": [
    {
      "policy_id": "deny-delete",
      "type": "deny",
      "actions": ["*.delete"],
      "reason": "Deletion requires manual approval"
    },
    {
      "policy_id": "rate-limit-api",
      "type": "rate_limit",
      "actions": ["ticket.*"],
      "parameters": {
        "max_calls": 100,
        "window_seconds": 60
      }
    }
  ],

  "actions": [
    {
      "action_id": "ticket.get",
      "name": "Get Ticket",
      "description": "Retrieve a support ticket by ID",
      "parameters_schema": {
        "type": "object",
        "required": ["ticket_id"],
        "properties": {
          "ticket_id": {"type": "string"}
        }
      },
      "risk_tier": "low"
    }
  ]
}
```

### Atlas Marketplace Vision

Atlases can be:
- **Private** — Internal to an organization
- **Public** — Open source, community-contributed
- **Licensed** — Commercial, paid access
- **Certified** — Verified CARP/TRACE compliance

```
┌─────────────────────────────────────────────────────────────────┐
│                      Atlas Marketplace                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  🏢 Enterprise Atlases          🌐 Community Atlases            │
│  ├── com.salesforce.crm        ├── org.github.issues            │
│  ├── com.stripe.payments       ├── org.kubernetes.ops           │
│  ├── com.snowflake.warehouse   ├── org.terraform.iac            │
│  └── com.servicenow.itsm       └── org.openapi.generic          │
│                                                                  │
│  🔒 Certified: Passes CARP/TRACE conformance tests              │
│  📊 Audited: Security review completed                          │
│  ⭐ Rated: Community ratings and reviews                        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Platform Vision

### Where CRA Fits

CRA is designed to be **infrastructure** — not an application, but a foundational layer that AI systems build upon.

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Layer                          │
│  Customer Support Bot │ DevOps Copilot │ Data Analyst Agent     │
├─────────────────────────────────────────────────────────────────┤
│                      Framework Layer                            │
│      LangChain      │     CrewAI      │       AutoGen           │
├─────────────────────────────────────────────────────────────────┤
│                      CRA Layer (Governance)                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  CARP Engine │ TRACE Collector │ Atlas Registry │ Auth  │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                      LLM Layer                                  │
│       OpenAI        │    Anthropic    │       Google            │
├─────────────────────────────────────────────────────────────────┤
│                      Tool Layer                                 │
│    APIs │ Databases │ Cloud Services │ Internal Systems         │
└─────────────────────────────────────────────────────────────────┘
```

### Integration Points

| Platform | Integration |
|----------|-------------|
| **OpenAI** | Function calling adapter, GPT Actions generator |
| **Anthropic** | Claude tool format, MCP server |
| **Google** | ADK agent tool definitions |
| **LangChain** | Native middleware, tool wrapper |
| **CrewAI** | Crew tool integration |
| **AutoGen** | Agent tool registry |
| **VS Code** | Extension for Atlas authoring |
| **Claude Desktop** | MCP server for local governance |

### The Infrastructure Goal

CRA should be like SQLite — embedded everywhere, invisible, just how things work:

```
Future State:
┌─────────────────────────────────────────────────────────────────┐
│                     Every AI Agent                               │
│                                                                  │
│  "I want to perform action X"                                   │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────┐                                            │
│  │   CRA (embedded) │  ◀── Not a service call                   │
│  │                  │      Just a function call                 │
│  │   • Is it allowed?  ──▶ Check policy                         │
│  │   • Log the event   ──▶ Append to TRACE                      │
│  │   • Return result   ──▶ Continue execution                   │
│  └─────────────────┘                                            │
│           │                                                      │
│           ▼                                                      │
│       Execute action                                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Current Implementation Status

### Repository Branches

This repository contains multiple implementation attempts:

#### Branch: `claude/plan-cra-platform-WoXIo` (Python — Production Ready)

**111 files** — Complete Python implementation with:

```
cra/
├── core/               # CARP, TRACE, Atlas, Policy, Replay engines
├── runtime/            # FastAPI server, PostgreSQL storage
├── cli/                # Full CLI (init, resolve, execute, trace, atlas, replay)
├── adapters/           # OpenAI, Anthropic, Google ADK, MCP
├── templates/          # LangChain, CrewAI, OpenAI GPT generators
├── auth/               # JWT, API keys, RBAC (5 built-in roles)
├── middleware/         # LangChain, OpenAI integration layers
├── observability/      # OpenTelemetry, SIEM (CEF, LEEF, JSON)
└── config/             # Pydantic settings, environment config

specs/                  # PROTOCOL-FIRST FOUNDATION
├── PROTOCOL.md         # Master specification (CARP, TRACE, Atlas)
├── openapi.yaml        # HTTP API spec (OpenAPI 3.1)
├── schemas/            # JSON Schema for all data structures
│   ├── carp-request.schema.json
│   ├── carp-resolution.schema.json
│   ├── trace-event.schema.json
│   └── atlas-manifest.schema.json
└── conformance/        # Conformance test suite
    ├── CONFORMANCE.md
    └── golden/         # Reference test cases

docs/                   # Documentation
├── ARCHITECTURE.md     # System design
├── API.md              # REST API reference
├── CLI.md              # CLI command reference
├── ATLASES.md          # Atlas development guide
├── DEPLOYMENT.md       # Production deployment
├── INTEGRATION.md      # Framework integration
└── TYPESCRIPT_SDK_PLAN.md  # TypeScript complement plan

examples/atlases/       # Example Atlas packages
├── customer-support/
├── devops/
└── data-analytics/
```

#### Branch: `claude/design-cra-architecture-WdoAv` (TypeScript)

**~60 files** — Node.js monorepo with 76 passing tests:

```
packages/
├── protocol/   # CARP/TRACE TypeScript type definitions
├── trace/      # Trace collector with hash chain
├── atlas/      # Atlas loader and validator
├── runtime/    # Core runtime engine
├── adapters/   # OpenAI, Claude, MCP adapters
├── cli/        # CLI application
├── mcp/        # MCP server implementation (started)
└── otel/       # OpenTelemetry bridge (started)
```

#### Branches: `2025-12-27/22-*-codex` (Python MVP)

Early prototypes with excellent documentation:
- Protocol specifications (CARP_1_0.md, TRACE_1_0.md)
- Executive brief and vision documents
- Conformance test requirements

### What's Working

| Component | Python | TypeScript | Status |
|-----------|--------|------------|--------|
| CARP Engine | ✅ | ✅ | Production ready |
| TRACE Collector | ✅ | ✅ | Hash chain verified |
| Atlas Loader | ✅ | ✅ | Full validation |
| Policy Engine | ✅ | ⚠️ | Deny/allow/rate limit |
| HTTP Server | ✅ | ❌ | FastAPI complete |
| PostgreSQL Storage | ✅ | ❌ | Async with streaming |
| JWT/API Key Auth | ✅ | ❌ | Full RBAC |
| LangChain Middleware | ✅ | ❌ | Native integration |
| MCP Server | ⚠️ | ⚠️ | Adapter only |
| OpenTelemetry | ✅ | ⚠️ | Full export |
| Conformance Tests | ✅ | ❌ | Golden traces |

---

## 7. Future Roadmap

### Phase 5: Advanced Agent Capabilities

| Feature | Description |
|---------|-------------|
| Multi-Agent Orchestration | Shared context, handoffs, delegation |
| Agent Memory Systems | Long-term memory with vector stores |
| Hierarchical Agents | Supervisor/child with policy inheritance |
| Agent-to-Agent Communication | Secure message passing |

### Phase 6: Advanced Governance

| Feature | Description |
|---------|-------------|
| Dynamic Policy Engine | Runtime updates without restart |
| Compliance Templates | HIPAA, SOC2, GDPR pre-built policies |
| Approval Workflows | Human-in-the-loop for sensitive actions |
| Cost/Budget Controls | Token budgets, API cost limits |

### Phase 7: Extended Platforms

| Feature | Description |
|---------|-------------|
| AutoGen Integration | Microsoft AutoGen adapter |
| Semantic Kernel | Microsoft SK integration |
| DSPy | Stanford DSPy support |
| AWS Bedrock | Native Bedrock adapter |
| Local LLMs | Ollama, llama.cpp support |

### Phase 8: Enterprise Features

| Feature | Description |
|---------|-------------|
| Multi-Tenancy | Isolated tenants, quotas |
| SSO/SAML/OIDC | Enterprise identity |
| Audit Dashboard | Web UI for traces |
| Atlas Marketplace | Registry for sharing |

### Phase 9: Developer Experience

| Feature | Description |
|---------|-------------|
| VS Code Extension | Atlas authoring, validation |
| Atlas SDK | Programmatic Atlas creation |
| Testing Framework | Unit/integration test helpers |
| Simulation Mode | Dry-run execution |

### Phase 10: Infrastructure Scale

| Feature | Description |
|---------|-------------|
| Redis Backend | High-performance caching |
| Event Streaming | Kafka/NATS for traces |
| Distributed Tracing | Cross-service correlation |
| Geographic Distribution | Multi-region deployment |

---

## 8. Why Rust Core

### The Current Problem

All current implementations have a limitation: **CRA is a service, not infrastructure.**

```
Current (Python/TypeScript):
┌─────────────┐      HTTP         ┌─────────────┐
│   Agent     │ ────────────────▶ │  CRA Server │
│             │ ◀────────────────  │             │
└─────────────┘   ~5-50ms/call    └─────────────┘

Problems:
• Network latency on every resolution
• CRA calls look like "tool use" to LLMs
• Requires running a separate service
• Can't embed in browsers/edge/OS
```

### The Vision: Embedded Governance

```
Rust Core (Target):
┌─────────────────────────────────────────────┐
│               Agent Runtime                  │
│                                              │
│  ┌─────────────┐    ┌─────────────────────┐ │
│  │    Agent    │───▶│  CRA Core (Rust)    │ │
│  │    Logic    │◀───│  ~0.001ms/call      │ │
│  └─────────────┘    │  Embedded library   │ │
│                     └─────────────────────┘ │
│                                              │
└──────────────────────────────────────────────┘

Benefits:
• Zero network overhead
• Invisible to LLMs (not a "tool")
• Embedded in every runtime
• Works in browsers, edge, OS daemons
```

### What Rust Enables

| Capability | Description |
|------------|-------------|
| **Python Embedding** | PyO3 native extension, in-process calls |
| **Node.js Embedding** | napi-rs native addon, in-process calls |
| **WASM** | Runs in browsers, Cloudflare Workers, Deno |
| **OS Daemon** | System service for all local agents |
| **Memory Safety** | No runtime errors in governance layer |
| **Tiny Footprint** | ~50KB binary, ~1ms startup |

### Universal Deployment

```
Rust Core compiles to:
                          │
    ┌─────────────────────┼─────────────────────┐
    │                     │                     │
    ▼                     ▼                     ▼
┌─────────┐         ┌─────────┐          ┌─────────┐
│ Native  │         │  WASM   │          │   FFI   │
│ Binary  │         │ Module  │          │ Library │
│         │         │         │          │         │
│ Linux   │         │ Browser │          │ Python  │
│ macOS   │         │ Deno    │          │ Node.js │
│ Windows │         │ Edge    │          │ Ruby    │
│ FreeBSD │         │ Workers │          │ Go      │
└─────────┘         └─────────┘          └─────────┘
```

---

## 9. Rust Refactor Implementation Plan

### Target Architecture

```
cra-rust/
├── cra-core/           # Core Rust library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Public API
│       ├── carp/
│       │   ├── mod.rs
│       │   ├── request.rs      # CARPRequest
│       │   ├── resolution.rs   # CARPResolution
│       │   ├── resolver.rs     # resolve() engine
│       │   └── policy.rs       # Policy evaluation
│       ├── trace/
│       │   ├── mod.rs
│       │   ├── event.rs        # TRACEEvent
│       │   ├── collector.rs    # Event collection
│       │   ├── chain.rs        # SHA-256 hash chain
│       │   └── replay.rs       # Deterministic replay
│       ├── atlas/
│       │   ├── mod.rs
│       │   ├── manifest.rs     # AtlasManifest
│       │   ├── loader.rs       # Load from disk/memory
│       │   └── validator.rs    # JSON Schema validation
│       └── ffi/
│           └── c_api.rs        # C ABI for any language
│
├── cra-python/         # Python binding (PyO3)
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── python/cra/
│       ├── __init__.py
│       ├── resolver.py
│       └── middleware/
│           ├── langchain.py
│           └── crewai.py
│
├── cra-node/           # Node.js binding (napi-rs)
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── npm/
│       ├── package.json
│       └── index.d.ts
│
└── cra-wasm/           # WASM binding (wasm-bindgen)
    ├── Cargo.toml
    ├── src/lib.rs
    └── pkg/            # Generated npm package
```

### Phase 1: Rust Core

Implement core engine validated against `specs/`:

```rust
// Public API
pub struct Resolver {
    atlases: HashMap<String, Atlas>,
    sessions: HashMap<String, Session>,
}

impl Resolver {
    pub fn new() -> Self;

    pub fn load_atlas(&mut self, manifest: &str) -> Result<AtlasId, Error>;
    pub fn unload_atlas(&mut self, atlas_id: &str) -> Result<(), Error>;

    pub fn create_session(&mut self, agent_id: &str, goal: &str) -> Result<SessionId, Error>;
    pub fn end_session(&mut self, session_id: &str) -> Result<(), Error>;

    pub fn resolve(&self, request: &CARPRequest) -> Result<CARPResolution, Error>;
    pub fn execute(&mut self, session_id: &str, action_id: &str, params: Value) -> Result<Value, Error>;

    pub fn get_trace(&self, session_id: &str) -> Result<Vec<TRACEEvent>, Error>;
    pub fn verify_chain(&self, session_id: &str) -> Result<ChainVerification, Error>;
    pub fn replay(&self, trace: &[TRACEEvent], atlas: &Atlas) -> Result<ReplayResult, Error>;
}
```

### Phase 2: Python Binding

Drop-in replacement for current Python:

```python
from cra import Resolver

# Rust-powered, but same API
resolver = Resolver()
resolver.load_atlas("./atlas.json")

resolution = resolver.resolve(
    goal="Help with support ticket",
    agent_id="support-agent",
    session_id="session-123"
)

for action in resolution.allowed_actions:
    print(f"Available: {action.action_id}")

# LangChain middleware wraps Rust core
from cra.middleware.langchain import LangChainMiddleware

middleware = LangChainMiddleware(resolver)
tools = middleware.get_tools(goal="Customer support")
```

### Phase 3: Node.js Binding

Native addon for MCP and tooling:

```typescript
import { Resolver } from '@cra/core';

const resolver = new Resolver();
await resolver.loadAtlas('./atlas.json');

const resolution = resolver.resolve({
  goal: 'Execute MCP tool',
  agentId: 'mcp-client',
  sessionId: 'session-456',
});

// MCP server uses native binding
import { createMCPServer } from '@cra/mcp';
const server = createMCPServer(resolver);
```

### Phase 4: WASM Build

Browser and edge deployment:

```typescript
import init, { Resolver } from '@cra/wasm';

await init(); // Load WASM module

const resolver = new Resolver();
resolver.loadAtlasFromJson(atlasJson);

// Runs entirely client-side
const resolution = resolver.resolve({
  goal: 'Client-side validation',
  agentId: 'browser-agent',
});
```

### Conformance Requirements

The Rust implementation MUST pass all tests in `specs/conformance/`:

1. **Schema Validation** — All JSON validates against `specs/schemas/*.json`
2. **Hash Chain** — SHA-256 computation matches reference implementation
3. **Policy Evaluation** — Correct ordering: deny → approval → rate_limit → allow
4. **Golden Traces** — Output matches `specs/conformance/golden/*`
5. **Replay Determinism** — Same input always produces same output

---

## 10. Reference Materials

### Key Files in This Repository

| File | Purpose |
|------|---------|
| `specs/PROTOCOL.md` | Master protocol specification |
| `specs/schemas/*.json` | JSON Schema definitions |
| `specs/conformance/CONFORMANCE.md` | Test requirements |
| `specs/conformance/golden/` | Reference test cases |
| `specs/openapi.yaml` | HTTP API specification |
| `cra/core/carp.py` | Python CARP implementation (reference) |
| `cra/core/trace.py` | Python TRACE implementation (reference) |
| `cra/core/atlas.py` | Python Atlas implementation (reference) |
| `cra/core/policy.py` | Python policy engine (reference) |
| `docs/ARCHITECTURE.md` | System architecture |
| `docs/TYPESCRIPT_SDK_PLAN.md` | TypeScript complement plan |

### Success Criteria

| Metric | Target |
|--------|--------|
| Conformance Tests | 100% pass |
| Resolution Latency | <0.01ms |
| Binary Size | <1MB (release) |
| WASM Size | <500KB |
| Python Binding | Compatible with existing middleware |
| Node Binding | Works with MCP server |

### Getting Started

```bash
# Clone repository
git clone <repo-url>
cd CRA-Core

# Review protocol specs
cat specs/PROTOCOL.md

# Review current Python implementation
ls -la cra/core/

# Run conformance tests (when Rust is ready)
cargo test --features conformance
```

---

## Summary

**CRA (Context Registry Agents)** is a governance layer for AI agents that:

1. **Solves the trust problem** — Runtime authority, not LLM authority
2. **Uses two protocols** — CARP (permissions) + TRACE (audit)
3. **Packages context in Atlases** — Versioned, portable, governed
4. **Targets infrastructure status** — Embedded everywhere, invisible

**The Rust refactor** will make CRA truly universal by enabling:
- In-process embedding (no HTTP overhead)
- Cross-platform deployment (Python, Node, WASM, native)
- OS-level integration (system daemons)
- Browser/edge execution (WASM)

The `specs/` directory is the **source of truth**. All implementations must conform to these specifications and pass the conformance test suite.

---

*This document provides complete context for implementing CRA as a protocol-first Rust core with universal language bindings.*
