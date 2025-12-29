//! CARP/1.0 — Context & Action Resolution Protocol
//!
//! CARP defines a deterministic contract between an acting agent and a
//! context authority (CRA). It answers what context is allowed and what
//! actions may occur.
//!
//! ## Core Guarantees
//!
//! - Runtime authority (not the model)
//! - Least-privilege context
//! - Explicit permissions
//! - Evidence-backed decisions
//!
//! ## Resolution Flow
//!
//! 1. Agent submits goal to CARP
//! 2. Runtime loads relevant Atlas(es)
//! 3. Runtime evaluates policies (deny → approval → rate_limit → allow)
//! 4. Runtime assembles context blocks with priority ordering
//! 5. Runtime returns resolution with allowed actions
//! 6. Resolution has TTL — agent must re-resolve when expired

mod request;
mod resolution;
mod policy;
mod resolver;

pub use request::{CARPRequest, RiskTier};
pub use resolution::{CARPResolution, Decision, AllowedAction, DeniedAction, Constraint, ConstraintType, ContextBlock};
pub use policy::{PolicyEvaluator, PolicyResult};
pub use resolver::Resolver;

/// CARP protocol version
pub const VERSION: &str = "1.0";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_carp_request_serialization() {
        let request = CARPRequest::new(
            "session-123".to_string(),
            "agent-1".to_string(),
            "Help with support tickets".to_string(),
        );

        let json = serde_json::to_string(&request).unwrap();
        let parsed: CARPRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.session_id, "session-123");
        assert_eq!(parsed.agent_id, "agent-1");
        assert_eq!(parsed.goal, "Help with support tickets");
    }

    #[test]
    fn test_carp_resolution_serialization() {
        let resolution = CARPResolution {
            carp_version: VERSION.to_string(),
            trace_id: "trace-123".to_string(),
            session_id: "session-123".to_string(),
            decision: Decision::Allow,
            allowed_actions: vec![AllowedAction {
                action_id: "ticket.get".to_string(),
                name: "Get Ticket".to_string(),
                description: Some("Retrieve a ticket".to_string()),
                parameters_schema: json!({}),
                risk_tier: "low".to_string(),
            }],
            denied_actions: vec![],
            context_blocks: vec![],
            constraints: vec![],
            ttl_seconds: 300,
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&resolution).unwrap();
        let parsed: CARPResolution = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.trace_id, "trace-123");
        assert!(matches!(parsed.decision, Decision::Allow));
    }
}
