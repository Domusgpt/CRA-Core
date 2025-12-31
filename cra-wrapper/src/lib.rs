//! CRA Wrapper - Agent-side component for CRA governance
//!
//! The wrapper is the agent-built component that:
//! - Intercepts agent I/O through hooks
//! - Queues TRACE events locally for async upload
//! - Caches context to avoid redundant fetches
//! - Communicates with CRA server via transport backends
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     CRA WRAPPER                              │
//! │                                                              │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
//! │  │   I/O       │  │   TRACE     │  │   Context   │         │
//! │  │   Hooks     │  │   Queue     │  │   Cache     │         │
//! │  └─────────────┘  └─────────────┘  └─────────────┘         │
//! │         │                │                │                 │
//! │         └────────────────┼────────────────┘                 │
//! │                          │                                  │
//! │                    ┌─────▼─────┐                           │
//! │                    │   CRA     │                           │
//! │                    │  Client   │                           │
//! │                    └───────────┘                           │
//! │                          │                                  │
//! │  ┌───────────────────────┼───────────────────────┐         │
//! │  │            TRANSPORT LAYER                     │         │
//! │  │  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐          │         │
//! │  │  │ MCP │  │REST │  │ WS  │  │Direct│          │         │
//! │  │  └─────┘  └─────┘  └─────┘  └─────┘          │         │
//! │  └───────────────────────────────────────────────┘         │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cra_wrapper::{Wrapper, WrapperConfig};
//!
//! // Create wrapper
//! let wrapper = Wrapper::new(WrapperConfig::default());
//!
//! // Start session
//! wrapper.start_session("Help user build a website").await?;
//!
//! // Use hooks for I/O interception
//! let input = wrapper.on_input(user_input).await?;
//! // ... agent processes ...
//! let output = wrapper.on_output(agent_output).await?;
//!
//! // Report actions
//! let decision = wrapper.report_action("write_file", params).await?;
//!
//! // End session
//! wrapper.end_session(Some("Task complete")).await?;
//! ```

pub mod hooks;
pub mod queue;
pub mod cache;
pub mod client;
pub mod transport;
pub mod config;
pub mod error;

pub use config::{WrapperConfig, QueueConfig, CacheConfig};
pub use error::{WrapperError, WrapperResult};
pub use hooks::{IOHooks, ActionDecision};
pub use queue::{TraceQueue, QueuedEvent};
pub use cache::{ContextCache, CachedContext};
pub use client::CRAClient;

use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The main CRA Wrapper
pub struct Wrapper {
    /// Configuration
    config: WrapperConfig,

    /// Current session state
    session: Arc<RwLock<Option<WrapperSession>>>,

    /// I/O hooks
    hooks: Arc<hooks::HookRegistry>,

    /// TRACE event queue
    queue: Arc<queue::TraceQueue>,

    /// Context cache
    cache: Arc<cache::ContextCache>,

    /// CRA client
    client: Arc<dyn client::CRAClient + Send + Sync>,
}

impl Wrapper {
    /// Create a new wrapper with default configuration
    pub fn new(config: WrapperConfig) -> Self {
        let queue = Arc::new(queue::TraceQueue::new(config.queue.clone()));
        let cache = Arc::new(cache::ContextCache::new(config.cache.clone()));
        let client = Arc::new(client::DirectClient::new());

        Self {
            config,
            session: Arc::new(RwLock::new(None)),
            hooks: Arc::new(hooks::HookRegistry::new()),
            queue,
            cache,
            client,
        }
    }

    /// Create with a custom CRA client
    pub fn with_client<C: client::CRAClient + Send + Sync + 'static>(
        config: WrapperConfig,
        client: C,
    ) -> Self {
        let queue = Arc::new(queue::TraceQueue::new(config.queue.clone()));
        let cache = Arc::new(cache::ContextCache::new(config.cache.clone()));

        Self {
            config,
            session: Arc::new(RwLock::new(None)),
            hooks: Arc::new(hooks::HookRegistry::new()),
            queue,
            cache,
            client: Arc::new(client),
        }
    }

    /// Start a governed session
    pub async fn start_session(&self, goal: &str) -> WrapperResult<String> {
        // Bootstrap with CRA
        let bootstrap_result = self.client.bootstrap(goal).await?;

        // Create session
        let session = WrapperSession {
            session_id: bootstrap_result.session_id.clone(),
            goal: goal.to_string(),
            started_at: Utc::now(),
            genesis_hash: bootstrap_result.genesis_hash.clone(),
            current_hash: bootstrap_result.current_hash.clone(),
            event_count: 1,
            contexts_received: bootstrap_result.context_ids.clone(),
        };

        // Cache initial contexts
        for ctx in &bootstrap_result.contexts {
            self.cache.set(
                &ctx.context_id,
                CachedContext {
                    context_id: ctx.context_id.clone(),
                    content: ctx.content.clone(),
                    fetched_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::hours(1),
                    priority: ctx.priority,
                },
            ).await;
        }

        // Store session
        *self.session.write().await = Some(session);

        // Emit session started event
        self.queue.enqueue(QueuedEvent {
            event_type: "wrapper.session_started".to_string(),
            session_id: bootstrap_result.session_id.clone(),
            timestamp: Utc::now(),
            payload: serde_json::json!({
                "goal": goal,
                "genesis_hash": bootstrap_result.genesis_hash
            }),
        }).await;

        Ok(bootstrap_result.session_id)
    }

    /// End the current session
    pub async fn end_session(&self, summary: Option<&str>) -> WrapperResult<SessionSummary> {
        let session = self.session.read().await
            .as_ref()
            .ok_or(WrapperError::NoActiveSession)?
            .clone();

        // Flush trace queue
        self.queue.flush().await?;

        // End session with CRA
        let result = self.client.end_session(&session.session_id, summary).await?;

        // Clear session
        *self.session.write().await = None;

        Ok(SessionSummary {
            session_id: session.session_id,
            duration_ms: (Utc::now() - session.started_at).num_milliseconds(),
            event_count: session.event_count,
            chain_verified: result.chain_verified,
            final_hash: result.final_hash,
        })
    }

    /// Process input through hooks
    pub async fn on_input(&self, input: &str) -> WrapperResult<ProcessedInput> {
        let session = self.session.read().await
            .as_ref()
            .ok_or(WrapperError::NoActiveSession)?
            .clone();

        // Run through input hooks
        let processed = input.to_string();
        let mut injected_context = Vec::new();

        // Check for checkpoint triggers (keyword matching)
        if self.config.checkpoints_enabled {
            let keywords = self.hooks.check_keywords(&processed);
            if !keywords.is_empty() {
                // Request context for matched keywords
                let contexts = self.client.request_context(
                    &session.session_id,
                    &format!("Keywords matched: {}", keywords.join(", ")),
                    Some(keywords),
                ).await?;

                for ctx in contexts {
                    injected_context.push(ctx.content.clone());
                    self.cache.set(&ctx.context_id, CachedContext {
                        context_id: ctx.context_id.clone(),
                        content: ctx.content,
                        fetched_at: Utc::now(),
                        expires_at: Utc::now() + chrono::Duration::hours(1),
                        priority: ctx.priority,
                    }).await;
                }
            }
        }

        // Emit input event
        self.queue.enqueue(QueuedEvent {
            event_type: "wrapper.input_received".to_string(),
            session_id: session.session_id.clone(),
            timestamp: Utc::now(),
            payload: serde_json::json!({
                "input_length": input.len(),
                "context_injected": !injected_context.is_empty()
            }),
        }).await;

        Ok(ProcessedInput {
            original: input.to_string(),
            processed,
            injected_context,
        })
    }

    /// Process output through hooks
    pub async fn on_output(&self, output: &str) -> WrapperResult<ProcessedOutput> {
        let session = self.session.read().await
            .as_ref()
            .ok_or(WrapperError::NoActiveSession)?
            .clone();

        // Emit output event
        self.queue.enqueue(QueuedEvent {
            event_type: "wrapper.output_produced".to_string(),
            session_id: session.session_id.clone(),
            timestamp: Utc::now(),
            payload: serde_json::json!({
                "output_length": output.len()
            }),
        }).await;

        Ok(ProcessedOutput {
            original: output.to_string(),
            processed: output.to_string(),
        })
    }

    /// Report an action before execution
    pub async fn report_action(
        &self,
        action: &str,
        params: serde_json::Value,
    ) -> WrapperResult<ActionDecision> {
        let session = self.session.read().await
            .as_ref()
            .ok_or(WrapperError::NoActiveSession)?
            .clone();

        // Report to CRA and get decision
        let report = self.client.report_action(
            &session.session_id,
            action,
            params.clone(),
        ).await?;

        // Emit action event
        self.queue.enqueue(QueuedEvent {
            event_type: "wrapper.action_reported".to_string(),
            session_id: session.session_id.clone(),
            timestamp: Utc::now(),
            payload: serde_json::json!({
                "action": action,
                "decision": report.decision
            }),
        }).await;

        Ok(ActionDecision {
            allowed: report.decision == "approved",
            reason: report.reason,
            injected_context: None,
        })
    }

    /// Submit feedback on context
    pub async fn feedback(
        &self,
        context_id: &str,
        helpful: bool,
        reason: Option<&str>,
    ) -> WrapperResult<()> {
        let session = self.session.read().await
            .as_ref()
            .ok_or(WrapperError::NoActiveSession)?
            .clone();

        self.client.feedback(
            &session.session_id,
            context_id,
            helpful,
            reason,
        ).await?;

        // Emit feedback event
        self.queue.enqueue(QueuedEvent {
            event_type: "wrapper.feedback_submitted".to_string(),
            session_id: session.session_id.clone(),
            timestamp: Utc::now(),
            payload: serde_json::json!({
                "context_id": context_id,
                "helpful": helpful
            }),
        }).await;

        Ok(())
    }

    /// Request context on demand
    pub async fn request_context(
        &self,
        need: &str,
        hints: Option<Vec<String>>,
    ) -> WrapperResult<Vec<ContextBlock>> {
        let session = self.session.read().await
            .as_ref()
            .ok_or(WrapperError::NoActiveSession)?
            .clone();

        // Check cache first
        // ... (cache lookup logic)

        // Request from CRA
        let contexts = self.client.request_context(
            &session.session_id,
            need,
            hints,
        ).await?;

        // Cache results
        for ctx in &contexts {
            self.cache.set(&ctx.context_id, CachedContext {
                context_id: ctx.context_id.clone(),
                content: ctx.content.clone(),
                fetched_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                priority: ctx.priority,
            }).await;
        }

        Ok(contexts)
    }

    /// Get current session info
    pub async fn current_session(&self) -> Option<WrapperSession> {
        self.session.read().await.clone()
    }

    /// Get queue statistics
    pub async fn queue_stats(&self) -> queue::QueueStats {
        self.queue.stats().await
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> cache::CacheStats {
        self.cache.stats().await
    }
}

/// Wrapper session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapperSession {
    pub session_id: String,
    pub goal: String,
    pub started_at: DateTime<Utc>,
    pub genesis_hash: String,
    pub current_hash: String,
    pub event_count: u64,
    pub contexts_received: Vec<String>,
}

/// Session summary after ending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub duration_ms: i64,
    pub event_count: u64,
    pub chain_verified: bool,
    pub final_hash: String,
}

/// Processed input from on_input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    pub original: String,
    pub processed: String,
    pub injected_context: Vec<String>,
}

/// Processed output from on_output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedOutput {
    pub original: String,
    pub processed: String,
}

/// Context block from CRA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    pub context_id: String,
    pub content: String,
    pub priority: i32,
}
