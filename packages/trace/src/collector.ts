/**
 * TRACE Collector
 *
 * Append-only event collection with hash chain integrity.
 * Rule: If it wasn't emitted by the runtime, it didn't happen.
 */

import { EventEmitter } from 'events';
import * as fs from 'fs';
import * as path from 'path';
import type {
  TRACEEvent,
  TRACEEventType,
  Severity,
  ArtifactReference,
  Span,
  SpanStatus,
  EventSource,
} from '@cra/protocol';
import {
  createEvent,
  createSpan,
  completeSpan,
  generateId,
  getTimestamp,
  toJsonl,
  fromJsonl,
  verifyChain,
  resetSequence,
} from '@cra/protocol';

export interface CollectorOptions {
  /** Session ID for this collector */
  session_id: string;
  /** Trace ID (optional, generated if not provided) */
  trace_id?: string;
  /** Output directory for trace files */
  output_dir?: string;
  /** Output file name (optional, auto-generated) */
  output_file?: string;
  /** Enable file output */
  file_output?: boolean;
  /** Event source information */
  source?: EventSource;
  /** Buffer size before flush */
  buffer_size?: number;
  /** Flush interval in ms */
  flush_interval_ms?: number;
}

export interface SpanContext {
  span: Span;
  events: TRACEEvent[];
}

/**
 * TRACE Collector
 *
 * Collects and streams TRACE events with integrity guarantees.
 */
export class TRACECollector extends EventEmitter {
  private readonly sessionId: string;
  private readonly traceId: string;
  private readonly source: EventSource;
  private readonly outputDir?: string;
  private readonly outputFile?: string;
  private readonly fileOutput: boolean;
  private readonly bufferSize: number;
  private readonly flushInterval: number;

  private events: TRACEEvent[] = [];
  private buffer: TRACEEvent[] = [];
  private spans: Map<string, SpanContext> = new Map();
  private lastEventHash?: string;
  private flushTimer?: NodeJS.Timeout;
  private fileHandle?: fs.promises.FileHandle;
  private closed = false;

  constructor(options: CollectorOptions) {
    super();
    this.sessionId = options.session_id;
    this.traceId = options.trace_id ?? generateId();
    this.source = options.source ?? {
      component: 'cra.runtime',
      version: '0.1.0',
    };
    this.outputDir = options.output_dir;
    this.outputFile = options.output_file;
    this.fileOutput = options.file_output ?? false;
    this.bufferSize = options.buffer_size ?? 100;
    this.flushInterval = options.flush_interval_ms ?? 1000;

    // Reset sequence counter for this session
    resetSequence(this.sessionId);

    // Start flush timer
    if (this.flushInterval > 0) {
      this.flushTimer = setInterval(() => this.flush(), this.flushInterval);
    }
  }

  /**
   * Get the trace ID
   */
  getTraceId(): string {
    return this.traceId;
  }

  /**
   * Get the session ID
   */
  getSessionId(): string {
    return this.sessionId;
  }

  /**
   * Record a TRACE event (renamed from emit to avoid EventEmitter conflict)
   */
  record(
    eventType: TRACEEventType,
    payload: Record<string, unknown>,
    options: {
      span_id?: string;
      parent_span_id?: string;
      severity?: Severity;
      artifacts?: ArtifactReference[];
      tags?: Record<string, string>;
    } = {}
  ): TRACEEvent {
    if (this.closed) {
      throw new Error('Collector is closed');
    }

    const event = createEvent(eventType, payload, {
      trace_id: this.traceId,
      span_id: options.span_id ?? this.traceId,
      session_id: this.sessionId,
      parent_span_id: options.parent_span_id,
      severity: options.severity,
      artifacts: options.artifacts,
      previous_event_hash: this.lastEventHash,
      source: this.source,
      tags: options.tags,
    });

    // Update chain
    this.lastEventHash = event.event_hash;

    // Store event
    this.events.push(event);
    this.buffer.push(event);

    // Associate with span if applicable
    if (options.span_id) {
      const spanContext = this.spans.get(options.span_id);
      if (spanContext) {
        spanContext.events.push(event);
      }
    }

    // Emit for streaming via EventEmitter
    super.emit('event', event);

    // Flush if buffer full
    if (this.buffer.length >= this.bufferSize) {
      this.flush();
    }

    return event;
  }

  /**
   * Start a new span
   */
  startSpan(
    name: string,
    options: {
      parent_span_id?: string;
      attributes?: Record<string, unknown>;
    } = {}
  ): Span {
    const span = createSpan(name, this.traceId, {
      parent_span_id: options.parent_span_id,
      attributes: options.attributes,
    });

    this.spans.set(span.span_id, { span, events: [] });

    // Record span start event
    this.record(
      `carp.${name.replace(/\./g, '_')}.started` as TRACEEventType,
      { span_name: name, ...options.attributes },
      { span_id: span.span_id, parent_span_id: options.parent_span_id }
    );

    return span;
  }

  /**
   * End a span
   */
  endSpan(
    spanId: string,
    status: SpanStatus = 'ok',
    options: {
      message?: string;
      attributes?: Record<string, unknown>;
    } = {}
  ): Span | undefined {
    const context = this.spans.get(spanId);
    if (!context) {
      return undefined;
    }

    const completedSpan = completeSpan(context.span, status, options.message);
    context.span = completedSpan;

    // Calculate duration
    const duration = new Date(completedSpan.ended_at!).getTime() -
      new Date(completedSpan.started_at).getTime();

    // Record span end event
    this.record(
      `carp.${completedSpan.name.replace(/\./g, '_')}.completed` as TRACEEventType,
      {
        span_name: completedSpan.name,
        status,
        duration_ms: duration,
        ...options.attributes,
      },
      { span_id: spanId, parent_span_id: completedSpan.parent_span_id }
    );

    return completedSpan;
  }

  /**
   * Get a span by ID
   */
  getSpan(spanId: string): Span | undefined {
    return this.spans.get(spanId)?.span;
  }

  /**
   * Get all events for a span
   */
  getSpanEvents(spanId: string): TRACEEvent[] {
    return this.spans.get(spanId)?.events ?? [];
  }

  /**
   * Get all collected events
   */
  getEvents(): TRACEEvent[] {
    return [...this.events];
  }

  /**
   * Get events as async iterable stream
   */
  async *stream(): AsyncIterable<TRACEEvent> {
    // Yield existing events first
    for (const event of this.events) {
      yield event;
    }

    // Then yield new events as they come
    const eventQueue: TRACEEvent[] = [];
    let resolve: (() => void) | null = null;

    const handler = (event: TRACEEvent) => {
      eventQueue.push(event);
      if (resolve) {
        resolve();
        resolve = null;
      }
    };

    this.on('event', handler);

    try {
      while (!this.closed) {
        if (eventQueue.length > 0) {
          yield eventQueue.shift()!;
        } else {
          await new Promise<void>(r => { resolve = r; });
        }
      }

      // Yield remaining events
      while (eventQueue.length > 0) {
        yield eventQueue.shift()!;
      }
    } finally {
      this.off('event', handler);
    }
  }

  /**
   * Flush buffered events to file
   */
  async flush(): Promise<void> {
    if (this.buffer.length === 0) return;
    if (!this.fileOutput || !this.outputDir) return;

    const eventsToFlush = [...this.buffer];
    this.buffer = [];

    try {
      // Ensure output directory exists
      await fs.promises.mkdir(this.outputDir, { recursive: true });

      // Open file if not already open
      if (!this.fileHandle) {
        const fileName = this.outputFile ??
          `${new Date().toISOString().replace(/[:.]/g, '-')}-${this.traceId.slice(0, 8)}.trace.jsonl`;
        const filePath = path.join(this.outputDir, fileName);
        this.fileHandle = await fs.promises.open(filePath, 'a');
      }

      // Write events as JSONL
      const lines = eventsToFlush.map(e => toJsonl(e) + '\n').join('');
      await this.fileHandle.write(lines);
    } catch (error) {
      // Put events back in buffer on failure
      this.buffer = eventsToFlush.concat(this.buffer);
      super.emit('error', error);
    }
  }

  /**
   * Verify the integrity of collected events
   */
  verify(): { valid: boolean; errors: string[] } {
    return verifyChain(this.events);
  }

  /**
   * Export events to JSONL string
   */
  toJsonl(): string {
    return this.events.map(e => toJsonl(e)).join('\n');
  }

  /**
   * Get summary of collected trace
   */
  getSummary(): {
    trace_id: string;
    session_id: string;
    event_count: number;
    span_count: number;
    started_at: string;
    ended_at?: string;
    duration_ms?: number;
  } {
    const firstEvent = this.events[0];
    const lastEvent = this.events[this.events.length - 1];

    return {
      trace_id: this.traceId,
      session_id: this.sessionId,
      event_count: this.events.length,
      span_count: this.spans.size,
      started_at: firstEvent?.timestamp ?? getTimestamp(),
      ended_at: lastEvent?.timestamp,
      duration_ms: firstEvent && lastEvent
        ? new Date(lastEvent.timestamp).getTime() - new Date(firstEvent.timestamp).getTime()
        : undefined,
    };
  }

  /**
   * Close the collector
   */
  async close(): Promise<void> {
    if (this.closed) return;

    this.closed = true;

    // Stop flush timer
    if (this.flushTimer) {
      clearInterval(this.flushTimer);
    }

    // Final flush
    await this.flush();

    // Close file handle
    if (this.fileHandle) {
      await this.fileHandle.close();
    }

    super.emit('close');
  }
}

/**
 * Load events from a trace file
 */
export async function loadTraceFile(filePath: string): Promise<TRACEEvent[]> {
  const content = await fs.promises.readFile(filePath, 'utf-8');
  const lines = content.split('\n').filter(line => line.trim());
  return lines.map(line => fromJsonl(line));
}

/**
 * Replay events from a trace file
 */
export async function* replayTrace(
  filePath: string,
  options: {
    speed?: number;
    start_at?: string;
    stop_at?: string;
  } = {}
): AsyncIterable<{
  event: TRACEEvent;
  position: number;
  total: number;
  delay_ms: number;
}> {
  const events = await loadTraceFile(filePath);
  const speed = options.speed ?? 1.0;
  let started = !options.start_at;
  let lastTimestamp: Date | null = null;

  for (let i = 0; i < events.length; i++) {
    const event = events[i];

    // Handle start_at
    if (!started) {
      if (event.event_id === options.start_at) {
        started = true;
      } else {
        continue;
      }
    }

    // Handle stop_at
    if (options.stop_at && event.event_id === options.stop_at) {
      break;
    }

    // Calculate delay
    let delay_ms = 0;
    if (lastTimestamp) {
      const eventTime = new Date(event.timestamp);
      delay_ms = Math.max(0, (eventTime.getTime() - lastTimestamp.getTime()) / speed);
    }

    // Wait for delay
    if (delay_ms > 0) {
      await new Promise(resolve => setTimeout(resolve, delay_ms));
    }

    lastTimestamp = new Date(event.timestamp);

    yield {
      event,
      position: i + 1,
      total: events.length,
      delay_ms,
    };
  }
}
