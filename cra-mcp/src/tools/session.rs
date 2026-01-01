//! Session management tools

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ToolDefinition;

/// cra_start_session tool definition
pub fn start_session_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_start_session".to_string(),
        description: "Start a governed session with CRA. Call this first before using other CRA tools. This establishes governance rules and provides initial domain context.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["goal"],
            "properties": {
                "goal": {
                    "type": "string",
                    "description": "What you're trying to accomplish in this session"
                },
                "atlas_hints": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional: domains/atlases relevant to your task"
                }
            }
        }),
    }
}

/// cra_end_session tool definition
pub fn end_session_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_end_session".to_string(),
        description: "End the CRA session. Finalizes the audit trail and uploads any remaining TRACE data.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "Optional summary of what was accomplished"
                }
            }
        }),
    }
}

/// cra_bootstrap tool definition
pub fn bootstrap_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_bootstrap".to_string(),
        description: "Initialize CRA governance with full bootstrap handshake. MUST be called before any other CRA tools. This establishes governance rules, streams domain context, and creates the audit trail.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["intent"],
            "properties": {
                "intent": {
                    "type": "string",
                    "description": "What you're trying to accomplish"
                },
                "capabilities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "What tools/abilities you have"
                }
            }
        }),
    }
}

/// Input for cra_start_session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionInput {
    pub goal: String,
    #[serde(default)]
    pub atlas_hints: Vec<String>,
}

/// Output from cra_start_session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionOutput {
    pub session_id: String,
    pub active_atlases: Vec<String>,
    pub initial_context: Vec<InitialContext>,
    pub genesis_hash: String,
}

/// Initial context provided at session start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialContext {
    pub context_id: String,
    pub priority: i32,
    pub content: String,
}

/// Input for cra_end_session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndSessionInput {
    #[serde(default)]
    pub summary: Option<String>,
}

/// Output from cra_end_session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndSessionOutput {
    pub session_id: String,
    pub duration_ms: i64,
    pub event_count: u64,
    pub chain_verified: bool,
    pub final_hash: String,
}

/// Input for cra_bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapInput {
    pub intent: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// cra_get_trace tool definition
pub fn get_trace_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_get_trace".to_string(),
        description: "Export the TRACE audit trail for the current session. Returns all events with their hashes for verification. Use this to save a copy of the trace before ending the session.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Optional: specific session ID. If not provided, uses current session."
                },
                "format": {
                    "type": "string",
                    "enum": ["json", "jsonl"],
                    "description": "Output format. 'json' returns an array, 'jsonl' returns newline-delimited JSON."
                }
            }
        }),
    }
}

/// Input for cra_get_trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTraceInput {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

/// Output from cra_get_trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTraceOutput {
    pub session_id: String,
    pub event_count: usize,
    pub genesis_hash: String,
    pub current_hash: String,
    pub is_valid: bool,
    pub events: serde_json::Value,
}
