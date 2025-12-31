//! # CRA Timing & Heartbeat System
//!
//! Integration layer for timer-based telemetry and lifecycle management.
//! Designed to synergize with external timer systems like minoots-timer-system.
//!
//! ## Features
//!
//! - **Heartbeat telemetry**: Periodic TRACE events for agent health monitoring
//! - **Session TTL**: Automatic session cleanup after inactivity
//! - **Resolution expiry**: Time-based invalidation of stale resolutions
//! - **Rate limit windows**: Sliding window counters with proper time tracking
//! - **Trace batching**: Periodic flush of accumulated events
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         TimerManager                                │
//! │  • Coordinates all timing needs                                     │
//! │  • Tracks sessions for TTL                                          │
//! │  • Schedules heartbeats and flushes                                │
//! └────────────────────────────────┬────────────────────────────────────┘
//!                                  │
//!                                  ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                       TimerBackend (trait)                          │
//! ├─────────────────┬─────────────────┬─────────────────────────────────┤
//! │ MockBackend     │ StdBackend      │ MinootsBackend (optional)       │
//! │ (testing)       │ (std::thread)   │ (Horology Kernel)               │
//! └─────────────────┴─────────────────┴─────────────────────────────────┘
//! ```
//!
//! ## Integration with minoots-timer-system
//!
//! This module provides traits that can be implemented using:
//! - minoots Horology Kernel (Rust/Tokio) - enable `minoots` feature
//! - Native tokio timers (when async-runtime feature enabled)
//! - std::thread timers (for simple sync usage)
//!
//! ## Example
//!
//! ```rust,ignore
//! use cra_core::timing::{TimerManager, HeartbeatConfig, SessionTTLConfig};
//! use cra_core::timing::backends::MockTimerBackend;
//!
//! // Create timer manager with mock backend
//! let backend = MockTimerBackend::new();
//! let manager = TimerManager::new(backend)
//!     .with_heartbeat(HeartbeatConfig::new().interval(Duration::from_secs(30)))
//!     .with_session_ttl(SessionTTLConfig::new().idle_timeout(Duration::from_secs(3600)));
//!
//! // Start the timer manager
//! manager.start().unwrap();
//!
//! // Track sessions
//! manager.track_session("session-1").unwrap();
//!
//! // Record activity (resets idle timer)
//! manager.touch_session("session-1").unwrap();
//! ```

pub mod backends;
pub mod manager;

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::Result;
use crate::trace::TRACEEvent;

// Re-export backends
pub use backends::{MockTimerBackend, StdTimerBackend};

// Re-export manager
pub use manager::{TimerManager, TimerHandler, NullTimerHandler};

/// Timer event types that CRA cares about
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerEvent {
    /// Heartbeat timer fired
    Heartbeat { session_id: String },
    /// Session idle timeout
    SessionIdle { session_id: String },
    /// Session max lifetime reached
    SessionExpired { session_id: String },
    /// Resolution TTL expired
    ResolutionExpired { resolution_id: String },
    /// Rate limit window rolled over
    RateLimitReset { policy_id: String, action_id: String },
    /// Trace batch flush time
    TraceBatchFlush,
    /// Custom timer
    Custom { name: String, data: serde_json::Value },
}

/// Callback for timer events
pub trait TimerCallback: Send + Sync {
    /// Called when a timer fires
    fn on_timer(&self, event: TimerEvent) -> Result<()>;
}

/// Abstract timer backend trait
///
/// Implement this to integrate with different timer systems:
/// - minoots Horology Kernel
/// - tokio::time
/// - External webhook/cron systems
pub trait TimerBackend: Send + Sync {
    /// Schedule a one-shot timer
    fn schedule_once(&self, id: &str, delay: Duration, event: TimerEvent) -> Result<()>;

    /// Schedule a repeating timer
    fn schedule_repeating(&self, id: &str, interval: Duration, event: TimerEvent) -> Result<()>;

    /// Cancel a scheduled timer
    fn cancel(&self, id: &str) -> Result<bool>;

    /// Check if a timer exists
    fn exists(&self, id: &str) -> bool;

    /// Get time remaining on a timer
    fn time_remaining(&self, id: &str) -> Option<Duration>;

    /// Backend name (for logging)
    fn name(&self) -> &'static str;
}

/// Heartbeat configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between heartbeats
    pub interval: Duration,
    /// Include runtime metrics in heartbeat
    pub include_metrics: bool,
    /// Include session summary in heartbeat
    pub include_sessions: bool,
    /// Custom payload to include
    pub custom_payload: Option<serde_json::Value>,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            include_metrics: true,
            include_sessions: false,
            custom_payload: None,
        }
    }
}

impl HeartbeatConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn include_metrics(mut self, include: bool) -> Self {
        self.include_metrics = include;
        self
    }

    pub fn include_sessions(mut self, include: bool) -> Self {
        self.include_sessions = include;
        self
    }

    pub fn custom_payload(mut self, payload: serde_json::Value) -> Self {
        self.custom_payload = Some(payload);
        self
    }
}

/// Session TTL configuration
#[derive(Debug, Clone)]
pub struct SessionTTLConfig {
    /// Time after last activity before session is considered idle
    pub idle_timeout: Duration,
    /// Maximum session lifetime regardless of activity
    pub max_lifetime: Option<Duration>,
    /// Grace period before actually ending session
    pub grace_period: Duration,
    /// Emit warning event before expiry
    pub warn_before: Option<Duration>,
}

impl Default for SessionTTLConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(3600), // 1 hour
            max_lifetime: Some(Duration::from_secs(86400)), // 24 hours
            grace_period: Duration::from_secs(60), // 1 minute
            warn_before: Some(Duration::from_secs(300)), // 5 minutes
        }
    }
}

impl SessionTTLConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = Some(lifetime);
        self
    }

    pub fn no_max_lifetime(mut self) -> Self {
        self.max_lifetime = None;
        self
    }

    pub fn grace_period(mut self, period: Duration) -> Self {
        self.grace_period = period;
        self
    }
}

/// Sliding window rate limiter with proper time tracking
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    /// Window duration
    window: Duration,
    /// Maximum requests per window
    max_requests: u64,
    /// Request timestamps per (policy_id, action_id)
    requests: RwLock<HashMap<(String, String), Vec<Instant>>>,
}

impl SlidingWindowRateLimiter {
    pub fn new(window: Duration, max_requests: u64) -> Self {
        Self {
            window,
            max_requests,
            requests: RwLock::new(HashMap::new()),
        }
    }

    /// Check if request is allowed and record it
    pub fn check_and_record(&self, policy_id: &str, action_id: &str) -> RateLimitResult {
        let key = (policy_id.to_string(), action_id.to_string());
        let now = Instant::now();
        let window_start = now - self.window;

        let mut requests = self.requests.write().unwrap();
        let timestamps = requests.entry(key).or_default();

        // Remove expired timestamps
        timestamps.retain(|&t| t > window_start);

        let current_count = timestamps.len() as u64;

        if current_count >= self.max_requests {
            // Calculate when the oldest request will expire
            let oldest = timestamps.first().copied();
            let reset_after = oldest.map(|t| {
                let expires_at = t + self.window;
                if expires_at > now {
                    expires_at - now
                } else {
                    Duration::ZERO
                }
            });

            RateLimitResult::Exceeded {
                current: current_count,
                limit: self.max_requests,
                reset_after,
            }
        } else {
            timestamps.push(now);
            RateLimitResult::Allowed {
                remaining: self.max_requests - current_count - 1,
                reset_after: self.window,
            }
        }
    }

    /// Get current count without recording
    pub fn current_count(&self, policy_id: &str, action_id: &str) -> u64 {
        let key = (policy_id.to_string(), action_id.to_string());
        let now = Instant::now();
        let window_start = now - self.window;

        let requests = self.requests.read().unwrap();
        requests
            .get(&key)
            .map(|ts| ts.iter().filter(|&&t| t > window_start).count() as u64)
            .unwrap_or(0)
    }

    /// Reset rate limit for a specific action
    pub fn reset(&self, policy_id: &str, action_id: &str) {
        let key = (policy_id.to_string(), action_id.to_string());
        let mut requests = self.requests.write().unwrap();
        requests.remove(&key);
    }
}

/// Result of rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed {
        /// Remaining requests in window
        remaining: u64,
        /// Time until window resets
        reset_after: Duration,
    },
    /// Rate limit exceeded
    Exceeded {
        /// Current request count
        current: u64,
        /// Maximum allowed
        limit: u64,
        /// Time until oldest request expires
        reset_after: Option<Duration>,
    },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }
}

/// Trace event batcher for periodic flushing
pub struct TraceBatcher {
    /// Accumulated events
    events: RwLock<Vec<TRACEEvent>>,
    /// Maximum batch size before forced flush
    max_batch_size: usize,
    /// Flush callback
    on_flush: Option<Arc<dyn Fn(Vec<TRACEEvent>) -> Result<()> + Send + Sync>>,
}

impl std::fmt::Debug for TraceBatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceBatcher")
            .field("pending", &self.pending_count())
            .field("max_batch_size", &self.max_batch_size)
            .field("has_callback", &self.on_flush.is_some())
            .finish()
    }
}

impl TraceBatcher {
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            max_batch_size,
            on_flush: None,
        }
    }

    pub fn with_flush_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<TRACEEvent>) -> Result<()> + Send + Sync + 'static,
    {
        self.on_flush = Some(Arc::new(callback));
        self
    }

    /// Add event to batch
    pub fn add(&self, event: TRACEEvent) -> Result<bool> {
        let mut events = self.events.write().unwrap();
        events.push(event);

        // Check if we should force flush
        if events.len() >= self.max_batch_size {
            drop(events);
            self.flush()?;
            return Ok(true); // Did flush
        }

        Ok(false) // Did not flush
    }

    /// Flush accumulated events
    pub fn flush(&self) -> Result<Vec<TRACEEvent>> {
        let mut events = self.events.write().unwrap();
        let batch = std::mem::take(&mut *events);

        if let Some(ref callback) = self.on_flush {
            callback(batch.clone())?;
        }

        Ok(batch)
    }

    /// Get current batch size
    pub fn pending_count(&self) -> usize {
        self.events.read().unwrap().len()
    }
}

/// Metrics collected for heartbeat
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeartbeatMetrics {
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Total resolutions performed
    pub total_resolutions: u64,
    /// Resolutions in last interval
    pub resolutions_last_interval: u64,
    /// Active session count
    pub active_sessions: usize,
    /// Pending trace events
    pub pending_traces: usize,
    /// Memory usage (if available)
    pub memory_bytes: Option<u64>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // Re-export MockTimerBackend for other test modules
    pub use super::backends::MockTimerBackend;

    #[test]
    fn test_sliding_window_rate_limiter() {
        let limiter = SlidingWindowRateLimiter::new(Duration::from_secs(60), 3);

        // First 3 requests should be allowed
        assert!(limiter.check_and_record("policy-1", "action-1").is_allowed());
        assert!(limiter.check_and_record("policy-1", "action-1").is_allowed());
        assert!(limiter.check_and_record("policy-1", "action-1").is_allowed());

        // 4th should be denied
        let result = limiter.check_and_record("policy-1", "action-1");
        assert!(!result.is_allowed());

        // Different action should still be allowed
        assert!(limiter.check_and_record("policy-1", "action-2").is_allowed());
    }

    #[test]
    fn test_trace_batcher() {
        use crate::trace::EventType;
        use serde_json::json;

        let batcher = TraceBatcher::new(3);

        let event = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::SessionStarted,
            json!({}),
        );

        // Add events, shouldn't flush yet
        assert!(!batcher.add(event.clone()).unwrap());
        assert!(!batcher.add(event.clone()).unwrap());
        assert_eq!(batcher.pending_count(), 2);

        // Third event should trigger flush
        assert!(batcher.add(event.clone()).unwrap());
        assert_eq!(batcher.pending_count(), 0);
    }

    #[test]
    fn test_heartbeat_config_builder() {
        let config = HeartbeatConfig::new()
            .interval(Duration::from_secs(60))
            .include_metrics(true)
            .include_sessions(true);

        assert_eq!(config.interval, Duration::from_secs(60));
        assert!(config.include_metrics);
        assert!(config.include_sessions);
    }

    #[test]
    fn test_session_ttl_config_builder() {
        let config = SessionTTLConfig::new()
            .idle_timeout(Duration::from_secs(1800))
            .max_lifetime(Duration::from_secs(7200))
            .grace_period(Duration::from_secs(30));

        assert_eq!(config.idle_timeout, Duration::from_secs(1800));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(7200)));
        assert_eq!(config.grace_period, Duration::from_secs(30));
    }
}
