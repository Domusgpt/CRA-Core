//! TRACE Event Collector
//!
//! The collector manages trace events for sessions, maintaining the hash chain
//! and providing access to events for auditing and replay.

use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

use crate::error::{CRAError, Result};

use super::{
    chain::{ChainVerification, ChainVerifier},
    event::{EventType, TRACEEvent},
    GENESIS_HASH,
};

/// Session trace state
#[derive(Debug)]
struct SessionTrace {
    /// Trace ID for this session
    trace_id: String,
    /// All events for this session
    events: Vec<TRACEEvent>,
    /// Current sequence number
    sequence: u64,
    /// Hash of the last event
    last_hash: String,
}

impl SessionTrace {
    fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            events: Vec::new(),
            sequence: 0,
            last_hash: GENESIS_HASH.to_string(),
        }
    }

    fn append(&mut self, mut event: TRACEEvent) -> &TRACEEvent {
        event = event.chain(self.sequence, self.last_hash.clone());
        self.last_hash = event.event_hash.clone();
        self.sequence += 1;
        self.events.push(event);
        self.events.last().unwrap()
    }
}

/// TRACE Event Collector
///
/// Collects, stores, and provides access to trace events with hash chain integrity.
pub struct TraceCollector {
    /// Session traces indexed by session ID
    sessions: HashMap<String, SessionTrace>,

    /// Optional callback for event emission (for streaming/export)
    #[allow(dead_code)]
    on_emit: Option<Box<dyn Fn(&TRACEEvent) + Send + Sync>>,
}

impl std::fmt::Debug for TraceCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceCollector")
            .field("sessions", &self.sessions)
            .field("on_emit", &self.on_emit.as_ref().map(|_| "<callback>"))
            .finish()
    }
}

impl TraceCollector {
    /// Create a new trace collector
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            on_emit: None,
        }
    }

    /// Create a collector with an event callback
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&TRACEEvent) + Send + Sync + 'static,
    {
        self.on_emit = Some(Box::new(callback));
        self
    }

    /// Emit a new trace event
    ///
    /// This creates the event, chains it to the previous event,
    /// and appends it to the session's trace.
    pub fn emit(
        &mut self,
        session_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<&TRACEEvent> {
        // Get or create session trace
        let trace_id = Uuid::new_v4().to_string();
        let session = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionTrace::new(trace_id));

        // Create and chain the event
        let event = TRACEEvent::new(
            session_id.to_string(),
            session.trace_id.clone(),
            event_type,
            payload,
        );

        // Append to chain
        let appended = session.append(event);

        // Call callback if set
        if let Some(ref callback) = self.on_emit {
            callback(appended);
        }

        Ok(appended)
    }

    /// Emit with a specific parent span
    pub fn emit_with_parent(
        &mut self,
        session_id: &str,
        parent_span_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<&TRACEEvent> {
        let trace_id = Uuid::new_v4().to_string();
        let session = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionTrace::new(trace_id));

        let event = TRACEEvent::new(
            session_id.to_string(),
            session.trace_id.clone(),
            event_type,
            payload,
        )
        .with_parent_span(parent_span_id.to_string());

        let appended = session.append(event);

        if let Some(ref callback) = self.on_emit {
            callback(appended);
        }

        Ok(appended)
    }

    /// Get all events for a session
    pub fn get_events(&self, session_id: &str) -> Result<Vec<TRACEEvent>> {
        self.sessions
            .get(session_id)
            .map(|s| s.events.clone())
            .ok_or_else(|| CRAError::SessionNotFound {
                session_id: session_id.to_string(),
            })
    }

    /// Get event count for a session
    pub fn event_count(&self, session_id: &str) -> Option<usize> {
        self.sessions.get(session_id).map(|s| s.events.len())
    }

    /// Get the last event for a session
    pub fn last_event(&self, session_id: &str) -> Option<&TRACEEvent> {
        self.sessions
            .get(session_id)
            .and_then(|s| s.events.last())
    }

    /// Get events by type
    pub fn get_events_by_type(
        &self,
        session_id: &str,
        event_type: EventType,
    ) -> Result<Vec<&TRACEEvent>> {
        let session = self.sessions.get(session_id).ok_or_else(|| {
            CRAError::SessionNotFound {
                session_id: session_id.to_string(),
            }
        })?;

        Ok(session
            .events
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect())
    }

    /// Verify the hash chain integrity for a session
    pub fn verify_chain(&self, session_id: &str) -> Result<ChainVerification> {
        let events = self.get_events(session_id)?;
        Ok(ChainVerifier::verify(&events))
    }

    /// Export events as JSONL (JSON Lines)
    pub fn export_jsonl(&self, session_id: &str) -> Result<String> {
        let events = self.get_events(session_id)?;
        let lines: Vec<String> = events
            .iter()
            .map(|e| serde_json::to_string(e).unwrap_or_default())
            .collect();
        Ok(lines.join("\n"))
    }

    /// Import events from JSONL
    pub fn import_jsonl(&mut self, session_id: &str, jsonl: &str) -> Result<usize> {
        let trace_id = Uuid::new_v4().to_string();
        let session = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionTrace::new(trace_id));

        let mut count = 0;
        for line in jsonl.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: TRACEEvent = serde_json::from_str(line).map_err(|e| {
                CRAError::InvalidTraceEvent {
                    reason: e.to_string(),
                }
            })?;
            session.events.push(event);
            count += 1;
        }

        // Update session state
        if let Some(last) = session.events.last() {
            session.sequence = last.sequence + 1;
            session.last_hash = last.event_hash.clone();
        }

        Ok(count)
    }

    /// Clear all events for a session
    pub fn clear_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Get all session IDs
    pub fn session_ids(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// Get the trace ID for a session
    pub fn trace_id(&self, session_id: &str) -> Option<&str> {
        self.sessions.get(session_id).map(|s| s.trace_id.as_str())
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_emit_event() {
        let mut collector = TraceCollector::new();

        let event = collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        assert_eq!(event.session_id, "session-1");
        assert_eq!(event.sequence, 0);
        assert_eq!(event.previous_event_hash, GENESIS_HASH);
    }

    #[test]
    fn test_event_chaining() {
        let mut collector = TraceCollector::new();

        // Emit first event
        collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        // Emit second event
        let second = collector
            .emit(
                "session-1",
                EventType::CARPRequestReceived,
                json!({"request_id": "req-1", "operation": "resolve", "goal": "test"}),
            )
            .unwrap();

        assert_eq!(second.sequence, 1);
        assert_ne!(second.previous_event_hash, GENESIS_HASH);

        // Verify chain
        let verification = collector.verify_chain("session-1").unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.event_count, 2);
    }

    #[test]
    fn test_get_events_by_type() {
        let mut collector = TraceCollector::new();

        collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        collector
            .emit(
                "session-1",
                EventType::ActionExecuted,
                json!({"action_id": "test.get", "execution_id": "exec-1", "duration_ms": 100}),
            )
            .unwrap();

        collector
            .emit(
                "session-1",
                EventType::ActionExecuted,
                json!({"action_id": "test.create", "execution_id": "exec-2", "duration_ms": 200}),
            )
            .unwrap();

        let action_events = collector
            .get_events_by_type("session-1", EventType::ActionExecuted)
            .unwrap();
        assert_eq!(action_events.len(), 2);
    }

    #[test]
    fn test_export_import_jsonl() {
        let mut collector = TraceCollector::new();

        collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        collector
            .emit(
                "session-1",
                EventType::SessionEnded,
                json!({"reason": "completed", "duration_ms": 1000}),
            )
            .unwrap();

        let jsonl = collector.export_jsonl("session-1").unwrap();
        assert!(jsonl.contains("session.started"));
        assert!(jsonl.contains("session.ended"));

        // Import into a new collector
        let mut new_collector = TraceCollector::new();
        let count = new_collector.import_jsonl("session-2", &jsonl).unwrap();
        assert_eq!(count, 2);

        let verification = new_collector.verify_chain("session-2").unwrap();
        assert!(verification.is_valid);
    }

    #[test]
    fn test_callback() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let mut collector = TraceCollector::new().with_callback(move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        collector
            .emit(
                "session-1",
                EventType::SessionEnded,
                json!({"reason": "completed", "duration_ms": 1000}),
            )
            .unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
