/**
 * CRA Protocol Package
 *
 * Core type definitions and utilities for CARP and TRACE protocols.
 */

// Re-export CARP types and utilities
export * from './carp/types.js';
export {
  CARP_VERSION,
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
} from './carp/utils.js';

// Re-export TRACE types and utilities
export * from './trace/types.js';
export {
  TRACE_VERSION,
  DEFAULT_SOURCE,
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
  getNextSequence,
  resetSequence,
  canonicalize,
} from './trace/utils.js';

// Common utilities (exported once to avoid conflicts)
export {
  generateId,
  getTimestamp,
  computeHash,
  estimateTokens,
} from './carp/utils.js';
