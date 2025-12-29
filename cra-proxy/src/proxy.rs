//! Proxy route handlers

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, Method},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::Serialize;

use crate::policy::{PolicyChecker, PolicyDecision, RequestContext};
use crate::ProxyState;

/// Stats tracking
static REQUESTS_TOTAL: AtomicU64 = AtomicU64::new(0);
static REQUESTS_ALLOWED: AtomicU64 = AtomicU64::new(0);
static REQUESTS_DENIED: AtomicU64 = AtomicU64::new(0);
static REQUESTS_FAILED: AtomicU64 = AtomicU64::new(0);

/// Create the proxy router
pub fn create_router(state: Arc<ProxyState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/stats", get(stats))
        .route("/forward", post(forward_request))
        .with_state(state)
}

/// Health check
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "cra-proxy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

/// Stats endpoint
async fn stats() -> Json<StatsResponse> {
    Json(StatsResponse {
        requests_total: REQUESTS_TOTAL.load(Ordering::Relaxed),
        requests_allowed: REQUESTS_ALLOWED.load(Ordering::Relaxed),
        requests_denied: REQUESTS_DENIED.load(Ordering::Relaxed),
        requests_failed: REQUESTS_FAILED.load(Ordering::Relaxed),
    })
}

#[derive(Serialize)]
struct StatsResponse {
    requests_total: u64,
    requests_allowed: u64,
    requests_denied: u64,
    requests_failed: u64,
}

/// Forward request response
#[derive(Serialize)]
struct ForwardResponse {
    /// Whether the request was allowed
    allowed: bool,
    /// Status code from target (if forwarded)
    status_code: Option<u16>,
    /// Response body from target (if forwarded)
    response_body: Option<serde_json::Value>,
    /// Denial reason (if denied)
    denial_reason: Option<String>,
    /// CRA trace ID for audit
    trace_id: String,
}

/// Error response
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

/// Forward a request through the proxy
async fn forward_request(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    REQUESTS_TOTAL.fetch_add(1, Ordering::Relaxed);

    // Extract target URL from headers
    let target_url = match headers.get("x-target-url") {
        Some(v) => match v.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Invalid X-Target-URL header".to_string(),
                        code: "INVALID_HEADER".to_string(),
                    }),
                ).into_response();
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Missing X-Target-URL header".to_string(),
                    code: "MISSING_HEADER".to_string(),
                }),
            ).into_response();
        }
    };

    // Extract optional metadata
    let timer_id = headers
        .get("x-timer-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let session_id = headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let method = headers
        .get("x-target-method")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("POST")
        .to_string();

    // Parse body as JSON if possible
    let body_json: Option<serde_json::Value> = if body.is_empty() {
        None
    } else {
        serde_json::from_slice(&body).ok()
    };

    // Build request context
    let ctx = RequestContext {
        target_url: target_url.clone(),
        method: method.clone(),
        body: body_json.clone(),
        timer_id: timer_id.clone(),
        session_id: session_id.clone(),
    };

    // Check policy
    let checker = PolicyChecker::with_defaults();
    let decision = checker.check(&ctx);

    let trace_id = uuid::Uuid::new_v4().to_string();

    // Log to TRACE (via resolver)
    {
        let mut resolver = match state.resolver.lock() {
            Ok(r) => r,
            Err(_) => {
                REQUESTS_FAILED.fetch_add(1, Ordering::Relaxed);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to acquire resolver lock".to_string(),
                        code: "INTERNAL_ERROR".to_string(),
                    }),
                ).into_response();
            }
        };

        // Create or get session for tracing
        let session_id = session_id.unwrap_or_else(|| format!("proxy-{}", trace_id));
        let _ = resolver.create_session("cra-proxy", &format!("Forward to {}", target_url));
    }

    match decision {
        PolicyDecision::Allow => {
            REQUESTS_ALLOWED.fetch_add(1, Ordering::Relaxed);

            // Forward the request
            let http_method = match method.to_uppercase().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                "PUT" => Method::PUT,
                "DELETE" => Method::DELETE,
                "PATCH" => Method::PATCH,
                _ => Method::POST,
            };

            let mut request_builder = state.http_client.request(http_method, &target_url);

            // Forward content-type
            if let Some(ct) = headers.get("content-type") {
                if let Ok(ct_str) = ct.to_str() {
                    request_builder = request_builder.header("content-type", ct_str);
                }
            }

            // Add CRA headers
            request_builder = request_builder
                .header("x-cra-trace-id", &trace_id)
                .header("x-cra-proxy", "true");

            // Add body
            if !body.is_empty() {
                request_builder = request_builder.body(body.to_vec());
            }

            // Send request
            match request_builder.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let response_body: Option<serde_json::Value> = response.json().await.ok();

                    Json(ForwardResponse {
                        allowed: true,
                        status_code: Some(status),
                        response_body,
                        denial_reason: None,
                        trace_id,
                    }).into_response()
                }
                Err(e) => {
                    REQUESTS_FAILED.fetch_add(1, Ordering::Relaxed);

                    (
                        StatusCode::BAD_GATEWAY,
                        Json(ForwardResponse {
                            allowed: true,
                            status_code: None,
                            response_body: None,
                            denial_reason: Some(format!("Forward failed: {}", e)),
                            trace_id,
                        }),
                    ).into_response()
                }
            }
        }
        PolicyDecision::Deny { reason } => {
            REQUESTS_DENIED.fetch_add(1, Ordering::Relaxed);

            tracing::warn!(
                trace_id = %trace_id,
                target_url = %target_url,
                reason = %reason,
                "Request denied by policy"
            );

            (
                StatusCode::FORBIDDEN,
                Json(ForwardResponse {
                    allowed: false,
                    status_code: None,
                    response_body: None,
                    denial_reason: Some(reason),
                    trace_id,
                }),
            ).into_response()
        }
    }
}
