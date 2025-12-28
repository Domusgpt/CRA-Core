/**
 * Storage Factory
 *
 * Create storage instances with shared configuration.
 */

import { ResolutionStore, type ResolutionStoreConfig } from './resolution-store.js';
import { SessionStore, type SessionStoreConfig } from './session-store.js';
import { TraceStore, type TraceStoreConfig } from './trace-store.js';

/**
 * Options for creating a complete storage setup
 */
export interface CreateStoreOptions {
  /** Storage backend */
  backend: 'memory' | 'sqlite' | 'postgresql';

  /** SQLite database path (for sqlite backend) */
  dbPath?: string;

  /** PostgreSQL connection string (for postgresql backend) */
  connectionString?: string;

  /** PostgreSQL schema (for postgresql backend) */
  schema?: string;

  /** Resolution TTL in seconds */
  resolutionTtlSeconds?: number;

  /** Session idle timeout in seconds */
  sessionIdleTimeoutSeconds?: number;

  /** Trace retention in days */
  traceRetentionDays?: number;

  /** Maximum items per store (for memory backend) */
  maxItems?: number;
}

/**
 * Complete storage setup with all stores
 */
export interface StorageSetup {
  /** Resolution store */
  resolutions: ResolutionStore;

  /** Session store */
  sessions: SessionStore;

  /** Trace store */
  traces: TraceStore;

  /** Initialize all stores */
  init(): Promise<void>;

  /** Close all stores */
  close(): Promise<void>;
}

/**
 * Create a complete storage setup
 */
export function createStore(options: CreateStoreOptions): StorageSetup {
  const resolutionConfig: ResolutionStoreConfig = {
    backend: options.backend,
    dbPath: options.dbPath,
    connectionString: options.connectionString,
    schema: options.schema,
    defaultTtlSeconds: options.resolutionTtlSeconds,
    maxItems: options.maxItems,
  };

  const sessionConfig: SessionStoreConfig = {
    backend: options.backend,
    dbPath: options.dbPath,
    connectionString: options.connectionString,
    schema: options.schema,
    idleTimeoutSeconds: options.sessionIdleTimeoutSeconds,
    maxItems: options.maxItems,
  };

  const traceConfig: TraceStoreConfig = {
    backend: options.backend,
    dbPath: options.dbPath,
    connectionString: options.connectionString,
    schema: options.schema,
    retentionDays: options.traceRetentionDays,
    maxItems: options.maxItems,
  };

  const resolutions = new ResolutionStore(resolutionConfig);
  const sessions = new SessionStore(sessionConfig);
  const traces = new TraceStore(traceConfig);

  return {
    resolutions,
    sessions,
    traces,

    async init(): Promise<void> {
      await Promise.all([
        resolutions.init(),
        sessions.init(),
        traces.init(),
      ]);
    },

    async close(): Promise<void> {
      await Promise.all([
        resolutions.close(),
        sessions.close(),
        traces.close(),
      ]);
    },
  };
}
