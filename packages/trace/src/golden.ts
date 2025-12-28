/**
 * Golden Trace Testing Framework
 *
 * Record, replay, and validate TRACE event sequences for testing.
 * Golden tests ensure consistent behavior across code changes.
 */

import { createHash } from 'crypto';
import type { TRACEEvent, GoldenAssertion, DifferenceSeverity } from '@cra/protocol';
import { TRACECollector } from './collector.js';

/**
 * Local golden trace test type for storing recorded traces
 * (Different from protocol's GoldenTraceTest which is for test configuration)
 */
export interface GoldenTraceTest {
  name: string;
  description: string;
  recorded_at: string;
  events: TRACEEvent[];
  assertions: GoldenAssertion[];
  metadata: Record<string, unknown>;
}

/**
 * Difference type for trace comparison
 */
export type DifferenceType = 'added' | 'removed' | 'modified';

/**
 * Trace difference result
 */
export interface TraceDifference {
  type: DifferenceType;
  path: string;
  expected: unknown;
  actual: unknown;
  severity: DifferenceSeverity;
  message: string;
}

/**
 * Golden trace comparison result
 */
export interface GoldenTraceResult {
  name: string;
  passed: boolean;
  differences: TraceDifference[];
  coverage: number;
  execution_time_ms: number;
}

/**
 * Golden trace configuration
 */
export interface GoldenTraceConfig {
  /** Test name */
  name: string;

  /** Test description */
  description?: string;

  /** Tags for categorization */
  tags?: string[];

  /** Fields to ignore during comparison */
  ignoreFields?: string[];

  /** Ignore timestamp differences */
  ignoreTimestamps?: boolean;

  /** Ignore hash differences */
  ignoreHashes?: boolean;

  /** Tolerance for numeric comparisons */
  numericTolerance?: number;

  /** Maximum allowed differences before failure */
  maxDifferences?: number;
}

/**
 * Recording state for golden trace capture
 */
interface RecordingState {
  name: string;
  startTime: Date;
  events: TRACEEvent[];
  metadata: Record<string, unknown>;
}

/**
 * Golden Trace Manager
 *
 * Manages recording, storage, and comparison of golden traces.
 */
export class GoldenTraceManager {
  private readonly goldenTraces: Map<string, GoldenTraceTest> = new Map();
  private recording: RecordingState | null = null;
  private readonly defaultConfig: Required<GoldenTraceConfig>;

  constructor(config: Partial<GoldenTraceConfig> = {}) {
    this.defaultConfig = {
      name: '',
      description: '',
      tags: [],
      ignoreFields: ['event_id', 'timestamp', 'event_hash'],
      ignoreTimestamps: true,
      ignoreHashes: true,
      numericTolerance: 0,
      maxDifferences: 0,
    };
    Object.assign(this.defaultConfig, config);
  }

  /**
   * Start recording events for a new golden trace
   */
  startRecording(
    collector: TRACECollector,
    name: string,
    metadata: Record<string, unknown> = {}
  ): void {
    if (this.recording) {
      throw new Error(`Already recording "${this.recording.name}". Call stopRecording() first.`);
    }

    this.recording = {
      name,
      startTime: new Date(),
      events: [],
      metadata,
    };

    // Capture events from collector
    const eventHandler = (event: TRACEEvent) => {
      if (this.recording) {
        this.recording.events.push({ ...event });
      }
    };

    collector.on('event', eventHandler);

    // Store handler for cleanup (attached to recording state)
    (this.recording as any)._eventHandler = eventHandler;
    (this.recording as any)._collector = collector;
  }

  /**
   * Stop recording and return the captured trace
   */
  stopRecording(): GoldenTraceTest {
    if (!this.recording) {
      throw new Error('Not currently recording');
    }

    const { name, startTime, events, metadata } = this.recording;
    const collector = (this.recording as any)._collector as TRACECollector;
    const eventHandler = (this.recording as any)._eventHandler;

    // Remove event listener
    if (collector && eventHandler) {
      collector.removeListener('event', eventHandler);
    }

    const goldenTrace: GoldenTraceTest = {
      name,
      description: metadata.description as string || '',
      recorded_at: startTime.toISOString(),
      events,
      assertions: [],
      metadata: {
        duration_ms: Date.now() - startTime.getTime(),
        event_count: events.length,
        ...metadata,
      },
    };

    this.recording = null;
    return goldenTrace;
  }

  /**
   * Register a golden trace for comparison
   */
  registerGolden(name: string, trace: GoldenTraceTest): void {
    this.goldenTraces.set(name, trace);
  }

  /**
   * Load golden traces from JSON
   */
  loadGoldens(traces: GoldenTraceTest[]): void {
    for (const trace of traces) {
      this.goldenTraces.set(trace.name, trace);
    }
  }

  /**
   * Get a registered golden trace
   */
  getGolden(name: string): GoldenTraceTest | undefined {
    return this.goldenTraces.get(name);
  }

  /**
   * List all registered golden traces
   */
  listGoldens(): string[] {
    return [...this.goldenTraces.keys()];
  }

  /**
   * Compare captured events against a golden trace
   */
  compare(
    name: string,
    capturedEvents: TRACEEvent[],
    config?: Partial<GoldenTraceConfig>
  ): GoldenTraceResult {
    const golden = this.goldenTraces.get(name);
    if (!golden) {
      return {
        name,
        passed: false,
        differences: [{
          type: 'removed',
          path: 'golden',
          expected: name,
          actual: null,
          severity: 'error',
          message: `Golden trace "${name}" not found`,
        }],
        coverage: 0,
        execution_time_ms: 0,
      };
    }

    const mergedConfig = { ...this.defaultConfig, ...config };
    const startTime = Date.now();

    // Normalize events for comparison
    const normalizedGolden = this.normalizeEvents(golden.events, mergedConfig);
    const normalizedCaptured = this.normalizeEvents(capturedEvents, mergedConfig);

    // Compare event sequences
    const differences = this.compareEventSequences(
      normalizedGolden,
      normalizedCaptured,
      mergedConfig
    );

    // Calculate coverage
    const coverage = this.calculateCoverage(normalizedGolden, normalizedCaptured);

    // Determine pass/fail
    const passed = differences.length <= mergedConfig.maxDifferences;

    return {
      name,
      passed,
      differences,
      coverage,
      execution_time_ms: Date.now() - startTime,
    };
  }

  /**
   * Run all registered golden tests against captured events
   */
  runAll(
    capturedEvents: TRACEEvent[],
    config?: Partial<GoldenTraceConfig>
  ): Map<string, GoldenTraceResult> {
    const results = new Map<string, GoldenTraceResult>();

    for (const name of this.goldenTraces.keys()) {
      results.set(name, this.compare(name, capturedEvents, config));
    }

    return results;
  }

  /**
   * Normalize events for comparison
   */
  private normalizeEvents(
    events: TRACEEvent[],
    config: Required<GoldenTraceConfig>
  ): TRACEEvent[] {
    return events.map(event => {
      const normalized = { ...event };

      // Remove ignored fields
      for (const field of config.ignoreFields) {
        delete (normalized as any)[field];
      }

      // Normalize timestamps if configured
      if (config.ignoreTimestamps) {
        delete (normalized as any).timestamp;
      }

      // Normalize hashes if configured
      if (config.ignoreHashes) {
        delete (normalized as any).event_hash;
      }

      // Normalize payload fields
      if (normalized.payload) {
        normalized.payload = this.normalizePayload(normalized.payload, config);
      }

      return normalized;
    });
  }

  /**
   * Normalize payload for comparison
   */
  private normalizePayload(
    payload: Record<string, unknown>,
    config: Required<GoldenTraceConfig>
  ): Record<string, unknown> {
    const normalized: Record<string, unknown> = {};

    for (const [key, value] of Object.entries(payload)) {
      if (config.ignoreFields.includes(`payload.${key}`)) {
        continue;
      }

      if (typeof value === 'number' && config.numericTolerance > 0) {
        // Round to tolerance
        normalized[key] = Math.round(value / config.numericTolerance) * config.numericTolerance;
      } else if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
        normalized[key] = this.normalizePayload(value as Record<string, unknown>, config);
      } else {
        normalized[key] = value;
      }
    }

    return normalized;
  }

  /**
   * Compare two event sequences
   */
  private compareEventSequences(
    expected: TRACEEvent[],
    actual: TRACEEvent[],
    _config: Required<GoldenTraceConfig>
  ): TraceDifference[] {
    const differences: TraceDifference[] = [];

    // Check event count
    if (expected.length !== actual.length) {
      differences.push({
        type: 'modified',
        path: 'event_count',
        expected: expected.length,
        actual: actual.length,
        severity: 'warning',
        message: `Expected ${expected.length} events, got ${actual.length}`,
      });
    }

    // Compare each event
    const maxLength = Math.max(expected.length, actual.length);
    for (let i = 0; i < maxLength; i++) {
      const expectedEvent = expected[i];
      const actualEvent = actual[i];

      if (!expectedEvent) {
        differences.push({
          type: 'added',
          path: `events[${i}]`,
          expected: null,
          actual: actualEvent?.event_type,
          severity: 'warning',
          message: `Unexpected event at index ${i}: ${actualEvent?.event_type}`,
        });
        continue;
      }

      if (!actualEvent) {
        differences.push({
          type: 'removed',
          path: `events[${i}]`,
          expected: expectedEvent.event_type,
          actual: null,
          severity: 'error',
          message: `Missing event at index ${i}: ${expectedEvent.event_type}`,
        });
        continue;
      }

      // Compare event type
      if (expectedEvent.event_type !== actualEvent.event_type) {
        differences.push({
          type: 'modified',
          path: `events[${i}].event_type`,
          expected: expectedEvent.event_type,
          actual: actualEvent.event_type,
          severity: 'error',
          message: `Event type mismatch at index ${i}`,
        });
      }

      // Compare severity
      if (expectedEvent.severity !== actualEvent.severity) {
        differences.push({
          type: 'modified',
          path: `events[${i}].severity`,
          expected: expectedEvent.severity,
          actual: actualEvent.severity,
          severity: 'warning',
          message: `Severity mismatch at index ${i}`,
        });
      }

      // Compare payload
      const payloadDiffs = this.comparePayloads(
        expectedEvent.payload || {},
        actualEvent.payload || {},
        `events[${i}].payload`
      );
      differences.push(...payloadDiffs);
    }

    return differences;
  }

  /**
   * Compare two payloads
   */
  private comparePayloads(
    expected: Record<string, unknown>,
    actual: Record<string, unknown>,
    basePath: string
  ): TraceDifference[] {
    const differences: TraceDifference[] = [];

    // Check for missing keys
    for (const key of Object.keys(expected)) {
      if (!(key in actual)) {
        differences.push({
          type: 'removed',
          path: `${basePath}.${key}`,
          expected: expected[key],
          actual: null,
          severity: 'warning',
          message: `Missing field: ${key}`,
        });
      }
    }

    // Check for added keys
    for (const key of Object.keys(actual)) {
      if (!(key in expected)) {
        differences.push({
          type: 'added',
          path: `${basePath}.${key}`,
          expected: null,
          actual: actual[key],
          severity: 'info',
          message: `Unexpected field: ${key}`,
        });
      }
    }

    // Compare values
    for (const key of Object.keys(expected)) {
      if (key in actual) {
        const expectedVal = expected[key];
        const actualVal = actual[key];

        if (typeof expectedVal !== typeof actualVal) {
          differences.push({
            type: 'modified',
            path: `${basePath}.${key}`,
            expected: expectedVal,
            actual: actualVal,
            severity: 'error',
            message: `Type mismatch for ${key}`,
          });
        } else if (
          typeof expectedVal === 'object' &&
          expectedVal !== null &&
          !Array.isArray(expectedVal)
        ) {
          differences.push(
            ...this.comparePayloads(
              expectedVal as Record<string, unknown>,
              actualVal as Record<string, unknown>,
              `${basePath}.${key}`
            )
          );
        } else if (JSON.stringify(expectedVal) !== JSON.stringify(actualVal)) {
          differences.push({
            type: 'modified',
            path: `${basePath}.${key}`,
            expected: expectedVal,
            actual: actualVal,
            severity: 'warning',
            message: `Value mismatch for ${key}`,
          });
        }
      }
    }

    return differences;
  }

  /**
   * Calculate coverage percentage
   */
  private calculateCoverage(expected: TRACEEvent[], actual: TRACEEvent[]): number {
    if (expected.length === 0) return actual.length === 0 ? 100 : 0;

    const expectedTypes = new Set(expected.map(e => e.event_type));
    const actualTypes = new Set(actual.map(e => e.event_type));

    let matchedTypes = 0;
    for (const type of expectedTypes) {
      if (actualTypes.has(type)) {
        matchedTypes++;
      }
    }

    return Math.round((matchedTypes / expectedTypes.size) * 100);
  }

  /**
   * Generate a fingerprint for a trace
   */
  fingerprint(events: TRACEEvent[]): string {
    const normalized = events.map(e => ({
      event_type: e.event_type,
      severity: e.severity,
      payload_keys: e.payload ? Object.keys(e.payload).sort() : [],
    }));

    const hash = createHash('sha256');
    hash.update(JSON.stringify(normalized));
    return hash.digest('hex').slice(0, 16);
  }

  /**
   * Export golden trace to JSON
   */
  exportGolden(name: string): string {
    const trace = this.goldenTraces.get(name);
    if (!trace) {
      throw new Error(`Golden trace "${name}" not found`);
    }
    return JSON.stringify(trace, null, 2);
  }

  /**
   * Export all golden traces
   */
  exportAll(): string {
    const traces = [...this.goldenTraces.values()];
    return JSON.stringify(traces, null, 2);
  }

  /**
   * Create a test assertion helper
   */
  createAssertion(
    collector: TRACECollector,
    config?: Partial<GoldenTraceConfig>
  ): GoldenTraceAssertion {
    return new GoldenTraceAssertion(this, collector, config);
  }
}

/**
 * Assertion helper for test frameworks
 */
export class GoldenTraceAssertion {
  private readonly manager: GoldenTraceManager;
  private readonly config: Partial<GoldenTraceConfig>;
  private capturedEvents: TRACEEvent[] = [];

  constructor(
    manager: GoldenTraceManager,
    collector: TRACECollector,
    config: Partial<GoldenTraceConfig> = {}
  ) {
    this.manager = manager;
    this.config = config;

    // Capture events from collector
    collector.on('event', (event) => {
      this.capturedEvents.push({ ...event });
    });
  }

  /**
   * Assert that captured events match a golden trace
   */
  matchesGolden(name: string): GoldenTraceResult {
    return this.manager.compare(name, this.capturedEvents, this.config);
  }

  /**
   * Assert that a specific event type was emitted
   */
  hasEventType(eventType: string): boolean {
    return this.capturedEvents.some(e => e.event_type === eventType);
  }

  /**
   * Assert event count
   */
  hasEventCount(count: number): boolean {
    return this.capturedEvents.length === count;
  }

  /**
   * Get all captured events
   */
  getEvents(): TRACEEvent[] {
    return [...this.capturedEvents];
  }

  /**
   * Clear captured events
   */
  clear(): void {
    this.capturedEvents = [];
  }

  /**
   * Record current events as a new golden trace
   */
  recordAsGolden(name: string, description: string = ''): GoldenTraceTest {
    return {
      name,
      description,
      recorded_at: new Date().toISOString(),
      events: [...this.capturedEvents],
      assertions: [],
      metadata: {
        event_count: this.capturedEvents.length,
        fingerprint: this.manager.fingerprint(this.capturedEvents),
      },
    };
  }
}

/**
 * Create a golden trace manager
 */
export function createGoldenTraceManager(
  config?: Partial<GoldenTraceConfig>
): GoldenTraceManager {
  return new GoldenTraceManager(config);
}
