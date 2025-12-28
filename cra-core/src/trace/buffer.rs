//! Lock-free ring buffer for trace events
//!
//! Uses crossbeam's ArrayQueue for lock-free MPSC (multiple producer, single consumer)
//! operations. The hot path pushes RawEvents without blocking, and a background
//! worker drains and processes them.

use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam::queue::ArrayQueue;

use super::raw::RawEvent;

/// Default buffer capacity (4096 events)
pub const DEFAULT_CAPACITY: usize = 4096;

/// Lock-free ring buffer for trace events
///
/// This buffer is designed for high-throughput, low-latency event collection:
/// - Push is O(1) and lock-free
/// - Multiple producers can push concurrently
/// - Single consumer drains in batches
/// - Backpressure via capacity limits
pub struct TraceRingBuffer {
    /// The lock-free queue
    buffer: ArrayQueue<RawEvent>,

    /// Counter for dropped events (when buffer is full)
    dropped: AtomicU64,

    /// Counter for total events pushed
    total_pushed: AtomicU64,

    /// Counter for total events drained
    total_drained: AtomicU64,
}

impl TraceRingBuffer {
    /// Create a new ring buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: ArrayQueue::new(capacity),
            dropped: AtomicU64::new(0),
            total_pushed: AtomicU64::new(0),
            total_drained: AtomicU64::new(0),
        }
    }

    /// Create a new ring buffer with default capacity
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// Push an event to the buffer
    ///
    /// Returns `true` if the event was pushed successfully.
    /// Returns `false` if the buffer is full (event is dropped).
    ///
    /// This operation is lock-free and O(1).
    pub fn push(&self, event: RawEvent) -> bool {
        match self.buffer.push(event) {
            Ok(()) => {
                self.total_pushed.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(_) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    /// Try to pop a single event from the buffer
    ///
    /// Returns `None` if the buffer is empty.
    /// This operation is lock-free and O(1).
    pub fn pop(&self) -> Option<RawEvent> {
        self.buffer.pop().inspect(|_| {
            self.total_drained.fetch_add(1, Ordering::Relaxed);
        })
    }

    /// Drain up to `max` events from the buffer
    ///
    /// Returns a vector of events (may be empty if buffer is empty).
    /// This is the primary method for batch processing.
    pub fn drain(&self, max: usize) -> Vec<RawEvent> {
        let mut events = Vec::with_capacity(max.min(self.len()));
        for _ in 0..max {
            match self.buffer.pop() {
                Some(event) => {
                    self.total_drained.fetch_add(1, Ordering::Relaxed);
                    events.push(event);
                }
                None => break,
            }
        }
        events
    }

    /// Drain all events from the buffer
    ///
    /// Useful for shutdown or forced flush.
    pub fn drain_all(&self) -> Vec<RawEvent> {
        let mut events = Vec::with_capacity(self.len());
        while let Some(event) = self.buffer.pop() {
            self.total_drained.fetch_add(1, Ordering::Relaxed);
            events.push(event);
        }
        events
    }

    /// Get the current number of events in the buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check if the buffer is full
    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get the buffer pressure (0.0 = empty, 1.0 = full)
    ///
    /// Useful for backpressure handling.
    pub fn pressure(&self) -> f32 {
        self.len() as f32 / self.capacity() as f32
    }

    /// Get the number of dropped events
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Get total events pushed
    pub fn total_pushed(&self) -> u64 {
        self.total_pushed.load(Ordering::Relaxed)
    }

    /// Get total events drained
    pub fn total_drained(&self) -> u64 {
        self.total_drained.load(Ordering::Relaxed)
    }

    /// Get buffer statistics
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            capacity: self.capacity(),
            current_len: self.len(),
            pressure: self.pressure(),
            total_pushed: self.total_pushed(),
            total_drained: self.total_drained(),
            dropped: self.dropped_count(),
        }
    }
}

impl Default for TraceRingBuffer {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

impl std::fmt::Debug for TraceRingBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceRingBuffer")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("pressure", &self.pressure())
            .field("dropped", &self.dropped_count())
            .finish()
    }
}

/// Buffer statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    /// Maximum capacity
    pub capacity: usize,
    /// Current number of events
    pub current_len: usize,
    /// Pressure (0.0 - 1.0)
    pub pressure: f32,
    /// Total events pushed
    pub total_pushed: u64,
    /// Total events drained
    pub total_drained: u64,
    /// Events dropped due to full buffer
    pub dropped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::EventType;
    use serde_json::json;

    fn make_event(session_id: &str) -> RawEvent {
        RawEvent::new(
            session_id.to_string(),
            "trace-1".to_string(),
            EventType::SessionStarted,
            json!({}),
        )
    }

    #[test]
    fn test_push_pop() {
        let buffer = TraceRingBuffer::new(10);

        assert!(buffer.is_empty());
        assert!(!buffer.is_full());

        let event = make_event("session-1");
        assert!(buffer.push(event));

        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());

        let popped = buffer.pop();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().session_id, "session-1");

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_drain_batch() {
        let buffer = TraceRingBuffer::new(100);

        // Push 10 events
        for i in 0..10 {
            buffer.push(make_event(&format!("session-{}", i)));
        }

        assert_eq!(buffer.len(), 10);

        // Drain 5
        let batch = buffer.drain(5);
        assert_eq!(batch.len(), 5);
        assert_eq!(buffer.len(), 5);

        // Drain remaining
        let batch = buffer.drain(100);
        assert_eq!(batch.len(), 5);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_full_buffer_drops() {
        let buffer = TraceRingBuffer::new(3);

        // Fill buffer
        assert!(buffer.push(make_event("1")));
        assert!(buffer.push(make_event("2")));
        assert!(buffer.push(make_event("3")));

        assert!(buffer.is_full());
        assert_eq!(buffer.dropped_count(), 0);

        // This should fail and increment dropped counter
        assert!(!buffer.push(make_event("4")));
        assert_eq!(buffer.dropped_count(), 1);

        // Pop one and try again
        buffer.pop();
        assert!(buffer.push(make_event("5")));
        assert_eq!(buffer.dropped_count(), 1); // Still 1
    }

    #[test]
    fn test_pressure() {
        let buffer = TraceRingBuffer::new(100);

        assert_eq!(buffer.pressure(), 0.0);

        for i in 0..50 {
            buffer.push(make_event(&format!("session-{}", i)));
        }

        assert!((buffer.pressure() - 0.5).abs() < 0.01);

        for i in 50..100 {
            buffer.push(make_event(&format!("session-{}", i)));
        }

        assert_eq!(buffer.pressure(), 1.0);
    }

    #[test]
    fn test_stats() {
        let buffer = TraceRingBuffer::new(100);

        for i in 0..10 {
            buffer.push(make_event(&format!("session-{}", i)));
        }

        buffer.drain(3);

        let stats = buffer.stats();
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.current_len, 7);
        assert_eq!(stats.total_pushed, 10);
        assert_eq!(stats.total_drained, 3);
        assert_eq!(stats.dropped, 0);
    }

    #[test]
    fn test_concurrent_push() {
        use std::sync::Arc;
        use std::thread;

        let buffer = Arc::new(TraceRingBuffer::new(10000));
        let mut handles = vec![];

        // Spawn 10 threads, each pushing 100 events
        for t in 0..10 {
            let buf = buffer.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    buf.push(make_event(&format!("t{}-{}", t, i)));
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(buffer.total_pushed(), 1000);
        assert_eq!(buffer.len(), 1000);
        assert_eq!(buffer.dropped_count(), 0);
    }
}
