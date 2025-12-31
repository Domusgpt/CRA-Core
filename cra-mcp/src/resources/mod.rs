//! MCP Resource implementations
//!
//! Resources are data that agents can read through the MCP protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Resource definition for MCP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    /// Resource URI pattern
    pub uri: String,

    /// Description
    pub description: String,

    /// MIME type of the resource
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// Get all CRA resource definitions
pub fn get_resource_definitions() -> Vec<ResourceDefinition> {
    vec![
        ResourceDefinition {
            uri: "cra://session/current".to_string(),
            description: "Current session state, loaded atlases, and context received".to_string(),
            mime_type: "application/json".to_string(),
        },
        ResourceDefinition {
            uri: "cra://trace/{session_id}".to_string(),
            description: "TRACE audit trail for a session".to_string(),
            mime_type: "application/x-ndjson".to_string(),
        },
        ResourceDefinition {
            uri: "cra://atlas/{atlas_id}".to_string(),
            description: "Full atlas manifest for inspection".to_string(),
            mime_type: "application/json".to_string(),
        },
        ResourceDefinition {
            uri: "cra://chain/{session_id}".to_string(),
            description: "Chain verification status for a session".to_string(),
            mime_type: "application/json".to_string(),
        },
    ]
}

/// Session resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResource {
    pub session_id: String,
    pub agent_id: String,
    pub goal: String,
    pub started_at: String,
    pub active_atlases: Vec<String>,
    pub injected_contexts: Vec<String>,
    pub event_count: u64,
    pub current_hash: String,
}

/// Trace resource content (JSONL format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResource {
    pub session_id: String,
    pub event_count: usize,
    pub events: Vec<Value>,
}

/// Atlas resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasResource {
    pub atlas_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub domains: Vec<String>,
    pub action_count: usize,
    pub policy_count: usize,
    pub context_count: usize,
}

/// Chain verification resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainResource {
    pub session_id: String,
    pub is_valid: bool,
    pub event_count: usize,
    pub genesis_hash: String,
    pub current_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_invalid_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
