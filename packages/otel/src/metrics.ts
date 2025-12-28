/**
 * CRA Metrics
 *
 * Prometheus-compatible metrics for CRA operations.
 */

import { MeterProvider, MetricReader } from '@opentelemetry/sdk-metrics';
import { metrics, Counter, Histogram, UpDownCounter } from '@opentelemetry/api';

/**
 * Metrics configuration
 */
export interface MetricsConfig {
  /** Service name */
  serviceName?: string;

  /** Metric reader (e.g., Prometheus exporter) */
  reader?: MetricReader;

  /** Enable default metrics */
  enableDefaultMetrics?: boolean;
}

/**
 * CRA Metrics collector
 *
 * Exposes Prometheus-compatible metrics:
 * - cra_resolutions_total: Total resolution requests
 * - cra_resolution_duration_seconds: Resolution latency
 * - cra_actions_total: Total action executions
 * - cra_action_duration_seconds: Action execution latency
 * - cra_cache_hits_total: Cache hit count
 * - cra_cache_misses_total: Cache miss count
 * - cra_trace_events_total: Total trace events
 * - cra_active_sessions: Current active sessions
 */
export class CRAMetrics {
  private readonly meterProvider: MeterProvider;
  private readonly meter;

  // Counters
  readonly resolutionsTotal: Counter;
  readonly actionsTotal: Counter;
  readonly cacheHitsTotal: Counter;
  readonly cacheMissesTotal: Counter;
  readonly traceEventsTotal: Counter;
  readonly errorsTotal: Counter;

  // Histograms
  readonly resolutionDuration: Histogram;
  readonly actionDuration: Histogram;

  // Gauges
  readonly activeSessions: UpDownCounter;
  readonly loadedAtlases: UpDownCounter;

  constructor(config: MetricsConfig = {}) {
    const serviceName = config.serviceName ?? 'cra';

    this.meterProvider = new MeterProvider();

    if (config.reader) {
      this.meterProvider.addMetricReader(config.reader);
    }

    metrics.setGlobalMeterProvider(this.meterProvider);
    this.meter = metrics.getMeter(serviceName, '0.1.0');

    // Create counters
    this.resolutionsTotal = this.meter.createCounter('cra_resolutions_total', {
      description: 'Total number of CARP resolution requests',
    });

    this.actionsTotal = this.meter.createCounter('cra_actions_total', {
      description: 'Total number of action executions',
    });

    this.cacheHitsTotal = this.meter.createCounter('cra_cache_hits_total', {
      description: 'Total number of cache hits',
    });

    this.cacheMissesTotal = this.meter.createCounter('cra_cache_misses_total', {
      description: 'Total number of cache misses',
    });

    this.traceEventsTotal = this.meter.createCounter('cra_trace_events_total', {
      description: 'Total number of TRACE events emitted',
    });

    this.errorsTotal = this.meter.createCounter('cra_errors_total', {
      description: 'Total number of errors',
    });

    // Create histograms
    this.resolutionDuration = this.meter.createHistogram('cra_resolution_duration_seconds', {
      description: 'CARP resolution latency in seconds',
      unit: 's',
    });

    this.actionDuration = this.meter.createHistogram('cra_action_duration_seconds', {
      description: 'Action execution latency in seconds',
      unit: 's',
    });

    // Create gauges
    this.activeSessions = this.meter.createUpDownCounter('cra_active_sessions', {
      description: 'Number of currently active sessions',
    });

    this.loadedAtlases = this.meter.createUpDownCounter('cra_loaded_atlases', {
      description: 'Number of currently loaded atlases',
    });
  }

  /**
   * Record a resolution
   */
  recordResolution(
    durationMs: number,
    attributes: {
      decision: string;
      risk_tier: string;
      agent_id: string;
    }
  ): void {
    this.resolutionsTotal.add(1, attributes);
    this.resolutionDuration.record(durationMs / 1000, attributes);
  }

  /**
   * Record an action execution
   */
  recordAction(
    durationMs: number,
    attributes: {
      action_type: string;
      status: string;
      risk_tier: string;
    }
  ): void {
    this.actionsTotal.add(1, attributes);
    this.actionDuration.record(durationMs / 1000, attributes);
  }

  /**
   * Record a cache hit
   */
  recordCacheHit(cacheType: string): void {
    this.cacheHitsTotal.add(1, { cache_type: cacheType });
  }

  /**
   * Record a cache miss
   */
  recordCacheMiss(cacheType: string): void {
    this.cacheMissesTotal.add(1, { cache_type: cacheType });
  }

  /**
   * Record a trace event
   */
  recordTraceEvent(eventType: string, severity: string): void {
    this.traceEventsTotal.add(1, { event_type: eventType, severity });
  }

  /**
   * Record an error
   */
  recordError(errorType: string): void {
    this.errorsTotal.add(1, { error_type: errorType });
  }

  /**
   * Update active sessions count
   */
  updateActiveSessions(delta: number): void {
    this.activeSessions.add(delta);
  }

  /**
   * Update loaded atlases count
   */
  updateLoadedAtlases(delta: number): void {
    this.loadedAtlases.add(delta);
  }

  /**
   * Shutdown the metrics provider
   */
  async shutdown(): Promise<void> {
    await this.meterProvider.shutdown();
  }
}
