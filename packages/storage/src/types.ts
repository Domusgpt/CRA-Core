/**
 * Storage Types
 *
 * Core interfaces for CRA persistence layer.
 */

import type { CARPResolution, TRACEEvent } from '@cra/protocol';

/**
 * Base store interface
 */
export interface Store {
  /** Initialize the store */
  init(): Promise<void>;

  /** Close the store */
  close(): Promise<void>;

  /** Check if store is ready */
  isReady(): boolean;
}

/**
 * Store configuration
 */
export interface StoreConfig {
  /** Storage backend type */
  backend: 'memory' | 'sqlite' | 'postgresql';

  /** Connection string or path */
  connection?: string;

  /** Enable debug logging */
  debug?: boolean;
}

/**
 * Resolution record stored in database
 */
export interface ResolutionRecord {
  /** Resolution ID */
  resolution_id: string;

  /** Session ID that owns this resolution */
  session_id: string;

  /** Agent ID that requested this resolution */
  agent_id: string;

  /** The full CARP resolution response */
  resolution: CARPResolution;

  /** Creation timestamp */
  created_at: string;

  /** Expiration timestamp */
  expires_at: string;

  /** Whether resolution is still valid */
  is_valid: boolean;

  /** Number of times this resolution was used */
  use_count: number;

  /** Last used timestamp */
  last_used_at?: string;
}

/**
 * Session record stored in database
 */
export interface SessionRecord {
  /** Session ID */
  session_id: string;

  /** Agent ID that owns this session */
  agent_id: string;

  /** Session state */
  state: 'active' | 'idle' | 'terminated';

  /** Session metadata */
  metadata: Record<string, unknown>;

  /** Creation timestamp */
  created_at: string;

  /** Last activity timestamp */
  last_activity_at: string;

  /** Termination timestamp */
  terminated_at?: string;

  /** Total resolutions in this session */
  resolution_count: number;

  /** Total actions in this session */
  action_count: number;
}

/**
 * Trace record stored in database
 */
export interface TraceRecord {
  /** Trace ID */
  trace_id: string;

  /** Session ID this trace belongs to */
  session_id: string;

  /** Span ID (if this is a span-level record) */
  span_id?: string;

  /** All events in this trace */
  events: TRACEEvent[];

  /** Creation timestamp */
  created_at: string;

  /** Last updated timestamp */
  updated_at: string;

  /** Whether the hash chain is verified */
  chain_verified: boolean;

  /** Total event count */
  event_count: number;
}

/**
 * Query options for listing records
 */
export interface QueryOptions {
  /** Maximum records to return */
  limit?: number;

  /** Offset for pagination */
  offset?: number;

  /** Sort field */
  sort_by?: string;

  /** Sort direction */
  sort_order?: 'asc' | 'desc';

  /** Filter by session ID */
  session_id?: string;

  /** Filter by agent ID */
  agent_id?: string;

  /** Filter by date range start */
  from_date?: string;

  /** Filter by date range end */
  to_date?: string;
}

/**
 * Query result with pagination info
 */
export interface QueryResult<T> {
  /** Records matching query */
  records: T[];

  /** Total count (before limit/offset) */
  total: number;

  /** Whether there are more records */
  has_more: boolean;
}
