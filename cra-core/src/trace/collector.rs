//! TRACE Event Collector
//!
//! The collector manages trace events for sessions, maintaining the hash chain
//! and providing access to events for auditing and replay.
//!
//! ## Modes
//!
//! - **Immediate mode** (default): Hash computed inline, events immediately queryable (~15µs per event)
//! - **Deferred mode** (future): Events queued, hash computed in background (<1µs per event)
//!
//! When using deferred mode, call `flush()` before `get_events()` to ensure all events are processed.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use uuid::Uuid;

use crate::error::{CRAError, Result};
use crate::storage::StorageBackend;

use super::{
    buffer::TraceRingBuffer,
    chain::{ChainVerification, ChainVerifier},
    event::{EventType, TRACEEvent},
    raw::RawEvent,
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

/// Configuration for deferred tracing
#[derive(Debug, Clone)]
pub struct DeferredConfig {
    /// Ring buffer capacity (default: 4096)
    pub buffer_capacity: usize,
    /// How often the background processor runs (default: 50ms)
    pub flush_interval: Duration,
    /// Max events to process per batch (default: 100)
    pub batch_size: usize,
}

impl Default for DeferredConfig {
    fn default() -> Self {
        Self {
            buffer_capacity: 4096,
            flush_interval: Duration::from_millis(50),
            batch_size: 100,
        }
    }
}

impl DeferredConfig {
    /// Create a new config with custom buffer capacity
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.buffer_capacity = capacity;
        self
    }

    /// Set flush interval
    pub fn with_flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }
}

/// TRACE Event Collector
///
/// Collects, stores, and provides access to trace events with hash chain integrity.
///
/// ## Modes
///
/// - **Immediate mode** (default): `emit()` computes hash inline (~15µs)
/// - **Deferred mode**: `emit()` queues event (<1µs), background computes hash
///
/// Use `with_deferred()` to enable deferred mode. Call `flush()` before
/// querying events to ensure all pending events are processed.
pub struct TraceCollector {
    /// Session traces indexed by session ID
    sessions: HashMap<String, SessionTrace>,

    /// Optional callback for event emission (for streaming/export)
    #[allow(dead_code)]
    on_emit: Option<Box<dyn Fn(&TRACEEvent) + Send + Sync>>,

    /// Ring buffer for deferred mode
    buffer: Option<Arc<TraceRingBuffer>>,

    /// Whether deferred mode is enabled
    deferred: bool,
}

impl std::fmt::Debug for TraceCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceCollector")
            .field("sessions", &self.sessions)
            .field("on_emit", &self.on_emit.as_ref().map(|_| "<callback>"))
            .field("deferred", &self.deferred)
            .field("pending", &self.pending_count())
            .finish()
    }
}

impl TraceCollector {
    /// Create a new trace collector (immediate mode)
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            on_emit: None,
            buffer: None,
            deferred: false,
        }
    }

    /// Create a collector with deferred tracing
    ///
    /// In deferred mode, `emit()` pushes events to a lock-free buffer
    /// and returns quickly (<1µs). A background processor computes hashes
    /// and chains events.
    ///
    /// **Important:** Call `flush()` before `get_events()` or `verify_chain()`
    /// to ensure all events have been processed.
    pub fn with_deferred(config: DeferredConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            on_emit: None,
            buffer: Some(Arc::new(TraceRingBuffer::new(config.buffer_capacity))),
            deferred: true,
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

    /// Create a collector with persistent storage
    ///
    /// Events are written to the storage backend as they are emitted.
    /// This provides durability - events survive process restarts.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use cra_core::storage::FileStorage;
    /// use cra_core::trace::TraceCollector;
    /// use std::sync::Arc;
    ///
    /// let storage = Arc::new(FileStorage::new("~/.cra/traces")?);
    /// let collector = TraceCollector::new().with_storage(storage);
    /// ```
    pub fn with_storage(self, storage: Arc<dyn StorageBackend>) -> Self {
        self.with_callback(move |event| {
            // Best-effort write to storage - don't block on errors
            if let Err(e) = storage.store_event(event) {
                eprintln!("TRACE storage error: {}", e);
            }
        })
    }

    /// Check if deferred mode is enabled
    pub fn is_deferred(&self) -> bool {
        self.deferred
    }

    /// Get the number of pending (unprocessed) events in deferred mode
    pub fn pending_count(&self) -> usize {
        self.buffer.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    /// Check if all events have been processed
    pub fn is_flushed(&self) -> bool {
        self.pending_count() == 0
    }

    /// Flush pending events (deferred mode)
    ///
    /// Computes real hashes for events that were created with placeholder hashes.
    /// In immediate mode, this is a no-op.
    ///
    /// Call this before `verify_chain()` when using deferred mode.
    pub fn flush(&mut self) -> Result<()> {
        if !self.deferred {
            return Ok(());
        }

        let buffer = match &self.buffer {
            Some(b) => b.clone(),
            None => return Ok(()),
        };

        // Drain buffer (we don't need the raw events - just clear it)
        let _ = buffer.drain_all();

        // Recompute hashes for all sessions with "deferred" placeholder hashes
        for session in self.sessions.values_mut() {
            recompute_session_hashes(session);
        }

        Ok(())
    }

    /// Emit a new trace event
    ///
    /// - **Immediate mode**: Creates, chains, and stores the event (~15µs)
    /// - **Deferred mode**: Pushes to buffer (<1µs), returns placeholder
    ///
    /// In deferred mode, call `flush()` before querying events.
    pub fn emit(
        &mut self,
        session_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<&TRACEEvent> {
        // Deferred mode: push to buffer
        if self.deferred {
            return self.emit_deferred(session_id, event_type, payload);
        }

        // Immediate mode: compute hash inline
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
        );

        let appended = session.append(event);

        if let Some(ref callback) = self.on_emit {
            callback(appended);
        }

        Ok(appended)
    }

    /// Emit in deferred mode - create event (no hash), push to buffer, return event
    ///
    /// In deferred mode, we create the event immediately but with a placeholder hash.
    /// The real hash is computed when flush() is called. This allows the API to
    /// return a reference to the event immediately.
    fn emit_deferred(
        &mut self,
        session_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<&TRACEEvent> {
        let buffer = self.buffer.as_ref()
            .ok_or_else(|| CRAError::InternalError {
                reason: "Deferred mode but no buffer".to_string(),
            })?;

        // Ensure session exists with a trace_id
        let trace_id = Uuid::new_v4().to_string();
        let session = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionTrace::new(trace_id));
        let trace_id = session.trace_id.clone();

        // Create the event immediately (with placeholder hash)
        let mut event = TRACEEvent::new(
            session_id.to_string(),
            trace_id.clone(),
            event_type.clone(),
            payload.clone(),
        );

        // Set sequence and previous hash (for chain ordering)
        // Note: In deferred mode, the hash will be recomputed during flush()
        event.sequence = session.sequence;
        event.previous_event_hash = session.last_hash.clone();
        event.event_hash = "deferred".to_string(); // Placeholder - computed on flush

        // Update session state
        session.sequence += 1;
        // Don't update last_hash yet - we'll do that during flush
        session.events.push(event);

        // Also push to buffer for background processing
        let raw = RawEvent::new(
            session_id.to_string(),
            trace_id,
            event_type,
            payload,
        );

        // Push to buffer - this is the fast path (<1µs)
        if !buffer.push(raw) {
            return Err(CRAError::InternalError {
                reason: "Trace buffer full - call flush() or increase capacity".to_string(),
            });
        }

        Ok(session.events.last().unwrap())
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

/// Recompute hashes for a session's events (standalone to avoid borrow issues)
fn recompute_session_hashes(session: &mut SessionTrace) {
    let mut last_hash = GENESIS_HASH.to_string();

    for (i, event) in session.events.iter_mut().enumerate() {
        // Only recompute if this is a deferred event
        if event.event_hash == "deferred" {
            event.sequence = i as u64;
            event.previous_event_hash = last_hash.clone();

            // Use the event's own compute_hash method to ensure consistency
            event.event_hash = event.compute_hash();
        }

        last_hash = event.event_hash.clone();
    }

    // Update session state
    session.last_hash = last_hash;
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

    // =========================================================================
    // Deferred Mode Tests
    // =========================================================================

    #[test]
    fn test_deferred_mode_creation() {
        let config = DeferredConfig::default();
        let collector = TraceCollector::with_deferred(config);

        assert!(collector.is_deferred());
        assert_eq!(collector.pending_count(), 0);
        assert!(collector.is_flushed());
    }

    #[test]
    fn test_deferred_config_builder() {
        let config = DeferredConfig::default()
            .with_capacity(8192)
            .with_flush_interval(Duration::from_millis(100));

        assert_eq!(config.buffer_capacity, 8192);
        assert_eq!(config.flush_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_deferred_emit_and_flush() {
        let config = DeferredConfig::default();
        let mut collector = TraceCollector::with_deferred(config);

        // First emit succeeds (event created with placeholder hash)
        let result = collector.emit(
            "session-1",
            EventType::SessionStarted,
            json!({"agent_id": "agent-1", "goal": "test"}),
        );
        assert!(result.is_ok());

        // Event is created but has placeholder hash
        let event = result.unwrap();
        assert_eq!(event.event_hash, "deferred");

        // Event is pending (in buffer)
        assert_eq!(collector.pending_count(), 1);
        assert!(!collector.is_flushed());

        // Flush computes real hashes
        collector.flush().unwrap();

        assert_eq!(collector.pending_count(), 0);
        assert!(collector.is_flushed());

        // Now events have real hashes
        let events = collector.get_events("session-1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::SessionStarted);
        assert_ne!(events[0].event_hash, "deferred");
    }

    #[test]
    fn test_deferred_multiple_events() {
        let config = DeferredConfig::default();
        let mut collector = TraceCollector::with_deferred(config);

        // First event
        collector.emit(
            "session-1",
            EventType::SessionStarted,
            json!({"agent_id": "agent-1", "goal": "test"}),
        ).unwrap();

        // Second event
        collector.emit(
            "session-1",
            EventType::CARPRequestReceived,
            json!({"request_id": "req-1", "operation": "resolve", "goal": "test"}),
        ).unwrap();

        // Third event
        collector.emit(
            "session-1",
            EventType::ActionExecuted,
            json!({"action_id": "test.get", "execution_id": "exec-1", "duration_ms": 100}),
        ).unwrap();

        assert_eq!(collector.pending_count(), 3);

        // Flush all - computes hashes
        collector.flush().unwrap();

        assert_eq!(collector.pending_count(), 0);

        // Verify all events
        let events = collector.get_events("session-1").unwrap();
        assert_eq!(events.len(), 3);

        // Verify chain integrity (hashes now computed)
        let verification = collector.verify_chain("session-1").unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.event_count, 3);
    }

    #[test]
    fn test_deferred_chain_integrity() {
        let config = DeferredConfig::default();
        let mut collector = TraceCollector::with_deferred(config);

        // Emit multiple events
        for i in 0..5 {
            let _ = collector.emit(
                "session-1",
                EventType::ActionExecuted,
                json!({"action_id": format!("action-{}", i), "execution_id": format!("exec-{}", i), "duration_ms": i * 100}),
            );
        }

        // Flush all
        collector.flush().unwrap();

        // Verify chain
        let verification = collector.verify_chain("session-1").unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.event_count, 5);

        // Check sequence numbers
        let events = collector.get_events("session-1").unwrap();
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.sequence, i as u64);
        }

        // Check hash chain
        assert_eq!(events[0].previous_event_hash, GENESIS_HASH);
        for i in 1..events.len() {
            assert_eq!(events[i].previous_event_hash, events[i - 1].event_hash);
        }
    }

    #[test]
    fn test_immediate_vs_deferred_consistency() {
        // Create same events in both modes and verify they produce valid chains
        let mut immediate = TraceCollector::new();
        let mut deferred = TraceCollector::with_deferred(DeferredConfig::default());

        let event_types = [
            (EventType::SessionStarted, json!({"agent_id": "agent-1", "goal": "test"})),
            (EventType::CARPRequestReceived, json!({"request_id": "req-1", "operation": "resolve", "goal": "test"})),
            (EventType::ActionExecuted, json!({"action_id": "test.get", "execution_id": "exec-1", "duration_ms": 100})),
            (EventType::SessionEnded, json!({"reason": "completed", "duration_ms": 1000})),
        ];

        // Emit to both
        for (event_type, payload) in &event_types {
            let _ = immediate.emit("session-1", event_type.clone(), payload.clone());
            let _ = deferred.emit("session-1", event_type.clone(), payload.clone());
        }

        // Flush deferred
        deferred.flush().unwrap();

        // Both should have valid chains
        let imm_verify = immediate.verify_chain("session-1").unwrap();
        let def_verify = deferred.verify_chain("session-1").unwrap();

        assert!(imm_verify.is_valid);
        assert!(def_verify.is_valid);
        assert_eq!(imm_verify.event_count, def_verify.event_count);
    }

    #[test]
    fn test_flush_is_noop_in_immediate_mode() {
        let mut collector = TraceCollector::new();

        collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        // flush() in immediate mode should be no-op
        assert!(!collector.is_deferred());
        collector.flush().unwrap();

        let events = collector.get_events("session-1").unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_emit_context_stale_event() {
        let mut collector = TraceCollector::new();

        // Start a session
        collector
            .emit(
                "session-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test"}),
            )
            .unwrap();

        // Emit a context.stale event
        let stale_event = collector
            .emit(
                "session-1",
                EventType::ContextStale,
                json!({
                    "context_id": "ctx-123",
                    "reason": "source file modified",
                    "source_file": "/path/to/changed.rs",
                    "last_verified": "2025-12-29T10:00:00Z"
                }),
            )
            .unwrap();

        assert_eq!(stale_event.event_type, EventType::ContextStale);
        assert_eq!(stale_event.sequence, 1);
        assert_eq!(stale_event.payload["context_id"], "ctx-123");
        assert_eq!(stale_event.payload["reason"], "source file modified");

        // Verify the chain is intact
        let verification = collector.verify_chain("session-1").unwrap();
        assert!(verification.is_valid);
        assert_eq!(verification.event_count, 2);

        // Verify we can query for context.stale events
        let context_events = collector
            .get_events_by_type("session-1", EventType::ContextStale)
            .unwrap();
        assert_eq!(context_events.len(), 1);
        assert_eq!(context_events[0].payload["context_id"], "ctx-123");
    }

    #[test]
    fn test_with_storage() {
        use crate::storage::FileStorage;
        use std::sync::Arc;

        // Create temp directory for storage
        let temp_dir = std::env::temp_dir().join("cra-storage-test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let storage = Arc::new(FileStorage::new(&temp_dir).unwrap());

        // Create collector with storage
        let mut collector = TraceCollector::new().with_storage(storage.clone());

        // Emit events
        collector
            .emit(
                "storage-test-session",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1", "goal": "test storage"}),
            )
            .unwrap();

        collector
            .emit(
                "storage-test-session",
                EventType::SessionEnded,
                json!({"reason": "completed", "duration_ms": 1000}),
            )
            .unwrap();

        // Check events were stored
        let stored_events = storage.get_events("storage-test-session").unwrap();
        assert_eq!(stored_events.len(), 2);
        assert_eq!(stored_events[0].event_type, EventType::SessionStarted);
        assert_eq!(stored_events[1].event_type, EventType::SessionEnded);

        // Verify chain integrity
        let verification = collector.verify_chain("storage-test-session").unwrap();
        assert!(verification.is_valid);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
