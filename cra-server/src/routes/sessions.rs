//! Session management routes

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Create session request
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: String,
    pub goal: String,
}

/// Session response
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub agent_id: String,
    pub status: String,
}

/// Create a new session
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    let mut resolver = state.resolver.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
    })?;

    let session_id = resolver.create_session(&req.agent_id, &req.goal)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(SessionResponse {
        session_id,
        agent_id: req.agent_id,
        status: "active".to_string(),
    }))
}

/// Get session info
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    let resolver = state.resolver.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
    })?;

    // Check if session exists by trying to get trace
    let _trace = resolver.get_trace(&session_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(SessionResponse {
        session_id,
        agent_id: "unknown".to_string(), // Would need to store this
        status: "active".to_string(),
    }))
}

/// End a session
pub async fn end_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    let mut resolver = state.resolver.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
    })?;

    resolver.end_session(&session_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(SessionResponse {
        session_id,
        agent_id: "unknown".to_string(),
        status: "ended".to_string(),
    }))
}
