//! Timer backend implementations
//!
//! Provides different TimerBackend implementations for various runtime environments:
//!
//! - `MockTimerBackend`: For testing, records scheduled timers
//! - `StdTimerBackend`: Uses std::thread for simple sync cases
//! - `MinootsTimerBackend`: Integrates with minoots Horology Kernel (optional)
//!
//! ## Choosing a Backend
//!
//! - **Tests**: Use `MockTimerBackend`
//! - **Simple sync apps**: Use `StdTimerBackend`
//! - **Async apps with tokio**: Use async-runtime feature
//! - **Integration with minoots**: Use `MinootsTimerBackend`

mod mock;
mod std_backend;

pub use mock::MockTimerBackend;
pub use std_backend::StdTimerBackend;

#[cfg(feature = "minoots")]
mod minoots;

#[cfg(feature = "minoots")]
pub use minoots::MinootsTimerBackend;
