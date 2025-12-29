//! CRA Proxy Binary
//!
//! Layer 3 enforcement - hard blocks at the network level.
//!
//! ## Usage
//!
//! ```bash
//! # Start with defaults
//! cra-proxy
//!
//! # Custom port
//! CRA_PROXY_PORT=9000 cra-proxy
//!
//! # Fail-open mode (allow on error)
//! CRA_PROXY_ALLOW_ON_ERROR=true cra-proxy
//! ```

use cra_core::Resolver;
use cra_proxy::{CRAProxy, ProxyConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cra_proxy=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Configuration from environment
    let port: u16 = std::env::var("CRA_PROXY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8421);

    let allow_on_error: bool = std::env::var("CRA_PROXY_ALLOW_ON_ERROR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(false);

    let timeout_ms: u64 = std::env::var("CRA_PROXY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30000);

    let config = ProxyConfig {
        port,
        allow_on_error,
        timeout_ms,
    };

    tracing::info!("Starting CRA Proxy v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Mode: {}", if allow_on_error { "fail-open" } else { "fail-closed" });

    let resolver = Resolver::new();
    let proxy = CRAProxy::new(resolver, config);
    proxy.run().await?;

    Ok(())
}
