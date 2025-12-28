/**
 * TRACE Protocol utilities
 */

import { createHash } from 'crypto';
import { v7 as uuidv7 } from 'uuid';
import type {
  TRACEEvent,
  TRACEEventType,
  TRACEVersion,
  Severity,
  EventSource,
  ArtifactReference,
  ArtifactType,
  Span,
  SpanKind,
  SpanStatus,
  TraceDiff,
  TraceDifference,
  TraceFilter,
} from './types.js';

/** Current TRACE protocol version */
export const TRACE_VERSION: TRACEVersion = '1.0';

/** Default source component */
export const DEFAULT_SOURCE: EventSource = {
  component: 'cra.runtime',
  version: '0.1.0',
};

/**
 * Generate a UUIDv7 (time-ordered)
 */
export function generateId(): string {
  return uuidv7();
}

/**
 * Get current ISO 8601 timestamp with microseconds
 */
export function getTimestamp(): string {
  const now = new Date();
  return now.toISOString();
}

/**
 * Compute SHA-256 hash of content
 */
export function computeHash(content: string | Buffer): string {
  return createHash('sha256').update(content).digest('hex');
}

/**
 * Compute event hash (excluding event_hash field itself)
 */
export function computeEventHash(event: Omit<TRACEEvent, 'event_hash'>): string {
  // Create canonical form with sorted keys
  const canonical = canonicalize(event);
  return computeHash(canonical);
}

/**
 * Create canonical JSON string with sorted keys
 */
export function canonicalize(obj: unknown): string {
  return JSON.stringify(obj, (_, value) => {
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      return Object.keys(value).sort().reduce((acc, key) => {
        acc[key] = value[key];
        return acc;
      }, {} as Record<string, unknown>);
    }
    return value;
  });
}

/**
 * Sequence counter for events within a session
 */
const sequenceCounters = new Map<string, number>();

/**
 * Get next sequence number for a session
 */
export function getNextSequence(sessionId: string): number {
  const current = sequenceCounters.get(sessionId) ?? 0;
  const next = current + 1;
  sequenceCounters.set(sessionId, next);
  return next;
}

/**
 * Reset sequence counter for a session
 */
export function resetSequence(sessionId: string): void {
  sequenceCounters.delete(sessionId);
}

/**
 * Create a TRACE event
 */
export function createEvent(
  eventType: TRACEEventType,
  payload: Record<string, unknown>,
  options: {
    trace_id: string;
    span_id: string;
    session_id: string;
    parent_span_id?: string;
    severity?: Severity;
    artifacts?: ArtifactReference[];
    previous_event_hash?: string;
    source?: EventSource;
    tags?: Record<string, string>;
  }
): TRACEEvent {
  const eventWithoutHash: Omit<TRACEEvent, 'event_hash'> = {
    trace_version: TRACE_VERSION,
    event_id: generateId(),
    sequence: getNextSequence(options.session_id),
    timestamp: getTimestamp(),
    trace_id: options.trace_id,
    span_id: options.span_id,
    parent_span_id: options.parent_span_id,
    session_id: options.session_id,
    event_type: eventType,
    severity: options.severity ?? 'info',
    payload,
    artifacts: options.artifacts,
    previous_event_hash: options.previous_event_hash,
    source: options.source ?? DEFAULT_SOURCE,
    tags: options.tags,
  };

  return {
    ...eventWithoutHash,
    event_hash: computeEventHash(eventWithoutHash),
  };
}

/**
 * Create an artifact reference
 */
export function createArtifactReference(
  type: ArtifactType,
  name: string,
  content: string | Buffer,
  options: {
    mime_type?: string;
    external_ref?: string;
    expires_at?: string;
  } = {}
): ArtifactReference {
  const contentBuffer = typeof content === 'string' ? Buffer.from(content) : content;
  const contentHash = computeHash(contentBuffer);
  const size = contentBuffer.length;
  const isSmall = size < 4096; // 4KB threshold

  return {
    artifact_id: generateId(),
    type,
    name,
    content_hash: contentHash,
    size_bytes: size,
    mime_type: options.mime_type ?? 'application/octet-stream',
    storage: isSmall && !options.external_ref ? 'inline' : 'external',
    inline_content: isSmall && !options.external_ref ? contentBuffer.toString('base64') : undefined,
    external_ref: options.external_ref,
    created_at: getTimestamp(),
    expires_at: options.expires_at,
  };
}

/**
 * Create a span
 */
export function createSpan(
  name: string,
  traceId: string,
  options: {
    parent_span_id?: string;
    kind?: SpanKind;
    attributes?: Record<string, unknown>;
  } = {}
): Span {
  return {
    span_id: generateId(),
    trace_id: traceId,
    parent_span_id: options.parent_span_id,
    name,
    kind: options.kind ?? 'internal',
    started_at: getTimestamp(),
    status: 'in_progress',
    attributes: options.attributes ?? {},
    events: [],
  };
}

/**
 * Complete a span
 */
export function completeSpan(
  span: Span,
  status: SpanStatus = 'ok',
  message?: string
): Span {
  return {
    ...span,
    ended_at: getTimestamp(),
    status,
    status_message: message,
  };
}

/**
 * Verify event hash integrity
 */
export function verifyEventHash(event: TRACEEvent): boolean {
  const eventWithoutHash = { ...event };
  delete (eventWithoutHash as Record<string, unknown>).event_hash;
  return computeEventHash(eventWithoutHash as Omit<TRACEEvent, 'event_hash'>) === event.event_hash;
}

/**
 * Verify hash chain integrity
 */
export function verifyChain(events: TRACEEvent[]): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  for (let i = 0; i < events.length; i++) {
    // Verify individual event hash
    if (!verifyEventHash(events[i])) {
      errors.push(`Event ${i} (${events[i].event_id}): hash mismatch`);
    }

    // Verify chain linkage (skip first event)
    if (i > 0) {
      if (events[i].previous_event_hash !== events[i - 1].event_hash) {
        errors.push(`Event ${i} (${events[i].event_id}): chain break`);
      }
    }
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Verify artifact integrity
 */
export function verifyArtifact(artifact: ArtifactReference, content: Buffer): boolean {
  return computeHash(content) === artifact.content_hash;
}

/**
 * Format event as JSONL line
 */
export function toJsonl(event: TRACEEvent): string {
  return JSON.stringify(event);
}

/**
 * Parse JSONL line to event
 */
export function fromJsonl(line: string): TRACEEvent {
  return JSON.parse(line) as TRACEEvent;
}

/**
 * Filter events based on criteria
 */
export function filterEvents(events: TRACEEvent[], filter: TraceFilter): TRACEEvent[] {
  let result = [...events];

  // Time range filter
  if (filter.from) {
    const fromDate = new Date(filter.from);
    result = result.filter(e => new Date(e.timestamp) >= fromDate);
  }
  if (filter.to) {
    const toDate = new Date(filter.to);
    result = result.filter(e => new Date(e.timestamp) <= toDate);
  }

  // Event type filter (glob patterns)
  if (filter.event_types?.length) {
    result = result.filter(e =>
      filter.event_types!.some(pattern => matchPattern(e.event_type, pattern))
    );
  }

  // Severity filter
  if (filter.severity?.length) {
    const severityOrder = ['debug', 'info', 'warn', 'error'];
    const minIndex = Math.min(...filter.severity.map(s => severityOrder.indexOf(s)));
    result = result.filter(e => severityOrder.indexOf(e.severity) >= minIndex);
  }

  // Span filter
  if (filter.spans?.length) {
    result = result.filter(e => filter.spans!.includes(e.span_id));
  }

  // Payload match filter
  if (filter.payload_match) {
    result = result.filter(e => matchPayload(e.payload, filter.payload_match!));
  }

  // Pagination
  if (filter.offset) {
    result = result.slice(filter.offset);
  }
  if (filter.limit) {
    result = result.slice(0, filter.limit);
  }

  return result;
}

/**
 * Match glob-like pattern
 */
function matchPattern(value: string, pattern: string): boolean {
  const regex = new RegExp(
    '^' + pattern.replace(/\./g, '\\.').replace(/\*/g, '.*') + '$'
  );
  return regex.test(value);
}

/**
 * Check if payload matches filter object
 */
function matchPayload(
  payload: Record<string, unknown>,
  filter: Record<string, unknown>
): boolean {
  for (const [key, value] of Object.entries(filter)) {
    if (payload[key] !== value) {
      return false;
    }
  }
  return true;
}

/**
 * Compare two traces for differences
 */
export function diffTraces(
  expected: TRACEEvent[],
  actual: TRACEEvent[],
  options: {
    ignore_fields?: string[];
    ignore_event_types?: string[];
  } = {}
): TraceDiff {
  const differences: TraceDifference[] = [];
  const ignoreFields = new Set(options.ignore_fields ?? [
    'event_id', 'timestamp', 'event_hash', 'previous_event_hash', 'sequence'
  ]);
  const ignoreTypes = new Set(options.ignore_event_types ?? []);

  // Filter out ignored event types
  const expectedFiltered = expected.filter(e => !ignoreTypes.has(e.event_type));
  const actualFiltered = actual.filter(e => !ignoreTypes.has(e.event_type));

  // Compare event counts
  if (expectedFiltered.length !== actualFiltered.length) {
    differences.push({
      type: expectedFiltered.length > actualFiltered.length ? 'removed' : 'added',
      path: 'events.length',
      expected: expectedFiltered.length,
      actual: actualFiltered.length,
      severity: 'warning',
      message: `Event count mismatch: expected ${expectedFiltered.length}, got ${actualFiltered.length}`,
    });
  }

  // Compare individual events
  const minLen = Math.min(expectedFiltered.length, actualFiltered.length);
  for (let i = 0; i < minLen; i++) {
    const expEvent = expectedFiltered[i];
    const actEvent = actualFiltered[i];

    // Compare event type
    if (expEvent.event_type !== actEvent.event_type) {
      differences.push({
        type: 'modified',
        path: `events[${i}].event_type`,
        expected: expEvent.event_type,
        actual: actEvent.event_type,
        severity: 'error',
        message: `Event type mismatch at position ${i}`,
      });
    }

    // Compare payload
    compareObjects(
      expEvent.payload,
      actEvent.payload,
      `events[${i}].payload`,
      ignoreFields,
      differences
    );
  }

  // Summarize
  const summary = {
    events_added: Math.max(0, actualFiltered.length - expectedFiltered.length),
    events_removed: Math.max(0, expectedFiltered.length - actualFiltered.length),
    events_modified: differences.filter(d => d.type === 'modified').length,
    artifacts_changed: 0,
  };

  // Determine compatibility
  let compatibility: TraceDiff['compatibility'] = 'identical';
  if (differences.some(d => d.severity === 'error')) {
    compatibility = 'breaking';
  } else if (differences.length > 0) {
    compatibility = 'compatible';
  }

  return { summary, differences, compatibility };
}

/**
 * Deep compare two objects
 */
function compareObjects(
  expected: unknown,
  actual: unknown,
  path: string,
  ignoreFields: Set<string>,
  differences: TraceDifference[]
): void {
  if (expected === actual) return;

  if (typeof expected !== typeof actual) {
    differences.push({
      type: 'modified',
      path,
      expected,
      actual,
      severity: 'error',
      message: `Type mismatch at ${path}`,
    });
    return;
  }

  if (typeof expected === 'object' && expected !== null && actual !== null) {
    const expObj = expected as Record<string, unknown>;
    const actObj = actual as Record<string, unknown>;
    const allKeys = new Set([...Object.keys(expObj), ...Object.keys(actObj)]);

    for (const key of allKeys) {
      if (ignoreFields.has(key)) continue;

      const keyPath = `${path}.${key}`;

      if (!(key in expObj)) {
        differences.push({
          type: 'added',
          path: keyPath,
          actual: actObj[key],
          severity: 'info',
          message: `New field at ${keyPath}`,
        });
      } else if (!(key in actObj)) {
        differences.push({
          type: 'removed',
          path: keyPath,
          expected: expObj[key],
          severity: 'warning',
          message: `Missing field at ${keyPath}`,
        });
      } else {
        compareObjects(expObj[key], actObj[key], keyPath, ignoreFields, differences);
      }
    }
  } else if (expected !== actual) {
    differences.push({
      type: 'modified',
      path,
      expected,
      actual,
      severity: 'warning',
      message: `Value mismatch at ${path}`,
    });
  }
}
