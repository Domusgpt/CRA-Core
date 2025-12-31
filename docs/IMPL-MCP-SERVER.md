# CRA MCP Server Implementation Specification

## Overview

The CRA MCP Server (`cra-mcp`) implements the Model Context Protocol to expose CRA governance tools to agents like Claude, GPT, and other MCP-compatible systems.

## Architecture

```
Agent (Claude, GPT, etc.)
       │
       ▼ JSON-RPC over stdio
┌─────────────────┐
│   MCP Server    │
│                 │
│  ┌───────────┐  │
│  │   Tools   │  │  - cra_start_session
│  │           │  │  - cra_request_context
│  │           │  │  - cra_report_action
│  │           │  │  - cra_feedback
│  │           │  │  - cra_end_session
│  │           │  │  - cra_bootstrap
│  └───────────┘  │
│                 │
│  ┌───────────┐  │
│  │ Resources │  │  - cra://session/current
│  │           │  │  - cra://trace/{id}
│  │           │  │  - cra://atlas/{id}
│  └───────────┘  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    cra-core     │
└─────────────────┘
```

## Crate Structure

```
cra-mcp/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── server.rs        # MCP server implementation
│   ├── session.rs       # Session management
│   ├── bootstrap.rs     # Bootstrap protocol
│   ├── error.rs         # Error types
│   ├── tools/
│   │   ├── mod.rs       # Tool registry
│   │   ├── session.rs   # Session tools
│   │   ├── context.rs   # Context tools
│   │   ├── action.rs    # Action tools
│   │   └── feedback.rs  # Feedback tools
│   └── resources/
│       └── mod.rs       # Resource handlers
```

## Tools Implemented

### 1. `cra_start_session`

Start a governed session with CRA.

**Input:**
```json
{
  "goal": "string",
  "atlas_hints": ["string"]  // optional
}
```

**Output:**
```json
{
  "session_id": "string",
  "active_atlases": ["string"],
  "initial_context": [...],
  "genesis_hash": "string"
}
```

### 2. `cra_request_context`

Request relevant context for a need.

**Input:**
```json
{
  "need": "string",
  "hints": ["string"]  // optional
}
```

**Output:**
```json
{
  "matched_contexts": [...],
  "trace_id": "string"
}
```

### 3. `cra_report_action`

Report an action for audit trail.

**Input:**
```json
{
  "action": "string",
  "params": {}  // optional
}
```

**Output:**
```json
{
  "decision": "approved|denied",
  "trace_id": "string",
  "reason": "string",  // if denied
  "policy_notes": ["string"]
}
```

### 4. `cra_feedback`

Submit feedback on context usefulness.

**Input:**
```json
{
  "context_id": "string",
  "helpful": true|false,
  "reason": "string"  // optional
}
```

### 5. `cra_end_session`

End the governed session.

**Input:**
```json
{
  "summary": "string"  // optional
}
```

**Output:**
```json
{
  "session_id": "string",
  "duration_ms": 12345,
  "event_count": 42,
  "chain_verified": true,
  "final_hash": "string"
}
```

### 6. `cra_bootstrap`

Full bootstrap handshake in one call.

**Input:**
```json
{
  "intent": "string",
  "capabilities": ["string"]  // optional
}
```

**Output:**
```json
{
  "session_id": "string",
  "genesis_hash": "string",
  "governance": {
    "rules": [...],
    "policies": [...],
    "you_must": [...]
  },
  "context": [...],
  "chain_state": {...},
  "ready": true,
  "message": "string"
}
```

## Resources Implemented

### `cra://session/current`

Returns current session state.

### `cra://trace/{session_id}`

Returns TRACE audit trail for a session.

### `cra://chain/{session_id}`

Returns chain verification status.

### `cra://atlas/{atlas_id}`

Returns atlas manifest.

## Usage

### CLI

```bash
# Run with atlases directory
cra-mcp-server --atlases ./atlases

# Run with verbose logging
cra-mcp-server --atlases ./atlases --verbose
```

### Claude Code Configuration

Add to `~/.claude/claude_code_config.json`:

```json
{
  "mcpServers": {
    "cra": {
      "command": "cra-mcp-server",
      "args": ["--atlases", "/path/to/atlases"]
    }
  }
}
```

## Dependencies

- `cra-core` - Core CRA library
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `clap` - CLI parsing
- `tracing` - Logging

## Key Components

### McpServer

Main server struct that handles JSON-RPC requests over stdio.

```rust
let server = McpServer::builder()
    .with_atlases_dir("./atlases")
    .build()
    .await?;

server.run_stdio().await?;
```

### SessionManager

Manages all active sessions and coordinates with cra-core.

```rust
let manager = SessionManager::new()
    .with_atlases_dir("./atlases");

manager.load_atlases()?;

let session = manager.start_session(
    "agent-id".to_string(),
    "Help build a website".to_string(),
    None,
)?;
```

### BootstrapProtocol

Implements the bootstrap handshake protocol.

```rust
let mut protocol = BootstrapProtocol::new();

let governance = protocol.handle_init(init_msg, session)?;
protocol.handle_ack(ack_msg)?;
let context = protocol.generate_context(contexts, false)?;
let session_msg = protocol.handle_ready(ready_msg)?;
```

## Integration Points

1. **cra-core Resolver** - Used for CARP resolution and policy evaluation
2. **cra-core TraceCollector** - Used for TRACE event emission
3. **cra-core ChainVerifier** - Used for chain verification
4. **Atlas loading** - Loads atlases from directory or programmatically

## Future Enhancements

1. **REST API adapter** - Expose same tools via HTTP
2. **WebSocket support** - Real-time event streaming
3. **Multi-tenant mode** - Support multiple concurrent agents
4. **Authentication** - API key and session token support
5. **Rate limiting** - Per-agent rate limits
