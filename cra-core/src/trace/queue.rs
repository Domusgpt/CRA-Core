//! Async TRACE Queue
//!
//! Provides a high-level async queue for trace events with:
//! - Configurable flush triggers (size, time, session end)
//! - Sync event support for high-risk operations
//! - Integration with ring buffer and background processor

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::error::{CRAError, Result};
use crate::storage::StorageBackend;

use super::buffer::TraceRingBuffer;
use super::event::{EventType, TRACEEvent};
use super::processor::{ProcessorConfig, TraceProcessor};
use super::raw::RawEvent;
use super::GENESIS_HASH;

/// Configuration for the async trace queue
#[derive(Debug, Clone)]
pub struct AsyncQueueConfig {
    /// Ring buffer capacity
    pub buffer_capacity: usize,

    /// Maximum events before force flush
    pub max_queue_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Batch size for processing
    pub batch_size: usize,

    /// Event types that require synchronous processing
    pub sync_event_types: HashSet<EventType>,

    /// Whether to flush on session end
    pub flush_on_session_end: bool,

    /// Poll interval for background processor
    pub poll_interval_ms: u64,
}

impl Default for AsyncQueueConfig {
    fn default() -> Self {
        let mut sync_types = HashSet::new();
        // Default sync events - high-risk operations
        sync_types.insert(EventType::SessionEnded);
        sync_types.insert(EventType::PolicyViolated);
        sync_types.insert(EventType::ActionDenied);

        Self {
            buffer_capacity: 4096,
            max_queue_size: 100,
            flush_interval_ms: 5000,
            batch_size: 50,
            sync_event_types: sync_types,
            flush_on_session_end: true,
            poll_interval_ms: 10,
        }
    }
}

impl AsyncQueueConfig {
    /// Create a minimal config (for testing)
    pub fn minimal() -> Self {
        Self {
            buffer_capacity: 100,
            max_queue_size: 10,
            flush_interval_ms: 100,
            batch_size: 10,
            sync_event_types: HashSet::new(),
            flush_on_session_end: true,
            poll_interval_ms: 5,
        }
    }

    /// Add an event type that requires sync processing
    pub fn with_sync_event(mut self, event_type: EventType) -> Self {
        self.sync_event_types.insert(event_type);
        self
    }

    /// Set buffer capacity
    pub fn with_buffer_capacity(mut self, capacity: usize) -> Self {
        self.buffer_capacity = capacity;
        self
    }

    /// Set flush interval
    pub fn with_flush_interval(mut self, ms: u64) -> Self {
        self.flush_interval_ms = ms;
        self
    }

    /// Set max queue size before force flush
    pub fn with_max_queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = size;
        self
    }
}

/// Session state for chain tracking
#[derive(Debug, Clone)]
struct SessionState {
    trace_id: String,
    sequence: u64,
    last_hash: String,
    pending_count: u64,
}

impl SessionState {
    fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            sequence: 0,
            last_hash: GENESIS_HASH.to_string(),
            pending_count: 0,
        }
    }
}

/// Async TRACE Queue
///
/// High-level queue for async trace event collection with:
/// - Background processing via ring buffer + processor
/// - Configurable flush triggers
/// - Sync event support for critical operations
pub struct AsyncTraceQueue {
    /// Ring buffer for async events
    buffer: Arc<TraceRingBuffer>,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Session states
    sessions: RwLock<std::collections::HashMap<String, SessionState>>,

    /// Configuration
    config: AsyncQueueConfig,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Last flush time
    last_flush: Mutex<Instant>,

    /// Events since last flush
    events_since_flush: AtomicU64,

    /// Background processor handle
    processor_handle: Mutex<Option<JoinHandle<()>>>,
}

impl AsyncTraceQueue {
    /// Create a new async queue
    pub fn new(storage: Arc<dyn StorageBackend>, config: AsyncQueueConfig) -> Self {
        let buffer = Arc::new(TraceRingBuffer::new(config.buffer_capacity));

        Self {
            buffer,
            storage,
            sessions: RwLock::new(std::collections::HashMap::new()),
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
            last_flush: Mutex::new(Instant::now()),
            events_since_flush: AtomicU64::new(0),
            processor_handle: Mutex::new(None),
        }
    }

    /// Create with default config
    pub fn with_defaults(storage: Arc<dyn StorageBackend>) -> Self {
        Self::new(storage, AsyncQueueConfig::default())
    }

    /// Start the background processor
    pub fn start(&self) -> Result<()> {
        let buffer = self.buffer.clone();
        let storage = self.storage.clone();
        let shutdown = self.shutdown.clone();
        let config = ProcessorConfig::default()
            .batch_size(self.config.batch_size)
            .poll_interval(Duration::from_millis(self.config.poll_interval_ms));

        let processor = TraceProcessor::new(buffer, storage, config);
        let handle = processor.start();

        // Store the handle's inner thread handle
        // Note: ProcessorHandle manages shutdown, we just need to track it
        let mut proc_handle = self.processor_handle.lock().unwrap();
        *proc_handle = Some(std::thread::spawn(move || {
            // Keep handle alive until shutdown
            while !shutdown.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
            }
            handle.join().ok();
        }));

        Ok(())
    }

    /// Emit an event (async by default, sync for configured types)
    pub fn emit(
        &self,
        session_id: &str,
        trace_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<()> {
        // Check if this event type requires sync processing
        if self.config.sync_event_types.contains(&event_type) {
            return self.emit_sync(session_id, trace_id, event_type, payload);
        }

        self.emit_async(session_id, trace_id, event_type, payload)
    }

    /// Emit an event asynchronously (non-blocking)
    fn emit_async(
        &self,
        session_id: &str,
        trace_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<()> {
        // Ensure session exists
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions
                .entry(session_id.to_string())
                .or_insert_with(|| SessionState::new(trace_id.to_string()));
        }

        // Create raw event and push to buffer
        let raw = RawEvent::new(
            session_id.to_string(),
            trace_id.to_string(),
            event_type,
            payload.clone(),
        );

        if !self.buffer.push(raw) {
            // Buffer full - force flush
            self.check_flush_triggers(true)?;

            // Retry
            let raw = RawEvent::new(
                session_id.to_string(),
                trace_id.to_string(),
                event_type,
                payload,
            );
            if !self.buffer.push(raw) {
                return Err(CRAError::InternalError {
                    reason: "Trace buffer full even after flush".to_string(),
                });
            }
        }

        // Increment counter
        self.events_since_flush.fetch_add(1, Ordering::Relaxed);

        // Check flush triggers
        self.check_flush_triggers(false)?;

        Ok(())
    }

    /// Emit an event synchronously (blocking, computes hash immediately)
    fn emit_sync(
        &self,
        session_id: &str,
        trace_id: &str,
        event_type: EventType,
        payload: Value,
    ) -> Result<()> {
        // Get/create session state
        let (sequence, previous_hash) = {
            let mut sessions = self.sessions.write().unwrap();
            let state = sessions
                .entry(session_id.to_string())
                .or_insert_with(|| SessionState::new(trace_id.to_string()));

            let seq = state.sequence;
            let prev = state.last_hash.clone();

            state.sequence += 1;

            (seq, prev)
        };

        // Create and chain event
        let event = TRACEEvent::new(
            session_id.to_string(),
            trace_id.to_string(),
            event_type,
            payload,
        )
        .chain(sequence, previous_hash);

        // Update session state with new hash
        {
            let mut sessions = self.sessions.write().unwrap();
            if let Some(state) = sessions.get_mut(session_id) {
                state.last_hash = event.event_hash.clone();
            }
        }

        // Store immediately
        self.storage.store_event(&event)?;

        Ok(())
    }

    /// Check and execute flush triggers
    fn check_flush_triggers(&self, force: bool) -> Result<()> {
        let should_flush = force || {
            let events = self.events_since_flush.load(Ordering::Relaxed);
            let elapsed = self.last_flush.lock().unwrap().elapsed();

            // Check size trigger
            events >= self.config.max_queue_size as u64
                // Check time trigger
                || elapsed >= Duration::from_millis(self.config.flush_interval_ms)
        };

        if should_flush {
            self.flush()?;
        }

        Ok(())
    }

    /// Flush pending events
    pub fn flush(&self) -> Result<()> {
        // Reset counters
        self.events_since_flush.store(0, Ordering::Relaxed);
        *self.last_flush.lock().unwrap() = Instant::now();

        // The background processor will drain the buffer
        // We just wait for it to catch up
        let start = Instant::now();
        let timeout = Duration::from_secs(5);

        while !self.buffer.is_empty() && start.elapsed() < timeout {
            std::thread::sleep(Duration::from_millis(10));
        }

        if !self.buffer.is_empty() {
            return Err(CRAError::InternalError {
                reason: format!(
                    "Flush timeout: {} events still pending",
                    self.buffer.len()
                ),
            });
        }

        Ok(())
    }

    /// End a session (flushes if configured)
    pub fn end_session(&self, session_id: &str) -> Result<()> {
        if self.config.flush_on_session_end {
            self.flush()?;
        }

        // Remove session state
        self.sessions.write().unwrap().remove(session_id);

        Ok(())
    }

    /// Shutdown the queue
    pub fn shutdown(&self) -> Result<()> {
        self.shutdown.store(true, Ordering::Relaxed);

        // Flush remaining events
        self.flush()?;

        // Wait for processor to stop
        if let Some(handle) = self.processor_handle.lock().unwrap().take() {
            handle.join().map_err(|_| CRAError::InternalError {
                reason: "Failed to join processor thread".to_string(),
            })?;
        }

        Ok(())
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        let buffer_stats = self.buffer.stats();

        QueueStats {
            buffer_len: buffer_stats.current_len,
            buffer_capacity: buffer_stats.capacity,
            buffer_pressure: buffer_stats.pressure,
            total_pushed: buffer_stats.total_pushed,
            total_drained: buffer_stats.total_drained,
            dropped: buffer_stats.dropped,
            events_since_flush: self.events_since_flush.load(Ordering::Relaxed),
            session_count: self.sessions.read().unwrap().len(),
        }
    }

    /// Check if a session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.read().unwrap().contains_key(session_id)
    }

    /// Get the number of active sessions
    pub fn session_count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }
}

impl Drop for AsyncTraceQueue {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    /// Current buffer length
    pub buffer_len: usize,
    /// Buffer capacity
    pub buffer_capacity: usize,
    /// Buffer pressure (0.0 - 1.0)
    pub buffer_pressure: f32,
    /// Total events pushed
    pub total_pushed: u64,
    /// Total events drained/processed
    pub total_drained: u64,
    /// Events dropped due to full buffer
    pub dropped: u64,
    /// Events since last flush
    pub events_since_flush: u64,
    /// Number of active sessions
    pub session_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use serde_json::json;

    #[test]
    fn test_async_queue_basic() {
        let storage = Arc::new(InMemoryStorage::new());
        let config = AsyncQueueConfig::minimal();
        let queue = AsyncTraceQueue::new(storage.clone(), config);

        queue.start().unwrap();

        // Emit some events
        queue
            .emit(
                "session-1",
                "trace-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1"}),
            )
            .unwrap();

        queue
            .emit(
                "session-1",
                "trace-1",
                EventType::ActionExecuted,
                json!({"action_id": "test"}),
            )
            .unwrap();

        // Flush and wait
        queue.flush().unwrap();

        // Give processor time
        std::thread::sleep(Duration::from_millis(100));

        queue.shutdown().unwrap();

        // Check events were stored
        let events = storage.get_events("session-1").unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_sync_events() {
        let storage = Arc::new(InMemoryStorage::new());
        let config = AsyncQueueConfig::minimal()
            .with_sync_event(EventType::ActionDenied);
        let queue = AsyncTraceQueue::new(storage.clone(), config);

        // Don't start processor - sync events should still work

        // Emit sync event
        queue
            .emit(
                "session-1",
                "trace-1",
                EventType::ActionDenied,
                json!({"action_id": "test", "reason": "policy"}),
            )
            .unwrap();

        // Should be stored immediately (sync)
        let events = storage.get_events("session-1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::ActionDenied);
    }

    #[test]
    fn test_queue_stats() {
        let storage = Arc::new(InMemoryStorage::new());
        let config = AsyncQueueConfig::minimal();
        let queue = AsyncTraceQueue::new(storage, config);

        queue.start().unwrap();

        // Emit events
        for i in 0..5 {
            queue
                .emit(
                    "session-1",
                    "trace-1",
                    EventType::ActionExecuted,
                    json!({"index": i}),
                )
                .unwrap();
        }

        let stats = queue.stats();
        assert!(stats.total_pushed >= 5);
        assert_eq!(stats.session_count, 1);

        queue.shutdown().unwrap();
    }

    #[test]
    fn test_session_end_flush() {
        let storage = Arc::new(InMemoryStorage::new());
        let mut config = AsyncQueueConfig::minimal();
        config.flush_on_session_end = true;
        let queue = AsyncTraceQueue::new(storage.clone(), config);

        queue.start().unwrap();

        queue
            .emit(
                "session-1",
                "trace-1",
                EventType::SessionStarted,
                json!({"agent_id": "agent-1"}),
            )
            .unwrap();

        // End session should flush
        queue.end_session("session-1").unwrap();

        std::thread::sleep(Duration::from_millis(50));

        queue.shutdown().unwrap();

        // Event should be stored
        let events = storage.get_events("session-1").unwrap();
        assert!(!events.is_empty());
    }
}
