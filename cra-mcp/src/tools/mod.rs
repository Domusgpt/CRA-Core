//! MCP Tool implementations
//!
//! These are the tools exposed to agents through the MCP protocol.

pub mod session;
pub mod context;
pub mod action;
pub mod feedback;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool definition for MCP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,

    /// Description shown to the agent
    pub description: String,

    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Get all CRA tool definitions
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        session::start_session_tool(),
        session::end_session_tool(),
        context::request_context_tool(),
        context::search_contexts_tool(),
        context::list_atlases_tool(),
        action::report_action_tool(),
        feedback::feedback_tool(),
        session::bootstrap_tool(),
    ]
}
