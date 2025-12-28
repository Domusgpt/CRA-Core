# Atlas Development Guide

Complete guide to creating, publishing, and maintaining CRA Atlases.

---

## Table of Contents

- [What is an Atlas?](#what-is-an-atlas)
- [Atlas Structure](#atlas-structure)
- [Creating an Atlas](#creating-an-atlas)
- [Manifest Reference](#manifest-reference)
- [Context Packs](#context-packs)
- [Policies](#policies)
- [Adapters](#adapters)
- [Testing](#testing)
- [Certification](#certification)
- [Best Practices](#best-practices)

---

## What is an Atlas?

An **Atlas** is a versioned package that contains everything an AI agent needs to correctly interact with a specific domain:

- **Context** — Domain knowledge, API references, operational constraints
- **Policies** — Rules governing what agents can and cannot do
- **Adapters** — Platform-specific tool definitions
- **Tests** — Golden traces and validation scenarios

Atlases enable:
1. **Portability** — One Atlas works across all supported platforms
2. **Governance** — Policies are enforced, not suggested
3. **Auditability** — Every action is traced
4. **Versioning** — Changes are tracked and reproducible

---

## Atlas Structure

```
my-atlas/
├── atlas.json              # Manifest (required)
├── context/                # Context packs
│   ├── overview.md         # Domain overview
│   ├── api-reference.json  # API schemas
│   └── constraints.md      # Operational constraints
├── policies/               # Policy rules
│   ├── default.policy.json # Default policy
│   └── production.policy.json
├── adapters/               # Pre-generated adapters
│   ├── openai.tools.json
│   ├── anthropic.skill.md
│   └── mcp.server.json
└── tests/                  # Test suite
    ├── golden-traces/      # Expected traces
    └── scenarios/          # Test scenarios
```

---

## Creating an Atlas

### Step 1: Create the Directory Structure

```bash
mkdir -p my-atlas/{context,policies,adapters,tests/golden-traces}
cd my-atlas
```

### Step 2: Create the Manifest

Create `atlas.json`:

```json
{
  "atlas_version": "1.0",
  "id": "com.mycompany.my-atlas",
  "version": "1.0.0",
  "name": "My Atlas",
  "description": "Description of what this Atlas enables",
  "capabilities": [
    "resource.create",
    "resource.read",
    "resource.update"
  ],
  "context_packs": [
    "context/overview.md",
    "context/api-reference.json"
  ],
  "policies": [
    "policies/default.policy.json"
  ],
  "adapters": {
    "openai": "adapters/openai.tools.json",
    "anthropic": "adapters/anthropic.skill.md",
    "mcp": "adapters/mcp.server.json"
  },
  "dependencies": [],
  "license": "MIT",
  "certification": {
    "carp_compliant": false,
    "trace_compliant": false
  }
}
```

### Step 3: Add Context

Create `context/overview.md`:

```markdown
# My Atlas

## Purpose

This Atlas provides tools for managing resources.

## Available Actions

- **resource.create** — Create a new resource
- **resource.read** — Read resource details
- **resource.update** — Update an existing resource

## Constraints

1. Resources must have unique identifiers
2. Updates require the resource to exist
3. All operations are logged
```

Create `context/api-reference.json`:

```json
{
  "version": "1.0",
  "endpoints": {
    "resource.create": {
      "method": "POST",
      "path": "/api/resources",
      "parameters": {
        "name": {
          "type": "string",
          "required": true,
          "description": "Resource name"
        },
        "type": {
          "type": "string",
          "enum": ["typeA", "typeB"],
          "required": true
        }
      }
    }
  }
}
```

### Step 4: Define Policies

Create `policies/default.policy.json`:

```json
{
  "policy_version": "1.0",
  "id": "my-atlas-default",
  "name": "Default Policy",
  "rules": [
    {
      "id": "require-name",
      "type": "validation",
      "description": "Name is required for creation",
      "action": "resource.create",
      "required_fields": ["name", "type"]
    },
    {
      "id": "rate-limit",
      "type": "rate_limit",
      "description": "Limit creation rate",
      "action": "resource.create",
      "limit": 10,
      "window_seconds": 60
    }
  ]
}
```

### Step 5: Load and Test

```bash
cra atlas load ./my-atlas
cra atlas info com.mycompany.my-atlas
```

---

## Manifest Reference

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `atlas_version` | string | Atlas format version (`"1.0"`) |
| `id` | string | Unique identifier (reverse domain) |
| `version` | string | Semantic version |
| `name` | string | Human-readable name |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Detailed description |
| `capabilities` | array | List of capability strings |
| `context_packs` | array | Paths to context files |
| `policies` | array | Paths to policy files |
| `adapters` | object | Platform → adapter path mapping |
| `dependencies` | array | Required Atlas IDs |
| `license` | string | License identifier |
| `certification` | object | Certification status |

### ID Format

Atlas IDs should follow reverse domain notation:

```
com.company.product-name
org.opensource.tool-name
```

Pattern: `^[a-z][a-z0-9._-]*$`

### Version Format

Versions must follow semantic versioning:

```
1.0.0
1.2.3-beta.1
2.0.0-rc.1
```

---

## Context Packs

Context packs provide domain knowledge to agents.

### Supported Formats

| Format | Extension | Use Case |
|--------|-----------|----------|
| Markdown | `.md` | Prose documentation |
| JSON | `.json` | Structured data, schemas |
| YAML | `.yaml` | Configuration |
| Text | `.txt` | Plain text |

### Context Block Schema

When loaded, context packs become `ContextBlock` objects:

```json
{
  "block_id": "overview",
  "purpose": "Domain overview and guidelines",
  "content_type": "text",
  "content": "...",
  "ttl_seconds": 3600,
  "priority": 1
}
```

### Best Practices

1. **Be concise** — Agents have context limits
2. **Be specific** — Vague instructions lead to hallucination
3. **Use examples** — Show correct usage patterns
4. **Update TTLs** — Set appropriate cache durations

---

## Policies

Policies define rules that govern agent behavior.

### Policy File Schema

```json
{
  "policy_version": "1.0",
  "id": "unique-policy-id",
  "name": "Human-Readable Name",
  "description": "What this policy does",
  "rules": [...]
}
```

### Rule Types

#### Validation Rules

Ensure required fields are present:

```json
{
  "id": "require-customer-id",
  "type": "validation",
  "action": "ticket.create",
  "required_fields": ["customer_id", "subject"]
}
```

#### Constraint Rules

Enforce value constraints:

```json
{
  "id": "max-refund",
  "type": "constraint",
  "action": "refund.request",
  "constraint": {
    "field": "amount",
    "operator": "lte",
    "value": 1000
  },
  "error": "Refunds over $1000 require manual processing"
}
```

#### Approval Rules

Require human approval:

```json
{
  "id": "production-approval",
  "type": "approval",
  "action": "deploy.production",
  "approvers": ["platform-team", "service-owner"],
  "min_approvers": 1
}
```

#### Rate Limit Rules

Prevent abuse:

```json
{
  "id": "query-rate-limit",
  "type": "rate_limit",
  "action": "query.run",
  "limit": 100,
  "window_seconds": 3600
}
```

#### Deny Rules

Block patterns entirely:

```json
{
  "id": "deny-production",
  "type": "deny",
  "pattern": "*.production.*",
  "reason": "Production access not permitted"
}
```

#### Redaction Rules

Protect sensitive data:

```json
{
  "id": "redact-pii",
  "type": "redaction",
  "patterns": ["email", "phone", "ssn"],
  "redaction_type": "mask"
}
```

#### Scope Rules

Require specific scopes:

```json
{
  "id": "require-admin",
  "type": "scope",
  "action": "user.delete",
  "required_scopes": ["admin:users"]
}
```

---

## Adapters

Adapters translate Atlas capabilities to platform-specific formats.

### OpenAI Adapter

`adapters/openai.tools.json`:

```json
{
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "resource_create",
        "description": "Create a new resource",
        "parameters": {
          "type": "object",
          "properties": {
            "name": {
              "type": "string",
              "description": "Resource name"
            },
            "type": {
              "type": "string",
              "enum": ["typeA", "typeB"]
            }
          },
          "required": ["name", "type"]
        }
      }
    }
  ]
}
```

### Anthropic Adapter

`adapters/anthropic.skill.md`:

```markdown
# My Atlas

## Context

This Atlas manages resources in the system.

## Available Tools

### resource_create

Create a new resource.

**Parameters:**
- `name` (string, required): Resource name
- `type` (string, required): Resource type (typeA or typeB)

## Constraints

- All resources must have unique names
- Rate limit: 10 creations per minute
```

### MCP Adapter

`adapters/mcp.server.json`:

```json
{
  "name": "com.mycompany.my-atlas",
  "version": "1.0.0",
  "description": "My Atlas MCP Server",
  "tools": [
    {
      "name": "resource_create",
      "description": "Create a new resource",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": {"type": "string"},
          "type": {"type": "string", "enum": ["typeA", "typeB"]}
        },
        "required": ["name", "type"]
      }
    }
  ],
  "resources": [],
  "prompts": []
}
```

### Auto-Generation

Adapters can be auto-generated:

```bash
cra atlas emit com.mycompany.my-atlas -p openai -o adapters/openai.tools.json
cra atlas emit com.mycompany.my-atlas -p anthropic -o adapters/anthropic.skill.md
cra atlas emit com.mycompany.my-atlas -p mcp -o adapters/mcp.server.json
```

---

## Testing

### Golden Traces

Golden traces are expected TRACE outputs for specific scenarios.

Create `tests/golden-traces/create-resource.json`:

```json
{
  "manifest_version": "1.0",
  "name": "Create Resource",
  "description": "Test resource creation flow",
  "input": {
    "goal": "Create a new resource",
    "action_id": "resource.create",
    "parameters": {
      "name": "test-resource",
      "type": "typeA"
    }
  },
  "expected_events": [
    {"event_type": "trace.carp.resolve.requested"},
    {"event_type": "trace.carp.resolve.returned"},
    {"event_type": "trace.action.invoked"},
    {"event_type": "trace.action.completed"}
  ],
  "nondeterminism": [
    {"field": "time", "rule": "ignore"},
    {"field": "*.span_id", "rule": "normalize"},
    {"field": "*.trace_id", "rule": "normalize"}
  ]
}
```

### Running Tests

```bash
cra replay --manifest tests/golden-traces/create-resource.json --compare
```

### Test Scenarios

Create `tests/scenarios/` with test scripts:

```python
# tests/scenarios/test_create_resource.py
import pytest
from cra.middleware import CRAMiddleware

def test_resource_creation():
    middleware = CRAMiddleware()

    resolution = middleware.resolve(
        goal="Create a resource",
        atlas_id="com.mycompany.my-atlas",
        capability="resource.create",
    )

    assert resolution.confidence > 0.9
    assert any(a["action_id"] == "resource.create" for a in resolution.allowed_actions)

    result = middleware.execute(
        action_id="resource.create",
        parameters={"name": "test", "type": "typeA"},
    )

    assert result["status"] == "completed"
```

---

## Certification

Atlases can be certified for CARP and TRACE compliance.

### Requirements

#### CARP Compliance

- [ ] Valid `atlas.json` manifest
- [ ] All capabilities have corresponding actions
- [ ] Policies are valid and enforceable
- [ ] Context packs are properly formatted

#### TRACE Compliance

- [ ] All actions emit appropriate events
- [ ] Golden traces pass replay
- [ ] No missing event types
- [ ] Proper span relationships

### Certification Process

1. Run validation:
   ```bash
   cra atlas validate com.mycompany.my-atlas
   ```

2. Run conformance tests:
   ```bash
   pytest tests/conformance/
   ```

3. Update manifest:
   ```json
   "certification": {
     "carp_compliant": true,
     "trace_compliant": true,
     "last_certified": "2025-01-15"
   }
   ```

---

## Best Practices

### Naming Conventions

- **Atlas ID:** `com.company.product-domain`
- **Capabilities:** `noun.verb` (e.g., `ticket.create`)
- **Policy IDs:** `descriptive-kebab-case`

### Versioning

- Use semantic versioning
- Document breaking changes
- Maintain backwards compatibility when possible

### Security

- Never include secrets in Atlases
- Use redaction policies for PII
- Require approval for high-risk actions
- Implement rate limits

### Documentation

- Keep context packs up to date
- Document all capabilities
- Provide examples for complex actions
- Include troubleshooting guidance

### Testing

- Create golden traces for happy paths
- Test edge cases and error conditions
- Validate policy enforcement
- Test across all target platforms

---

## Example Atlases

See the `examples/atlases/` directory:

| Atlas | Description |
|-------|-------------|
| `hello-world` | Minimal example |
| `customer-support` | Ticket management, KB |
| `devops` | Deployments, infrastructure |
| `data-analytics` | Queries, reports, exports |

---

*For more information, see the [API Reference](API.md) and [CLI Reference](CLI.md).*
