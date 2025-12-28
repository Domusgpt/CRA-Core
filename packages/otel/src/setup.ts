/**
 * CRA OpenTelemetry Setup
 *
 * Convenience function for setting up OpenTelemetry with CRA.
 */

import { TRACECollector } from '@cra/trace';
import { CRATraceExporter, type TraceExporterConfig } from './trace-exporter.js';
import { CRAMetrics, type MetricsConfig } from './metrics.js';
import { TRACEToOTel, type BridgeConfig } from './bridge.js';

/**
 * CRA OpenTelemetry configuration
 */
export interface CRAOTelConfig {
  /** Service name */
  serviceName?: string;

  /** Enable tracing */
  enableTracing?: boolean;

  /** Enable metrics */
  enableMetrics?: boolean;

  /** Trace exporter config */
  traceExporter?: TraceExporterConfig;

  /** Metrics config */
  metrics?: MetricsConfig;

  /** Bridge config */
  bridge?: BridgeConfig;

  /** TRACE collector to connect to */
  collector?: TRACECollector;
}

/**
 * CRA OpenTelemetry setup result
 */
export interface CRAOTelSetup {
  /** Trace exporter */
  exporter?: CRATraceExporter;

  /** Metrics collector */
  metrics?: CRAMetrics;

  /** TRACE to OTel bridge */
  bridge?: TRACEToOTel;

  /** Shutdown all components */
  shutdown: () => Promise<void>;
}

/**
 * Set up OpenTelemetry for CRA
 */
export function setupCRAOTel(config: CRAOTelConfig = {}): CRAOTelSetup {
  const serviceName = config.serviceName ?? 'cra';
  const result: CRAOTelSetup = {
    shutdown: async () => {},
  };

  const shutdownFns: (() => Promise<void>)[] = [];

  // Set up tracing
  if (config.enableTracing !== false) {
    const exporter = new CRATraceExporter({
      serviceName,
      ...config.traceExporter,
    });
    result.exporter = exporter;

    const bridge = new TRACEToOTel({
      serviceName,
      exporter,
      ...config.bridge,
    });
    result.bridge = bridge;

    // Connect to collector if provided
    if (config.collector) {
      bridge.connect(config.collector);
    }

    shutdownFns.push(() => bridge.shutdown());
  }

  // Set up metrics
  if (config.enableMetrics !== false) {
    const metrics = new CRAMetrics({
      serviceName,
      ...config.metrics,
    });
    result.metrics = metrics;

    shutdownFns.push(() => metrics.shutdown());
  }

  // Create shutdown function
  result.shutdown = async () => {
    for (const fn of shutdownFns) {
      await fn();
    }
  };

  return result;
}
