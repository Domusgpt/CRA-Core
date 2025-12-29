//! CRA MCP Server - Self-Documenting Governance
//!
//! This MCP server automatically explains itself to any agent that connects.
//! No external documentation needed - the governance context IS the documentation.
//!
//! ## How It Works
//!
//! 1. Agent connects to CRA MCP server
//! 2. Server automatically provides:
//!    - `cra://system/about` - What is CRA and how to use it
//!    - `cra://system/rules` - Current governance rules
//!    - `cra://system/examples` - Example interactions
//! 3. Agent reads these resources, understands the system
//! 4. Agent uses governed tools with full context
//!
//! ## Self-Documenting Philosophy
//!
//! The best documentation is documentation that's impossible to miss.
//! By embedding governance context directly in the MCP resources,
//! every agent automatically learns how to use the system correctly.

pub mod context;
pub mod tools;
pub mod server;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cra_core::{AtlasManifest, Resolver};

/// The core CRA MCP server
pub struct CRAMCPServer {
    resolver: Arc<Mutex<Resolver>>,
    /// System context blocks (auto-injected to all sessions)
    system_context: Vec<SystemContext>,
    /// Loaded atlases
    atlases: HashMap<String, AtlasManifest>,
}

/// System-level context that's always available
#[derive(Debug, Clone)]
pub struct SystemContext {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub content: String,
    pub mime_type: String,
}

impl CRAMCPServer {
    /// Create a new CRA MCP server with self-documenting context
    pub fn new() -> Self {
        let resolver = Resolver::new();

        Self {
            resolver: Arc::new(Mutex::new(resolver)),
            system_context: Self::build_system_context(),
            atlases: HashMap::new(),
        }
    }

    /// Build the auto-injected system context
    fn build_system_context() -> Vec<SystemContext> {
        vec![
            SystemContext {
                id: "about".to_string(),
                uri: "cra://system/about".to_string(),
                name: "About CRA".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../context/about.md").to_string(),
            },
            SystemContext {
                id: "rules".to_string(),
                uri: "cra://system/rules".to_string(),
                name: "Governance Rules".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../context/rules.md").to_string(),
            },
            SystemContext {
                id: "examples".to_string(),
                uri: "cra://system/examples".to_string(),
                name: "Usage Examples".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../context/examples.md").to_string(),
            },
            SystemContext {
                id: "quick-start".to_string(),
                uri: "cra://system/quick-start".to_string(),
                name: "Quick Start".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../context/quick-start.md").to_string(),
            },
        ]
    }

    /// Load an Atlas into the server
    pub fn load_atlas(&mut self, atlas: AtlasManifest) -> Result<(), String> {
        let mut resolver = self.resolver.lock().map_err(|e| e.to_string())?;
        resolver.load_atlas(atlas.clone()).map_err(|e| e.to_string())?;
        self.atlases.insert(atlas.atlas_id.clone(), atlas);
        Ok(())
    }

    /// Get all MCP resources (system context + atlas context)
    pub fn get_resources(&self) -> Vec<MCPResource> {
        let mut resources = Vec::new();

        // System context is always first
        for ctx in &self.system_context {
            resources.push(MCPResource {
                uri: ctx.uri.clone(),
                name: ctx.name.clone(),
                description: Some(format!("CRA System: {}", ctx.name)),
                mime_type: Some(ctx.mime_type.clone()),
            });
        }

        // Add atlas-specific resources
        // (would come from loaded atlases)

        resources
    }

    /// Get all MCP tools (from loaded atlases)
    pub fn get_tools(&self) -> Vec<MCPTool> {
        let mut tools = Vec::new();

        // Always include meta-tools for self-documentation
        tools.push(MCPTool {
            name: "cra_help".to_string(),
            description: "Get help about CRA governance. Call this if you're unsure about anything.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "What do you need help with? (rules, actions, policies, examples)"
                    }
                }
            }),
        });

        tools.push(MCPTool {
            name: "cra_check".to_string(),
            description: "Check if an action is allowed BEFORE attempting it. Always call this first.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "The action ID you want to perform"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "The parameters you plan to use"
                    }
                }
            }),
        });

        tools.push(MCPTool {
            name: "cra_list_actions".to_string(),
            description: "List all available actions and their governance status.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        });

        // Add tools from atlases
        for atlas in self.atlases.values() {
            for action in &atlas.actions {
                tools.push(MCPTool {
                    name: action.action_id.clone(),
                    description: format!(
                        "{}\n\n⚠️ Risk tier: {}. Call cra_check first.",
                        &action.description,
                        action.risk_tier
                    ),
                    input_schema: action.parameters_schema.clone(),
                });
            }
        }

        tools
    }

    /// Read a resource by URI
    pub fn read_resource(&self, uri: &str) -> Option<String> {
        // Check system context first
        for ctx in &self.system_context {
            if ctx.uri == uri {
                return Some(ctx.content.clone());
            }
        }

        // Could also check atlas context blocks here
        None
    }
}

impl Default for CRAMCPServer {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP Resource descriptor
#[derive(Debug, Clone, serde::Serialize)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Tool descriptor
#[derive(Debug, Clone, serde::Serialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP Server Info (returned on initialize)
#[derive(Debug, Clone, serde::Serialize)]
pub struct MCPServerInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Instructions shown to the agent on connect
    pub instructions: String,
}

impl CRAMCPServer {
    /// Get server info for MCP initialize response
    pub fn server_info(&self) -> MCPServerInfo {
        MCPServerInfo {
            name: "cra-governance".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "CRA Governance Server - Policy enforcement for AI agents".to_string(),
            instructions: r#"
# CRA Governance Server

You are connected to a governance-enabled environment. This means:

1. **Check before acting**: Use `cra_check` before attempting any action
2. **Read the rules**: The `cra://system/rules` resource contains current policies
3. **Ask for help**: Use `cra_help` if you're unsure about anything

## Quick Start

1. Call `cra_list_actions` to see available actions
2. Call `cra_check` with your intended action
3. If allowed, proceed. If denied, explain to the user why.

## Important

- All actions are logged to an immutable audit trail
- Some actions require explicit user approval
- Rate limits may apply to certain actions

When in doubt, check first. It's always better to ask than to be blocked.
"#.to_string(),
        }
    }
}
