//! I/O Hooks for intercepting agent input/output

use std::sync::RwLock;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::WrapperResult;

/// Action decision from a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDecision {
    /// Whether the action is allowed
    pub allowed: bool,

    /// Reason for the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Context to inject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injected_context: Option<String>,
}

impl ActionDecision {
    /// Create an allowed decision
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
            injected_context: None,
        }
    }

    /// Create a denied decision
    pub fn deny(reason: &str) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.to_string()),
            injected_context: None,
        }
    }

    /// Create an allowed decision with context
    pub fn allow_with_context(context: &str) -> Self {
        Self {
            allowed: true,
            reason: None,
            injected_context: Some(context.to_string()),
        }
    }
}

/// I/O hooks interface
#[async_trait]
pub trait IOHooks: Send + Sync {
    /// Called before agent processes input
    async fn on_input(&self, input: &str) -> WrapperResult<String>;

    /// Called after agent produces output
    async fn on_output(&self, output: &str) -> WrapperResult<String>;

    /// Called before agent executes action
    async fn on_before_action(&self, action: &str, params: &serde_json::Value) -> WrapperResult<ActionDecision>;

    /// Called after action completes
    async fn on_after_action(&self, action: &str, result: &ActionResult);
}

/// Result of an action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,

    /// Action output (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// Registry for managing hooks
pub struct HookRegistry {
    /// Registered keywords for triggering context injection
    keywords: RwLock<Vec<String>>,

    /// Custom hook handlers
    handlers: RwLock<Vec<Box<dyn IOHooks>>>,
}

impl HookRegistry {
    /// Create a new hook registry
    pub fn new() -> Self {
        Self {
            keywords: RwLock::new(Vec::new()),
            handlers: RwLock::new(Vec::new()),
        }
    }

    /// Register keywords for context injection
    pub fn register_keywords(&self, keywords: Vec<String>) {
        if let Ok(mut kw) = self.keywords.write() {
            kw.extend(keywords);
        }
    }

    /// Check input for keyword matches
    pub fn check_keywords(&self, input: &str) -> Vec<String> {
        let input_lower = input.to_lowercase();

        if let Ok(keywords) = self.keywords.read() {
            keywords.iter()
                .filter(|kw| input_lower.contains(&kw.to_lowercase()))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Register a custom hook handler
    pub fn register_handler(&self, handler: Box<dyn IOHooks>) {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.push(handler);
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Default no-op hook implementation
pub struct NoOpHooks;

#[async_trait]
impl IOHooks for NoOpHooks {
    async fn on_input(&self, input: &str) -> WrapperResult<String> {
        Ok(input.to_string())
    }

    async fn on_output(&self, output: &str) -> WrapperResult<String> {
        Ok(output.to_string())
    }

    async fn on_before_action(&self, _action: &str, _params: &serde_json::Value) -> WrapperResult<ActionDecision> {
        Ok(ActionDecision::allow())
    }

    async fn on_after_action(&self, _action: &str, _result: &ActionResult) {
        // No-op
    }
}
