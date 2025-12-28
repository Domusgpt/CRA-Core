# CRA Runtime API Reference

Complete REST API documentation for the CRA Runtime.

**Base URL:** `http://localhost:8420` (default)
**API Version:** `v1`
**Content-Type:** `application/json`

---

## Table of Contents

- [Authentication](#authentication)
- [Health](#health)
- [Sessions](#sessions)
- [CARP Resolution](#carp-resolution)
- [Action Execution](#action-execution)
- [Traces](#traces)
- [Atlases](#atlases)
- [Error Handling](#error-handling)

---

## Authentication

The API supports two authentication methods:

### JWT Bearer Token

```http
Authorization: Bearer <jwt_token>
```

### API Key

```http
X-API-Key: <api_key>
```

### Unauthenticated Endpoints

The following endpoints do not require authentication:
- `GET /v1/health`
- `GET /docs`
- `GET /redoc`
- `GET /openapi.json`

---

## Health

### GET /v1/health

Check runtime health and status.

**Response**

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "carp_version": "1.0",
  "trace_version": "1.0",
  "uptime_seconds": 3600.5
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | `healthy` or `unhealthy` |
| `version` | string | Runtime version |
| `carp_version` | string | CARP protocol version |
| `trace_version` | string | TRACE protocol version |
| `uptime_seconds` | float | Server uptime |

---

## Sessions

### POST /v1/sessions

Create a new session.

**Request**

```json
{
  "principal": {
    "type": "agent",
    "id": "my-agent-001"
  },
  "scopes": ["ticket.create", "ticket.read"],
  "ttl_seconds": 3600
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `principal.type` | string | Yes | `user`, `service`, or `agent` |
| `principal.id` | string | Yes | Principal identifier |
| `scopes` | array | No | Requested scopes |
| `ttl_seconds` | integer | No | Session TTL (default: 3600) |

**Response** `201 Created`

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "expires_at": "2025-01-15T11:00:00Z",
  "principal": {
    "type": "agent",
    "id": "my-agent-001"
  },
  "scopes": ["ticket.create", "ticket.read"]
}
```

**TRACE Events Emitted:** `trace.session.started`

---

### GET /v1/sessions/{session_id}

Get session details.

**Response** `200 OK`

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "principal": {
    "type": "agent",
    "id": "my-agent-001"
  },
  "scopes": ["ticket.create", "ticket.read"],
  "created_at": "2025-01-15T10:00:00Z",
  "expires_at": "2025-01-15T11:00:00Z",
  "ended_at": null
}
```

---

### POST /v1/sessions/{session_id}/end

End a session.

**Response** `200 OK`

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "ended_at": "2025-01-15T10:30:00Z",
  "trace_summary": {
    "total_events": 42,
    "resolutions": 5,
    "actions_executed": 12,
    "actions_denied": 2
  }
}
```

**TRACE Events Emitted:** `trace.session.ended`

---

## CARP Resolution

### POST /v1/carp/resolve

Resolve context and permissions for a goal.

**Request**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "goal": "Create a support ticket for customer complaint",
  "atlas_id": "com.example.customer-support",
  "capability": "ticket.create",
  "risk_tier": "medium",
  "context": {
    "customer_id": "CUST-123",
    "priority": "high"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `session_id` | UUID | Yes | Active session ID |
| `goal` | string | Yes | Description of the goal |
| `atlas_id` | string | No | Specific Atlas to use |
| `capability` | string | No | Filter by capability |
| `risk_tier` | string | No | `low`, `medium`, `high`, `critical` |
| `context` | object | No | Additional context data |

**Response** `200 OK`

```json
{
  "carp_version": "1.0",
  "type": "carp.response",
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "time": "2025-01-15T10:00:00Z",
  "resolution": {
    "resolution_id": "789e0123-e89b-12d3-a456-426614174000",
    "confidence": 0.95,
    "context_blocks": [
      {
        "block_id": "ticket-policies",
        "purpose": "Ticket creation guidelines",
        "content_type": "text",
        "content": "...",
        "ttl_seconds": 3600
      }
    ],
    "allowed_actions": [
      {
        "action_id": "ticket.create",
        "kind": "write",
        "description": "Create a new support ticket",
        "adapter": "internal",
        "requires_approval": false,
        "timeout_ms": 30000,
        "schema": {
          "type": "object",
          "properties": {
            "customer_id": {"type": "string"},
            "subject": {"type": "string"},
            "description": {"type": "string"},
            "priority": {"type": "string", "enum": ["low", "medium", "high"]}
          },
          "required": ["customer_id", "subject", "description"]
        },
        "constraints": []
      }
    ],
    "denylist": [
      {
        "pattern": "ticket.delete",
        "reason": "Deletion not permitted for this session"
      }
    ],
    "merge_rules": {
      "context_priority": ["atlas", "session", "request"],
      "action_intersection": true
    },
    "next_steps": [
      {
        "step": "Gather customer details",
        "expected_artifacts": ["customer_info"]
      }
    ]
  },
  "trace": {
    "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "span_id": "abc12345-1234-5678-90ab-cdef12345678"
  }
}
```

**TRACE Events Emitted:** `trace.carp.resolve.requested`, `trace.carp.resolve.returned`

---

## Action Execution

### POST /v1/carp/execute

Execute a granted action.

**Request**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "resolution_id": "789e0123-e89b-12d3-a456-426614174000",
  "action_id": "ticket.create",
  "parameters": {
    "customer_id": "CUST-123",
    "subject": "Order not delivered",
    "description": "Customer reports order #ORD-456 has not arrived",
    "priority": "high"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `session_id` | UUID | Yes | Active session ID |
| `resolution_id` | UUID | Yes | Resolution from prior resolve |
| `action_id` | string | Yes | Action to execute |
| `parameters` | object | No | Action parameters |

**Response** `200 OK`

```json
{
  "execution_id": "def45678-e89b-12d3-a456-426614174000",
  "status": "completed",
  "action_id": "ticket.create",
  "result": {
    "ticket_id": "TKT-789",
    "created_at": "2025-01-15T10:00:01Z",
    "status": "open"
  },
  "trace": {
    "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "span_id": "xyz98765-1234-5678-90ab-cdef12345678"
  }
}
```

**Status Values:**
- `pending` — Awaiting approval
- `running` — Currently executing
- `completed` — Successfully completed
- `failed` — Execution failed
- `denied` — Action not permitted

**TRACE Events Emitted:** `trace.action.invoked`, `trace.action.completed` or `trace.action.failed`

---

### Error Response (Action Denied)

```json
{
  "execution_id": "def45678-e89b-12d3-a456-426614174000",
  "status": "denied",
  "action_id": "ticket.delete",
  "error": "Action not in allowed_actions for this resolution",
  "trace": {
    "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "span_id": "xyz98765-1234-5678-90ab-cdef12345678"
  }
}
```

**TRACE Events Emitted:** `trace.action.denied`

---

## Traces

### GET /v1/traces/{trace_id}/events

Query trace events.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `event_type` | string | Filter by event type prefix (e.g., `trace.action`) |
| `severity` | string | Filter by severity (`debug`, `info`, `warn`, `error`) |
| `limit` | integer | Max events to return (default: 100) |
| `offset` | integer | Pagination offset |

**Response** `200 OK`

```json
{
  "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "events": [
    {
      "trace_version": "1.0",
      "event_type": "trace.session.started",
      "time": "2025-01-15T10:00:00Z",
      "trace": {
        "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "span_id": "111e2222-3333-4444-5555-666677778888"
      },
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "actor": {
        "type": "runtime",
        "id": "cra-runtime"
      },
      "severity": "info",
      "payload": {
        "principal": {"type": "agent", "id": "my-agent-001"},
        "scopes": ["ticket.create"]
      }
    }
  ],
  "total_count": 42,
  "has_more": true
}
```

---

### GET /v1/traces/{trace_id}/stream

Stream trace events via Server-Sent Events (SSE).

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `event_type` | string | Filter by event type prefix |
| `severity` | string | Filter by severity |

**Response** `200 OK` (text/event-stream)

```
data: {"trace_version":"1.0","event_type":"trace.session.started",...}

data: {"trace_version":"1.0","event_type":"trace.carp.resolve.requested",...}

data: {"trace_version":"1.0","event_type":"trace.action.completed",...}
```

---

## Atlases

### GET /v1/atlases

List all registered Atlases.

**Response** `200 OK`

```json
{
  "atlases": [
    {
      "id": "com.example.customer-support",
      "version": "1.0.0",
      "name": "Customer Support Atlas",
      "description": "Tools for customer support operations",
      "capabilities": ["ticket.create", "ticket.update", "ticket.resolve"],
      "adapters": ["openai", "anthropic", "mcp"],
      "certification": {
        "carp_compliant": true,
        "trace_compliant": true
      }
    }
  ],
  "count": 1
}
```

---

### POST /v1/atlases/load

Load an Atlas from a path.

**Request**

```json
{
  "path": "/path/to/atlas"
}
```

**Response** `200 OK`

```json
{
  "success": true,
  "atlas": {
    "id": "com.example.customer-support",
    "version": "1.0.0",
    "name": "Customer Support Atlas",
    "capabilities": ["ticket.create", "ticket.update"],
    "adapters": ["openai", "anthropic"]
  }
}
```

---

### GET /v1/atlases/{atlas_id}

Get Atlas details.

**Response** `200 OK`

```json
{
  "id": "com.example.customer-support",
  "version": "1.0.0",
  "name": "Customer Support Atlas",
  "description": "Complete toolkit for customer support operations",
  "capabilities": ["ticket.create", "ticket.update", "ticket.resolve", "kb.search"],
  "context_packs": 3,
  "policies": 2,
  "adapters": ["openai", "anthropic", "mcp"],
  "certification": {
    "carp_compliant": true,
    "trace_compliant": true,
    "last_certified": "2025-01-01"
  }
}
```

---

### DELETE /v1/atlases/{atlas_id}

Unload an Atlas.

**Response** `200 OK`

```json
{
  "success": true,
  "atlas_id": "com.example.customer-support"
}
```

---

### GET /v1/atlases/{atlas_id}/emit/{platform}

Emit Atlas in platform-specific format.

**Path Parameters**

| Parameter | Description |
|-----------|-------------|
| `atlas_id` | Atlas identifier |
| `platform` | Target platform: `openai`, `anthropic`, `google_adk`, `mcp` |

**Response** `200 OK` (OpenAI example)

```json
{
  "atlas_id": "com.example.customer-support",
  "platform": "openai",
  "output": {
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "ticket_create",
          "description": "Create a new support ticket",
          "parameters": {
            "type": "object",
            "properties": {
              "customer_id": {"type": "string"},
              "subject": {"type": "string"},
              "description": {"type": "string"}
            },
            "required": ["customer_id", "subject", "description"]
          }
        }
      }
    ]
  }
}
```

---

### GET /v1/atlases/{atlas_id}/context

Get context blocks from an Atlas.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `capability` | string | Filter by capability |

**Response** `200 OK`

```json
{
  "atlas_id": "com.example.customer-support",
  "blocks": [
    {
      "block_id": "ticket-guidelines",
      "purpose": "Guidelines for ticket creation",
      "content_type": "text",
      "ttl_seconds": 3600
    }
  ],
  "count": 1
}
```

---

### GET /v1/atlases/{atlas_id}/actions

Get allowed actions from an Atlas.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `capability` | string | Filter by capability |

**Response** `200 OK`

```json
{
  "atlas_id": "com.example.customer-support",
  "actions": [
    {
      "action_id": "ticket.create",
      "kind": "write",
      "adapter": "internal",
      "description": "Create a new support ticket"
    }
  ],
  "count": 1
}
```

---

## Error Handling

### Error Response Format

```json
{
  "detail": "Error message describing what went wrong",
  "status_code": 400,
  "error_type": "validation_error"
}
```

### HTTP Status Codes

| Code | Description |
|------|-------------|
| `200` | Success |
| `201` | Created |
| `400` | Bad Request — Invalid input |
| `401` | Unauthorized — Authentication required |
| `403` | Forbidden — Insufficient permissions |
| `404` | Not Found — Resource doesn't exist |
| `409` | Conflict — Resource already exists |
| `422` | Unprocessable Entity — Validation failed |
| `429` | Too Many Requests — Rate limit exceeded |
| `500` | Internal Server Error |

### Common Errors

**Session Expired**
```json
{
  "detail": "Session has expired",
  "status_code": 401,
  "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Resolution Not Found**
```json
{
  "detail": "Resolution not found or expired",
  "status_code": 404,
  "resolution_id": "789e0123-e89b-12d3-a456-426614174000"
}
```

**Action Not Permitted**
```json
{
  "detail": "Action 'ticket.delete' not in allowed_actions",
  "status_code": 403,
  "action_id": "ticket.delete"
}
```

**Policy Violation**
```json
{
  "detail": "Action denied by policy: Rate limit exceeded",
  "status_code": 429,
  "policy_id": "rate-limit-tickets",
  "retry_after_seconds": 60
}
```

---

## Rate Limiting

Default rate limits:

| Endpoint | Limit |
|----------|-------|
| `/v1/sessions` | 100/minute |
| `/v1/carp/resolve` | 1000/minute |
| `/v1/carp/execute` | 500/minute |
| `/v1/traces/*` | 100/minute |

Rate limit headers:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1705312800
```

---

## OpenAPI Specification

The full OpenAPI specification is available at:

- **Swagger UI:** `GET /docs`
- **ReDoc:** `GET /redoc`
- **OpenAPI JSON:** `GET /openapi.json`

---

*For usage examples, see the [Integration Guide](INTEGRATION.md).*
