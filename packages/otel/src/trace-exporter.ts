/**
 * CRA Trace Exporter
 *
 * Exports TRACE events to OpenTelemetry-compatible backends.
 */

import { SpanExporter, ReadableSpan } from '@opentelemetry/sdk-trace-base';
import { ExportResult, ExportResultCode } from '@opentelemetry/core';
import type { TRACEEvent } from '@cra/protocol';

/**
 * Trace exporter configuration
 */
export interface TraceExporterConfig {
  /** Service name for spans */
  serviceName?: string;

  /** Additional resource attributes */
  resourceAttributes?: Record<string, string>;

  /** Console output for debugging */
  consoleOutput?: boolean;

  /** Custom export handler */
  onExport?: (spans: ReadableSpan[]) => void;
}

/**
 * CRA Trace Exporter
 *
 * Exports OpenTelemetry spans to various backends.
 */
export class CRATraceExporter implements SpanExporter {
  private readonly config: Required<TraceExporterConfig>;
  private readonly exportedSpans: ReadableSpan[] = [];
  private isShutdown = false;

  constructor(config: TraceExporterConfig = {}) {
    this.config = {
      serviceName: config.serviceName ?? 'cra',
      resourceAttributes: config.resourceAttributes ?? {},
      consoleOutput: config.consoleOutput ?? false,
      onExport: config.onExport ?? (() => {}),
    };
  }

  /**
   * Export spans
   */
  export(
    spans: ReadableSpan[],
    resultCallback: (result: ExportResult) => void
  ): void {
    if (this.isShutdown) {
      resultCallback({ code: ExportResultCode.FAILED });
      return;
    }

    try {
      for (const span of spans) {
        this.exportedSpans.push(span);

        if (this.config.consoleOutput) {
          console.log(JSON.stringify({
            traceId: span.spanContext().traceId,
            spanId: span.spanContext().spanId,
            name: span.name,
            startTime: span.startTime,
            endTime: span.endTime,
            status: span.status,
            attributes: span.attributes,
          }));
        }
      }

      this.config.onExport(spans);
      resultCallback({ code: ExportResultCode.SUCCESS });
    } catch (error) {
      resultCallback({ code: ExportResultCode.FAILED });
    }
  }

  /**
   * Shutdown the exporter
   */
  async shutdown(): Promise<void> {
    this.isShutdown = true;
  }

  /**
   * Force flush any pending exports
   */
  async forceFlush(): Promise<void> {
    // No-op for this simple exporter
  }

  /**
   * Get all exported spans (for testing)
   */
  getExportedSpans(): ReadableSpan[] {
    return [...this.exportedSpans];
  }

  /**
   * Clear exported spans (for testing)
   */
  clearExportedSpans(): void {
    this.exportedSpans.length = 0;
  }
}

/**
 * Convert TRACE event to OpenTelemetry span attributes
 */
export function traceEventToSpanAttributes(
  event: TRACEEvent
): Record<string, string | number | boolean> {
  return {
    'cra.event_id': event.event_id,
    'cra.trace_id': event.trace_id,
    'cra.span_id': event.span_id,
    'cra.event_type': event.event_type,
    'cra.severity': event.severity,
    'cra.event_hash': event.event_hash,
    ...(event.parent_span_id ? { 'cra.parent_span_id': event.parent_span_id } : {}),
  };
}
