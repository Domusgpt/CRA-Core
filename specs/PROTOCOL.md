# CRA Protocol Specification

**Version:** 1.0.0
**Status:** Draft
**Last Updated:** 2024-12-28

---

## Abstract

This document defines the Context Registry Agents (CRA) protocol suite, consisting of:

- **CARP/1.0** — Context & Action Resolution Protocol
- **TRACE/1.0** — Telemetry & Replay Audit Contract for Execution
- **Atlas/1.0** — Agent context package format

These protocols are language-agnostic and implementation-independent. Any conforming runtime MUST implement the behaviors specified herein and pass the conformance test suite.

---

## Table of Contents

1. [Terminology](#1-terminology)
2. [Protocol Overview](#2-protocol-overview)
3. [CARP/1.0 Specification](#3-carp10-specification)
4. [TRACE/1.0 Specification](#4-trace10-specification)
5. [Atlas/1.0 Specification](#5-atlas10-specification)
6. [Wire Formats](#6-wire-formats)
7. [Conformance Requirements](#7-conformance-requirements)
8. [Security Considerations](#8-security-considerations)
9. [IANA Considerations](#9-iana-considerations)

---

## 1. Terminology

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

| Term | Definition |
|------|------------|
| **Agent** | An autonomous software entity that uses LLMs to accomplish goals |
| **Atlas** | A versioned package containing domain context, policies, and action definitions |
| **Action** | A discrete operation an agent may request to perform |
| **Resolution** | The outcome of a CARP request, specifying allowed/denied actions |
| **Trace** | An immutable, append-only log of execution events |
| **Session** | A bounded interaction context with a unique identifier |
| **Runtime** | A conforming implementation of the CRA protocols |

---

## 2. Protocol Overview

### 2.1 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CRA Runtime                          │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    CARP Engine                       │   │
│  │                                                      │   │
│  │   Request ──▶ Policy Eval ──▶ Resolution            │   │
│  │                    │                                 │   │
│  │                    ▼                                 │   │
│  │              Atlas Registry                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   TRACE Collector                    │   │
│  │                                                      │   │
│  │   Events ──▶ Hash Chain ──▶ Append-Only Log         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Protocol Relationships

```
Atlas/1.0 ────────────▶ CARP/1.0 ────────────▶ TRACE/1.0
(defines what's       (resolves what's      (records what
 available)            allowed)               happened)
```

### 2.3 Data Flow

1. Agent submits a CARP Request with a goal
2. Runtime loads relevant Atlas(es)
3. Runtime evaluates policies against the request
4. Runtime returns a CARP Resolution with allowed actions
5. Agent executes actions through the runtime
6. Runtime emits TRACE events for each operation
7. TRACE events form an immutable audit log

---

## 3. CARP/1.0 Specification

### 3.1 Overview

CARP (Context & Action Resolution Protocol) determines what context and actions are available to an agent for a given goal.

### 3.2 Request Object

A CARP Request MUST contain the following fields:

```json
{
  "carp_version": "1.0",
  "request_id": "<UUIDv7>",
  "timestamp": "<ISO 8601>",
  "operation": "resolve | execute | validate",
  "requester": {
    "agent_id": "<string>",
    "session_id": "<UUIDv7>",
    "parent_session_id": "<UUIDv7 | null>"
  },
  "task": {
    "goal": "<string>",
    "risk_tier": "low | medium | high | critical",
    "context_hints": ["<string>"],
    "required_capabilities": ["<string>"]
  },
  "atlas_ids": ["<string>"],
  "context": {}
}
```

#### 3.2.1 Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `carp_version` | string | REQUIRED | MUST be "1.0" |
| `request_id` | string | REQUIRED | UUIDv7 unique to this request |
| `timestamp` | string | REQUIRED | ISO 8601 timestamp with timezone |
| `operation` | string | REQUIRED | One of: "resolve", "execute", "validate" |
| `requester.agent_id` | string | REQUIRED | Identifier for the requesting agent |
| `requester.session_id` | string | REQUIRED | UUIDv7 for the current session |
| `requester.parent_session_id` | string | OPTIONAL | Parent session for nested agents |
| `task.goal` | string | REQUIRED | Natural language description of intent |
| `task.risk_tier` | string | OPTIONAL | Risk classification, defaults to "low" |
| `task.context_hints` | array | OPTIONAL | Hints for context selection |
| `task.required_capabilities` | array | OPTIONAL | Required capability identifiers |
| `atlas_ids` | array | OPTIONAL | Specific Atlases to query |
| `context` | object | OPTIONAL | Additional context key-value pairs |

### 3.3 Resolution Object

A CARP Resolution MUST contain the following fields:

```json
{
  "carp_version": "1.0",
  "resolution_id": "<UUIDv7>",
  "request_id": "<UUIDv7>",
  "timestamp": "<ISO 8601>",
  "decision": {
    "type": "allow | deny | partial | requires_approval",
    "reason": "<string | null>",
    "approval_id": "<string | null>",
    "expires_at": "<ISO 8601 | null>"
  },
  "context_blocks": [
    {
      "block_id": "<string>",
      "source": "<atlas_id>",
      "content_type": "text/markdown | application/json",
      "content": "<string>",
      "priority": "<integer>",
      "token_estimate": "<integer>"
    }
  ],
  "allowed_actions": [
    {
      "action_id": "<string>",
      "name": "<string>",
      "description": "<string>",
      "parameters_schema": {},
      "returns_schema": {},
      "risk_tier": "low | medium | high | critical",
      "requires_confirmation": "<boolean>",
      "rate_limit": {
        "max_calls": "<integer>",
        "window_seconds": "<integer>"
      }
    }
  ],
  "denied_actions": [
    {
      "action_id": "<string>",
      "reason": "<string>",
      "policy_id": "<string>"
    }
  ],
  "constraints": [
    {
      "constraint_id": "<string>",
      "type": "rate_limit | budget | time_window | require_approval",
      "parameters": {}
    }
  ],
  "ttl_seconds": "<integer>",
  "trace_id": "<UUIDv7>"
}
```

#### 3.3.1 Decision Types

| Type | Description |
|------|-------------|
| `allow` | Request fully approved, all requested capabilities available |
| `deny` | Request denied, no actions permitted |
| `partial` | Some actions allowed, some denied (see denied_actions) |
| `requires_approval` | Human approval required before proceeding |

### 3.4 Execute Request

When `operation` is "execute":

```json
{
  "carp_version": "1.0",
  "request_id": "<UUIDv7>",
  "timestamp": "<ISO 8601>",
  "operation": "execute",
  "requester": { ... },
  "execution": {
    "resolution_id": "<UUIDv7>",
    "action_id": "<string>",
    "parameters": {},
    "idempotency_key": "<string | null>"
  }
}
```

### 3.5 Execute Response

```json
{
  "carp_version": "1.0",
  "execution_id": "<UUIDv7>",
  "request_id": "<UUIDv7>",
  "resolution_id": "<UUIDv7>",
  "timestamp": "<ISO 8601>",
  "status": "success | error | denied | pending_approval",
  "result": {},
  "error": {
    "code": "<string>",
    "message": "<string>",
    "details": {}
  },
  "trace_id": "<UUIDv7>"
}
```

### 3.6 Validation Rules

A conforming runtime MUST enforce:

1. `request_id` MUST be unique within a session
2. `timestamp` MUST be within acceptable clock skew (default: 5 minutes)
3. `session_id` MUST exist and be active
4. `resolution_id` in execute requests MUST reference a valid, non-expired resolution
5. `action_id` in execute requests MUST be in the resolution's `allowed_actions`
6. Parameters MUST validate against the action's `parameters_schema`

---

## 4. TRACE/1.0 Specification

### 4.1 Overview

TRACE (Telemetry & Replay Audit Contract for Execution) defines an immutable, append-only event log with cryptographic integrity.

### 4.2 Event Object

Every TRACE event MUST contain:

```json
{
  "trace_version": "1.0",
  "event_id": "<UUIDv7>",
  "trace_id": "<UUIDv7>",
  "span_id": "<UUIDv7>",
  "parent_span_id": "<UUIDv7 | null>",
  "session_id": "<UUIDv7>",
  "sequence": "<integer>",
  "timestamp": "<ISO 8601>",
  "event_type": "<string>",
  "payload": {},
  "event_hash": "<string>",
  "previous_event_hash": "<string>"
}
```

#### 4.2.1 Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `trace_version` | string | REQUIRED | MUST be "1.0" |
| `event_id` | string | REQUIRED | UUIDv7 unique to this event |
| `trace_id` | string | REQUIRED | UUIDv7 grouping related events |
| `span_id` | string | REQUIRED | UUIDv7 for this operation span |
| `parent_span_id` | string | OPTIONAL | Parent span for nested operations |
| `session_id` | string | REQUIRED | Session this event belongs to |
| `sequence` | integer | REQUIRED | Monotonically increasing per session |
| `timestamp` | string | REQUIRED | ISO 8601 with microsecond precision |
| `event_type` | string | REQUIRED | Event type identifier |
| `payload` | object | REQUIRED | Event-specific data |
| `event_hash` | string | REQUIRED | SHA-256 hash of this event |
| `previous_event_hash` | string | REQUIRED | Hash of preceding event |

### 4.3 Event Types

Conforming runtimes MUST emit these event types:

#### 4.3.1 Session Events

| Event Type | Description | Required Payload Fields |
|------------|-------------|------------------------|
| `session.started` | Session created | `agent_id`, `goal` |
| `session.ended` | Session completed | `reason`, `duration_ms` |

#### 4.3.2 CARP Events

| Event Type | Description | Required Payload Fields |
|------------|-------------|------------------------|
| `carp.request.received` | CARP request received | `request_id`, `operation`, `goal` |
| `carp.resolution.completed` | Resolution computed | `resolution_id`, `decision_type`, `allowed_count`, `denied_count` |
| `carp.resolution.cached` | Resolution served from cache | `resolution_id`, `cache_hit` |

#### 4.3.3 Action Events

| Event Type | Description | Required Payload Fields |
|------------|-------------|------------------------|
| `action.requested` | Action execution requested | `action_id`, `parameters_hash` |
| `action.approved` | Action passed policy check | `action_id`, `resolution_id` |
| `action.denied` | Action denied by policy | `action_id`, `reason`, `policy_id` |
| `action.executed` | Action executed successfully | `action_id`, `execution_id`, `duration_ms` |
| `action.failed` | Action execution failed | `action_id`, `error_code`, `error_message` |

#### 4.3.4 Policy Events

| Event Type | Description | Required Payload Fields |
|------------|-------------|------------------------|
| `policy.evaluated` | Policy rule evaluated | `policy_id`, `result` |
| `policy.violated` | Policy violation detected | `policy_id`, `violation_type`, `details` |

#### 4.3.5 Context Events

| Event Type | Description | Required Payload Fields |
|------------|-------------|------------------------|
| `context.injected` | Context block added | `block_id`, `source`, `token_count` |
| `context.redacted` | Content redacted | `block_id`, `redaction_reason` |

### 4.4 Hash Chain

The hash chain provides tamper-evidence. For each event:

```
event_hash = SHA256(
  trace_version ||
  event_id ||
  trace_id ||
  span_id ||
  parent_span_id ||
  session_id ||
  sequence ||
  timestamp ||
  event_type ||
  canonical_json(payload) ||
  previous_event_hash
)
```

#### 4.4.1 Genesis Event

The first event in a session MUST have:
- `sequence`: 0
- `previous_event_hash`: "0000000000000000000000000000000000000000000000000000000000000000"

#### 4.4.2 Chain Verification

To verify chain integrity:

```
for i in 1..events.length:
  computed = compute_hash(events[i])
  if computed != events[i].event_hash:
    return INVALID("hash mismatch at event {i}")
  if events[i].previous_event_hash != events[i-1].event_hash:
    return INVALID("chain broken at event {i}")
  if events[i].sequence != events[i-1].sequence + 1:
    return INVALID("sequence gap at event {i}")
return VALID
```

### 4.5 Replay Semantics

A conforming runtime MUST support replay:

1. **Deterministic Replay**: Given the same TRACE and Atlas, replay produces identical resolutions
2. **Diff Generation**: Compare two traces and report semantic differences
3. **State Reconstruction**: Rebuild session state from TRACE events

### 4.6 Retention

Implementations SHOULD support configurable retention policies:
- Minimum retention: 24 hours
- Recommended retention: 30 days
- Compliance retention: As required by applicable regulations

---

## 5. Atlas/1.0 Specification

### 5.1 Overview

An Atlas is a versioned package containing everything needed to govern agent behavior in a domain.

### 5.2 Directory Structure

```
atlas-name/
├── atlas.json          # Manifest (REQUIRED)
├── context/            # Context documents
│   ├── overview.md
│   └── *.md
├── policies/           # Policy definitions
│   └── *.json
├── actions/            # Action definitions
│   └── *.json
├── adapters/           # Platform-specific configs
│   ├── openai.json
│   ├── anthropic.json
│   └── mcp.json
└── tests/              # Conformance tests
    └── *.trace.jsonl
```

### 5.3 Manifest Schema

```json
{
  "atlas_version": "1.0",
  "atlas_id": "<reverse-domain-notation>",
  "version": "<semver>",
  "name": "<string>",
  "description": "<string>",
  "authors": ["<string>"],
  "license": "<SPDX identifier>",
  "domains": ["<string>"],
  "capabilities": [
    {
      "capability_id": "<string>",
      "name": "<string>",
      "description": "<string>",
      "actions": ["<action_id>"]
    }
  ],
  "context_packs": [
    {
      "pack_id": "<string>",
      "name": "<string>",
      "files": ["<path>"],
      "priority": "<integer>",
      "conditions": {}
    }
  ],
  "policies": [
    {
      "policy_id": "<string>",
      "type": "allow | deny | rate_limit | require_approval | budget",
      "conditions": {},
      "actions": {}
    }
  ],
  "actions": [
    {
      "action_id": "<string>",
      "name": "<string>",
      "description": "<string>",
      "parameters_schema": {},
      "returns_schema": {},
      "risk_tier": "low | medium | high | critical",
      "idempotent": "<boolean>",
      "executor": "<string>"
    }
  ],
  "dependencies": {
    "<atlas_id>": "<semver-range>"
  }
}
```

### 5.4 Identifier Format

#### 5.4.1 Atlas ID

Atlas IDs use reverse domain notation:
- Format: `<tld>.<domain>.<name>`
- Example: `com.example.customer-support`
- MUST match regex: `^[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+$`

#### 5.4.2 Action ID

Action IDs are dot-separated hierarchical:
- Format: `<domain>.<resource>.<verb>`
- Example: `ticket.lookup`, `order.create`
- MUST match regex: `^[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)+$`

### 5.5 Policy Evaluation Order

Policies are evaluated in order:

1. Explicit `deny` rules (highest priority)
2. `require_approval` rules
3. `rate_limit` rules
4. `budget` rules
5. Explicit `allow` rules
6. Default deny (if no allow matches)

### 5.6 Versioning

Atlas versions follow Semantic Versioning 2.0.0:
- MAJOR: Breaking changes to actions or context
- MINOR: New actions or context, backward compatible
- PATCH: Bug fixes, documentation

---

## 6. Wire Formats

### 6.1 JSON Encoding

All protocol messages MUST be valid JSON (RFC 8259).

#### 6.1.1 Canonical JSON

For hashing, use canonical JSON:
- Keys sorted lexicographically
- No whitespace
- UTF-8 encoding
- Numbers as decimal (no scientific notation)

### 6.2 JSONL for Traces

TRACE events are serialized as JSON Lines:
- One JSON object per line
- UTF-8 encoded
- LF (0x0A) line separator
- File extension: `.trace.jsonl`

### 6.3 HTTP Transport

When exposed over HTTP:

| Endpoint | Method | Request | Response |
|----------|--------|---------|----------|
| `/v1/sessions` | POST | CreateSession | Session |
| `/v1/sessions/{id}` | GET | - | Session |
| `/v1/sessions/{id}` | DELETE | - | 204 |
| `/v1/resolve` | POST | CARPRequest | CARPResolution |
| `/v1/execute` | POST | ExecuteRequest | ExecuteResponse |
| `/v1/traces/{session_id}` | GET | - | TRACE[] |
| `/v1/atlases` | GET | - | AtlasSummary[] |
| `/v1/atlases/{id}` | GET | - | AtlasManifest |
| `/v1/health` | GET | - | HealthStatus |

### 6.4 WebSocket Transport

For real-time trace streaming:

```
ws://host/v1/traces/stream?session_id={id}

# Server sends TRACE events as JSON messages
{"event_type": "action.executed", ...}
```

---

## 7. Conformance Requirements

### 7.1 Conformance Levels

| Level | Requirements |
|-------|--------------|
| **CARP-Core** | CARP request/resolution, policy evaluation |
| **TRACE-Core** | Event emission, hash chain, basic verification |
| **Atlas-Core** | Manifest parsing, context loading |
| **CRA-Full** | All of the above + HTTP API + WebSocket |

### 7.2 Required Tests

Conforming implementations MUST pass:

1. **Schema Validation Tests**: All messages validate against JSON Schema
2. **Hash Chain Tests**: Chain computation matches reference
3. **Policy Evaluation Tests**: Policy ordering produces expected results
4. **Replay Tests**: Replay produces deterministic results
5. **Golden Trace Tests**: Output matches reference traces

### 7.3 Test Suite Location

Official conformance tests are at:
```
specs/conformance/
├── schema/           # JSON Schema validation tests
├── hash-chain/       # Hash computation tests
├── policy/           # Policy evaluation tests
├── replay/           # Replay determinism tests
└── golden/           # Golden trace comparisons
```

---

## 8. Security Considerations

### 8.1 Authentication

Implementations SHOULD support:
- JWT bearer tokens
- API key authentication
- Mutual TLS

### 8.2 Authorization

Implementations MUST:
- Validate session ownership
- Enforce policy denials
- Rate limit requests

### 8.3 Data Protection

Implementations SHOULD:
- Encrypt traces at rest
- Support field-level redaction
- Provide audit log access controls

### 8.4 Hash Chain Security

The hash chain provides:
- Tamper evidence (not tamper prevention)
- Append-only semantics
- Sequence verification

It does NOT provide:
- Non-repudiation (no signatures)
- Encryption
- Access control

---

## 9. IANA Considerations

### 9.1 Media Types

| Media Type | Description |
|------------|-------------|
| `application/vnd.cra.carp+json` | CARP request/resolution |
| `application/vnd.cra.trace+json` | Single TRACE event |
| `application/vnd.cra.trace+jsonl` | TRACE event stream |
| `application/vnd.cra.atlas+json` | Atlas manifest |

### 9.2 URI Schemes

| Scheme | Description |
|--------|-------------|
| `cra://` | CRA resource identifier |
| `atlas://` | Atlas package reference |

---

## Appendix A: JSON Schema Locations

| Schema | Location |
|--------|----------|
| CARP Request | `specs/schemas/carp-request.schema.json` |
| CARP Resolution | `specs/schemas/carp-resolution.schema.json` |
| TRACE Event | `specs/schemas/trace-event.schema.json` |
| Atlas Manifest | `specs/schemas/atlas-manifest.schema.json` |

---

## Appendix B: Reference Hash Computation

```python
import hashlib
import json

def canonical_json(obj):
    return json.dumps(obj, sort_keys=True, separators=(',', ':'))

def compute_event_hash(event):
    data = (
        event['trace_version'] +
        event['event_id'] +
        event['trace_id'] +
        event['span_id'] +
        (event.get('parent_span_id') or '') +
        event['session_id'] +
        str(event['sequence']) +
        event['timestamp'] +
        event['event_type'] +
        canonical_json(event['payload']) +
        event['previous_event_hash']
    )
    return hashlib.sha256(data.encode('utf-8')).hexdigest()
```

---

## Appendix C: Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-12-28 | Initial specification |

---

*This specification is released under the MIT License.*
