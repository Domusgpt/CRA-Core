//! Configuration for CRA Wrapper

use serde::{Deserialize, Serialize};

/// Main wrapper configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapperConfig {
    /// Version of the wrapper
    pub version: String,

    /// Whether checkpoints are enabled
    #[serde(default = "default_true")]
    pub checkpoints_enabled: bool,

    /// Queue configuration
    #[serde(default)]
    pub queue: QueueConfig,

    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,

    /// Transport configuration
    #[serde(default)]
    pub transport: TransportConfig,

    /// Hook configuration
    #[serde(default)]
    pub hooks: HookConfig,
}

fn default_true() -> bool { true }

impl Default for WrapperConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            checkpoints_enabled: true,
            queue: QueueConfig::default(),
            cache: CacheConfig::default(),
            transport: TransportConfig::default(),
            hooks: HookConfig::default(),
        }
    }
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum queue size before auto-flush
    #[serde(default = "default_max_size")]
    pub max_size: usize,

    /// Flush interval in milliseconds
    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,

    /// Event types that require synchronous flush
    #[serde(default)]
    pub sync_events: Vec<String>,
}

fn default_max_size() -> usize { 100 }
fn default_flush_interval() -> u64 { 5000 }

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size: 100,
            flush_interval_ms: 5000,
            sync_events: vec![
                "policy_check".to_string(),
                "session_end".to_string(),
            ],
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Default TTL in seconds
    #[serde(default = "default_ttl")]
    pub default_ttl_seconds: u64,

    /// Maximum cache entries
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,

    /// Cache backend type
    #[serde(default)]
    pub backend: CacheBackendType,
}

fn default_ttl() -> u64 { 300 }
fn default_max_entries() -> usize { 1000 }

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl_seconds: 300,
            max_entries: 1000,
            backend: CacheBackendType::Memory,
        }
    }
}

/// Cache backend type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheBackendType {
    #[default]
    Memory,
    File,
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type
    #[serde(default)]
    pub transport_type: TransportType,

    /// MCP server command (for MCP transport)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_command: Option<String>,

    /// REST API base URL (for REST transport)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,

    /// Connection timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 { 30000 }

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::Direct,
            mcp_command: None,
            rest_url: None,
            timeout_ms: 30000,
        }
    }
}

/// Transport type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    /// Direct library call (same process)
    #[default]
    Direct,
    /// MCP protocol
    Mcp,
    /// REST API
    Rest,
    /// WebSocket
    WebSocket,
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Whether input hooks are enabled
    #[serde(default = "default_true")]
    pub intercept_input: bool,

    /// Whether output hooks are enabled
    #[serde(default = "default_true")]
    pub intercept_output: bool,

    /// Whether action hooks are enabled
    #[serde(default = "default_true")]
    pub intercept_actions: bool,

    /// Keywords to trigger context injection
    #[serde(default)]
    pub trigger_keywords: Vec<String>,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            intercept_input: true,
            intercept_output: true,
            intercept_actions: true,
            trigger_keywords: Vec::new(),
        }
    }
}
