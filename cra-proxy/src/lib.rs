//! CRA Webhook Proxy - Layer 3 Hard Enforcement
//!
//! This proxy sits between services (like MINOOTS) and external endpoints.
//! All outbound HTTP requests go through CRA for policy enforcement.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐      ┌─────────────────────┐      ┌─────────────┐
//! │   MINOOTS   │──────│   CRA Proxy         │──────│  External   │
//! │   Timer     │ HTTP │                     │ HTTP │  Webhooks   │
//! └─────────────┘      │  1. Extract target  │      └─────────────┘
//!                      │  2. Check policy    │
//!                      │  3. Forward or 403  │
//!                      │  4. Log to TRACE    │
//!                      └─────────────────────┘
//! ```
//!
//! ## Usage
//!
//! Instead of calling external webhooks directly:
//! ```text
//! POST http://cra-proxy:8421/forward
//! X-Target-URL: https://api.example.com/webhook
//! X-Timer-ID: timer-123
//! X-Session-ID: session-456
//! Content-Type: application/json
//!
//! {"event": "timer.fired", ...}
//! ```

pub mod policy;
pub mod proxy;

use std::sync::Arc;
use std::net::SocketAddr;

use axum::Router;
use cra_core::Resolver;

/// Proxy configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Port to listen on
    pub port: u16,
    /// Allow requests when policy check fails (fail-open vs fail-closed)
    pub allow_on_error: bool,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: 8421,
            allow_on_error: false, // Fail closed by default
            timeout_ms: 30000,
        }
    }
}

/// Shared proxy state
pub struct ProxyState {
    pub resolver: std::sync::Mutex<Resolver>,
    pub http_client: reqwest::Client,
    pub config: ProxyConfig,
}

impl ProxyState {
    pub fn new(resolver: Resolver, config: ProxyConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            resolver: std::sync::Mutex::new(resolver),
            http_client,
            config,
        }
    }
}

/// CRA Webhook Proxy
pub struct CRAProxy {
    state: Arc<ProxyState>,
}

impl CRAProxy {
    /// Create a new proxy
    pub fn new(resolver: Resolver, config: ProxyConfig) -> Self {
        Self {
            state: Arc::new(ProxyState::new(resolver, config)),
        }
    }

    /// Build the router
    pub fn router(&self) -> Router {
        proxy::create_router(Arc::clone(&self.state))
    }

    /// Get the socket address
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.state.config.port))
    }

    /// Run the proxy
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.router();
        let addr = self.addr();

        tracing::info!("CRA Proxy listening on http://{}", addr);
        tracing::info!("Endpoints:");
        tracing::info!("  POST /forward     - Forward webhook with policy check");
        tracing::info!("  GET  /health      - Health check");
        tracing::info!("  GET  /stats       - Proxy statistics");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
