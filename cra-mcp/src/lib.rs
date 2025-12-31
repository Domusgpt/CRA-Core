//! CRA MCP Server Library
//!
//! This crate implements the Model Context Protocol (MCP) server for CRA,
//! enabling agents to interact with CRA governance through standardized tools.
//!
//! ## Architecture
//!
//! ```text
//! Agent (Claude, GPT, etc.)
//!        │
//!        ▼
//! ┌─────────────────┐
//! │   MCP Server    │ ◄── This crate
//! │                 │
//! │  ┌───────────┐  │
//! │  │   Tools   │  │ - cra_start_session
//! │  │           │  │ - cra_request_context
//! │  │           │  │ - cra_report_action
//! │  │           │  │ - cra_feedback
//! │  │           │  │ - cra_end_session
//! │  └───────────┘  │
//! │                 │
//! │  ┌───────────┐  │
//! │  │ Resources │  │ - cra://session/current
//! │  │           │  │ - cra://trace/{id}
//! │  │           │  │ - cra://atlas/{id}
//! │  └───────────┘  │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │    cra-core     │
//! │                 │
//! │  CARP │ TRACE   │
//! │  Atlas│ Storage │
//! └─────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cra_mcp::McpServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::new()
//!         .with_atlases_dir("./atlases")
//!         .build()
//!         .await
//!         .unwrap();
//!
//!     server.run_stdio().await.unwrap();
//! }
//! ```

pub mod tools;
pub mod resources;
pub mod server;
pub mod error;
pub mod session;
pub mod bootstrap;

pub use server::McpServer;
pub use error::{McpError, McpResult};
pub use session::SessionManager;
pub use bootstrap::BootstrapProtocol;

/// Server metadata for MCP protocol
pub const SERVER_NAME: &str = "cra-governance";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVER_DESCRIPTION: &str = "Context Registry for Agents - Governance and audit layer";
