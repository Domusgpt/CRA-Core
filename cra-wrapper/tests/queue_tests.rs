//! TraceQueue tests

use cra_wrapper::queue::{TraceQueue, QueuedEvent};
use cra_wrapper::config::QueueConfig;
use chrono::Utc;

#[tokio::test]
async fn test_queue_creation() {
    let config = QueueConfig::default();
    let queue = TraceQueue::new(config);

    assert!(queue.is_empty().await);
    assert_eq!(queue.pending_count().await, 0);
}

#[tokio::test]
async fn test_enqueue_event() {
    let config = QueueConfig {
        max_size: 100,
        sync_events: vec![],
        flush_interval_ms: 5000,
    };
    let queue = TraceQueue::new(config);

    let event = QueuedEvent {
        event_type: "test.event".to_string(),
        session_id: "session-123".to_string(),
        timestamp: Utc::now(),
        payload: serde_json::json!({"key": "value"}),
    };

    queue.enqueue(event).await;

    assert!(!queue.is_empty().await);
    assert_eq!(queue.pending_count().await, 1);
}

#[tokio::test]
async fn test_enqueue_multiple_events() {
    let config = QueueConfig {
        max_size: 100,
        sync_events: vec![],
        flush_interval_ms: 5000,
    };
    let queue = TraceQueue::new(config);

    for i in 0..5 {
        let event = QueuedEvent {
            event_type: format!("test.event.{}", i),
            session_id: "session-123".to_string(),
            timestamp: Utc::now(),
            payload: serde_json::json!({"index": i}),
        };
        queue.enqueue(event).await;
    }

    assert_eq!(queue.pending_count().await, 5);
}

#[tokio::test]
async fn test_flush_clears_queue() {
    let config = QueueConfig {
        max_size: 100,
        sync_events: vec![],
        flush_interval_ms: 5000,
    };
    let queue = TraceQueue::new(config);

    // Enqueue some events
    for i in 0..3 {
        let event = QueuedEvent {
            event_type: "test.event".to_string(),
            session_id: "session-123".to_string(),
            timestamp: Utc::now(),
            payload: serde_json::json!({"index": i}),
        };
        queue.enqueue(event).await;
    }

    assert_eq!(queue.pending_count().await, 3);

    // Flush
    let result = queue.flush().await.unwrap();

    assert_eq!(result.flushed_count, 3);
    assert!(result.success);
    assert!(queue.is_empty().await);
}

#[tokio::test]
async fn test_flush_empty_queue() {
    let config = QueueConfig::default();
    let queue = TraceQueue::new(config);

    // Flush empty queue
    let result = queue.flush().await.unwrap();

    assert_eq!(result.flushed_count, 0);
    assert!(result.success);
}

#[tokio::test]
async fn test_auto_flush_at_max_size() {
    let config = QueueConfig {
        max_size: 3, // Auto-flush at 3 events
        sync_events: vec![],
        flush_interval_ms: 5000,
    };
    let queue = TraceQueue::new(config);

    // Enqueue events up to max
    for i in 0..3 {
        let event = QueuedEvent {
            event_type: "test.event".to_string(),
            session_id: "session-123".to_string(),
            timestamp: Utc::now(),
            payload: serde_json::json!({"index": i}),
        };
        queue.enqueue(event).await;
    }

    // Queue should have been auto-flushed
    assert!(queue.is_empty().await);
}

#[tokio::test]
async fn test_sync_event_triggers_flush() {
    let config = QueueConfig {
        max_size: 100,
        sync_events: vec!["session.end".to_string()],
        flush_interval_ms: 5000,
    };
    let queue = TraceQueue::new(config);

    // Enqueue normal event
    queue.enqueue(QueuedEvent {
        event_type: "normal.event".to_string(),
        session_id: "session-123".to_string(),
        timestamp: Utc::now(),
        payload: serde_json::json!({}),
    }).await;

    assert_eq!(queue.pending_count().await, 1);

    // Enqueue sync event (should trigger flush)
    queue.enqueue(QueuedEvent {
        event_type: "session.end".to_string(),
        session_id: "session-123".to_string(),
        timestamp: Utc::now(),
        payload: serde_json::json!({}),
    }).await;

    // Queue should be empty after sync event
    assert!(queue.is_empty().await);
}

#[tokio::test]
async fn test_queue_stats() {
    let config = QueueConfig {
        max_size: 100,
        sync_events: vec![],
        flush_interval_ms: 5000,
    };
    let queue = TraceQueue::new(config);

    // Initial stats
    let stats = queue.stats().await;
    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.total_enqueued, 0);
    assert_eq!(stats.total_flushed, 0);
    assert_eq!(stats.flush_count, 0);
    assert!(stats.last_flush_at.is_none());

    // Enqueue some events
    for _ in 0..5 {
        queue.enqueue(QueuedEvent {
            event_type: "test".to_string(),
            session_id: "session".to_string(),
            timestamp: Utc::now(),
            payload: serde_json::json!({}),
        }).await;
    }

    let stats = queue.stats().await;
    assert_eq!(stats.pending_count, 5);
    assert_eq!(stats.total_enqueued, 5);

    // Flush
    queue.flush().await.unwrap();

    let stats = queue.stats().await;
    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.total_enqueued, 5);
    assert_eq!(stats.total_flushed, 5);
    assert_eq!(stats.flush_count, 1);
    assert!(stats.last_flush_at.is_some());
}

#[tokio::test]
async fn test_queued_event_serialization() {
    let event = QueuedEvent {
        event_type: "test.event".to_string(),
        session_id: "session-123".to_string(),
        timestamp: Utc::now(),
        payload: serde_json::json!({
            "action": "write_file",
            "path": "/tmp/test.txt"
        }),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("test.event"));
    assert!(json.contains("session-123"));

    let parsed: QueuedEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.event_type, event.event_type);
    assert_eq!(parsed.session_id, event.session_id);
}

#[tokio::test]
async fn test_concurrent_enqueue() {
    use std::sync::Arc;

    let config = QueueConfig {
        max_size: 1000,
        sync_events: vec![],
        flush_interval_ms: 5000,
    };
    let queue = Arc::new(TraceQueue::new(config));

    let mut handles = vec![];

    // Spawn 10 tasks, each enqueueing 10 events
    for i in 0..10 {
        let queue_clone = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                queue_clone.enqueue(QueuedEvent {
                    event_type: format!("test.{}.{}", i, j),
                    session_id: "session".to_string(),
                    timestamp: Utc::now(),
                    payload: serde_json::json!({}),
                }).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Should have all 100 events
    assert_eq!(queue.pending_count().await, 100);
}
