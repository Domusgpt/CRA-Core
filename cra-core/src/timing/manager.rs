//! Timer Manager
//!
//! Coordinates all CRA timing needs through a unified interface.
//! Integrates with TimerBackend implementations (minoots, tokio, std).

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::error::Result;

use super::{HeartbeatConfig, SessionTTLConfig, TimerBackend, TimerEvent};

/// Handler for timer events
pub trait TimerHandler: Send + Sync {
    /// Called when a heartbeat timer fires
    fn on_heartbeat(&self, session_id: &str) -> Result<()>;

    /// Called when a session becomes idle
    fn on_session_idle(&self, session_id: &str) -> Result<()>;

    /// Called when a session expires
    fn on_session_expired(&self, session_id: &str) -> Result<()>;

    /// Called when it's time to flush traces
    fn on_trace_flush(&self) -> Result<()>;

    /// Called when a rate limit window resets
    fn on_rate_limit_reset(&self, policy_id: &str, action_id: &str) -> Result<()>;
}

/// Session tracking state
#[derive(Debug, Clone)]
struct SessionState {
    /// When the session was created
    created_at: Instant,
    /// Last activity time
    last_activity: Instant,
    /// Whether idle warning was sent
    idle_warned: bool,
}

impl SessionState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            created_at: now,
            last_activity: now,
            idle_warned: false,
        }
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
        self.idle_warned = false;
    }
}

/// Timer manager that coordinates all CRA timing needs
///
/// Provides a unified interface for:
/// - Heartbeat emission
/// - Session TTL management
/// - Rate limit tracking
/// - Trace batch flushing
pub struct TimerManager<B: TimerBackend> {
    /// Timer backend implementation
    backend: B,

    /// Heartbeat configuration
    heartbeat_config: HeartbeatConfig,

    /// Session TTL configuration
    session_ttl_config: SessionTTLConfig,

    /// Trace flush interval
    trace_flush_interval: Duration,

    /// Tracked sessions
    sessions: RwLock<HashMap<String, SessionState>>,

    /// Whether heartbeat is running
    heartbeat_running: RwLock<bool>,
}

impl<B: TimerBackend> TimerManager<B> {
    /// Create a new timer manager
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            heartbeat_config: HeartbeatConfig::default(),
            session_ttl_config: SessionTTLConfig::default(),
            trace_flush_interval: Duration::from_secs(5),
            sessions: RwLock::new(HashMap::new()),
            heartbeat_running: RwLock::new(false),
        }
    }

    /// Set heartbeat configuration
    pub fn with_heartbeat(mut self, config: HeartbeatConfig) -> Self {
        self.heartbeat_config = config;
        self
    }

    /// Set session TTL configuration
    pub fn with_session_ttl(mut self, config: SessionTTLConfig) -> Self {
        self.session_ttl_config = config;
        self
    }

    /// Set trace flush interval
    pub fn with_trace_flush_interval(mut self, interval: Duration) -> Self {
        self.trace_flush_interval = interval;
        self
    }

    /// Start the timer manager (schedules repeating timers)
    pub fn start(&self) -> Result<()> {
        // Start heartbeat timer
        self.backend.schedule_repeating(
            "cra:heartbeat",
            self.heartbeat_config.interval,
            TimerEvent::Heartbeat {
                session_id: "*".to_string(), // Broadcast to all
            },
        )?;

        // Start trace flush timer
        self.backend.schedule_repeating(
            "cra:trace-flush",
            self.trace_flush_interval,
            TimerEvent::TraceBatchFlush,
        )?;

        *self.heartbeat_running.write().unwrap() = true;

        Ok(())
    }

    /// Stop the timer manager
    pub fn stop(&self) -> Result<()> {
        self.backend.cancel("cra:heartbeat")?;
        self.backend.cancel("cra:trace-flush")?;

        // Cancel all session timers
        let sessions = self.sessions.read().unwrap();
        for session_id in sessions.keys() {
            let _ = self.backend.cancel(&format!("cra:session-idle:{}", session_id));
            let _ = self.backend.cancel(&format!("cra:session-expire:{}", session_id));
        }

        *self.heartbeat_running.write().unwrap() = false;

        Ok(())
    }

    /// Track a new session for TTL management
    pub fn track_session(&self, session_id: &str) -> Result<()> {
        // Add to tracked sessions
        self.sessions
            .write()
            .unwrap()
            .insert(session_id.to_string(), SessionState::new());

        // Schedule idle timeout
        self.backend.schedule_once(
            &format!("cra:session-idle:{}", session_id),
            self.session_ttl_config.idle_timeout,
            TimerEvent::SessionIdle {
                session_id: session_id.to_string(),
            },
        )?;

        // Schedule max lifetime if configured
        if let Some(max_lifetime) = self.session_ttl_config.max_lifetime {
            self.backend.schedule_once(
                &format!("cra:session-expire:{}", session_id),
                max_lifetime,
                TimerEvent::SessionExpired {
                    session_id: session_id.to_string(),
                },
            )?;
        }

        Ok(())
    }

    /// Record activity for a session (resets idle timer)
    pub fn touch_session(&self, session_id: &str) -> Result<()> {
        // Update last activity
        if let Some(state) = self.sessions.write().unwrap().get_mut(session_id) {
            state.touch();
        } else {
            return Ok(()); // Session not tracked, ignore
        }

        // Cancel and reschedule idle timeout
        let _ = self.backend.cancel(&format!("cra:session-idle:{}", session_id));
        self.backend.schedule_once(
            &format!("cra:session-idle:{}", session_id),
            self.session_ttl_config.idle_timeout,
            TimerEvent::SessionIdle {
                session_id: session_id.to_string(),
            },
        )?;

        Ok(())
    }

    /// Stop tracking a session (when it ends normally)
    pub fn untrack_session(&self, session_id: &str) -> Result<()> {
        self.sessions.write().unwrap().remove(session_id);

        // Cancel timers
        let _ = self.backend.cancel(&format!("cra:session-idle:{}", session_id));
        let _ = self.backend.cancel(&format!("cra:session-expire:{}", session_id));

        Ok(())
    }

    /// Get session age in milliseconds
    pub fn session_age(&self, session_id: &str) -> Option<u64> {
        self.sessions
            .read()
            .unwrap()
            .get(session_id)
            .map(|s| s.created_at.elapsed().as_millis() as u64)
    }

    /// Get time since last session activity
    pub fn session_idle_time(&self, session_id: &str) -> Option<u64> {
        self.sessions
            .read()
            .unwrap()
            .get(session_id)
            .map(|s| s.last_activity.elapsed().as_millis() as u64)
    }

    /// Check if heartbeat is running
    pub fn is_running(&self) -> bool {
        *self.heartbeat_running.read().unwrap()
    }

    /// Get the number of tracked sessions
    pub fn tracked_session_count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    /// Get backend name (for logging)
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Access the underlying backend
    pub fn backend(&self) -> &B {
        &self.backend
    }
}

/// No-op timer handler for testing
pub struct NullTimerHandler;

impl TimerHandler for NullTimerHandler {
    fn on_heartbeat(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    fn on_session_idle(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    fn on_session_expired(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    fn on_trace_flush(&self) -> Result<()> {
        Ok(())
    }

    fn on_rate_limit_reset(&self, _policy_id: &str, _action_id: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::tests::MockTimerBackend;

    #[test]
    fn test_timer_manager_creation() {
        let backend = MockTimerBackend::new();
        let manager = TimerManager::new(backend)
            .with_heartbeat(HeartbeatConfig::default())
            .with_session_ttl(SessionTTLConfig::default())
            .with_trace_flush_interval(Duration::from_secs(10));

        assert!(!manager.is_running());
        assert_eq!(manager.backend_name(), "mock");
    }

    #[test]
    fn test_session_tracking() {
        let backend = MockTimerBackend::new();
        let manager = TimerManager::new(backend);

        manager.track_session("session-1").unwrap();
        assert_eq!(manager.tracked_session_count(), 1);

        manager.track_session("session-2").unwrap();
        assert_eq!(manager.tracked_session_count(), 2);

        manager.untrack_session("session-1").unwrap();
        assert_eq!(manager.tracked_session_count(), 1);
    }

    #[test]
    fn test_session_touch() {
        let backend = MockTimerBackend::new();
        let manager = TimerManager::new(backend);

        manager.track_session("session-1").unwrap();

        // Wait a tiny bit
        std::thread::sleep(Duration::from_millis(10));

        let idle_before = manager.session_idle_time("session-1").unwrap();
        assert!(idle_before >= 10);

        // Touch the session
        manager.touch_session("session-1").unwrap();

        let idle_after = manager.session_idle_time("session-1").unwrap();
        assert!(idle_after < idle_before);
    }
}
