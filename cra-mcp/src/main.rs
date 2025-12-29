//! CRA MCP Server Binary
//!
//! Self-documenting governance server for AI agents.
//!
//! ## Usage
//!
//! ```bash
//! # Run as MCP server (stdio)
//! cra-mcp
//!
//! # With atlas directory
//! CRA_ATLAS_DIR=./atlases cra-mcp
//! ```

use std::path::PathBuf;

use cra_core::AtlasLoader;
use cra_mcp::CRAMCPServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (to stderr so it doesn't interfere with stdio MCP)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cra_mcp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Starting CRA MCP Server v{}", env!("CARGO_PKG_VERSION"));

    // Create server
    let mut server = CRAMCPServer::new();

    // Load atlases if directory specified
    if let Some(dir) = std::env::var("CRA_ATLAS_DIR").ok().map(PathBuf::from) {
        if dir.exists() {
            let mut loader = AtlasLoader::new().with_search_path(dir.clone());
            match loader.load_discovered() {
                Ok(atlas_ids) => {
                    for atlas_id in &atlas_ids {
                        if let Some(loaded) = loader.get(atlas_id) {
                            tracing::info!("Loading atlas: {}", loaded.manifest.atlas_id);
                            if let Err(e) = server.load_atlas(loaded.manifest.clone()) {
                                tracing::warn!("Failed to load atlas {}: {}", atlas_id, e);
                            }
                        }
                    }
                    tracing::info!("Loaded {} atlas(es)", atlas_ids.len());
                }
                Err(e) => {
                    tracing::warn!("Failed to discover atlases: {}", e);
                }
            }
        }
    }

    // Run stdio server
    tracing::info!("MCP server ready, listening on stdio");
    server.run_stdio()?;

    Ok(())
}
