//! Buffered Trace Collection
//!
//! Non-blocking trace collection with background flushing.
//! This separates the hot path (resolution) from the cold path (audit).
//!
//! # Design
//!
//! ```text
//! record() ──► mpsc::channel ──► Background Thread ──► TraceSink
//!    │             │                    │
//!    │             │                    ├─► MemorySink (testing)
//!    │             │                    ├─► StorageSink (persistence)
//!    │             │                    └─► HttpSink (remote collection)
//!    │             │
//!    └─ Non-blocking (returns immediately)
//! ```

use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::Utc;
use serde::Serialize;
use sha2::{Sha256, Digest};

use crate::trace::{TRACEEvent, EventType};

/// Commands sent to the background flush thread
enum TraceCommand {
    Record(TRACEEvent),
    Flush,
    Shutdown,
}

/// Configuration for the buffered collector
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum events to buffer before forcing a flush
    pub buffer_size: usize,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            flush_interval_ms: 1000,
        }
    }
}

/// Trait for trace output destinations
pub trait TraceSink: Send + 'static {
    /// Write a batch of events
    fn write(&mut self, events: &[TRACEEvent]) -> Result<(), String>;

    /// Flush any pending writes
    fn flush(&mut self) -> Result<(), String>;
}

/// In-memory sink for testing
pub struct MemorySink {
    events: Arc<Mutex<Vec<TRACEEvent>>>,
}

impl MemorySink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn events(&self) -> Vec<TRACEEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceSink for MemorySink {
    fn write(&mut self, events: &[TRACEEvent]) -> Result<(), String> {
        let mut storage = self.events.lock().map_err(|e| e.to_string())?;
        storage.extend(events.iter().cloned());
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Buffered trace collector with background flushing
///
/// Non-blocking `record()` method writes to a channel.
/// Background thread batches and flushes to a sink.
pub struct BufferedCollector {
    sender: Sender<TraceCommand>,
    sequence: AtomicU64,
    session_id: String,
    trace_id: String,
    previous_hash: Mutex<String>,
    flush_thread: Option<JoinHandle<()>>,
}

impl BufferedCollector {
    /// Create a new buffered collector
    pub fn new<S: TraceSink>(
        session_id: String,
        config: BufferConfig,
        mut sink: S,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<TraceCommand>();
        let trace_id = uuid::Uuid::new_v4().to_string();

        // Spawn background flush thread
        let flush_thread = thread::spawn(move || {
            Self::flush_loop(receiver, &mut sink, config);
        });

        Self {
            sender,
            sequence: AtomicU64::new(0),
            session_id,
            trace_id,
            previous_hash: Mutex::new("genesis".to_string()),
            flush_thread: Some(flush_thread),
        }
    }

    /// Background flush loop
    fn flush_loop<S: TraceSink>(
        receiver: Receiver<TraceCommand>,
        sink: &mut S,
        config: BufferConfig,
    ) {
        let mut buffer: Vec<TRACEEvent> = Vec::with_capacity(config.buffer_size);
        let flush_duration = Duration::from_millis(config.flush_interval_ms);

        loop {
            match receiver.recv_timeout(flush_duration) {
                Ok(TraceCommand::Record(event)) => {
                    buffer.push(event);

                    // Flush if buffer is full
                    if buffer.len() >= config.buffer_size {
                        if let Err(e) = sink.write(&buffer) {
                            eprintln!("Failed to write traces: {}", e);
                        }
                        buffer.clear();
                    }
                }
                Ok(TraceCommand::Flush) => {
                    if !buffer.is_empty() {
                        if let Err(e) = sink.write(&buffer) {
                            eprintln!("Failed to write traces: {}", e);
                        }
                        buffer.clear();
                    }
                    let _ = sink.flush();
                }
                Ok(TraceCommand::Shutdown) => {
                    // Final flush
                    if !buffer.is_empty() {
                        let _ = sink.write(&buffer);
                    }
                    let _ = sink.flush();
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Periodic flush
                    if !buffer.is_empty() {
                        if let Err(e) = sink.write(&buffer) {
                            eprintln!("Failed to write traces: {}", e);
                        }
                        buffer.clear();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    }

    /// Record a trace event (non-blocking)
    ///
    /// This method returns immediately. The event is queued
    /// and will be flushed asynchronously by the background thread.
    pub fn record<T: Serialize>(&self, event_type: EventType, payload: &T) {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        let payload_json = serde_json::to_value(payload).unwrap_or_default();

        // Compute hash chain
        let mut prev_hash = self.previous_hash.lock().unwrap();
        let hash_input = format!(
            "{}:{}:{}:{}:{}",
            self.session_id,
            sequence,
            event_type,
            serde_json::to_string(&payload_json).unwrap_or_default(),
            *prev_hash
        );

        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let event_hash = hex::encode(hasher.finalize());

        let event = TRACEEvent {
            trace_version: crate::TRACE_VERSION.to_string(),
            event_id: uuid::Uuid::new_v4().to_string(),
            trace_id: self.trace_id.clone(),
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: None,
            session_id: self.session_id.clone(),
            sequence,
            timestamp: Utc::now(),
            event_type,
            payload: payload_json,
            event_hash: event_hash.clone(),
            previous_event_hash: prev_hash.clone(),
        };

        *prev_hash = event_hash;
        drop(prev_hash);

        // Non-blocking send
        let _ = self.sender.send(TraceCommand::Record(event));
    }

    /// Force a flush of buffered events
    pub fn flush(&self) {
        let _ = self.sender.send(TraceCommand::Flush);
    }

    /// Shutdown the collector and wait for flush to complete
    pub fn shutdown(mut self) {
        let _ = self.sender.send(TraceCommand::Shutdown);
        if let Some(handle) = self.flush_thread.take() {
            let _ = handle.join();
        }
    }

    /// Get the current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl Drop for BufferedCollector {
    fn drop(&mut self) {
        let _ = self.sender.send(TraceCommand::Shutdown);
        if let Some(handle) = self.flush_thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::thread::sleep;

    #[test]
    fn test_buffered_collector() {
        let sink = MemorySink::new();
        let events_ref = sink.events.clone();

        let config = BufferConfig {
            buffer_size: 10,
            flush_interval_ms: 100,
        };

        let collector = BufferedCollector::new(
            "test-session".to_string(),
            config,
            sink,
        );

        // Record some events
        collector.record(EventType::SessionStarted, &json!({"test": true}));
        collector.record(EventType::ActionRequested, &json!({"action": "test"}));

        // Force flush
        collector.flush();
        sleep(Duration::from_millis(200));

        // Check events were collected
        let events = events_ref.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 0);
        assert_eq!(events[1].sequence, 1);

        // Verify hash chain
        assert_eq!(events[0].previous_event_hash, "genesis");
        assert_eq!(events[1].previous_event_hash, events[0].event_hash);
    }

    #[test]
    fn test_auto_flush_on_buffer_full() {
        let sink = MemorySink::new();
        let events_ref = sink.events.clone();

        let config = BufferConfig {
            buffer_size: 5,
            flush_interval_ms: 10000, // Long interval
        };

        let collector = BufferedCollector::new(
            "test-session".to_string(),
            config,
            sink,
        );

        // Record more events than buffer size
        for i in 0..7 {
            collector.record(EventType::ActionRequested, &json!({"i": i}));
        }

        // Give time for buffer flush
        sleep(Duration::from_millis(100));

        // Should have flushed first 5
        let events = events_ref.lock().unwrap();
        assert!(events.len() >= 5);
    }
}
