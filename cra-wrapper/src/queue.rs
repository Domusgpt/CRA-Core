//! TRACE event queue for async upload

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::QueueConfig;
use crate::error::WrapperResult;

/// A queued TRACE event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    /// Event type
    pub event_type: String,

    /// Session ID
    pub session_id: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Event payload
    pub payload: serde_json::Value,
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Number of events currently in queue
    pub pending_count: usize,

    /// Total events enqueued
    pub total_enqueued: u64,

    /// Total events flushed
    pub total_flushed: u64,

    /// Number of flush operations
    pub flush_count: u64,

    /// Last flush time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_flush_at: Option<DateTime<Utc>>,
}

/// TRACE event queue
pub struct TraceQueue {
    /// Queue configuration
    config: QueueConfig,

    /// Pending events
    events: RwLock<Vec<QueuedEvent>>,

    /// Statistics
    total_enqueued: AtomicU64,
    total_flushed: AtomicU64,
    flush_count: AtomicU64,
    last_flush_at: RwLock<Option<DateTime<Utc>>>,
}

impl TraceQueue {
    /// Create a new trace queue
    pub fn new(config: QueueConfig) -> Self {
        Self {
            config,
            events: RwLock::new(Vec::new()),
            total_enqueued: AtomicU64::new(0),
            total_flushed: AtomicU64::new(0),
            flush_count: AtomicU64::new(0),
            last_flush_at: RwLock::new(None),
        }
    }

    /// Enqueue an event
    pub async fn enqueue(&self, event: QueuedEvent) {
        let should_flush = {
            let mut events = self.events.write().await;
            events.push(event.clone());
            self.total_enqueued.fetch_add(1, Ordering::SeqCst);

            // Check if we should auto-flush
            events.len() >= self.config.max_size ||
                self.config.sync_events.contains(&event.event_type)
        };

        if should_flush {
            let _ = self.flush().await;
        }
    }

    /// Flush all pending events
    pub async fn flush(&self) -> WrapperResult<FlushResult> {
        let events: Vec<QueuedEvent> = {
            let mut queue = self.events.write().await;
            std::mem::take(&mut *queue)
        };

        if events.is_empty() {
            return Ok(FlushResult {
                flushed_count: 0,
                success: true,
            });
        }

        let count = events.len() as u64;

        // TODO: Actually upload events to CRA
        // For now, just mark as flushed

        self.total_flushed.fetch_add(count, Ordering::SeqCst);
        self.flush_count.fetch_add(1, Ordering::SeqCst);
        *self.last_flush_at.write().await = Some(Utc::now());

        Ok(FlushResult {
            flushed_count: count as usize,
            success: true,
        })
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let pending_count = self.events.read().await.len();
        let last_flush_at = self.last_flush_at.read().await.clone();

        QueueStats {
            pending_count,
            total_enqueued: self.total_enqueued.load(Ordering::SeqCst),
            total_flushed: self.total_flushed.load(Ordering::SeqCst),
            flush_count: self.flush_count.load(Ordering::SeqCst),
            last_flush_at,
        }
    }

    /// Get pending event count
    pub async fn pending_count(&self) -> usize {
        self.events.read().await.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.events.read().await.is_empty()
    }
}

/// Result of a flush operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlushResult {
    /// Number of events flushed
    pub flushed_count: usize,

    /// Whether flush was successful
    pub success: bool,
}
