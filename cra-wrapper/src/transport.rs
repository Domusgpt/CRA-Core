//! Transport backends for CRA communication

use async_trait::async_trait;

use crate::error::WrapperResult;

/// Transport backend interface
#[async_trait]
pub trait TransportBackend: Send + Sync {
    /// Backend name
    fn name(&self) -> &str;

    /// Connect to CRA
    async fn connect(&mut self) -> WrapperResult<()>;

    /// Disconnect from CRA
    async fn disconnect(&mut self) -> WrapperResult<()>;

    /// Send a request and get response
    async fn request(&self, method: &str, params: serde_json::Value) -> WrapperResult<serde_json::Value>;
}

/// MCP transport backend
pub struct McpTransport {
    /// Server command
    command: String,

    /// Server arguments
    args: Vec<String>,

    /// Whether connected
    connected: bool,
}

impl McpTransport {
    pub fn new(command: &str, args: Vec<String>) -> Self {
        Self {
            command: command.to_string(),
            args,
            connected: false,
        }
    }
}

#[async_trait]
impl TransportBackend for McpTransport {
    fn name(&self) -> &str {
        "mcp"
    }

    async fn connect(&mut self) -> WrapperResult<()> {
        // TODO: Start MCP server process and connect
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> WrapperResult<()> {
        self.connected = false;
        Ok(())
    }

    async fn request(&self, method: &str, params: serde_json::Value) -> WrapperResult<serde_json::Value> {
        // TODO: Send JSON-RPC request to MCP server
        Ok(serde_json::json!({}))
    }
}

/// REST API transport backend
pub struct RestTransport {
    /// Base URL
    base_url: String,
    // HTTP client would go here in production
}

impl RestTransport {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl TransportBackend for RestTransport {
    fn name(&self) -> &str {
        "rest"
    }

    async fn connect(&mut self) -> WrapperResult<()> {
        Ok(())
    }

    async fn disconnect(&mut self) -> WrapperResult<()> {
        Ok(())
    }

    async fn request(&self, method: &str, params: serde_json::Value) -> WrapperResult<serde_json::Value> {
        // TODO: Send HTTP request
        Ok(serde_json::json!({}))
    }
}

/// Direct transport (same process)
pub struct DirectTransport;

#[async_trait]
impl TransportBackend for DirectTransport {
    fn name(&self) -> &str {
        "direct"
    }

    async fn connect(&mut self) -> WrapperResult<()> {
        Ok(())
    }

    async fn disconnect(&mut self) -> WrapperResult<()> {
        Ok(())
    }

    async fn request(&self, method: &str, params: serde_json::Value) -> WrapperResult<serde_json::Value> {
        // Direct mode - handle inline
        Ok(serde_json::json!({}))
    }
}
