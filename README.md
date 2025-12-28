# CRA — Context Registry Agents

[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CARP 1.0](https://img.shields.io/badge/CARP-1.0-green.svg)](docs/CARP.md)
[![TRACE 1.0](https://img.shields.io/badge/TRACE-1.0-green.svg)](docs/TRACE.md)

**A governed context layer that makes AI agents use tools, systems, and proprietary knowledge correctly.**

CRA provides runtime authority over AI agent operations through two core protocols:
- **CARP** (Context & Action Resolution Protocol) — resolves what context and actions are permitted
- **TRACE** (Telemetry & Replay Contract) — proves what actually happened

> **Core Principle:** *If it wasn't emitted by the runtime, it didn't happen.*

---

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Core Concepts](#core-concepts)
- [Usage Examples](#usage-examples)
- [Documentation](#documentation)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [License](#license)

---

## Features

### 🔒 Governance & Compliance
- **Policy Engine** — Scope validation, deny patterns, approval gates, rate limiting
- **RBAC** — Role-based access control with built-in roles (admin, developer, agent, auditor)
- **Audit Trails** — Immutable TRACE records for every operation
- **Replay & Regression** — Deterministic replay for testing and compliance

### 🔌 Multi-Platform Adapters
- **OpenAI** — Function calling / tools format
- **Anthropic** — Claude SKILL.md format
- **Google ADK** — AgentTool definitions
- **MCP** — Model Context Protocol server descriptors

### 🏗️ Agent Framework Integration
- **LangChain/LangGraph** — Native tool integration
- **CrewAI** — Multi-agent crew support
- **OpenAI SDK** — Direct function calling

### 📦 Production Ready
- **JWT & API Key Auth** — Flexible authentication
- **PostgreSQL Storage** — Durable trace storage with streaming
- **OpenTelemetry Export** — Integration with existing observability
- **SIEM Export** — CEF, LEEF, JSON, Syslog formats

---

## Quick Start

### 1. Install CRA

```bash
pip install cra-core

# With optional dependencies
pip install cra-core[postgres,otel]      # Production features
pip install cra-core[langchain]          # LangChain integration
pip install cra-core[all]                # Everything
```

### 2. Initialize a Project

```bash
cra init
```

This creates:
- `cra.config.json` — Runtime configuration
- `agents.md` — Agent behavior contract
- `cra.trace/` — Local trace storage

### 3. Start the Runtime

```bash
cra doctor     # Verify setup
cra runtime    # Start the server (default: http://localhost:8420)
```

### 4. Resolve and Execute

```bash
# Resolve context for a goal
cra resolve --goal "Deploy service to staging" --atlas com.example.devops

# Stream trace events
cra trace tail --follow
```

---

## Installation

### Requirements

- Python 3.11 or higher
- pip or Poetry

### From PyPI

```bash
pip install cra-core
```

### From Source

```bash
git clone https://github.com/your-org/CRA-Core.git
cd CRA-Core
pip install -e .
```

### Optional Dependencies

| Extra | Description | Install |
|-------|-------------|---------|
| `postgres` | PostgreSQL trace storage | `pip install cra-core[postgres]` |
| `otel` | OpenTelemetry export | `pip install cra-core[otel]` |
| `langchain` | LangChain/LangGraph integration | `pip install cra-core[langchain]` |
| `crewai` | CrewAI integration | `pip install cra-core[crewai]` |
| `all` | All optional dependencies | `pip install cra-core[all]` |

---

## Core Concepts

### CARP — Context & Action Resolution Protocol

CARP is the contract between agents and the CRA runtime. It determines:

1. **What context is allowed** — Minimal, relevant context blocks
2. **What actions are permitted** — Explicit allow-list with constraints
3. **What is denied** — Pattern-based deny rules with reasons
4. **What evidence is required** — Approval gates and audit requirements

```python
from cra.middleware import CRAMiddleware

middleware = CRAMiddleware(runtime_url="http://localhost:8420")

# Resolve context and permissions
resolution = middleware.resolve(
    goal="Process customer refund",
    atlas_id="com.example.customer-support",
    capability="refund.request",
)

# Check what's allowed
for action in resolution.allowed_actions:
    print(f"Allowed: {action['action_id']}")

# Execute an action
result = middleware.execute(
    action_id="refund.request",
    parameters={"order_id": "ORD-123", "amount": 50.00, "reason": "Defective item"}
)
```

### TRACE — Telemetry & Replay Contract

TRACE is the immutable record of what actually happened:

```json
{
  "trace_version": "1.0",
  "event_type": "trace.action.completed",
  "time": "2025-01-15T10:30:00Z",
  "trace": {
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "span_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  },
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "actor": {"type": "agent", "id": "customer-support-bot"},
  "severity": "info",
  "payload": {
    "action_id": "refund.request",
    "status": "completed",
    "result": {"refund_id": "REF-456"}
  }
}
```

### Atlases

Atlases are versioned packages containing domain-specific context, policies, and adapters:

```
my-atlas/
├── atlas.json           # Manifest
├── context/
│   ├── overview.md      # Domain context
│   └── api-reference.json
├── policies/
│   ├── default.policy.json
│   └── production.policy.json
└── adapters/
    ├── openai.tools.json
    └── anthropic.skill.md
```

---

## Usage Examples

### OpenAI Integration

```python
from openai import OpenAI
from cra.middleware import OpenAIMiddleware

client = OpenAI()
middleware = OpenAIMiddleware()

# Get CRA-governed tools
tools = middleware.get_tools(
    goal="Help user with data analysis",
    atlas_id="com.example.data-analytics",
)

# Use with OpenAI
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Run a sales report for Q4"}],
    tools=tools,
)

# Handle tool calls through CRA
for tool_call in response.choices[0].message.tool_calls or []:
    result = middleware.handle_tool_call(tool_call)
    print(f"Trace ID: {middleware.get_trace_id()}")
```

### LangChain Integration

```python
from cra.middleware import LangChainMiddleware

middleware = LangChainMiddleware()

# Get a ready-to-use agent executor
executor = middleware.get_runnable(
    goal="Manage customer support tickets",
    atlas_id="com.example.customer-support",
    model="gpt-4",
)

# Run the agent
result = executor.invoke({"input": "Create a ticket for order #12345"})
print(result["output"])
```

### Direct API Usage

```python
import httpx

# Create a session
response = httpx.post("http://localhost:8420/v1/sessions", json={
    "principal": {"type": "agent", "id": "my-agent"},
    "scopes": ["ticket.create", "ticket.update"],
})
session = response.json()

# Resolve context
response = httpx.post("http://localhost:8420/v1/carp/resolve", json={
    "session_id": session["session_id"],
    "goal": "Create a support ticket",
    "atlas_id": "com.example.customer-support",
})
resolution = response.json()

# Execute an action
response = httpx.post("http://localhost:8420/v1/carp/execute", json={
    "session_id": session["session_id"],
    "resolution_id": resolution["resolution"]["resolution_id"],
    "action_id": "ticket.create",
    "parameters": {
        "customer_id": "CUST-123",
        "subject": "Order issue",
        "description": "Item arrived damaged",
    },
})
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/ARCHITECTURE.md) | System design and components |
| [API Reference](docs/API.md) | Complete REST API documentation |
| [CLI Reference](docs/CLI.md) | Command-line interface guide |
| [Atlas Development](docs/ATLASES.md) | Creating and publishing Atlases |
| [Deployment Guide](docs/DEPLOYMENT.md) | Production deployment |
| [Integration Guide](docs/INTEGRATION.md) | Framework integrations |
| [CARP Specification](docs/CARP.md) | Protocol specification |
| [TRACE Specification](docs/TRACE.md) | Telemetry specification |

---

## Project Structure

```
CRA-Core/
├── cra/
│   ├── core/               # Domain models
│   │   ├── carp.py         # CARP protocol types
│   │   ├── trace.py        # TRACE event types
│   │   ├── atlas.py        # Atlas types and loader
│   │   ├── session.py      # Session management
│   │   ├── policy.py       # Policy engine
│   │   └── validation.py   # Schema validation
│   │
│   ├── runtime/            # CRA Runtime server
│   │   ├── server.py       # FastAPI application
│   │   ├── api/            # REST endpoints
│   │   ├── services/       # Business logic
│   │   └── storage/        # Storage backends
│   │
│   ├── cli/                # Command-line interface
│   │   ├── main.py         # Typer application
│   │   └── commands/       # CLI commands
│   │
│   ├── adapters/           # Platform adapters
│   │   ├── openai.py       # OpenAI tools
│   │   ├── anthropic.py    # Claude SKILL.md
│   │   ├── google_adk.py   # Google ADK
│   │   └── mcp.py          # MCP server
│   │
│   ├── templates/          # Agent template generators
│   │   ├── openai_gpt.py   # GPT Actions
│   │   ├── langchain.py    # LangChain/LangGraph
│   │   └── crewai.py       # CrewAI
│   │
│   ├── middleware/         # Framework middleware
│   │   ├── base.py         # Base middleware
│   │   ├── openai.py       # OpenAI SDK
│   │   └── langchain.py    # LangChain
│   │
│   ├── auth/               # Authentication
│   │   ├── jwt.py          # JWT tokens
│   │   ├── api_key.py      # API keys
│   │   ├── rbac.py         # Role-based access
│   │   └── middleware.py   # Auth middleware
│   │
│   ├── config/             # Configuration
│   │   └── settings.py     # Settings management
│   │
│   └── observability/      # Observability exports
│       ├── otel.py         # OpenTelemetry
│       └── siem.py         # SIEM formats
│
├── examples/
│   └── atlases/            # Example Atlases
│       ├── hello-world/
│       ├── customer-support/
│       ├── devops/
│       └── data-analytics/
│
├── tests/
│   ├── unit/
│   ├── integration/
│   └── conformance/
│
└── docs/                   # Documentation
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CRA_ENV` | Environment (development/staging/production) | `development` |
| `CRA_RUNTIME_URL` | Runtime server URL | `http://localhost:8420` |
| `CRA_JWT_SECRET` | JWT signing secret | (required in production) |
| `CRA_DATABASE_URL` | PostgreSQL connection string | `postgresql://localhost:5432/cra` |
| `CRA_API_KEY` | API key for authentication | — |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OpenTelemetry endpoint | `http://localhost:4317` |

---

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/your-org/CRA-Core.git
cd CRA-Core

# Install with development dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Run linting
ruff check cra/
mypy cra/
```

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

CRA is built on these core principles:

1. **Runtime Authority** — The runtime, not the model, is authoritative
2. **Minimal Context** — Only provide what's needed
3. **Explicit Permissions** — No implicit access
4. **Immutable Evidence** — TRACE is the source of truth

---

<p align="center">
  <strong>The context authority layer that makes agents reliable, governable, and provable.</strong>
</p>
