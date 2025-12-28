/**
 * Storage Package Tests
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { MemoryStore } from '../memory-store.js';
import { ResolutionStore } from '../resolution-store.js';
import { SessionStore } from '../session-store.js';
import { TraceStore } from '../trace-store.js';
import { createStore } from '../factory.js';
import type { CARPResolution, TRACEEvent } from '@cra/protocol';

// Mock resolution for testing
function createMockResolution(id: string): CARPResolution {
  return {
    carp_version: '0.1',
    resolution_id: id,
    timestamp: new Date().toISOString(),
    request_ref: 'req_123',
    decision: {
      type: 'allow',
      reasoning: 'Test resolution',
    },
    context_blocks: [],
    allowed_actions: [],
    policies_applied: [],
    telemetry_link: {
      trace_id: 'trace_123',
      span_id: 'span_123',
    },
    cache: {
      ttl_seconds: 300,
      cache_key: 'test_key',
    },
  };
}

// Mock TRACE event for testing
function createMockEvent(type: string, traceId: string): TRACEEvent {
  return {
    trace_version: '0.1',
    event_id: `evt_${Date.now()}`,
    trace_id: traceId,
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

describe('MemoryStore', () => {
  let store: MemoryStore;

  beforeEach(async () => {
    store = new MemoryStore({ maxItems: 100 });
    await store.init();
  });

  afterEach(async () => {
    await store.close();
  });

  describe('Lifecycle', () => {
    it('should initialize correctly', () => {
      expect(store.isReady()).toBe(true);
    });

    it('should close correctly', async () => {
      await store.close();
      expect(store.isReady()).toBe(false);
    });
  });

  describe('Resolutions', () => {
    it('should save and retrieve resolutions', async () => {
      const resolution = createMockResolution('res_1');
      await store.saveResolution({
        resolution_id: 'res_1',
        session_id: 'sess_1',
        agent_id: 'agent_1',
        resolution,
        created_at: new Date().toISOString(),
        expires_at: new Date(Date.now() + 300000).toISOString(),
        is_valid: true,
        use_count: 0,
      });

      const retrieved = await store.getResolution('res_1');
      expect(retrieved).not.toBeNull();
      expect(retrieved?.resolution_id).toBe('res_1');
    });

    it('should delete resolutions', async () => {
      const resolution = createMockResolution('res_2');
      await store.saveResolution({
        resolution_id: 'res_2',
        session_id: 'sess_1',
        agent_id: 'agent_1',
        resolution,
        created_at: new Date().toISOString(),
        expires_at: new Date(Date.now() + 300000).toISOString(),
        is_valid: true,
        use_count: 0,
      });

      const deleted = await store.deleteResolution('res_2');
      expect(deleted).toBe(true);

      const retrieved = await store.getResolution('res_2');
      expect(retrieved).toBeNull();
    });

    it('should list resolutions by session', async () => {
      for (let i = 0; i < 3; i++) {
        await store.saveResolution({
          resolution_id: `res_${i}`,
          session_id: 'sess_1',
          agent_id: 'agent_1',
          resolution: createMockResolution(`res_${i}`),
          created_at: new Date().toISOString(),
          expires_at: new Date(Date.now() + 300000).toISOString(),
          is_valid: true,
          use_count: 0,
        });
      }

      const result = await store.listResolutions({ session_id: 'sess_1' });
      expect(result.records.length).toBe(3);
    });

    it('should invalidate resolutions', async () => {
      await store.saveResolution({
        resolution_id: 'res_inv',
        session_id: 'sess_1',
        agent_id: 'agent_1',
        resolution: createMockResolution('res_inv'),
        created_at: new Date().toISOString(),
        expires_at: new Date(Date.now() + 300000).toISOString(),
        is_valid: true,
        use_count: 0,
      });

      await store.invalidateResolution('res_inv');
      const retrieved = await store.getResolution('res_inv');
      expect(retrieved?.is_valid).toBe(false);
    });

    it('should increment use count', async () => {
      await store.saveResolution({
        resolution_id: 'res_count',
        session_id: 'sess_1',
        agent_id: 'agent_1',
        resolution: createMockResolution('res_count'),
        created_at: new Date().toISOString(),
        expires_at: new Date(Date.now() + 300000).toISOString(),
        is_valid: true,
        use_count: 0,
      });

      await store.incrementUseCount('res_count');
      await store.incrementUseCount('res_count');

      const retrieved = await store.getResolution('res_count');
      expect(retrieved?.use_count).toBe(2);
    });
  });

  describe('Sessions', () => {
    it('should save and retrieve sessions', async () => {
      await store.saveSession({
        session_id: 'sess_1',
        agent_id: 'agent_1',
        state: 'active',
        metadata: { test: true },
        created_at: new Date().toISOString(),
        last_activity_at: new Date().toISOString(),
        resolution_count: 0,
        action_count: 0,
      });

      const retrieved = await store.getSession('sess_1');
      expect(retrieved).not.toBeNull();
      expect(retrieved?.agent_id).toBe('agent_1');
    });

    it('should update session state', async () => {
      await store.saveSession({
        session_id: 'sess_state',
        agent_id: 'agent_1',
        state: 'active',
        metadata: {},
        created_at: new Date().toISOString(),
        last_activity_at: new Date().toISOString(),
        resolution_count: 0,
        action_count: 0,
      });

      await store.updateSessionState('sess_state', 'idle');
      const retrieved = await store.getSession('sess_state');
      expect(retrieved?.state).toBe('idle');
    });

    it('should increment session counts', async () => {
      await store.saveSession({
        session_id: 'sess_counts',
        agent_id: 'agent_1',
        state: 'active',
        metadata: {},
        created_at: new Date().toISOString(),
        last_activity_at: new Date().toISOString(),
        resolution_count: 0,
        action_count: 0,
      });

      await store.incrementSessionCounts('sess_counts', 3, 5);
      const retrieved = await store.getSession('sess_counts');
      expect(retrieved?.resolution_count).toBe(3);
      expect(retrieved?.action_count).toBe(5);
    });
  });

  describe('Traces', () => {
    it('should save and retrieve traces', async () => {
      await store.saveTrace({
        trace_id: 'trace_1',
        session_id: 'sess_1',
        events: [createMockEvent('test', 'trace_1')],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        chain_verified: false,
        event_count: 1,
      });

      const retrieved = await store.getTrace('trace_1');
      expect(retrieved).not.toBeNull();
      expect(retrieved?.events.length).toBe(1);
    });

    it('should append events to traces', async () => {
      await store.saveTrace({
        trace_id: 'trace_append',
        session_id: 'sess_1',
        events: [createMockEvent('event1', 'trace_append')],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        chain_verified: false,
        event_count: 1,
      });

      await store.appendTraceEvents('trace_append', [
        createMockEvent('event2', 'trace_append'),
        createMockEvent('event3', 'trace_append'),
      ]);

      const retrieved = await store.getTrace('trace_append');
      expect(retrieved?.events.length).toBe(3);
      expect(retrieved?.event_count).toBe(3);
    });
  });

  describe('Stats', () => {
    it('should return store statistics', async () => {
      await store.saveResolution({
        resolution_id: 'res_stat',
        session_id: 'sess_1',
        agent_id: 'agent_1',
        resolution: createMockResolution('res_stat'),
        created_at: new Date().toISOString(),
        expires_at: new Date(Date.now() + 300000).toISOString(),
        is_valid: true,
        use_count: 0,
      });

      const stats = await store.getStats();
      expect(stats.resolutions).toBeGreaterThanOrEqual(1);
    });
  });
});

describe('ResolutionStore', () => {
  let store: ResolutionStore;

  beforeEach(async () => {
    store = new ResolutionStore({ backend: 'memory' });
    await store.init();
  });

  afterEach(async () => {
    await store.close();
  });

  it('should save and retrieve resolutions', async () => {
    const resolution = createMockResolution('res_high');
    await store.save(resolution, {
      sessionId: 'sess_1',
      agentId: 'agent_1',
    });

    const retrieved = await store.get('res_high');
    expect(retrieved).not.toBeNull();
    expect(retrieved?.resolution_id).toBe('res_high');
  });

  it('should check validity', async () => {
    const resolution = createMockResolution('res_valid');
    await store.save(resolution, {
      sessionId: 'sess_1',
      agentId: 'agent_1',
      ttlSeconds: 3600,
    });

    const isValid = await store.isValid('res_valid');
    expect(isValid).toBe(true);
  });

  it('should invalidate resolutions', async () => {
    const resolution = createMockResolution('res_to_inv');
    await store.save(resolution, {
      sessionId: 'sess_1',
      agentId: 'agent_1',
    });

    await store.invalidate('res_to_inv');
    const isValid = await store.isValid('res_to_inv');
    expect(isValid).toBe(false);
  });

  it('should list resolutions by session', async () => {
    for (let i = 0; i < 3; i++) {
      await store.save(createMockResolution(`res_list_${i}`), {
        sessionId: 'sess_list',
        agentId: 'agent_1',
      });
    }

    const list = await store.listBySession('sess_list');
    expect(list.length).toBe(3);
  });
});

describe('SessionStore', () => {
  let store: SessionStore;

  beforeEach(async () => {
    store = new SessionStore({ backend: 'memory' });
    await store.init();
  });

  afterEach(async () => {
    await store.close();
  });

  it('should create sessions', async () => {
    const session = await store.create('sess_new', 'agent_1', { env: 'test' });
    expect(session.session_id).toBe('sess_new');
    expect(session.state).toBe('active');
  });

  it('should retrieve sessions', async () => {
    await store.create('sess_get', 'agent_1');
    const session = await store.get('sess_get');
    expect(session).not.toBeNull();
  });

  it('should terminate sessions', async () => {
    await store.create('sess_term', 'agent_1');
    await store.terminate('sess_term');
    const session = await store.get('sess_term');
    expect(session?.state).toBe('terminated');
  });

  it('should check if session is active', async () => {
    await store.create('sess_active', 'agent_1');
    const isActive = await store.isActive('sess_active');
    expect(isActive).toBe(true);
  });

  it('should record resolutions and actions', async () => {
    await store.create('sess_record', 'agent_1');
    await store.recordResolution('sess_record');
    await store.recordAction('sess_record');
    await store.recordAction('sess_record');

    const session = await store.get('sess_record');
    expect(session?.resolution_count).toBe(1);
    expect(session?.action_count).toBe(2);
  });
});

describe('TraceStore', () => {
  let store: TraceStore;

  beforeEach(async () => {
    store = new TraceStore({ backend: 'memory' });
    await store.init();
  });

  afterEach(async () => {
    await store.close();
  });

  it('should create traces', async () => {
    const trace = await store.create('trace_new', 'sess_1');
    expect(trace.trace_id).toBe('trace_new');
    expect(trace.events).toEqual([]);
  });

  it('should append events', async () => {
    await store.create('trace_evt', 'sess_1');
    await store.appendEvents('trace_evt', [
      createMockEvent('evt1', 'trace_evt'),
      createMockEvent('evt2', 'trace_evt'),
    ]);

    const events = await store.getEvents('trace_evt');
    expect(events.length).toBe(2);
  });

  it('should filter events by type', async () => {
    await store.create('trace_filter', 'sess_1', [
      createMockEvent('type_a', 'trace_filter'),
      createMockEvent('type_b', 'trace_filter'),
      createMockEvent('type_a', 'trace_filter'),
    ]);

    const filtered = await store.getEventsByType('trace_filter', ['type_a']);
    expect(filtered.length).toBe(2);
  });

  it('should get trace summary', async () => {
    await store.create('trace_sum', 'sess_1', [
      createMockEvent('evt1', 'trace_sum'),
      createMockEvent('evt2', 'trace_sum'),
    ]);

    const summary = await store.getSummary('trace_sum');
    expect(summary?.event_count).toBe(2);
    expect(summary?.event_types).toContain('evt1');
  });
});

describe('Storage Factory', () => {
  it('should create complete storage setup', async () => {
    const storage = createStore({ backend: 'memory' });
    await storage.init();

    expect(storage.resolutions).toBeDefined();
    expect(storage.sessions).toBeDefined();
    expect(storage.traces).toBeDefined();

    await storage.close();
  });

  it('should initialize all stores', async () => {
    const storage = createStore({ backend: 'memory' });
    await storage.init();

    // Test each store works
    await storage.sessions.create('test_sess', 'test_agent');
    const session = await storage.sessions.get('test_sess');
    expect(session).not.toBeNull();

    await storage.close();
  });
});
