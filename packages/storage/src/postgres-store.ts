/**
 * PostgreSQL Store
 *
 * Persistent storage using PostgreSQL for production deployments.
 */

import { Pool, PoolClient, PoolConfig } from 'pg';
import type {
  Store,
  ResolutionRecord,
  SessionRecord,
  TraceRecord,
  QueryOptions,
  QueryResult,
} from './types.js';

/**
 * PostgreSQL store configuration
 */
export interface PostgresStoreConfig {
  /** Connection string (e.g., postgres://user:pass@host:5432/db) */
  connectionString?: string;

  /** Pool configuration */
  pool?: PoolConfig;

  /** Schema name (defaults to 'cra') */
  schema?: string;

  /** Enable debug logging */
  debug?: boolean;
}

/**
 * PostgreSQL storage implementation
 */
export class PostgresStore implements Store {
  private pool: Pool | null = null;
  private readonly config: Required<Omit<PostgresStoreConfig, 'connectionString' | 'pool'>> & PostgresStoreConfig;
  private ready = false;

  constructor(config: PostgresStoreConfig) {
    this.config = {
      schema: 'cra',
      debug: false,
      ...config,
    };
  }

  async init(): Promise<void> {
    // Create pool
    if (this.config.connectionString) {
      this.pool = new Pool({ connectionString: this.config.connectionString });
    } else if (this.config.pool) {
      this.pool = new Pool(this.config.pool);
    } else {
      this.pool = new Pool(); // Use environment variables
    }

    // Test connection
    const client = await this.pool.connect();
    try {
      // Create schema
      await client.query(`CREATE SCHEMA IF NOT EXISTS ${this.config.schema}`);

      // Create tables
      await client.query(`
        CREATE TABLE IF NOT EXISTS ${this.config.schema}.resolutions (
          resolution_id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          agent_id TEXT NOT NULL,
          resolution JSONB NOT NULL,
          created_at TIMESTAMPTZ NOT NULL,
          expires_at TIMESTAMPTZ NOT NULL,
          is_valid BOOLEAN NOT NULL DEFAULT true,
          use_count INTEGER NOT NULL DEFAULT 0,
          last_used_at TIMESTAMPTZ
        );

        CREATE INDEX IF NOT EXISTS idx_resolutions_session
          ON ${this.config.schema}.resolutions(session_id);
        CREATE INDEX IF NOT EXISTS idx_resolutions_agent
          ON ${this.config.schema}.resolutions(agent_id);
        CREATE INDEX IF NOT EXISTS idx_resolutions_expires
          ON ${this.config.schema}.resolutions(expires_at);
      `);

      await client.query(`
        CREATE TABLE IF NOT EXISTS ${this.config.schema}.sessions (
          session_id TEXT PRIMARY KEY,
          agent_id TEXT NOT NULL,
          state TEXT NOT NULL DEFAULT 'active',
          metadata JSONB NOT NULL DEFAULT '{}',
          created_at TIMESTAMPTZ NOT NULL,
          last_activity_at TIMESTAMPTZ NOT NULL,
          terminated_at TIMESTAMPTZ,
          resolution_count INTEGER NOT NULL DEFAULT 0,
          action_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_agent
          ON ${this.config.schema}.sessions(agent_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_state
          ON ${this.config.schema}.sessions(state);
      `);

      await client.query(`
        CREATE TABLE IF NOT EXISTS ${this.config.schema}.traces (
          trace_id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          span_id TEXT,
          events JSONB NOT NULL DEFAULT '[]',
          created_at TIMESTAMPTZ NOT NULL,
          updated_at TIMESTAMPTZ NOT NULL,
          chain_verified BOOLEAN NOT NULL DEFAULT false,
          event_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_traces_session
          ON ${this.config.schema}.traces(session_id);
      `);

      this.ready = true;

      if (this.config.debug) {
        console.log(`[PostgresStore] Initialized with schema: ${this.config.schema}`);
      }
    } finally {
      client.release();
    }
  }

  async close(): Promise<void> {
    if (this.pool) {
      await this.pool.end();
      this.pool = null;
    }
    this.ready = false;
  }

  isReady(): boolean {
    return this.ready && this.pool !== null;
  }

  private ensureReady(): Pool {
    if (!this.pool) {
      throw new Error('Store not initialized');
    }
    return this.pool;
  }

  private get table() {
    return {
      resolutions: `${this.config.schema}.resolutions`,
      sessions: `${this.config.schema}.sessions`,
      traces: `${this.config.schema}.traces`,
    };
  }

  // Resolution methods
  async saveResolution(record: ResolutionRecord): Promise<void> {
    const pool = this.ensureReady();
    await pool.query(
      `
      INSERT INTO ${this.table.resolutions}
      (resolution_id, session_id, agent_id, resolution, created_at, expires_at, is_valid, use_count, last_used_at)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      ON CONFLICT (resolution_id) DO UPDATE SET
        session_id = EXCLUDED.session_id,
        agent_id = EXCLUDED.agent_id,
        resolution = EXCLUDED.resolution,
        expires_at = EXCLUDED.expires_at,
        is_valid = EXCLUDED.is_valid,
        use_count = EXCLUDED.use_count,
        last_used_at = EXCLUDED.last_used_at
      `,
      [
        record.resolution_id,
        record.session_id,
        record.agent_id,
        JSON.stringify(record.resolution),
        record.created_at,
        record.expires_at,
        record.is_valid,
        record.use_count,
        record.last_used_at ?? null,
      ]
    );
  }

  async getResolution(resolutionId: string): Promise<ResolutionRecord | null> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `SELECT * FROM ${this.table.resolutions} WHERE resolution_id = $1`,
      [resolutionId]
    );
    if (result.rows.length === 0) return null;
    return this.rowToResolution(result.rows[0]);
  }

  async deleteResolution(resolutionId: string): Promise<boolean> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `DELETE FROM ${this.table.resolutions} WHERE resolution_id = $1`,
      [resolutionId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  async listResolutions(options: QueryOptions = {}): Promise<QueryResult<ResolutionRecord>> {
    const pool = this.ensureReady();

    const conditions: string[] = [];
    const params: unknown[] = [];
    let paramIndex = 1;

    if (options.session_id) {
      conditions.push(`session_id = $${paramIndex++}`);
      params.push(options.session_id);
    }
    if (options.agent_id) {
      conditions.push(`agent_id = $${paramIndex++}`);
      params.push(options.agent_id);
    }
    if (options.from_date) {
      conditions.push(`created_at >= $${paramIndex++}`);
      params.push(options.from_date);
    }
    if (options.to_date) {
      conditions.push(`created_at <= $${paramIndex++}`);
      params.push(options.to_date);
    }

    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
    const sortBy = options.sort_by ?? 'created_at';
    const sortOrder = options.sort_order ?? 'desc';
    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;

    // Get total count
    const countResult = await pool.query(
      `SELECT COUNT(*) as total FROM ${this.table.resolutions} ${whereClause}`,
      params
    );
    const total = parseInt(countResult.rows[0].total, 10);

    // Get records
    const result = await pool.query(
      `SELECT * FROM ${this.table.resolutions}
       ${whereClause}
       ORDER BY ${sortBy} ${sortOrder}
       LIMIT $${paramIndex++} OFFSET $${paramIndex}`,
      [...params, limit, offset]
    );

    const records = result.rows.map((row) => this.rowToResolution(row));

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async invalidateResolution(resolutionId: string): Promise<boolean> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `UPDATE ${this.table.resolutions} SET is_valid = false WHERE resolution_id = $1`,
      [resolutionId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  async incrementUseCount(resolutionId: string): Promise<boolean> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `UPDATE ${this.table.resolutions}
       SET use_count = use_count + 1, last_used_at = NOW()
       WHERE resolution_id = $1`,
      [resolutionId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  // Session methods
  async saveSession(record: SessionRecord): Promise<void> {
    const pool = this.ensureReady();
    await pool.query(
      `
      INSERT INTO ${this.table.sessions}
      (session_id, agent_id, state, metadata, created_at, last_activity_at, terminated_at, resolution_count, action_count)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      ON CONFLICT (session_id) DO UPDATE SET
        agent_id = EXCLUDED.agent_id,
        state = EXCLUDED.state,
        metadata = EXCLUDED.metadata,
        last_activity_at = EXCLUDED.last_activity_at,
        terminated_at = EXCLUDED.terminated_at,
        resolution_count = EXCLUDED.resolution_count,
        action_count = EXCLUDED.action_count
      `,
      [
        record.session_id,
        record.agent_id,
        record.state,
        JSON.stringify(record.metadata),
        record.created_at,
        record.last_activity_at,
        record.terminated_at ?? null,
        record.resolution_count,
        record.action_count,
      ]
    );
  }

  async getSession(sessionId: string): Promise<SessionRecord | null> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `SELECT * FROM ${this.table.sessions} WHERE session_id = $1`,
      [sessionId]
    );
    if (result.rows.length === 0) return null;
    return this.rowToSession(result.rows[0]);
  }

  async deleteSession(sessionId: string): Promise<boolean> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `DELETE FROM ${this.table.sessions} WHERE session_id = $1`,
      [sessionId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  async listSessions(options: QueryOptions = {}): Promise<QueryResult<SessionRecord>> {
    const pool = this.ensureReady();

    const conditions: string[] = [];
    const params: unknown[] = [];
    let paramIndex = 1;

    if (options.agent_id) {
      conditions.push(`agent_id = $${paramIndex++}`);
      params.push(options.agent_id);
    }

    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;

    // Get total count
    const countResult = await pool.query(
      `SELECT COUNT(*) as total FROM ${this.table.sessions} ${whereClause}`,
      params
    );
    const total = parseInt(countResult.rows[0].total, 10);

    // Get records
    const result = await pool.query(
      `SELECT * FROM ${this.table.sessions}
       ${whereClause}
       ORDER BY last_activity_at DESC
       LIMIT $${paramIndex++} OFFSET $${paramIndex}`,
      [...params, limit, offset]
    );

    const records = result.rows.map((row) => this.rowToSession(row));

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
    const pool = this.ensureReady();
    const result = await pool.query(
      `UPDATE ${this.table.sessions}
       SET state = $1,
           last_activity_at = NOW(),
           terminated_at = CASE WHEN $1 = 'terminated' THEN NOW() ELSE terminated_at END
       WHERE session_id = $2`,
      [state, sessionId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  async incrementSessionCounts(
    sessionId: string,
    resolutions: number,
    actions: number
  ): Promise<boolean> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `UPDATE ${this.table.sessions}
       SET resolution_count = resolution_count + $1,
           action_count = action_count + $2,
           last_activity_at = NOW()
       WHERE session_id = $3`,
      [resolutions, actions, sessionId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  // Trace methods
  async saveTrace(record: TraceRecord): Promise<void> {
    const pool = this.ensureReady();
    await pool.query(
      `
      INSERT INTO ${this.table.traces}
      (trace_id, session_id, span_id, events, created_at, updated_at, chain_verified, event_count)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      ON CONFLICT (trace_id) DO UPDATE SET
        session_id = EXCLUDED.session_id,
        span_id = EXCLUDED.span_id,
        events = EXCLUDED.events,
        updated_at = EXCLUDED.updated_at,
        chain_verified = EXCLUDED.chain_verified,
        event_count = EXCLUDED.event_count
      `,
      [
        record.trace_id,
        record.session_id,
        record.span_id ?? null,
        JSON.stringify(record.events),
        record.created_at,
        record.updated_at,
        record.chain_verified,
        record.event_count,
      ]
    );
  }

  async getTrace(traceId: string): Promise<TraceRecord | null> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `SELECT * FROM ${this.table.traces} WHERE trace_id = $1`,
      [traceId]
    );
    if (result.rows.length === 0) return null;
    return this.rowToTrace(result.rows[0]);
  }

  async deleteTrace(traceId: string): Promise<boolean> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `DELETE FROM ${this.table.traces} WHERE trace_id = $1`,
      [traceId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  async listTraces(options: QueryOptions = {}): Promise<QueryResult<TraceRecord>> {
    const pool = this.ensureReady();

    const conditions: string[] = [];
    const params: unknown[] = [];
    let paramIndex = 1;

    if (options.session_id) {
      conditions.push(`session_id = $${paramIndex++}`);
      params.push(options.session_id);
    }

    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;

    // Get total count
    const countResult = await pool.query(
      `SELECT COUNT(*) as total FROM ${this.table.traces} ${whereClause}`,
      params
    );
    const total = parseInt(countResult.rows[0].total, 10);

    // Get records
    const result = await pool.query(
      `SELECT * FROM ${this.table.traces}
       ${whereClause}
       ORDER BY updated_at DESC
       LIMIT $${paramIndex++} OFFSET $${paramIndex}`,
      [...params, limit, offset]
    );

    const records = result.rows.map((row) => this.rowToTrace(row));

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async appendTraceEvents(traceId: string, events: TraceRecord['events']): Promise<boolean> {
    const pool = this.ensureReady();

    // PostgreSQL JSONB array concatenation
    const result = await pool.query(
      `UPDATE ${this.table.traces}
       SET events = events || $1::jsonb,
           event_count = event_count + $2,
           updated_at = NOW()
       WHERE trace_id = $3`,
      [JSON.stringify(events), events.length, traceId]
    );
    return (result.rowCount ?? 0) > 0;
  }

  // Stats
  async getStats(): Promise<{
    resolutions: number;
    sessions: number;
    traces: number;
  }> {
    const pool = this.ensureReady();

    const [resResult, sessResult, traceResult] = await Promise.all([
      pool.query(`SELECT COUNT(*) as c FROM ${this.table.resolutions}`),
      pool.query(`SELECT COUNT(*) as c FROM ${this.table.sessions}`),
      pool.query(`SELECT COUNT(*) as c FROM ${this.table.traces}`),
    ]);

    return {
      resolutions: parseInt(resResult.rows[0].c, 10),
      sessions: parseInt(sessResult.rows[0].c, 10),
      traces: parseInt(traceResult.rows[0].c, 10),
    };
  }

  // Transaction support
  async withTransaction<T>(fn: (client: PoolClient) => Promise<T>): Promise<T> {
    const pool = this.ensureReady();
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const result = await fn(client);
      await client.query('COMMIT');
      return result;
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  // Bulk operations (PostgreSQL-specific optimization)
  async bulkSaveResolutions(records: ResolutionRecord[]): Promise<void> {
    if (records.length === 0) return;

    await this.withTransaction(async (client) => {
      for (const record of records) {
        await client.query(
          `INSERT INTO ${this.table.resolutions}
           (resolution_id, session_id, agent_id, resolution, created_at, expires_at, is_valid, use_count, last_used_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           ON CONFLICT (resolution_id) DO UPDATE SET
             resolution = EXCLUDED.resolution,
             is_valid = EXCLUDED.is_valid,
             use_count = EXCLUDED.use_count,
             last_used_at = EXCLUDED.last_used_at`,
          [
            record.resolution_id,
            record.session_id,
            record.agent_id,
            JSON.stringify(record.resolution),
            record.created_at,
            record.expires_at,
            record.is_valid,
            record.use_count,
            record.last_used_at ?? null,
          ]
        );
      }
    });
  }

  // Cleanup expired resolutions
  async cleanupExpired(): Promise<number> {
    const pool = this.ensureReady();
    const result = await pool.query(
      `DELETE FROM ${this.table.resolutions} WHERE expires_at < NOW()`
    );
    return result.rowCount ?? 0;
  }

  // Row conversion helpers
  private rowToResolution(row: Record<string, unknown>): ResolutionRecord {
    return {
      resolution_id: row.resolution_id as string,
      session_id: row.session_id as string,
      agent_id: row.agent_id as string,
      resolution: row.resolution as ResolutionRecord['resolution'],
      created_at: (row.created_at as Date).toISOString(),
      expires_at: (row.expires_at as Date).toISOString(),
      is_valid: row.is_valid as boolean,
      use_count: row.use_count as number,
      last_used_at: row.last_used_at
        ? (row.last_used_at as Date).toISOString()
        : undefined,
    };
  }

  private rowToSession(row: Record<string, unknown>): SessionRecord {
    return {
      session_id: row.session_id as string,
      agent_id: row.agent_id as string,
      state: row.state as SessionRecord['state'],
      metadata: row.metadata as Record<string, unknown>,
      created_at: (row.created_at as Date).toISOString(),
      last_activity_at: (row.last_activity_at as Date).toISOString(),
      terminated_at: row.terminated_at
        ? (row.terminated_at as Date).toISOString()
        : undefined,
      resolution_count: row.resolution_count as number,
      action_count: row.action_count as number,
    };
  }

  private rowToTrace(row: Record<string, unknown>): TraceRecord {
    return {
      trace_id: row.trace_id as string,
      session_id: row.session_id as string,
      span_id: row.span_id as string | undefined,
      events: row.events as TraceRecord['events'],
      created_at: (row.created_at as Date).toISOString(),
      updated_at: (row.updated_at as Date).toISOString(),
      chain_verified: row.chain_verified as boolean,
      event_count: row.event_count as number,
    };
  }
}
