/**
 * CRA Redaction Engine
 *
 * Automatically redacts sensitive data from TRACE events and payloads.
 * Supports configurable patterns, field-based rules, and custom redactors.
 */

import type { TRACEEvent } from '@cra/protocol';

/**
 * Redaction pattern configuration
 */
export interface RedactionPattern {
  /** Pattern name for reference */
  name: string;

  /** Regex pattern to match sensitive data */
  pattern: RegExp;

  /** Replacement string (can use $1, $2, etc. for capture groups) */
  replacement: string;

  /** Optional: only apply to specific field paths */
  fieldPaths?: string[];
}

/**
 * Field-based redaction rule
 */
export interface FieldRedactionRule {
  /** Field path (supports dot notation and wildcards) */
  path: string;

  /** Redaction mode */
  mode: 'full' | 'partial' | 'hash' | 'mask' | 'remove';

  /** Characters to show (for partial mode) */
  showChars?: number;

  /** Show position (for partial mode) */
  showPosition?: 'start' | 'end';
}

/**
 * Redaction configuration
 */
export interface RedactionConfig {
  /** Enable redaction */
  enabled?: boolean;

  /** Built-in patterns to enable */
  builtInPatterns?: (
    | 'email'
    | 'phone'
    | 'ssn'
    | 'credit_card'
    | 'api_key'
    | 'jwt'
    | 'ip_address'
    | 'password'
  )[];

  /** Custom regex patterns */
  customPatterns?: RedactionPattern[];

  /** Field-based rules */
  fieldRules?: FieldRedactionRule[];

  /** Fields to always redact (shorthand) */
  sensitiveFields?: string[];

  /** Redaction marker */
  redactionMarker?: string;

  /** Hash algorithm for hash mode */
  hashAlgorithm?: 'sha256' | 'sha512' | 'md5';

  /** Salt for hashing */
  hashSalt?: string;
}

/**
 * Built-in redaction patterns
 */
const BUILT_IN_PATTERNS: Record<string, RedactionPattern> = {
  email: {
    name: 'email',
    pattern: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/gi,
    replacement: '[EMAIL_REDACTED]',
  },
  phone: {
    name: 'phone',
    pattern: /\b(?:\+?1[-.]?)?\(?[0-9]{3}\)?[-.]?[0-9]{3}[-.]?[0-9]{4}\b/g,
    replacement: '[PHONE_REDACTED]',
  },
  ssn: {
    name: 'ssn',
    pattern: /\b\d{3}[-]?\d{2}[-]?\d{4}\b/g,
    replacement: '[SSN_REDACTED]',
  },
  credit_card: {
    name: 'credit_card',
    pattern: /\b(?:\d{4}[-\s]?){3}\d{4}\b/g,
    replacement: '[CARD_REDACTED]',
  },
  api_key: {
    name: 'api_key',
    pattern: /\b(sk[-_]|pk[-_]|api[-_]?key[-_]?)[a-zA-Z0-9]{20,}\b/gi,
    replacement: '[API_KEY_REDACTED]',
  },
  jwt: {
    name: 'jwt',
    pattern: /\beyJ[A-Za-z0-9-_=]+\.eyJ[A-Za-z0-9-_=]+\.[A-Za-z0-9-_.+/=]+\b/g,
    replacement: '[JWT_REDACTED]',
  },
  ip_address: {
    name: 'ip_address',
    pattern: /\b(?:\d{1,3}\.){3}\d{1,3}\b/g,
    replacement: '[IP_REDACTED]',
  },
  password: {
    name: 'password',
    pattern: /(password|passwd|pwd|secret)["']?\s*[:=]\s*["']?[^"'\s,}]+/gi,
    replacement: '$1=[REDACTED]',
  },
};

/**
 * Default sensitive field names
 */
const DEFAULT_SENSITIVE_FIELDS = [
  'password',
  'passwd',
  'secret',
  'token',
  'api_key',
  'apiKey',
  'authorization',
  'auth',
  'bearer',
  'credential',
  'private_key',
  'privateKey',
  'ssn',
  'credit_card',
  'creditCard',
  'cvv',
  'pin',
];

/**
 * Redaction Engine
 */
export class RedactionEngine {
  private readonly config: Required<RedactionConfig>;
  private readonly patterns: RedactionPattern[];
  private readonly fieldRules: Map<string, FieldRedactionRule>;
  private readonly sensitiveFieldsSet: Set<string>;

  constructor(config: RedactionConfig = {}) {
    this.config = {
      enabled: config.enabled ?? true,
      builtInPatterns: config.builtInPatterns ?? [],
      customPatterns: config.customPatterns ?? [],
      fieldRules: config.fieldRules ?? [],
      sensitiveFields: config.sensitiveFields ?? DEFAULT_SENSITIVE_FIELDS,
      redactionMarker: config.redactionMarker ?? '[REDACTED]',
      hashAlgorithm: config.hashAlgorithm ?? 'sha256',
      hashSalt: config.hashSalt ?? '',
    };

    // Compile patterns
    this.patterns = [
      ...this.config.builtInPatterns.map((name) => BUILT_IN_PATTERNS[name]).filter(Boolean),
      ...this.config.customPatterns,
    ];

    // Index field rules by path
    this.fieldRules = new Map();
    for (const rule of this.config.fieldRules) {
      this.fieldRules.set(rule.path, rule);
    }

    // Create sensitive fields set (lowercase for case-insensitive matching)
    this.sensitiveFieldsSet = new Set(
      this.config.sensitiveFields.map((f) => f.toLowerCase())
    );
  }

  /**
   * Redact a TRACE event
   */
  redactEvent(event: TRACEEvent): TRACEEvent {
    if (!this.config.enabled) {
      return event;
    }

    const redacted = { ...event };

    // Redact payload
    if (redacted.payload) {
      redacted.payload = this.redactObject(redacted.payload, 'payload');
    }

    return redacted;
  }

  /**
   * Redact multiple events
   */
  redactEvents(events: TRACEEvent[]): TRACEEvent[] {
    return events.map((e) => this.redactEvent(e));
  }

  /**
   * Redact an arbitrary object
   */
  redactObject<T extends Record<string, unknown>>(obj: T, basePath = ''): T {
    if (!this.config.enabled) {
      return obj;
    }

    const result: Record<string, unknown> = {};

    for (const [key, value] of Object.entries(obj)) {
      const fieldPath = basePath ? `${basePath}.${key}` : key;
      result[key] = this.redactValue(value, fieldPath, key);
    }

    return result as T;
  }

  /**
   * Redact a single value
   */
  redactValue(value: unknown, fieldPath: string, fieldName: string): unknown {
    // Check field rules first (explicit rules take precedence)
    const rule = this.findMatchingRule(fieldPath);
    if (rule) {
      return this.applyFieldRule(value, rule);
    }

    // Check if field should be fully redacted by name
    if (this.isSensitiveField(fieldName)) {
      return this.config.redactionMarker;
    }

    // Handle different value types
    if (typeof value === 'string') {
      return this.redactString(value);
    }

    if (Array.isArray(value)) {
      return value.map((item, index) =>
        this.redactValue(item, `${fieldPath}[${index}]`, String(index))
      );
    }

    if (value !== null && typeof value === 'object') {
      return this.redactObject(value as Record<string, unknown>, fieldPath);
    }

    return value;
  }

  /**
   * Redact a string using all patterns
   */
  redactString(value: string): string {
    if (!this.config.enabled) {
      return value;
    }

    let result = value;

    for (const pattern of this.patterns) {
      result = result.replace(pattern.pattern, pattern.replacement);
    }

    return result;
  }

  /**
   * Check if a field name is sensitive
   */
  private isSensitiveField(fieldName: string): boolean {
    return this.sensitiveFieldsSet.has(fieldName.toLowerCase());
  }

  /**
   * Find a matching field rule
   */
  private findMatchingRule(fieldPath: string): FieldRedactionRule | undefined {
    // Exact match
    if (this.fieldRules.has(fieldPath)) {
      return this.fieldRules.get(fieldPath);
    }

    // Wildcard matching
    for (const [pattern, rule] of this.fieldRules) {
      if (pattern.includes('*')) {
        const regex = new RegExp(
          '^' + pattern.replace(/\*/g, '[^.]+').replace(/\./g, '\\.') + '$'
        );
        if (regex.test(fieldPath)) {
          return rule;
        }
      }
    }

    return undefined;
  }

  /**
   * Apply a field rule to a value
   */
  private applyFieldRule(value: unknown, rule: FieldRedactionRule): unknown {
    switch (rule.mode) {
      case 'remove':
        return undefined;

      case 'full':
        return this.config.redactionMarker;

      case 'hash':
        return this.hashValue(String(value));

      case 'mask':
        return this.maskValue(String(value));

      case 'partial':
        return this.partialRedact(String(value), rule.showChars ?? 4, rule.showPosition ?? 'end');

      default:
        return value;
    }
  }

  /**
   * Hash a value
   */
  private hashValue(value: string): string {
    // Simple hash for browser/node compatibility
    // In production, use crypto.createHash
    let hash = 0;
    const str = this.config.hashSalt + value;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = (hash << 5) - hash + char;
      hash = hash & hash;
    }
    return `[HASH:${Math.abs(hash).toString(16)}]`;
  }

  /**
   * Mask a value with asterisks
   */
  private maskValue(value: string): string {
    return '*'.repeat(Math.min(value.length, 20));
  }

  /**
   * Partially redact a value
   */
  private partialRedact(value: string, showChars: number, position: 'start' | 'end'): string {
    if (value.length <= showChars) {
      return '*'.repeat(value.length);
    }

    const masked = '*'.repeat(value.length - showChars);

    if (position === 'start') {
      return value.slice(0, showChars) + masked;
    } else {
      return masked + value.slice(-showChars);
    }
  }

  /**
   * Check if a string contains sensitive patterns
   */
  hasSensitiveData(value: string): boolean {
    for (const pattern of this.patterns) {
      if (pattern.pattern.test(value)) {
        return true;
      }
    }
    return false;
  }

  /**
   * Get redaction statistics for an event
   */
  getRedactionStats(event: TRACEEvent): {
    fieldsRedacted: number;
    patternsMatched: string[];
    sensitiveFieldsFound: string[];
  } {
    const stats = {
      fieldsRedacted: 0,
      patternsMatched: new Set<string>(),
      sensitiveFieldsFound: new Set<string>(),
    };

    const analyze = (obj: unknown, path: string): void => {
      if (obj === null || obj === undefined) return;

      if (typeof obj === 'string') {
        for (const pattern of this.patterns) {
          if (pattern.pattern.test(obj)) {
            stats.patternsMatched.add(pattern.name);
            stats.fieldsRedacted++;
          }
        }
      } else if (Array.isArray(obj)) {
        obj.forEach((item, index) => analyze(item, `${path}[${index}]`));
      } else if (typeof obj === 'object') {
        for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
          const fieldPath = path ? `${path}.${key}` : key;
          if (this.isSensitiveField(key)) {
            stats.sensitiveFieldsFound.add(key);
            stats.fieldsRedacted++;
          }
          analyze(value, fieldPath);
        }
      }
    };

    analyze(event.payload, 'payload');

    return {
      fieldsRedacted: stats.fieldsRedacted,
      patternsMatched: [...stats.patternsMatched],
      sensitiveFieldsFound: [...stats.sensitiveFieldsFound],
    };
  }

  /**
   * Create a redaction engine with common security patterns
   */
  static createSecurityEngine(): RedactionEngine {
    return new RedactionEngine({
      enabled: true,
      builtInPatterns: [
        'email',
        'phone',
        'ssn',
        'credit_card',
        'api_key',
        'jwt',
        'password',
      ],
      fieldRules: [
        { path: 'headers.authorization', mode: 'full' },
        { path: 'headers.cookie', mode: 'full' },
        { path: '*.password', mode: 'full' },
        { path: '*.secret', mode: 'full' },
        { path: '*.token', mode: 'partial', showChars: 4, showPosition: 'end' },
      ],
    });
  }

  /**
   * Create a redaction engine for audit logs
   */
  static createAuditEngine(): RedactionEngine {
    return new RedactionEngine({
      enabled: true,
      builtInPatterns: ['email', 'ip_address'],
      fieldRules: [
        { path: '*.email', mode: 'hash' },
        { path: '*.ip', mode: 'partial', showChars: 3, showPosition: 'start' },
        { path: '*.user_agent', mode: 'partial', showChars: 20, showPosition: 'start' },
      ],
    });
  }
}

/**
 * Create a configured redaction engine
 */
export function createRedactionEngine(config?: RedactionConfig): RedactionEngine {
  return new RedactionEngine(config);
}

/**
 * Convenience function to redact a single value
 */
export function redact(value: string, patterns: ('email' | 'phone' | 'api_key')[] = ['email', 'phone', 'api_key']): string {
  const engine = new RedactionEngine({
    builtInPatterns: patterns,
  });
  return engine.redactString(value);
}
