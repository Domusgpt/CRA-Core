//! Standard library timer backend
//!
//! Uses std::thread for simple timer implementation.
//! Suitable for sync applications that don't need tokio.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::Result;
use crate::timing::{TimerBackend, TimerEvent};

/// Timer state
struct TimerState {
    /// Whether the timer is cancelled
    cancelled: Arc<AtomicBool>,
    /// Thread handle (for non-repeating)
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
}

/// Callback type for timer events
pub type TimerCallback = Arc<dyn Fn(TimerEvent) + Send + Sync>;

/// Standard library timer backend
///
/// Spawns a thread for each timer. Simple but not efficient
/// for many timers - consider tokio backend for that use case.
pub struct StdTimerBackend {
    /// Active timers
    timers: RwLock<HashMap<String, TimerState>>,
    /// Callback for timer events
    callback: Option<TimerCallback>,
}

impl StdTimerBackend {
    /// Create a new std timer backend
    pub fn new() -> Self {
        Self {
            timers: RwLock::new(HashMap::new()),
            callback: None,
        }
    }

    /// Create with a callback for timer events
    pub fn with_callback<F>(callback: F) -> Self
    where
        F: Fn(TimerEvent) + Send + Sync + 'static,
    {
        Self {
            timers: RwLock::new(HashMap::new()),
            callback: Some(Arc::new(callback)),
        }
    }

    /// Set the callback for timer events
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(TimerEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
    }

    /// Fire a timer event (calls the callback if set)
    fn fire_event(callback: &Option<TimerCallback>, event: TimerEvent) {
        if let Some(ref cb) = callback {
            cb(event);
        }
    }
}

impl Default for StdTimerBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerBackend for StdTimerBackend {
    fn schedule_once(&self, id: &str, delay: Duration, event: TimerEvent) -> Result<()> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        let callback = self.callback.clone();

        let handle = thread::spawn(move || {
            thread::sleep(delay);
            if !cancelled_clone.load(Ordering::Relaxed) {
                Self::fire_event(&callback, event);
            }
        });

        self.timers.write().unwrap().insert(
            id.to_string(),
            TimerState {
                cancelled,
                handle: Some(handle),
            },
        );

        Ok(())
    }

    fn schedule_repeating(&self, id: &str, interval: Duration, event: TimerEvent) -> Result<()> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        let callback = self.callback.clone();

        // Spawn a thread that loops until cancelled
        let _handle = thread::spawn(move || {
            loop {
                thread::sleep(interval);
                if cancelled_clone.load(Ordering::Relaxed) {
                    break;
                }
                Self::fire_event(&callback, event.clone());
            }
        });

        self.timers.write().unwrap().insert(
            id.to_string(),
            TimerState {
                cancelled,
                handle: None, // Don't store handle for repeating to avoid blocking on drop
            },
        );

        Ok(())
    }

    fn cancel(&self, id: &str) -> Result<bool> {
        let mut timers = self.timers.write().unwrap();
        if let Some(state) = timers.get(id) {
            state.cancelled.store(true, Ordering::Relaxed);
            timers.remove(id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn exists(&self, id: &str) -> bool {
        self.timers.read().unwrap().contains_key(id)
    }

    fn time_remaining(&self, _id: &str) -> Option<Duration> {
        // Not easily tracked with std threads
        None
    }

    fn name(&self) -> &'static str {
        "std"
    }
}

impl Drop for StdTimerBackend {
    fn drop(&mut self) {
        // Cancel all timers on drop
        let timers = self.timers.read().unwrap();
        for state in timers.values() {
            state.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn test_std_backend_schedule_once() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        let backend = StdTimerBackend::with_callback(move |_event| {
            fired_clone.store(true, Ordering::Relaxed);
        });

        backend
            .schedule_once(
                "test",
                Duration::from_millis(50),
                TimerEvent::TraceBatchFlush,
            )
            .unwrap();

        assert!(backend.exists("test"));

        // Wait for timer to fire
        thread::sleep(Duration::from_millis(100));

        assert!(fired.load(Ordering::Relaxed));
    }

    #[test]
    fn test_std_backend_cancel() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        let backend = StdTimerBackend::with_callback(move |_event| {
            fired_clone.store(true, Ordering::Relaxed);
        });

        backend
            .schedule_once(
                "test",
                Duration::from_millis(100),
                TimerEvent::TraceBatchFlush,
            )
            .unwrap();

        // Cancel before it fires
        backend.cancel("test").unwrap();

        // Wait past when it would have fired
        thread::sleep(Duration::from_millis(150));

        // Should not have fired
        assert!(!fired.load(Ordering::Relaxed));
    }

    #[test]
    fn test_std_backend_repeating() {
        let count = Arc::new(AtomicU32::new(0));
        let count_clone = count.clone();

        let backend = StdTimerBackend::with_callback(move |_event| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        });

        backend
            .schedule_repeating(
                "heartbeat",
                Duration::from_millis(30),
                TimerEvent::Heartbeat {
                    session_id: "test".to_string(),
                },
            )
            .unwrap();

        // Wait for a few ticks
        thread::sleep(Duration::from_millis(100));

        // Should have fired at least 2 times
        let final_count = count.load(Ordering::Relaxed);
        assert!(final_count >= 2, "Expected at least 2, got {}", final_count);

        // Cancel
        backend.cancel("heartbeat").unwrap();
    }
}
