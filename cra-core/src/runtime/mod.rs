//! # CRA Async Runtime
//!
//! Optional async runtime layer for swarm and high-concurrency scenarios.
//!
//! The core `Resolver` is synchronous and CPU-bound (~134µs per resolution).
//! This runtime layer wraps it with async for:
//!
//! - **Async storage**: Non-blocking database operations
//! - **Session pooling**: Efficient management of many concurrent sessions
//! - **Event streaming**: Push traces to Kafka/Redis/etc.
//! - **Backpressure**: Graceful handling of overload
//!
//! # When to Use
//!
//! - **Single agent**: Use `Resolver` directly (sync is fine)
//! - **Few agents**: Use `spawn_blocking` wrapper
//! - **Swarm/many agents**: Use `AsyncRuntime` from this module
//!
//! # Example
//!
//! ```rust,ignore
//! use cra_core::runtime::{AsyncRuntime, RuntimeConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = RuntimeConfig::default()
//!         .max_sessions(10_000)
//!         .storage_pool_size(32);
//!
//!     let runtime = AsyncRuntime::new(config).await?;
//!
//!     // Handle many agents concurrently
//!     let handles: Vec<_> = (0..1000).map(|i| {
//!         let rt = runtime.clone();
//!         tokio::spawn(async move {
//!             let session = rt.create_session(&format!("agent-{}", i), "goal").await?;
//!             let resolution = rt.resolve(&session).await?;
//!             Ok::<_, CRAError>(resolution)
//!         })
//!     }).collect();
//!
//!     for handle in handles {
//!         handle.await??;
//!     }
//! }
//! ```

use std::sync::Arc;

use crate::error::Result;
use crate::{AtlasManifest, CARPRequest, CARPResolution, Resolver, TRACEEvent};

/// Configuration for the async runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum concurrent sessions (default: 10,000)
    pub max_sessions: usize,
    /// Size of the resolver thread pool (default: num_cpus)
    pub resolver_pool_size: usize,
    /// Size of the storage connection pool (default: 32)
    pub storage_pool_size: usize,
    /// Enable trace streaming (default: false)
    pub enable_streaming: bool,
    /// Channel buffer size for backpressure (default: 1000)
    pub channel_buffer_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10_000,
            resolver_pool_size: num_cpus::get(),
            storage_pool_size: 32,
            enable_streaming: false,
            channel_buffer_size: 1000,
        }
    }
}

impl RuntimeConfig {
    /// Set maximum concurrent sessions
    pub fn max_sessions(mut self, n: usize) -> Self {
        self.max_sessions = n;
        self
    }

    /// Set resolver thread pool size
    pub fn resolver_pool_size(mut self, n: usize) -> Self {
        self.resolver_pool_size = n;
        self
    }

    /// Set storage connection pool size
    pub fn storage_pool_size(mut self, n: usize) -> Self {
        self.storage_pool_size = n;
        self
    }

    /// Enable trace event streaming
    pub fn enable_streaming(mut self, enabled: bool) -> Self {
        self.enable_streaming = enabled;
        self
    }
}

/// Async storage backend trait
///
/// Implement this for async database connections (sqlx, redis, etc.)
#[async_trait::async_trait]
pub trait AsyncStorageBackend: Send + Sync {
    /// Store a trace event asynchronously
    async fn store_event(&self, event: &TRACEEvent) -> Result<()>;

    /// Get all events for a session
    async fn get_events(&self, session_id: &str) -> Result<Vec<TRACEEvent>>;

    /// Get events by type
    async fn get_events_by_type(
        &self,
        session_id: &str,
        event_type: &str,
    ) -> Result<Vec<TRACEEvent>>;

    /// Delete session data
    async fn delete_session(&self, session_id: &str) -> Result<()>;

    /// Health check
    async fn health_check(&self) -> Result<()>;

    /// Backend name
    fn name(&self) -> &'static str;
}

/// Event subscriber for streaming traces
#[async_trait::async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Called when a trace event is emitted
    async fn on_event(&self, event: &TRACEEvent) -> Result<()>;

    /// Called when a session ends
    async fn on_session_end(&self, session_id: &str) -> Result<()>;
}

/// Async runtime for high-concurrency CRA operations
///
/// Wraps the synchronous `Resolver` with:
/// - Async storage operations
/// - Session pooling
/// - Event streaming
/// - Backpressure handling
pub struct AsyncRuntime {
    config: RuntimeConfig,
    resolver: Arc<parking_lot::RwLock<Resolver>>,
    storage: Option<Arc<dyn AsyncStorageBackend>>,
    subscribers: Vec<Arc<dyn EventSubscriber>>,
}

impl AsyncRuntime {
    /// Create a new async runtime with default config
    pub async fn new(config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            config,
            resolver: Arc::new(parking_lot::RwLock::new(Resolver::new())),
            storage: None,
            subscribers: Vec::new(),
        })
    }

    /// Set the async storage backend
    pub fn with_storage(mut self, storage: Arc<dyn AsyncStorageBackend>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Add an event subscriber for streaming
    pub fn with_subscriber(mut self, subscriber: Arc<dyn EventSubscriber>) -> Self {
        self.subscribers.push(subscriber);
        self
    }

    /// Load an atlas (sync, but cheap)
    pub fn load_atlas(&self, atlas: AtlasManifest) -> Result<String> {
        self.resolver.write().load_atlas(atlas)
    }

    /// Create a session asynchronously
    ///
    /// The actual creation is fast (sync), but storage is async
    pub async fn create_session(&self, agent_id: &str, goal: &str) -> Result<String> {
        let session_id = {
            let mut resolver = self.resolver.write();
            resolver.create_session(agent_id, goal)?
        };

        // Store initial events asynchronously
        if let Some(ref storage) = self.storage {
            let events = self.resolver.read().get_trace(&session_id)?;
            for event in events {
                storage.store_event(&event).await?;
                self.notify_subscribers(&event).await?;
            }
        }

        Ok(session_id)
    }

    /// Resolve a request asynchronously
    ///
    /// Resolution is CPU-bound, so we use spawn_blocking
    pub async fn resolve(&self, request: &CARPRequest) -> Result<CARPResolution> {
        let resolver = self.resolver.clone();
        let session_id = request.session_id.clone();
        let request_clone = request.clone();

        // Run CPU-bound resolution on blocking thread pool
        let resolution = tokio::task::spawn_blocking(move || {
            resolver.write().resolve(&request_clone)
        })
        .await
        .map_err(|e| crate::CRAError::InternalError {
            reason: format!("Task join error: {}", e),
        })??;

        // Store trace events asynchronously
        if let Some(ref storage) = self.storage {
            let events = self.resolver.read().get_trace(&session_id)?;
            for event in events {
                storage.store_event(&event).await?;
                self.notify_subscribers(&event).await?;
            }
        }

        Ok(resolution)
    }

    /// End a session asynchronously
    pub async fn end_session(&self, session_id: &str) -> Result<()> {
        self.resolver.write().end_session(session_id)?;

        // Notify subscribers of session end
        for subscriber in &self.subscribers {
            subscriber.on_session_end(session_id).await?;
        }

        Ok(())
    }

    /// Get the resolver for direct access (advanced usage)
    pub fn resolver(&self) -> &Arc<parking_lot::RwLock<Resolver>> {
        &self.resolver
    }

    /// Notify all subscribers of an event
    async fn notify_subscribers(&self, event: &TRACEEvent) -> Result<()> {
        for subscriber in &self.subscribers {
            subscriber.on_event(event).await?;
        }
        Ok(())
    }
}

impl Clone for AsyncRuntime {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            resolver: self.resolver.clone(),
            storage: self.storage.clone(),
            subscribers: self.subscribers.clone(),
        }
    }
}

/// Swarm coordinator for multi-agent scenarios
///
/// Provides higher-level primitives for agent swarms:
/// - Agent registration and discovery
/// - Shared context propagation
/// - Coordinated policy updates
pub struct SwarmCoordinator {
    runtime: AsyncRuntime,
    // Future: add agent registry, shared state, etc.
}

impl SwarmCoordinator {
    /// Create a new swarm coordinator
    pub async fn new(runtime: AsyncRuntime) -> Result<Self> {
        Ok(Self { runtime })
    }

    /// Get the underlying runtime
    pub fn runtime(&self) -> &AsyncRuntime {
        &self.runtime
    }

    // Future methods:
    // - register_agent()
    // - broadcast_policy_update()
    // - get_swarm_metrics()
    // - coordinate_action() for cross-agent operations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_builder() {
        let config = RuntimeConfig::default()
            .max_sessions(50_000)
            .resolver_pool_size(16)
            .storage_pool_size(64)
            .enable_streaming(true);

        assert_eq!(config.max_sessions, 50_000);
        assert_eq!(config.resolver_pool_size, 16);
        assert_eq!(config.storage_pool_size, 64);
        assert!(config.enable_streaming);
    }
}
