//! HTTP route handlers

mod sessions;
mod resolve;
mod traces;

use std::sync::Arc;
use axum::{routing::{get, post}, Router, Json};
use serde::Serialize;

use crate::AppState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create the router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/sessions", post(sessions::create_session))
        .route("/v1/sessions/:session_id", get(sessions::get_session))
        .route("/v1/sessions/:session_id/end", post(sessions::end_session))
        .route("/v1/resolve", post(resolve::resolve))
        .route("/v1/traces/:session_id", get(traces::get_trace))
        .route("/v1/traces/:session_id/verify", get(traces::verify_chain))
        .with_state(state)
}
