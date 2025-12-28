# @cra/server

HTTP/WebSocket server for CRA (Context Registry Agents).

## Features

- REST API for CARP resolve/execute operations
- Batch operations (up to 100 per request)
- SSE streaming for long-running resolutions
- WebSocket streaming for real-time TRACE events
- Health checks and agent discovery

## Installation

```bash
npm install @cra/server
```

## Usage

### Basic Server

```typescript
import { createServer } from '@cra/server';

const server = await createServer({
  port: 3000,
  enableWebSocket: true,
  enableCors: true,
});

await server.start();
console.log('Server running on http://localhost:3000');
```

### With Runtime and Storage

```typescript
import { createServer } from '@cra/server';
import { CRARuntime } from '@cra/runtime';
import { createStore } from '@cra/storage';

const runtime = new CRARuntime();
await runtime.loadAtlas('./atlases/github-ops');

const store = createStore({ type: 'file', basePath: './data' });

const server = await createServer({
  port: 3000,
  runtime,
  store,
  enableWebSocket: true,
});

await server.start();
```

## API Endpoints

### POST /v1/resolve

CARP resolution request.

```bash
curl -X POST http://localhost:3000/v1/resolve \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "my-agent",
    "session_id": "session-1",
    "task": {
      "goal": "Create a GitHub issue",
      "risk_tier": "low"
    }
  }'
```

### POST /v1/execute

Execute a permitted action.

```bash
curl -X POST http://localhost:3000/v1/execute \
  -H "Content-Type: application/json" \
  -d '{
    "resolution_id": "res-123",
    "action_type": "github.create_issue",
    "parameters": {
      "title": "Bug report",
      "body": "Description..."
    }
  }'
```

### POST /v1/batch

Batch operations (max 100).

```bash
curl -X POST http://localhost:3000/v1/batch \
  -H "Content-Type: application/json" \
  -d '{
    "operations": [
      { "type": "resolve", "request": { ... } },
      { "type": "execute", "request": { ... } }
    ]
  }'
```

### POST /v1/stream/resolve

SSE streaming resolution for long-running tasks.

```typescript
const eventSource = new EventSource('http://localhost:3000/v1/stream/resolve');
eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Progress:', data);
};
```

### GET /v1/discover

Agent discovery endpoint - returns capabilities and quick start info.

### GET /health

Health check endpoint.

### WS /v1/trace

WebSocket endpoint for real-time TRACE event streaming.

```typescript
const ws = new WebSocket('ws://localhost:3000/v1/trace');
ws.onmessage = (event) => {
  const traceEvent = JSON.parse(event.data);
  console.log('TRACE:', traceEvent.event_type);
};
```

## Configuration

```typescript
interface ServerConfig {
  port: number;
  host?: string;              // Default: '0.0.0.0'
  enableWebSocket?: boolean;  // Default: false
  enableCors?: boolean;       // Default: false
  runtime?: CRARuntime;
  store?: Store;
}
```

## License

MIT
