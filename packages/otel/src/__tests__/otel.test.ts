/**
 * CRA OpenTelemetry Integration Tests
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { CRATraceExporter, traceEventToSpanAttributes } from '../trace-exporter.js';
import { CRAMetrics } from '../metrics.js';
import { TRACEToOTel } from '../bridge.js';
import { setupCRAOTel } from '../setup.js';
import type { TRACEEvent } from '@cra/protocol';

// Mock TRACE event
function createMockEvent(type: string): TRACEEvent {
  return {
    trace_version: '0.1',
    event_id: `evt_${Date.now()}`,
    trace_id: 'trace_123',
    span_id: 'span_123',
    parent_span_id: undefined,
    timestamp: new Date().toISOString(),
    event_type: type,
    severity: 'info',
    payload: { test: true },
    hash: 'abc123',
    prev_hash: undefined,
  };
}

describe('CRATraceExporter', () => {
  let exporter: CRATraceExporter;

  beforeEach(() => {
    exporter = new CRATraceExporter();
  });

  afterEach(async () => {
    await exporter.shutdown();
  });

  describe('Initialization', () => {
    it('should create with default config', () => {
      expect(exporter).toBeDefined();
    });

    it('should create with custom config', () => {
      const exp = new CRATraceExporter({
        serviceName: 'test-service',
        consoleOutput: true,
      });
      expect(exp).toBeDefined();
    });
  });

  describe('Export', () => {
    it('should start with empty exported spans', () => {
      expect(exporter.getExportedSpans()).toEqual([]);
    });

    it('should clear exported spans', () => {
      exporter.clearExportedSpans();
      expect(exporter.getExportedSpans()).toEqual([]);
    });
  });
});

describe('traceEventToSpanAttributes', () => {
  it('should convert TRACE event to span attributes', () => {
    const event = createMockEvent('test.event');
    const attrs = traceEventToSpanAttributes(event);

    expect(attrs['cra.event_id']).toBe(event.event_id);
    expect(attrs['cra.trace_id']).toBe('trace_123');
    expect(attrs['cra.span_id']).toBe('span_123');
    expect(attrs['cra.event_type']).toBe('test.event');
    expect(attrs['cra.severity']).toBe('info');
  });

  it('should include parent_span_id when present', () => {
    const event = createMockEvent('test.event');
    event.parent_span_id = 'parent_span';
    const attrs = traceEventToSpanAttributes(event);

    expect(attrs['cra.parent_span_id']).toBe('parent_span');
  });
});

describe('CRAMetrics', () => {
  let metrics: CRAMetrics;

  beforeEach(() => {
    metrics = new CRAMetrics({ serviceName: 'test' });
  });

  afterEach(async () => {
    await metrics.shutdown();
  });

  describe('Initialization', () => {
    it('should create with default config', () => {
      const m = new CRAMetrics();
      expect(m).toBeDefined();
    });

    it('should have all counter metrics', () => {
      expect(metrics.resolutionsTotal).toBeDefined();
      expect(metrics.actionsTotal).toBeDefined();
      expect(metrics.cacheHitsTotal).toBeDefined();
      expect(metrics.cacheMissesTotal).toBeDefined();
      expect(metrics.traceEventsTotal).toBeDefined();
      expect(metrics.errorsTotal).toBeDefined();
    });

    it('should have all histogram metrics', () => {
      expect(metrics.resolutionDuration).toBeDefined();
      expect(metrics.actionDuration).toBeDefined();
    });

    it('should have all gauge metrics', () => {
      expect(metrics.activeSessions).toBeDefined();
      expect(metrics.loadedAtlases).toBeDefined();
    });
  });

  describe('Recording', () => {
    it('should record resolution', () => {
      expect(() => {
        metrics.recordResolution(100, {
          decision: 'allow',
          risk_tier: 'low',
          agent_id: 'test',
        });
      }).not.toThrow();
    });

    it('should record action', () => {
      expect(() => {
        metrics.recordAction(50, {
          action_type: 'api.test',
          status: 'success',
          risk_tier: 'low',
        });
      }).not.toThrow();
    });

    it('should record cache hit', () => {
      expect(() => {
        metrics.recordCacheHit('resolution');
      }).not.toThrow();
    });

    it('should record cache miss', () => {
      expect(() => {
        metrics.recordCacheMiss('resolution');
      }).not.toThrow();
    });

    it('should record trace event', () => {
      expect(() => {
        metrics.recordTraceEvent('test.event', 'info');
      }).not.toThrow();
    });

    it('should record error', () => {
      expect(() => {
        metrics.recordError('validation');
      }).not.toThrow();
    });

    it('should update active sessions', () => {
      expect(() => {
        metrics.updateActiveSessions(1);
        metrics.updateActiveSessions(-1);
      }).not.toThrow();
    });

    it('should update loaded atlases', () => {
      expect(() => {
        metrics.updateLoadedAtlases(3);
      }).not.toThrow();
    });
  });
});

describe('TRACEToOTel Bridge', () => {
  let bridge: TRACEToOTel;

  beforeEach(() => {
    bridge = new TRACEToOTel({ serviceName: 'test' });
  });

  afterEach(async () => {
    await bridge.shutdown();
  });

  describe('Initialization', () => {
    it('should create with default config', () => {
      const b = new TRACEToOTel();
      expect(b).toBeDefined();
    });

    it('should start with no active spans', () => {
      expect(bridge.getActiveSpans().size).toBe(0);
    });

    it('should have an exporter', () => {
      expect(bridge.getExporter()).toBeDefined();
    });
  });

  describe('Connection', () => {
    it('should connect and disconnect', () => {
      // Note: Would need a mock collector for full testing
      expect(() => bridge.disconnect()).not.toThrow();
    });
  });
});

describe('setupCRAOTel', () => {
  it('should create setup with default config', async () => {
    const setup = setupCRAOTel();
    expect(setup).toBeDefined();
    expect(setup.exporter).toBeDefined();
    expect(setup.metrics).toBeDefined();
    expect(setup.bridge).toBeDefined();
    await setup.shutdown();
  });

  it('should create setup with tracing only', async () => {
    const setup = setupCRAOTel({
      enableTracing: true,
      enableMetrics: false,
    });
    expect(setup.exporter).toBeDefined();
    expect(setup.bridge).toBeDefined();
    expect(setup.metrics).toBeUndefined();
    await setup.shutdown();
  });

  it('should create setup with metrics only', async () => {
    const setup = setupCRAOTel({
      enableTracing: false,
      enableMetrics: true,
    });
    expect(setup.exporter).toBeUndefined();
    expect(setup.bridge).toBeUndefined();
    expect(setup.metrics).toBeDefined();
    await setup.shutdown();
  });

  it('should use custom service name', async () => {
    const setup = setupCRAOTel({
      serviceName: 'custom-service',
    });
    expect(setup).toBeDefined();
    await setup.shutdown();
  });

  it('should call shutdown without errors', async () => {
    const setup = setupCRAOTel();
    await expect(setup.shutdown()).resolves.not.toThrow();
  });
});
