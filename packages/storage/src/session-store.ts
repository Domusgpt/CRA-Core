/**
 * Session Store
 *
 * High-level API for managing agent sessions.
 */

import { MemoryStore } from './memory-store.js';
import { SQLiteStore } from './sqlite-store.js';
import { PostgresStore } from './postgres-store.js';
import type { SessionRecord, QueryOptions, QueryResult } from './types.js';

/**
 * Session store configuration
 */
export interface SessionStoreConfig {
  /** Storage backend */
  backend: 'memory' | 'sqlite' | 'postgresql';

  /** SQLite database path (for sqlite backend) */
  dbPath?: string;

  /** PostgreSQL connection string (for postgresql backend) */
  connectionString?: string;

  /** PostgreSQL schema (for postgresql backend) */
  schema?: string;

  /** Session idle timeout in seconds */
  idleTimeoutSeconds?: number;

  /** Maximum sessions (for memory backend) */
  maxItems?: number;
}

/**
 * Session store for managing agent sessions
 */
export class SessionStore {
  private store: MemoryStore | SQLiteStore | PostgresStore;
  private readonly idleTimeoutSeconds: number;

  constructor(config: SessionStoreConfig = { backend: 'memory' }) {
    this.idleTimeoutSeconds = config.idleTimeoutSeconds ?? 3600; // 1 hour

    if (config.backend === 'postgresql') {
      this.store = new PostgresStore({
        connectionString: config.connectionString,
        schema: config.schema,
      });
    } else if (config.backend === 'sqlite') {
      this.store = new SQLiteStore({
        path: config.dbPath ?? ':memory:',
      });
    } else {
      this.store = new MemoryStore({
        maxItems: config.maxItems ?? 1000,
        ttlMs: this.idleTimeoutSeconds * 1000,
      });
    }
  }

  async init(): Promise<void> {
    await this.store.init();
  }

  async close(): Promise<void> {
    await this.store.close();
  }

  /**
   * Create a new session
   */
  async create(
    sessionId: string,
    agentId: string,
    metadata: Record<string, unknown> = {}
  ): Promise<SessionRecord> {
    const now = new Date().toISOString();
    const record: SessionRecord = {
      session_id: sessionId,
      agent_id: agentId,
      state: 'active',
      metadata,
      created_at: now,
      last_activity_at: now,
      resolution_count: 0,
      action_count: 0,
    };

    await this.store.saveSession(record);
    return record;
  }

  /**
   * Get a session by ID
   */
  async get(sessionId: string): Promise<SessionRecord | null> {
    return this.store.getSession(sessionId);
  }

  /**
   * Update session metadata
   */
  async updateMetadata(
    sessionId: string,
    metadata: Record<string, unknown>
  ): Promise<boolean> {
    const record = await this.store.getSession(sessionId);
    if (!record) return false;

    record.metadata = { ...record.metadata, ...metadata };
    record.last_activity_at = new Date().toISOString();
    await this.store.saveSession(record);
    return true;
  }

  /**
   * Mark session as active
   */
  async activate(sessionId: string): Promise<boolean> {
    return this.store.updateSessionState(sessionId, 'active');
  }

  /**
   * Mark session as idle
   */
  async setIdle(sessionId: string): Promise<boolean> {
    return this.store.updateSessionState(sessionId, 'idle');
  }

  /**
   * Terminate a session
   */
  async terminate(sessionId: string): Promise<boolean> {
    return this.store.updateSessionState(sessionId, 'terminated');
  }

  /**
   * Record a resolution in this session
   */
  async recordResolution(sessionId: string): Promise<boolean> {
    return this.store.incrementSessionCounts(sessionId, 1, 0);
  }

  /**
   * Record an action in this session
   */
  async recordAction(sessionId: string): Promise<boolean> {
    return this.store.incrementSessionCounts(sessionId, 0, 1);
  }

  /**
   * Delete a session
   */
  async delete(sessionId: string): Promise<boolean> {
    return this.store.deleteSession(sessionId);
  }

  /**
   * List sessions with optional filters
   */
  async list(options?: QueryOptions): Promise<QueryResult<SessionRecord>> {
    return this.store.listSessions(options);
  }

  /**
   * List active sessions for an agent
   */
  async listActiveByAgent(agentId: string): Promise<SessionRecord[]> {
    const result = await this.store.listSessions({ agent_id: agentId });
    return result.records.filter(r => r.state === 'active');
  }

  /**
   * Check if session is active
   */
  async isActive(sessionId: string): Promise<boolean> {
    const record = await this.store.getSession(sessionId);
    if (!record) return false;
    if (record.state !== 'active') return false;

    // Check idle timeout
    const lastActivity = new Date(record.last_activity_at);
    const now = new Date();
    const idleMs = now.getTime() - lastActivity.getTime();

    if (idleMs > this.idleTimeoutSeconds * 1000) {
      await this.setIdle(sessionId);
      return false;
    }

    return true;
  }

  /**
   * Touch session to update last activity
   */
  async touch(sessionId: string): Promise<boolean> {
    const record = await this.store.getSession(sessionId);
    if (!record) return false;

    record.last_activity_at = new Date().toISOString();
    if (record.state === 'idle') {
      record.state = 'active';
    }
    await this.store.saveSession(record);
    return true;
  }

  /**
   * Get session statistics
   */
  async getStats(): Promise<{
    total: number;
    active: number;
    idle: number;
    terminated: number;
  }> {
    const result = await this.store.listSessions({ limit: 10000 });
    let active = 0;
    let idle = 0;
    let terminated = 0;

    for (const record of result.records) {
      switch (record.state) {
        case 'active':
          active++;
          break;
        case 'idle':
          idle++;
          break;
        case 'terminated':
          terminated++;
          break;
      }
    }

    return { total: result.total, active, idle, terminated };
  }
}
