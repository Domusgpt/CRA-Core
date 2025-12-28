//! CRA Server - HTTP wrapper for CRA Core
//!
//! This crate provides an HTTP API around the CRA core library,
//! enabling the dual-mode architecture:
//!
//! - **Mode 1 (Embedded)**: Use `cra-core` directly (~0.001ms)
//! - **Mode 2 (HTTP)**: Use this server via REST API (~5ms)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    CRAServer                         │
//! │  ┌─────────────────────────────────────────────┐    │
//! │  │              cra-core::Resolver              │    │
//! │  │         (all logic lives here)              │    │
//! │  └─────────────────────────────────────────────┘    │
//! │                        │                             │
//! │  ┌─────────────────────┼─────────────────────┐      │
//! │  │                     │                     │      │
//! │  ▼                     ▼                     ▼      │
//! │ /v1/sessions    /v1/resolve    /v1/traces/:id      │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! The server is a thin wrapper - all CRA logic remains in `cra-core`.

pub mod routes;
mod config;

pub use config::{ServerConfig, ServerConfigBuilder};

use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use axum::Router;
use cra_core::Resolver;

/// Shared application state
pub struct AppState {
    /// The CRA resolver (wraps cra-core)
    pub resolver: Mutex<Resolver>,
}

impl AppState {
    /// Create new app state with the given resolver
    pub fn new(resolver: Resolver) -> Self {
        Self {
            resolver: Mutex::new(resolver),
        }
    }
}

/// CRA HTTP Server
///
/// Wraps `cra-core::Resolver` with HTTP endpoints.
///
/// # Example
///
/// ```rust,ignore
/// use cra_server::{CRAServer, ServerConfig};
/// use cra_core::Resolver;
///
/// #[tokio::main]
/// async fn main() {
///     let resolver = Resolver::new();
///     let config = ServerConfig::builder()
///         .port(8420)
///         .build();
///
///     let server = CRAServer::new(resolver, config);
///     server.run().await.unwrap();
/// }
/// ```
pub struct CRAServer {
    state: Arc<AppState>,
    config: ServerConfig,
}

impl CRAServer {
    /// Create a new CRA server wrapping the given resolver
    pub fn new(resolver: Resolver, config: ServerConfig) -> Self {
        Self {
            state: Arc::new(AppState::new(resolver)),
            config,
        }
    }

    /// Build the Axum router with all routes
    pub fn router(&self) -> Router {
        routes::create_router(Arc::clone(&self.state))
    }

    /// Get the socket address for the server
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.config.port))
    }

    /// Run the server
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.router();
        let addr = self.addr();

        tracing::info!("CRA Server listening on http://{}", addr);
        tracing::info!("Endpoints:");
        tracing::info!("  GET  /health");
        tracing::info!("  POST /v1/sessions");
        tracing::info!("  POST /v1/resolve");
        tracing::info!("  GET  /v1/traces/:session_id");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
