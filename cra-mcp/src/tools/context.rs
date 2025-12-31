//! Context-related tools

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ToolDefinition;

/// cra_request_context tool definition
pub fn request_context_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_request_context".to_string(),
        description: "Ask CRA for context relevant to what you're working on. Be specific about what information would help. This retrieves domain knowledge from loaded atlases.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["need"],
            "properties": {
                "need": {
                    "type": "string",
                    "description": "What information would help you right now"
                },
                "hints": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional keywords to improve context matching"
                }
            }
        }),
    }
}

/// cra_search_contexts tool definition
pub fn search_contexts_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_search_contexts".to_string(),
        description: "Search all available context blocks across loaded atlases.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for context content and keywords"
                },
                "limit": {
                    "type": "integer",
                    "default": 10,
                    "description": "Maximum results to return"
                }
            }
        }),
    }
}

/// cra_list_atlases tool definition
pub fn list_atlases_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_list_atlases".to_string(),
        description: "List all available atlases that can provide context.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
    }
}

/// Input for cra_request_context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContextInput {
    pub need: String,
    #[serde(default)]
    pub hints: Vec<String>,
}

/// Output from cra_request_context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContextOutput {
    pub matched_contexts: Vec<MatchedContext>,
    pub trace_id: String,
}

/// A matched context block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedContext {
    pub context_id: String,
    pub name: String,
    pub priority: i32,
    pub match_score: f64,
    pub content: String,
}

/// Input for cra_search_contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContextsInput {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }

/// Output from cra_search_contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContextsOutput {
    pub results: Vec<ContextSearchResult>,
    pub total_count: usize,
}

/// A search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSearchResult {
    pub context_id: String,
    pub atlas_id: String,
    pub name: String,
    pub snippet: String,
    pub relevance: f64,
}

/// Output from cra_list_atlases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAtlasesOutput {
    pub atlases: Vec<AtlasInfo>,
}

/// Atlas information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasInfo {
    pub atlas_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub domains: Vec<String>,
}
