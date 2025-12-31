//! Error types for CRA Wrapper

use thiserror::Error;

/// Result type for wrapper operations
pub type WrapperResult<T> = Result<T, WrapperError>;

/// Errors that can occur in the wrapper
#[derive(Error, Debug)]
pub enum WrapperError {
    /// No active session
    #[error("No active session. Call start_session first.")]
    NoActiveSession,

    /// Session already exists
    #[error("Session already active: {0}")]
    SessionExists(String),

    /// Bootstrap failed
    #[error("Bootstrap failed: {0}")]
    BootstrapFailed(String),

    /// Action denied by policy
    #[error("Action denied: {0}")]
    ActionDenied(String),

    /// Context not found
    #[error("Context not found: {0}")]
    ContextNotFound(String),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Queue error
    #[error("Queue error: {0}")]
    Queue(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
