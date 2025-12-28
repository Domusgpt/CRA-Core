//! CARP Resolution types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VERSION;

/// A CARP resolution containing what the agent is allowed to do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPResolution {
    /// CARP protocol version
    pub carp_version: String,

    /// Unique trace ID for this resolution (links to TRACE events)
    pub trace_id: String,

    /// Session this resolution belongs to
    pub session_id: String,

    /// Overall decision for this resolution
    pub decision: Decision,

    /// Actions the agent is allowed to perform
    pub allowed_actions: Vec<AllowedAction>,

    /// Actions that were denied with reasons
    pub denied_actions: Vec<DeniedAction>,

    /// Context blocks to inject into the agent's context
    pub context_blocks: Vec<ContextBlock>,

    /// Active constraints on the agent's behavior
    pub constraints: Vec<Constraint>,

    /// Time-to-live in seconds (resolution expires after this)
    pub ttl_seconds: u64,

    /// When this resolution was created
    pub timestamp: DateTime<Utc>,
}

impl CARPResolution {
    /// Create a new resolution builder
    pub fn builder(session_id: String) -> CARPResolutionBuilder {
        CARPResolutionBuilder::new(session_id)
    }

    /// Check if the resolution has expired
    pub fn is_expired(&self) -> bool {
        let expiry = self.timestamp + chrono::Duration::seconds(self.ttl_seconds as i64);
        Utc::now() > expiry
    }

    /// Get the expiry time
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.timestamp + chrono::Duration::seconds(self.ttl_seconds as i64)
    }

    /// Check if a specific action is allowed
    pub fn is_action_allowed(&self, action_id: &str) -> bool {
        self.allowed_actions.iter().any(|a| a.action_id == action_id)
    }

    /// Get an allowed action by ID
    pub fn get_action(&self, action_id: &str) -> Option<&AllowedAction> {
        self.allowed_actions.iter().find(|a| a.action_id == action_id)
    }

    /// Get the denial reason for a specific action
    pub fn get_denial_reason(&self, action_id: &str) -> Option<&str> {
        self.denied_actions
            .iter()
            .find(|d| d.action_id == action_id)
            .map(|d| d.reason.as_str())
    }
}

/// Builder for CARP resolutions
#[derive(Debug, Clone)]
pub struct CARPResolutionBuilder {
    resolution: CARPResolution,
}

impl CARPResolutionBuilder {
    pub fn new(session_id: String) -> Self {
        Self {
            resolution: CARPResolution {
                carp_version: VERSION.to_string(),
                trace_id: uuid::Uuid::new_v4().to_string(),
                session_id,
                decision: Decision::Allow,
                allowed_actions: vec![],
                denied_actions: vec![],
                context_blocks: vec![],
                constraints: vec![],
                ttl_seconds: 300, // 5 minutes default
                timestamp: Utc::now(),
            },
        }
    }

    pub fn trace_id(mut self, trace_id: String) -> Self {
        self.resolution.trace_id = trace_id;
        self
    }

    pub fn decision(mut self, decision: Decision) -> Self {
        self.resolution.decision = decision;
        self
    }

    pub fn allowed_actions(mut self, actions: Vec<AllowedAction>) -> Self {
        self.resolution.allowed_actions = actions;
        self
    }

    pub fn add_allowed_action(mut self, action: AllowedAction) -> Self {
        self.resolution.allowed_actions.push(action);
        self
    }

    pub fn denied_actions(mut self, actions: Vec<DeniedAction>) -> Self {
        self.resolution.denied_actions = actions;
        self
    }

    pub fn add_denied_action(mut self, action: DeniedAction) -> Self {
        self.resolution.denied_actions.push(action);
        self
    }

    pub fn context_blocks(mut self, blocks: Vec<ContextBlock>) -> Self {
        self.resolution.context_blocks = blocks;
        self
    }

    pub fn add_context_block(mut self, block: ContextBlock) -> Self {
        self.resolution.context_blocks.push(block);
        self
    }

    pub fn constraints(mut self, constraints: Vec<Constraint>) -> Self {
        self.resolution.constraints = constraints;
        self
    }

    pub fn add_constraint(mut self, constraint: Constraint) -> Self {
        self.resolution.constraints.push(constraint);
        self
    }

    pub fn ttl_seconds(mut self, ttl: u64) -> Self {
        self.resolution.ttl_seconds = ttl;
        self
    }

    pub fn build(self) -> CARPResolution {
        self.resolution
    }
}

/// Decision outcome for a CARP resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// All requested actions are allowed
    Allow,
    /// No actions are allowed
    Deny,
    /// Some actions allowed with modifications
    Partial,
    /// Actions allowed but with constraints
    AllowWithConstraints,
    /// Actions require human approval before execution
    RequiresApproval,
}

impl Decision {
    /// Check if this decision permits any action
    pub fn permits_action(&self) -> bool {
        !matches!(self, Decision::Deny)
    }

    /// Check if this decision requires human intervention
    pub fn requires_human(&self) -> bool {
        matches!(self, Decision::RequiresApproval)
    }
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Allow => write!(f, "allow"),
            Decision::Deny => write!(f, "deny"),
            Decision::Partial => write!(f, "partial"),
            Decision::AllowWithConstraints => write!(f, "allow_with_constraints"),
            Decision::RequiresApproval => write!(f, "requires_approval"),
        }
    }
}

/// An action that is allowed in this resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedAction {
    /// Unique identifier for the action
    pub action_id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what the action does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for parameters
    pub parameters_schema: Value,

    /// Risk tier of this action
    pub risk_tier: String,
}

impl AllowedAction {
    /// Create a new allowed action
    pub fn new(action_id: String, name: String, parameters_schema: Value) -> Self {
        Self {
            action_id,
            name,
            description: None,
            parameters_schema,
            risk_tier: "low".to_string(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the risk tier
    pub fn with_risk_tier(mut self, risk_tier: String) -> Self {
        self.risk_tier = risk_tier;
        self
    }
}

/// An action that was denied with reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeniedAction {
    /// The action that was denied
    pub action_id: String,

    /// The policy that denied it
    pub policy_id: String,

    /// Human-readable reason for denial
    pub reason: String,

    /// Whether this denial can be appealed/overridden
    pub is_permanent: bool,
}

impl DeniedAction {
    /// Create a new denied action
    pub fn new(action_id: String, policy_id: String, reason: String) -> Self {
        Self {
            action_id,
            policy_id,
            reason,
            is_permanent: false,
        }
    }

    /// Mark as permanent denial
    pub fn permanent(mut self) -> Self {
        self.is_permanent = true;
        self
    }
}

/// A block of context to be injected into the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    /// Unique identifier for this block
    pub block_id: String,

    /// Human-readable name
    pub name: String,

    /// The actual content to inject
    pub content: String,

    /// Priority for ordering (higher = earlier)
    pub priority: i32,

    /// Content type (e.g., "text/markdown", "application/json")
    pub content_type: String,

    /// Source atlas that provided this context
    pub source_atlas: String,
}

impl ContextBlock {
    /// Create a new context block
    pub fn new(block_id: String, name: String, content: String) -> Self {
        Self {
            block_id,
            name,
            content,
            priority: 0,
            content_type: "text/plain".to_string(),
            source_atlas: String::new(),
        }
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = content_type;
        self
    }

    /// Set the source atlas
    pub fn with_source_atlas(mut self, atlas: String) -> Self {
        self.source_atlas = atlas;
        self
    }
}

/// A constraint on agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Unique identifier for this constraint
    pub constraint_id: String,

    /// Type of constraint
    pub constraint_type: ConstraintType,

    /// Human-readable description
    pub description: String,

    /// Constraint parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

impl Constraint {
    /// Create a new constraint
    pub fn new(constraint_id: String, constraint_type: ConstraintType, description: String) -> Self {
        Self {
            constraint_id,
            constraint_type,
            description,
            parameters: None,
        }
    }

    /// Add parameters
    pub fn with_parameters(mut self, params: Value) -> Self {
        self.parameters = Some(params);
        self
    }
}

/// Types of constraints that can be applied
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    /// Limit on number of calls
    RateLimit,
    /// Time-based restriction
    TimeWindow,
    /// Data field restrictions
    FieldMask,
    /// Geographic restrictions
    GeoRestriction,
    /// Budget/cost limits
    BudgetLimit,
    /// Custom constraint type
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolution_builder() {
        let resolution = CARPResolution::builder("session-1".to_string())
            .decision(Decision::AllowWithConstraints)
            .add_allowed_action(AllowedAction::new(
                "test.action".to_string(),
                "Test Action".to_string(),
                json!({}),
            ))
            .add_constraint(Constraint::new(
                "rate-1".to_string(),
                ConstraintType::RateLimit,
                "Max 100 calls per minute".to_string(),
            ))
            .ttl_seconds(600)
            .build();

        assert_eq!(resolution.session_id, "session-1");
        assert!(matches!(resolution.decision, Decision::AllowWithConstraints));
        assert_eq!(resolution.allowed_actions.len(), 1);
        assert_eq!(resolution.constraints.len(), 1);
    }

    #[test]
    fn test_resolution_expiry() {
        let resolution = CARPResolution::builder("session-1".to_string())
            .ttl_seconds(1)
            .build();

        assert!(!resolution.is_expired());

        // Simulate expired resolution
        let mut expired = resolution.clone();
        expired.timestamp = Utc::now() - chrono::Duration::seconds(10);
        assert!(expired.is_expired());
    }

    #[test]
    fn test_action_lookup() {
        let resolution = CARPResolution::builder("session-1".to_string())
            .add_allowed_action(AllowedAction::new(
                "ticket.get".to_string(),
                "Get Ticket".to_string(),
                json!({}),
            ))
            .add_denied_action(DeniedAction::new(
                "ticket.delete".to_string(),
                "deny-delete".to_string(),
                "Deletion not allowed".to_string(),
            ))
            .build();

        assert!(resolution.is_action_allowed("ticket.get"));
        assert!(!resolution.is_action_allowed("ticket.delete"));
        assert_eq!(
            resolution.get_denial_reason("ticket.delete"),
            Some("Deletion not allowed")
        );
    }
}
