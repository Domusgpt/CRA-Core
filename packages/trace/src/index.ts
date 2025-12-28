/**
 * CRA TRACE Package
 *
 * TRACE collector and event streaming for audit-grade telemetry.
 */

export { TRACECollector, loadTraceFile, replayTrace } from './collector.js';
export type { CollectorOptions, SpanContext } from './collector.js';

// Redaction engine
export {
  RedactionEngine,
  createRedactionEngine,
  redact,
  type RedactionConfig,
  type RedactionPattern,
  type FieldRedactionRule,
} from './redaction.js';

// Golden trace testing
export {
  GoldenTraceManager,
  GoldenTraceAssertion,
  createGoldenTraceManager,
  type GoldenTraceConfig,
} from './golden.js';

// Re-export protocol types
export type {
  TRACEEvent,
  TRACEEventType,
  Severity,
  ArtifactReference,
  ArtifactType,
  Span,
  SpanStatus,
  SpanKind,
  TraceDiff,
  TraceDifference,
  TraceFilter,
  GoldenTraceTest,
  GoldenTraceResult,
} from '@cra/protocol';

export {
  TRACE_VERSION,
  createEvent,
  createArtifactReference,
  createSpan,
  completeSpan,
  verifyEventHash,
  verifyChain,
  verifyArtifact,
  toJsonl,
  fromJsonl,
  filterEvents,
  diffTraces,
  generateId,
  getTimestamp,
  computeHash,
} from '@cra/protocol';
