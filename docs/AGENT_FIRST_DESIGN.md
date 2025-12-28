# CRA Agent-First Design

## Philosophy: Zero-Friction Agent Integration

When an AI agent first encounters CRA, it should:
1. **Instantly understand** what CRA does via a single API call
2. **Self-configure** its agents.md and context automatically
3. **Discover all capabilities** without documentation hunting
4. **Integrate seamlessly** with minimal tool calls

---

## The Problem with Traditional APIs

Traditional APIs require agents to:
- Read documentation (multiple tool calls)
- Understand authentication (trial and error)
- Discover endpoints (guessing or exploration)
- Map capabilities to their needs (reasoning overhead)

**CRA Solution**: Single-endpoint introspection that returns everything an agent needs.

---

## Agent Discovery Endpoint

```
GET /v1/discover
```

Returns a complete, structured description of CRA that agents can parse immediately:

```json
{
  "system": {
    "name": "CRA - Context Registry Agents",
    "version": "0.2.0",
    "purpose": "Authority layer for agentic systems with context governance",
    "one_liner": "Ask CRA what you can do before doing it"
  },

  "quick_start": {
    "step_1": "POST /v1/resolve with your goal to get permitted actions",
    "step_2": "Use returned context_blocks in your system prompt",
    "step_3": "Only use actions from allowed_actions list",
    "step_4": "POST /v1/execute when taking permitted actions"
  },

  "integration": {
    "agents_md_snippet": "## CRA Integration\n\nBefore any action:\n1. Resolve via POST /v1/resolve\n2. Only use permitted actions\n3. TRACE events are authoritative\n",
    "recommended_system_prompt": "You operate under CRA governance. Always resolve context before acting. Only use permitted tools.",
    "update_frequency": "Resolve once per task, refresh on context change"
  },

  "endpoints": {
    "resolve": {
      "method": "POST",
      "path": "/v1/resolve",
      "purpose": "Get context and permitted actions for a goal",
      "required_fields": ["goal"],
      "example_request": {"goal": "Create a GitHub issue", "risk_tier": "medium"},
      "example_response_shape": "CARPResolution with context_blocks and allowed_actions"
    },
    "execute": {
      "method": "POST",
      "path": "/v1/execute",
      "purpose": "Execute a permitted action",
      "required_fields": ["action_type", "resolution_id", "parameters"],
      "prerequisite": "Must have valid resolution_id from /resolve"
    },
    "stream": {
      "method": "WebSocket",
      "path": "/v1/stream",
      "purpose": "Real-time resolution and trace streaming",
      "message_types": ["resolution_chunk", "trace_event", "action_result"]
    }
  },

  "atlases": {
    "loaded": ["github-ops", "slack-comms"],
    "available_domains": ["api.github", "api.slack", "file.read", "file.write"],
    "total_actions": 47,
    "total_context_packs": 12
  },

  "for_agents": {
    "output_formats": {
      "json": "Default structured output",
      "jsonl": "Streaming line-delimited JSON",
      "markdown": "Human-readable with structure preserved"
    },
    "batch_operations": {
      "supported": true,
      "endpoint": "/v1/batch",
      "max_items": 100
    },
    "caching": {
      "resolution_ttl_seconds": 300,
      "cache_key_in_response": true
    }
  }
}
```

---

## Agent-Optimized Response Formats

### Standard Response Envelope

Every response includes agent-friendly metadata:

```json
{
  "_meta": {
    "request_id": "req_123",
    "processing_ms": 45,
    "cache_hit": false,
    "next_actions": ["execute", "refresh", "stream"],
    "related_endpoints": ["/v1/execute", "/v1/trace/current"]
  },
  "data": { ... actual response ... },
  "_agent_hints": {
    "context_tokens": 1247,
    "action_count": 5,
    "suggested_next": "Include context_blocks in your system prompt"
  }
}
```

### Streaming Format (JSONL)

For long-running operations, agents receive incremental updates:

```jsonl
{"type":"resolution.started","resolution_id":"res_123"}
{"type":"context.chunk","domain":"api.github","tokens":340}
{"type":"context.chunk","domain":"api.github","tokens":280}
{"type":"actions.available","count":5,"risk_tiers":{"low":3,"medium":2}}
{"type":"resolution.complete","ttl_seconds":300}
```

---

## Dual-Mode UI Architecture

### Human Mode (Terminal-Style)
- Rich interactive terminal with syntax highlighting
- Command history and autocomplete
- Visual trace timeline
- Collapsible context blocks
- Real-time streaming output

### Agent Mode (Structured Output)
- Pure JSON responses
- No formatting overhead
- Predictable structure
- Batch operation support
- Minimal tokens for context

### Mode Detection
```
Accept: application/json          → Agent mode
Accept: text/html                 → Human mode
X-Agent-Mode: true               → Force agent mode
?format=agent                    → Force agent mode via query
```

---

## Self-Updating Integration

### Auto-Generated agents.md

When an agent calls `/v1/discover?generate=agents-md`:

```markdown
# agents.md — CRA Contract (Auto-Generated)

## About This File
This file was auto-generated by CRA at 2024-01-15T10:30:00Z.
It defines operational rules for AI agents in this project.

## CRA Integration

### Before Any Action
1. Call `POST /v1/resolve` with your goal
2. Parse the response for `allowed_actions`
3. Include `context_blocks` in your reasoning
4. Only execute actions that were permitted

### Available Atlases
- **github-ops** (v1.2.0): GitHub repository operations
- **slack-comms** (v1.0.0): Slack messaging and channels

### Risk Tiers
- **low**: Read-only operations, always auto-approved
- **medium**: Modifications, auto-approved with logging
- **high**: Destructive operations, requires explicit approval
- **critical**: Never auto-approved

### Quick Reference
\`\`\`bash
# Resolve before acting
curl -X POST http://localhost:3000/v1/resolve \
  -d '{"goal": "your goal here"}'

# Execute permitted action
curl -X POST http://localhost:3000/v1/execute \
  -d '{"action_type": "api.github.create_issue", ...}'
\`\`\`

## Telemetry
All operations are traced via TRACE protocol.
View traces at: http://localhost:3000/ui/traces
```

---

## Agent Onboarding Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Encounters CRA                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  GET /v1/discover                                            │
│  → Receives complete system description                      │
│  → Understands purpose in one call                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  GET /v1/discover?generate=agents-md                        │
│  → Receives ready-to-use agents.md content                  │
│  → Auto-configures its context                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  POST /v1/resolve {"goal": "first task"}                    │
│  → Gets context and permitted actions                       │
│  → Learns what's available dynamically                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Agent is now fully integrated                               │
│  → Knows all capabilities                                    │
│  → Has proper context                                        │
│  → Can operate within governance                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Packages

1. **@cra/ui** - Dual-mode web interface
2. **@cra/discover** - Agent discovery and onboarding
3. **@cra/stream** - Real-time streaming support
4. **@cra/redact** - Sensitive data protection
5. **@cra/postgres** - PostgreSQL storage backend

---

## Success Metrics

An agent should be able to:
- Understand CRA's purpose in **1 API call**
- Self-configure in **2 API calls**
- Start operating in **under 30 seconds**
- Never need to read external documentation
