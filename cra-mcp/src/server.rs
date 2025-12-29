//! MCP Server protocol implementation
//!
//! This module handles the MCP JSON-RPC protocol over stdio.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};

use crate::{CRAMCPServer, MCPResource, MCPTool};
use crate::tools::{ToolCall, ToolResult};

/// MCP JSON-RPC request
#[derive(Debug, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// MCP JSON-RPC response
#[derive(Debug, Serialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Debug, Serialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
}

impl CRAMCPServer {
    /// Run the MCP server over stdio
    pub fn run_stdio(&self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request: MCPRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let response = MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(MCPError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                        }),
                    };
                    writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                    stdout.flush()?;
                    continue;
                }
            };

            let response = self.handle_request(request);
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// Handle an MCP request
    pub fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(request.params),
            "resources/list" => self.handle_resources_list(),
            "resources/read" => self.handle_resources_read(request.params),
            "prompts/list" => self.handle_prompts_list(),
            _ => {
                return MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(MCPError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                    }),
                };
            }
        };

        MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        }
    }

    fn handle_initialize(&self) -> Value {
        let info = self.server_info();
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": info.name,
                "version": info.version
            },
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "instructions": info.instructions
        })
    }

    fn handle_tools_list(&self) -> Value {
        let tools: Vec<Value> = self.get_tools()
            .into_iter()
            .map(|t| serde_json::json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema
            }))
            .collect();

        serde_json::json!({ "tools": tools })
    }

    fn handle_tools_call(&self, params: Value) -> Value {
        let name = params.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        let arguments = params.get("arguments")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        let call = ToolCall { name, arguments };
        let result = self.handle_tool_call(call);

        serde_json::json!({
            "content": result.content,
            "isError": result.is_error.unwrap_or(false)
        })
    }

    fn handle_resources_list(&self) -> Value {
        let resources: Vec<Value> = self.get_resources()
            .into_iter()
            .map(|r| serde_json::json!({
                "uri": r.uri,
                "name": r.name,
                "description": r.description,
                "mimeType": r.mime_type
            }))
            .collect();

        serde_json::json!({ "resources": resources })
    }

    fn handle_resources_read(&self, params: Value) -> Value {
        let uri = params.get("uri")
            .and_then(|u| u.as_str())
            .unwrap_or("");

        match self.read_resource(uri) {
            Some(content) => serde_json::json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/markdown",
                    "text": content
                }]
            }),
            None => serde_json::json!({
                "contents": [],
                "error": format!("Resource not found: {}", uri)
            }),
        }
    }

    fn handle_prompts_list(&self) -> Value {
        // CRA governance prompt - always available
        serde_json::json!({
            "prompts": [{
                "name": "cra-governance",
                "description": "CRA governance context and rules",
                "arguments": []
            }]
        })
    }
}
