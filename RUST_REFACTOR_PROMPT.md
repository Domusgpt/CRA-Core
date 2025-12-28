# CRA: Context Registry Agents — Complete System Overview & Rust Refactor Guide

## Table of Contents

1. [The Problem](#1-the-problem)
2. [The Solution: CRA](#2-the-solution-cra)
3. [Core Protocols](#3-core-protocols)
4. [The Atlas System](#4-the-atlas-system)
5. [Platform Vision](#5-platform-vision)
6. [Current Implementation Status](#6-current-implementation-status)
7. [Future Roadmap](#7-future-roadmap)
8. [Dual-Mode Architecture](#8-dual-mode-architecture) ← **CRITICAL PATTERN**
9. [Why Rust Core](#9-why-rust-core)
10. [Rust Refactor Implementation Plan](#10-rust-refactor-implementation-plan)
11. [Reference Materials](#11-reference-materials)

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

## 8. Dual-Mode Architecture

### The TypeScript Branch Pattern

The TypeScript implementation (`claude/design-cra-architecture-WdoAv`) demonstrates an excellent **dual-mode architecture** that the Rust core MUST adopt:

```
┌─────────────────────────────────────────────────────────────────────┐
│                      DUAL-MODE ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   Mode 1: EMBEDDED (Library)          Mode 2: SERVICE (HTTP)        │
│   ─────────────────────────           ──────────────────────        │
│                                                                      │
│   ┌──────────────────────┐           ┌──────────────────────┐       │
│   │   Your Application   │           │   Your Application   │       │
│   │                      │           │                      │       │
│   │   ┌──────────────┐   │           │   HTTP Client        │       │
│   │   │  CRARuntime  │   │           │        │             │       │
│   │   │  (in-process)│   │           └────────┼─────────────┘       │
│   │   └──────────────┘   │                    │                     │
│   │        │             │                    │ HTTP                │
│   └────────┼─────────────┘                    ▼                     │
│            │                           ┌──────────────────────┐     │
│            │ Direct call               │   CRAServer          │     │
│            │ ~0.001ms                  │   ┌──────────────┐   │     │
│            ▼                           │   │  CRARuntime  │   │     │
│   ┌──────────────────────┐             │   │  (in-process)│   │     │
│   │    Atlas/Tools       │             │   └──────────────┘   │     │
│   └──────────────────────┘             └──────────────────────┘     │
│                                                                      │
│   Use when:                            Use when:                     │
│   • Same-process agents                • Multi-service architecture │
│   • Minimum latency needed             • Language without bindings  │
│   • Edge/browser deployment            • Centralized governance     │
│   • WASM environments                  • Existing infrastructure    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### TypeScript Implementation Reference

The TypeScript branch shows this pattern with two separate packages:

**CRARuntime (packages/runtime/src/runtime.ts):**
```typescript
export class CRARuntime {
  private atlasRegistry: AtlasRegistry;
  private traceCollector: TraceCollector;
  private sessions: Map<string, Session>;

  constructor(config?: RuntimeConfig) {
    this.atlasRegistry = new AtlasRegistry();
    this.traceCollector = new TraceCollector();
    this.sessions = new Map();
  }

  // Core library API - no HTTP, no server
  async resolve(request: CARPRequest): Promise<CARPResolution> { ... }
  async execute(sessionId: string, actionId: string, params: unknown): Promise<unknown> { ... }
  async loadAtlas(path: string): Promise<void> { ... }
  getTrace(sessionId: string): TraceEvent[] { ... }
}
```

**CRAServer (packages/server/src/server.ts):**
```typescript
export class CRAServer {
  private runtime: CRARuntime;  // ← Wraps the library
  private app: Express;

  constructor(config?: ServerConfig) {
    this.runtime = new CRARuntime(config?.runtime);  // ← Composition
    this.app = express();
    this.setupRoutes();
  }

  private setupRoutes() {
    this.app.post('/v1/resolve', async (req, res) => {
      const resolution = await this.runtime.resolve(req.body);  // ← Delegates
      res.json(resolution);
    });
  }

  listen(port: number) {
    this.app.listen(port);
  }
}
```

**Key Design Principles:**
1. **Library-first** — The runtime is a complete, standalone library
2. **Server wraps library** — HTTP layer is thin, just routing and serialization
3. **Same behavior** — Embedded and HTTP modes produce identical results
4. **No hidden state** — All state lives in the runtime, server is stateless

### Scaling Path

The TypeScript branch defines a clear scaling progression that the Rust implementation should enable:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         SCALING PATH                                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   Stage 1: SINGLE-NODE              Stage 2: SAAS                   │
│   ──────────────────────           ─────────────                    │
│   • Embedded library                • Multi-tenant server           │
│   • In-memory storage               • PostgreSQL storage            │
│   • Single process                  • JWT authentication            │
│   • For: Local dev, CLI             • For: Small teams              │
│                                                                      │
│   ┌────────────────────┐           ┌────────────────────┐           │
│   │     Agent App      │           │    CRA Service     │           │
│   │  ┌──────────────┐  │           │  ┌──────────────┐  │           │
│   │  │  CRARuntime  │  │           │  │  CRAServer   │  │           │
│   │  │  (embedded)  │  │           │  │  (shared)    │  │           │
│   │  └──────────────┘  │           │  └──────────────┘  │           │
│   └────────────────────┘           └────────────────────┘           │
│                                                                      │
│   Stage 3: ENTERPRISE               Stage 4: EDGE                   │
│   ───────────────────              ────────────────                 │
│   • SSO/SAML integration           • WASM in browser                │
│   • Compliance dashboards          • Cloudflare Workers             │
│   • Audit log retention            • Embedded in IoT                │
│   • For: Large orgs                • For: Distributed agents        │
│                                                                      │
│   ┌────────────────────┐           ┌────────────────────┐           │
│   │   Enterprise CRA   │           │  Edge Device/CDN   │           │
│   │  ┌──────────────┐  │           │  ┌──────────────┐  │           │
│   │  │  CRACluster  │  │           │  │  CRA-WASM    │  │           │
│   │  │  (HA/Scaled) │  │           │  │  (embedded)  │  │           │
│   │  └──────────────┘  │           │  └──────────────┘  │           │
│   └────────────────────┘           └────────────────────┘           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 9. Why Rust Core

### The Infrastructure Requirement

CRA needs to be **infrastructure** — embedded everywhere, invisible, just how things work:

```
Current State (Python/TypeScript HTTP):
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

## 10. Rust Refactor Implementation Plan

### Target Architecture (Dual-Mode)

The architecture MUST follow the dual-mode pattern demonstrated in the TypeScript branch:

```
cra-rust/
├── cra-core/           # Core library (MODE 1: EMBEDDED)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Public API (CRARuntime)
│       ├── runtime/
│       │   ├── mod.rs          # CRARuntime struct
│       │   ├── config.rs       # RuntimeConfig
│       │   └── session.rs      # Session management
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
│       │   ├── registry.rs     # Atlas registry
│       │   ├── loader.rs       # Load from disk/memory
│       │   └── validator.rs    # JSON Schema validation
│       ├── storage/
│       │   ├── mod.rs          # StorageBackend trait
│       │   ├── memory.rs       # In-memory (default)
│       │   └── postgres.rs     # PostgreSQL (optional)
│       └── ffi/
│           └── c_api.rs        # C ABI for any language
│
├── cra-server/         # HTTP server (MODE 2: SERVICE)
│   ├── Cargo.toml      # Depends on cra-core
│   └── src/
│       ├── main.rs             # CLI entry point
│       ├── lib.rs              # CRAServer struct
│       ├── routes/
│       │   ├── mod.rs
│       │   ├── sessions.rs     # /v1/sessions/*
│       │   ├── resolve.rs      # /v1/resolve
│       │   ├── execute.rs      # /v1/sessions/{id}/execute
│       │   ├── traces.rs       # /v1/sessions/{id}/trace
│       │   └── atlases.rs      # /v1/atlases/*
│       ├── auth/
│       │   ├── mod.rs
│       │   ├── jwt.rs          # JWT validation
│       │   └── api_key.rs      # API key validation
│       ├── middleware/
│       │   ├── mod.rs
│       │   ├── tenant.rs       # Multi-tenancy
│       │   └── rate_limit.rs   # Rate limiting
│       └── config.rs           # ServerConfig
│
├── cra-python/         # Python binding (PyO3)
│   ├── Cargo.toml      # Depends on cra-core
│   ├── src/lib.rs
│   └── python/cra/
│       ├── __init__.py
│       ├── runtime.py          # CRARuntime wrapper
│       └── middleware/
│           ├── langchain.py
│           └── crewai.py
│
├── cra-node/           # Node.js binding (napi-rs)
│   ├── Cargo.toml      # Depends on cra-core
│   ├── src/lib.rs
│   └── npm/
│       ├── package.json
│       └── index.d.ts
│
└── cra-wasm/           # WASM binding (wasm-bindgen)
    ├── Cargo.toml      # Depends on cra-core (no_std where possible)
    ├── src/lib.rs
    └── pkg/            # Generated npm package
```

### Dual-Mode API Design

The Rust implementation MUST expose the same API for both modes:

**Mode 1: Embedded Library (cra-core)**
```rust
use cra_core::{CRARuntime, RuntimeConfig};

// Create runtime (library usage)
let config = RuntimeConfig::builder()
    .with_storage(MemoryStorage::new())  // Or PostgresStorage
    .build();

let mut runtime = CRARuntime::new(config);

// Load atlases
runtime.load_atlas("./atlas.json")?;

// Create session and resolve (direct call, ~0.001ms)
let session = runtime.create_session("agent-1", "Help with support")?;
let resolution = runtime.resolve(&session.id, &request)?;

// Execute action
let result = runtime.execute(&session.id, "ticket.get", json!({"ticket_id": "123"}))?;

// Get trace
let trace = runtime.get_trace(&session.id)?;
```

**Mode 2: HTTP Server (cra-server)**
```rust
use cra_server::{CRAServer, ServerConfig};
use cra_core::{CRARuntime, RuntimeConfig};

// Create runtime first (same as embedded)
let runtime = CRARuntime::new(RuntimeConfig::default());

// Wrap in server (thin HTTP layer)
let server_config = ServerConfig::builder()
    .with_port(3000)
    .with_auth(JWTAuth::new("secret"))
    .build();

let server = CRAServer::new(runtime, server_config);  // ← Composition!
server.listen().await?;  // Starts HTTP server
```

**Key Principle: Server Wraps Runtime**
```rust
// CRAServer implementation pattern
pub struct CRAServer {
    runtime: CRARuntime,  // ← The library
    config: ServerConfig,
}

impl CRAServer {
    pub fn new(runtime: CRARuntime, config: ServerConfig) -> Self {
        Self { runtime, config }
    }

    // HTTP handlers delegate to runtime
    async fn handle_resolve(&self, req: Request) -> Response {
        let carp_request: CARPRequest = req.json().await?;
        let resolution = self.runtime.resolve(&req.session_id(), &carp_request)?;
        Response::json(&resolution)
    }
}
```

### Phase 1: Rust Core Library (cra-core)

Implement the core runtime as a standalone library:

```rust
// cra-core/src/lib.rs - Public API
pub use runtime::{CRARuntime, RuntimeConfig};
pub use carp::{CARPRequest, CARPResolution};
pub use trace::{TRACEEvent, TraceCollector};
pub use atlas::{Atlas, AtlasRegistry};
pub use storage::{StorageBackend, MemoryStorage};

// cra-core/src/runtime/mod.rs
pub struct CRARuntime {
    config: RuntimeConfig,
    atlas_registry: AtlasRegistry,
    trace_collector: TraceCollector,
    sessions: HashMap<String, Session>,
    storage: Box<dyn StorageBackend>,
}

impl CRARuntime {
    pub fn new(config: RuntimeConfig) -> Self;

    // Atlas management
    pub fn load_atlas(&mut self, path: &str) -> Result<AtlasId, Error>;
    pub fn load_atlas_from_json(&mut self, json: &str) -> Result<AtlasId, Error>;
    pub fn unload_atlas(&mut self, atlas_id: &str) -> Result<(), Error>;
    pub fn list_atlases(&self) -> Vec<&Atlas>;

    // Session management
    pub fn create_session(&mut self, agent_id: &str, goal: &str) -> Result<Session, Error>;
    pub fn get_session(&self, session_id: &str) -> Option<&Session>;
    pub fn end_session(&mut self, session_id: &str) -> Result<(), Error>;

    // CARP operations
    pub fn resolve(&self, session_id: &str, request: &CARPRequest) -> Result<CARPResolution, Error>;
    pub fn execute(&mut self, session_id: &str, action_id: &str, params: Value) -> Result<Value, Error>;

    // TRACE operations
    pub fn get_trace(&self, session_id: &str) -> Result<Vec<TRACEEvent>, Error>;
    pub fn verify_chain(&self, session_id: &str) -> Result<ChainVerification, Error>;
    pub fn replay(&self, trace: &[TRACEEvent], atlas: &Atlas) -> Result<ReplayResult, Error>;
}
```

**Phase 1 Deliverables:**
- [ ] CARP resolver with policy evaluation
- [ ] TRACE collector with SHA-256 hash chain
- [ ] Atlas loader and validator
- [ ] In-memory storage backend
- [ ] 100% conformance test passing

### Phase 2: HTTP Server (cra-server)

Implement the HTTP layer as a thin wrapper around cra-core:

```rust
// cra-server/src/lib.rs
use cra_core::CRARuntime;

pub struct CRAServer {
    runtime: CRARuntime,  // ← Wraps the library (composition)
    config: ServerConfig,
}

impl CRAServer {
    pub fn new(runtime: CRARuntime, config: ServerConfig) -> Self {
        Self { runtime, config }
    }

    pub async fn listen(&self) -> Result<(), Error> {
        let app = self.build_router();
        axum::Server::bind(&self.config.addr)
            .serve(app.into_make_service())
            .await
    }

    fn build_router(&self) -> Router {
        Router::new()
            .route("/v1/sessions", post(handlers::create_session))
            .route("/v1/sessions/:id", get(handlers::get_session))
            .route("/v1/sessions/:id", delete(handlers::end_session))
            .route("/v1/resolve", post(handlers::resolve))
            .route("/v1/sessions/:id/execute", post(handlers::execute))
            .route("/v1/sessions/:id/trace", get(handlers::get_trace))
            .route("/v1/atlases", get(handlers::list_atlases))
            .route("/v1/atlases", post(handlers::load_atlas))
            .layer(auth_layer)
            .with_state(Arc::new(self.runtime.clone()))
    }
}
```

**Phase 2 Deliverables:**
- [ ] HTTP server with all OpenAPI routes
- [ ] JWT and API key authentication
- [ ] Multi-tenant middleware
- [ ] Rate limiting
- [ ] PostgreSQL storage backend (optional feature)

### Phase 3: Python Binding (cra-python)

Drop-in replacement for current Python implementation:

```python
from cra import CRARuntime, RuntimeConfig

# Rust-powered, but same API as TypeScript runtime
config = RuntimeConfig(storage="memory")  # or "postgres://..."
runtime = CRARuntime(config)

# Load atlases
runtime.load_atlas("./atlas.json")

# Create session and resolve (direct call to Rust, ~0.001ms)
session = runtime.create_session(
    agent_id="support-agent",
    goal="Help with support ticket"
)

resolution = runtime.resolve(session.id, request)

for action in resolution.allowed_actions:
    print(f"Available: {action.action_id}")

# Execute action
result = runtime.execute(session.id, "ticket.get", {"ticket_id": "123"})

# Get trace
trace = runtime.get_trace(session.id)
```

**LangChain/CrewAI Middleware (Python wrapper around Rust runtime):**
```python
from cra.middleware.langchain import CRAMiddleware

# Wraps Rust runtime in LangChain-compatible tools
middleware = CRAMiddleware(runtime)
tools = middleware.get_tools(goal="Customer support")

# Use with LangChain agents
from langchain.agents import AgentExecutor
agent = AgentExecutor(agent=agent, tools=tools)
```

**Phase 3 Deliverables:**
- [ ] PyO3 bindings exposing full CRARuntime API
- [ ] Compatible with existing Python middleware patterns
- [ ] LangChain/CrewAI middleware wrappers
- [ ] Pip installable package with pre-built wheels

### Phase 4: Node.js Binding (cra-node)

Native addon for MCP servers and tooling:

```typescript
import { CRARuntime, RuntimeConfig } from '@cra/core';

// Same API pattern as Rust and Python
const config: RuntimeConfig = { storage: 'memory' };
const runtime = new CRARuntime(config);

// Load atlases
await runtime.loadAtlas('./atlas.json');

// Create session and resolve
const session = runtime.createSession('mcp-client', 'Execute MCP tool');
const resolution = runtime.resolve(session.id, request);

// MCP server wrapping CRA runtime
import { createMCPServer } from '@cra/mcp';
const mcpServer = createMCPServer(runtime);  // ← Wraps runtime
mcpServer.listen(3001);
```

**Phase 4 Deliverables:**
- [ ] napi-rs bindings exposing full CRARuntime API
- [ ] MCP server implementation
- [ ] npm package with pre-built binaries

### Phase 5: WASM Build (cra-wasm)

Browser and edge deployment:

```typescript
import init, { CRARuntime } from '@cra/wasm';

await init(); // Load WASM module (~300KB)

const runtime = new CRARuntime();
runtime.loadAtlasFromJson(atlasJson);

// Create session and resolve (runs entirely client-side)
const session = runtime.createSession('browser-agent', 'Client-side validation');
const resolution = runtime.resolve(session.id, request);

// Works in:
// - Browsers (React, Vue, Svelte)
// - Cloudflare Workers
// - Deno Deploy
// - Edge functions
```

**Phase 5 Deliverables:**
- [ ] wasm-bindgen build with full API
- [ ] <500KB bundle size
- [ ] Works in browsers and edge runtimes

### Conformance Requirements

The Rust implementation MUST pass all tests in `specs/conformance/`:

1. **Schema Validation** — All JSON validates against `specs/schemas/*.json`
2. **Hash Chain** — SHA-256 computation matches reference implementation
3. **Policy Evaluation** — Correct ordering: deny → approval → rate_limit → allow
4. **Golden Traces** — Output matches `specs/conformance/golden/*`
5. **Replay Determinism** — Same input always produces same output

---

## 11. Reference Materials

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
| Embedded Resolution Latency | <0.01ms (library mode) |
| HTTP Resolution Latency | <5ms (server mode) |
| Core Binary Size | <1MB (release) |
| WASM Size | <500KB |
| Python Binding | Compatible with existing middleware |
| Node Binding | Works with MCP server |
| Dual-Mode Parity | Identical behavior embedded vs. HTTP |

### TypeScript Reference Implementation

The TypeScript branch (`claude/design-cra-architecture-WdoAv`) provides working reference code:

| File | What to Learn |
|------|---------------|
| `packages/runtime/src/runtime.ts` | CRARuntime library pattern |
| `packages/server/src/server.ts` | CRAServer wrapping runtime |
| `packages/trace/src/collector.ts` | Hash chain implementation |
| `packages/atlas/src/loader.ts` | Atlas validation logic |
| `docs/ARCHITECTURE.md` | Scaling path stages |
| `docs/IMPROVEMENT_PLAN.md` | Gaps to address |

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

### Dual-Mode Architecture Is Mandatory

The Rust implementation MUST follow the dual-mode pattern from the TypeScript branch:

```
┌────────────────────────────────────────────────────────────────┐
│                      CRA ARCHITECTURE                          │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│   cra-core (CRARuntime)         cra-server (CRAServer)        │
│   ─────────────────────         ──────────────────────        │
│   • Standalone library           • Wraps cra-core             │
│   • No HTTP, no server           • Thin HTTP layer            │
│   • In-process calls             • Same API via REST          │
│   • ~0.001ms latency             • ~5ms latency               │
│   • Use for: embedding           • Use for: services          │
│                                                                │
│   ┌──────────────┐              ┌──────────────────┐          │
│   │  CRARuntime  │◀─────────────│    CRAServer     │          │
│   │              │  composes    │   ┌──────────┐   │          │
│   │              │              │   │ runtime  │   │          │
│   └──────────────┘              │   └──────────┘   │          │
│         │                       └──────────────────┘          │
│         │ bindings                                             │
│         ▼                                                      │
│   ┌────────────────────────────────────────────────┐          │
│   │  cra-python  │  cra-node  │  cra-wasm          │          │
│   │  (PyO3)      │  (napi-rs) │  (wasm-bindgen)    │          │
│   └────────────────────────────────────────────────┘          │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

**Key Principle:** The server is just a thin wrapper. All logic lives in the runtime library.

### The Rust Refactor Enables

| Capability | How |
|------------|-----|
| **In-process embedding** | cra-core as library |
| **HTTP service** | cra-server wrapping cra-core |
| **Python agents** | PyO3 binding to cra-core |
| **Node.js/MCP** | napi-rs binding to cra-core |
| **Browser/Edge** | WASM build of cra-core |
| **OS daemon** | Native binary of cra-server |
| **Scaling path** | Single-Node → SaaS → Enterprise → Edge |

### Source of Truth

The `specs/` directory defines all behavior:
- `specs/PROTOCOL.md` — Wire formats and semantics
- `specs/schemas/*.json` — JSON Schema validation
- `specs/conformance/golden/` — Reference test cases

All implementations (Rust, Python bindings, HTTP server) MUST pass conformance tests.

---

*This document provides complete context for implementing CRA as a protocol-first Rust core with dual-mode architecture (embedded library + HTTP server) and universal language bindings.*
