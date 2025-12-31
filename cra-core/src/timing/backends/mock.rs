//! Mock timer backend for testing
//!
//! Records all scheduled timers for inspection in tests.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use crate::error::Result;
use crate::timing::{TimerBackend, TimerEvent};

/// Recorded timer
#[derive(Debug, Clone)]
pub struct RecordedTimer {
    /// Timer ID
    pub id: String,
    /// Delay or interval
    pub duration: Duration,
    /// Event to fire
    pub event: TimerEvent,
    /// Whether this is repeating
    pub repeating: bool,
    /// Whether this timer was cancelled
    pub cancelled: bool,
}

/// Mock timer backend that records scheduled timers
///
/// Useful for testing that timers are scheduled correctly
/// without actually waiting for them to fire.
pub struct MockTimerBackend {
    /// Recorded timers by ID
    timers: RwLock<HashMap<String, RecordedTimer>>,
}

impl MockTimerBackend {
    /// Create a new mock backend
    pub fn new() -> Self {
        Self {
            timers: RwLock::new(HashMap::new()),
        }
    }

    /// Get a recorded timer
    pub fn get_timer(&self, id: &str) -> Option<RecordedTimer> {
        self.timers.read().unwrap().get(id).cloned()
    }

    /// Get all recorded timers
    pub fn all_timers(&self) -> Vec<RecordedTimer> {
        self.timers.read().unwrap().values().cloned().collect()
    }

    /// Get all active (non-cancelled) timers
    pub fn active_timers(&self) -> Vec<RecordedTimer> {
        self.timers
            .read()
            .unwrap()
            .values()
            .filter(|t| !t.cancelled)
            .cloned()
            .collect()
    }

    /// Clear all recorded timers
    pub fn clear(&self) {
        self.timers.write().unwrap().clear();
    }

    /// Count of timers
    pub fn count(&self) -> usize {
        self.timers.read().unwrap().len()
    }

    /// Count of active timers
    pub fn active_count(&self) -> usize {
        self.timers
            .read()
            .unwrap()
            .values()
            .filter(|t| !t.cancelled)
            .count()
    }
}

impl Default for MockTimerBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerBackend for MockTimerBackend {
    fn schedule_once(&self, id: &str, delay: Duration, event: TimerEvent) -> Result<()> {
        self.timers.write().unwrap().insert(
            id.to_string(),
            RecordedTimer {
                id: id.to_string(),
                duration: delay,
                event,
                repeating: false,
                cancelled: false,
            },
        );
        Ok(())
    }

    fn schedule_repeating(&self, id: &str, interval: Duration, event: TimerEvent) -> Result<()> {
        self.timers.write().unwrap().insert(
            id.to_string(),
            RecordedTimer {
                id: id.to_string(),
                duration: interval,
                event,
                repeating: true,
                cancelled: false,
            },
        );
        Ok(())
    }

    fn cancel(&self, id: &str) -> Result<bool> {
        let mut timers = self.timers.write().unwrap();
        if let Some(timer) = timers.get_mut(id) {
            timer.cancelled = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn exists(&self, id: &str) -> bool {
        self.timers
            .read()
            .unwrap()
            .get(id)
            .map(|t| !t.cancelled)
            .unwrap_or(false)
    }

    fn time_remaining(&self, id: &str) -> Option<Duration> {
        // In mock, we just return the original duration
        self.timers
            .read()
            .unwrap()
            .get(id)
            .filter(|t| !t.cancelled)
            .map(|t| t.duration)
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_backend_schedule() {
        let backend = MockTimerBackend::new();

        backend
            .schedule_once(
                "timer-1",
                Duration::from_secs(5),
                TimerEvent::Heartbeat {
                    session_id: "test".to_string(),
                },
            )
            .unwrap();

        assert!(backend.exists("timer-1"));
        assert_eq!(backend.count(), 1);

        let timer = backend.get_timer("timer-1").unwrap();
        assert_eq!(timer.duration, Duration::from_secs(5));
        assert!(!timer.repeating);
        assert!(!timer.cancelled);
    }

    #[test]
    fn test_mock_backend_cancel() {
        let backend = MockTimerBackend::new();

        backend
            .schedule_once(
                "timer-1",
                Duration::from_secs(5),
                TimerEvent::TraceBatchFlush,
            )
            .unwrap();

        assert!(backend.exists("timer-1"));

        backend.cancel("timer-1").unwrap();

        assert!(!backend.exists("timer-1"));
        assert_eq!(backend.active_count(), 0);

        // Timer still recorded but cancelled
        let timer = backend.get_timer("timer-1").unwrap();
        assert!(timer.cancelled);
    }

    #[test]
    fn test_mock_backend_repeating() {
        let backend = MockTimerBackend::new();

        backend
            .schedule_repeating(
                "heartbeat",
                Duration::from_secs(30),
                TimerEvent::Heartbeat {
                    session_id: "*".to_string(),
                },
            )
            .unwrap();

        let timer = backend.get_timer("heartbeat").unwrap();
        assert!(timer.repeating);
    }
}
