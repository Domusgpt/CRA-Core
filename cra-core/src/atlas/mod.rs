//! Atlas/1.0 — Agent Context Package Format
//!
//! An Atlas is a versioned package containing everything needed to govern
//! agent behavior in a domain:
//!
//! - Context documents (knowledge, policies, procedures)
//! - Policy definitions (deny, allow, rate limit, approval)
//! - Action definitions (tools available to agents)
//! - Capability groupings
//! - Platform-specific adapters

mod manifest;
mod loader;
mod validator;

pub use manifest::{
    AtlasManifest, AtlasAction, AtlasPolicy, AtlasCapability, AtlasContextPack,
    AtlasContextBlock, PolicyType, RiskTier, InjectMode, AtlasSources,
};
pub use loader::AtlasLoader;
pub use validator::AtlasValidator;

/// Atlas format version
pub const VERSION: &str = "1.0";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_atlas_manifest_serialization() {
        let manifest = AtlasManifest {
            atlas_version: VERSION.to_string(),
            atlas_id: "com.test.example".to_string(),
            version: "1.0.0".to_string(),
            name: "Test Atlas".to_string(),
            description: "An atlas for testing".to_string(),
            authors: vec!["Test Author".to_string()],
            license: Some("MIT".to_string()),
            domains: vec!["test".to_string()],
            capabilities: vec![],
            context_packs: vec![],
            context_blocks: vec![],
            policies: vec![],
            actions: vec![],
            dependencies: None,
            sources: None,
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: AtlasManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.atlas_id, "com.test.example");
        assert_eq!(parsed.version, "1.0.0");
    }

    #[test]
    fn test_atlas_action_serialization() {
        let action = AtlasAction {
            action_id: "ticket.get".to_string(),
            name: "Get Ticket".to_string(),
            description: "Retrieve a ticket by ID".to_string(),
            parameters_schema: json!({
                "type": "object",
                "required": ["ticket_id"],
                "properties": {
                    "ticket_id": { "type": "string" }
                }
            }),
            returns_schema: Some(json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "title": { "type": "string" }
                }
            })),
            risk_tier: "low".to_string(),
            idempotent: true,
            executor: None,
        };

        let json = serde_json::to_string(&action).unwrap();
        let parsed: AtlasAction = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.action_id, "ticket.get");
        assert!(parsed.idempotent);
    }
}
