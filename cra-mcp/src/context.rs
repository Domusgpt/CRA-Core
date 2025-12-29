//! Context management for auto-injection

use crate::SystemContext;

/// Build onboarding context for new agents
pub fn build_onboarding_context() -> SystemContext {
    SystemContext {
        id: "onboarding".to_string(),
        uri: "cra://system/onboarding".to_string(),
        name: "Welcome to CRA".to_string(),
        mime_type: "text/markdown".to_string(),
        content: r#"# Welcome to CRA-Governed Environment

Hi! You're now connected to a governance-enabled system. This is a good thing - it means:

1. **Clear boundaries** - You know exactly what you can and can't do
2. **No guessing** - Just ask and the system tells you
3. **Transparency** - Everything is logged for the user's benefit

## Your First Steps

1. Run `cra_list_actions` to see what's available
2. Before any action, run `cra_check` to verify it's allowed
3. If something is denied, explain to the user and suggest alternatives

## Quick Tips

- When in doubt, `cra_help` is your friend
- It's always better to check than to be blocked
- Denials aren't failures - they're information

You're ready to go. Welcome aboard!
"#.to_string(),
    }
}

/// Build context blocks from an Atlas manifest
pub fn extract_atlas_context(atlas: &cra_core::AtlasManifest) -> Vec<SystemContext> {
    // Would extract context_blocks from the atlas
    // For now, create a summary context
    vec![SystemContext {
        id: format!("atlas-{}", atlas.atlas_id),
        uri: format!("cra://atlas/{}/info", atlas.atlas_id),
        name: format!("Atlas: {}", atlas.name),
        mime_type: "text/markdown".to_string(),
        content: format!(
            "# {}\n\n{}\n\n**Version:** {}\n**Domains:** {}\n\n**Actions:** {}",
            atlas.name,
            atlas.description,
            atlas.version,
            atlas.domains.join(", "),
            atlas.actions.len()
        ),
    }]
}
