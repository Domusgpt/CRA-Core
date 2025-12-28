//! TRACE/1.0 — Telemetry & Replay Audit Contract for Execution
//!
//! TRACE provides an immutable, append-only event log with cryptographic integrity.
//!
//! ## Core Principle
//!
//! > If it wasn't emitted by the runtime, it didn't happen.
//!
//! ## Key Properties
//!
//! - **Append-Only**: Events can only be added, never modified
//! - **Hash Chain**: Each event includes hash of previous event
//! - **Tamper-Evident**: Any modification breaks chain verification
//! - **Replayable**: Given TRACE + Atlas, can reproduce exact behavior
//! - **Diffable**: Compare traces to detect behavioral changes
//!
//! ## Architecture
//!
//! The trace system uses a lock-free ring buffer for high-throughput collection:
//!
//! ```text
//! Hot Path (sync)        Background Worker
//! ────────────────       ─────────────────
//! emit() ──────────────► RingBuffer ──────► TraceProcessor
//!   │                      (lock-free)         │
//!   └─ Returns immediately                     └─ Computes hashes
//!      No blocking                                Chains events
//!                                                 Stores to backend
//! ```

mod event;
mod collector;
mod chain;
mod replay;
mod raw;
mod buffer;
mod processor;

pub use event::{TRACEEvent, EventType, EventPayload};
pub use collector::TraceCollector;
pub use chain::{ChainVerification, ChainVerifier};
pub use replay::{ReplayEngine, ReplayResult, ReplayDiff};
pub use raw::RawEvent;
pub use buffer::{TraceRingBuffer, BufferStats};
pub use processor::{TraceProcessor, ProcessorConfig, ProcessorHandle};

/// TRACE protocol version
pub const VERSION: &str = "1.0";

/// Genesis hash - used as previous_event_hash for first event
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_trace_event_serialization() {
        let event = TRACEEvent::new(
            "session-123".to_string(),
            "trace-123".to_string(),
            EventType::SessionStarted,
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        let json = serde_json::to_string(&event).unwrap();
        let parsed: TRACEEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.session_id, "session-123");
        assert!(matches!(parsed.event_type, EventType::SessionStarted));
    }

    #[test]
    fn test_collector_and_chain() {
        let mut collector = TraceCollector::new();

        // Emit first event
        collector.emit(
            "session-1",
            EventType::SessionStarted,
            json!({"agent_id": "agent-1", "goal": "test"}),
        ).unwrap();

        // Emit second event
        collector.emit(
            "session-1",
            EventType::CARPRequestReceived,
            json!({"request_id": "req-1", "operation": "resolve", "goal": "test"}),
        ).unwrap();

        // Verify chain
        let verification = collector.verify_chain("session-1").unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.event_count, 2);
    }
}
