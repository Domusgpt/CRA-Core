//! Action reporting tools

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ToolDefinition;

/// cra_report_action tool definition
pub fn report_action_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_report_action".to_string(),
        description: "Tell CRA what action you're taking. This creates an audit trail entry and evaluates the action against policies. Call this before executing significant actions.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "description": "What you're doing (e.g., 'write_file', 'execute_code', 'api_call')"
                },
                "params": {
                    "type": "object",
                    "description": "Relevant parameters for the action"
                }
            }
        }),
    }
}

/// Input for cra_report_action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportActionInput {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Output from cra_report_action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportActionOutput {
    /// Decision: "approved" or "denied"
    pub decision: String,

    /// Trace ID for this event
    pub trace_id: String,

    /// Reason (for denials)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Policy notes
    pub policy_notes: Vec<String>,

    /// Suggested alternatives (for denials)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alternatives: Vec<String>,
}

/// Action result for completed actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub execution_id: String,
    pub success: bool,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
