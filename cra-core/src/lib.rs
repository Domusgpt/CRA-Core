//! # CRA Core - Context Registry Agents
//!
//! CRA is a governance layer for AI agents that provides:
//!
//! - **CARP** (Context & Action Resolution Protocol): Determines what context and actions
//!   are available for a given goal
//! - **TRACE** (Telemetry & Replay Audit Contract for Execution): Proves what actually
//!   happened with cryptographic integrity
//! - **Atlas**: Versioned packages containing everything needed to govern agent behavior
//!
//! ## Core Principle
//!
//! > If it wasn't emitted by the runtime, it didn't happen.
//!
//! CRA inverts the authority model: the runtime is authoritative, not the LLM.
//!
//! ## Example
//!
//! ```rust
//! use cra_core::{Resolver, CARPRequest, AtlasManifest};
//! use serde_json::json;
//!
//! // Create a resolver
//! let mut resolver = Resolver::new();
//!
//! // Load an atlas (would normally come from a JSON file)
//! let atlas_json = r#"{
//!     "atlas_version": "1.0",
//!     "atlas_id": "com.example.test",
//!     "version": "1.0.0",
//!     "name": "Test Atlas",
//!     "description": "A test atlas",
//!     "domains": ["test"],
//!     "capabilities": [],
//!     "policies": [],
//!     "actions": []
//! }"#;
//! let atlas: AtlasManifest = serde_json::from_str(atlas_json).unwrap();
//! resolver.load_atlas(atlas).unwrap();
//!
//! // Create a session
//! let session_id = resolver.create_session("test-agent", "Help with a task").unwrap();
//!
//! // Resolve a CARP request
//! let request = CARPRequest::new(
//!     session_id.clone(),
//!     "test-agent".to_string(),
//!     "Execute a test action".to_string(),
//! );
//! let resolution = resolver.resolve(&request).unwrap();
//!
//! // Check allowed actions
//! for action in resolution.allowed_actions {
//!     println!("Allowed: {}", action.action_id);
//! }
//!
//! // Get the trace for audit
//! let trace = resolver.get_trace(&session_id).unwrap();
//! assert!(resolver.verify_chain(&session_id).unwrap().is_valid);
//! ```

pub mod carp;
pub mod trace;
pub mod atlas;
pub mod error;
pub mod storage;
pub mod timing;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "async-runtime")]
pub mod runtime;

// Re-export main types
pub use carp::{
    CARPRequest, CARPResolution, Decision, AllowedAction, DeniedAction,
    Constraint, Resolver,
};
pub use trace::{
    TRACEEvent, EventType, TraceCollector, ChainVerification, ReplayResult,
};
pub use atlas::{
    AtlasManifest, AtlasAction, AtlasPolicy, AtlasCapability, PolicyType,
    AtlasLoader,
};
pub use error::{CRAError, Result, ErrorCategory, ErrorResponse, ErrorDetail};
pub use storage::{StorageBackend, InMemoryStorage, FileStorage, NullStorage};
pub use timing::{
    TimerEvent, TimerCallback, TimerBackend,
    HeartbeatConfig, SessionTTLConfig,
    SlidingWindowRateLimiter, RateLimitResult,
    TraceBatcher, HeartbeatMetrics,
};

/// Protocol version constants
pub const CARP_VERSION: &str = "1.0";
pub const TRACE_VERSION: &str = "1.0";
pub const ATLAS_VERSION: &str = "1.0";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_atlas() -> AtlasManifest {
        serde_json::from_value(json!({
            "atlas_version": "1.0",
            "atlas_id": "com.test.support",
            "version": "1.0.0",
            "name": "Test Support Atlas",
            "description": "Atlas for testing",
            "domains": ["support", "test"],
            "capabilities": [
                {
                    "capability_id": "ticket.read",
                    "name": "Read Tickets",
                    "actions": ["ticket.get", "ticket.list"]
                },
                {
                    "capability_id": "ticket.write",
                    "name": "Write Tickets",
                    "actions": ["ticket.create", "ticket.update"]
                }
            ],
            "policies": [
                {
                    "policy_id": "deny-delete",
                    "type": "deny",
                    "actions": ["*.delete"],
                    "reason": "Deletion requires approval"
                },
                {
                    "policy_id": "rate-limit-api",
                    "type": "rate_limit",
                    "actions": ["ticket.*"],
                    "parameters": {
                        "max_calls": 100,
                        "window_seconds": 60
                    }
                }
            ],
            "actions": [
                {
                    "action_id": "ticket.get",
                    "name": "Get Ticket",
                    "description": "Retrieve a ticket by ID",
                    "parameters_schema": {
                        "type": "object",
                        "required": ["ticket_id"],
                        "properties": {
                            "ticket_id": { "type": "string" }
                        }
                    },
                    "risk_tier": "low"
                },
                {
                    "action_id": "ticket.list",
                    "name": "List Tickets",
                    "description": "List all tickets",
                    "parameters_schema": {
                        "type": "object",
                        "properties": {}
                    },
                    "risk_tier": "low"
                },
                {
                    "action_id": "ticket.create",
                    "name": "Create Ticket",
                    "description": "Create a new ticket",
                    "parameters_schema": {
                        "type": "object",
                        "required": ["title"],
                        "properties": {
                            "title": { "type": "string" },
                            "description": { "type": "string" }
                        }
                    },
                    "risk_tier": "medium"
                },
                {
                    "action_id": "ticket.delete",
                    "name": "Delete Ticket",
                    "description": "Delete a ticket",
                    "parameters_schema": {
                        "type": "object",
                        "required": ["ticket_id"],
                        "properties": {
                            "ticket_id": { "type": "string" }
                        }
                    },
                    "risk_tier": "high"
                }
            ]
        })).unwrap()
    }

    #[test]
    fn test_full_workflow() {
        let mut resolver = Resolver::new();

        // Load atlas
        let atlas = create_test_atlas();
        resolver.load_atlas(atlas).unwrap();

        // Create session
        let session_id = resolver.create_session("agent-1", "Help with tickets").unwrap();

        // Resolve request
        let request = CARPRequest::new(
            session_id.clone(),
            "agent-1".to_string(),
            "I need to manage support tickets".to_string(),
        );
        let resolution = resolver.resolve(&request).unwrap();

        // Verify resolution - should be Partial since some actions are denied
        assert!(matches!(
            resolution.decision,
            Decision::Allow | Decision::AllowWithConstraints | Decision::Partial
        ));
        assert!(!resolution.allowed_actions.is_empty());

        // Verify deny policy worked
        let delete_allowed = resolution.allowed_actions.iter()
            .any(|a| a.action_id == "ticket.delete");
        assert!(!delete_allowed, "ticket.delete should be denied by policy");

        // Get trace and verify chain
        let trace = resolver.get_trace(&session_id).unwrap();
        assert!(!trace.is_empty());

        let verification = resolver.verify_chain(&session_id).unwrap();
        assert!(verification.is_valid);

        // End session
        resolver.end_session(&session_id).unwrap();
    }

    #[test]
    fn test_policy_evaluation_order() {
        let mut resolver = Resolver::new();
        let atlas = create_test_atlas();
        resolver.load_atlas(atlas).unwrap();

        let session_id = resolver.create_session("agent-1", "Test policy order").unwrap();

        let request = CARPRequest::new(
            session_id.clone(),
            "agent-1".to_string(),
            "Delete a ticket".to_string(),
        );
        let resolution = resolver.resolve(&request).unwrap();

        // Delete should be denied
        let denied = resolution.denied_actions.iter()
            .find(|d| d.action_id == "ticket.delete");
        assert!(denied.is_some(), "ticket.delete should be in denied list");
        assert!(denied.unwrap().reason.contains("approval"));
    }
}
