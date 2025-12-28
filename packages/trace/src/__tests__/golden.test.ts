/**
 * Golden Trace Testing Framework Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  GoldenTraceManager,
  GoldenTraceAssertion,
  createGoldenTraceManager,
  type GoldenTraceTest,
} from '../golden.js';
import { TRACECollector } from '../collector.js';
import type { TRACEEvent } from '@cra/protocol';

describe('GoldenTraceManager', () => {
  let manager: GoldenTraceManager;
  let collector: TRACECollector;

  beforeEach(() => {
    manager = createGoldenTraceManager();
    collector = new TRACECollector({
      session_id: 'test-session',
      trace_id: 'test-trace',
      file_output: false,
    });
  });

  describe('Recording', () => {
    it('should start and stop recording', () => {
      manager.startRecording(collector, 'test-recording');

      // Record some events
      collector.record('session.started', { user: 'test' });
      collector.record('carp.action.started', { action: 'test-action' });

      const golden = manager.stopRecording();

      expect(golden.name).toBe('test-recording');
      expect(golden.events.length).toBe(2);
      expect(golden.metadata.event_count).toBe(2);
    });

    it('should throw if already recording', () => {
      manager.startRecording(collector, 'first');

      expect(() => {
        manager.startRecording(collector, 'second');
      }).toThrow('Already recording');
    });

    it('should throw if not recording when stopping', () => {
      expect(() => {
        manager.stopRecording();
      }).toThrow('Not currently recording');
    });

    it('should include metadata in recording', () => {
      manager.startRecording(collector, 'with-metadata', {
        description: 'Test description',
        version: '1.0',
      });

      collector.record('session.started', {});
      const golden = manager.stopRecording();

      expect(golden.metadata.description).toBe('Test description');
      expect(golden.metadata.version).toBe('1.0');
      expect(golden.metadata.duration_ms).toBeGreaterThanOrEqual(0);
    });
  });

  describe('Registration', () => {
    it('should register and retrieve golden traces', () => {
      const golden: GoldenTraceTest = {
        name: 'test-golden',
        description: 'A test golden trace',
        recorded_at: new Date().toISOString(),
        events: [],
        assertions: [],
        metadata: {},
      };

      manager.registerGolden('test-golden', golden);
      const retrieved = manager.getGolden('test-golden');

      expect(retrieved).toEqual(golden);
    });

    it('should load multiple golden traces', () => {
      const traces: GoldenTraceTest[] = [
        {
          name: 'trace-1',
          description: '',
          recorded_at: new Date().toISOString(),
          events: [],
          assertions: [],
          metadata: {},
        },
        {
          name: 'trace-2',
          description: '',
          recorded_at: new Date().toISOString(),
          events: [],
          assertions: [],
          metadata: {},
        },
      ];

      manager.loadGoldens(traces);

      expect(manager.listGoldens()).toContain('trace-1');
      expect(manager.listGoldens()).toContain('trace-2');
    });

    it('should return undefined for unregistered traces', () => {
      expect(manager.getGolden('nonexistent')).toBeUndefined();
    });
  });

  describe('Comparison', () => {
    const createEvent = (type: string, severity: string = 'info'): TRACEEvent => ({
      event_id: 'evt-1',
      trace_id: 'trace-1',
      span_id: 'span-1',
      event_type: type as any,
      timestamp: new Date().toISOString(),
      severity: severity as any,
      event_hash: 'hash-1',
    });

    it('should pass when events match', () => {
      const goldenEvents = [
        createEvent('session.started'),
        createEvent('action.executed'),
      ];

      manager.registerGolden('matching', {
        name: 'matching',
        description: '',
        recorded_at: new Date().toISOString(),
        events: goldenEvents,
        assertions: [],
        metadata: {},
      });

      const capturedEvents = [
        createEvent('session.started'),
        createEvent('action.executed'),
      ];

      const result = manager.compare('matching', capturedEvents);

      expect(result.passed).toBe(true);
      expect(result.differences.length).toBe(0);
      expect(result.coverage).toBe(100);
    });

    it('should detect missing events', () => {
      manager.registerGolden('with-three', {
        name: 'with-three',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [
          createEvent('session.started'),
          createEvent('action.executed'),
          createEvent('session.ended'),
        ],
        assertions: [],
        metadata: {},
      });

      const result = manager.compare('with-three', [
        createEvent('session.started'),
      ]);

      expect(result.passed).toBe(false);
      expect(result.differences.some(d => d.type === 'removed')).toBe(true);
    });

    it('should detect added events', () => {
      manager.registerGolden('with-one', {
        name: 'with-one',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [createEvent('session.started')],
        assertions: [],
        metadata: {},
      });

      const result = manager.compare('with-one', [
        createEvent('session.started'),
        createEvent('action.executed'),
      ]);

      expect(result.passed).toBe(false);
      expect(result.differences.some(d => d.type === 'added')).toBe(true);
    });

    it('should detect event type changes', () => {
      manager.registerGolden('original', {
        name: 'original',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [createEvent('session.started')],
        assertions: [],
        metadata: {},
      });

      const result = manager.compare('original', [
        createEvent('session.ended'), // Different type
      ]);

      expect(result.passed).toBe(false);
      expect(result.differences.some(d => d.path.includes('event_type'))).toBe(true);
    });

    it('should return error for missing golden', () => {
      const result = manager.compare('nonexistent', []);

      expect(result.passed).toBe(false);
      expect(result.differences[0].message).toContain('not found');
    });

    it('should allow differences with maxDifferences config', () => {
      manager.registerGolden('with-diff', {
        name: 'with-diff',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [createEvent('session.started')],
        assertions: [],
        metadata: {},
      });

      const result = manager.compare('with-diff', [createEvent('session.ended')], {
        maxDifferences: 10,
      });

      expect(result.passed).toBe(true); // Passes because differences <= maxDifferences
    });
  });

  describe('Fingerprinting', () => {
    it('should generate consistent fingerprints', () => {
      const events = [
        {
          event_id: 'evt-1',
          trace_id: 'trace-1',
          span_id: 'span-1',
          event_type: 'session.started' as const,
          timestamp: new Date().toISOString(),
          severity: 'info' as const,
          event_hash: 'hash-1',
        },
      ];

      const fp1 = manager.fingerprint(events);
      const fp2 = manager.fingerprint(events);

      expect(fp1).toBe(fp2);
      expect(fp1.length).toBe(16);
    });

    it('should generate different fingerprints for different events', () => {
      const events1 = [{
        event_id: 'evt-1',
        trace_id: 'trace-1',
        span_id: 'span-1',
        event_type: 'session.started' as const,
        timestamp: new Date().toISOString(),
        severity: 'info' as const,
        event_hash: 'hash-1',
      }];

      const events2 = [{
        event_id: 'evt-1',
        trace_id: 'trace-1',
        span_id: 'span-1',
        event_type: 'session.ended' as const,
        timestamp: new Date().toISOString(),
        severity: 'info' as const,
        event_hash: 'hash-1',
      }];

      expect(manager.fingerprint(events1)).not.toBe(manager.fingerprint(events2));
    });
  });

  describe('Export', () => {
    it('should export golden trace as JSON', () => {
      const golden: GoldenTraceTest = {
        name: 'export-test',
        description: 'For export',
        recorded_at: new Date().toISOString(),
        events: [],
        assertions: [],
        metadata: {},
      };

      manager.registerGolden('export-test', golden);
      const json = manager.exportGolden('export-test');

      const parsed = JSON.parse(json);
      expect(parsed.name).toBe('export-test');
    });

    it('should export all golden traces', () => {
      manager.registerGolden('trace-a', {
        name: 'trace-a',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [],
        assertions: [],
        metadata: {},
      });

      manager.registerGolden('trace-b', {
        name: 'trace-b',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [],
        assertions: [],
        metadata: {},
      });

      const json = manager.exportAll();
      const parsed = JSON.parse(json);

      expect(parsed.length).toBe(2);
    });

    it('should throw when exporting nonexistent trace', () => {
      expect(() => {
        manager.exportGolden('nonexistent');
      }).toThrow('not found');
    });
  });

  describe('Run All', () => {
    it('should run all registered golden tests', () => {
      const event = {
        event_id: 'evt-1',
        trace_id: 'trace-1',
        span_id: 'span-1',
        event_type: 'session.started' as const,
        timestamp: new Date().toISOString(),
        severity: 'info' as const,
        event_hash: 'hash-1',
      };

      manager.registerGolden('test-1', {
        name: 'test-1',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [event],
        assertions: [],
        metadata: {},
      });

      manager.registerGolden('test-2', {
        name: 'test-2',
        description: '',
        recorded_at: new Date().toISOString(),
        events: [event],
        assertions: [],
        metadata: {},
      });

      const results = manager.runAll([event]);

      expect(results.size).toBe(2);
      expect(results.get('test-1')?.passed).toBe(true);
      expect(results.get('test-2')?.passed).toBe(true);
    });
  });
});

describe('GoldenTraceAssertion', () => {
  let manager: GoldenTraceManager;
  let collector: TRACECollector;
  let assertion: GoldenTraceAssertion;

  beforeEach(() => {
    manager = createGoldenTraceManager();
    collector = new TRACECollector({
      session_id: 'assertion-session',
      trace_id: 'assertion-trace',
      file_output: false,
    });
    assertion = manager.createAssertion(collector);
  });

  it('should capture events', () => {
    collector.record('session.started', {});
    collector.record('carp.action.started', {});

    expect(assertion.getEvents().length).toBe(2);
  });

  it('should check for event types', () => {
    collector.record('session.started', {});

    expect(assertion.hasEventType('session.started')).toBe(true);
    expect(assertion.hasEventType('session.ended')).toBe(false);
  });

  it('should check event count', () => {
    collector.record('session.started', {});
    collector.record('carp.action.started', {});

    expect(assertion.hasEventCount(2)).toBe(true);
    expect(assertion.hasEventCount(3)).toBe(false);
  });

  it('should clear events', () => {
    collector.record('session.started', {});
    assertion.clear();

    expect(assertion.getEvents().length).toBe(0);
  });

  it('should record as golden', () => {
    collector.record('session.started', {});

    const golden = assertion.recordAsGolden('new-golden', 'Test description');

    expect(golden.name).toBe('new-golden');
    expect(golden.description).toBe('Test description');
    expect(golden.events.length).toBe(1);
    expect(golden.metadata.fingerprint).toBeDefined();
  });

  it('should match against golden', () => {
    const event = {
      event_id: 'evt-1',
      trace_id: 'trace-1',
      span_id: 'span-1',
      event_type: 'session.started' as const,
      timestamp: new Date().toISOString(),
      severity: 'info' as const,
      event_hash: 'hash-1',
    };

    manager.registerGolden('assertion-test', {
      name: 'assertion-test',
      description: '',
      recorded_at: new Date().toISOString(),
      events: [event],
      assertions: [],
      metadata: {},
    });

    collector.record('session.started', {});

    const result = assertion.matchesGolden('assertion-test');
    expect(result.name).toBe('assertion-test');
  });
});

describe('createGoldenTraceManager', () => {
  it('should create manager with default config', () => {
    const manager = createGoldenTraceManager();
    expect(manager).toBeInstanceOf(GoldenTraceManager);
  });

  it('should create manager with custom config', () => {
    const manager = createGoldenTraceManager({
      ignoreTimestamps: false,
      maxDifferences: 5,
    });
    expect(manager).toBeInstanceOf(GoldenTraceManager);
  });
});
