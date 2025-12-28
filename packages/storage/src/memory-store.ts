/**
 * In-Memory Store
 *
 * Fast, ephemeral storage for development and testing.
 * Uses LRU cache for automatic eviction.
 */

import { LRUCache } from 'lru-cache';
import type {
  Store,
  ResolutionRecord,
  SessionRecord,
  TraceRecord,
  QueryOptions,
  QueryResult,
} from './types.js';

/**
 * Memory store configuration
 */
export interface MemoryStoreConfig {
  /** Maximum items per cache */
  maxItems?: number;

  /** Default TTL in milliseconds */
  ttlMs?: number;
}

/**
 * In-memory storage implementation
 */
export class MemoryStore implements Store {
  private resolutions: LRUCache<string, ResolutionRecord>;
  private sessions: LRUCache<string, SessionRecord>;
  private traces: LRUCache<string, TraceRecord>;
  private ready = false;

  constructor(config: MemoryStoreConfig = {}) {
    const maxItems = config.maxItems ?? 1000;
    const ttlMs = config.ttlMs ?? 1000 * 60 * 60; // 1 hour default

    this.resolutions = new LRUCache<string, ResolutionRecord>({
      max: maxItems,
      ttl: ttlMs,
    });

    this.sessions = new LRUCache<string, SessionRecord>({
      max: maxItems,
      ttl: ttlMs * 24, // Sessions live longer
    });

    this.traces = new LRUCache<string, TraceRecord>({
      max: maxItems,
      ttl: ttlMs * 24,
    });
  }

  async init(): Promise<void> {
    this.ready = true;
  }

  async close(): Promise<void> {
    this.resolutions.clear();
    this.sessions.clear();
    this.traces.clear();
    this.ready = false;
  }

  isReady(): boolean {
    return this.ready;
  }

  // Resolution methods
  async saveResolution(record: ResolutionRecord): Promise<void> {
    this.resolutions.set(record.resolution_id, record);
  }

  async getResolution(resolutionId: string): Promise<ResolutionRecord | null> {
    return this.resolutions.get(resolutionId) ?? null;
  }

  async deleteResolution(resolutionId: string): Promise<boolean> {
    return this.resolutions.delete(resolutionId);
  }

  async listResolutions(options: QueryOptions = {}): Promise<QueryResult<ResolutionRecord>> {
    let records = Array.from(this.resolutions.values());

    // Apply filters
    if (options.session_id) {
      records = records.filter(r => r.session_id === options.session_id);
    }
    if (options.agent_id) {
      records = records.filter(r => r.agent_id === options.agent_id);
    }

    // Sort
    const sortBy = options.sort_by ?? 'created_at';
    const sortOrder = options.sort_order ?? 'desc';
    records.sort((a, b) => {
      const aVal = (a as unknown as Record<string, string>)[sortBy] ?? '';
      const bVal = (b as unknown as Record<string, string>)[sortBy] ?? '';
      return sortOrder === 'asc' ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
    });

    const total = records.length;
    const offset = options.offset ?? 0;
    const limit = options.limit ?? 100;

    records = records.slice(offset, offset + limit);

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async invalidateResolution(resolutionId: string): Promise<boolean> {
    const record = this.resolutions.get(resolutionId);
    if (!record) return false;
    record.is_valid = false;
    this.resolutions.set(resolutionId, record);
    return true;
  }

  async incrementUseCount(resolutionId: string): Promise<boolean> {
    const record = this.resolutions.get(resolutionId);
    if (!record) return false;
    record.use_count++;
    record.last_used_at = new Date().toISOString();
    this.resolutions.set(resolutionId, record);
    return true;
  }

  // Session methods
  async saveSession(record: SessionRecord): Promise<void> {
    this.sessions.set(record.session_id, record);
  }

  async getSession(sessionId: string): Promise<SessionRecord | null> {
    return this.sessions.get(sessionId) ?? null;
  }

  async deleteSession(sessionId: string): Promise<boolean> {
    return this.sessions.delete(sessionId);
  }

  async listSessions(options: QueryOptions = {}): Promise<QueryResult<SessionRecord>> {
    let records = Array.from(this.sessions.values());

    if (options.agent_id) {
      records = records.filter(r => r.agent_id === options.agent_id);
    }

    const total = records.length;
    const offset = options.offset ?? 0;
    const limit = options.limit ?? 100;

    records = records.slice(offset, offset + limit);

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
    const record = this.sessions.get(sessionId);
    if (!record) return false;
    record.state = state;
    record.last_activity_at = new Date().toISOString();
    if (state === 'terminated') {
      record.terminated_at = new Date().toISOString();
    }
    this.sessions.set(sessionId, record);
    return true;
  }

  async incrementSessionCounts(
    sessionId: string,
    resolutions: number,
    actions: number
  ): Promise<boolean> {
    const record = this.sessions.get(sessionId);
    if (!record) return false;
    record.resolution_count += resolutions;
    record.action_count += actions;
    record.last_activity_at = new Date().toISOString();
    this.sessions.set(sessionId, record);
    return true;
  }

  // Trace methods
  async saveTrace(record: TraceRecord): Promise<void> {
    this.traces.set(record.trace_id, record);
  }

  async getTrace(traceId: string): Promise<TraceRecord | null> {
    return this.traces.get(traceId) ?? null;
  }

  async deleteTrace(traceId: string): Promise<boolean> {
    return this.traces.delete(traceId);
  }

  async listTraces(options: QueryOptions = {}): Promise<QueryResult<TraceRecord>> {
    let records = Array.from(this.traces.values());

    if (options.session_id) {
      records = records.filter(r => r.session_id === options.session_id);
    }

    const total = records.length;
    const offset = options.offset ?? 0;
    const limit = options.limit ?? 100;

    records = records.slice(offset, offset + limit);

    return {
      records,
      total,
      has_more: offset + records.length < total,
    };
  }

  async appendTraceEvents(traceId: string, events: TraceRecord['events']): Promise<boolean> {
    const record = this.traces.get(traceId);
    if (!record) return false;
    record.events.push(...events);
    record.event_count = record.events.length;
    record.updated_at = new Date().toISOString();
    this.traces.set(traceId, record);
    return true;
  }

  // Stats
  async getStats(): Promise<{
    resolutions: number;
    sessions: number;
    traces: number;
  }> {
    return {
      resolutions: this.resolutions.size,
      sessions: this.sessions.size,
      traces: this.traces.size,
    };
  }
}
