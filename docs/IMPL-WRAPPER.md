# CRA Wrapper Implementation Specification

## Overview

The CRA Wrapper (`cra-wrapper`) is the agent-side component that:
- Intercepts agent I/O through hooks
- Queues TRACE events locally for async upload
- Caches context to avoid redundant fetches
- Communicates with CRA server via transport backends

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     CRA WRAPPER                              │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   I/O       │  │   TRACE     │  │   Context   │         │
│  │   Hooks     │  │   Queue     │  │   Cache     │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          │                                  │
│                    ┌─────▼─────┐                           │
│                    │   CRA     │                           │
│                    │  Client   │                           │
│                    └───────────┘                           │
│                          │                                  │
│  ┌───────────────────────┼───────────────────────┐         │
│  │            TRANSPORT LAYER                     │         │
│  │  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐          │         │
│  │  │ MCP │  │REST │  │ WS  │  │Direct│          │         │
│  │  └─────┘  └─────┘  └─────┘  └─────┘          │         │
│  └───────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
cra-wrapper/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Main wrapper implementation
│   ├── error.rs         # Error types
│   ├── config.rs        # Configuration structs
│   ├── hooks.rs         # I/O hook system
│   ├── queue.rs         # TRACE event queue
│   ├── cache.rs         # Context cache
│   ├── client.rs        # CRA client interface
│   └── transport.rs     # Transport backends
```

## Core Components

### 1. Wrapper

The main wrapper struct that coordinates all components.

```rust
use cra_wrapper::{Wrapper, WrapperConfig};

// Create wrapper
let wrapper = Wrapper::new(WrapperConfig::default());

// Start session
let session_id = wrapper.start_session("Help user build a website").await?;

// Process I/O through hooks
let input = wrapper.on_input(user_input).await?;
let output = wrapper.on_output(agent_output).await?;

// Report actions
let decision = wrapper.report_action("write_file", params).await?;

// End session
let summary = wrapper.end_session(Some("Task complete")).await?;
```

### 2. I/O Hooks

Intercept agent input and output for context injection and auditing.

```rust
#[async_trait]
pub trait IOHooks: Send + Sync {
    /// Called before agent processes input
    async fn on_input(&self, input: &str) -> WrapperResult<String>;

    /// Called after agent produces output
    async fn on_output(&self, output: &str) -> WrapperResult<String>;

    /// Called before agent executes action
    async fn on_before_action(&self, action: &str, params: &Value) -> WrapperResult<ActionDecision>;

    /// Called after action completes
    async fn on_after_action(&self, action: &str, result: &ActionResult);
}
```

### 3. TRACE Queue

Async event queue for non-blocking trace collection.

```rust
pub struct TraceQueue {
    config: QueueConfig,
    events: RwLock<Vec<QueuedEvent>>,
    // statistics...
}

impl TraceQueue {
    pub async fn enqueue(&self, event: QueuedEvent);
    pub async fn flush(&self) -> WrapperResult<FlushResult>;
    pub async fn stats(&self) -> QueueStats;
}
```

**Configuration:**
```rust
pub struct QueueConfig {
    pub max_size: usize,           // Default: 100
    pub flush_interval_ms: u64,    // Default: 5000
    pub sync_events: Vec<String>,  // Events requiring sync flush
}
```

### 4. Context Cache

Cache for avoiding redundant context fetches.

```rust
pub struct ContextCache {
    config: CacheConfig,
    entries: RwLock<HashMap<String, CachedContext>>,
    // statistics...
}

impl ContextCache {
    pub async fn get(&self, key: &str) -> Option<CachedContext>;
    pub async fn set(&self, key: &str, context: CachedContext);
    pub async fn invalidate(&self, key: &str);
    pub async fn clear(&self);
    pub async fn evict_expired(&self);
}
```

**Configuration:**
```rust
pub struct CacheConfig {
    pub enabled: bool,              // Default: true
    pub default_ttl_seconds: u64,   // Default: 300
    pub max_entries: usize,         // Default: 1000
    pub backend: CacheBackendType,  // Memory or File
}
```

### 5. CRA Client

Interface for communicating with CRA server.

```rust
#[async_trait]
pub trait CRAClient: Send + Sync {
    async fn bootstrap(&self, goal: &str) -> WrapperResult<BootstrapResult>;
    async fn request_context(&self, session_id: &str, need: &str, hints: Option<Vec<String>>) -> WrapperResult<Vec<ContextBlock>>;
    async fn report_action(&self, session_id: &str, action: &str, params: Value) -> WrapperResult<ActionReport>;
    async fn feedback(&self, session_id: &str, context_id: &str, helpful: bool, reason: Option<&str>) -> WrapperResult<()>;
    async fn upload_trace(&self, events: Vec<Value>) -> WrapperResult<UploadResult>;
    async fn end_session(&self, session_id: &str, summary: Option<&str>) -> WrapperResult<EndSessionResult>;
}
```

**Implementations:**
- `DirectClient` - Same process (for testing)
- `McpClient` - Via MCP protocol (planned)
- `RestClient` - Via HTTP REST API (planned)

### 6. Transport Backends

Lower-level transport abstraction.

```rust
#[async_trait]
pub trait TransportBackend: Send + Sync {
    fn name(&self) -> &str;
    async fn connect(&mut self) -> WrapperResult<()>;
    async fn disconnect(&mut self) -> WrapperResult<()>;
    async fn request(&self, method: &str, params: Value) -> WrapperResult<Value>;
}
```

**Implementations:**
- `DirectTransport` - Same process
- `McpTransport` - MCP protocol
- `RestTransport` - HTTP REST

## Configuration

### Full Configuration Example

```rust
WrapperConfig {
    version: "1.0.0".to_string(),
    checkpoints_enabled: true,
    queue: QueueConfig {
        max_size: 100,
        flush_interval_ms: 5000,
        sync_events: vec!["policy_check".to_string(), "session_end".to_string()],
    },
    cache: CacheConfig {
        enabled: true,
        default_ttl_seconds: 300,
        max_entries: 1000,
        backend: CacheBackendType::Memory,
    },
    transport: TransportConfig {
        transport_type: TransportType::Direct,
        mcp_command: None,
        rest_url: None,
        timeout_ms: 30000,
    },
    hooks: HookConfig {
        intercept_input: true,
        intercept_output: true,
        intercept_actions: true,
        trigger_keywords: vec![],
    },
}
```

### JSON Configuration

```json
{
  "version": "1.0.0",
  "checkpoints_enabled": true,
  "queue": {
    "max_size": 100,
    "flush_interval_ms": 5000,
    "sync_events": ["policy_check", "session_end"]
  },
  "cache": {
    "enabled": true,
    "default_ttl_seconds": 300,
    "max_entries": 1000,
    "backend": "memory"
  },
  "transport": {
    "transport_type": "mcp",
    "mcp_command": "cra-mcp-server",
    "timeout_ms": 30000
  },
  "hooks": {
    "intercept_input": true,
    "intercept_output": true,
    "intercept_actions": true,
    "trigger_keywords": ["geometry", "shader", "vib3"]
  }
}
```

## Lifecycle

### Session Lifecycle

```
1. wrapper.start_session(goal)
   │
   ├── Bootstrap with CRA
   ├── Receive governance rules
   ├── Receive initial context
   ├── Cache contexts
   └── Emit session_started event
   │
   ▼
2. Operational Loop
   │
   ├── wrapper.on_input(input)
   │   ├── Check for checkpoint triggers
   │   ├── Request context if needed
   │   ├── Inject context into input
   │   └── Emit input_received event
   │
   ├── Agent processes...
   │
   ├── wrapper.on_output(output)
   │   └── Emit output_produced event
   │
   ├── wrapper.report_action(action, params)
   │   ├── Check policy
   │   └── Emit action_reported event
   │
   └── [Background: queue flushes periodically]
   │
   ▼
3. wrapper.end_session(summary)
   │
   ├── Flush queue (sync)
   ├── Verify chain
   └── Return summary
```

## Platform Integration

### Python

```python
from cra_wrapper import Wrapper, WrapperConfig

wrapper = Wrapper(WrapperConfig.default())

@wrapper.wrap
def process_request(input):
    # Agent logic here
    return output
```

### TypeScript/Node.js

```typescript
import { Wrapper, WrapperConfig } from '@cra/wrapper';

const wrapper = new Wrapper(WrapperConfig.default());

wrapper.startSession("Help user");

const input = await wrapper.onInput(userInput);
// ... agent processes ...
await wrapper.onOutput(agentOutput);

await wrapper.endSession("Done");
```

### Claude Code Hooks

```javascript
// .claude/hooks/cra-wrapper.js
const { Wrapper } = require('@cra/wrapper');

const wrapper = new Wrapper({ transport: 'mcp' });

module.exports = {
  async onPreToolUse(tool, input) {
    const decision = await wrapper.reportAction(tool.name, input);
    if (!decision.allowed) {
      throw new Error(decision.reason);
    }
    return input;
  },

  async onPostToolUse(tool, output) {
    await wrapper.onOutput(JSON.stringify(output));
    return output;
  }
};
```

## Dependencies

- `tokio` - Async runtime
- `async-trait` - Async traits
- `serde` / `serde_json` - Serialization
- `chrono` - Time handling
- `uuid` - ID generation
- `tracing` - Logging

## Future Enhancements

1. **Plugin System** - Extensible hook points
2. **WebSocket Transport** - Real-time bidirectional
3. **Offline Mode** - Queue indefinitely when disconnected
4. **Encryption** - Encrypt queued events
5. **Compression** - Compress large payloads
6. **Metrics** - Prometheus/OpenTelemetry integration
