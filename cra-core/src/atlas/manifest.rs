//! Atlas Manifest types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VERSION;

/// The main Atlas manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasManifest {
    /// Atlas format version (always "1.0")
    pub atlas_version: String,

    /// Unique identifier in reverse-domain notation (e.g., "com.acme.support")
    pub atlas_id: String,

    /// Semantic version (e.g., "1.2.3")
    pub version: String,

    /// Human-readable name
    pub name: String,

    /// Description of the atlas
    pub description: String,

    /// Authors of this atlas
    #[serde(default)]
    pub authors: Vec<String>,

    /// SPDX license identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Domain tags for discovery
    #[serde(default)]
    pub domains: Vec<String>,

    /// Capability groupings
    #[serde(default)]
    pub capabilities: Vec<AtlasCapability>,

    /// Context packs (file-based)
    #[serde(default)]
    pub context_packs: Vec<AtlasContextPack>,

    /// Inline context blocks (content in manifest)
    #[serde(default)]
    pub context_blocks: Vec<AtlasContextBlock>,

    /// Policy definitions
    #[serde(default)]
    pub policies: Vec<AtlasPolicy>,

    /// Action definitions
    #[serde(default)]
    pub actions: Vec<AtlasAction>,

    /// Dependencies on other atlases
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,

    /// External sources (repositories, documentation, demos)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<AtlasSources>,
}

impl AtlasManifest {
    /// Create a new atlas manifest builder
    pub fn builder(atlas_id: String, name: String) -> AtlasManifestBuilder {
        AtlasManifestBuilder::new(atlas_id, name)
    }

    /// Get an action by ID
    pub fn get_action(&self, action_id: &str) -> Option<&AtlasAction> {
        self.actions.iter().find(|a| a.action_id == action_id)
    }

    /// Get a policy by ID
    pub fn get_policy(&self, policy_id: &str) -> Option<&AtlasPolicy> {
        self.policies.iter().find(|p| p.policy_id == policy_id)
    }

    /// Get a capability by ID
    pub fn get_capability(&self, capability_id: &str) -> Option<&AtlasCapability> {
        self.capabilities
            .iter()
            .find(|c| c.capability_id == capability_id)
    }

    /// Get all actions for a capability
    pub fn get_capability_actions(&self, capability_id: &str) -> Vec<&AtlasAction> {
        self.get_capability(capability_id)
            .map(|cap| {
                cap.actions
                    .iter()
                    .filter_map(|action_id| self.get_action(action_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Validate the manifest structure
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = vec![];

        if self.atlas_version != VERSION {
            errors.push(format!(
                "Unsupported atlas version: expected {}, got {}",
                VERSION, self.atlas_version
            ));
        }

        if self.atlas_id.is_empty() {
            errors.push("atlas_id cannot be empty".to_string());
        }

        if self.version.is_empty() {
            errors.push("version cannot be empty".to_string());
        }

        if self.name.is_empty() {
            errors.push("name cannot be empty".to_string());
        }

        // Validate actions have unique IDs
        let mut action_ids: Vec<&str> = self.actions.iter().map(|a| a.action_id.as_str()).collect();
        action_ids.sort();
        for window in action_ids.windows(2) {
            if window[0] == window[1] {
                errors.push(format!("Duplicate action_id: {}", window[0]));
            }
        }

        // Validate policies have unique IDs
        let mut policy_ids: Vec<&str> = self.policies.iter().map(|p| p.policy_id.as_str()).collect();
        policy_ids.sort();
        for window in policy_ids.windows(2) {
            if window[0] == window[1] {
                errors.push(format!("Duplicate policy_id: {}", window[0]));
            }
        }

        // Validate capability actions exist
        for capability in &self.capabilities {
            for action_id in &capability.actions {
                if self.get_action(action_id).is_none() {
                    errors.push(format!(
                        "Capability {} references unknown action: {}",
                        capability.capability_id, action_id
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Builder for AtlasManifest
#[derive(Debug)]
pub struct AtlasManifestBuilder {
    manifest: AtlasManifest,
}

impl AtlasManifestBuilder {
    pub fn new(atlas_id: String, name: String) -> Self {
        Self {
            manifest: AtlasManifest {
                atlas_version: VERSION.to_string(),
                atlas_id,
                version: "1.0.0".to_string(),
                name,
                description: String::new(),
                authors: vec![],
                license: None,
                domains: vec![],
                capabilities: vec![],
                context_packs: vec![],
                context_blocks: vec![],
                policies: vec![],
                actions: vec![],
                dependencies: None,
                sources: None,
            },
        }
    }

    pub fn sources(mut self, sources: AtlasSources) -> Self {
        self.manifest.sources = Some(sources);
        self
    }

    pub fn version(mut self, version: &str) -> Self {
        self.manifest.version = version.to_string();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.manifest.description = description.to_string();
        self
    }

    pub fn authors(mut self, authors: Vec<String>) -> Self {
        self.manifest.authors = authors;
        self
    }

    pub fn license(mut self, license: &str) -> Self {
        self.manifest.license = Some(license.to_string());
        self
    }

    pub fn domains(mut self, domains: Vec<String>) -> Self {
        self.manifest.domains = domains;
        self
    }

    pub fn add_capability(mut self, capability: AtlasCapability) -> Self {
        self.manifest.capabilities.push(capability);
        self
    }

    pub fn add_policy(mut self, policy: AtlasPolicy) -> Self {
        self.manifest.policies.push(policy);
        self
    }

    pub fn add_action(mut self, action: AtlasAction) -> Self {
        self.manifest.actions.push(action);
        self
    }

    pub fn build(self) -> AtlasManifest {
        self.manifest
    }
}

/// A capability grouping of related actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasCapability {
    /// Unique identifier for this capability
    pub capability_id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this capability provides
    #[serde(default)]
    pub description: String,

    /// Action IDs included in this capability
    pub actions: Vec<String>,
}

impl AtlasCapability {
    /// Create a new capability
    pub fn new(capability_id: String, name: String, actions: Vec<String>) -> Self {
        Self {
            capability_id,
            name,
            description: String::new(),
            actions,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// A context pack containing related content (file-based)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasContextPack {
    /// Unique identifier for this pack
    pub pack_id: String,

    /// Human-readable name
    pub name: String,

    /// Files included in this pack (relative paths)
    pub files: Vec<String>,

    /// Priority for ordering (higher = earlier injection)
    #[serde(default)]
    pub priority: i32,

    /// Injection mode: always, on_match, on_demand, or risk_based
    #[serde(default)]
    pub inject_mode: InjectMode,

    /// Conditions for when to include this pack
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Value>,
}

/// An inline context block with content directly in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasContextBlock {
    /// Unique identifier for this context
    pub context_id: String,

    /// Human-readable name
    pub name: String,

    /// Priority for ordering (higher = earlier injection)
    #[serde(default)]
    pub priority: i32,

    /// The actual content to inject
    pub content: String,

    /// Content type (defaults to text/markdown)
    #[serde(default = "default_content_type")]
    pub content_type: String,

    /// Injection mode: always, on_match, on_demand, or risk_based
    #[serde(default)]
    pub inject_mode: InjectMode,

    /// Other context_ids to inject when this block matches
    #[serde(default)]
    pub also_inject: Vec<String>,

    /// Action patterns that trigger injection
    #[serde(default)]
    pub inject_when: Vec<String>,

    /// Keyword conditions for matching
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Risk tiers this applies to
    #[serde(default)]
    pub risk_tiers: Vec<String>,
}

fn default_content_type() -> String {
    "text/markdown".to_string()
}

/// A policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasPolicy {
    /// Unique identifier for this policy
    pub policy_id: String,

    /// Type of policy
    #[serde(rename = "type")]
    pub policy_type: PolicyType,

    /// Action patterns this policy applies to
    pub actions: Vec<String>,

    /// Reason for this policy (shown when triggered)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Policy-specific parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

impl AtlasPolicy {
    /// Create a new deny policy
    pub fn deny(policy_id: String, actions: Vec<String>, reason: String) -> Self {
        Self {
            policy_id,
            policy_type: PolicyType::Deny,
            actions,
            reason: Some(reason),
            parameters: None,
        }
    }

    /// Create a new allow policy
    pub fn allow(policy_id: String, actions: Vec<String>) -> Self {
        Self {
            policy_id,
            policy_type: PolicyType::Allow,
            actions,
            reason: None,
            parameters: None,
        }
    }

    /// Create a new rate limit policy
    pub fn rate_limit(
        policy_id: String,
        actions: Vec<String>,
        max_calls: u64,
        window_seconds: u64,
    ) -> Self {
        Self {
            policy_id,
            policy_type: PolicyType::RateLimit,
            actions,
            reason: None,
            parameters: Some(serde_json::json!({
                "max_calls": max_calls,
                "window_seconds": window_seconds
            })),
        }
    }

    /// Create a new approval policy
    pub fn requires_approval(policy_id: String, actions: Vec<String>) -> Self {
        Self {
            policy_id,
            policy_type: PolicyType::RequiresApproval,
            actions,
            reason: Some("Requires human approval".to_string()),
            parameters: None,
        }
    }
}

/// Types of policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyType {
    /// Explicitly allow actions
    Allow,
    /// Deny actions
    Deny,
    /// Rate limit actions
    RateLimit,
    /// Require human approval
    RequiresApproval,
    /// Budget/cost limit
    Budget,
}

impl std::fmt::Display for PolicyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyType::Allow => write!(f, "allow"),
            PolicyType::Deny => write!(f, "deny"),
            PolicyType::RateLimit => write!(f, "rate_limit"),
            PolicyType::RequiresApproval => write!(f, "requires_approval"),
            PolicyType::Budget => write!(f, "budget"),
        }
    }
}

/// An action definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasAction {
    /// Unique identifier (e.g., "ticket.get")
    pub action_id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this action does
    pub description: String,

    /// JSON Schema for parameters
    pub parameters_schema: Value,

    /// JSON Schema for return value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns_schema: Option<Value>,

    /// Risk tier classification
    #[serde(default = "default_risk_tier")]
    pub risk_tier: String,

    /// Whether this action is idempotent
    #[serde(default)]
    pub idempotent: bool,

    /// Executor identifier (for routing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
}

fn default_risk_tier() -> String {
    "low".to_string()
}

impl AtlasAction {
    /// Create a new action
    pub fn new(action_id: String, name: String, description: String) -> Self {
        Self {
            action_id,
            name,
            description,
            parameters_schema: serde_json::json!({"type": "object"}),
            returns_schema: None,
            risk_tier: "low".to_string(),
            idempotent: false,
            executor: None,
        }
    }

    /// Set the parameters schema
    pub fn with_parameters_schema(mut self, schema: Value) -> Self {
        self.parameters_schema = schema;
        self
    }

    /// Set the returns schema
    pub fn with_returns_schema(mut self, schema: Value) -> Self {
        self.returns_schema = Some(schema);
        self
    }

    /// Set the risk tier
    pub fn with_risk_tier(mut self, tier: RiskTier) -> Self {
        self.risk_tier = tier.to_string();
        self
    }

    /// Mark as idempotent
    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }
}

/// Risk tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Critical,
}

/// Injection mode for context blocks
///
/// Controls when and how context is injected into resolutions.
/// Inspired by Claude Skills' progressive disclosure pattern.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectMode {
    /// Always inject when atlas is active (essential facts, critical warnings)
    Always,
    /// Inject when keywords match the goal (current default behavior)
    #[default]
    OnMatch,
    /// Only inject if explicitly requested via context_hints
    OnDemand,
    /// Inject based on the risk tier of the request
    RiskBased,
}

impl std::fmt::Display for InjectMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InjectMode::Always => write!(f, "always"),
            InjectMode::OnMatch => write!(f, "on_match"),
            InjectMode::OnDemand => write!(f, "on_demand"),
            InjectMode::RiskBased => write!(f, "risk_based"),
        }
    }
}

/// External sources for an atlas
///
/// Links to repositories, documentation, and demos for deeper reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtlasSources {
    /// Source code repositories
    #[serde(default)]
    pub repositories: Vec<String>,

    /// Documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,

    /// Live demo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo: Option<String>,
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
    use serde_json::json;

    #[test]
    fn test_manifest_builder() {
        let manifest = AtlasManifest::builder(
            "com.test.example".to_string(),
            "Test Atlas".to_string(),
        )
        .version("2.0.0")
        .description("A test atlas")
        .license("MIT")
        .domains(vec!["test".to_string()])
        .add_action(AtlasAction::new(
            "test.action".to_string(),
            "Test Action".to_string(),
            "A test action".to_string(),
        ))
        .build();

        assert_eq!(manifest.atlas_id, "com.test.example");
        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.actions.len(), 1);
    }

    #[test]
    fn test_manifest_validation() {
        let valid = AtlasManifest::builder(
            "com.test.valid".to_string(),
            "Valid Atlas".to_string(),
        )
        .add_action(AtlasAction::new(
            "test.action".to_string(),
            "Test".to_string(),
            "Test".to_string(),
        ))
        .add_capability(AtlasCapability::new(
            "test.cap".to_string(),
            "Test Cap".to_string(),
            vec!["test.action".to_string()],
        ))
        .build();

        assert!(valid.validate().is_ok());

        // Invalid: capability references unknown action
        let invalid = AtlasManifest::builder(
            "com.test.invalid".to_string(),
            "Invalid Atlas".to_string(),
        )
        .add_capability(AtlasCapability::new(
            "test.cap".to_string(),
            "Test Cap".to_string(),
            vec!["unknown.action".to_string()],
        ))
        .build();

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_policy_helpers() {
        let deny = AtlasPolicy::deny(
            "deny-delete".to_string(),
            vec!["*.delete".to_string()],
            "No deletions".to_string(),
        );
        assert_eq!(deny.policy_type, PolicyType::Deny);

        let rate_limit = AtlasPolicy::rate_limit(
            "rate-api".to_string(),
            vec!["api.*".to_string()],
            100,
            60,
        );
        assert_eq!(rate_limit.policy_type, PolicyType::RateLimit);
        assert!(rate_limit.parameters.is_some());
    }

    #[test]
    fn test_action_builder() {
        let action = AtlasAction::new(
            "ticket.create".to_string(),
            "Create Ticket".to_string(),
            "Create a new support ticket".to_string(),
        )
        .with_parameters_schema(json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": { "type": "string" }
            }
        }))
        .with_risk_tier(RiskTier::Medium)
        .idempotent();

        assert_eq!(action.risk_tier, "medium");
        assert!(action.idempotent);
    }

    #[test]
    fn test_inject_mode_default() {
        let block: AtlasContextBlock = serde_json::from_str(r#"{
            "context_id": "test",
            "name": "Test",
            "content": "Test content"
        }"#).unwrap();

        assert_eq!(block.inject_mode, InjectMode::OnMatch);
    }

    #[test]
    fn test_inject_mode_always() {
        let block: AtlasContextBlock = serde_json::from_str(r#"{
            "context_id": "essential",
            "name": "Essential Facts",
            "content": "Critical information",
            "inject_mode": "always",
            "priority": 350
        }"#).unwrap();

        assert_eq!(block.inject_mode, InjectMode::Always);
        assert_eq!(block.priority, 350);
    }

    #[test]
    fn test_atlas_sources() {
        let sources: AtlasSources = serde_json::from_str(r#"{
            "repositories": ["https://github.com/example/repo"],
            "documentation": "https://docs.example.com",
            "demo": "https://demo.example.com"
        }"#).unwrap();

        assert_eq!(sources.repositories.len(), 1);
        assert_eq!(sources.documentation, Some("https://docs.example.com".to_string()));
        assert_eq!(sources.demo, Some("https://demo.example.com".to_string()));
    }

    #[test]
    fn test_manifest_with_sources() {
        let manifest = AtlasManifest::builder(
            "com.test.sources".to_string(),
            "Sources Test".to_string(),
        )
        .sources(AtlasSources {
            repositories: vec!["https://github.com/test/repo".to_string()],
            documentation: Some("https://docs.test.com".to_string()),
            demo: None,
        })
        .build();

        assert!(manifest.sources.is_some());
        let sources = manifest.sources.unwrap();
        assert_eq!(sources.repositories.len(), 1);
    }

    #[test]
    fn test_also_inject() {
        let block: AtlasContextBlock = serde_json::from_str(r#"{
            "context_id": "workflow",
            "name": "Workflow Guide",
            "content": "How to do X",
            "also_inject": ["essential-facts", "parameters-ref"]
        }"#).unwrap();

        assert_eq!(block.also_inject.len(), 2);
        assert_eq!(block.also_inject[0], "essential-facts");
    }
}
