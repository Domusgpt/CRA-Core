# CRA Platform Implementation Plan (v0.1 → v1.0)

**Created:** 2025-12-27
**Status:** Planning
**Non-negotiable principle:** *TRACE is authoritative; LLM narration is non-authoritative.*

---

## Table of Contents

1. [Technology Stack Decision](#1-technology-stack-decision)
2. [Project Structure](#2-project-structure)
3. [Phase 0: Hello TRACE](#3-phase-0-hello-trace)
4. [Phase 1: Tight CARP/TRACE Compliance](#4-phase-1-tight-carptrace-compliance)
5. [Phase 2: Atlas Execution + Adapters](#5-phase-2-atlas-execution--adapters)
6. [Phase 3: Marketplace + Licensing](#6-phase-3-marketplace--licensing)
7. [Phase 4: Infrastructure Scale](#7-phase-4-infrastructure-scale)
8. [Schema Specifications](#8-schema-specifications)
9. [Testing Strategy](#9-testing-strategy)
10. [Open Questions](#10-open-questions)

---

## 1. Technology Stack Decision

### Recommended: Python + FastAPI

**Rationale:**
- FastAPI provides native async support (critical for SSE streaming)
- Pydantic v2 for strict JSON schema validation (CARP/TRACE envelopes)
- Rich ecosystem for CLI (Typer/Click) and async operations
- Strong typing support aligns with protocol strictness
- Widely adopted in AI/agent tooling (OpenAI SDK, Anthropic SDK, LangChain)

**Alternative Considered: TypeScript + Node.js**
- Pros: Native JSON, good for MCP integration (MCP SDK is TS)
- Cons: Less mature async patterns for complex streaming

### Stack Components

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Runtime API | FastAPI + Uvicorn | Async, OpenAPI auto-gen, SSE support |
| CLI | Typer + Rich | Modern CLI with async support, beautiful output |
| Schema Validation | Pydantic v2 | Strict validation, JSON Schema export |
| Database (traces) | SQLite (dev) → PostgreSQL (prod) | Simple start, scale later |
| Event Streaming | SSE via Starlette | Native FastAPI support |
| Trace Storage | Append-only log files + DB index | Immutability guarantee |
| Testing | pytest + pytest-asyncio | Industry standard |
| Packaging | Poetry | Dependency management + publishing |

---

## 2. Project Structure

```
CRA-Core/
├── README.md
├── IMPLEMENTATION_PLAN.md
├── pyproject.toml                 # Poetry config
├── cra/
│   ├── __init__.py
│   ├── version.py                 # Single source of version
│   │
│   ├── core/                      # Core domain models
│   │   ├── __init__.py
│   │   ├── carp.py                # CARP protocol types
│   │   ├── trace.py               # TRACE event types
│   │   ├── session.py             # Session management
│   │   ├── atlas.py               # Atlas types
│   │   └── policy.py              # Policy engine types
│   │
│   ├── runtime/                   # CRA Runtime (the authority)
│   │   ├── __init__.py
│   │   ├── server.py              # FastAPI app factory
│   │   ├── api/
│   │   │   ├── __init__.py
│   │   │   ├── health.py          # GET /v1/health
│   │   │   ├── sessions.py        # Session endpoints
│   │   │   ├── carp.py            # CARP resolution endpoints
│   │   │   └── traces.py          # TRACE query/stream endpoints
│   │   ├── services/
│   │   │   ├── __init__.py
│   │   │   ├── resolver.py        # CARP resolution logic
│   │   │   ├── tracer.py          # TRACE emission service
│   │   │   ├── policy_engine.py   # Policy evaluation
│   │   │   └── session_manager.py # Session lifecycle
│   │   └── storage/
│   │       ├── __init__.py
│   │       ├── trace_store.py     # Append-only trace storage
│   │       └── session_store.py   # Session state
│   │
│   ├── cli/                       # CLI application
│   │   ├── __init__.py
│   │   ├── main.py                # Typer app entry
│   │   ├── commands/
│   │   │   ├── __init__.py
│   │   │   ├── doctor.py          # cra doctor
│   │   │   ├── init.py            # cra init
│   │   │   ├── resolve.py         # cra resolve
│   │   │   └── trace.py           # cra trace tail/replay
│   │   └── output/
│   │       ├── __init__.py
│   │       ├── jsonl.py           # JSONL formatter
│   │       └── human.py           # Human-readable formatter
│   │
│   ├── adapters/                  # Platform adapters (Phase 2)
│   │   ├── __init__.py
│   │   ├── base.py                # Adapter interface
│   │   ├── openai.py              # OpenAI tools schema
│   │   ├── anthropic.py           # Claude SKILL.md
│   │   ├── google_adk.py          # Google ADK AgentTool
│   │   └── mcp.py                 # MCP server descriptor
│   │
│   └── schemas/                   # JSON Schema definitions
│       ├── __init__.py
│       ├── carp_v1.json
│       └── trace_v1.json
│
├── tests/
│   ├── conftest.py
│   ├── unit/
│   │   ├── test_carp.py
│   │   ├── test_trace.py
│   │   └── test_policy.py
│   ├── integration/
│   │   ├── test_runtime_api.py
│   │   └── test_cli.py
│   └── conformance/               # CARP/TRACE conformance suite (Phase 1)
│       ├── test_carp_compliance.py
│       └── test_trace_compliance.py
│
├── examples/
│   ├── atlases/
│   │   └── hello-world/           # Example Atlas
│   │       ├── atlas.json
│   │       ├── context/
│   │       └── policies/
│   └── traces/
│       └── golden/                # Golden trace examples
│
└── docs/
    ├── CARP_1_0.md
    ├── TRACE_1_0.md
    └── ARCHITECTURE.md
```

---

## 3. Phase 0: Hello TRACE

**Goal:** Runnable runtime + CLI with real TRACE streaming

### 3.1 Runtime API Endpoints

#### GET /v1/health
```python
# Response
{
    "status": "healthy",
    "version": "0.1.0",
    "carp_version": "1.0",
    "trace_version": "1.0",
    "uptime_seconds": 123.45
}
```

#### POST /v1/sessions
```python
# Request
{
    "principal": {
        "type": "user|service|agent",
        "id": "string"
    },
    "scopes": ["atlas.read", "action.execute"],
    "ttl_seconds": 3600  # optional, default 1h
}

# Response
{
    "session_id": "uuid",
    "expires_at": "RFC3339",
    "trace_id": "uuid"  # Root trace for this session
}

# TRACE emitted: trace.session.started
```

#### POST /v1/sessions/{id}/end
```python
# Response
{
    "session_id": "uuid",
    "ended_at": "RFC3339",
    "trace_summary": {
        "total_events": 42,
        "resolutions": 5,
        "actions_executed": 12
    }
}

# TRACE emitted: trace.session.ended
```

#### POST /v1/carp/resolve
```python
# Request: Full CARP envelope (see schema section)

# Response: Resolution Bundle
{
    "carp_version": "1.0",
    "type": "carp.response",
    "id": "uuid",
    "time": "RFC3339",
    "session": { ... },
    "payload": {
        "operation": "resolve",
        "resolution": {
            "resolution_id": "uuid",
            "confidence": 0.95,
            "context_blocks": [...],
            "allowed_actions": [...],
            "denylist": [...],
            "merge_rules": {...},
            "next_steps": [...]
        }
    },
    "trace": {
        "trace_id": "uuid",
        "span_id": "uuid",
        "parent_span_id": "uuid|null"
    }
}

# TRACE emitted: trace.carp.resolve.requested, trace.carp.resolve.returned
```

#### GET /v1/traces/{trace_id}/events
```python
# Query params: ?severity=info&event_type=carp.*&limit=100&offset=0

# Response
{
    "trace_id": "uuid",
    "events": [...],
    "total_count": 150,
    "has_more": true
}
```

#### GET /v1/traces/{trace_id}/stream (SSE)
```python
# Server-Sent Events stream
# Each event is a JSONL line

data: {"trace_version":"1.0","event_type":"trace.session.started",...}

data: {"trace_version":"1.0","event_type":"trace.carp.resolve.requested",...}
```

### 3.2 CLI Commands

#### cra doctor
```bash
$ cra doctor

CRA Doctor - System Check
=========================
Runtime:     http://localhost:8420  [OK]
Version:     0.1.0                  [OK]
CARP:        1.0                    [OK]
TRACE:       1.0                    [OK]
Config:      ./cra.config.json      [FOUND]
Trace Dir:   ./cra.trace/           [WRITABLE]
Atlases:     1 loaded               [OK]

All checks passed.
```

#### cra init
```bash
$ cra init

Initializing CRA project...
Created: agents.md           # Agent behavior contract
Created: cra.config.json     # Runtime configuration
Created: cra.trace/          # Local trace storage
Created: cra.atlases.lock    # Atlas version lock

Project initialized. Run 'cra doctor' to verify.
```

**Generated files:**

`agents.md`:
```markdown
# CRA Agent Contract

## Rules
1. Always resolve via CARP before taking action
2. Never guess tool usage or API behavior
3. TRACE output is authoritative; LLM narration is not
4. Respect TTLs on context blocks
5. Honor denylist patterns

## Runtime
- Endpoint: ${CRA_RUNTIME_URL}
- Session: Active sessions expire per TTL
```

`cra.config.json`:
```json
{
    "cra_version": "0.1.0",
    "runtime": {
        "url": "http://localhost:8420",
        "timeout_ms": 30000
    },
    "trace": {
        "directory": "./cra.trace",
        "retention_days": 30,
        "streaming": true
    },
    "atlases": [],
    "policies": {
        "default_risk_tier": "medium",
        "require_approval_for": ["high"]
    }
}
```

#### cra resolve
```bash
$ cra resolve --goal "Deploy service to staging" --risk-tier high

# Output: Raw JSONL from runtime (unmodified)
{"trace_version":"1.0","event_type":"trace.carp.resolve.requested","time":"2025-01-01T12:00:00Z",...}
{"trace_version":"1.0","event_type":"trace.carp.resolve.returned","time":"2025-01-01T12:00:01Z",...}

# Resolution Bundle printed
{
  "resolution_id": "abc-123",
  "confidence": 0.92,
  "context_blocks": [
    {"block_id": "deploy-rules", "purpose": "Staging deployment constraints", ...}
  ],
  "allowed_actions": [
    {"action_id": "ci.deploy", "requires_approval": true, ...}
  ],
  "denylist": [
    {"pattern": "*.production.*", "reason": "Production access not in scope"}
  ]
}
```

#### cra trace tail
```bash
$ cra trace tail --trace-id abc-123 --follow

# Streaming JSONL output
{"trace_version":"1.0","event_type":"trace.action.invoked",...}
{"trace_version":"1.0","event_type":"trace.action.completed",...}

# With filters
$ cra trace tail --trace-id abc-123 --severity error --event-type "trace.action.*"
```

### 3.3 Phase 0 Implementation Steps

1. **Project setup**
   - Initialize Poetry project
   - Configure pyproject.toml with dependencies
   - Set up pre-commit hooks (ruff, mypy)

2. **Core domain models** (`cra/core/`)
   - Implement Pydantic models for CARP envelope
   - Implement Pydantic models for TRACE events
   - Implement Session model

3. **Trace storage** (`cra/runtime/storage/`)
   - Implement append-only file-based trace store
   - Add trace indexing for queries
   - Implement trace streaming generator

4. **Runtime API** (`cra/runtime/api/`)
   - Implement health endpoint
   - Implement session endpoints
   - Implement CARP resolve endpoint (basic resolution)
   - Implement trace query/stream endpoints

5. **Tracer service** (`cra/runtime/services/`)
   - Implement event emission
   - Wire tracer into all endpoints
   - Ensure all operations emit TRACE events

6. **CLI** (`cra/cli/`)
   - Implement Typer application
   - Implement doctor command
   - Implement init command (file generation)
   - Implement resolve command (calls runtime, streams JSONL)
   - Implement trace tail command

7. **Testing**
   - Unit tests for all core models
   - Integration tests for API endpoints
   - CLI tests using subprocess

8. **Documentation**
   - Update README with quick start
   - Add ARCHITECTURE.md

---

## 4. Phase 1: Tight CARP/TRACE Compliance

### 4.1 Strict Schema Validation

- Export Pydantic models to JSON Schema
- Validate all incoming requests against CARP schema
- Validate all emitted events against TRACE schema
- Reject non-conforming messages with detailed errors

### 4.2 Policy Hooks

```python
# Policy engine interface
class PolicyEngine:
    async def evaluate(
        self,
        session: Session,
        request: CARPRequest,
        context: PolicyContext
    ) -> PolicyDecision:
        """
        Returns: allow, deny, allow_with_constraints
        Emits: trace.carp.policy.evaluated
        """
        pass

# Hook types
- Scope validation (does principal have required scopes?)
- Deny rules (pattern matching against actions/resources)
- Redaction (remove PII/secrets from context blocks)
- Approval gates (require human approval for high-risk)
- Rate limiting (per-session, per-action limits)
```

### 4.3 Action Grants & Invocation

```python
# POST /v1/carp/execute
{
    "session_id": "uuid",
    "resolution_id": "uuid",  # From prior resolve
    "action_id": "ci.deploy",
    "parameters": {...}
}

# Response
{
    "execution_id": "uuid",
    "status": "pending|running|completed|failed",
    "result": {...},
    "trace": {...}
}

# TRACE emitted: trace.action.invoked, trace.action.completed|failed
```

### 4.4 Replay & Golden Traces

```python
# Replay manifest format
{
    "manifest_version": "1.0",
    "trace_id": "uuid",
    "artifacts": [
        {"name": "input.json", "sha256": "...", "uri": "..."},
        {"name": "expected_output.json", "sha256": "...", "uri": "..."}
    ],
    "nondeterminism": [
        {"field": "time", "rule": "ignore"},
        {"field": "*.span_id", "rule": "normalize"}
    ]
}

# cra trace replay --manifest golden/deploy-test.json
# Compares actual trace against expected, respecting nondeterminism rules
```

### 4.5 Conformance Test Suite

```python
# tests/conformance/
- test_carp_envelope_validation.py
- test_carp_resolution_contract.py
- test_trace_event_emission.py
- test_trace_replay_determinism.py
- test_policy_enforcement.py
```

---

## 5. Phase 2: Atlas Execution + Adapters

### 5.1 Atlas Format

```
hello-world/
├── atlas.json              # Manifest
├── context/
│   ├── overview.md         # Domain context
│   ├── api-reference.json  # API schemas
│   └── constraints.md      # Operational constraints
├── policies/
│   ├── default.policy.json # Default policy rules
│   └── high-risk.policy.json
├── adapters/
│   ├── openai.tools.json   # Pre-generated OpenAI tools
│   ├── anthropic.skill.md  # Pre-generated Claude skill
│   └── mcp.server.json     # MCP server descriptor
└── tests/
    ├── golden-traces/
    └── scenarios/
```

**atlas.json:**
```json
{
    "atlas_version": "1.0",
    "id": "com.example.hello-world",
    "version": "1.0.0",
    "name": "Hello World Atlas",
    "description": "Example Atlas for demonstration",
    "capabilities": ["greeting.send", "greeting.customize"],
    "context_packs": ["context/overview.md", "context/api-reference.json"],
    "policies": ["policies/default.policy.json"],
    "adapters": {
        "openai": "adapters/openai.tools.json",
        "anthropic": "adapters/anthropic.skill.md",
        "mcp": "adapters/mcp.server.json"
    },
    "dependencies": [],
    "license": "MIT",
    "certification": {
        "carp_compliant": true,
        "trace_compliant": true,
        "last_certified": "2025-01-01"
    }
}
```

### 5.2 Adapter Implementations

#### OpenAI Tools Adapter
```python
class OpenAIAdapter(BaseAdapter):
    def emit_tools_schema(self, atlas: Atlas, actions: List[Action]) -> dict:
        """
        Generates OpenAI function calling schema
        """
        return {
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": action.action_id,
                        "description": action.description,
                        "parameters": action.schema
                    }
                }
                for action in actions
            ]
        }
```

#### Claude/Anthropic Adapter
```python
class AnthropicAdapter(BaseAdapter):
    def emit_skill_md(self, atlas: Atlas, resolution: Resolution) -> str:
        """
        Generates SKILL.md for Claude agent consumption
        """
        return f"""
# {atlas.name}

## Context
{self._render_context_blocks(resolution.context_blocks)}

## Allowed Actions
{self._render_actions(resolution.allowed_actions)}

## Constraints
{self._render_constraints(resolution)}

## Deny Patterns
{self._render_denylist(resolution.denylist)}
"""
```

#### Google ADK Adapter
```python
class GoogleADKAdapter(BaseAdapter):
    def emit_agent_tools(self, atlas: Atlas, actions: List[Action]) -> dict:
        """
        Generates Google ADK AgentTool stubs
        """
        return {
            "tools": [
                {
                    "name": action.action_id,
                    "description": action.description,
                    "schema": action.schema,
                    "orchestration_hints": {
                        "requires_approval": action.requires_approval,
                        "timeout_ms": action.timeout_ms
                    }
                }
                for action in actions
            ]
        }
```

#### MCP Adapter
```python
class MCPAdapter(BaseAdapter):
    def emit_server_descriptor(self, atlas: Atlas) -> dict:
        """
        Generates MCP server descriptor
        """
        return {
            "name": atlas.id,
            "version": atlas.version,
            "resources": self._extract_resources(atlas),
            "tools": self._extract_tools(atlas),
            "prompts": self._extract_prompts(atlas)
        }
```

---

## 6. Phase 3: Marketplace + Licensing

### 6.1 Registry API

```python
# POST /v1/registry/publish
# PUT /v1/registry/{atlas_id}/versions/{version}
# GET /v1/registry/search?q=...&capability=...
# GET /v1/registry/{atlas_id}
# GET /v1/registry/{atlas_id}/versions
```

### 6.2 Licensing Models

```python
class LicenseType(Enum):
    FREE = "free"
    PAID_ONCE = "paid_once"
    SUBSCRIPTION = "subscription"
    USAGE_METERED = "usage_metered"

# Usage metering backed by TRACE spans
# Each trace.action.completed event contributes to usage count
# Signed tokens for offline verification
```

### 6.3 Certification Pipeline

1. **Automated checks:**
   - CARP schema compliance
   - TRACE emission completeness
   - Policy rule validation
   - Golden trace replay pass

2. **Optional security review:**
   - Manual review for high-trust certification
   - Penetration testing for sensitive domains

---

## 7. Phase 4: Infrastructure Scale

### 7.1 Agent SDK Middleware

```python
# Pipe CRA resolution into any agent framework
from cra.middleware import CRAMiddleware

# OpenAI example
middleware = CRAMiddleware(runtime_url="http://localhost:8420")
tools = middleware.resolve_and_inject(
    goal="Process customer refund",
    platform="openai"
)
response = openai.chat.completions.create(
    model="gpt-4",
    messages=[...],
    tools=tools  # CRA-resolved tools
)
```

### 7.2 Governance Features

- RBAC: Role-based access to Atlases and capabilities
- Org policies: Enterprise-wide constraints
- Audit exports: Bulk TRACE export for compliance
- SIEM integration: Forward TRACE events to security systems

### 7.3 Observability Bridge

```python
# OpenTelemetry export (TRACE remains canonical)
class OTelExporter:
    def export_trace(self, trace_id: str) -> None:
        """
        Export CRA TRACE events as OpenTelemetry spans.
        Note: TRACE is canonical; OTel is derivative.
        """
        pass
```

---

## 8. Schema Specifications

### 8.1 CARP Envelope (Full)

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "CARP Envelope",
    "type": "object",
    "required": ["carp_version", "type", "id", "time", "session", "payload", "trace"],
    "properties": {
        "carp_version": {"const": "1.0"},
        "type": {"enum": ["carp.request", "carp.response"]},
        "id": {"type": "string", "format": "uuid"},
        "time": {"type": "string", "format": "date-time"},
        "session": {
            "type": "object",
            "required": ["session_id", "principal", "scopes"],
            "properties": {
                "session_id": {"type": "string", "format": "uuid"},
                "principal": {
                    "type": "object",
                    "required": ["type", "id"],
                    "properties": {
                        "type": {"enum": ["user", "service", "agent"]},
                        "id": {"type": "string"}
                    }
                },
                "scopes": {"type": "array", "items": {"type": "string"}},
                "expires_at": {"type": "string", "format": "date-time"}
            }
        },
        "atlas": {
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "version": {"type": "string"},
                "capability": {"type": "string"}
            }
        },
        "payload": {"type": "object"},
        "trace": {
            "type": "object",
            "required": ["trace_id", "span_id"],
            "properties": {
                "trace_id": {"type": "string", "format": "uuid"},
                "span_id": {"type": "string", "format": "uuid"},
                "parent_span_id": {"type": ["string", "null"], "format": "uuid"}
            }
        }
    }
}
```

### 8.2 TRACE Event (Full)

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "TRACE Event",
    "type": "object",
    "required": ["trace_version", "event_type", "time", "trace", "session_id", "actor", "severity"],
    "properties": {
        "trace_version": {"const": "1.0"},
        "event_type": {"type": "string", "pattern": "^trace\\.[a-z]+\\.[a-z]+"},
        "time": {"type": "string", "format": "date-time"},
        "trace": {
            "type": "object",
            "required": ["trace_id", "span_id"],
            "properties": {
                "trace_id": {"type": "string", "format": "uuid"},
                "span_id": {"type": "string", "format": "uuid"},
                "parent_span_id": {"type": ["string", "null"]}
            }
        },
        "session_id": {"type": "string", "format": "uuid"},
        "atlas": {
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "version": {"type": "string"}
            }
        },
        "actor": {
            "type": "object",
            "required": ["type", "id"],
            "properties": {
                "type": {"enum": ["runtime", "agent", "user", "tool"]},
                "id": {"type": "string"}
            }
        },
        "severity": {"enum": ["debug", "info", "warn", "error"]},
        "payload": {"type": "object"},
        "artifacts": {
            "type": "array",
            "items": {
                "type": "object",
                "required": ["name", "uri", "sha256", "content_type"],
                "properties": {
                    "name": {"type": "string"},
                    "uri": {"type": "string"},
                    "sha256": {"type": "string"},
                    "content_type": {"type": "string"}
                }
            }
        }
    }
}
```

---

## 9. Testing Strategy

### 9.1 Testing Pyramid

```
                    ┌─────────────────┐
                    │   E2E Tests     │  ← Full CLI → Runtime flows
                    │   (Few, slow)   │
                    └────────┬────────┘
                             │
               ┌─────────────┴─────────────┐
               │    Integration Tests      │  ← API endpoint tests
               │    (Some, medium speed)   │
               └─────────────┬─────────────┘
                             │
        ┌────────────────────┴────────────────────┐
        │           Unit Tests                     │  ← Core logic, models
        │           (Many, fast)                   │
        └──────────────────────────────────────────┘
```

### 9.2 Conformance Testing

Every PR must pass:
1. CARP envelope schema validation
2. TRACE event schema validation
3. All mandatory event types emitted for each operation
4. Golden trace replay (no regressions)

### 9.3 Property-Based Testing

Use Hypothesis for:
- CARP request fuzzing
- Policy rule edge cases
- Trace replay determinism

---

## 10. Open Questions

### 10.1 Needs Decision

1. **Storage backend for production traces**
   - PostgreSQL with JSONB?
   - ClickHouse for analytics?
   - S3 + index in Postgres?

2. **Authentication mechanism**
   - JWT tokens?
   - API keys?
   - OAuth2/OIDC integration?

3. **Multi-tenancy model**
   - Single-tenant deployments?
   - Multi-tenant SaaS?
   - Hybrid?

4. **Atlas distribution**
   - OCI registry (Docker-like)?
   - Custom registry?
   - Git-based?

### 10.2 Future Considerations

1. **WebSocket support** for bidirectional streaming
2. **GraphQL API** for complex queries
3. **Distributed tracing** across multiple CRA instances
4. **Caching layer** for frequently-resolved contexts
5. **Plugin system** for custom policy hooks

---

## Implementation Priority

### Phase 0 Milestones (Current Focus)

| Milestone | Deliverable | Priority |
|-----------|-------------|----------|
| M0.1 | Project scaffolding + core models | P0 |
| M0.2 | Trace storage + emission | P0 |
| M0.3 | Health + Session endpoints | P0 |
| M0.4 | CARP resolve endpoint | P0 |
| M0.5 | Trace query/stream endpoints | P0 |
| M0.6 | CLI: doctor, init | P0 |
| M0.7 | CLI: resolve, trace tail | P0 |
| M0.8 | Integration tests | P0 |
| M0.9 | Documentation | P1 |

---

*This is a living document. Update as decisions are made.*
