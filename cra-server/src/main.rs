//! CRA Server Binary
//!
//! HTTP server providing REST API access to CRA Core functionality.
//!
//! ## Usage
//!
//! ```bash
//! # Start with defaults (port 8420)
//! cra-server
//!
//! # Custom port
//! CRA_PORT=3000 cra-server
//!
//! # With atlas directory
//! CRA_ATLAS_DIR=./atlases cra-server
//! ```

use std::path::PathBuf;

use cra_core::{AtlasLoader, Resolver};
use cra_server::{CRAServer, ServerConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cra_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get configuration from environment
    let port: u16 = std::env::var("CRA_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8420);

    let atlas_dir = std::env::var("CRA_ATLAS_DIR").ok().map(PathBuf::from);

    // Create resolver
    let mut resolver = Resolver::new();

    // Load atlases if directory specified
    if let Some(dir) = atlas_dir {
        if dir.exists() {
            let mut loader = AtlasLoader::new().with_search_path(dir.clone());
            match loader.load_discovered() {
                Ok(atlas_ids) => {
                    for atlas_id in &atlas_ids {
                        if let Some(loaded) = loader.get(atlas_id) {
                            tracing::info!(
                                "Loading atlas: {} v{}",
                                loaded.manifest.atlas_id,
                                loaded.manifest.version
                            );
                            resolver.load_atlas(loaded.manifest.clone())?;
                        }
                    }
                    tracing::info!("Loaded {} atlas(es)", atlas_ids.len());
                }
                Err(e) => {
                    tracing::warn!("Failed to load atlases from {:?}: {}", dir, e);
                }
            }
        } else {
            tracing::warn!("Atlas directory {:?} does not exist", dir);
        }
    }

    // Create and run server
    let config = ServerConfig::builder().port(port).build();

    tracing::info!("Starting CRA Server v{}", env!("CARGO_PKG_VERSION"));

    let server = CRAServer::new(resolver, config);
    server.run().await?;

    Ok(())
}
