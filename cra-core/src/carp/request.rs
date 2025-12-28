//! CARP Request types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VERSION;

/// A CARP request from an agent to resolve context and actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPRequest {
    /// CARP protocol version (always "1.0")
    pub carp_version: String,

    /// Unique identifier for this session
    pub session_id: String,

    /// Identifier for the agent making the request
    pub agent_id: String,

    /// The agent's stated goal or task description
    pub goal: String,

    /// Optional risk tier hint from the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_tier: Option<RiskTier>,

    /// Optional context hints from the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_hints: Option<Vec<String>>,

    /// Optional capabilities the agent is requesting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_capabilities: Option<Vec<String>>,

    /// Optional specific actions the agent wants to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_actions: Option<Vec<String>>,

    /// Optional metadata attached to the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,

    /// Timestamp when the request was created
    pub timestamp: DateTime<Utc>,
}

impl CARPRequest {
    /// Create a new CARP request with minimal required fields
    pub fn new(session_id: String, agent_id: String, goal: String) -> Self {
        Self {
            carp_version: VERSION.to_string(),
            session_id,
            agent_id,
            goal,
            risk_tier: None,
            context_hints: None,
            requested_capabilities: None,
            requested_actions: None,
            metadata: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a new request builder for more complex requests
    pub fn builder(session_id: String, agent_id: String, goal: String) -> CARPRequestBuilder {
        CARPRequestBuilder::new(session_id, agent_id, goal)
    }

    /// Validate the request
    pub fn validate(&self) -> Result<(), String> {
        if self.carp_version != VERSION {
            return Err(format!(
                "Unsupported CARP version: expected {}, got {}",
                VERSION, self.carp_version
            ));
        }
        if self.session_id.is_empty() {
            return Err("Session ID cannot be empty".to_string());
        }
        if self.agent_id.is_empty() {
            return Err("Agent ID cannot be empty".to_string());
        }
        if self.goal.is_empty() {
            return Err("Goal cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Builder for constructing CARP requests
#[derive(Debug, Clone)]
pub struct CARPRequestBuilder {
    request: CARPRequest,
}

impl CARPRequestBuilder {
    /// Create a new builder with required fields
    pub fn new(session_id: String, agent_id: String, goal: String) -> Self {
        Self {
            request: CARPRequest::new(session_id, agent_id, goal),
        }
    }

    /// Set the risk tier
    pub fn risk_tier(mut self, tier: RiskTier) -> Self {
        self.request.risk_tier = Some(tier);
        self
    }

    /// Add context hints
    pub fn context_hints(mut self, hints: Vec<String>) -> Self {
        self.request.context_hints = Some(hints);
        self
    }

    /// Add requested capabilities
    pub fn requested_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.request.requested_capabilities = Some(capabilities);
        self
    }

    /// Add requested actions
    pub fn requested_actions(mut self, actions: Vec<String>) -> Self {
        self.request.requested_actions = Some(actions);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.request.metadata = Some(metadata);
        self
    }

    /// Build the request
    pub fn build(self) -> CARPRequest {
        self.request
    }
}

/// Risk tier classification for requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskTier {
    /// Low risk operations (read-only, informational)
    Low,
    /// Medium risk operations (create, update)
    Medium,
    /// High risk operations (delete, financial, security-sensitive)
    High,
    /// Critical risk operations (system-wide impact)
    Critical,
}

impl RiskTier {
    /// Get the numeric level (higher = more risky)
    pub fn level(&self) -> u8 {
        match self {
            RiskTier::Low => 1,
            RiskTier::Medium => 2,
            RiskTier::High => 3,
            RiskTier::Critical => 4,
        }
    }

    /// Check if this tier requires approval
    pub fn requires_approval(&self) -> bool {
        matches!(self, RiskTier::High | RiskTier::Critical)
    }
}

impl Default for RiskTier {
    fn default() -> Self {
        RiskTier::Low
    }
}

impl std::fmt::Display for RiskTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskTier::Low => write!(f, "low"),
            RiskTier::Medium => write!(f, "medium"),
            RiskTier::High => write!(f, "high"),
            RiskTier::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for RiskTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(RiskTier::Low),
            "medium" => Ok(RiskTier::Medium),
            "high" => Ok(RiskTier::High),
            "critical" => Ok(RiskTier::Critical),
            _ => Err(format!("Unknown risk tier: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let request = CARPRequest::new(
            "session-1".to_string(),
            "agent-1".to_string(),
            "Test goal".to_string(),
        );

        assert_eq!(request.carp_version, VERSION);
        assert_eq!(request.session_id, "session-1");
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_request_builder() {
        let request = CARPRequest::builder(
            "session-1".to_string(),
            "agent-1".to_string(),
            "Test goal".to_string(),
        )
        .risk_tier(RiskTier::High)
        .context_hints(vec!["support".to_string()])
        .requested_capabilities(vec!["ticket.read".to_string()])
        .build();

        assert_eq!(request.risk_tier, Some(RiskTier::High));
        assert_eq!(
            request.context_hints,
            Some(vec!["support".to_string()])
        );
    }

    #[test]
    fn test_request_validation() {
        let mut request = CARPRequest::new(
            "session-1".to_string(),
            "agent-1".to_string(),
            "Test goal".to_string(),
        );

        assert!(request.validate().is_ok());

        request.session_id = String::new();
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_risk_tier() {
        assert_eq!(RiskTier::Low.level(), 1);
        assert_eq!(RiskTier::Critical.level(), 4);
        assert!(RiskTier::High.requires_approval());
        assert!(!RiskTier::Low.requires_approval());
    }
}
