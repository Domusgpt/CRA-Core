//! Context Registry implementation

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::atlas::AtlasContextPack;
use crate::carp::ContextBlock;

/// Source of context content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextSource {
    /// Context from an Atlas
    Atlas(String),  // atlas_id
    /// Context from a file path
    File(String),   // file path
    /// Inline context
    Inline,
    /// Runtime-generated context
    Runtime(String), // generator name
}

impl ContextSource {
    pub fn as_string(&self) -> String {
        match self {
            ContextSource::Atlas(id) => id.clone(),
            ContextSource::File(path) => format!("file:{}", path),
            ContextSource::Inline => "inline".to_string(),
            ContextSource::Runtime(name) => format!("runtime:{}", name),
        }
    }
}

/// A loaded context item ready for injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedContext {
    /// Pack identifier
    pub pack_id: String,

    /// Where this context came from
    pub source: ContextSource,

    /// The actual content to inject
    pub content: String,

    /// Content type (text/markdown, application/json, etc.)
    pub content_type: String,

    /// Priority for ordering (higher = injected first)
    pub priority: i32,

    /// Keywords for matching
    pub keywords: Vec<String>,

    /// Conditions for when to inject
    pub conditions: Option<Value>,
}

impl LoadedContext {
    /// Estimate token count (rough: ~4 chars per token)
    pub fn token_estimate(&self) -> usize {
        self.content.len() / 4
    }

    /// Convert to ContextBlock for resolution
    pub fn to_context_block(&self) -> ContextBlock {
        ContextBlock::new(
            self.pack_id.clone(),
            self.source.as_string(),
            self.content_type.clone(),
            self.content.clone(),
            self.priority,
        )
    }
}

/// The Context Registry - manages and queries available context
#[derive(Debug, Default)]
pub struct ContextRegistry {
    /// All loaded context items
    contexts: Vec<LoadedContext>,

    /// Index by pack_id for quick lookup
    by_pack_id: HashMap<String, usize>,

    /// Index by atlas_id for scoped queries
    by_atlas: HashMap<String, Vec<usize>>,

    /// Keywords index for semantic matching
    keyword_index: HashMap<String, Vec<usize>>,
}

impl ContextRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Add context to the registry
    pub fn add_context(&mut self, context: LoadedContext) {
        let idx = self.contexts.len();

        // Index by pack_id
        self.by_pack_id.insert(context.pack_id.clone(), idx);

        // Index by atlas
        if let ContextSource::Atlas(atlas_id) = &context.source {
            self.by_atlas
                .entry(atlas_id.clone())
                .or_default()
                .push(idx);
        }

        // Index keywords
        for keyword in &context.keywords {
            self.keyword_index
                .entry(keyword.to_lowercase())
                .or_default()
                .push(idx);
        }

        self.contexts.push(context);
    }

    /// Load context from an Atlas context_pack
    pub fn load_from_pack(
        &mut self,
        atlas_id: &str,
        pack: &AtlasContextPack,
        file_loader: impl Fn(&str) -> Option<String>,
    ) {
        // Load each file in the pack
        for file_path in &pack.files {
            if let Some(content) = file_loader(file_path) {
                // Extract keywords from filename and content
                let keywords = Self::extract_keywords(&content, file_path);

                let context = LoadedContext {
                    pack_id: pack.pack_id.clone(),
                    source: ContextSource::Atlas(atlas_id.to_string()),
                    content,
                    content_type: Self::infer_content_type(file_path),
                    priority: pack.priority,
                    keywords,
                    conditions: pack.conditions.clone(),
                };

                self.add_context(context);
            }
        }
    }

    /// Query for matching context based on goal text
    pub fn query(&self, goal: &str, atlas_filter: Option<&str>) -> Vec<&LoadedContext> {
        let goal_words: Vec<String> = goal
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut scored: Vec<(usize, i32)> = Vec::new();

        // Score each context item
        for (idx, context) in self.contexts.iter().enumerate() {
            // Apply atlas filter
            if let Some(filter) = atlas_filter {
                if let ContextSource::Atlas(atlas_id) = &context.source {
                    if atlas_id != filter {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Check conditions (if any)
            if let Some(conditions) = &context.conditions {
                if !self.evaluate_conditions(conditions, goal) {
                    continue;
                }
            }

            // Calculate match score
            let mut score = context.priority;

            // Keyword matching
            for word in &goal_words {
                if context.keywords.iter().any(|k| k.contains(word) || word.contains(k)) {
                    score += 10;
                }
            }

            // Content matching (lower weight)
            let content_lower = context.content.to_lowercase();
            for word in &goal_words {
                if content_lower.contains(word) {
                    score += 2;
                }
            }

            if score > context.priority {
                // Only include if there was some match
                scored.push((idx, score));
            }
        }

        // Sort by score (highest first)
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        // Return matching contexts
        scored.iter().map(|(idx, _)| &self.contexts[*idx]).collect()
    }

    /// Query and return as ContextBlocks ready for resolution
    pub fn query_as_blocks(&self, goal: &str, atlas_filter: Option<&str>) -> Vec<ContextBlock> {
        self.query(goal, atlas_filter)
            .into_iter()
            .map(|ctx| ctx.to_context_block())
            .collect()
    }

    /// Get context by pack_id
    pub fn get_by_pack_id(&self, pack_id: &str) -> Option<&LoadedContext> {
        self.by_pack_id.get(pack_id).map(|idx| &self.contexts[*idx])
    }

    /// Get all context for an atlas
    pub fn get_by_atlas(&self, atlas_id: &str) -> Vec<&LoadedContext> {
        self.by_atlas
            .get(atlas_id)
            .map(|indices| indices.iter().map(|idx| &self.contexts[*idx]).collect())
            .unwrap_or_default()
    }

    /// Get all loaded contexts
    pub fn all(&self) -> &[LoadedContext] {
        &self.contexts
    }

    /// Number of loaded context items
    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }

    /// Evaluate conditions against a goal
    fn evaluate_conditions(&self, conditions: &Value, goal: &str) -> bool {
        // Simple condition evaluation
        // TODO: Expand with proper expression language

        if let Some(keywords) = conditions.get("keywords").and_then(|v| v.as_array()) {
            let goal_lower = goal.to_lowercase();
            for keyword in keywords {
                if let Some(kw) = keyword.as_str() {
                    if goal_lower.contains(&kw.to_lowercase()) {
                        return true;
                    }
                }
            }
            return false;
        }

        if let Some(pattern) = conditions.get("file_pattern").and_then(|v| v.as_str()) {
            // File pattern matching - check if goal mentions the pattern
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            for part in pattern_parts {
                let clean = part.replace("*", "").replace(".", "");
                if !clean.is_empty() && goal.to_lowercase().contains(&clean.to_lowercase()) {
                    return true;
                }
            }
            return false;
        }

        // Default: include if no conditions specified
        true
    }

    /// Extract keywords from content and filename
    fn extract_keywords(content: &str, file_path: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // Extract from filename
        let filename = file_path.split('/').last().unwrap_or("");
        let name_parts: Vec<&str> = filename.split(&['-', '_', '.'][..]).collect();
        for part in name_parts {
            if part.len() > 2 && part != "md" && part != "json" && part != "txt" {
                keywords.push(part.to_lowercase());
            }
        }

        // Extract important words from content (headers, emphasized text)
        for line in content.lines() {
            // Markdown headers
            if line.starts_with('#') {
                let header = line.trim_start_matches('#').trim();
                keywords.extend(
                    header.split_whitespace()
                        .filter(|w| w.len() > 3)
                        .map(|w| w.to_lowercase())
                );
            }

            // Words in bold/caps
            for word in line.split_whitespace() {
                if word.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) && word.len() > 3 {
                    keywords.push(word.to_lowercase());
                }
            }
        }

        // Dedupe
        keywords.sort();
        keywords.dedup();
        keywords
    }

    /// Infer content type from file extension
    fn infer_content_type(file_path: &str) -> String {
        match file_path.split('.').last() {
            Some("md") => "text/markdown".to_string(),
            Some("json") => "application/json".to_string(),
            Some("txt") => "text/plain".to_string(),
            Some("yaml") | Some("yml") => "application/yaml".to_string(),
            _ => "text/plain".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_query_context() {
        let mut registry = ContextRegistry::new();

        registry.add_context(LoadedContext {
            pack_id: "hash-rules".to_string(),
            source: ContextSource::Atlas("dev.cra".to_string()),
            content: "Never reimplement hash computation. Use TRACEEvent::compute_hash().".to_string(),
            content_type: "text/markdown".to_string(),
            priority: 100,
            keywords: vec!["hash".to_string(), "trace".to_string(), "compute".to_string()],
            conditions: None,
        });

        registry.add_context(LoadedContext {
            pack_id: "policy-rules".to_string(),
            source: ContextSource::Atlas("dev.cra".to_string()),
            content: "Policies are evaluated in deny-first order.".to_string(),
            content_type: "text/markdown".to_string(),
            priority: 50,
            keywords: vec!["policy".to_string(), "deny".to_string(), "evaluate".to_string()],
            conditions: None,
        });

        // Query for hash-related context
        let results = registry.query("working on hash chain implementation", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pack_id, "hash-rules");

        // Query for policy-related context
        let results = registry.query("how are policies evaluated", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pack_id, "policy-rules");
    }

    #[test]
    fn test_conditional_matching() {
        let mut registry = ContextRegistry::new();

        registry.add_context(LoadedContext {
            pack_id: "trace-editing".to_string(),
            source: ContextSource::Atlas("dev.cra".to_string()),
            content: "When editing trace files...".to_string(),
            content_type: "text/markdown".to_string(),
            priority: 100,
            keywords: vec!["trace".to_string()],
            conditions: Some(serde_json::json!({"keywords": ["trace", "event", "hash"]})),
        });

        // Should match when keywords present
        let results = registry.query("editing trace events", None);
        assert_eq!(results.len(), 1);

        // Should not match when keywords absent
        let results = registry.query("editing policy rules", None);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_to_context_block() {
        let context = LoadedContext {
            pack_id: "test".to_string(),
            source: ContextSource::Atlas("com.test".to_string()),
            content: "Test content".to_string(),
            content_type: "text/markdown".to_string(),
            priority: 50,
            keywords: vec![],
            conditions: None,
        };

        let block = context.to_context_block();
        assert_eq!(block.block_id, "test");
        assert_eq!(block.source, "com.test");
        assert_eq!(block.content, "Test content");
        assert_eq!(block.priority, 50);
    }
}
