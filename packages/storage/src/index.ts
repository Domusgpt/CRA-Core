/**
 * CRA Storage Package
 *
 * Persistence layer for resolutions, traces, and sessions.
 * Supports multiple backends: in-memory, SQLite, PostgreSQL.
 */

// Base interfaces
export type {
  Store,
  StoreConfig,
  ResolutionRecord,
  SessionRecord,
  TraceRecord,
} from './types.js';

// Store implementations
export { MemoryStore } from './memory-store.js';
export { SQLiteStore } from './sqlite-store.js';
export { PostgresStore, type PostgresStoreConfig } from './postgres-store.js';

// Resolution store
export { ResolutionStore, type ResolutionStoreConfig } from './resolution-store.js';

// Session store
export { SessionStore, type SessionStoreConfig } from './session-store.js';

// Trace store
export { TraceStore, type TraceStoreConfig } from './trace-store.js';

// Factory
export { createStore, type CreateStoreOptions } from './factory.js';
