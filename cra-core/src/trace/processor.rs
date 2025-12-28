//! Background trace processor
//!
//! The TraceProcessor runs in a background thread, draining raw events from
//! the ring buffer, computing their hashes, chaining them, and forwarding
//! to storage backends.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::Result;
use crate::storage::StorageBackend;

use super::buffer::TraceRingBuffer;
use super::event::TRACEEvent;
use super::raw::RawEvent;
use super::GENESIS_HASH;

/// Default batch size for processing
const DEFAULT_BATCH_SIZE: usize = 100;

/// Default poll interval when buffer is empty
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Chain state for a session
#[derive(Debug, Clone)]
struct ChainState {
    /// Current sequence number
    sequence: u64,
    /// Hash of the last event
    last_hash: String,
    /// Trace ID for this session
    trace_id: String,
}

impl ChainState {
    fn new(trace_id: String) -> Self {
        Self {
            sequence: 0,
            last_hash: GENESIS_HASH.to_string(),
            trace_id,
        }
    }
}

/// Configuration for the trace processor
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Maximum events to process in one batch
    pub batch_size: usize,
    /// How long to wait when buffer is empty
    pub poll_interval: Duration,
    /// Whether to flush on shutdown
    pub flush_on_shutdown: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            poll_interval: DEFAULT_POLL_INTERVAL,
            flush_on_shutdown: true,
        }
    }
}

impl ProcessorConfig {
    /// Set batch size
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set poll interval
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
}

/// Background trace processor
///
/// Drains events from the ring buffer, computes hashes, chains them,
/// and forwards to storage. Runs in a dedicated thread.
pub struct TraceProcessor {
    /// The ring buffer to drain from
    buffer: Arc<TraceRingBuffer>,

    /// Storage backend to write processed events
    storage: Arc<dyn StorageBackend>,

    /// Chain state per session
    chains: RwLock<HashMap<String, ChainState>>,

    /// Configuration
    config: ProcessorConfig,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Worker thread handle
    handle: Option<JoinHandle<()>>,
}

impl TraceProcessor {
    /// Create a new processor
    pub fn new(
        buffer: Arc<TraceRingBuffer>,
        storage: Arc<dyn StorageBackend>,
        config: ProcessorConfig,
    ) -> Self {
        Self {
            buffer,
            storage,
            chains: RwLock::new(HashMap::new()),
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(
        buffer: Arc<TraceRingBuffer>,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self::new(buffer, storage, ProcessorConfig::default())
    }

    /// Start the processor in a background thread
    pub fn start(mut self) -> ProcessorHandle {
        let buffer = self.buffer.clone();
        let storage = self.storage.clone();
        let chains = Arc::new(self.chains);
        let config = self.config.clone();
        let shutdown = self.shutdown.clone();

        let handle = thread::spawn(move || {
            Self::run_loop(buffer, storage, chains, config, shutdown);
        });

        self.handle = Some(handle);

        ProcessorHandle {
            shutdown: self.shutdown.clone(),
            handle: self.handle.take(),
        }
    }

    /// The main processing loop
    fn run_loop(
        buffer: Arc<TraceRingBuffer>,
        storage: Arc<dyn StorageBackend>,
        chains: Arc<RwLock<HashMap<String, ChainState>>>,
        config: ProcessorConfig,
        shutdown: Arc<AtomicBool>,
    ) {
        while !shutdown.load(Ordering::Relaxed) {
            // Drain a batch of events
            let events = buffer.drain(config.batch_size);

            if events.is_empty() {
                // Nothing to process, sleep a bit
                thread::sleep(config.poll_interval);
                continue;
            }

            // Process the batch
            for raw_event in events {
                if let Err(e) = Self::process_event(&raw_event, &chains, storage.as_ref()) {
                    // Log error but continue processing
                    eprintln!("Error processing trace event: {:?}", e);
                }
            }
        }

        // Flush remaining events on shutdown
        if config.flush_on_shutdown {
            let remaining = buffer.drain_all();
            for raw_event in remaining {
                if let Err(e) = Self::process_event(&raw_event, &chains, storage.as_ref()) {
                    eprintln!("Error processing trace event during shutdown: {:?}", e);
                }
            }
        }
    }

    /// Process a single raw event
    fn process_event(
        raw: &RawEvent,
        chains: &RwLock<HashMap<String, ChainState>>,
        storage: &dyn StorageBackend,
    ) -> Result<()> {
        // Get or create chain state
        let (sequence, previous_hash, trace_id) = {
            let mut chains = chains.write().unwrap();
            let state = chains
                .entry(raw.session_id.clone())
                .or_insert_with(|| ChainState::new(raw.trace_id.clone()));

            let seq = state.sequence;
            let prev = state.last_hash.clone();
            let tid = state.trace_id.clone();

            // Will update after computing hash
            state.sequence += 1;

            (seq, prev, tid)
        };

        // Convert to TRACEEvent with computed hash
        let mut event = TRACEEvent::new(
            raw.session_id.clone(),
            trace_id,
            raw.event_type,
            raw.payload.clone(),
        );

        // Set fields from raw event
        event.event_id = raw.event_id.clone();
        event.span_id = raw.span_id.clone();
        event.parent_span_id = raw.parent_span_id.clone();
        event.timestamp = raw.timestamp;

        // Chain the event (computes hash)
        event = event.chain(sequence, previous_hash);

        // Update chain state with new hash
        {
            let mut chains = chains.write().unwrap();
            if let Some(state) = chains.get_mut(&raw.session_id) {
                state.last_hash = event.event_hash.clone();
            }
        }

        // Store the processed event
        storage.store_event(&event)?;

        Ok(())
    }

    /// Get the chain state for a session (for verification)
    pub fn get_chain_state(&self, session_id: &str) -> Option<(u64, String)> {
        self.chains
            .read()
            .unwrap()
            .get(session_id)
            .map(|s| (s.sequence, s.last_hash.clone()))
    }

    /// Clear chain state for a session (when session ends)
    pub fn clear_session(&self, session_id: &str) {
        self.chains.write().unwrap().remove(session_id);
    }
}

/// Handle to control the running processor
pub struct ProcessorHandle {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ProcessorHandle {
    /// Signal the processor to shut down
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Wait for the processor to finish (after shutdown)
    pub fn join(mut self) -> thread::Result<()> {
        self.shutdown();
        if let Some(handle) = self.handle.take() {
            handle.join()
        } else {
            Ok(())
        }
    }

    /// Check if the processor is still running
    pub fn is_running(&self) -> bool {
        !self.shutdown.load(Ordering::Relaxed)
    }
}

impl Drop for ProcessorHandle {
    fn drop(&mut self) {
        // Signal shutdown on drop
        self.shutdown.store(true, Ordering::Relaxed);
        // Don't block on join during drop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use crate::trace::EventType;
    use serde_json::json;

    #[test]
    fn test_processor_basic() {
        let buffer = Arc::new(TraceRingBuffer::new(100));
        let storage = Arc::new(InMemoryStorage::new());

        // Push some events
        buffer.push(RawEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::SessionStarted,
            json!({"agent_id": "agent-1"}),
        ));

        buffer.push(RawEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::SessionEnded,
            json!({"reason": "completed"}),
        ));

        // Process events
        let config = ProcessorConfig::default()
            .batch_size(10)
            .poll_interval(Duration::from_millis(1));

        let processor = TraceProcessor::new(
            buffer.clone(),
            storage.clone(),
            config,
        );

        let handle = processor.start();

        // Wait a bit for processing
        thread::sleep(Duration::from_millis(50));

        // Shutdown
        handle.join().unwrap();

        // Verify events were stored
        let events = storage.get_events("session-1").unwrap();
        assert_eq!(events.len(), 2);

        // Verify chain integrity
        assert_eq!(events[0].sequence, 0);
        assert_eq!(events[0].previous_event_hash, GENESIS_HASH);
        assert_eq!(events[1].sequence, 1);
        assert_eq!(events[1].previous_event_hash, events[0].event_hash);
    }

    #[test]
    fn test_processor_multiple_sessions() {
        let buffer = Arc::new(TraceRingBuffer::new(100));
        let storage = Arc::new(InMemoryStorage::new());

        // Push events for multiple sessions
        for i in 0..3 {
            buffer.push(RawEvent::new(
                format!("session-{}", i),
                format!("trace-{}", i),
                EventType::SessionStarted,
                json!({"agent_id": format!("agent-{}", i)}),
            ));
        }

        let processor = TraceProcessor::with_defaults(buffer.clone(), storage.clone());
        let handle = processor.start();

        thread::sleep(Duration::from_millis(50));
        handle.join().unwrap();

        // Each session should have independent chain
        for i in 0..3 {
            let events = storage.get_events(&format!("session-{}", i)).unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].sequence, 0);
            assert_eq!(events[0].previous_event_hash, GENESIS_HASH);
        }
    }

    #[test]
    fn test_processor_hash_integrity() {
        let buffer = Arc::new(TraceRingBuffer::new(100));
        let storage = Arc::new(InMemoryStorage::new());

        // Push several events
        for i in 0..5 {
            buffer.push(RawEvent::new(
                "session-1".to_string(),
                "trace-1".to_string(),
                if i == 0 {
                    EventType::SessionStarted
                } else if i == 4 {
                    EventType::SessionEnded
                } else {
                    EventType::ActionExecuted
                },
                json!({"index": i}),
            ));
        }

        let processor = TraceProcessor::with_defaults(buffer.clone(), storage.clone());
        let handle = processor.start();

        thread::sleep(Duration::from_millis(100));
        handle.join().unwrap();

        let events = storage.get_events("session-1").unwrap();
        assert_eq!(events.len(), 5);

        // Verify all hashes
        for event in &events {
            assert!(event.verify_hash(), "Hash verification failed for event {}", event.sequence);
        }

        // Verify chain
        for i in 1..events.len() {
            assert_eq!(
                events[i].previous_event_hash,
                events[i - 1].event_hash,
                "Chain broken at event {}", i
            );
        }
    }
}
