/**
 * CARP Protocol Tests
 *
 * Tests for CARP request/resolution creation, validation, and utilities.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  createRequest,
  createResolution,
  createError,
  createContextBlock,
  validateRequest,
  isResolutionExpired,
  isContextExpired,
  getDecisionType,
  isAllowDecision,
  computeGoalHash,
  generateId,
  getTimestamp,
  computeHash,
  estimateTokens,
  CARP_VERSION,
} from '../index.js';
import type { CARPRequest, CARPDecision } from '../index.js';

describe('CARP Protocol', () => {
  describe('createRequest', () => {
    it('should create a valid CARP request with UUIDv7', () => {
      const request = createRequest('resolve', {
        agent_id: 'test-agent',
        session_id: 'test-session',
      }, {
        task: { goal: 'Test goal' },
      });

      expect(request.carp_version).toBe(CARP_VERSION);
      expect(request.request_id).toMatch(/^[0-9a-f-]{36}$/);
      expect(request.operation).toBe('resolve');
      expect(request.requester.agent_id).toBe('test-agent');
      expect(request.requester.session_id).toBe('test-session');
      expect(request.task?.goal).toBe('Test goal');
      expect(request.timestamp).toBeDefined();
    });

    it('should generate goal_hash when goal is provided', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {
        task: { goal: 'Create a bug report' },
      });

      expect(request.task?.goal_hash).toBeDefined();
      expect(request.task?.goal_hash).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should include scope when provided', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {
        task: { goal: 'Test' },
        scope: {
          atlases: ['github-ops'],
          max_context_tokens: 4096,
        },
      });

      expect(request.scope?.atlases).toContain('github-ops');
      expect(request.scope?.max_context_tokens).toBe(4096);
    });

    it('should support different operation types', () => {
      const ops: Array<'resolve' | 'execute' | 'validate'> = ['resolve', 'execute', 'validate'];

      for (const op of ops) {
        const request = createRequest(op, {
          agent_id: 'test',
          session_id: 'sess',
        }, {});

        expect(request.operation).toBe(op);
      }
    });
  });

  describe('validateRequest', () => {
    it('should validate a correct request', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {
        task: { goal: 'Test' },
      });

      const result = validateRequest(request);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject request with wrong version', () => {
      const request = {
        carp_version: '2.0',
        request_id: generateId(),
        timestamp: getTimestamp(),
        operation: 'resolve',
        requester: { agent_id: 'test', session_id: 'sess' },
        task: { goal: 'Test' },
      } as CARPRequest;

      const result = validateRequest(request);
      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.includes('version'))).toBe(true);
    });

    it('should reject request without requester', () => {
      const result = validateRequest({
        carp_version: CARP_VERSION,
        request_id: generateId(),
        timestamp: getTimestamp(),
        operation: 'resolve',
      } as CARPRequest);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.includes('requester'))).toBe(true);
    });

    it('should reject request without operation', () => {
      const result = validateRequest({
        carp_version: CARP_VERSION,
        request_id: generateId(),
        timestamp: getTimestamp(),
        requester: { agent_id: 'test', session_id: 'sess' },
      } as CARPRequest);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.includes('operation'))).toBe(true);
    });
  });

  describe('createResolution', () => {
    let request: CARPRequest;

    beforeEach(() => {
      request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {
        task: { goal: 'Test goal' },
      });
    });

    it('should create a valid resolution with allow decision', () => {
      const decision: CARPDecision = { type: 'allow' };
      const resolution = createResolution(request, decision, {
        context_blocks: [],
        allowed_actions: [],
        ttl_seconds: 300,
      });

      expect(resolution.carp_version).toBe(CARP_VERSION);
      expect(resolution.resolution_id).toMatch(/^[0-9a-f-]{36}$/);
      expect(resolution.request_id).toBe(request.request_id);
      expect(resolution.decision.type).toBe('allow');
      expect(resolution.ttl.resolution_expires_at).toBeDefined();
    });

    it('should create resolution with context blocks', () => {
      const block = createContextBlock(
        'test-atlas@1.0',
        'pack-1',
        'test.domain',
        'Test content'
      );

      const resolution = createResolution(request, { type: 'allow' }, {
        context_blocks: [block],
        allowed_actions: [],
        ttl_seconds: 300,
      });

      expect(resolution.context_blocks).toHaveLength(1);
      expect(resolution.context_blocks[0].content).toBe('Test content');
      expect(resolution.context_blocks[0].token_count).toBeGreaterThan(0);
    });

    it('should create resolution with denied actions', () => {
      const resolution = createResolution(request, { type: 'deny', reason: 'Policy violation' }, {
        context_blocks: [],
        allowed_actions: [],
        denied_actions: [{
          action_type: 'dangerous.action',
          reason: 'Too risky',
          policy_refs: ['policy-1'],
        }],
        ttl_seconds: 300,
      });

      expect(resolution.decision.type).toBe('deny');
      expect(resolution.denied_actions).toHaveLength(1);
      expect(resolution.denied_actions[0].reason).toBe('Too risky');
    });
  });

  describe('createError', () => {
    it('should create a valid CARP error', () => {
      const error = createError('req-123', 'INVALID_REQUEST', 'Bad request');

      expect(error.carp_version).toBe(CARP_VERSION);
      expect(error.request_id).toBe('req-123');
      expect(error.error.code).toBe('INVALID_REQUEST');
      expect(error.error.message).toBe('Bad request');
    });

    it('should include details when provided', () => {
      const error = createError('req-123', 'POLICY_VIOLATION', 'Not allowed', {
        details: { policy_id: 'pol-1' },
      });

      expect(error.error.details?.policy_id).toBe('pol-1');
    });
  });

  describe('isResolutionExpired', () => {
    it('should return false for valid resolution', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {});

      const resolution = createResolution(request, { type: 'allow' }, {
        context_blocks: [],
        allowed_actions: [],
        ttl_seconds: 300,
      });

      expect(isResolutionExpired(resolution)).toBe(false);
    });

    it('should return true for expired resolution', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {});

      const resolution = createResolution(request, { type: 'allow' }, {
        context_blocks: [],
        allowed_actions: [],
        ttl_seconds: -1, // Already expired
      });

      expect(isResolutionExpired(resolution)).toBe(true);
    });
  });

  describe('isContextExpired', () => {
    it('should return false for valid context', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {});

      const resolution = createResolution(request, { type: 'allow' }, {
        context_blocks: [],
        allowed_actions: [],
        ttl_seconds: 300,
      });

      expect(isContextExpired(resolution)).toBe(false);
    });

    it('should return true for expired context', () => {
      const request = createRequest('resolve', {
        agent_id: 'test',
        session_id: 'sess',
      }, {});

      const resolution = createResolution(request, { type: 'allow' }, {
        context_blocks: [],
        allowed_actions: [],
        ttl_seconds: -1,
      });

      expect(isContextExpired(resolution)).toBe(true);
    });
  });

  describe('getDecisionType / isAllowDecision', () => {
    it('should identify allow decision', () => {
      const decision: CARPDecision = { type: 'allow' };

      expect(getDecisionType(decision)).toBe('allow');
      expect(isAllowDecision(decision)).toBe(true);
    });

    it('should identify deny decision', () => {
      const decision: CARPDecision = { type: 'deny', reason: 'Denied' };

      expect(getDecisionType(decision)).toBe('deny');
      expect(isAllowDecision(decision)).toBe(false);
    });

    it('should identify allow_with_constraints decision', () => {
      const decision: CARPDecision = {
        type: 'allow_with_constraints',
        constraints: [{
          id: 'c1',
          type: 'rate_limit',
          description: 'Limited',
          params: {},
          enforcement: 'hard',
        }],
      };

      expect(getDecisionType(decision)).toBe('allow_with_constraints');
      expect(isAllowDecision(decision)).toBe(true);
    });
  });

  describe('computeGoalHash', () => {
    it('should compute consistent hash for same goal', () => {
      const goal = 'Create a bug report';
      const hash1 = computeGoalHash(goal);
      const hash2 = computeGoalHash(goal);

      expect(hash1).toBe(hash2);
      expect(hash1).toMatch(/^[a-f0-9]{64}$/);
    });

    it('should compute different hash for different goals', () => {
      const hash1 = computeGoalHash('Goal A');
      const hash2 = computeGoalHash('Goal B');

      expect(hash1).not.toBe(hash2);
    });
  });

  describe('Utility Functions', () => {
    describe('generateId', () => {
      it('should generate valid UUIDs', () => {
        const id1 = generateId();
        const id2 = generateId();

        expect(id1).toMatch(/^[0-9a-f-]{36}$/);
        expect(id2).toMatch(/^[0-9a-f-]{36}$/);
        expect(id1).not.toBe(id2);
      });
    });

    describe('getTimestamp', () => {
      it('should return ISO 8601 timestamp', () => {
        const ts = getTimestamp();
        expect(ts).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
      });
    });

    describe('computeHash', () => {
      it('should compute SHA-256 hash', () => {
        const hash = computeHash('test data');
        expect(hash).toMatch(/^[a-f0-9]{64}$/);
      });

      it('should be deterministic', () => {
        const hash1 = computeHash('same input');
        const hash2 = computeHash('same input');
        expect(hash1).toBe(hash2);
      });
    });

    describe('estimateTokens', () => {
      it('should estimate tokens for text', () => {
        const tokens = estimateTokens('This is a test sentence.');
        expect(tokens).toBeGreaterThan(0);
        expect(typeof tokens).toBe('number');
      });

      it('should estimate higher tokens for longer text', () => {
        const short = estimateTokens('Short');
        const long = estimateTokens('This is a much longer sentence with more words and content');
        expect(long).toBeGreaterThan(short);
      });
    });
  });
});
