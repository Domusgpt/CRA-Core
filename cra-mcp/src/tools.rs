//! MCP Tool implementations

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CRAMCPServer;

/// Tool call request
#[derive(Debug, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Tool call response
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl CRAMCPServer {
    /// Handle a tool call
    pub fn handle_tool_call(&self, call: ToolCall) -> ToolResult {
        match call.name.as_str() {
            "cra_help" => self.handle_help(call.arguments),
            "cra_check" => self.handle_check(call.arguments),
            "cra_list_actions" => self.handle_list_actions(),
            _ => self.handle_atlas_action(call),
        }
    }

    /// Handle cra_help tool
    fn handle_help(&self, args: Value) -> ToolResult {
        let topic = args.get("topic")
            .and_then(|t| t.as_str())
            .unwrap_or("general");

        let help_text = match topic.to_lowercase().as_str() {
            "rules" | "policies" => {
                self.read_resource("cra://system/rules")
                    .unwrap_or_else(|| "Rules resource not found.".to_string())
            }
            "examples" => {
                self.read_resource("cra://system/examples")
                    .unwrap_or_else(|| "Examples resource not found.".to_string())
            }
            "actions" => {
                let actions: Vec<String> = self.get_tools()
                    .iter()
                    .map(|t| format!("- **{}**: {}", t.name, t.description))
                    .collect();
                format!("# Available Actions\n\n{}", actions.join("\n"))
            }
            "quick" | "start" | "quickstart" => {
                self.read_resource("cra://system/quick-start")
                    .unwrap_or_else(|| "Quick start resource not found.".to_string())
            }
            _ => {
                self.read_resource("cra://system/about")
                    .unwrap_or_else(|| "About resource not found.".to_string())
            }
        };

        ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: help_text,
            }],
            is_error: None,
        }
    }

    /// Handle cra_check tool
    fn handle_check(&self, args: Value) -> ToolResult {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("unknown");

        let parameters = args.get("parameters").cloned();

        // Check against loaded atlases
        let mut found = false;
        let mut allowed = true;
        let mut risk_tier = "unknown".to_string();
        let mut constraints: Vec<String> = vec![];

        for atlas in self.atlases.values() {
            for atlas_action in &atlas.actions {
                if atlas_action.action_id == action {
                    found = true;
                    risk_tier = atlas_action.risk_tier.clone();

                    // Check policies
                    for policy in &atlas.policies {
                        if policy.matches_action(action) {
                            use cra_core::PolicyType;
                            match &policy.policy_type {
                                PolicyType::Deny => {
                                    allowed = false;
                                    constraints.push(format!("Denied: {}",
                                        policy.reason.clone().unwrap_or_default()));
                                }
                                PolicyType::RateLimit => {
                                    constraints.push("Rate limit applies".to_string());
                                }
                                PolicyType::RequiresApproval => {
                                    constraints.push("Requires approval".to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                    break;
                }
            }
        }

        let response = if !found {
            serde_json::json!({
                "allowed": false,
                "reason": "Action not found. Use cra_list_actions to see available actions.",
                "suggestion": "Call cra_list_actions first"
            })
        } else if !allowed {
            serde_json::json!({
                "allowed": false,
                "reason": constraints.join("; "),
                "action": action,
                "risk_tier": risk_tier
            })
        } else {
            let mut result = serde_json::json!({
                "allowed": true,
                "action": action,
                "risk_tier": risk_tier
            });
            if !constraints.is_empty() {
                result["constraints"] = serde_json::json!(constraints);
            }
            result
        };

        ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&response).unwrap_or_default(),
            }],
            is_error: None,
        }
    }

    /// Handle cra_list_actions tool
    fn handle_list_actions(&self) -> ToolResult {
        let mut actions_md = String::from("# Available Actions\n\n");

        if self.atlases.is_empty() {
            actions_md.push_str("No atlases loaded. Only meta-tools available:\n\n");
            actions_md.push_str("- `cra_help` - Get help about CRA\n");
            actions_md.push_str("- `cra_check` - Check if an action is allowed\n");
            actions_md.push_str("- `cra_list_actions` - This command\n");
        } else {
            for atlas in self.atlases.values() {
                actions_md.push_str(&format!("## {} ({})\n\n", atlas.name, atlas.atlas_id));

                for action in &atlas.actions {
                    actions_md.push_str(&format!(
                        "### `{}`\n{}\n- Risk tier: {}\n\n",
                        action.action_id,
                        &action.description,
                        action.risk_tier
                    ));
                }
            }
        }

        ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: actions_md,
            }],
            is_error: None,
        }
    }

    /// Handle an atlas-defined action
    fn handle_atlas_action(&self, call: ToolCall) -> ToolResult {
        // First check if action is allowed
        let check_result = self.handle_check(serde_json::json!({
            "action": call.name,
            "parameters": call.arguments
        }));

        // Parse check result
        let check_text = &check_result.content[0].text;
        let check_json: Value = serde_json::from_str(check_text).unwrap_or_default();

        if check_json.get("allowed") != Some(&Value::Bool(true)) {
            return ToolResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: format!(
                        "Action `{}` was blocked by governance.\n\nReason: {}",
                        call.name,
                        check_json.get("reason").and_then(|r| r.as_str()).unwrap_or("Unknown")
                    ),
                }],
                is_error: Some(true),
            };
        }

        // Action would be executed here
        // For now, return a placeholder indicating the action was approved
        ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!(
                    "Action `{}` approved and queued for execution.\n\nParameters: {}",
                    call.name,
                    serde_json::to_string_pretty(&call.arguments).unwrap_or_default()
                ),
            }],
            is_error: None,
        }
    }
}

// Helper trait for policy matching
trait PolicyMatcher {
    fn matches_action(&self, action: &str) -> bool;
}

impl PolicyMatcher for cra_core::AtlasPolicy {
    fn matches_action(&self, action: &str) -> bool {
        for pattern in &self.actions {
            if pattern == action {
                return true;
            }
            // Simple glob matching
            if pattern.ends_with(".*") {
                let prefix = &pattern[..pattern.len() - 2];
                if action.starts_with(prefix) {
                    return true;
                }
            }
            if pattern == "*" {
                return true;
            }
        }
        false
    }
}
