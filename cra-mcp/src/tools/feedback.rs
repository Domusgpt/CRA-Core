//! Feedback tools

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ToolDefinition;

/// cra_feedback tool definition
pub fn feedback_tool() -> ToolDefinition {
    ToolDefinition {
        name: "cra_feedback".to_string(),
        description: "Report if context was helpful. This improves CRA's context matching for future requests and helps atlas maintainers improve their content.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["context_id", "helpful"],
            "properties": {
                "context_id": {
                    "type": "string",
                    "description": "Which context block you're giving feedback on"
                },
                "helpful": {
                    "type": "boolean",
                    "description": "Was this context helpful for your task?"
                },
                "reason": {
                    "type": "string",
                    "description": "Why it was or wasn't helpful (improves atlas quality)"
                }
            }
        }),
    }
}

/// Input for cra_feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackInput {
    pub context_id: String,
    pub helpful: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Output from cra_feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackOutput {
    pub recorded: bool,
    pub trace_id: String,
}
