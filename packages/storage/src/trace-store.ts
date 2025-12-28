/**
 * Trace Store
 *
 * High-level API for managing TRACE telemetry records.
 */

import type { TRACEEvent } from '@cra/protocol';
import { MemoryStore } from './memory-store.js';
import { SQLiteStore } from './sqlite-store.js';
import type { TraceRecord, QueryOptions, QueryResult } from './types.js';

/**
 * Trace store configuration
 */
export interface TraceStoreConfig {
  /** Storage backend */
  backend: 'memory' | 'sqlite';

  /** SQLite database path (for sqlite backend) */
  dbPath?: string;

  /** Maximum traces to retain (for memory backend) */
  maxItems?: number;

  /** Retention period in days */
  retentionDays?: number;
}

/**
 * Trace store for managing TRACE telemetry
 */
export class TraceStore {
  private store: MemoryStore | SQLiteStore;
  private readonly retentionDays: number;

  constructor(config: TraceStoreConfig = { backend: 'memory' }) {
    this.retentionDays = config.retentionDays ?? 30;

    if (config.backend === 'sqlite') {
      this.store = new SQLiteStore({
        path: config.dbPath ?? ':memory:',
      });
    } else {
      this.store = new MemoryStore({
        maxItems: config.maxItems ?? 1000,
        ttlMs: this.retentionDays * 24 * 60 * 60 * 1000,
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
   * Create a new trace
   */
  async create(
    traceId: string,
    sessionId: string,
    initialEvents: TRACEEvent[] = []
  ): Promise<TraceRecord> {
    const now = new Date().toISOString();
    const record: TraceRecord = {
      trace_id: traceId,
      session_id: sessionId,
      events: initialEvents,
      created_at: now,
      updated_at: now,
      chain_verified: false,
      event_count: initialEvents.length,
    };

    await this.store.saveTrace(record);
    return record;
  }

  /**
   * Get a trace by ID
   */
  async get(traceId: string): Promise<TraceRecord | null> {
    return this.store.getTrace(traceId);
  }

  /**
   * Get events from a trace
   */
  async getEvents(traceId: string): Promise<TRACEEvent[]> {
    const record = await this.store.getTrace(traceId);
    return record?.events ?? [];
  }

  /**
   * Append events to a trace
   */
  async appendEvents(traceId: string, events: TRACEEvent[]): Promise<boolean> {
    return this.store.appendTraceEvents(traceId, events);
  }

  /**
   * Set chain verification status
   */
  async setVerified(traceId: string, verified: boolean): Promise<boolean> {
    const record = await this.store.getTrace(traceId);
    if (!record) return false;

    record.chain_verified = verified;
    record.updated_at = new Date().toISOString();
    await this.store.saveTrace(record);
    return true;
  }

  /**
   * Delete a trace
   */
  async delete(traceId: string): Promise<boolean> {
    return this.store.deleteTrace(traceId);
  }

  /**
   * List traces with optional filters
   */
  async list(options?: QueryOptions): Promise<QueryResult<TraceRecord>> {
    return this.store.listTraces(options);
  }

  /**
   * List traces for a specific session
   */
  async listBySession(sessionId: string): Promise<TraceRecord[]> {
    const result = await this.store.listTraces({ session_id: sessionId });
    return result.records;
  }

  /**
   * Get events filtered by type
   */
  async getEventsByType(
    traceId: string,
    eventTypes: string[]
  ): Promise<TRACEEvent[]> {
    const events = await this.getEvents(traceId);
    return events.filter(e => eventTypes.includes(e.event_type));
  }

  /**
   * Get events within a time range
   */
  async getEventsInRange(
    traceId: string,
    fromTime: string,
    toTime: string
  ): Promise<TRACEEvent[]> {
    const events = await this.getEvents(traceId);
    const from = new Date(fromTime).getTime();
    const to = new Date(toTime).getTime();

    return events.filter(e => {
      const ts = new Date(e.timestamp).getTime();
      return ts >= from && ts <= to;
    });
  }

  /**
   * Get span events
   */
  async getSpanEvents(traceId: string, spanId: string): Promise<TRACEEvent[]> {
    const events = await this.getEvents(traceId);
    return events.filter(e => e.span_id === spanId);
  }

  /**
   * Count events by type
   */
  async countEventsByType(
    traceId: string
  ): Promise<Map<string, number>> {
    const events = await this.getEvents(traceId);
    const counts = new Map<string, number>();

    for (const event of events) {
      const current = counts.get(event.event_type) ?? 0;
      counts.set(event.event_type, current + 1);
    }

    return counts;
  }

  /**
   * Get trace summary
   */
  async getSummary(traceId: string): Promise<{
    trace_id: string;
    event_count: number;
    span_count: number;
    duration_ms: number;
    event_types: string[];
    error_count: number;
  } | null> {
    const record = await this.store.getTrace(traceId);
    if (!record) return null;

    const events = record.events;
    if (events.length === 0) {
      return {
        trace_id: traceId,
        event_count: 0,
        span_count: 0,
        duration_ms: 0,
        event_types: [],
        error_count: 0,
      };
    }

    const spanIds = new Set(events.map(e => e.span_id).filter(Boolean));
    const eventTypes = [...new Set(events.map(e => e.event_type))];
    const errorCount = events.filter(e => e.severity === 'error').length;

    const timestamps = events.map(e => new Date(e.timestamp).getTime());
    const durationMs = Math.max(...timestamps) - Math.min(...timestamps);

    return {
      trace_id: traceId,
      event_count: events.length,
      span_count: spanIds.size,
      duration_ms: durationMs,
      event_types: eventTypes,
      error_count: errorCount,
    };
  }

  /**
   * Get storage statistics
   */
  async getStats(): Promise<{
    total: number;
    totalEvents: number;
    verified: number;
  }> {
    const result = await this.store.listTraces({ limit: 10000 });
    let totalEvents = 0;
    let verified = 0;

    for (const record of result.records) {
      totalEvents += record.event_count;
      if (record.chain_verified) verified++;
    }

    return { total: result.total, totalEvents, verified };
  }
}
