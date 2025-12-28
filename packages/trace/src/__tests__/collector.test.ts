/**
 * TRACE Collector Tests
 *
 * Tests for TRACE event collection, hash chain integrity, and streaming.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { TRACECollector, loadTraceFile, replayTrace } from '../collector.js';
import {
  verifyChain,
  verifyEventHash,
  createEvent,
  toJsonl,
  fromJsonl,
  diffTraces,
  TRACE_VERSION,
} from '@cra/protocol';
import type { TRACEEvent } from '@cra/protocol';

describe('TRACE Collector', () => {
  let collector: TRACECollector;
  const testOutputDir = './test-traces';

  beforeEach(() => {
    collector = new TRACECollector({
      session_id: 'test-session',
      trace_id: 'test-trace-id',
      output_dir: testOutputDir,
      file_output: false, // Disable file output for unit tests
      flush_interval_ms: 0, // Disable auto-flush
    });
  });

  afterEach(async () => {
    await collector.close();
    // Cleanup test traces
    try {
      if (fs.existsSync(testOutputDir)) {
        const files = fs.readdirSync(testOutputDir);
        for (const file of files) {
          fs.unlinkSync(path.join(testOutputDir, file));
        }
        fs.rmdirSync(testOutputDir);
      }
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Event Recording', () => {
    it('should record events with correct structure', () => {
      const event = collector.record('session.started', { test: true });

      expect(event.trace_version).toBe(TRACE_VERSION);
      expect(event.event_id).toMatch(/^[0-9a-f-]{36}$/);
      expect(event.trace_id).toBe('test-trace-id');
      expect(event.session_id).toBe('test-session');
      expect(event.event_type).toBe('session.started');
      expect(event.payload).toEqual({ test: true });
      expect(event.timestamp).toBeDefined();
      expect(event.sequence).toBeDefined();
    });

    it('should assign sequential sequence numbers', () => {
      const event1 = collector.record('event.one', {});
      const event2 = collector.record('event.two', {});
      const event3 = collector.record('event.three', {});

      expect(event2.sequence).toBe(event1.sequence + 1);
      expect(event3.sequence).toBe(event2.sequence + 1);
    });

    it('should store events in order', () => {
      collector.record('event.one', { order: 1 });
      collector.record('event.two', { order: 2 });
      collector.record('event.three', { order: 3 });

      const events = collector.getEvents();
      expect(events).toHaveLength(3);
      expect(events[0].payload.order).toBe(1);
      expect(events[1].payload.order).toBe(2);
      expect(events[2].payload.order).toBe(3);
    });

    it('should include optional severity', () => {
      const event = collector.record('error.occurred', { msg: 'fail' }, { severity: 'error' });
      expect(event.severity).toBe('error');
    });

    it('should include optional span_id', () => {
      const event = collector.record('span.event', {}, { span_id: 'span-123' });
      expect(event.span_id).toBe('span-123');
    });
  });

  describe('Hash Chain Integrity', () => {
    it('should create linked hash chain', () => {
      collector.record('event.one', {});
      collector.record('event.two', {});
      collector.record('event.three', {});

      const events = collector.getEvents();

      // First event has no previous hash
      expect(events[0].previous_event_hash).toBeUndefined();

      // Subsequent events link to previous
      expect(events[1].previous_event_hash).toBe(events[0].event_hash);
      expect(events[2].previous_event_hash).toBe(events[1].event_hash);
    });

    it('should pass chain verification', () => {
      collector.record('session.started', {});
      collector.record('carp.request.received', { goal: 'Test' });
      collector.record('carp.resolution.completed', { decision: 'allow' });
      collector.record('session.ended', {});

      const events = collector.getEvents();
      const { valid, errors } = verifyChain(events);

      expect(valid).toBe(true);
      expect(errors).toHaveLength(0);
    });

    it('should detect chain tampering', () => {
      collector.record('session.started', {});
      collector.record('carp.request.received', {});

      const events = collector.getEvents();

      // Tamper with first event's payload
      (events[0] as { payload: Record<string, unknown> }).payload = { tampered: true };

      const { valid, errors } = verifyChain(events);
      expect(valid).toBe(false);
      expect(errors.length).toBeGreaterThan(0);
    });

    it('should detect broken chain links', () => {
      collector.record('event.one', {});
      collector.record('event.two', {});
      collector.record('event.three', {});

      const events = collector.getEvents();

      // Break the chain by modifying previous_event_hash
      (events[1] as { previous_event_hash: string }).previous_event_hash = 'wrong-hash';

      const { valid, errors } = verifyChain(events);
      expect(valid).toBe(false);
    });

    it('should verify individual event hashes', () => {
      const event = collector.record('test.event', { data: 'value' });
      expect(verifyEventHash(event)).toBe(true);
    });
  });

  describe('Span Management', () => {
    it('should start a span', () => {
      const span = collector.startSpan('test.operation', {
        attributes: { key: 'value' },
      });

      expect(span.span_id).toMatch(/^[0-9a-f-]{36}$/);
      expect(span.name).toBe('test.operation');
      expect(span.trace_id).toBe('test-trace-id');
      expect(span.started_at).toBeDefined();
      expect(span.status).toBe('in_progress');
    });

    it('should end a span', () => {
      const span = collector.startSpan('test.operation');
      const completed = collector.endSpan(span.span_id, 'ok');

      expect(completed).toBeDefined();
      expect(completed!.status).toBe('ok');
      expect(completed!.ended_at).toBeDefined();
    });

    it('should end span with error status', () => {
      const span = collector.startSpan('failing.operation');
      const completed = collector.endSpan(span.span_id, 'error', { message: 'Something failed' });

      expect(completed!.status).toBe('error');
      expect(completed!.status_message).toBe('Something failed');
    });

    it('should associate events with spans', () => {
      const span = collector.startSpan('parent.operation');

      collector.record('child.event.one', {}, { span_id: span.span_id });
      collector.record('child.event.two', {}, { span_id: span.span_id });

      const spanEvents = collector.getSpanEvents(span.span_id);
      expect(spanEvents.length).toBeGreaterThanOrEqual(2);
    });

    it('should support nested spans', () => {
      const parentSpan = collector.startSpan('parent');
      const childSpan = collector.startSpan('child', { parent_span_id: parentSpan.span_id });

      expect(childSpan.parent_span_id).toBe(parentSpan.span_id);

      collector.endSpan(childSpan.span_id, 'ok');
      collector.endSpan(parentSpan.span_id, 'ok');

      const events = collector.getEvents();
      expect(events.length).toBeGreaterThanOrEqual(4); // Start and end for each span
    });
  });

  describe('Event Streaming', () => {
    it('should emit events via EventEmitter', async () => {
      const received: TRACEEvent[] = [];

      collector.on('event', (event: TRACEEvent) => {
        received.push(event);
      });

      collector.record('event.one', {});
      collector.record('event.two', {});

      // Wait a tick for events to propagate
      await new Promise(resolve => setTimeout(resolve, 10));

      expect(received).toHaveLength(2);
    });
  });

  describe('Summary', () => {
    it('should provide trace summary', () => {
      collector.record('session.started', {});
      const span = collector.startSpan('operation');
      collector.record('operation.step', {}, { span_id: span.span_id });
      collector.endSpan(span.span_id, 'ok');
      collector.record('session.ended', {});

      const summary = collector.getSummary();

      expect(summary.trace_id).toBe('test-trace-id');
      expect(summary.session_id).toBe('test-session');
      expect(summary.event_count).toBeGreaterThan(0);
      expect(summary.span_count).toBe(1);
      expect(summary.started_at).toBeDefined();
    });
  });

  describe('Verification', () => {
    it('should verify its own events', () => {
      collector.record('event.one', {});
      collector.record('event.two', {});
      collector.record('event.three', {});

      const { valid, errors } = collector.verify();
      expect(valid).toBe(true);
      expect(errors).toHaveLength(0);
    });
  });

  describe('JSONL Export', () => {
    it('should export events as JSONL', () => {
      collector.record('event.one', { data: 1 });
      collector.record('event.two', { data: 2 });

      const jsonl = collector.toJsonl();
      const lines = jsonl.split('\n').filter(l => l.trim());

      expect(lines).toHaveLength(2);

      const parsed1 = JSON.parse(lines[0]);
      const parsed2 = JSON.parse(lines[1]);

      expect(parsed1.event_type).toBe('event.one');
      expect(parsed2.event_type).toBe('event.two');
    });
  });
});

describe('TRACE Utilities', () => {
  describe('toJsonl / fromJsonl', () => {
    it('should serialize and deserialize events', () => {
      const event = createEvent('test.event', { key: 'value' }, {
        trace_id: 'trace-1',
        span_id: 'span-1',
        session_id: 'session-1',
      });

      const jsonl = toJsonl(event);
      const restored = fromJsonl(jsonl);

      expect(restored.event_type).toBe(event.event_type);
      expect(restored.trace_id).toBe(event.trace_id);
      expect(restored.payload).toEqual(event.payload);
    });
  });

  describe('diffTraces', () => {
    it('should identify identical traces', () => {
      const events1: TRACEEvent[] = [
        createEvent('event.one', { v: 1 }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.two', { v: 2 }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const events2: TRACEEvent[] = [
        createEvent('event.one', { v: 1 }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.two', { v: 2 }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const diff = diffTraces(events1, events2, {
        ignore_fields: ['event_id', 'timestamp', 'sequence', 'event_hash', 'previous_event_hash'],
      });

      expect(diff.compatibility).toBe('identical');
      expect(diff.differences).toHaveLength(0);
    });

    it('should detect added events', () => {
      const events1: TRACEEvent[] = [
        createEvent('event.one', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const events2: TRACEEvent[] = [
        createEvent('event.one', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.two', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const diff = diffTraces(events1, events2, {
        ignore_fields: ['event_id', 'timestamp', 'sequence', 'event_hash', 'previous_event_hash'],
      });

      expect(diff.summary.events_added).toBe(1);
    });

    it('should detect removed events', () => {
      const events1: TRACEEvent[] = [
        createEvent('event.one', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.two', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const events2: TRACEEvent[] = [
        createEvent('event.one', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const diff = diffTraces(events1, events2, {
        ignore_fields: ['event_id', 'timestamp', 'sequence', 'event_hash', 'previous_event_hash'],
      });

      expect(diff.summary.events_removed).toBe(1);
    });

    it('should detect modified payloads', () => {
      const events1: TRACEEvent[] = [
        createEvent('event.one', { value: 'old' }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const events2: TRACEEvent[] = [
        createEvent('event.one', { value: 'new' }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const diff = diffTraces(events1, events2, {
        ignore_fields: ['event_id', 'timestamp', 'sequence', 'event_hash', 'previous_event_hash'],
      });

      expect(diff.summary.events_modified).toBeGreaterThan(0);
    });
  });
});

describe('File Operations', () => {
  const testDir = './test-trace-files';
  const testFile = path.join(testDir, 'test.trace.jsonl');

  beforeEach(async () => {
    await fs.promises.mkdir(testDir, { recursive: true });
  });

  afterEach(async () => {
    try {
      if (fs.existsSync(testFile)) {
        await fs.promises.unlink(testFile);
      }
      if (fs.existsSync(testDir)) {
        await fs.promises.rmdir(testDir);
      }
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('loadTraceFile', () => {
    it('should load events from JSONL file', async () => {
      const events = [
        createEvent('event.one', { n: 1 }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.two', { n: 2 }, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const content = events.map(e => toJsonl(e)).join('\n');
      await fs.promises.writeFile(testFile, content);

      const loaded = await loadTraceFile(testFile);

      expect(loaded).toHaveLength(2);
      expect(loaded[0].event_type).toBe('event.one');
      expect(loaded[1].event_type).toBe('event.two');
    });
  });

  describe('replayTrace', () => {
    it('should replay events from file', async () => {
      const events = [
        createEvent('event.one', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.two', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
        createEvent('event.three', {}, { trace_id: 't', span_id: 's', session_id: 'sess' }),
      ];

      const content = events.map(e => toJsonl(e)).join('\n');
      await fs.promises.writeFile(testFile, content);

      const replayed: TRACEEvent[] = [];
      for await (const { event, position, total } of replayTrace(testFile, { speed: 100 })) {
        replayed.push(event);
        expect(position).toBeLessThanOrEqual(total);
      }

      expect(replayed).toHaveLength(3);
    });
  });
});
