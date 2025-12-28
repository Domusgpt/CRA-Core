/**
 * Redaction Engine Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { RedactionEngine, createRedactionEngine, redact } from '../redaction.js';
import type { TRACEEvent } from '@cra/protocol';

describe('RedactionEngine', () => {
  let engine: RedactionEngine;

  beforeEach(() => {
    engine = new RedactionEngine({
      enabled: true,
      builtInPatterns: ['email', 'phone', 'api_key', 'jwt', 'credit_card'],
    });
  });

  describe('String Redaction', () => {
    it('should redact email addresses', () => {
      const input = 'Contact me at user@example.com for more info';
      const result = engine.redactString(input);
      expect(result).toBe('Contact me at [EMAIL_REDACTED] for more info');
    });

    it('should redact multiple emails', () => {
      const input = 'From: alice@test.com, To: bob@example.org';
      const result = engine.redactString(input);
      expect(result).toBe('From: [EMAIL_REDACTED], To: [EMAIL_REDACTED]');
    });

    it('should redact phone numbers', () => {
      const input = 'Call me at 555-123-4567 or (555) 987-6543';
      const result = engine.redactString(input);
      expect(result).toContain('[PHONE_REDACTED]');
    });

    it('should redact API keys', () => {
      const input = 'Use this key: sk-abcdefghijklmnopqrstuvwxyz';
      const result = engine.redactString(input);
      expect(result).toBe('Use this key: [API_KEY_REDACTED]');
    });

    it('should redact JWTs', () => {
      const jwt = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.Gfx6VO9tcxwk6xqx9yYzSfebfeakZp5JYIgP_edcw_A';
      const input = `Bearer ${jwt}`;
      const result = engine.redactString(input);
      expect(result).toBe('Bearer [JWT_REDACTED]');
    });

    it('should redact credit card numbers', () => {
      const input = 'Card: 4111-1111-1111-1111';
      const result = engine.redactString(input);
      expect(result).toBe('Card: [CARD_REDACTED]');
    });

    it('should handle strings without sensitive data', () => {
      const input = 'Hello, this is a normal message';
      const result = engine.redactString(input);
      expect(result).toBe(input);
    });
  });

  describe('Object Redaction', () => {
    it('should redact sensitive field names', () => {
      const obj = {
        username: 'alice',
        password: 'secret123',
        email: 'alice@test.com',
      };
      const result = engine.redactObject(obj);
      expect(result.username).toBe('alice');
      expect(result.password).toBe('[REDACTED]');
      // Email field name isn't sensitive, but value should be redacted by pattern
      expect(result.email).toBe('[EMAIL_REDACTED]');
    });

    it('should redact nested objects', () => {
      const obj = {
        user: {
          name: 'Bob',
          credentials: {
            apiKey: 'sk-verysecretkey1234567890',
            token: 'secret-token',
          },
        },
      };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.user.name).toBe('Bob');
      expect(result.user.credentials.apiKey).toBe('[REDACTED]');
      expect(result.user.credentials.token).toBe('[REDACTED]');
    });

    it('should handle arrays', () => {
      const obj = {
        contacts: [
          { email: 'alice@test.com' },
          { email: 'bob@example.org' },
        ],
      };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.contacts[0].email).toBe('[EMAIL_REDACTED]');
      expect(result.contacts[1].email).toBe('[EMAIL_REDACTED]');
    });

    it('should preserve non-sensitive data', () => {
      const obj = {
        id: 123,
        name: 'Product',
        active: true,
        tags: ['a', 'b', 'c'],
      };
      const result = engine.redactObject(obj);
      expect(result).toEqual(obj);
    });
  });

  describe('Event Redaction', () => {
    it('should redact TRACE event payload', () => {
      const event: TRACEEvent = {
        event_id: 'evt-1',
        trace_id: 'trace-1',
        span_id: 'span-1',
        event_type: 'session.started',
        timestamp: new Date().toISOString(),
        severity: 'info',
        event_hash: 'hash-1',
        payload: {
          user_email: 'user@test.com',
          password: 'secret123',
        },
      };

      const result = engine.redactEvent(event);
      expect(result.payload?.user_email).toBe('[EMAIL_REDACTED]');
      expect(result.payload?.password).toBe('[REDACTED]');
      expect(result.event_id).toBe('evt-1'); // Non-payload fields unchanged
    });

    it('should handle events without payload', () => {
      const event: TRACEEvent = {
        event_id: 'evt-1',
        trace_id: 'trace-1',
        span_id: 'span-1',
        event_type: 'session.started',
        timestamp: new Date().toISOString(),
        severity: 'info',
        event_hash: 'hash-1',
      };

      const result = engine.redactEvent(event);
      expect(result).toEqual(event);
    });

    it('should redact multiple events', () => {
      const events: TRACEEvent[] = [
        {
          event_id: 'evt-1',
          trace_id: 'trace-1',
          span_id: 'span-1',
          event_type: 'user.login',
          timestamp: new Date().toISOString(),
          severity: 'info',
          event_hash: 'hash-1',
          payload: { email: 'user1@test.com' },
        },
        {
          event_id: 'evt-2',
          trace_id: 'trace-1',
          span_id: 'span-1',
          event_type: 'user.login',
          timestamp: new Date().toISOString(),
          severity: 'info',
          event_hash: 'hash-2',
          payload: { email: 'user2@test.com' },
        },
      ];

      const results = engine.redactEvents(events);
      expect(results[0].payload?.email).toBe('[EMAIL_REDACTED]');
      expect(results[1].payload?.email).toBe('[EMAIL_REDACTED]');
    });
  });

  describe('Field Rules', () => {
    it('should apply full redaction rule', () => {
      const engine = new RedactionEngine({
        fieldRules: [{ path: 'data.secret', mode: 'full' }],
      });

      const obj = { data: { secret: 'mysecret', name: 'test' } };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.data.secret).toBe('[REDACTED]');
      expect(result.data.name).toBe('test');
    });

    it('should apply partial redaction rule', () => {
      const engine = new RedactionEngine({
        fieldRules: [{ path: 'card', mode: 'partial', showChars: 4, showPosition: 'end' }],
      });

      const obj = { card: '4111111111111111' };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.card).toBe('************1111');
    });

    it('should apply partial redaction with start position', () => {
      const engine = new RedactionEngine({
        fieldRules: [{ path: 'phone', mode: 'partial', showChars: 3, showPosition: 'start' }],
      });

      const obj = { phone: '5551234567' };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.phone).toBe('555*******');
    });

    it('should apply mask redaction rule', () => {
      const engine = new RedactionEngine({
        fieldRules: [{ path: 'ssn', mode: 'mask' }],
      });

      const obj = { ssn: '123-45-6789' };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.ssn).toBe('***********');
    });

    it('should apply hash redaction rule', () => {
      const engine = new RedactionEngine({
        fieldRules: [{ path: 'user_id', mode: 'hash' }],
        hashSalt: 'test-salt',
      });

      const obj = { user_id: 'user123' };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.user_id).toMatch(/^\[HASH:[a-f0-9]+\]$/);
    });

    it('should apply wildcard field rules', () => {
      const engine = new RedactionEngine({
        fieldRules: [{ path: '*.password', mode: 'full' }],
      });

      const obj = {
        user: { password: 'secret1' },
        admin: { password: 'secret2' },
      };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.user.password).toBe('[REDACTED]');
      expect(result.admin.password).toBe('[REDACTED]');
    });
  });

  describe('Configuration', () => {
    it('should respect enabled flag', () => {
      const engine = new RedactionEngine({
        enabled: false,
        builtInPatterns: ['email'],
      });

      const input = 'Email: user@test.com';
      expect(engine.redactString(input)).toBe(input);
    });

    it('should use custom redaction marker', () => {
      const engine = new RedactionEngine({
        redactionMarker: '***HIDDEN***',
      });

      const obj = { password: 'secret' };
      const result = engine.redactObject(obj) as typeof obj;
      expect(result.password).toBe('***HIDDEN***');
    });

    it('should support custom patterns', () => {
      const engine = new RedactionEngine({
        customPatterns: [
          {
            name: 'custom_id',
            pattern: /CUS-\d{6}/g,
            replacement: '[CUSTOMER_ID]',
          },
        ],
      });

      const input = 'Customer: CUS-123456';
      expect(engine.redactString(input)).toBe('Customer: [CUSTOMER_ID]');
    });
  });

  describe('Sensitive Data Detection', () => {
    it('should detect sensitive patterns', () => {
      const engine = new RedactionEngine({
        builtInPatterns: ['email', 'phone'],
      });

      expect(engine.hasSensitiveData('Contact: user@test.com')).toBe(true);
      expect(engine.hasSensitiveData('Call 555-123-4567')).toBe(true);
      expect(engine.hasSensitiveData('Hello world')).toBe(false);
    });

    it('should provide redaction statistics', () => {
      const engine = new RedactionEngine({
        builtInPatterns: ['email'],
      });

      const event: TRACEEvent = {
        event_id: 'evt-1',
        trace_id: 'trace-1',
        span_id: 'span-1',
        event_type: 'test',
        timestamp: new Date().toISOString(),
        severity: 'info',
        event_hash: 'hash-1',
        payload: {
          email: 'user@test.com',
          password: 'secret',
        },
      };

      const stats = engine.getRedactionStats(event);
      expect(stats.patternsMatched).toContain('email');
      expect(stats.sensitiveFieldsFound).toContain('password');
      expect(stats.fieldsRedacted).toBeGreaterThan(0);
    });
  });

  describe('Factory Functions', () => {
    it('should create security engine', () => {
      const engine = RedactionEngine.createSecurityEngine();
      const input = 'Key: sk-testkey12345678901234567890 Email: test@example.com';
      const result = engine.redactString(input);
      expect(result).toContain('[API_KEY_REDACTED]');
      expect(result).toContain('[EMAIL_REDACTED]');
    });

    it('should create audit engine', () => {
      const engine = RedactionEngine.createAuditEngine();
      expect(engine.hasSensitiveData('user@test.com')).toBe(true);
      expect(engine.hasSensitiveData('192.168.1.1')).toBe(true);
    });

    it('createRedactionEngine should work', () => {
      const engine = createRedactionEngine({
        builtInPatterns: ['email'],
      });
      expect(engine.redactString('test@example.com')).toBe('[EMAIL_REDACTED]');
    });

    it('redact convenience function should work', () => {
      const result = redact('Email: user@test.com, Key: api_key_abc123def456xyz789012');
      expect(result).toContain('[EMAIL_REDACTED]');
      expect(result).toContain('[API_KEY_REDACTED]');
    });
  });
});
