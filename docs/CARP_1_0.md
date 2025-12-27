# CARP/1.0 — Context & Action Resolution Protocol (Draft v0.1)

## Purpose
CARP is a contract between a Requester (acting agent/model) and a Resolver (CRA runtime) that determines what context may be injected, what actions are allowed, what constraints apply, and how telemetry is linked.

## Envelope
```json
{
  "carp_version": "1.0",
  "type": "carp.request|carp.response",
  "id": "uuid",
  "time": "RFC3339",
  "session": {"session_id":"uuid","principal":{"type":"user|service|agent","id":"string"},"scopes":["string"],"expires_at":"RFC3339"},
  "atlas": {"id":"string","version":"semver","capability":"string"},
  "payload": {},
  "trace": {"trace_id":"uuid","span_id":"uuid","parent_span_id":"uuid|null"}
}
```

## Operation: resolve (request payload)
```json
{
  "operation": "resolve",
  "task": {
    "goal": "string",
    "inputs": [{"name":"string","type":"text|json|uri|file_ref","value":"..."}],
    "constraints": ["string"],
    "target_platforms": ["openai.tools","anthropic.skills","google.adk","mcp"],
    "risk_tier": "low|medium|high"
  },
  "environment": {
    "project_root": "string|null",
    "os": "string|null",
    "cli_capabilities": ["bash","python","git","docker"],
    "network_policy": "offline|restricted|open"
  },
  "preferences": {"verbosity":"compact|standard|extended","format":["json","markdown"],"explainability":"minimal|standard|deep"}
}
```

## Operation: resolve (response payload)
```json
{
  "operation": "resolve",
  "resolution": {
    "resolution_id": "uuid",
    "confidence": 0.0,
    "context_blocks": [
      {
        "block_id": "string",
        "purpose": "string",
        "ttl_seconds": 3600,
        "content_type": "text/markdown|application/json|text/plain|image/png",
        "content": "string or object",
        "redactions": [{"field":"string","reason":"string"}],
        "source_evidence": [{"type":"doc|api|policy|test","ref":"string","hash":"sha256"}]
      }
    ],
    "allowed_actions": [
      {
        "action_id": "string",
        "kind": "tool_call|mcp_call|cli_command|agent_tool",
        "adapter": "string",
        "schema": {"json_schema": {}},
        "constraints": [{"type":"rate_limit|scope|approval|sandbox","value":"..."}],
        "requires_approval": true
      }
    ],
    "denylist": [{"pattern":"string","reason":"string"}],
    "merge_rules": {"conflict":"fail|last_write_wins|priority"},
    "next_steps": [{"step":"string","expected_artifacts":["string"]}]
  }
}
```

## Governance Hooks
- Authentication and authorization via scopes
- Policy evaluation (allow/deny)
- Redaction for secrets and PII
- Approvals and rate limiting

## Transport Guidance
CARP is transport‑agnostic. This repository provides HTTP and CLI bindings, and it can be carried over MCP or vendor tool‑calling transports via adapters.
