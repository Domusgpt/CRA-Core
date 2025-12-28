/**
 * SQLite Store
 *
 * Persistent storage using SQLite for local development and single-node deployments.
 */

import Database from 'better-sqlite3';
import type {
  Store,
  ResolutionRecord,
  SessionRecord,
  TraceRecord,
  QueryOptions,
  QueryResult,
} from './types.js';

/**
 * SQLite store configuration
 */
export interface SQLiteStoreConfig {
  /** Database file path (use ':memory:' for in-memory) */
  path: string;

  /** Enable WAL mode for better concurrency */
  walMode?: boolean;

  /** Enable debug logging */
  debug?: boolean;
}

/**
 * SQLite storage implementation
 */
export class SQLiteStore implements Store {
  private db: Database.Database | null = null;
  private readonly config: SQLiteStoreConfig;
  private ready = false;

  constructor(config: SQLiteStoreConfig) {
    this.config = {
      walMode: true,
      debug: false,
      ...config,
    };
  }

  async init(): Promise<void> {
    this.db = new Database(this.config.path);

    if (this.config.walMode) {
      this.db.pragma('journal_mode = WAL');
    }

    // Create tables
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS resolutions (
        resolution_id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        resolution TEXT NOT NULL,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        is_valid INTEGER NOT NULL DEFAULT 1,
        use_count INTEGER NOT NULL DEFAULT 0,
        last_used_at TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_resolutions_session ON resolutions(session_id);
      CREATE INDEX IF NOT EXISTS idx_resolutions_agent ON resolutions(agent_id);
      CREATE INDEX IF NOT EXISTS idx_resolutions_expires ON resolutions(expires_at);

      CREATE TABLE IF NOT EXISTS sessions (
        session_id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        state TEXT NOT NULL DEFAULT 'active',
        metadata TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        last_activity_at TEXT NOT NULL,
        terminated_at TEXT,
        resolution_count INTEGER NOT NULL DEFAULT 0,
        action_count INTEGER NOT NULL DEFAULT 0
      );

      CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_id);
      CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(state);

      CREATE TABLE IF NOT EXISTS traces (
        trace_id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        span_id TEXT,
        events TEXT NOT NULL DEFAULT '[]',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        chain_verified INTEGER NOT NULL DEFAULT 0,
        event_count INTEGER NOT NULL DEFAULT 0
      );

      CREATE INDEX IF NOT EXISTS idx_traces_session ON traces(session_id);
    `);

    this.ready = true;
  }

  async close(): Promise<void> {
    if (this.db) {
      this.db.close();
      this.db = null;
    }
    this.ready = false;
  }

  isReady(): boolean {
    return this.ready && this.db !== null;
  }

  private ensureReady(): Database.Database {
    if (!this.db) {
      throw new Error('Store not initialized');
    }
    return this.db;
  }

  // Resolution methods
  async saveResolution(record: ResolutionRecord): Promise<void> {
    const db = this.ensureReady();
    const stmt = db.prepare(`
      INSERT OR REPLACE INTO resolutions
      (resolution_id, session_id, agent_id, resolution, created_at, expires_at, is_valid, use_count, last_used_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);
    stmt.run(
      record.resolution_id,
      record.session_id,
      record.agent_id,
      JSON.stringify(record.resolution),
      record.created_at,
      record.expires_at,
      record.is_valid ? 1 : 0,
      record.use_count,
      record.last_used_at ?? null
    );
  }

  async getResolution(resolutionId: string): Promise<ResolutionRecord | null> {
    const db = this.ensureReady();
    const stmt = db.prepare('SELECT * FROM resolutions WHERE resolution_id = ?');
    const row = stmt.get(resolutionId) as Record<string, unknown> | undefined;
    if (!row) return null;
    return this.rowToResolution(row);
  }

  async deleteResolution(resolutionId: string): Promise<boolean> {
    const db = this.ensureReady();
    const stmt = db.prepare('DELETE FROM resolutions WHERE resolution_id = ?');
    const result = stmt.run(resolutionId);
    return result.changes > 0;
  }

  async listResolutions(options: QueryOptions = {}): Promise<QueryResult<ResolutionRecord>> {
    const db = this.ensureReady();

    let whereClause = '1=1';
    const params: unknown[] = [];

    if (options.session_id) {
      whereClause += ' AND session_id = ?';
      params.push(options.session_id);
    }
    if (options.agent_id) {
      whereClause += ' AND agent_id = ?';
      params.push(options.agent_id);
    }

    const sortBy = options.sort_by ?? 'created_at';
    const sortOrder = options.sort_order ?? 'desc';
    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;

    const countStmt = db.prepare(`SELECT COUNT(*) as total FROM resolutions WHERE ${whereClause}`);
    const countResult = countStmt.get(...params) as { total: number };
    const total = countResult.total;

    const stmt = db.prepare(`
      SELECT * FROM resolutions
      WHERE ${whereClause}
      ORDER BY ${sortBy} ${sortOrder}
      LIMIT ? OFFSET ?
    `);
    const rows = stmt.all(...params, limit, offset) as Record<string, unknown>[];
    const records = rows.map(row => this.rowToResolution(row));

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async invalidateResolution(resolutionId: string): Promise<boolean> {
    const db = this.ensureReady();
    const stmt = db.prepare('UPDATE resolutions SET is_valid = 0 WHERE resolution_id = ?');
    const result = stmt.run(resolutionId);
    return result.changes > 0;
  }

  async incrementUseCount(resolutionId: string): Promise<boolean> {
    const db = this.ensureReady();
    const stmt = db.prepare(`
      UPDATE resolutions
      SET use_count = use_count + 1, last_used_at = ?
      WHERE resolution_id = ?
    `);
    const result = stmt.run(new Date().toISOString(), resolutionId);
    return result.changes > 0;
  }

  // Session methods
  async saveSession(record: SessionRecord): Promise<void> {
    const db = this.ensureReady();
    const stmt = db.prepare(`
      INSERT OR REPLACE INTO sessions
      (session_id, agent_id, state, metadata, created_at, last_activity_at, terminated_at, resolution_count, action_count)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);
    stmt.run(
      record.session_id,
      record.agent_id,
      record.state,
      JSON.stringify(record.metadata),
      record.created_at,
      record.last_activity_at,
      record.terminated_at ?? null,
      record.resolution_count,
      record.action_count
    );
  }

  async getSession(sessionId: string): Promise<SessionRecord | null> {
    const db = this.ensureReady();
    const stmt = db.prepare('SELECT * FROM sessions WHERE session_id = ?');
    const row = stmt.get(sessionId) as Record<string, unknown> | undefined;
    if (!row) return null;
    return this.rowToSession(row);
  }

  async deleteSession(sessionId: string): Promise<boolean> {
    const db = this.ensureReady();
    const stmt = db.prepare('DELETE FROM sessions WHERE session_id = ?');
    const result = stmt.run(sessionId);
    return result.changes > 0;
  }

  async listSessions(options: QueryOptions = {}): Promise<QueryResult<SessionRecord>> {
    const db = this.ensureReady();

    let whereClause = '1=1';
    const params: unknown[] = [];

    if (options.agent_id) {
      whereClause += ' AND agent_id = ?';
      params.push(options.agent_id);
    }

    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;

    const countStmt = db.prepare(`SELECT COUNT(*) as total FROM sessions WHERE ${whereClause}`);
    const countResult = countStmt.get(...params) as { total: number };
    const total = countResult.total;

    const stmt = db.prepare(`
      SELECT * FROM sessions
      WHERE ${whereClause}
      ORDER BY last_activity_at DESC
      LIMIT ? OFFSET ?
    `);
    const rows = stmt.all(...params, limit, offset) as Record<string, unknown>[];
    const records = rows.map(row => this.rowToSession(row));

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async updateSessionState(
    sessionId: string,
    state: SessionRecord['state']
  ): Promise<boolean> {
    const db = this.ensureReady();
    const now = new Date().toISOString();
    const terminatedAt = state === 'terminated' ? now : null;
    const stmt = db.prepare(`
      UPDATE sessions
      SET state = ?, last_activity_at = ?, terminated_at = COALESCE(?, terminated_at)
      WHERE session_id = ?
    `);
    const result = stmt.run(state, now, terminatedAt, sessionId);
    return result.changes > 0;
  }

  async incrementSessionCounts(
    sessionId: string,
    resolutions: number,
    actions: number
  ): Promise<boolean> {
    const db = this.ensureReady();
    const stmt = db.prepare(`
      UPDATE sessions
      SET resolution_count = resolution_count + ?,
          action_count = action_count + ?,
          last_activity_at = ?
      WHERE session_id = ?
    `);
    const result = stmt.run(resolutions, actions, new Date().toISOString(), sessionId);
    return result.changes > 0;
  }

  // Trace methods
  async saveTrace(record: TraceRecord): Promise<void> {
    const db = this.ensureReady();
    const stmt = db.prepare(`
      INSERT OR REPLACE INTO traces
      (trace_id, session_id, span_id, events, created_at, updated_at, chain_verified, event_count)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `);
    stmt.run(
      record.trace_id,
      record.session_id,
      record.span_id ?? null,
      JSON.stringify(record.events),
      record.created_at,
      record.updated_at,
      record.chain_verified ? 1 : 0,
      record.event_count
    );
  }

  async getTrace(traceId: string): Promise<TraceRecord | null> {
    const db = this.ensureReady();
    const stmt = db.prepare('SELECT * FROM traces WHERE trace_id = ?');
    const row = stmt.get(traceId) as Record<string, unknown> | undefined;
    if (!row) return null;
    return this.rowToTrace(row);
  }

  async deleteTrace(traceId: string): Promise<boolean> {
    const db = this.ensureReady();
    const stmt = db.prepare('DELETE FROM traces WHERE trace_id = ?');
    const result = stmt.run(traceId);
    return result.changes > 0;
  }

  async listTraces(options: QueryOptions = {}): Promise<QueryResult<TraceRecord>> {
    const db = this.ensureReady();

    let whereClause = '1=1';
    const params: unknown[] = [];

    if (options.session_id) {
      whereClause += ' AND session_id = ?';
      params.push(options.session_id);
    }

    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;

    const countStmt = db.prepare(`SELECT COUNT(*) as total FROM traces WHERE ${whereClause}`);
    const countResult = countStmt.get(...params) as { total: number };
    const total = countResult.total;

    const stmt = db.prepare(`
      SELECT * FROM traces
      WHERE ${whereClause}
      ORDER BY updated_at DESC
      LIMIT ? OFFSET ?
    `);
    const rows = stmt.all(...params, limit, offset) as Record<string, unknown>[];
    const records = rows.map(row => this.rowToTrace(row));

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async appendTraceEvents(traceId: string, events: TraceRecord['events']): Promise<boolean> {
    this.ensureReady();
    const existing = await this.getTrace(traceId);
    if (!existing) return false;

    existing.events.push(...events);
    existing.event_count = existing.events.length;
    existing.updated_at = new Date().toISOString();

    await this.saveTrace(existing);
    return true;
  }

  // Stats
  async getStats(): Promise<{
    resolutions: number;
    sessions: number;
    traces: number;
  }> {
    const db = this.ensureReady();
    const resolutions = (db.prepare('SELECT COUNT(*) as c FROM resolutions').get() as { c: number }).c;
    const sessions = (db.prepare('SELECT COUNT(*) as c FROM sessions').get() as { c: number }).c;
    const traces = (db.prepare('SELECT COUNT(*) as c FROM traces').get() as { c: number }).c;
    return { resolutions, sessions, traces };
  }

  // Row conversion helpers
  private rowToResolution(row: Record<string, unknown>): ResolutionRecord {
    return {
      resolution_id: row.resolution_id as string,
      session_id: row.session_id as string,
      agent_id: row.agent_id as string,
      resolution: JSON.parse(row.resolution as string),
      created_at: row.created_at as string,
      expires_at: row.expires_at as string,
      is_valid: row.is_valid === 1,
      use_count: row.use_count as number,
      last_used_at: row.last_used_at as string | undefined,
    };
  }

  private rowToSession(row: Record<string, unknown>): SessionRecord {
    return {
      session_id: row.session_id as string,
      agent_id: row.agent_id as string,
      state: row.state as SessionRecord['state'],
      metadata: JSON.parse(row.metadata as string),
      created_at: row.created_at as string,
      last_activity_at: row.last_activity_at as string,
      terminated_at: row.terminated_at as string | undefined,
      resolution_count: row.resolution_count as number,
      action_count: row.action_count as number,
    };
  }

  private rowToTrace(row: Record<string, unknown>): TraceRecord {
    return {
      trace_id: row.trace_id as string,
      session_id: row.session_id as string,
      span_id: row.span_id as string | undefined,
      events: JSON.parse(row.events as string),
      created_at: row.created_at as string,
      updated_at: row.updated_at as string,
      chain_verified: row.chain_verified === 1,
      event_count: row.event_count as number,
    };
  }
}
