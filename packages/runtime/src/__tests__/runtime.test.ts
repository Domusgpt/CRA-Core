/**
 * CRA Runtime Tests
 *
 * Integration tests for the CRA runtime, CARP resolution, and TRACE emission.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { CRARuntime } from '../runtime.js';
import { createRequest } from '@cra/protocol';
import type { CARPResolution, CARPError, TRACEEvent } from '@cra/protocol';

describe('CRA Runtime', () => {
  let runtime: CRARuntime;
  const testTraceDir = './test-runtime-traces';

  beforeEach(() => {
    runtime = new CRARuntime({
      session_id: 'test-session',
      trace_dir: testTraceDir,
      trace_to_file: false, // Disable file output for tests
      default_ttl_seconds: 300,
      max_context_tokens: 8192,
      max_actions_per_resolution: 50,
    });
  });

  afterEach(async () => {
    try {
      await runtime.shutdown();
    } catch {
      // Ignore if already shut down
    }
    // Cleanup
    try {
      if (fs.existsSync(testTraceDir)) {
        const files = fs.readdirSync(testTraceDir);
        for (const file of files) {
          fs.unlinkSync(path.join(testTraceDir, file));
        }
        fs.rmdirSync(testTraceDir);
      }
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Initialization', () => {
    it('should initialize with default config', () => {
      const rt = new CRARuntime();
      expect(rt.getSessionId()).toMatch(/^[0-9a-f-]{36}$/);
      rt.shutdown();
    });

    it('should use provided session ID', () => {
      expect(runtime.getSessionId()).toBe('test-session');
    });

    it('should provide TRACE collector', () => {
      const trace = runtime.getTrace();
      expect(trace).toBeDefined();
      expect(trace.getTraceId()).toMatch(/^[0-9a-f-]{36}$/);
    });

    it('should emit session.started event', () => {
      const events = runtime.getTrace().getEvents();
      const startEvent = events.find(e => e.event_type === 'session.started');
      expect(startEvent).toBeDefined();
      expect(startEvent?.payload.session_id).toBe('test-session');
    });
  });

  describe('Resolution without Atlases', () => {
    it('should return error when no atlases loaded', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'test-session',
      }, {
        task: { goal: 'Do something' },
      });

      const result = await runtime.resolve(request);

      expect('error' in result).toBe(true);
      const error = result as CARPError;
      expect(error.error.code).toBe('ATLAS_NOT_FOUND');
    });
  });

  describe('Request Validation', () => {
    it('should reject invalid request', async () => {
      const invalidRequest = {
        carp_version: '2.0', // Wrong version
        request_id: 'test',
        timestamp: new Date().toISOString(),
        operation: 'resolve',
        requester: { agent_id: 'test', session_id: 'sess' },
      } as any;

      const result = await runtime.resolve(invalidRequest);

      expect('error' in result).toBe(true);
      const error = result as CARPError;
      expect(error.error.code).toBe('INVALID_REQUEST');
    });
  });

  describe('TRACE Emission', () => {
    it('should emit carp.request.received on resolve', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'test-session',
      }, {
        task: { goal: 'Test goal' },
      });

      await runtime.resolve(request);

      const events = runtime.getTrace().getEvents();
      const receivedEvent = events.find(e => e.event_type === 'carp.request.received');

      expect(receivedEvent).toBeDefined();
      expect(receivedEvent?.payload.request_id).toBe(request.request_id);
      expect(receivedEvent?.payload.goal).toBe('Test goal');
    });

    it('should maintain hash chain integrity', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'test-session',
      }, {
        task: { goal: 'Test' },
      });

      await runtime.resolve(request);

      const { valid, errors } = runtime.getTrace().verify();
      expect(valid).toBe(true);
      expect(errors).toHaveLength(0);
    });

    it('should associate events with spans', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'test-session',
      }, {
        task: { goal: 'Test' },
      });

      await runtime.resolve(request);

      const events = runtime.getTrace().getEvents();

      // Find carp.resolve span events
      const spanStartEvents = events.filter(e =>
        e.event_type.includes('started') && e.span_id
      );
      expect(spanStartEvents.length).toBeGreaterThan(0);
    });
  });

  describe('Statistics', () => {
    it('should track resolution statistics', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'test-session',
      }, {
        task: { goal: 'Test' },
      });

      await runtime.resolve(request);

      const stats = runtime.getStats();
      expect(stats.resolutions_total).toBe(1);
      expect(stats.resolutions_denied).toBe(1); // No atlas, so denied
      expect(stats.atlases_loaded).toBe(0);
      expect(stats.session_start).toBeDefined();
      expect(stats.uptime_ms).toBeGreaterThanOrEqual(0);
    });
  });

  describe('Shutdown', () => {
    it('should emit session.ended on shutdown', async () => {
      await runtime.shutdown();

      const events = runtime.getTrace().getEvents();
      const endEvent = events.find(e => e.event_type === 'session.ended');

      expect(endEvent).toBeDefined();
      expect(endEvent?.payload.session_id).toBe('test-session');
      expect(endEvent?.payload.stats).toBeDefined();
    });
  });
});

describe('CRA Runtime with Atlas', () => {
  let runtime: CRARuntime;
  const testAtlasDir = './test-atlas';
  const testTraceDir = './test-atlas-traces';

  beforeEach(async () => {
    // Create a minimal test atlas
    await fs.promises.mkdir(testAtlasDir, { recursive: true });
    await fs.promises.mkdir(path.join(testAtlasDir, 'context'), { recursive: true });
    await fs.promises.mkdir(path.join(testAtlasDir, 'adapters'), { recursive: true });

    // Write atlas.json
    const atlasManifest = {
      atlas_version: '0.1',
      metadata: {
        id: 'test-atlas',
        version: '1.0.0',
        name: 'Test Atlas',
        description: 'Atlas for testing',
      },
      domains: [
        {
          id: 'test.domain',
          name: 'Test Domain',
          description: 'A test domain',
        },
      ],
      context_packs: [
        {
          id: 'test-context',
          name: 'Test Context',
          domain: 'test.domain',
          source: 'context/test.md',
          format: 'markdown',
          priority: 100,
          tags: ['test'],
          evidence_sources: [],
        },
      ],
      actions: [
        {
          id: 'test.action',
          type: 'test.action.type',
          name: 'Test Action',
          description: 'A test action',
          domain: 'test.domain',
          risk_tier: 'low',
          schema: {
            type: 'object',
            properties: {
              input: { type: 'string' },
            },
          },
          examples: [],
        },
      ],
      policies: [
        {
          id: 'test-policy',
          name: 'Test Policy',
          version: '1.0',
          rules: [
            {
              id: 'allow-low-risk',
              name: 'Allow Low Risk',
              priority: 100,
              condition: {
                type: 'risk_tier',
                operator: 'eq',
                value: 'low',
              },
              effect: 'allow',
            },
          ],
        },
      ],
      adapters: [],
    };

    await fs.promises.writeFile(
      path.join(testAtlasDir, 'atlas.json'),
      JSON.stringify(atlasManifest, null, 2)
    );

    // Write context file
    await fs.promises.writeFile(
      path.join(testAtlasDir, 'context', 'test.md'),
      '# Test Context\n\nThis is test context content for the test domain.'
    );

    runtime = new CRARuntime({
      session_id: 'atlas-test-session',
      trace_dir: testTraceDir,
      trace_to_file: false,
    });
  });

  afterEach(async () => {
    await runtime.shutdown();

    // Cleanup
    try {
      await fs.promises.rm(testAtlasDir, { recursive: true, force: true });
      await fs.promises.rm(testTraceDir, { recursive: true, force: true });
    } catch {
      // Ignore
    }
  });

  describe('Atlas Loading', () => {
    it('should load a valid atlas', async () => {
      const atlas = await runtime.loadAtlas(testAtlasDir);

      expect(atlas.ref).toBe('test-atlas@1.0.0');
      expect(atlas.manifest.domains).toHaveLength(1);
      expect(atlas.manifest.context_packs).toHaveLength(1);
      expect(atlas.manifest.actions).toHaveLength(1);
    });

    it('should emit atlas.load events', async () => {
      await runtime.loadAtlas(testAtlasDir);

      const events = runtime.getTrace().getEvents();
      const loadStarted = events.find(e => e.event_type === 'atlas.load.started');
      const loadCompleted = events.find(e => e.event_type === 'atlas.load.completed');

      expect(loadStarted).toBeDefined();
      expect(loadCompleted).toBeDefined();
      expect(loadCompleted?.payload.atlas_ref).toBe('test-atlas@1.0.0');
    });

    it('should update statistics on load', async () => {
      await runtime.loadAtlas(testAtlasDir);
      const stats = runtime.getStats();

      expect(stats.atlases_loaded).toBe(1);
    });
  });

  describe('Resolution with Atlas', () => {
    beforeEach(async () => {
      await runtime.loadAtlas(testAtlasDir);
    });

    it('should resolve with context blocks', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: {
          goal: 'Perform test action',
          risk_tier: 'low',
          context_hints: ['test.domain'],
        },
      });

      const result = await runtime.resolve(request);

      expect('resolution_id' in result).toBe(true);
      const resolution = result as CARPResolution;

      expect(resolution.context_blocks.length).toBeGreaterThan(0);
      expect(resolution.context_blocks[0].domain).toBe('test.domain');
    });

    it('should resolve with allowed actions', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: {
          goal: 'Perform test action',
          risk_tier: 'low',
          context_hints: ['test.domain'],
        },
      });

      const result = await runtime.resolve(request);
      const resolution = result as CARPResolution;

      expect(resolution.allowed_actions.length).toBeGreaterThan(0);
      expect(resolution.allowed_actions[0].action_type).toBe('test.action.type');
    });

    it('should return allow decision for low risk', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: {
          goal: 'Perform test action',
          risk_tier: 'low',
          context_hints: ['test.domain'],
        },
      });

      const result = await runtime.resolve(request);
      const resolution = result as CARPResolution;

      expect(resolution.decision.type).toBe('allow');
    });

    it('should include telemetry link', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: { goal: 'Test', context_hints: ['test.domain'] },
      });

      const result = await runtime.resolve(request);
      const resolution = result as CARPResolution;

      expect(resolution.telemetry_link).toBeDefined();
      expect(resolution.telemetry_link.trace_id).toBe(runtime.getTrace().getTraceId());
    });

    it('should emit resolution completed event', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: { goal: 'Test', context_hints: ['test.domain'] },
      });

      await runtime.resolve(request);

      const events = runtime.getTrace().getEvents();
      const completedEvent = events.find(e => e.event_type === 'carp.resolution.completed');

      expect(completedEvent).toBeDefined();
      expect(completedEvent?.payload.decision_type).toBe('allow');
      expect(completedEvent?.payload.context_blocks_count).toBeGreaterThan(0);
      expect(completedEvent?.payload.allowed_actions_count).toBeGreaterThan(0);
    });

    it('should cache resolutions', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: { goal: 'Test', context_hints: ['test.domain'] },
      });

      const result1 = await runtime.resolve(request);
      const result2 = await runtime.resolve(request);

      // Same resolution should be returned from cache
      expect((result1 as CARPResolution).resolution_id).toBe((result2 as CARPResolution).resolution_id);

      const events = runtime.getTrace().getEvents();
      const cacheHit = events.find(e => e.event_type === 'carp.resolution.cache_hit');
      expect(cacheHit).toBeDefined();
    });
  });

  describe('High Risk Resolution', () => {
    beforeEach(async () => {
      await runtime.loadAtlas(testAtlasDir);
    });

    it('should return allow_with_constraints for high risk', async () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'atlas-test-session',
      }, {
        task: {
          goal: 'Perform risky action',
          risk_tier: 'high',
          context_hints: ['test.domain'],
        },
      });

      const result = await runtime.resolve(request);
      const resolution = result as CARPResolution;

      expect(resolution.decision.type).toBe('allow_with_constraints');
    });
  });
});

describe('Event Analysis', () => {
  it('should produce analyzable trace output', async () => {
    const runtime = new CRARuntime({
      session_id: 'analysis-session',
      trace_to_file: false,
    });

    const request = createRequest('resolve', {
      agent_id: 'analyzer',
      session_id: 'analysis-session',
    }, {
      task: { goal: 'Analyze this' },
    });

    await runtime.resolve(request);
    await runtime.shutdown();

    const events = runtime.getTrace().getEvents();

    // Analyze event types
    const eventTypes = new Map<string, number>();
    for (const event of events) {
      const count = eventTypes.get(event.event_type) || 0;
      eventTypes.set(event.event_type, count + 1);
    }

    // Should have standard event types
    expect(eventTypes.has('session.started')).toBe(true);
    expect(eventTypes.has('session.ended')).toBe(true);
    expect(eventTypes.has('carp.request.received')).toBe(true);

    // Analyze severities
    const severities = new Map<string, number>();
    for (const event of events) {
      const count = severities.get(event.severity) || 0;
      severities.set(event.severity, count + 1);
    }

    console.log('\n=== Test Results Analysis ===');
    console.log('\nEvent Type Distribution:');
    for (const [type, count] of eventTypes.entries()) {
      console.log(`  ${type}: ${count}`);
    }

    console.log('\nSeverity Distribution:');
    for (const [sev, count] of severities.entries()) {
      console.log(`  ${sev}: ${count}`);
    }

    console.log(`\nTotal Events: ${events.length}`);
    console.log('Hash Chain Verified:', runtime.getTrace().verify().valid);
    console.log('===========================\n');
  });
});
