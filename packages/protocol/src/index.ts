/**
 * CRA Protocol Package
 *
 * Core type definitions and utilities for CARP and TRACE protocols.
 */

// Re-export CARP types and utilities
export * from './carp/index.js';

// Re-export TRACE types and utilities
export * from './trace/index.js';

// Protocol versions
export { CARP_VERSION } from './carp/utils.js';
export { TRACE_VERSION } from './trace/utils.js';
