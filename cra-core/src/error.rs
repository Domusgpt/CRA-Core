//! Error types for CRA operations
//!
//! This module provides a comprehensive error handling system with:
//! - Structured error types with descriptive messages
//! - Error codes for programmatic handling
//! - HTTP status code mapping for server integrations
//! - Error categories for grouping and filtering
//! - JSON serialization for API responses
//!
//! # Error Codes
//!
//! Each error variant has a unique, stable error code (e.g., `SESSION_NOT_FOUND`)
//! that can be used for:
//! - Internationalization (i18n) - map codes to localized messages
//! - Client handling - switch on error codes for specific behaviors
//! - Logging and monitoring - aggregate errors by code
//!
//! # Example
//!
//! ```rust
//! use cra_core::error::{CRAError, ErrorCategory};
//!
//! fn handle_error(err: CRAError) {
//!     // Check error category
//!     match err.category() {
//!         ErrorCategory::NotFound => println!("Resource not found"),
//!         ErrorCategory::Validation => println!("Invalid input"),
//!         ErrorCategory::Authorization => println!("Access denied"),
//!         _ => println!("Other error"),
//!     }
//!
//!     // Get HTTP status for API response
//!     let status = err.http_status_code();
//!
//!     // Check if retry might help
//!     if err.is_recoverable() {
//!         println!("Retry may succeed");
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type alias for CRA operations
pub type Result<T> = std::result::Result<T, CRAError>;

/// Error category for grouping related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Resource not found (404)
    NotFound,
    /// Input validation failed (400)
    Validation,
    /// Authorization/policy denied (403)
    Authorization,
    /// Resource conflict (409)
    Conflict,
    /// Rate limiting (429)
    RateLimit,
    /// Data integrity error (422)
    Integrity,
    /// Internal server error (500)
    Internal,
    /// External service error (502)
    External,
}

/// Errors that can occur in CRA operations
///
/// All errors include:
/// - A human-readable error message
/// - A stable error code for programmatic handling
/// - A category for grouping
/// - An HTTP status code for server integrations
#[derive(Error, Debug)]
pub enum CRAError {
    // ═══════════════════════════════════════════════════════════════════════
    // Atlas errors (loading and managing atlas manifests)
    // ═══════════════════════════════════════════════════════════════════════

    /// Atlas with the specified ID was not found in the resolver
    #[error("Atlas not found: '{atlas_id}'. Ensure the atlas is loaded before creating sessions.")]
    AtlasNotFound { atlas_id: String },

    /// Atlas manifest is malformed or missing required fields
    #[error("Invalid atlas manifest: {reason}. Check the manifest against the Atlas/1.0 specification.")]
    InvalidAtlasManifest { reason: String },

    /// Atlas version in manifest doesn't match expected version
    #[error("Atlas version mismatch: expected '{expected}', got '{actual}'. Update your atlas or resolver.")]
    AtlasVersionMismatch { expected: String, actual: String },

    /// Attempted to load an atlas that's already loaded
    #[error("Atlas already loaded: '{atlas_id}'. Use unload_atlas() first if you need to reload.")]
    AtlasAlreadyLoaded { atlas_id: String },

    /// Failed to load atlas from file or URL
    #[error("Failed to load atlas from '{path}': {reason}")]
    AtlasLoadError { path: String, reason: String },

    // ═══════════════════════════════════════════════════════════════════════
    // Session errors (session lifecycle management)
    // ═══════════════════════════════════════════════════════════════════════

    /// Session with the specified ID doesn't exist
    #[error("Session not found: '{session_id}'. Create a session with create_session() first.")]
    SessionNotFound { session_id: String },

    /// Attempted to create a session with an ID that already exists
    #[error("Session already exists: '{session_id}'. Use a different session ID or end the existing session.")]
    SessionAlreadyExists { session_id: String },

    /// Session has expired due to inactivity or TTL
    #[error("Session expired: '{session_id}'. Create a new session to continue.")]
    SessionExpired { session_id: String },

    /// Attempted to use a session that has been ended
    #[error("Session already ended: '{session_id}'. Create a new session to continue.")]
    SessionAlreadyEnded { session_id: String },

    // ═══════════════════════════════════════════════════════════════════════
    // CARP errors (context and action resolution)
    // ═══════════════════════════════════════════════════════════════════════

    /// CARP request is malformed or missing required fields
    #[error("Invalid CARP request: {reason}")]
    InvalidCARPRequest { reason: String },

    /// Resolution has expired and can no longer be used
    #[error("Resolution expired: TTL exceeded. Request a new resolution.")]
    ResolutionExpired,

    /// Action ID doesn't exist in any loaded atlas
    #[error("Action not found: '{action_id}'. Verify the action exists in a loaded atlas.")]
    ActionNotFound { action_id: String },

    /// Action is explicitly denied by a policy
    #[error("Action denied by policy '{policy_id}': {reason}")]
    ActionDenied { policy_id: String, reason: String },

    /// Action requires human approval before execution
    #[error("Action '{action_id}' requires approval. Submit for review before executing.")]
    ActionRequiresApproval { action_id: String },

    /// Rate limit for this action has been exceeded
    #[error("Rate limit exceeded for action '{action_id}'. Wait before retrying.")]
    RateLimitExceeded { action_id: String },

    // ═══════════════════════════════════════════════════════════════════════
    // TRACE errors (audit trail and integrity)
    // ═══════════════════════════════════════════════════════════════════════

    /// Hash chain verification failed
    #[error("Trace chain integrity failure: {reason}. The audit trail may have been tampered with.")]
    TraceChainIntegrityError { reason: String },

    /// Trace event is malformed or has invalid data
    #[error("Invalid trace event: {reason}")]
    InvalidTraceEvent { reason: String },

    /// Replay of trace events failed
    #[error("Replay failed: {reason}")]
    ReplayError { reason: String },

    // ═══════════════════════════════════════════════════════════════════════
    // Policy errors (policy definition and evaluation)
    // ═══════════════════════════════════════════════════════════════════════

    /// Policy definition is invalid
    #[error("Invalid policy '{policy_id}': {reason}")]
    InvalidPolicy { policy_id: String, reason: String },

    /// Error occurred during policy evaluation
    #[error("Policy evaluation error: {reason}")]
    PolicyEvaluationError { reason: String },

    // ═══════════════════════════════════════════════════════════════════════
    // Schema and parameter validation errors
    // ═══════════════════════════════════════════════════════════════════════

    /// JSON Schema validation failed
    #[error("Schema validation failed: {reason}")]
    SchemaValidationError { reason: String },

    /// Action parameters don't match the required schema
    #[error("Invalid parameters for action '{action_id}': {reason}")]
    InvalidParameters { action_id: String, reason: String },

    // ═══════════════════════════════════════════════════════════════════════
    // Execution errors
    // ═══════════════════════════════════════════════════════════════════════

    /// Action execution failed
    #[error("Execution failed for action '{action_id}': {reason}")]
    ExecutionError { action_id: String, reason: String },

    // ═══════════════════════════════════════════════════════════════════════
    // Infrastructure errors (serialization, storage, I/O)
    // ═══════════════════════════════════════════════════════════════════════

    /// JSON serialization or deserialization failed
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Storage backend lock is poisoned (panic occurred while holding lock)
    #[error("Storage backend lock poisoned. This is a bug; please report it.")]
    StorageLocked,

    /// I/O operation failed
    #[error("IO error: {message}")]
    IoError { message: String },

    /// Internal error that shouldn't happen
    #[error("Internal error: {reason}. This is a bug; please report it.")]
    InternalError { reason: String },
}

impl CRAError {
    /// Returns true if this error might succeed on retry
    ///
    /// Recoverable errors include:
    /// - Rate limits (wait and retry)
    /// - Expired resolutions (request new resolution)
    /// - Storage locks (rare, indicates contention)
    ///
    /// Non-recoverable errors include:
    /// - Policy denials (permanent)
    /// - Not found errors (need different input)
    /// - Validation errors (need correct input)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CRAError::ResolutionExpired
                | CRAError::RateLimitExceeded { .. }
                | CRAError::ActionRequiresApproval { .. }
                | CRAError::StorageLocked
        )
    }

    /// Returns true if this error indicates a permanent policy denial
    ///
    /// Permanent denials mean the action will never be allowed for this
    /// agent/context combination. The agent should not retry.
    pub fn is_permanent_denial(&self) -> bool {
        matches!(self, CRAError::ActionDenied { .. })
    }

    /// Returns true if this error is a client error (4xx equivalent)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self.http_status_code(),
            400..=499
        )
    }

    /// Returns true if this error is a server error (5xx equivalent)
    pub fn is_server_error(&self) -> bool {
        matches!(
            self.http_status_code(),
            500..=599
        )
    }

    /// Returns the error category for grouping
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Not found
            CRAError::AtlasNotFound { .. }
            | CRAError::SessionNotFound { .. }
            | CRAError::ActionNotFound { .. } => ErrorCategory::NotFound,

            // Validation
            CRAError::InvalidAtlasManifest { .. }
            | CRAError::AtlasVersionMismatch { .. }
            | CRAError::InvalidCARPRequest { .. }
            | CRAError::InvalidTraceEvent { .. }
            | CRAError::InvalidPolicy { .. }
            | CRAError::SchemaValidationError { .. }
            | CRAError::InvalidParameters { .. } => ErrorCategory::Validation,

            // Authorization
            CRAError::ActionDenied { .. }
            | CRAError::ActionRequiresApproval { .. } => ErrorCategory::Authorization,

            // Conflict
            CRAError::AtlasAlreadyLoaded { .. }
            | CRAError::SessionAlreadyExists { .. }
            | CRAError::SessionAlreadyEnded { .. } => ErrorCategory::Conflict,

            // Rate limit
            CRAError::RateLimitExceeded { .. }
            | CRAError::ResolutionExpired
            | CRAError::SessionExpired { .. } => ErrorCategory::RateLimit,

            // Integrity
            CRAError::TraceChainIntegrityError { .. }
            | CRAError::ReplayError { .. } => ErrorCategory::Integrity,

            // Internal
            CRAError::StorageLocked
            | CRAError::InternalError { .. }
            | CRAError::PolicyEvaluationError { .. } => ErrorCategory::Internal,

            // External (I/O, JSON, file loading)
            CRAError::AtlasLoadError { .. }
            | CRAError::ExecutionError { .. }
            | CRAError::JsonError(_)
            | CRAError::IoError { .. } => ErrorCategory::External,
        }
    }

    /// Returns the stable error code for this error
    ///
    /// Error codes are uppercase, underscore-separated identifiers that
    /// remain stable across versions. Use these for:
    /// - Internationalization (mapping to translated messages)
    /// - Client-side error handling
    /// - Logging and alerting
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
            CRAError::StorageLocked => "STORAGE_LOCKED",
            CRAError::IoError { .. } => "IO_ERROR",
            CRAError::InternalError { .. } => "INTERNAL_ERROR",
        }
    }

    /// Returns the HTTP status code for this error
    ///
    /// Use this when building HTTP API responses. Maps errors to
    /// appropriate HTTP status codes following REST conventions.
    pub fn http_status_code(&self) -> u16 {
        match self {
            // 400 Bad Request - Client sent invalid data
            CRAError::InvalidAtlasManifest { .. }
            | CRAError::AtlasVersionMismatch { .. }
            | CRAError::InvalidCARPRequest { .. }
            | CRAError::InvalidTraceEvent { .. }
            | CRAError::InvalidPolicy { .. }
            | CRAError::SchemaValidationError { .. }
            | CRAError::InvalidParameters { .. } => 400,

            // 403 Forbidden - Action not allowed
            CRAError::ActionDenied { .. } => 403,

            // 404 Not Found - Resource doesn't exist
            CRAError::AtlasNotFound { .. }
            | CRAError::SessionNotFound { .. }
            | CRAError::ActionNotFound { .. } => 404,

            // 409 Conflict - Resource state conflict
            CRAError::AtlasAlreadyLoaded { .. }
            | CRAError::SessionAlreadyExists { .. }
            | CRAError::SessionAlreadyEnded { .. } => 409,

            // 410 Gone - Resource no longer available
            CRAError::SessionExpired { .. }
            | CRAError::ResolutionExpired => 410,

            // 422 Unprocessable Entity - Semantic error
            CRAError::TraceChainIntegrityError { .. }
            | CRAError::ReplayError { .. }
            | CRAError::PolicyEvaluationError { .. } => 422,

            // 423 Locked - Resource temporarily unavailable
            CRAError::ActionRequiresApproval { .. } => 423,

            // 429 Too Many Requests - Rate limited
            CRAError::RateLimitExceeded { .. } => 429,

            // 500 Internal Server Error - Our fault
            CRAError::StorageLocked
            | CRAError::InternalError { .. } => 500,

            // 502 Bad Gateway - External dependency failed
            CRAError::AtlasLoadError { .. }
            | CRAError::ExecutionError { .. }
            | CRAError::JsonError(_)
            | CRAError::IoError { .. } => 502,
        }
    }

    /// Converts this error to a JSON-serializable response object
    ///
    /// Returns a structure suitable for API error responses:
    /// ```json
    /// {
    ///   "error": {
    ///     "code": "SESSION_NOT_FOUND",
    ///     "message": "Session not found: 'abc123'...",
    ///     "category": "not_found",
    ///     "recoverable": false
    ///   }
    /// }
    /// ```
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: ErrorDetail {
                code: self.error_code().to_string(),
                message: self.to_string(),
                category: self.category(),
                recoverable: self.is_recoverable(),
            },
        }
    }
}

/// JSON-serializable error response for APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error details
    pub error: ErrorDetail,
}

/// Error detail for JSON responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Stable error code (e.g., "SESSION_NOT_FOUND")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Error category
    pub category: ErrorCategory,
    /// Whether retry might succeed
    pub recoverable: bool,
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
        assert!(CRAError::StorageLocked.is_recoverable());
        assert!(!CRAError::ActionDenied {
            policy_id: "deny-1".to_string(),
            reason: "denied".to_string()
        }
        .is_recoverable());
        assert!(!CRAError::SessionNotFound {
            session_id: "test".to_string()
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
        assert_eq!(
            CRAError::ActionDenied {
                policy_id: "p1".to_string(),
                reason: "denied".to_string()
            }
            .error_code(),
            "ACTION_DENIED"
        );
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(
            CRAError::SessionNotFound {
                session_id: "test".to_string()
            }
            .http_status_code(),
            404
        );
        assert_eq!(
            CRAError::ActionDenied {
                policy_id: "p1".to_string(),
                reason: "denied".to_string()
            }
            .http_status_code(),
            403
        );
        assert_eq!(
            CRAError::RateLimitExceeded {
                action_id: "test".to_string()
            }
            .http_status_code(),
            429
        );
        assert_eq!(CRAError::StorageLocked.http_status_code(), 500);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(
            CRAError::SessionNotFound {
                session_id: "test".to_string()
            }
            .category(),
            ErrorCategory::NotFound
        );
        assert_eq!(
            CRAError::ActionDenied {
                policy_id: "p1".to_string(),
                reason: "denied".to_string()
            }
            .category(),
            ErrorCategory::Authorization
        );
        assert_eq!(
            CRAError::InvalidAtlasManifest {
                reason: "test".to_string()
            }
            .category(),
            ErrorCategory::Validation
        );
    }

    #[test]
    fn test_is_client_server_error() {
        let client_err = CRAError::SessionNotFound {
            session_id: "test".to_string(),
        };
        assert!(client_err.is_client_error());
        assert!(!client_err.is_server_error());

        let server_err = CRAError::StorageLocked;
        assert!(!server_err.is_client_error());
        assert!(server_err.is_server_error());
    }

    #[test]
    fn test_error_response_serialization() {
        let err = CRAError::SessionNotFound {
            session_id: "abc123".to_string(),
        };
        let response = err.to_error_response();

        let json = serde_json::to_string_pretty(&response).unwrap();
        assert!(json.contains("SESSION_NOT_FOUND"));
        assert!(json.contains("abc123"));
        assert!(json.contains("not_found"));

        // Verify it can be deserialized
        let parsed: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.error.code, "SESSION_NOT_FOUND");
        assert!(!parsed.error.recoverable);
    }

    #[test]
    fn test_error_messages_are_helpful() {
        let err = CRAError::SessionNotFound {
            session_id: "test-123".to_string(),
        };
        let msg = err.to_string();
        // Message should include the ID and helpful context
        assert!(msg.contains("test-123"));
        assert!(msg.contains("create_session"));
    }
}
