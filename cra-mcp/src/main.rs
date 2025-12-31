//! CRA MCP Server
//!
//! This is the Model Context Protocol server that exposes CRA governance
//! to agents like Claude, GPT, and other MCP-compatible systems.
//!
//! ## Usage
//!
//! ```bash
//! # Run with atlases directory
//! cra-mcp-server --atlases ./atlases
//!
//! # Run without atlases (agents can load them later)
//! cra-mcp-server
//! ```
//!
//! ## Configuration
//!
//! For Claude Code, add to `~/.claude/claude_code_config.json`:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "cra": {
//!       "command": "cra-mcp-server",
//!       "args": ["--atlases", "/path/to/atlases"]
//!     }
//!   }
//! }
//! ```

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cra_mcp::McpServer;

/// CRA MCP Server - Governance layer for AI agents
#[derive(Parser, Debug)]
#[command(name = "cra-mcp-server")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory containing atlas JSON files
    #[arg(short, long)]
    atlases: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| format!("cra_mcp={}", log_level).into()))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Starting CRA MCP Server v{}", env!("CARGO_PKG_VERSION"));

    // Build server
    let mut builder = McpServer::builder();

    if let Some(atlases_dir) = &args.atlases {
        tracing::info!("Loading atlases from: {}", atlases_dir);
        builder = builder.with_atlases_dir(atlases_dir);
    }

    let server = builder.build().await?;

    // Run on stdio
    server.run_stdio().await?;

    Ok(())
}
