# @cra/storage

Pluggable persistence layer for CRA (Context Registry Agents).

## Features

- Resolution storage with TTL management
- Session state persistence
- TRACE event storage
- Multiple backends: Memory, File, PostgreSQL
- Transaction support (PostgreSQL)
- Bulk operations

## Installation

```bash
npm install @cra/storage

# For PostgreSQL support
npm install pg
```

## Usage

### Memory Store (Testing)

```typescript
import { createStore } from '@cra/storage';

const store = createStore({ type: 'memory' });
```

### File Store

```typescript
import { createStore } from '@cra/storage';

const store = createStore({
  type: 'file',
  basePath: './data',
});
```

### PostgreSQL Store

```typescript
import { createStore } from '@cra/storage';

const store = createStore({
  type: 'postgresql',
  connectionString: 'postgresql://user:pass@localhost:5432/cra',
});

// Or with options
const store = createStore({
  type: 'postgresql',
  host: 'localhost',
  port: 5432,
  database: 'cra',
  user: 'user',
  password: 'pass',
  ssl: true,
});
```

## API

### Resolutions

```typescript
// Save a resolution
await store.saveResolution({
  resolution_id: 'res-123',
  request_id: 'req-456',
  agent_id: 'my-agent',
  session_id: 'session-1',
  decision: 'allow',
  context_blocks: [...],
  allowed_actions: [...],
  created_at: new Date().toISOString(),
  expires_at: new Date(Date.now() + 3600000).toISOString(),
});

// Get a resolution
const resolution = await store.getResolution('res-123');

// List resolutions with filters
const resolutions = await store.listResolutions({
  agent_id: 'my-agent',
  session_id: 'session-1',
  limit: 100,
});

// Delete a resolution
await store.deleteResolution('res-123');
```

### Sessions

```typescript
// Save a session
await store.saveSession({
  session_id: 'session-1',
  agent_id: 'my-agent',
  status: 'active',
  created_at: new Date().toISOString(),
  metadata: { ... },
});

// Get a session
const session = await store.getSession('session-1');

// Update session status
await store.updateSession('session-1', { status: 'ended' });

// List sessions
const sessions = await store.listSessions({ agent_id: 'my-agent' });
```

### Traces

```typescript
// Save TRACE events
await store.saveTraceEvents([event1, event2, event3]);

// Get events for a trace
const events = await store.getTraceEvents('trace-123');

// Get events for a session
const sessionEvents = await store.getSessionTraceEvents('session-1');
```

### PostgreSQL-Specific Features

```typescript
import { PostgresStore } from '@cra/storage';

const store = new PostgresStore({
  connectionString: 'postgresql://...',
});

// Transaction support
await store.withTransaction(async (client) => {
  await store.saveResolution(resolution1, client);
  await store.saveResolution(resolution2, client);
});

// Bulk operations
await store.bulkSaveResolutions([res1, res2, res3]);

// Cleanup expired resolutions
const deletedCount = await store.cleanupExpired();
```

## Database Schema (PostgreSQL)

The PostgreSQL store automatically creates the following tables:

```sql
CREATE TABLE resolutions (
  resolution_id TEXT PRIMARY KEY,
  request_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  decision TEXT NOT NULL,
  context_blocks JSONB,
  allowed_actions JSONB,
  denied_actions JSONB,
  policies_applied JSONB,
  created_at TIMESTAMPTZ NOT NULL,
  expires_at TIMESTAMPTZ,
  metadata JSONB
);

CREATE TABLE sessions (
  session_id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  ended_at TIMESTAMPTZ,
  metadata JSONB
);

CREATE TABLE trace_events (
  event_id TEXT PRIMARY KEY,
  trace_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  sequence INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  payload JSONB,
  event_hash TEXT NOT NULL
);
```

## Configuration

```typescript
interface StoreConfig {
  type: 'memory' | 'file' | 'postgresql';

  // File store options
  basePath?: string;

  // PostgreSQL options
  connectionString?: string;
  host?: string;
  port?: number;
  database?: string;
  user?: string;
  password?: string;
  ssl?: boolean;
  max?: number;  // Connection pool size
}
```

## License

MIT
