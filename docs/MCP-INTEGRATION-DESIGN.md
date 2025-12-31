# CRA MCP Integration Design

## Overview

This document describes how CRA (Context Registry for Agents) can be exposed through MCP (Model Context Protocol) and similar interfaces, enabling agents on different platforms to self-integrate with CRA governance.

## The Problem

Agents need a way to:
1. **Request context** when they need information
2. **Report actions** for audit trail
3. **Give feedback** to improve atlas quality
4. **Authenticate** their session with CRA

Currently this requires building custom wrappers for each agent platform.

## The Solution: MCP-Based Integration

MCP provides a standardized protocol for exposing tools to LLMs. By implementing CRA as an MCP server, any MCP-compatible agent can use CRA tools natively.

```
┌─────────────────────────────────────────────────────────────┐
│                     Agent Platforms                          │
├─────────────┬─────────────┬─────────────┬──────────────────┤
│   Claude    │   OpenAI    │   Local     │    Custom        │
│   (MCP)     │   (Actions) │   (Ollama)  │    Agents        │
└──────┬──────┴──────┬──────┴──────┬──────┴──────┬───────────┘
       │             │             │             │
       └─────────────┴──────┬──────┴─────────────┘
                            │
                    ┌───────▼───────┐
                    │  CRA MCP      │
                    │  Server       │
                    └───────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
        ┌─────▼─────┐ ┌─────▼─────┐ ┌─────▼─────┐
        │   CARP    │ │   TRACE   │ │   Atlas   │
        │  Resolver │ │ Collector │ │  Registry │
        └───────────┘ └───────────┘ └───────────┘
```

---

## MCP Server Implementation

### Server Metadata

```json
{
  "name": "cra-governance",
  "version": "1.0.0",
  "description": "Context Registry for Agents - Governance and audit layer",
  "vendor": "CRA Development"
}
```

### Tools Exposed

#### 1. cra_start_session

Start a governed session with CRA.

```json
{
  "name": "cra_start_session",
  "description": "Start a governed session with CRA. Call this first before using other CRA tools.",
  "inputSchema": {
    "type": "object",
    "required": ["goal"],
    "properties": {
      "goal": {
        "type": "string",
        "description": "What you're trying to accomplish in this session"
      },
      "atlas_hints": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Optional: domains/atlases relevant to your task"
      }
    }
  }
}
```

**Returns:**
```json
{
  "session_id": "uuid",
  "active_atlases": ["vib3-webpage-development", "cra-development"],
  "initial_context": [
    {
      "context_id": "vib3-what-it-is",
      "priority": 400,
      "content": "# What VIB3+ Is..."
    }
  ]
}
```

#### 2. cra_request_context

Request relevant context for your current need.

```json
{
  "name": "cra_request_context",
  "description": "Ask CRA for context relevant to what you're working on. Be specific about what information would help.",
  "inputSchema": {
    "type": "object",
    "required": ["need"],
    "properties": {
      "need": {
        "type": "string",
        "description": "What information would help you right now"
      },
      "hints": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Optional keywords to improve context matching"
      }
    }
  }
}
```

**Returns:**
```json
{
  "matched_contexts": [
    {
      "context_id": "vib3-geometry-system",
      "name": "Geometry System",
      "priority": 380,
      "match_score": 0.85,
      "content": "# VIB3+ Geometry System..."
    }
  ],
  "trace_id": "event-uuid-for-audit"
}
```

#### 3. cra_report_action

Report what action you're about to take (for audit trail).

```json
{
  "name": "cra_report_action",
  "description": "Tell CRA what action you're taking. This creates an audit trail and may be subject to policy evaluation.",
  "inputSchema": {
    "type": "object",
    "required": ["action"],
    "properties": {
      "action": {
        "type": "string",
        "description": "What you're doing (e.g., 'write_file', 'execute_code', 'api_call')"
      },
      "params": {
        "type": "object",
        "description": "Relevant parameters for the action"
      }
    }
  }
}
```

**Returns:**
```json
{
  "decision": "approved",
  "trace_id": "event-uuid",
  "policy_notes": ["Action permitted under default policy"]
}
```

Or if denied:
```json
{
  "decision": "denied",
  "trace_id": "event-uuid",
  "reason": "Policy 'no-production-writes' prevents file writes to /prod/*",
  "alternatives": ["Write to /staging/* instead"]
}
```

#### 4. cra_feedback

Report whether context was helpful (improves future responses).

```json
{
  "name": "cra_feedback",
  "description": "Report if context was helpful. This improves CRA's context matching for future requests.",
  "inputSchema": {
    "type": "object",
    "required": ["context_id", "helpful"],
    "properties": {
      "context_id": {
        "type": "string",
        "description": "Which context block you're giving feedback on"
      },
      "helpful": {
        "type": "boolean",
        "description": "Was this context helpful for your task?"
      },
      "reason": {
        "type": "string",
        "description": "Why it was or wasn't helpful (improves atlas quality)"
      }
    }
  }
}
```

#### 5. cra_list_atlases

Discover what atlases are available.

```json
{
  "name": "cra_list_atlases",
  "description": "List all available atlases that can provide context",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

**Returns:**
```json
{
  "atlases": [
    {
      "atlas_id": "vib3-webpage-development",
      "name": "VIB3+ Development Atlas",
      "version": "6.0.0",
      "description": "Context for building websites with VIB3+ shader backgrounds",
      "domains": ["web-development", "vib3", "shaders"]
    }
  ]
}
```

#### 6. cra_search_contexts

Search all contexts across atlases.

```json
{
  "name": "cra_search_contexts",
  "description": "Search all available context blocks across loaded atlases",
  "inputSchema": {
    "type": "object",
    "required": ["query"],
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query for context content and keywords"
      },
      "limit": {
        "type": "integer",
        "default": 10,
        "description": "Maximum results to return"
      }
    }
  }
}
```

#### 7. cra_end_session

End the governed session.

```json
{
  "name": "cra_end_session",
  "description": "End the CRA session. Finalizes the audit trail.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "summary": {
        "type": "string",
        "description": "Optional summary of what was accomplished"
      }
    }
  }
}
```

---

## Resources Exposed

MCP also supports "resources" - data the agent can read.

### Active Session Info
```
cra://session/current
```
Returns current session state, loaded atlases, context received.

### Audit Trail
```
cra://trace/{session_id}
```
Returns the TRACE audit trail for a session.

### Atlas Manifest
```
cra://atlas/{atlas_id}
```
Returns full atlas manifest for inspection.

---

## Platform Adapters

### Claude (Native MCP)

Claude Code and Claude Desktop support MCP natively. Configuration:

```json
{
  "mcpServers": {
    "cra": {
      "command": "cra-mcp-server",
      "args": ["--atlases", "./atlases"],
      "env": {}
    }
  }
}
```

### OpenAI Actions

OpenAI uses "Actions" with OpenAPI schemas. Adapter translates:

```yaml
openapi: 3.1.0
info:
  title: CRA Governance API
  version: 1.0.0
paths:
  /session/start:
    post:
      operationId: cra_start_session
      summary: Start a governed session
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [goal]
              properties:
                goal:
                  type: string
```

### Local Agents (REST API)

For agents that don't support MCP, expose REST endpoints:

```
POST /api/v1/session/start
POST /api/v1/context/request
POST /api/v1/action/report
POST /api/v1/feedback
POST /api/v1/session/end
```

### Direct Rust Integration

For Rust-based agents, use the library directly:

```rust
use cra_core::{Resolver, CARPRequest};

let resolver = Resolver::new(atlases);
let resolution = resolver.resolve(CARPRequest {
    goal: "Add shader background".into(),
    ..Default::default()
});
```

---

## Authentication Flow

### Option 1: Session Tokens

```
1. Agent calls cra_start_session
2. CRA returns session_id token
3. Agent includes session_id in subsequent calls
4. CRA validates token matches active session
```

### Option 2: API Keys (for hosted CRA)

```
1. Developer registers for API key
2. Key configured in agent's MCP settings
3. All requests authenticated via key
4. CRA logs activity per key
```

### Option 3: Zero-Auth (Local)

For local development, no authentication required. All calls trusted.

---

## Feedback Loop for Atlas Improvement

```
┌──────────────────────────────────────────────────────────────┐
│                    Feedback Loop                              │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Agent requests context for "how to embed VIB3"           │
│                        ↓                                      │
│  2. CRA returns vib3-embed-iframe context                    │
│                        ↓                                      │
│  3. Agent uses context, succeeds                             │
│                        ↓                                      │
│  4. Agent sends feedback: helpful=true, reason="..."         │
│                        ↓                                      │
│  5. CRA records in TRACE:                                    │
│     - Which context was matched                              │
│     - What the need was                                       │
│     - Whether it helped                                       │
│     - Why (if provided)                                      │
│                        ↓                                      │
│  6. Atlas maintainer reviews feedback                        │
│     - High positive feedback → context is good               │
│     - Negative feedback → improve content or keywords         │
│     - Missing matches → add new context blocks               │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Feedback Aggregation

```json
{
  "context_id": "vib3-geometry-system",
  "feedback_summary": {
    "total_requests": 47,
    "helpful_count": 42,
    "not_helpful_count": 5,
    "common_needs_when_helpful": [
      "geometry formula",
      "index calculation"
    ],
    "common_reasons_not_helpful": [
      "needed API examples, not just formula"
    ]
  }
}
```

---

## Implementation Phases

### Phase 1: Core MCP Server (MVP)

1. Implement MCP server in Rust using `mcp-server` crate
2. Expose 5 basic tools (start, request, report, feedback, end)
3. Connect to existing CRA resolver
4. Test with Claude Code

### Phase 2: Platform Adapters

1. OpenAPI spec generation for OpenAI Actions
2. REST API wrapper for legacy systems
3. WebSocket support for real-time updates

### Phase 3: Feedback System

1. Implement feedback storage
2. Build aggregation queries
3. Create feedback dashboard
4. Automated atlas suggestions

### Phase 4: Hosted Service (Optional)

1. Multi-tenant CRA service
2. API key management
3. Rate limiting
4. Usage analytics

---

## Example: Claude Code Integration

### Setup

```bash
# Add to ~/.claude/claude_code_config.json
{
  "mcpServers": {
    "cra": {
      "command": "npx",
      "args": ["-y", "cra-mcp-server", "--atlases", "/path/to/atlases"]
    }
  }
}
```

### Agent Behavior

When Claude Code starts a task, it would:

```
1. User: "Add a VIB3 shader background to my site"

2. Claude calls: cra_start_session(goal="Add VIB3 shader background")
   → Gets session_id, initial context about VIB3

3. Claude calls: cra_request_context(need="How to embed VIB3 in HTML")
   → Gets vib3-embed-iframe context with code examples

4. Claude calls: cra_report_action(action="write_file", params={path: "index.html"})
   → Gets approved

5. Claude writes the file using the context

6. Claude calls: cra_feedback(context_id="vib3-embed-iframe", helpful=true)

7. Claude calls: cra_end_session()
```

---

## File Structure

```
cra-mcp/
├── src/
│   ├── main.rs           # MCP server entry point
│   ├── tools/            # Tool implementations
│   │   ├── session.rs
│   │   ├── context.rs
│   │   ├── action.rs
│   │   └── feedback.rs
│   ├── resources/        # Resource handlers
│   └── adapters/         # Platform adapters
├── Cargo.toml
└── README.md
```

---

## Next Steps

1. **Create cra-mcp crate** - MCP server implementation
2. **Define tool schemas** - Finalize input/output schemas
3. **Test with Claude Code** - Verify MCP integration works
4. **Create OpenAPI adapter** - For OpenAI compatibility
5. **Build feedback dashboard** - Visualize context effectiveness
