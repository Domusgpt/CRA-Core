//! Example: HTTP Server Wrapper for CRA Core
//!
//! This demonstrates how to wrap the Rust core with an HTTP API.
//! It's intentionally kept separate from cra-core to maintain the
//! embedded-first, pure-logic architecture.
//!
//! ## Usage
//!
//! Add to Cargo.toml:
//! ```toml
//! [dependencies]
//! cra-core = { path = "../cra-core" }
//! axum = "0.7"
//! tokio = { version = "1", features = ["full"] }
//! serde_json = "1"
//! ```
//!
//! Run:
//! ```bash
//! cargo run --example http_server
//! ```
//!
//! Test:
//! ```bash
//! # Create session
//! curl -X POST http://localhost:8420/v1/sessions \
//!   -H "Content-Type: application/json" \
//!   -d '{"agent_id": "my-agent", "goal": "Help with support"}'
//!
//! # Resolve
//! curl -X POST http://localhost:8420/v1/resolve \
//!   -H "Content-Type: application/json" \
//!   -d '{"session_id": "...", "agent_id": "my-agent", "goal": "Help"}'
//! ```

use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// In real usage, import from cra_core
// use cra_core::{AtlasManifest, CARPRequest, Resolver};

// Placeholder types for this example
#[derive(Clone)]
struct Resolver {
    // In real implementation, this is cra_core::Resolver
}

impl Resolver {
    fn new() -> Self { Self {} }

    fn create_session(&mut self, agent_id: &str, goal: &str) -> Result<String, String> {
        Ok(format!("session-{}", uuid::Uuid::new_v4()))
    }

    fn resolve(&mut self, request: &ResolveRequest) -> Result<Value, String> {
        Ok(json!({
            "carp_version": "1.0",
            "decision": "allow",
            "allowed_actions": [
                {"action_id": "demo.action", "name": "Demo Action"}
            ]
        }))
    }

    fn get_trace(&self, session_id: &str) -> Result<Vec<Value>, String> {
        Ok(vec![
            json!({"event_type": "session.started", "session_id": session_id})
        ])
    }
}

// Shared state
type AppState = Arc<Mutex<Resolver>>;

// Request/Response types
#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    agent_id: String,
    goal: String,
}

#[derive(Debug, Serialize)]
struct CreateSessionResponse {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct ResolveRequest {
    session_id: String,
    agent_id: String,
    goal: String,
}

// Handlers
async fn health() -> &'static str {
    "OK"
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let mut resolver = state.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let session_id = resolver.create_session(&req.agent_id, &req.goal)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(CreateSessionResponse { session_id }))
}

async fn resolve(
    State(state): State<AppState>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut resolver = state.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let resolution = resolver.resolve(&req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(resolution))
}

async fn get_trace(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let resolver = state.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let trace = resolver.get_trace(&session_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;

    Ok(Json(trace))
}

#[tokio::main]
async fn main() {
    // Initialize resolver with loaded atlases
    let resolver = Resolver::new();
    // resolver.load_atlas(atlas).unwrap();  // In real usage

    let state: AppState = Arc::new(Mutex::new(resolver));

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/sessions", post(create_session))
        .route("/v1/resolve", post(resolve))
        .route("/v1/traces/:session_id", get(get_trace))
        .with_state(state);

    // Run server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8420")
        .await
        .unwrap();

    println!("CRA Server listening on http://127.0.0.1:8420");
    println!("Endpoints:");
    println!("  GET  /health");
    println!("  POST /v1/sessions");
    println!("  POST /v1/resolve");
    println!("  GET  /v1/traces/:session_id");

    axum::serve(listener, app).await.unwrap();
}

// =============================================================================
// THE POINT OF THIS EXAMPLE
// =============================================================================
//
// This entire HTTP server is ~120 lines of code.
//
// It wraps cra-core (the pure Rust library) with HTTP endpoints.
// The core logic (CARP, TRACE, Atlas, Policy) remains in cra-core.
//
// Why keep them separate?
//
// 1. cra-core can be embedded directly (0.134ms latency)
// 2. HTTP wrapper adds ~1-5ms network overhead
// 3. Users choose: embedded (fast) or server (compatible)
// 4. Different apps need different servers (axum, actix, warp, etc.)
// 5. Storage backends vary (SQLite, PostgreSQL, in-memory)
//
// The Python "dual mode" architecture bundles everything together.
// Our approach: compose small pieces.
//
// cra-core (embedded) + axum (HTTP) + sqlx (storage) = server mode
// cra-core (embedded) alone = embedded mode
//
// Both modes use the SAME cra-core. No code duplication.
// =============================================================================
