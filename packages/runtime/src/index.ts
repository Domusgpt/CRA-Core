/**
 * CRA Runtime Package
 *
 * The authoritative CARP resolver and policy engine.
 */

export { CRARuntime } from './runtime.js';
export type { RuntimeConfig, RuntimeStats } from './runtime.js';

// Re-export protocol types for convenience
export type {
  CARPRequest,
  CARPResolution,
  CARPDecision,
  CARPActionRequest,
  CARPExecutionResult,
  CARPError,
  ContextBlock,
  ActionPermission,
} from '@cra/protocol';

export {
  createRequest,
  createResolution,
  createError,
  validateRequest,
  isResolutionExpired,
  CARP_VERSION,
} from '@cra/protocol';
