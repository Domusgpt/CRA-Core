/**
 * TRACE to OpenTelemetry Bridge
 *
 * Converts CRA TRACE events to OpenTelemetry spans.
 */

import {
  trace,
  context,
  SpanStatusCode,
  SpanKind,
  Span,
  Tracer,
} from '@opentelemetry/api';
import { BasicTracerProvider, SimpleSpanProcessor } from '@opentelemetry/sdk-trace-base';
import type { TRACEEvent } from '@cra/protocol';
import { TRACECollector } from '@cra/trace';
import { CRATraceExporter, traceEventToSpanAttributes } from './trace-exporter.js';

/**
 * Bridge configuration
 */
export interface BridgeConfig {
  /** Service name */
  serviceName?: string;

  /** Custom span exporter */
  exporter?: CRATraceExporter;

  /** Convert all TRACE events to spans */
  spanPerEvent?: boolean;

  /** Enable console output */
  consoleOutput?: boolean;
}

/**
 * TRACE to OpenTelemetry Bridge
 *
 * Listens to TRACE events and creates OpenTelemetry spans.
 */
export class TRACEToOTel {
  private readonly config: Required<BridgeConfig>;
  private readonly provider: BasicTracerProvider;
  private readonly tracer: Tracer;
  private readonly exporter: CRATraceExporter;
  private readonly activeSpans: Map<string, Span> = new Map();
  private collector: TRACECollector | null = null;

  constructor(config: BridgeConfig = {}) {
    this.config = {
      serviceName: config.serviceName ?? 'cra',
      exporter: config.exporter ?? new CRATraceExporter(),
      spanPerEvent: config.spanPerEvent ?? false,
      consoleOutput: config.consoleOutput ?? false,
    };

    this.exporter = this.config.exporter;

    // Set up OpenTelemetry provider
    this.provider = new BasicTracerProvider();
    this.provider.addSpanProcessor(new SimpleSpanProcessor(this.exporter));
    this.provider.register();

    this.tracer = trace.getTracer(this.config.serviceName, '0.1.0');
  }

  /**
   * Connect to a TRACE collector
   */
  connect(collector: TRACECollector): void {
    this.collector = collector;

    collector.on('event', (event: TRACEEvent) => {
      this.handleEvent(event);
    });
  }

  /**
   * Disconnect from the collector
   */
  disconnect(): void {
    if (this.collector) {
      this.collector.removeAllListeners('event');
      this.collector = null;
    }
  }

  /**
   * Handle a TRACE event
   */
  private handleEvent(event: TRACEEvent): void {
    if (this.config.consoleOutput) {
      console.log(`[OTel Bridge] Event: ${event.event_type}`);
    }

    // Handle span lifecycle events
    if (event.event_type.endsWith('.started') || event.event_type.includes('.begin')) {
      this.startSpan(event);
    } else if (event.event_type.endsWith('.completed') || event.event_type.endsWith('.ended')) {
      this.endSpan(event);
    } else if (event.event_type.endsWith('.failed') || event.event_type.endsWith('.error')) {
      this.failSpan(event);
    } else if (this.config.spanPerEvent) {
      // Create instant span for other events
      this.createInstantSpan(event);
    }
  }

  /**
   * Start a new span
   */
  private startSpan(event: TRACEEvent): void {
    const parentSpan = event.parent_span_id
      ? this.activeSpans.get(event.parent_span_id)
      : undefined;

    const ctx = parentSpan
      ? trace.setSpan(context.active(), parentSpan)
      : context.active();

    const span = this.tracer.startSpan(
      event.event_type.replace('.started', '').replace('.begin', ''),
      {
        kind: SpanKind.INTERNAL,
        attributes: traceEventToSpanAttributes(event),
        startTime: new Date(event.timestamp),
      },
      ctx
    );

    this.activeSpans.set(event.span_id, span);
  }

  /**
   * End a span successfully
   */
  private endSpan(event: TRACEEvent): void {
    const span = this.activeSpans.get(event.span_id);
    if (span) {
      span.setStatus({ code: SpanStatusCode.OK });
      span.end(new Date(event.timestamp));
      this.activeSpans.delete(event.span_id);
    }
  }

  /**
   * End a span with error
   */
  private failSpan(event: TRACEEvent): void {
    const span = this.activeSpans.get(event.span_id);
    if (span) {
      span.setStatus({
        code: SpanStatusCode.ERROR,
        message: event.payload?.error?.toString() ?? 'Unknown error',
      });
      span.end(new Date(event.timestamp));
      this.activeSpans.delete(event.span_id);
    }
  }

  /**
   * Create an instant span for a single event
   */
  private createInstantSpan(event: TRACEEvent): void {
    const parentSpan = event.parent_span_id
      ? this.activeSpans.get(event.parent_span_id)
      : undefined;

    const ctx = parentSpan
      ? trace.setSpan(context.active(), parentSpan)
      : context.active();

    const span = this.tracer.startSpan(
      event.event_type,
      {
        kind: SpanKind.INTERNAL,
        attributes: traceEventToSpanAttributes(event),
        startTime: new Date(event.timestamp),
      },
      ctx
    );

    span.end(new Date(event.timestamp));
  }

  /**
   * Get active spans (for testing)
   */
  getActiveSpans(): Map<string, Span> {
    return new Map(this.activeSpans);
  }

  /**
   * Get the exporter (for testing)
   */
  getExporter(): CRATraceExporter {
    return this.exporter;
  }

  /**
   * Shutdown the bridge
   */
  async shutdown(): Promise<void> {
    this.disconnect();

    // End any remaining spans
    for (const span of this.activeSpans.values()) {
      span.end();
    }
    this.activeSpans.clear();

    await this.provider.shutdown();
  }
}
