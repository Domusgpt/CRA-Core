//! CRA Client for communicating with CRA server

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::WrapperResult;
use crate::ContextBlock;

/// CRA Client interface
#[async_trait]
pub trait CRAClient: Send + Sync {
    /// Bootstrap/initialize with CRA
    async fn bootstrap(&self, goal: &str) -> WrapperResult<BootstrapResult>;

    /// Request context for a need
    async fn request_context(
        &self,
        session_id: &str,
        need: &str,
        hints: Option<Vec<String>>,
    ) -> WrapperResult<Vec<ContextBlock>>;

    /// Report an action
    async fn report_action(
        &self,
        session_id: &str,
        action: &str,
        params: serde_json::Value,
    ) -> WrapperResult<ActionReport>;

    /// Submit feedback
    async fn feedback(
        &self,
        session_id: &str,
        context_id: &str,
        helpful: bool,
        reason: Option<&str>,
    ) -> WrapperResult<()>;

    /// Upload TRACE events
    async fn upload_trace(&self, events: Vec<serde_json::Value>) -> WrapperResult<UploadResult>;

    /// End session
    async fn end_session(&self, session_id: &str, summary: Option<&str>) -> WrapperResult<EndSessionResult>;
}

/// Result from bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Session ID
    pub session_id: String,

    /// Genesis hash
    pub genesis_hash: String,

    /// Current hash
    pub current_hash: String,

    /// Context IDs provided
    pub context_ids: Vec<String>,

    /// Contexts with content
    pub contexts: Vec<BootstrapContext>,

    /// Governance rules
    pub rules: Vec<GovernanceRule>,
}

/// Context provided during bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapContext {
    pub context_id: String,
    pub content: String,
    pub priority: i32,
}

/// Governance rule from bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRule {
    pub rule_id: String,
    pub description: String,
    pub enforcement: String,
}

/// Result from action report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionReport {
    /// Decision: "approved" or "denied"
    pub decision: String,

    /// Trace ID
    pub trace_id: String,

    /// Reason (for denials)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Policy notes
    pub policy_notes: Vec<String>,
}

/// Result from trace upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    /// Number of events uploaded
    pub uploaded_count: usize,

    /// Whether upload was successful
    pub success: bool,
}

/// Result from end session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndSessionResult {
    /// Whether chain was verified
    pub chain_verified: bool,

    /// Final hash
    pub final_hash: String,

    /// Event count
    pub event_count: u64,
}

/// Direct client (same process, for testing)
pub struct DirectClient;

impl DirectClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DirectClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CRAClient for DirectClient {
    async fn bootstrap(&self, goal: &str) -> WrapperResult<BootstrapResult> {
        // Direct mode - create mock bootstrap result
        let session_id = uuid::Uuid::new_v4().to_string();
        let genesis_hash = format!("genesis_{}", uuid::Uuid::new_v4());

        Ok(BootstrapResult {
            session_id: session_id.clone(),
            genesis_hash: genesis_hash.clone(),
            current_hash: genesis_hash,
            context_ids: Vec::new(),
            contexts: Vec::new(),
            rules: vec![
                GovernanceRule {
                    rule_id: "trace.required".to_string(),
                    description: "All actions must be reported".to_string(),
                    enforcement: "hard".to_string(),
                },
            ],
        })
    }

    async fn request_context(
        &self,
        _session_id: &str,
        _need: &str,
        _hints: Option<Vec<String>>,
    ) -> WrapperResult<Vec<ContextBlock>> {
        // Direct mode - no external contexts
        Ok(Vec::new())
    }

    async fn report_action(
        &self,
        _session_id: &str,
        action: &str,
        _params: serde_json::Value,
    ) -> WrapperResult<ActionReport> {
        // Direct mode - always approve
        Ok(ActionReport {
            decision: "approved".to_string(),
            trace_id: uuid::Uuid::new_v4().to_string(),
            reason: None,
            policy_notes: vec!["Action permitted (direct mode)".to_string()],
        })
    }

    async fn feedback(
        &self,
        _session_id: &str,
        _context_id: &str,
        _helpful: bool,
        _reason: Option<&str>,
    ) -> WrapperResult<()> {
        Ok(())
    }

    async fn upload_trace(&self, events: Vec<serde_json::Value>) -> WrapperResult<UploadResult> {
        Ok(UploadResult {
            uploaded_count: events.len(),
            success: true,
        })
    }

    async fn end_session(&self, _session_id: &str, _summary: Option<&str>) -> WrapperResult<EndSessionResult> {
        Ok(EndSessionResult {
            chain_verified: true,
            final_hash: format!("final_{}", uuid::Uuid::new_v4()),
            event_count: 0,
        })
    }
}
