//! Session management for MCP server

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use cra_core::{Resolver, AtlasManifest, ContextBlock};

use crate::error::{McpError, McpResult};

/// Session state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub session_id: String,

    /// Agent identifier
    pub agent_id: String,

    /// Session goal/intent
    pub goal: String,

    /// When the session started
    pub started_at: DateTime<Utc>,

    /// Active atlas IDs
    pub active_atlases: Vec<String>,

    /// Genesis hash (first hash in the chain)
    pub genesis_hash: String,

    /// Current chain hash
    pub current_hash: String,

    /// Number of events in trace
    pub event_count: u64,

    /// Contexts that have been injected
    pub injected_contexts: Vec<String>,

    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(agent_id: String, goal: String, active_atlases: Vec<String>, genesis_hash: String) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            agent_id,
            goal,
            started_at: Utc::now(),
            active_atlases,
            genesis_hash: genesis_hash.clone(),
            current_hash: genesis_hash,
            event_count: 1, // Genesis event
            injected_contexts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Update the chain hash
    pub fn update_hash(&mut self, hash: String) {
        self.current_hash = hash;
        self.event_count += 1;
    }

    /// Record context injection
    pub fn record_context_injection(&mut self, context_id: String) {
        if !self.injected_contexts.contains(&context_id) {
            self.injected_contexts.push(context_id);
        }
    }

    /// Get session duration in milliseconds
    pub fn duration_ms(&self) -> i64 {
        let now = Utc::now();
        (now - self.started_at).num_milliseconds()
    }
}

/// Manages all active sessions
pub struct SessionManager {
    /// The CRA resolver
    resolver: RwLock<Resolver>,

    /// Active sessions by session_id
    sessions: RwLock<HashMap<String, Session>>,

    /// Loaded atlases directory (if any)
    atlases_dir: Option<String>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            resolver: RwLock::new(Resolver::new()),
            sessions: RwLock::new(HashMap::new()),
            atlases_dir: None,
        }
    }

    /// Create with an atlases directory
    pub fn with_atlases_dir(mut self, dir: &str) -> Self {
        self.atlases_dir = Some(dir.to_string());
        self
    }

    /// Load atlases from directory
    pub fn load_atlases(&self) -> McpResult<Vec<String>> {
        let Some(dir) = &self.atlases_dir else {
            return Ok(Vec::new());
        };

        let mut loaded = Vec::new();
        let entries = std::fs::read_dir(dir)
            .map_err(|e| McpError::Io(e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| McpError::Io(e))?;

                let manifest: AtlasManifest = serde_json::from_str(&content)
                    .map_err(|e| McpError::Atlas(format!("Failed to parse {}: {}", path.display(), e)))?;

                let atlas_id = manifest.atlas_id.clone();

                let mut resolver = self.resolver.write()
                    .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

                resolver.load_atlas(manifest)?;
                loaded.push(atlas_id);
            }
        }

        Ok(loaded)
    }

    /// Load a single atlas
    pub fn load_atlas(&self, manifest: AtlasManifest) -> McpResult<String> {
        let mut resolver = self.resolver.write()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        let atlas_id = resolver.load_atlas(manifest)?;
        Ok(atlas_id)
    }

    /// Start a new session
    pub fn start_session(&self, agent_id: String, goal: String, atlas_hints: Option<Vec<String>>) -> McpResult<Session> {
        let mut resolver = self.resolver.write()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        // Create session in resolver
        let session_id = resolver.create_session(&agent_id, &goal)?;

        // Get active atlases
        let active_atlases = resolver.list_atlases().iter().map(|s| s.to_string()).collect();

        // Get genesis hash from trace
        let trace = resolver.get_trace(&session_id)?;
        let genesis_hash = trace.first()
            .map(|e| e.event_hash.clone())
            .unwrap_or_else(|| "genesis".to_string());

        // Create session record
        let session = Session::new(agent_id, goal, active_atlases, genesis_hash);
        let session_clone = session.clone();

        // Store session
        let mut sessions = self.sessions.write()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;
        sessions.insert(session_id, session);

        Ok(session_clone)
    }

    /// Get a session
    pub fn get_session(&self, session_id: &str) -> McpResult<Session> {
        let sessions = self.sessions.read()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        sessions.get(session_id)
            .cloned()
            .ok_or_else(|| McpError::InvalidSession(session_id.to_string()))
    }

    /// Get the current session (most recent)
    pub fn get_current_session(&self) -> McpResult<Session> {
        let sessions = self.sessions.read()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        sessions.values()
            .max_by_key(|s| s.started_at)
            .cloned()
            .ok_or_else(|| McpError::NoActiveSession)
    }

    /// End a session
    pub fn end_session(&self, session_id: &str, summary: Option<String>) -> McpResult<Session> {
        // Get final session state
        let session = {
            let mut sessions = self.sessions.write()
                .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

            sessions.remove(session_id)
                .ok_or_else(|| McpError::InvalidSession(session_id.to_string()))?
        };

        // End session in resolver
        let mut resolver = self.resolver.write()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        resolver.end_session(session_id)?;

        Ok(session)
    }

    /// Request context for a need
    pub fn request_context(&self, session_id: &str, need: &str, hints: Option<Vec<String>>) -> McpResult<Vec<MatchedContext>> {
        let resolver = self.resolver.read()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        // For now, search contexts by keyword matching
        // In the future, this would use the context registry more fully
        let mut matched = Vec::new();

        // Get all context blocks from loaded atlases
        for atlas_id in resolver.list_atlases() {
            // TODO: Access context registry through resolver
            // For now, return basic context info
        }

        Ok(matched)
    }

    /// Report an action for audit trail
    pub fn report_action(&self, session_id: &str, action: &str, params: serde_json::Value) -> McpResult<ActionReport> {
        let mut resolver = self.resolver.write()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        // Create a CARP request to check if action is allowed
        let session = self.get_session(session_id)?;

        let request = cra_core::CARPRequest::new(
            session_id.to_string(),
            session.agent_id.clone(),
            format!("Execute action: {}", action),
        );

        let resolution = resolver.resolve(&request)?;

        // Check if this specific action is allowed
        let allowed = resolution.allowed_actions.iter()
            .any(|a| a.action_id == action || action.starts_with(&a.action_id));

        let denied = resolution.denied_actions.iter()
            .find(|d| d.action_id == action || action.starts_with(&d.action_id));

        if let Some(denied_action) = denied {
            return Ok(ActionReport {
                decision: "denied".to_string(),
                trace_id: resolution.trace_id,
                reason: Some(denied_action.reason.clone()),
                policy_notes: vec![format!("Denied by policy: {}", denied_action.policy_id)],
                alternatives: Vec::new(),
            });
        }

        // Action is allowed (or not explicitly denied)
        Ok(ActionReport {
            decision: "approved".to_string(),
            trace_id: resolution.trace_id,
            reason: None,
            policy_notes: vec!["Action permitted".to_string()],
            alternatives: Vec::new(),
        })
    }

    /// Submit feedback on context
    pub fn submit_feedback(&self, session_id: &str, context_id: &str, helpful: bool, reason: Option<String>) -> McpResult<()> {
        // Record feedback in trace
        // For now, just validate session exists
        let _session = self.get_session(session_id)?;

        // TODO: Emit feedback event to trace

        Ok(())
    }

    /// Get trace for a session
    pub fn get_trace(&self, session_id: &str) -> McpResult<Vec<cra_core::TRACEEvent>> {
        let resolver = self.resolver.read()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        let events = resolver.get_trace(session_id)?;
        Ok(events)
    }

    /// Verify chain for a session
    pub fn verify_chain(&self, session_id: &str) -> McpResult<cra_core::ChainVerification> {
        let resolver = self.resolver.read()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        let verification = resolver.verify_chain(session_id)?;
        Ok(verification)
    }

    /// List all loaded atlases
    pub fn list_atlases(&self) -> McpResult<Vec<AtlasInfo>> {
        let resolver = self.resolver.read()
            .map_err(|_| McpError::Internal("Lock poisoned".to_string()))?;

        let atlas_ids = resolver.list_atlases();

        // TODO: Get full atlas info from resolver
        let infos: Vec<AtlasInfo> = atlas_ids.iter()
            .map(|id| AtlasInfo {
                atlas_id: id.to_string(),
                name: id.to_string(), // Would get from manifest
                version: "1.0.0".to_string(),
                description: None,
                domains: Vec::new(),
            })
            .collect();

        Ok(infos)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Matched context from a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedContext {
    pub context_id: String,
    pub name: String,
    pub priority: i32,
    pub match_score: f64,
    pub content: String,
}

/// Action report from report_action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionReport {
    pub decision: String,
    pub trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub policy_notes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
}

/// Atlas information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasInfo {
    pub atlas_id: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub domains: Vec<String>,
}
