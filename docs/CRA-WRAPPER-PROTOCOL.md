# CRA Wrapper Protocol Specification

## Overview

This document specifies how CRA wrappers are constructed, what they must do, and how they communicate with CRA. The protocol is designed to be extensible for future plugins, integrations, and platform-specific implementations.

---

## Core Requirements

Every CRA wrapper MUST:

1. **Identify itself** - Report agent identity during construction
2. **Provide I/O access** - Let CRA see inputs and outputs
3. **Accept context injection** - Receive and apply injected context
4. **Collect TRACE data** - Capture events for the audit trail
5. **Upload TRACE data** - Send collected data to CRA (async by default)

Every CRA wrapper SHOULD:

1. **Cache context** - Avoid redundant fetches
2. **Handle checkpoints** - Trigger injection at configured points
3. **Support policies** - Respect allow/deny decisions
4. **Provide feedback** - Report context usefulness

---

## Wrapper Structure

### Minimal Wrapper

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
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Extended Wrapper (with plugins)

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
│  ┌───────────────────────┼───────────────────────┐         │
│  │              PLUGIN LAYER                      │         │
│  │                                                │         │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐       │         │
│  │  │ Plugin  │  │ Plugin  │  │ Plugin  │       │         │
│  │  │   A     │  │   B     │  │   C     │       │         │
│  │  └─────────┘  └─────────┘  └─────────┘       │         │
│  │                                                │         │
│  └───────────────────────┼───────────────────────┘         │
│                          │                                  │
│                    ┌─────▼─────┐                           │
│                    │   CRA     │                           │
│                    │  Client   │                           │
│                    └───────────┘                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Specifications

### 1. I/O Hooks

**Purpose:** Intercept agent inputs and outputs

**Interface:**
```typescript
interface IOHooks {
  // Called before agent processes input
  onInput(input: AgentInput): AgentInput | Promise<AgentInput>;

  // Called after agent produces output
  onOutput(output: AgentOutput): AgentOutput | Promise<AgentOutput>;

  // Called before agent executes action
  onBeforeAction(action: Action): ActionDecision | Promise<ActionDecision>;

  // Called after action completes
  onAfterAction(action: Action, result: ActionResult): void;
}

interface ActionDecision {
  allowed: boolean;
  reason?: string;
  injectedContext?: string;
}
```

**Extensibility:** Hooks can be chained. Plugins can add additional hooks.

---

### 2. TRACE Queue

**Purpose:** Collect events for async upload

**Interface:**
```typescript
interface TraceQueue {
  // Add event to queue
  enqueue(event: TraceEvent): void;

  // Flush queue (upload to CRA)
  flush(): Promise<FlushResult>;

  // Get queue status
  status(): QueueStatus;

  // Set flush triggers
  configure(config: QueueConfig): void;
}

interface QueueConfig {
  maxSize: number;           // Flush when queue reaches this size
  flushIntervalMs: number;   // Flush on this interval
  syncEvents: string[];      // Event types that require sync flush
}

interface TraceEvent {
  eventType: string;
  timestamp: string;
  sessionId: string;
  sequence: number;
  payload: Record<string, unknown>;
  // Hash computed on flush, not on enqueue
}
```

**Extensibility:** Custom event types can be registered. Plugins can add events.

---

### 3. Context Cache

**Purpose:** Store fetched context to avoid redundant requests

**Interface:**
```typescript
interface ContextCache {
  // Get cached context
  get(key: string): CachedContext | null;

  // Store context
  set(key: string, context: ContextBlock, ttl?: number): void;

  // Invalidate cache entry
  invalidate(key: string): void;

  // Clear entire cache
  clear(): void;
}

interface CachedContext {
  content: string;
  fetchedAt: number;
  expiresAt: number;
  hash: string;
}
```

**Extensibility:** Cache backends can be swapped (memory, disk, redis, etc.)

---

### 4. CRA Client

**Purpose:** Communicate with CRA (MCP or direct)

**Interface:**
```typescript
interface CRAClient {
  // Bootstrap/initialize
  bootstrap(config: BootstrapConfig): Promise<BootstrapResult>;

  // Request context
  requestContext(need: string, hints?: string[]): Promise<ContextBlock[]>;

  // Report action
  reportAction(action: string, params: Record<string, unknown>): Promise<ActionResult>;

  // Submit feedback
  feedback(contextId: string, helpful: boolean, reason?: string): Promise<void>;

  // Upload TRACE batch
  uploadTrace(events: TraceEvent[]): Promise<UploadResult>;

  // End session
  endSession(summary?: string): Promise<SessionSummary>;
}
```

**Extensibility:** Multiple transport backends (MCP, REST, WebSocket, direct library call).

---

## Wrapper Lifecycle

### Construction Phase

```
1. Agent initiates wrapper construction
        │
        ▼
2. CRA Client connects to CRA
        │
        ▼
3. CRA auto-detects agent info
        │
        ▼
4. CRA asks essential questions (if needed)
        │
        ▼
5. Agent provides answers
        │
        ▼
6. CRA returns:
   - Wrapper template/instructions
   - Governance rules
   - Initial context
   - Session ID + genesis hash
        │
        ▼
7. Agent constructs wrapper components:
   - I/O Hooks
   - TRACE Queue
   - Context Cache
   - CRA Client
        │
        ▼
8. CRA verifies wrapper construction
        │
        ▼
9. Wrapper is active
```

### Operational Phase

```
┌─────────────────────────────────────────────────────────────┐
│                     OPERATIONAL LOOP                         │
│                                                              │
│  Input arrives                                               │
│       │                                                      │
│       ▼                                                      │
│  I/O Hook: onInput()                                        │
│       │                                                      │
│       ├── Check if checkpoint triggered                     │
│       │        │                                            │
│       │        ▼ (if yes)                                   │
│       │   Request context from CRA                          │
│       │   Inject into input                                 │
│       │                                                      │
│       ▼                                                      │
│  Agent processes (with injected context)                    │
│       │                                                      │
│       ▼                                                      │
│  I/O Hook: onOutput()                                       │
│       │                                                      │
│       ├── Enqueue TRACE event (async)                       │
│       │                                                      │
│       ▼                                                      │
│  Output returned                                             │
│                                                              │
│  [Background: TRACE queue flushes periodically]             │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Termination Phase

```
1. Session end triggered (explicit or timeout)
        │
        ▼
2. TRACE queue: sync flush (wait for completion)
        │
        ▼
3. CRA Client: endSession()
        │
        ▼
4. CRA finalizes hash chain
        │
        ▼
5. Wrapper deactivates
```

---

## Platform Implementations

### Claude Code

**Wrapper Location:** `.claude/hooks/` or MCP server

**I/O Hooks via:**
- `pre_tool_use` hook
- `post_tool_use` hook
- System prompt injection

**Example structure:**
```
.claude/
├── hooks/
│   └── cra-wrapper.js
├── settings.json (MCP config)
└── cra/
    ├── session.json
    └── trace/
```

---

### OpenAI API Agents

**Wrapper Location:** Middleware in API call chain

**I/O Hooks via:**
- Request interceptor
- Response interceptor
- Function call wrapper

**Example structure:**
```python
from cra import CRAWrapper

wrapper = CRAWrapper(config)

@wrapper.wrap
def call_openai(messages):
    return openai.chat.completions.create(messages=messages)
```

---

### LangChain / LlamaIndex

**Wrapper Location:** Callback handler or middleware

**I/O Hooks via:**
- `on_llm_start` / `on_llm_end`
- `on_tool_start` / `on_tool_end`
- `on_chain_start` / `on_chain_end`

**Example:**
```python
from cra.integrations.langchain import CRACallbackHandler

handler = CRACallbackHandler(config)
llm = ChatOpenAI(callbacks=[handler])
```

---

### Custom Agents

**Wrapper Location:** Decorator or context manager

**Example:**
```python
from cra import CRAWrapper

wrapper = CRAWrapper(config)

with wrapper.session("Build VIB3 website"):
    # All agent operations are wrapped
    result = agent.run(task)
```

---

## Extensibility Points

### 1. Plugin System

Plugins can extend wrapper functionality:

```typescript
interface CRAPlugin {
  name: string;
  version: string;

  // Called during wrapper construction
  onConstruct?(wrapper: CRAWrapper): void;

  // Add custom hooks
  hooks?: Partial<IOHooks>;

  // Add custom event types
  eventTypes?: EventTypeDefinition[];

  // Add custom cache backend
  cacheBackend?: CacheBackend;

  // Add custom transport
  transport?: TransportBackend;
}
```

**Example plugins:**
- `cra-plugin-langchain` - LangChain integration
- `cra-plugin-metrics` - Prometheus/Grafana metrics
- `cra-plugin-alerts` - Slack/Discord notifications
- `cra-plugin-replay` - Session replay functionality

---

### 2. Transport Backends

How wrapper communicates with CRA:

```typescript
interface TransportBackend {
  name: string;

  connect(config: ConnectionConfig): Promise<void>;
  disconnect(): Promise<void>;

  request<T>(method: string, params: unknown): Promise<T>;

  // For streaming/push
  subscribe?(event: string, handler: EventHandler): void;
}
```

**Built-in transports:**
- `mcp` - Model Context Protocol (default)
- `rest` - HTTP REST API
- `websocket` - WebSocket (for real-time)
- `direct` - Direct library call (same process)

**Future transports:**
- `grpc` - For high-performance
- `queue` - Message queue (RabbitMQ, etc.)

---

### 3. Cache Backends

Where context is cached:

```typescript
interface CacheBackend {
  name: string;

  get(key: string): Promise<CachedContext | null>;
  set(key: string, value: CachedContext, ttl?: number): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}
```

**Built-in backends:**
- `memory` - In-memory (default)
- `file` - File system

**Future backends:**
- `redis` - Redis
- `sqlite` - SQLite
- `indexeddb` - Browser IndexedDB

---

### 4. Hook Extensions

Custom hooks can be added:

```typescript
// Register custom hook point
wrapper.registerHookPoint('onCustomEvent', {
  description: 'Called when custom event occurs',
  async: true,
});

// Add handler
wrapper.addHook('onCustomEvent', async (event) => {
  // Custom handling
});
```

---

## Future: Claude Code Skills

CRA can be packaged as a Claude Code skill:

```json
{
  "name": "cra-governance",
  "description": "CRA governance and context injection",
  "version": "1.0.0",
  "type": "mcp",
  "server": {
    "command": "npx",
    "args": ["cra-mcp-server"]
  },
  "tools": [
    "cra_bootstrap",
    "cra_request_context",
    "cra_report_action",
    "cra_feedback"
  ],
  "hooks": {
    "pre_tool_use": ".claude/hooks/cra-pre.js",
    "post_tool_use": ".claude/hooks/cra-post.js"
  }
}
```

**Installation:**
```bash
claude skill install cra-governance
```

---

## Future: Marketplace Integration

Wrappers can integrate with atlas marketplace:

```typescript
interface MarketplaceIntegration {
  // Browse available atlases
  browse(query: string): Promise<AtlasListing[]>;

  // Install atlas
  install(atlasId: string): Promise<InstallResult>;

  // Update atlas
  update(atlasId: string): Promise<UpdateResult>;

  // Report usage (for creators)
  reportUsage(atlasId: string, metrics: UsageMetrics): Promise<void>;
}
```

---

## Verification Protocol

CRA verifies wrapper construction:

### 1. Component Check

```typescript
interface VerificationResult {
  valid: boolean;
  components: {
    ioHooks: ComponentStatus;
    traceQueue: ComponentStatus;
    contextCache: ComponentStatus;
    craClient: ComponentStatus;
  };
  errors: string[];
}
```

### 2. Functional Test

CRA sends test events to verify wrapper responds correctly:

```
CRA: Send test input
Wrapper: Intercepts, responds with confirmation
CRA: Send test context
Wrapper: Caches, confirms receipt
CRA: Request TRACE flush
Wrapper: Flushes, CRA verifies format
```

### 3. Hash Recording

Wrapper construction is recorded in TRACE:

```json
{
  "eventType": "wrapper_constructed",
  "timestamp": "2024-01-15T12:05:33Z",
  "sessionId": "sess-abc123",
  "payload": {
    "wrapperVersion": "1.0.0",
    "components": ["ioHooks", "traceQueue", "contextCache"],
    "plugins": ["cra-plugin-langchain"],
    "transport": "mcp",
    "cacheBackend": "memory"
  },
  "hash": "sha256:..."
}
```

---

## Configuration Schema

```json
{
  "wrapper": {
    "version": "1.0.0",

    "components": {
      "ioHooks": {
        "enabled": true,
        "interceptInput": true,
        "interceptOutput": true,
        "interceptActions": true
      },
      "traceQueue": {
        "enabled": true,
        "maxSize": 100,
        "flushIntervalMs": 5000,
        "syncEvents": ["policy_check", "session_end"]
      },
      "contextCache": {
        "enabled": true,
        "backend": "memory",
        "defaultTtlSeconds": 300
      }
    },

    "transport": {
      "type": "mcp",
      "config": {}
    },

    "plugins": [
      {
        "name": "cra-plugin-langchain",
        "config": {}
      }
    ]
  }
}
```

---

## Summary

The wrapper protocol is designed to be:

1. **Minimal by default** - Only required components
2. **Extensible** - Plugins, transports, backends
3. **Platform-agnostic** - Works with any agent environment
4. **Future-ready** - Skills, marketplace, integrations

Core components:
- I/O Hooks (intercept agent I/O)
- TRACE Queue (async event collection)
- Context Cache (avoid redundant fetches)
- CRA Client (communicate with CRA)

Extensibility points:
- Plugin system
- Transport backends
- Cache backends
- Hook extensions
- Claude Code skills
- Marketplace integration
