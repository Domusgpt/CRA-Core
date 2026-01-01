//! MCP Server implementation

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::bootstrap::{BootstrapProtocol, BootstrapResult, BootstrapContext, GovernanceSection, ChainState, GovernanceRule, PolicySummary};
use crate::error::{McpError, McpResult};
use crate::session::SessionManager;
use crate::tools::{self, ToolDefinition};
use crate::resources::{self, ResourceDefinition};

/// MCP Server for CRA
pub struct McpServer {
    /// Session manager
    session_manager: Arc<SessionManager>,

    /// Server name
    name: String,

    /// Server version
    version: String,
}

impl McpServer {
    /// Create a new MCP server builder
    pub fn builder() -> McpServerBuilder {
        McpServerBuilder::new()
    }

    /// Run the server on stdio (standard MCP transport)
    pub async fn run_stdio(&self) -> McpResult<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);

        tracing::info!("CRA MCP Server started on stdio");

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                tracing::info!("EOF received, shutting down");
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<JsonRpcRequest>(line) {
                Ok(request) => {
                    let response = self.handle_request(request).await;
                    let response_json = serde_json::to_string(&response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
                Err(e) => {
                    let error_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                    };
                    let response_json = serde_json::to_string(&error_response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
        }

        Ok(())
    }

    /// Handle a JSON-RPC request
    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            // MCP Protocol methods
            "initialize" => self.handle_initialize(&request.params).await,
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_call_tool(&request.params).await,
            "resources/list" => self.handle_list_resources().await,
            "resources/read" => self.handle_read_resource(&request.params).await,

            // Unknown method
            _ => Err(McpError::Validation(format!("Unknown method: {}", request.method))),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: e.error_code(),
                    message: e.to_string(),
                    data: None,
                }),
            },
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, _params: &Option<Value>) -> McpResult<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": self.name,
                "version": self.version
            }
        }))
    }

    /// Handle tools/list request
    async fn handle_list_tools(&self) -> McpResult<Value> {
        let tools = tools::get_tool_definitions();
        Ok(json!({ "tools": tools }))
    }

    /// Handle tools/call request
    async fn handle_call_tool(&self, params: &Option<Value>) -> McpResult<Value> {
        let params = params.as_ref()
            .ok_or_else(|| McpError::Validation("Missing params".to_string()))?;

        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::Validation("Missing tool name".to_string()))?;

        let arguments = params.get("arguments")
            .cloned()
            .unwrap_or(json!({}));

        let result = match name {
            "cra_start_session" => self.call_start_session(arguments).await?,
            "cra_end_session" => self.call_end_session(arguments).await?,
            "cra_get_trace" => self.call_get_trace(arguments).await?,
            "cra_request_context" => self.call_request_context(arguments).await?,
            "cra_search_contexts" => self.call_search_contexts(arguments).await?,
            "cra_list_atlases" => self.call_list_atlases(arguments).await?,
            "cra_report_action" => self.call_report_action(arguments).await?,
            "cra_feedback" => self.call_feedback(arguments).await?,
            "cra_bootstrap" => self.call_bootstrap(arguments).await?,
            _ => return Err(McpError::Validation(format!("Unknown tool: {}", name))),
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }]
        }))
    }

    /// Handle resources/list request
    async fn handle_list_resources(&self) -> McpResult<Value> {
        let resources = resources::get_resource_definitions();
        Ok(json!({ "resources": resources }))
    }

    /// Handle resources/read request
    async fn handle_read_resource(&self, params: &Option<Value>) -> McpResult<Value> {
        let params = params.as_ref()
            .ok_or_else(|| McpError::Validation("Missing params".to_string()))?;

        let uri = params.get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::Validation("Missing resource URI".to_string()))?;

        let content = self.read_resource(uri).await?;

        Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&content)?
            }]
        }))
    }

    /// Read a resource by URI
    async fn read_resource(&self, uri: &str) -> McpResult<Value> {
        if uri == "cra://session/current" {
            let session = self.session_manager.get_current_session()?;
            Ok(json!({
                "session_id": session.session_id,
                "agent_id": session.agent_id,
                "goal": session.goal,
                "started_at": session.started_at.to_rfc3339(),
                "active_atlases": session.active_atlases,
                "injected_contexts": session.injected_contexts,
                "event_count": session.event_count,
                "current_hash": session.current_hash
            }))
        } else if uri.starts_with("cra://trace/") {
            let session_id = uri.strip_prefix("cra://trace/")
                .ok_or_else(|| McpError::Validation("Invalid trace URI".to_string()))?;

            let events = self.session_manager.get_trace(session_id)?;
            Ok(json!({
                "session_id": session_id,
                "event_count": events.len(),
                "events": events
            }))
        } else if uri.starts_with("cra://chain/") {
            let session_id = uri.strip_prefix("cra://chain/")
                .ok_or_else(|| McpError::Validation("Invalid chain URI".to_string()))?;

            let verification = self.session_manager.verify_chain(session_id)?;
            let session = self.session_manager.get_session(session_id)?;

            Ok(json!({
                "session_id": session_id,
                "is_valid": verification.is_valid,
                "event_count": verification.event_count,
                "genesis_hash": session.genesis_hash,
                "current_hash": session.current_hash,
                "first_invalid_index": verification.first_invalid_index,
                "error": verification.error_message
            }))
        } else if uri.starts_with("cra://atlas/") {
            let atlas_id = uri.strip_prefix("cra://atlas/")
                .ok_or_else(|| McpError::Validation("Invalid atlas URI".to_string()))?;

            // TODO: Get full atlas manifest
            Ok(json!({
                "atlas_id": atlas_id,
                "status": "not_implemented"
            }))
        } else {
            Err(McpError::Validation(format!("Unknown resource URI: {}", uri)))
        }
    }

    // Tool implementations

    async fn call_start_session(&self, args: Value) -> McpResult<Value> {
        let input: tools::session::StartSessionInput = serde_json::from_value(args)?;

        let session = self.session_manager.start_session(
            "mcp-agent".to_string(),
            input.goal,
            Some(input.atlas_hints),
        )?;

        Ok(json!({
            "session_id": session.session_id,
            "active_atlases": session.active_atlases,
            "initial_context": [],
            "genesis_hash": session.genesis_hash
        }))
    }

    async fn call_end_session(&self, args: Value) -> McpResult<Value> {
        let input: tools::session::EndSessionInput = serde_json::from_value(args)?;

        let session = self.session_manager.get_current_session()?;
        let verification = self.session_manager.verify_chain(&session.session_id)?;
        let ended_session = self.session_manager.end_session(&session.session_id, input.summary)?;

        Ok(json!({
            "session_id": ended_session.session_id,
            "duration_ms": ended_session.duration_ms(),
            "event_count": ended_session.event_count,
            "chain_verified": verification.is_valid,
            "final_hash": ended_session.current_hash
        }))
    }

    async fn call_get_trace(&self, args: Value) -> McpResult<Value> {
        let input: tools::session::GetTraceInput = serde_json::from_value(args)?;

        // Get session (current or specified)
        let session = if let Some(session_id) = input.session_id {
            self.session_manager.get_session(&session_id)?
        } else {
            self.session_manager.get_current_session()?
        };

        // Get trace events
        let events = self.session_manager.get_trace(&session.session_id)?;
        let verification = self.session_manager.verify_chain(&session.session_id)?;

        // Format based on request
        let format = input.format.unwrap_or_else(|| "json".to_string());
        let events_output: Value = if format == "jsonl" {
            // Return as newline-delimited JSON string
            let jsonl: String = events.iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n");
            json!(jsonl)
        } else {
            // Return as JSON array
            json!(events)
        };

        Ok(json!({
            "session_id": session.session_id,
            "event_count": events.len(),
            "genesis_hash": session.genesis_hash,
            "current_hash": session.current_hash,
            "is_valid": verification.is_valid,
            "events": events_output
        }))
    }

    async fn call_request_context(&self, args: Value) -> McpResult<Value> {
        let input: tools::context::RequestContextInput = serde_json::from_value(args)?;

        let session = self.session_manager.get_current_session()?;
        let matched = self.session_manager.request_context(
            &session.session_id,
            &input.need,
            Some(input.hints),
        )?;

        Ok(json!({
            "matched_contexts": matched,
            "trace_id": uuid::Uuid::new_v4().to_string()
        }))
    }

    async fn call_search_contexts(&self, args: Value) -> McpResult<Value> {
        let input: tools::context::SearchContextsInput = serde_json::from_value(args)?;

        // TODO: Implement context search
        Ok(json!({
            "results": [],
            "total_count": 0
        }))
    }

    async fn call_list_atlases(&self, _args: Value) -> McpResult<Value> {
        let atlases = self.session_manager.list_atlases()?;

        Ok(json!({
            "atlases": atlases
        }))
    }

    async fn call_report_action(&self, args: Value) -> McpResult<Value> {
        let input: tools::action::ReportActionInput = serde_json::from_value(args)?;

        let session = self.session_manager.get_current_session()?;
        let report = self.session_manager.report_action(
            &session.session_id,
            &input.action,
            input.params,
        )?;

        Ok(json!(report))
    }

    async fn call_feedback(&self, args: Value) -> McpResult<Value> {
        let input: tools::feedback::FeedbackInput = serde_json::from_value(args)?;

        let session = self.session_manager.get_current_session()?;
        self.session_manager.submit_feedback(
            &session.session_id,
            &input.context_id,
            input.helpful,
            input.reason,
        )?;

        Ok(json!({
            "recorded": true,
            "trace_id": uuid::Uuid::new_v4().to_string()
        }))
    }

    async fn call_bootstrap(&self, args: Value) -> McpResult<Value> {
        let input: tools::session::BootstrapInput = serde_json::from_value(args)?;

        // Start session
        let session = self.session_manager.start_session(
            "mcp-agent".to_string(),
            input.intent.clone(),
            None,
        )?;

        // Create bootstrap result with full governance info
        let result = BootstrapResult {
            session_id: session.session_id.clone(),
            genesis_hash: session.genesis_hash.clone(),
            governance: GovernanceSection {
                rules: vec![
                    GovernanceRule {
                        rule_id: "trace.required".to_string(),
                        description: "All actions must be reported through cra_report_action".to_string(),
                        enforcement: "hard".to_string(),
                    },
                    GovernanceRule {
                        rule_id: "context.must_request".to_string(),
                        description: "Request context before making domain-specific decisions".to_string(),
                        enforcement: "soft".to_string(),
                    },
                    GovernanceRule {
                        rule_id: "feedback.expected".to_string(),
                        description: "Provide feedback on context usefulness".to_string(),
                        enforcement: "soft".to_string(),
                    },
                ],
                policies: Vec::new(),
                you_must: vec![
                    "Report all significant actions via cra_report_action".to_string(),
                    "Request context when unsure about domain specifics".to_string(),
                    "Provide feedback on context usefulness".to_string(),
                ],
            },
            context: Vec::new(), // Would be populated from atlases
            chain_state: ChainState {
                current_hash: session.current_hash.clone(),
                sequence: session.event_count,
                verified: true,
            },
            ready: true,
            message: "Governance established. Context internalized. You may begin.".to_string(),
        };

        Ok(json!(result))
    }
}

/// Builder for McpServer
pub struct McpServerBuilder {
    atlases_dir: Option<String>,
    traces_dir: Option<String>,
    name: String,
    version: String,
}

impl McpServerBuilder {
    pub fn new() -> Self {
        Self {
            atlases_dir: None,
            traces_dir: None,
            name: crate::SERVER_NAME.to_string(),
            version: crate::SERVER_VERSION.to_string(),
        }
    }

    pub fn with_atlases_dir(mut self, dir: &str) -> Self {
        self.atlases_dir = Some(dir.to_string());
        self
    }

    /// Enable trace persistence to a directory
    ///
    /// Trace events will be written to JSONL files in the specified directory,
    /// one file per session (e.g., `{session_id}.jsonl`).
    pub fn with_traces_dir(mut self, dir: &str) -> Self {
        self.traces_dir = Some(dir.to_string());
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub async fn build(self) -> McpResult<McpServer> {
        let mut session_manager = SessionManager::new();

        // Configure trace persistence
        if let Some(traces_dir) = &self.traces_dir {
            session_manager = session_manager.with_traces_dir(traces_dir);
        }

        // Configure atlases
        if let Some(atlases_dir) = &self.atlases_dir {
            session_manager = session_manager.with_atlases_dir(atlases_dir);
            session_manager.load_atlases()?;
        }

        Ok(McpServer {
            session_manager: Arc::new(session_manager),
            name: self.name,
            version: self.version,
        })
    }
}

impl Default for McpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// JSON-RPC types

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}
