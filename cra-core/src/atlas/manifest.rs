//! Atlas Manifest types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VERSION;
use super::steward::StewardConfig;
use crate::carp::{StewardCheckpointDef, CheckpointTrigger};

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

    /// Steward configuration (access, delivery, notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<StewardConfig>,

    /// Capability groupings
    #[serde(default)]
    pub capabilities: Vec<AtlasCapability>,

    /// Steward-defined checkpoints (interactive gates, guidance injection)
    #[serde(default)]
    pub checkpoints: Vec<StewardCheckpointDef>,

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

    /// Get a checkpoint by ID
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<&StewardCheckpointDef> {
        self.checkpoints
            .iter()
            .find(|c| c.checkpoint_id == checkpoint_id)
    }

    /// Get all checkpoints that should trigger on session start
    pub fn get_session_start_checkpoints(&self) -> Vec<&StewardCheckpointDef> {
        self.checkpoints
            .iter()
            .filter(|c| matches!(c.trigger, CheckpointTrigger::SessionStart))
            .collect()
    }

    /// Get all checkpoints for a given action pattern
    pub fn get_action_checkpoints(&self, action_id: &str) -> Vec<&StewardCheckpointDef> {
        self.checkpoints
            .iter()
            .filter(|c| {
                match &c.trigger {
                    CheckpointTrigger::ActionPre { patterns } |
                    CheckpointTrigger::ActionPost { patterns } => {
                        patterns.iter().any(|p| Self::pattern_matches(p, action_id))
                    }
                    _ => false,
                }
            })
            .collect()
    }

    /// Check if a pattern matches an action ID
    /// Supports wildcards: "*.delete" matches "user.delete", "ticket.*" matches "ticket.get"
    fn pattern_matches(pattern: &str, action_id: &str) -> bool {
        if pattern == action_id {
            return true;
        }
        if pattern.starts_with('*') {
            // *.delete matches user.delete
            let suffix = pattern.trim_start_matches('*');
            action_id.ends_with(suffix)
        } else if pattern.ends_with('*') {
            // ticket.* matches ticket.get
            let prefix = pattern.trim_end_matches('*');
            action_id.starts_with(prefix)
        } else if pattern.contains('*') {
            // More complex patterns - simple glob matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                action_id.starts_with(parts[0]) && action_id.ends_with(parts[1])
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get all checkpoints for a given capability
    pub fn get_capability_checkpoints(&self, capability_id: &str) -> Vec<&StewardCheckpointDef> {
        self.checkpoints
            .iter()
            .filter(|c| {
                matches!(&c.trigger, CheckpointTrigger::CapabilityAccess { capability_ids }
                    if capability_ids.contains(&capability_id.to_string()))
            })
            .collect()
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

        // Validate checkpoints have unique IDs
        let mut checkpoint_ids: Vec<&str> = self.checkpoints.iter().map(|c| c.checkpoint_id.as_str()).collect();
        checkpoint_ids.sort();
        for window in checkpoint_ids.windows(2) {
            if window[0] == window[1] {
                errors.push(format!("Duplicate checkpoint_id: {}", window[0]));
            }
        }

        // Validate checkpoint context references exist
        for checkpoint in &self.checkpoints {
            for context_id in &checkpoint.inject_contexts {
                let context_exists = self.context_blocks.iter().any(|b| &b.context_id == context_id)
                    || self.context_packs.iter().any(|p| &p.pack_id == context_id);
                if !context_exists {
                    errors.push(format!(
                        "Checkpoint {} references unknown context: {}",
                        checkpoint.checkpoint_id, context_id
                    ));
                }
            }

            // Validate capability references
            for cap_id in &checkpoint.unlock_capabilities {
                if self.get_capability(cap_id).is_none() {
                    errors.push(format!(
                        "Checkpoint {} references unknown capability to unlock: {}",
                        checkpoint.checkpoint_id, cap_id
                    ));
                }
            }
            for cap_id in &checkpoint.lock_capabilities {
                if self.get_capability(cap_id).is_none() {
                    errors.push(format!(
                        "Checkpoint {} references unknown capability to lock: {}",
                        checkpoint.checkpoint_id, cap_id
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
                steward: None,
                capabilities: vec![],
                checkpoints: vec![],
                context_packs: vec![],
                context_blocks: vec![],
                policies: vec![],
                actions: vec![],
                dependencies: None,
                sources: None,
            },
        }
    }

    pub fn steward(mut self, steward: StewardConfig) -> Self {
        self.manifest.steward = Some(steward);
        self
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

    pub fn add_checkpoint(mut self, checkpoint: StewardCheckpointDef) -> Self {
        self.manifest.checkpoints.push(checkpoint);
        self
    }

    pub fn add_context_block(mut self, block: AtlasContextBlock) -> Self {
        self.manifest.context_blocks.push(block);
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

    #[test]
    fn test_manifest_with_checkpoints() {
        use crate::carp::{
            StewardCheckpointDef, CheckpointTrigger, CheckpointQuestion, GuidanceBlock,
        };

        let manifest = AtlasManifest::builder(
            "com.test.checkpoints".to_string(),
            "Checkpoints Test".to_string(),
        )
        .add_capability(AtlasCapability::new(
            "basic-access".to_string(),
            "Basic Access".to_string(),
            vec![],
        ))
        .add_context_block(AtlasContextBlock {
            context_id: "onboarding".to_string(),
            name: "Onboarding".to_string(),
            priority: 100,
            content: "Welcome to the system!".to_string(),
            content_type: "text/markdown".to_string(),
            inject_mode: InjectMode::OnDemand,
            also_inject: vec![],
            inject_when: vec![],
            keywords: vec![],
            risk_tiers: vec![],
        })
        .add_checkpoint(
            StewardCheckpointDef::new(
                "session-onboarding",
                "Session Onboarding",
                CheckpointTrigger::SessionStart,
            )
            .blocking()
            .with_question(CheckpointQuestion::boolean(
                "agree-terms",
                "Do you agree to the terms of service?",
            ))
            .with_guidance(GuidanceBlock::text("Welcome!"))
            .inject_contexts(vec!["onboarding".to_string()])
            .unlock_capabilities(vec!["basic-access".to_string()])
        )
        .add_checkpoint(
            StewardCheckpointDef::new(
                "delete-confirm",
                "Delete Confirmation",
                CheckpointTrigger::ActionPre {
                    patterns: vec!["*.delete".to_string()],
                },
            )
            .blocking()
            .with_question(CheckpointQuestion::acknowledgment(
                "confirm-delete",
                "I understand this action cannot be undone.",
            ))
        )
        .build();

        // Test checkpoint counts
        assert_eq!(manifest.checkpoints.len(), 2);

        // Test get_checkpoint
        let onboarding = manifest.get_checkpoint("session-onboarding");
        assert!(onboarding.is_some());
        assert_eq!(onboarding.unwrap().questions.len(), 1);

        // Test get_session_start_checkpoints
        let session_start = manifest.get_session_start_checkpoints();
        assert_eq!(session_start.len(), 1);
        assert_eq!(session_start[0].checkpoint_id, "session-onboarding");

        // Test get_action_checkpoints
        let delete_checkpoints = manifest.get_action_checkpoints("user.delete");
        assert_eq!(delete_checkpoints.len(), 1);
        assert_eq!(delete_checkpoints[0].checkpoint_id, "delete-confirm");

        // Test validation passes
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_checkpoint_validation_errors() {
        use crate::carp::{StewardCheckpointDef, CheckpointTrigger};

        let manifest = AtlasManifest::builder(
            "com.test.invalid".to_string(),
            "Invalid Checkpoints".to_string(),
        )
        .add_checkpoint(
            StewardCheckpointDef::new(
                "bad-checkpoint",
                "References Unknown Context",
                CheckpointTrigger::SessionStart,
            )
            .inject_contexts(vec!["nonexistent-context".to_string()])
            .unlock_capabilities(vec!["nonexistent-capability".to_string()])
        )
        .build();

        let result = manifest.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("unknown context")));
        assert!(errors.iter().any(|e| e.contains("unknown capability")));
    }
}
