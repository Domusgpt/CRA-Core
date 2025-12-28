/**
 * Resolution Store
 *
 * High-level API for managing CARP resolutions.
 */

import type { CARPResolution } from '@cra/protocol';
import { MemoryStore } from './memory-store.js';
import { SQLiteStore } from './sqlite-store.js';
import { PostgresStore } from './postgres-store.js';
import type { ResolutionRecord, QueryOptions, QueryResult } from './types.js';

/**
 * Resolution store configuration
 */
export interface ResolutionStoreConfig {
  /** Storage backend */
  backend: 'memory' | 'sqlite' | 'postgresql';

  /** SQLite database path (for sqlite backend) */
  dbPath?: string;

  /** PostgreSQL connection string (for postgresql backend) */
  connectionString?: string;

  /** PostgreSQL schema (for postgresql backend) */
  schema?: string;

  /** Default TTL in seconds */
  defaultTtlSeconds?: number;

  /** Maximum cached resolutions (for memory backend) */
  maxItems?: number;
}

/**
 * Resolution store for managing CARP resolutions
 */
export class ResolutionStore {
  private store: MemoryStore | SQLiteStore | PostgresStore;
  private readonly defaultTtlSeconds: number;

  constructor(config: ResolutionStoreConfig = { backend: 'memory' }) {
    this.defaultTtlSeconds = config.defaultTtlSeconds ?? 300; // 5 minutes

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
        ttlMs: this.defaultTtlSeconds * 1000,
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
   * Store a new resolution
   */
  async save(
    resolution: CARPResolution,
    options: {
      sessionId: string;
      agentId: string;
      ttlSeconds?: number;
    }
  ): Promise<ResolutionRecord> {
    const now = new Date();
    const ttl = options.ttlSeconds ?? this.defaultTtlSeconds;
    const expiresAt = new Date(now.getTime() + ttl * 1000);

    const record: ResolutionRecord = {
      resolution_id: resolution.resolution_id,
      session_id: options.sessionId,
      agent_id: options.agentId,
      resolution,
      created_at: now.toISOString(),
      expires_at: expiresAt.toISOString(),
      is_valid: true,
      use_count: 0,
    };

    await this.store.saveResolution(record);
    return record;
  }

  /**
   * Get a resolution by ID
   */
  async get(resolutionId: string): Promise<CARPResolution | null> {
    const record = await this.store.getResolution(resolutionId);
    if (!record) return null;

    // Check if expired
    if (new Date(record.expires_at) < new Date()) {
      await this.store.invalidateResolution(resolutionId);
      return null;
    }

    // Check if still valid
    if (!record.is_valid) {
      return null;
    }

    return record.resolution;
  }

  /**
   * Get full resolution record
   */
  async getRecord(resolutionId: string): Promise<ResolutionRecord | null> {
    return this.store.getResolution(resolutionId);
  }

  /**
   * Mark a resolution as used
   */
  async markUsed(resolutionId: string): Promise<boolean> {
    return this.store.incrementUseCount(resolutionId);
  }

  /**
   * Invalidate a resolution
   */
  async invalidate(resolutionId: string): Promise<boolean> {
    return this.store.invalidateResolution(resolutionId);
  }

  /**
   * Delete a resolution
   */
  async delete(resolutionId: string): Promise<boolean> {
    return this.store.deleteResolution(resolutionId);
  }

  /**
   * List resolutions with optional filters
   */
  async list(options?: QueryOptions): Promise<QueryResult<ResolutionRecord>> {
    return this.store.listResolutions(options);
  }

  /**
   * List resolutions for a specific session
   */
  async listBySession(sessionId: string): Promise<ResolutionRecord[]> {
    const result = await this.store.listResolutions({ session_id: sessionId });
    return result.records;
  }

  /**
   * Check if a resolution is valid for use
   */
  async isValid(resolutionId: string): Promise<boolean> {
    const record = await this.store.getResolution(resolutionId);
    if (!record) return false;
    if (!record.is_valid) return false;
    if (new Date(record.expires_at) < new Date()) return false;
    return true;
  }

  /**
   * Get storage statistics
   */
  async getStats(): Promise<{ total: number; valid: number; expired: number }> {
    const result = await this.store.listResolutions({ limit: 10000 });
    const now = new Date();
    let valid = 0;
    let expired = 0;

    for (const record of result.records) {
      if (!record.is_valid || new Date(record.expires_at) < now) {
        expired++;
      } else {
        valid++;
      }
    }

    return { total: result.total, valid, expired };
  }
}
