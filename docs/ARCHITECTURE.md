# CRA Architecture

This document describes the architecture of the Context Registry Agents (CRA) platform.

## Table of Contents

- [Overview](#overview)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [Protocol Stack](#protocol-stack)
- [Storage Architecture](#storage-architecture)
- [Security Model](#security-model)
- [Scalability](#scalability)

---

## Overview

CRA is designed as a **runtime authority layer** that sits between AI agents and the systems they interact with. The architecture ensures:

1. **Deterministic Resolution** — Same inputs produce same outputs
2. **Immutable Audit Trail** — Every operation is recorded
3. **Platform Agnostic** — Works with any LLM provider
4. **Governance First** — Policies are enforced, not suggested

```
┌─────────────────────────────────────────────────────────────────┐
│                        AI Agents                                 │
│  (OpenAI, Claude, LangChain, CrewAI, Custom)                    │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      CRA Middleware                              │
│  (OpenAIMiddleware, LangChainMiddleware, CRAMiddleware)         │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       CRA Runtime                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Resolver   │  │   Executor   │  │    Tracer    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │Policy Engine │  │ Atlas Loader │  │Session Mgmt  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Storage Layer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Trace Store  │  │Session Store │  │ Atlas Store  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### CRA Runtime

The Runtime is the central authority that processes all agent requests.

```
cra/runtime/
├── server.py           # FastAPI application factory
├── api/                # REST API endpoints
│   ├── health.py       # Health checks
│   ├── sessions.py     # Session management
│   ├── carp.py         # CARP resolution
│   ├── execute.py      # Action execution
│   ├── traces.py       # Trace queries & streaming
│   └── atlas.py        # Atlas management
└── services/           # Business logic
    ├── resolver.py     # CARP resolution logic
    ├── executor.py     # Action execution
    ├── tracer.py       # TRACE event emission
    ├── session_manager.py
    └── atlas_service.py
```

#### Key Responsibilities

| Component | Responsibility |
|-----------|---------------|
| **Resolver** | Processes CARP requests, determines context and permissions |
| **Executor** | Validates and executes granted actions |
| **Tracer** | Emits immutable TRACE events for all operations |
| **Session Manager** | Manages principal sessions and lifecycle |
| **Policy Engine** | Evaluates policies and enforces constraints |

### Domain Models

Core domain types that define the CARP and TRACE protocols:

```
cra/core/
├── carp.py         # CARP protocol types
│   ├── ContextBlock
│   ├── AllowedAction
│   ├── DenyRule
│   ├── Resolution
│   └── MergeRules
├── trace.py        # TRACE event types
│   ├── TraceEvent
│   ├── TraceContext
│   ├── Actor
│   └── Severity
├── session.py      # Session management
│   ├── Session
│   └── Principal
├── atlas.py        # Atlas types
│   ├── AtlasManifest
│   └── LoadedAtlas
└── policy.py       # Policy engine
    ├── PolicyEngine
    └── PolicyRule
```

### Platform Adapters

Adapters translate CRA resolutions to platform-specific formats:

```
cra/adapters/
├── base.py         # BaseAdapter interface
├── openai.py       # OpenAI function calling format
├── anthropic.py    # Claude SKILL.md format
├── google_adk.py   # Google ADK AgentTool format
└── mcp.py          # MCP server descriptor format
```

#### Adapter Interface

```python
class BaseAdapter(ABC):
    @property
    def platform_name(self) -> str: ...

    def emit_tools(
        self,
        actions: list[AllowedAction],
        atlas: LoadedAtlas | None = None,
    ) -> AdapterOutput: ...

    def emit_context(
        self,
        context_blocks: list[ContextBlock],
        resolution: Resolution,
    ) -> AdapterOutput: ...

    def emit_full(
        self,
        resolution: Resolution,
        atlas: LoadedAtlas | None = None,
    ) -> AdapterOutput: ...
```

---

## Data Flow

### Resolution Flow

```
1. Agent Request
   │
   ▼
2. Session Validation
   │ - Verify session exists and is valid
   │ - Check principal permissions
   │
   ▼
3. Atlas Loading
   │ - Load relevant Atlas(es)
   │ - Extract context packs and policies
   │
   ▼
4. Policy Evaluation
   │ - Scope validation
   │ - Deny pattern matching
   │ - Risk tier assessment
   │
   ▼
5. Resolution Building
   │ - Select context blocks
   │ - Determine allowed actions
   │ - Apply merge rules
   │
   ▼
6. TRACE Emission
   │ - Emit resolution events
   │
   ▼
7. Response to Agent
```

### Execution Flow

```
1. Execute Request
   │
   ▼
2. Resolution Validation
   │ - Verify resolution_id is valid
   │ - Check action is in allowed_actions
   │
   ▼
3. Constraint Checking
   │ - Validate parameters against schema
   │ - Check rate limits
   │ - Verify approval if required
   │
   ▼
4. Action Invocation
   │ - Execute the action
   │ - Capture result or error
   │
   ▼
5. TRACE Emission
   │ - Emit invoked event
   │ - Emit completed/failed event
   │
   ▼
6. Response to Agent
```

---

## Protocol Stack

### CARP Protocol

CARP (Context & Action Resolution Protocol) defines the contract for context resolution:

```
┌─────────────────────────────────────────────────┐
│                 CARP Envelope                   │
├─────────────────────────────────────────────────┤
│  carp_version: "1.0"                           │
│  type: "carp.request" | "carp.response"        │
│  id: UUID                                       │
│  time: RFC3339                                  │
├─────────────────────────────────────────────────┤
│  session: {                                     │
│    session_id: UUID                             │
│    principal: { type, id }                      │
│    scopes: [string]                             │
│  }                                              │
├─────────────────────────────────────────────────┤
│  payload: {                                     │
│    operation: "resolve" | "execute"             │
│    ...operation-specific fields                 │
│  }                                              │
├─────────────────────────────────────────────────┤
│  trace: {                                       │
│    trace_id: UUID                               │
│    span_id: UUID                                │
│    parent_span_id: UUID | null                  │
│  }                                              │
└─────────────────────────────────────────────────┘
```

### TRACE Protocol

TRACE (Telemetry & Replay Contract) defines immutable event records:

```
┌─────────────────────────────────────────────────┐
│                 TRACE Event                     │
├─────────────────────────────────────────────────┤
│  trace_version: "1.0"                          │
│  event_type: "trace.{category}.{action}"       │
│  time: RFC3339                                  │
│  severity: debug | info | warn | error          │
├─────────────────────────────────────────────────┤
│  trace: {                                       │
│    trace_id: UUID                               │
│    span_id: UUID                                │
│    parent_span_id: UUID | null                  │
│  }                                              │
├─────────────────────────────────────────────────┤
│  session_id: UUID                               │
│  actor: { type, id }                            │
├─────────────────────────────────────────────────┤
│  payload: { ...event-specific data }            │
│  artifacts: [{ name, uri, sha256 }]             │
└─────────────────────────────────────────────────┘
```

#### Event Types

| Category | Events |
|----------|--------|
| `trace.session` | `started`, `ended` |
| `trace.carp` | `resolve.requested`, `resolve.returned` |
| `trace.action` | `invoked`, `completed`, `failed`, `denied` |
| `trace.policy` | `evaluated`, `violated` |

---

## Storage Architecture

### Storage Abstraction

```python
class TraceStore(ABC):
    async def append(self, event: TraceEvent) -> None: ...
    async def get_events(self, trace_id: UUID, ...) -> list[TraceEvent]: ...
    async def stream_events(self, trace_id: UUID, ...) -> AsyncIterator[TraceEvent]: ...
    async def get_traces(self, session_id: UUID, ...) -> list[dict]: ...

class SessionStore(ABC):
    async def create(self, session: Session) -> Session: ...
    async def get(self, session_id: UUID) -> Session | None: ...
    async def update(self, session: Session) -> Session: ...
    async def delete(self, session_id: UUID) -> bool: ...
```

### Backend Implementations

| Backend | Use Case | Features |
|---------|----------|----------|
| **InMemoryStore** | Development | Fast, no persistence |
| **PostgresStore** | Production | Durable, scalable, streaming via LISTEN/NOTIFY |

### PostgreSQL Schema

```sql
CREATE TABLE trace_events (
    id SERIAL PRIMARY KEY,
    trace_id UUID NOT NULL,
    span_id UUID NOT NULL,
    parent_span_id UUID,
    session_id UUID NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    actor_type VARCHAR(50) NOT NULL,
    actor_id VARCHAR(255) NOT NULL,
    payload JSONB,
    artifacts JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX trace_events_trace_id_idx ON trace_events (trace_id);
CREATE INDEX trace_events_session_idx ON trace_events (session_id);
CREATE INDEX trace_events_time_idx ON trace_events (created_at);
```

---

## Security Model

### Authentication

CRA supports two authentication methods:

1. **JWT Tokens** — For user and service authentication
2. **API Keys** — For machine-to-machine communication

```
┌──────────────────────────────────────────────────────────────┐
│                     Request                                   │
│  Authorization: Bearer <jwt>                                  │
│  X-API-Key: <api_key>                                        │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                   Auth Middleware                             │
│  1. Try JWT verification                                      │
│  2. Try API key validation                                    │
│  3. Extract principal                                         │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                   RBAC Engine                                 │
│  Check: principal has required permission?                    │
└──────────────────────────────────────────────────────────────┘
```

### RBAC Permissions

| Permission | Description |
|------------|-------------|
| `atlas:read` | View Atlas metadata and content |
| `atlas:write` | Create and modify Atlases |
| `session:create` | Create new sessions |
| `carp:resolve` | Perform CARP resolution |
| `carp:execute` | Execute granted actions |
| `trace:read` | Query trace events |
| `admin:*` | Administrative operations |

### Built-in Roles

```python
BUILTIN_ROLES = {
    "admin": [all permissions],
    "developer": ["atlas:read", "atlas:write", "session:*", "carp:*", "trace:read"],
    "agent": ["atlas:read", "session:create", "carp:*", "trace:read"],
    "viewer": ["atlas:read", "session:read", "trace:read"],
    "auditor": ["atlas:read", "session:read", "trace:*", "admin:audit"],
}
```

---

## Scalability

### Horizontal Scaling

```
                    ┌─────────────────┐
                    │  Load Balancer  │
                    └────────┬────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
    ┌──────────┐       ┌──────────┐       ┌──────────┐
    │ Runtime  │       │ Runtime  │       │ Runtime  │
    │ Instance │       │ Instance │       │ Instance │
    └────┬─────┘       └────┬─────┘       └────┬─────┘
         │                  │                  │
         └──────────────────┼──────────────────┘
                            │
                    ┌───────┴───────┐
                    │  PostgreSQL   │
                    │   (Primary)   │
                    └───────┬───────┘
                            │
                    ┌───────┴───────┐
                    │  PostgreSQL   │
                    │   (Replica)   │
                    └───────────────┘
```

### Caching Strategy

1. **Atlas Cache** — Loaded Atlases cached in memory
2. **Resolution Cache** — Frequently used resolutions
3. **Policy Cache** — Compiled policy rules

### Performance Considerations

| Operation | Target Latency |
|-----------|---------------|
| Health check | < 5ms |
| Session creation | < 50ms |
| CARP resolution | < 100ms |
| Action execution | < 200ms + action time |
| Trace query | < 100ms |

---

## Observability

### OpenTelemetry Export

TRACE events can be exported as OpenTelemetry spans:

```
TRACE Event                         OTel Span
┌────────────────┐                 ┌────────────────┐
│ trace_id       │ ─────────────▶  │ trace_id       │
│ span_id        │                 │ span_id        │
│ event_type     │ ─────────────▶  │ name           │
│ severity       │ ─────────────▶  │ status         │
│ payload        │ ─────────────▶  │ attributes     │
└────────────────┘                 └────────────────┘
```

### SIEM Integration

TRACE events can be exported to SIEM systems:

| Format | Target System |
|--------|---------------|
| CEF | ArcSight, Splunk |
| LEEF | IBM QRadar |
| JSON | Generic SIEM |
| Syslog | RFC 5424 compatible |

---

## Extension Points

### Custom Policy Rules

```python
class CustomPolicyRule(PolicyRule):
    def evaluate(
        self,
        session: Session,
        resolution: Resolution,
        context: PolicyContext,
    ) -> PolicyDecision:
        # Custom logic
        return PolicyDecision.ALLOW
```

### Custom Adapters

```python
class CustomAdapter(BaseAdapter):
    @property
    def platform_name(self) -> str:
        return "custom"

    def emit_tools(self, actions, atlas):
        # Custom format
        return CustomOutput(...)
```

### Custom Storage

```python
class CustomTraceStore(TraceStore):
    async def append(self, event: TraceEvent) -> None:
        # Custom storage logic
        pass
```

---

## Design Decisions

### Why Runtime Authority?

LLMs are non-deterministic and cannot be trusted to correctly use tools. By making the runtime authoritative:

1. **Consistent behavior** — Same request produces same resolution
2. **Enforceable policies** — Agents cannot bypass governance
3. **Auditable operations** — Every action has proof

### Why Append-Only TRACE?

Append-only logs provide:

1. **Immutability** — Events cannot be altered after emission
2. **Replay** — Exact reproduction of historical sessions
3. **Compliance** — Tamper-evident audit trails

### Why Platform Adapters?

Different AI platforms have different tool formats. Adapters:

1. **Decouple** — Atlas authors write once
2. **Optimize** — Platform-specific best practices
3. **Evolve** — Update adapters without changing Atlases

---

*For implementation details, see the [API Reference](API.md) and [source code](../cra/).*
