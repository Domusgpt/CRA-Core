//! Error types for CRA operations

use thiserror::Error;

/// Result type alias for CRA operations
pub type Result<T> = std::result::Result<T, CRAError>;

/// Errors that can occur in CRA operations
#[derive(Error, Debug)]
pub enum CRAError {
    // Atlas errors
    #[error("Atlas not found: {atlas_id}")]
    AtlasNotFound { atlas_id: String },

    #[error("Invalid atlas manifest: {reason}")]
    InvalidAtlasManifest { reason: String },

    #[error("Atlas version mismatch: expected {expected}, got {actual}")]
    AtlasVersionMismatch { expected: String, actual: String },

    #[error("Atlas already loaded: {atlas_id}")]
    AtlasAlreadyLoaded { atlas_id: String },

    #[error("Failed to load atlas from path: {path}: {reason}")]
    AtlasLoadError { path: String, reason: String },

    // Session errors
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Session already exists: {session_id}")]
    SessionAlreadyExists { session_id: String },

    #[error("Session expired: {session_id}")]
    SessionExpired { session_id: String },

    #[error("Session already ended: {session_id}")]
    SessionAlreadyEnded { session_id: String },

    // CARP errors
    #[error("Invalid CARP request: {reason}")]
    InvalidCARPRequest { reason: String },

    #[error("Resolution expired: TTL exceeded")]
    ResolutionExpired,

    #[error("Action not found: {action_id}")]
    ActionNotFound { action_id: String },

    #[error("Action denied by policy: {policy_id}: {reason}")]
    ActionDenied { policy_id: String, reason: String },

    #[error("Action requires approval: {action_id}")]
    ActionRequiresApproval { action_id: String },

    #[error("Rate limit exceeded for action: {action_id}")]
    RateLimitExceeded { action_id: String },

    // TRACE errors
    #[error("Trace chain integrity failure: {reason}")]
    TraceChainIntegrityError { reason: String },

    #[error("Invalid trace event: {reason}")]
    InvalidTraceEvent { reason: String },

    #[error("Replay failed: {reason}")]
    ReplayError { reason: String },

    // Policy errors
    #[error("Invalid policy: {policy_id}: {reason}")]
    InvalidPolicy { policy_id: String, reason: String },

    #[error("Policy evaluation error: {reason}")]
    PolicyEvaluationError { reason: String },

    // Schema validation errors
    #[error("Schema validation failed: {reason}")]
    SchemaValidationError { reason: String },

    #[error("Invalid parameters for action {action_id}: {reason}")]
    InvalidParameters { action_id: String, reason: String },

    // Execution errors
    #[error("Execution failed for action {action_id}: {reason}")]
    ExecutionError { action_id: String, reason: String },

    // Serialization errors
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    // IO errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    // Internal errors
    #[error("Internal error: {reason}")]
    InternalError { reason: String },
}

impl CRAError {
    /// Returns true if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CRAError::ResolutionExpired
                | CRAError::RateLimitExceeded { .. }
                | CRAError::ActionRequiresApproval { .. }
        )
    }

    /// Returns true if this error indicates a permanent denial
    pub fn is_permanent_denial(&self) -> bool {
        matches!(self, CRAError::ActionDenied { .. })
    }

    /// Returns the error code for serialization
    pub fn error_code(&self) -> &'static str {
        match self {
            CRAError::AtlasNotFound { .. } => "ATLAS_NOT_FOUND",
            CRAError::InvalidAtlasManifest { .. } => "INVALID_ATLAS_MANIFEST",
            CRAError::AtlasVersionMismatch { .. } => "ATLAS_VERSION_MISMATCH",
            CRAError::AtlasAlreadyLoaded { .. } => "ATLAS_ALREADY_LOADED",
            CRAError::AtlasLoadError { .. } => "ATLAS_LOAD_ERROR",
            CRAError::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            CRAError::SessionAlreadyExists { .. } => "SESSION_ALREADY_EXISTS",
            CRAError::SessionExpired { .. } => "SESSION_EXPIRED",
            CRAError::SessionAlreadyEnded { .. } => "SESSION_ALREADY_ENDED",
            CRAError::InvalidCARPRequest { .. } => "INVALID_CARP_REQUEST",
            CRAError::ResolutionExpired => "RESOLUTION_EXPIRED",
            CRAError::ActionNotFound { .. } => "ACTION_NOT_FOUND",
            CRAError::ActionDenied { .. } => "ACTION_DENIED",
            CRAError::ActionRequiresApproval { .. } => "ACTION_REQUIRES_APPROVAL",
            CRAError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            CRAError::TraceChainIntegrityError { .. } => "TRACE_CHAIN_INTEGRITY_ERROR",
            CRAError::InvalidTraceEvent { .. } => "INVALID_TRACE_EVENT",
            CRAError::ReplayError { .. } => "REPLAY_ERROR",
            CRAError::InvalidPolicy { .. } => "INVALID_POLICY",
            CRAError::PolicyEvaluationError { .. } => "POLICY_EVALUATION_ERROR",
            CRAError::SchemaValidationError { .. } => "SCHEMA_VALIDATION_ERROR",
            CRAError::InvalidParameters { .. } => "INVALID_PARAMETERS",
            CRAError::ExecutionError { .. } => "EXECUTION_ERROR",
            CRAError::JsonError(_) => "JSON_ERROR",
            CRAError::IoError(_) => "IO_ERROR",
            CRAError::InternalError { .. } => "INTERNAL_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_recoverable() {
        assert!(CRAError::ResolutionExpired.is_recoverable());
        assert!(CRAError::RateLimitExceeded {
            action_id: "test".to_string()
        }
        .is_recoverable());
        assert!(!CRAError::ActionDenied {
            policy_id: "deny-1".to_string(),
            reason: "denied".to_string()
        }
        .is_recoverable());
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            CRAError::SessionNotFound {
                session_id: "test".to_string()
            }
            .error_code(),
            "SESSION_NOT_FOUND"
        );
    }
}
