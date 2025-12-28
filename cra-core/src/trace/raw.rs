//! Raw TRACE events without hash computation
//!
//! RawEvent is the unhashed form of a trace event. It is pushed to the
//! ring buffer immediately by the hot path, then processed by a background
//! worker that computes hashes and chains events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::event::EventType;

/// Raw event before hash computation
///
/// This is pushed to the ring buffer immediately without blocking.
/// The background processor will compute the hash and chain it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    /// Session this event belongs to
    pub session_id: String,

    /// Trace ID grouping related events
    pub trace_id: String,

    /// Unique identifier for this event
    pub event_id: String,

    /// Span ID for this operation
    pub span_id: String,

    /// Parent span ID for nested operations
    pub parent_span_id: Option<String>,

    /// Type of event
    pub event_type: EventType,

    /// Event-specific payload data
    pub payload: Value,

    /// When this event was created
    pub timestamp: DateTime<Utc>,
}

impl RawEvent {
    /// Create a new raw event
    pub fn new(
        session_id: String,
        trace_id: String,
        event_type: EventType,
        payload: Value,
    ) -> Self {
        Self {
            session_id,
            trace_id,
            event_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            event_type,
            payload,
            timestamp: Utc::now(),
        }
    }

    /// Set the parent span
    pub fn with_parent_span(mut self, parent_span_id: String) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// Set a specific trace ID
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = trace_id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_raw_event_creation() {
        let event = RawEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::SessionStarted,
            json!({"agent_id": "agent-1"}),
        );

        assert_eq!(event.session_id, "session-1");
        assert!(!event.event_id.is_empty());
        assert!(!event.span_id.is_empty());
    }

    #[test]
    fn test_raw_event_with_parent() {
        let event = RawEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::ActionExecuted,
            json!({}),
        )
        .with_parent_span("parent-span".to_string());

        assert_eq!(event.parent_span_id, Some("parent-span".to_string()));
    }
}
