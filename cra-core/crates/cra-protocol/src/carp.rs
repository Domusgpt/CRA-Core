use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPRequest {
    pub carp_version: String,
    pub request_id: String,
    pub timestamp: String,
    pub operation: Operation,
    pub requester: Requester,
    pub task: Task,
    #[serde(default)]
    pub atlas_ids: Vec<String>,
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Resolve,
    Execute,
    Validate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requester {
    pub agent_id: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_tier: Option<RiskTier>,
    #[serde(default)]
    pub context_hints: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPResolution {
    pub resolution_id: String,
    pub request_id: String,
    pub timestamp: String,
    pub decision: Decision,
    pub context_blocks: Vec<ContextBlock>,
    pub allowed_actions: Vec<ActionPermission>,
    pub denied_actions: Vec<DeniedAction>,
    pub constraints: Vec<Constraint>,
    pub ttl_seconds: u32,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny { reason: String },
    RequiresApproval { approver: String, timeout_seconds: u32 },
    Partial { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    pub id: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPermission {
    pub action_type: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeniedAction {
    pub action_type: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub description: String,
}
