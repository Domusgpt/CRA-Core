# @cra/ui

Dual-mode web interface for CRA (Context Registry Agents).

## Features

- **Human Terminal Interface**: Interactive CLI-style interface for humans
- **Agent JSON API**: Structured data output optimized for LLM agents
- Real-time trace visualization via WebSocket
- Agent discovery and onboarding
- Automatic `agents.md` snippet generation

## Installation

```bash
npm install @cra/ui
```

## Usage

### Starting the UI Server

```typescript
import { UIServer } from '@cra/ui';
import { CRARuntime } from '@cra/runtime';
import { createStore } from '@cra/storage';

const runtime = new CRARuntime();
const store = createStore({ type: 'file', basePath: './data' });

const ui = new UIServer({
  port: 8080,
  runtime,
  store,
  enableWebSocket: true,
});

await ui.start();
console.log('UI available at http://localhost:8080');
```

### Configuration

```typescript
interface UIServerConfig {
  port: number;
  host?: string;              // Default: '0.0.0.0'
  runtime?: CRARuntime;
  store?: Store;
  enableWebSocket?: boolean;  // Default: true
}
```

## Human Terminal Interface

The terminal interface provides an interactive CLI-style experience in the browser.

### Available Commands

| Command | Description |
|---------|-------------|
| `help` | Show available commands |
| `discover` | Show CRA capabilities |
| `stats` | Display session statistics |
| `resolve <goal>` | Resolve context for a goal |
| `execute <action>` | Execute a permitted action |
| `traces` | List recent traces |
| `trace <id>` | View trace details |
| `clear` | Clear terminal |

### Keyboard Shortcuts

- **Up/Down Arrow**: Navigate command history
- **Ctrl+C**: Cancel current input
- **Ctrl+L**: Clear terminal

## Agent JSON API

The Agent API provides structured data optimized for LLM consumption.

```typescript
import { AgentAPI } from '@cra/ui';

const api = new AgentAPI(runtime, store);
```

### Dashboard Data

```typescript
const dashboard = api.getDashboardData();
// Returns:
// {
//   system_status: 'healthy',
//   active_sessions: 5,
//   total_resolutions: 150,
//   total_actions: 42,
//   recent_activity: [...],
//   available_atlases: ['github-ops', 'aws-ops'],
// }
```

### Terminal Data

```typescript
const terminal = api.getTerminalData();
// Returns available commands and their descriptions
```

### Traces Data

```typescript
const traces = api.getTracesData();
// Returns list of traces with summary info

const trace = api.getTraceData('trace-123');
// Returns detailed trace with all events
```

### Full Context (for agents.md)

```typescript
const context = api.getFullContext();
// Returns:
// {
//   agents_md_snippet: '## CRA Configuration\n...',
//   quick_start: ['POST /v1/resolve', 'POST /v1/execute'],
//   capabilities: [
//     { name: 'CARP Resolution', endpoint: '/v1/resolve' },
//     { name: 'Action Execution', endpoint: '/v1/execute' },
//   ],
//   available_actions: [...],
// }
```

## Trace Visualization

### Timeline View

Real-time trace events displayed in a timeline format:

```
▶ 10:30:45.123 session.started     info    Session initialized
▶ 10:30:45.234 carp.request.received info  Goal: Create issue
▶ 10:30:45.345 carp.resolution.completed info Decision: allow
```

### JSON View

Raw event data for debugging:

```json
{
  "event_id": "evt-123",
  "event_type": "carp.resolution.completed",
  "timestamp": "2025-01-15T10:30:45.345Z",
  "payload": {
    "decision": "allow",
    "context_blocks": 3
  }
}
```

### WebSocket Streaming

```typescript
const ws = new WebSocket('ws://localhost:8080/trace');
ws.onmessage = (event) => {
  const traceEvent = JSON.parse(event.data);
  // Handle real-time event
};
```

## Agent Onboarding

The UI provides paradigm-shifting agent onboarding:

1. **Single API Call Discovery**: Agents can understand CRA capabilities in one request
2. **Auto-Configuration**: Generates ready-to-use `agents.md` snippets
3. **Structured Responses**: All data formatted for optimal LLM comprehension

### Example: Agent Self-Configuration

```typescript
// Agent makes single discovery call
const response = await fetch('http://localhost:8080/api/agent/context');
const { agents_md_snippet, capabilities } = await response.json();

// Agent now has:
// - Full understanding of CRA capabilities
// - Ready-to-use configuration for agents.md
// - List of available actions and endpoints
```

## Components

### Terminal Component

```typescript
import { Terminal } from '@cra/ui';

const terminal = new Terminal({
  onCommand: async (cmd) => {
    // Handle command
    return { output: 'Result...' };
  },
});
```

### Trace Viewer Component

```typescript
import { TraceViewer } from '@cra/ui';

const viewer = new TraceViewer({
  events: traceEvents,
  mode: 'timeline',  // or 'json'
  live: true,        // Enable live updates
});
```

## License

MIT
