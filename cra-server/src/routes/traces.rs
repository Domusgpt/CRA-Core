//! TRACE query routes

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

use cra_core::{TRACEEvent, ChainVerification};
use crate::AppState;

/// Get trace events for a session
pub async fn get_trace(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<TRACEEvent>>, (StatusCode, String)> {
    let resolver = state.resolver.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
    })?;

    let events = resolver.get_trace(&session_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(events))
}

/// Chain verification response
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub is_valid: bool,
    pub event_count: usize,
    pub first_invalid_index: Option<usize>,
    pub last_valid_hash: Option<String>,
    pub error_message: Option<String>,
}

impl From<ChainVerification> for VerifyResponse {
    fn from(v: ChainVerification) -> Self {
        Self {
            is_valid: v.is_valid,
            event_count: v.event_count,
            first_invalid_index: v.first_invalid_index,
            last_valid_hash: v.last_valid_hash,
            error_message: v.error_message,
        }
    }
}

/// Verify hash chain for a session
pub async fn verify_chain(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    let resolver = state.resolver.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock error: {}", e))
    })?;

    let verification = resolver.verify_chain(&session_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(VerifyResponse::from(verification)))
}
