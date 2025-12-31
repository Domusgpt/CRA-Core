//! Error types for CRA MCP Server

use thiserror::Error;

/// Result type for MCP operations
pub type McpResult<T> = Result<T, McpError>;

/// Errors that can occur in the MCP server
#[derive(Error, Debug)]
pub enum McpError {
    /// Session-related errors
    #[error("Session error: {0}")]
    Session(String),

    /// No active session
    #[error("No active session. Call cra_start_session first.")]
    NoActiveSession,

    /// Session already exists
    #[error("Session already active: {0}")]
    SessionExists(String),

    /// Invalid session ID
    #[error("Invalid session ID: {0}")]
    InvalidSession(String),

    /// Atlas-related errors
    #[error("Atlas error: {0}")]
    Atlas(String),

    /// Atlas not found
    #[error("Atlas not found: {0}")]
    AtlasNotFound(String),

    /// Context-related errors
    #[error("Context error: {0}")]
    Context(String),

    /// Action denied by policy
    #[error("Action denied: {0}")]
    ActionDenied(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// CRA Core error
    #[error("CRA Core error: {0}")]
    Core(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl McpError {
    /// Create an MCP-formatted error response
    pub fn to_mcp_error(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string()
            }
        })
    }

    /// Get error code for MCP protocol
    pub fn error_code(&self) -> i32 {
        match self {
            McpError::NoActiveSession => -32001,
            McpError::SessionExists(_) => -32002,
            McpError::InvalidSession(_) => -32003,
            McpError::AtlasNotFound(_) => -32004,
            McpError::ActionDenied(_) => -32005,
            McpError::Validation(_) => -32600,
            McpError::Core(_) => -32603,
            McpError::Io(_) => -32603,
            McpError::Serialization(_) => -32700,
            _ => -32603,
        }
    }
}

impl From<cra_core::CRAError> for McpError {
    fn from(err: cra_core::CRAError) -> Self {
        McpError::Core(err.to_string())
    }
}
