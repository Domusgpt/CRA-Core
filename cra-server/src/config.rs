//! Server configuration

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,
    /// Enable CORS
    pub cors_enabled: bool,
}

impl ServerConfig {
    /// Create a new configuration builder
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8420,
            cors_enabled: true,
        }
    }
}

/// Builder for ServerConfig
#[derive(Debug, Default)]
pub struct ServerConfigBuilder {
    port: Option<u16>,
    cors_enabled: Option<bool>,
}

impl ServerConfigBuilder {
    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Enable or disable CORS
    pub fn cors(mut self, enabled: bool) -> Self {
        self.cors_enabled = Some(enabled);
        self
    }

    /// Build the configuration
    pub fn build(self) -> ServerConfig {
        ServerConfig {
            port: self.port.unwrap_or(8420),
            cors_enabled: self.cors_enabled.unwrap_or(true),
        }
    }
}
