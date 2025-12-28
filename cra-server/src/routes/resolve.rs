//! CARP resolution route

use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use cra_core::{CARPRequest, CARPResolution, Decision};
use crate::AppState;

/// Resolution request (HTTP wrapper)
#[derive(Debug, Deserialize)]
pub struct ResolveRequest {
    pub session_id: String,
    pub agent_id: String,
    pub goal: String,
    #[serde(default)]
    pub context: Value,
}

/// Allowed action in response
#[derive(Debug, Serialize)]
pub struct AllowedActionResponse {
    pub action_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters_schema: Value,
    pub risk_tier: String,
}

/// Denied action in response
#[derive(Debug, Serialize)]
pub struct DeniedActionResponse {
    pub action_id: String,
    pub reason: String,
    pub policy_id: String,
}

/// Resolution response
#[derive(Debug, Serialize)]
pub struct ResolveResponse {
    pub decision: String,
    pub allowed_actions: Vec<AllowedActionResponse>,
    pub denied_actions: Vec<DeniedActionResponse>,
    pub ttl_seconds: u64,
}

/// Resolve a CARP request
pub async fn resolve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<ResolveResponse>, (StatusCode, String)> {
    let mut resolver = state.resolver.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
    })?;

    let carp_request = CARPRequest::new(
        req.session_id,
        req.agent_id,
        req.goal,
    );

    let resolution: CARPResolution = resolver.resolve(&carp_request)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let response = ResolveResponse {
        decision: match resolution.decision {
            Decision::Allow => "allow".to_string(),
            Decision::AllowWithConstraints => "allow_with_constraints".to_string(),
            Decision::Partial => "partial".to_string(),
            Decision::Deny => "deny".to_string(),
            Decision::RequiresApproval => "requires_approval".to_string(),
        },
        allowed_actions: resolution.allowed_actions.into_iter().map(|a| {
            AllowedActionResponse {
                action_id: a.action_id,
                name: a.name,
                description: a.description,
                parameters_schema: a.parameters_schema,
                risk_tier: a.risk_tier,
            }
        }).collect(),
        denied_actions: resolution.denied_actions.into_iter().map(|d| {
            DeniedActionResponse {
                action_id: d.action_id,
                reason: d.reason,
                policy_id: d.policy_id,
            }
        }).collect(),
        ttl_seconds: resolution.ttl_seconds,
    };

    Ok(Json(response))
}
