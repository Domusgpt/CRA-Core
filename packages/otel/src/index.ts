/**
 * CRA OpenTelemetry Integration
 *
 * Provides OpenTelemetry-compatible tracing and metrics for CRA.
 * Bridges TRACE events to OpenTelemetry spans and exports metrics.
 */

export { CRATraceExporter, type TraceExporterConfig } from './trace-exporter.js';
export { CRAMetrics, type MetricsConfig } from './metrics.js';
export { TRACEToOTel, type BridgeConfig } from './bridge.js';
export { setupCRAOTel, type CRAOTelConfig } from './setup.js';
