//! Context Registry - The "C" in CRA
//!
//! The Context Registry manages context injection into agent prompts.
//! It bridges Atlas context_packs to Resolution context_blocks.
//!
//! ## Architecture
//!
//! ```text
//! Atlas (context_packs)     CARP Request (goal, hints)
//!         │                           │
//!         ▼                           ▼
//!    ┌────────────────────────────────────┐
//!    │         Context Registry           │
//!    │                                    │
//!    │  1. Load context files from Atlas  │
//!    │  2. Evaluate pack conditions       │
//!    │  3. Match goal to relevant packs   │
//!    │  4. Generate ContextBlocks         │
//!    │  5. Emit context.injected events   │
//!    └────────────────────────────────────┘
//!                     │
//!                     ▼
//!         Resolution (context_blocks)
//! ```

mod registry;
mod matcher;

pub use registry::{ContextRegistry, LoadedContext, ContextSource};
pub use matcher::{ContextMatcher, MatchResult, MatchScore};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_registry_basic() {
        let mut registry = ContextRegistry::new();

        // Add some context
        registry.add_context(LoadedContext {
            pack_id: "hash-rules".to_string(),
            source: ContextSource::Atlas("dev.cra.self-governance".to_string()),
            content: "NEVER reimplement hash computation".to_string(),
            content_type: "text/markdown".to_string(),
            priority: 100,
            keywords: vec!["hash".to_string(), "trace".to_string(), "event".to_string()],
            conditions: Some(json!({"file_pattern": "trace/*.rs"})),
        });

        // Query for matching context
        let matches = registry.query("editing trace event hashing", None);
        assert!(!matches.is_empty());
    }
}
