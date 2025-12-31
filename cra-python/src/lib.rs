//! CRA Python bindings via PyO3
//!
//! This module provides Python bindings for the CRA Core library,
//! allowing Python applications to use CRA for agent governance.
//!
//! ## Example
//!
//! ```python
//! from cra import Resolver, CARPRequest
//!
//! # Create a resolver
//! resolver = Resolver()
//!
//! # Load an atlas from JSON
//! atlas_id = resolver.load_atlas_json(atlas_json)
//!
//! # Create a session
//! session_id = resolver.create_session("my-agent", "Help the user")
//!
//! # Resolve a request
//! resolution = resolver.resolve(session_id, "my-agent", "I want to greet someone")
//!
//! # Access resolution as Python objects
//! print(f"Decision: {resolution.decision}")
//! for action in resolution.allowed_actions:
//!     print(f"  - {action.action_id}: {action.description}")
//!
//! # Get the trace as events
//! for event in resolver.get_trace_events(session_id):
//!     print(f"{event.event_type}: {event.payload}")
//!
//! # Verify chain integrity
//! verification = resolver.verify_chain(session_id)
//! assert verification.is_valid
//!
//! # End the session
//! resolver.end_session(session_id)
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use std::collections::HashMap;

use cra_core::{
    self,
    AtlasManifest,
    CARPRequest as CoreCARPRequest,
    CARPResolution as CoreCARPResolution,
    AllowedAction as CoreAllowedAction,
    DeniedAction as CoreDeniedAction,
    Resolver as CoreResolver,
    TRACEEvent as CoreTRACEEvent,
    ChainVerification as CoreChainVerification,
};

// =============================================================================
// Python Types - Proper Python objects, not just JSON strings
// =============================================================================

/// An allowed action from a CARP resolution
#[pyclass]
#[derive(Clone)]
pub struct AllowedAction {
    #[pyo3(get)]
    pub action_id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: Option<String>,
    #[pyo3(get)]
    pub risk_tier: String,
    #[pyo3(get)]
    pub parameters_schema: Option<String>,
}

#[pymethods]
impl AllowedAction {
    fn __repr__(&self) -> String {
        format!("AllowedAction(action_id='{}', name='{}')", self.action_id, self.name)
    }

    fn __str__(&self) -> String {
        self.action_id.clone()
    }

    /// Convert to dict
    fn to_dict(&self) -> HashMap<String, PyObject> {
        Python::with_gil(|py| {
            let mut map = HashMap::new();
            map.insert("action_id".to_string(), self.action_id.clone().into_py(py));
            map.insert("name".to_string(), self.name.clone().into_py(py));
            map.insert("description".to_string(), self.description.clone().into_py(py));
            map.insert("risk_tier".to_string(), self.risk_tier.clone().into_py(py));
            map
        })
    }
}

impl From<&CoreAllowedAction> for AllowedAction {
    fn from(action: &CoreAllowedAction) -> Self {
        AllowedAction {
            action_id: action.action_id.clone(),
            name: action.name.clone(),
            description: action.description.clone(),
            risk_tier: action.risk_tier.clone(),
            parameters_schema: Some(serde_json::to_string(&action.parameters_schema).unwrap_or_default()),
        }
    }
}

/// A denied action from a CARP resolution
#[pyclass]
#[derive(Clone)]
pub struct DeniedAction {
    #[pyo3(get)]
    pub action_id: String,
    #[pyo3(get)]
    pub policy_id: String,
    #[pyo3(get)]
    pub reason: String,
}

#[pymethods]
impl DeniedAction {
    fn __repr__(&self) -> String {
        format!("DeniedAction(action_id='{}', reason='{}')", self.action_id, self.reason)
    }
}

impl From<&CoreDeniedAction> for DeniedAction {
    fn from(action: &CoreDeniedAction) -> Self {
        DeniedAction {
            action_id: action.action_id.clone(),
            policy_id: action.policy_id.clone(),
            reason: action.reason.clone(),
        }
    }
}

/// A CARP resolution result
#[pyclass]
#[derive(Clone)]
pub struct CARPResolution {
    #[pyo3(get)]
    pub resolution_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub trace_id: String,
    #[pyo3(get)]
    pub decision: String,
    #[pyo3(get)]
    pub allowed_actions: Vec<AllowedAction>,
    #[pyo3(get)]
    pub denied_actions: Vec<DeniedAction>,
    #[pyo3(get)]
    pub ttl_seconds: u64,
}

#[pymethods]
impl CARPResolution {
    fn __repr__(&self) -> String {
        format!(
            "CARPResolution(decision='{}', allowed={}, denied={})",
            self.decision,
            self.allowed_actions.len(),
            self.denied_actions.len()
        )
    }

    /// Check if decision allows actions
    #[getter]
    fn is_allowed(&self) -> bool {
        self.decision == "allow" || self.decision == "allow_with_constraints" || self.decision == "partial"
    }

    /// Check if decision denies all actions
    #[getter]
    fn is_denied(&self) -> bool {
        self.decision == "deny"
    }

    /// Get action by ID
    fn get_action(&self, action_id: &str) -> Option<AllowedAction> {
        self.allowed_actions.iter()
            .find(|a| a.action_id == action_id)
            .cloned()
    }

    /// Check if an action is allowed
    fn is_action_allowed(&self, action_id: &str) -> bool {
        self.allowed_actions.iter().any(|a| a.action_id == action_id)
    }

    /// Convert to JSON string
    fn to_json(&self) -> PyResult<String> {
        // Reconstruct and serialize
        Ok(format!(
            r#"{{"resolution_id":"{}","session_id":"{}","decision":"{}","allowed_count":{},"denied_count":{}}}"#,
            self.resolution_id, self.session_id, self.decision,
            self.allowed_actions.len(), self.denied_actions.len()
        ))
    }
}

impl From<CoreCARPResolution> for CARPResolution {
    fn from(res: CoreCARPResolution) -> Self {
        CARPResolution {
            resolution_id: res.trace_id.clone(),  // Use trace_id as resolution_id
            session_id: res.session_id,
            trace_id: res.trace_id,
            decision: res.decision.to_string(),
            allowed_actions: res.allowed_actions.iter().map(AllowedAction::from).collect(),
            denied_actions: res.denied_actions.iter().map(DeniedAction::from).collect(),
            ttl_seconds: res.ttl_seconds,
        }
    }
}

/// A TRACE event
#[pyclass]
#[derive(Clone)]
pub struct TRACEEvent {
    #[pyo3(get)]
    pub event_id: String,
    #[pyo3(get)]
    pub trace_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp: String,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub payload: String,  // JSON string
    #[pyo3(get)]
    pub event_hash: String,
    #[pyo3(get)]
    pub previous_event_hash: String,
}

#[pymethods]
impl TRACEEvent {
    fn __repr__(&self) -> String {
        format!("TRACEEvent(type='{}', seq={})", self.event_type, self.sequence)
    }

    /// Get payload as Python dict
    fn get_payload_dict(&self, py: Python) -> PyResult<PyObject> {
        let value: serde_json::Value = serde_json::from_str(&self.payload)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        // Convert to Python object
        json_to_py(py, &value)
    }

    /// Convert to JSON string
    fn to_json(&self) -> String {
        format!(
            r#"{{"event_id":"{}","event_type":"{}","sequence":{},"event_hash":"{}"}}"#,
            self.event_id, self.event_type, self.sequence, self.event_hash
        )
    }
}

impl From<&CoreTRACEEvent> for TRACEEvent {
    fn from(event: &CoreTRACEEvent) -> Self {
        TRACEEvent {
            event_id: event.event_id.clone(),
            trace_id: event.trace_id.clone(),
            session_id: event.session_id.clone(),
            sequence: event.sequence,
            timestamp: event.timestamp.to_rfc3339(),
            event_type: event.event_type.to_string(),
            payload: serde_json::to_string(&event.payload).unwrap_or_default(),
            event_hash: event.event_hash.clone(),
            previous_event_hash: event.previous_event_hash.clone(),
        }
    }
}

/// Chain verification result
#[pyclass]
#[derive(Clone)]
pub struct ChainVerification {
    #[pyo3(get)]
    pub is_valid: bool,
    #[pyo3(get)]
    pub event_count: usize,
    #[pyo3(get)]
    pub first_invalid_index: Option<usize>,
    #[pyo3(get)]
    pub error_type: Option<String>,
    #[pyo3(get)]
    pub error_message: Option<String>,
}

#[pymethods]
impl ChainVerification {
    fn __repr__(&self) -> String {
        if self.is_valid {
            format!("ChainVerification(valid=True, events={})", self.event_count)
        } else {
            format!("ChainVerification(valid=False, error={:?})", self.error_type)
        }
    }

    fn __bool__(&self) -> bool {
        self.is_valid
    }
}

impl From<CoreChainVerification> for ChainVerification {
    fn from(v: CoreChainVerification) -> Self {
        ChainVerification {
            is_valid: v.is_valid,
            event_count: v.event_count,
            first_invalid_index: v.first_invalid_index,
            error_type: v.error_type.map(|e| format!("{:?}", e)),
            error_message: v.error_message,
        }
    }
}

// =============================================================================
// Resolver - The main Python interface
// =============================================================================

/// Python wrapper for the CRA Resolver
#[pyclass]
pub struct Resolver {
    inner: CoreResolver,
}

#[pymethods]
impl Resolver {
    /// Create a new resolver
    #[new]
    fn new() -> Self {
        Resolver {
            inner: CoreResolver::new(),
        }
    }

    /// Load an atlas from a JSON string
    ///
    /// Returns the atlas ID on success
    fn load_atlas_json(&mut self, json: &str) -> PyResult<String> {
        let manifest: AtlasManifest = serde_json::from_str(json)
            .map_err(|e| PyValueError::new_err(format!("Invalid atlas JSON: {}", e)))?;

        self.inner
            .load_atlas(manifest)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to load atlas: {}", e)))
    }

    /// Load an atlas from a file path
    fn load_atlas_file(&mut self, path: &str) -> PyResult<String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to read file: {}", e)))?;
        self.load_atlas_json(&content)
    }

    /// Unload an atlas by ID
    fn unload_atlas(&mut self, atlas_id: &str) -> PyResult<()> {
        self.inner
            .unload_atlas(atlas_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to unload atlas: {}", e)))
    }

    /// List all loaded atlas IDs
    fn list_atlases(&self) -> Vec<String> {
        self.inner.list_atlases().iter().map(|s| s.to_string()).collect()
    }

    /// Create a new session
    ///
    /// Returns the session ID
    fn create_session(&mut self, agent_id: &str, goal: &str) -> PyResult<String> {
        self.inner
            .create_session(agent_id, goal)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create session: {}", e)))
    }

    /// End a session
    fn end_session(&mut self, session_id: &str) -> PyResult<()> {
        self.inner
            .end_session(session_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to end session: {}", e)))
    }

    /// Resolve a CARP request
    ///
    /// Returns a CARPResolution object with allowed/denied actions
    fn resolve(&mut self, session_id: &str, agent_id: &str, goal: &str) -> PyResult<CARPResolution> {
        let request = CoreCARPRequest::new(
            session_id.to_string(),
            agent_id.to_string(),
            goal.to_string(),
        );

        let resolution = self
            .inner
            .resolve(&request)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to resolve: {}", e)))?;

        Ok(CARPResolution::from(resolution))
    }

    /// Resolve and return JSON string (for compatibility)
    fn resolve_json(&mut self, session_id: &str, agent_id: &str, goal: &str) -> PyResult<String> {
        let request = CoreCARPRequest::new(
            session_id.to_string(),
            agent_id.to_string(),
            goal.to_string(),
        );

        let resolution = self
            .inner
            .resolve(&request)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to resolve: {}", e)))?;

        serde_json::to_string(&resolution)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize: {}", e)))
    }

    /// Execute an action
    ///
    /// Returns the result as a JSON string
    fn execute(
        &mut self,
        session_id: &str,
        resolution_id: &str,
        action_id: &str,
        parameters_json: Option<&str>,
    ) -> PyResult<String> {
        let params: serde_json::Value = match parameters_json {
            Some(json) => serde_json::from_str(json)
                .map_err(|e| PyValueError::new_err(format!("Invalid parameters JSON: {}", e)))?,
            None => serde_json::json!({}),
        };

        let result = self
            .inner
            .execute(session_id, resolution_id, action_id, params)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to execute: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize: {}", e)))
    }

    /// Get the trace for a session as JSONL string
    fn get_trace(&self, session_id: &str) -> PyResult<String> {
        let events = self
            .inner
            .get_trace(session_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get trace: {}", e)))?;

        let lines: Vec<String> = events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect();

        Ok(lines.join("\n"))
    }

    /// Get the trace for a session as a list of TRACEEvent objects
    fn get_trace_events(&self, session_id: &str) -> PyResult<Vec<TRACEEvent>> {
        let events = self
            .inner
            .get_trace(session_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get trace: {}", e)))?;

        Ok(events.iter().map(TRACEEvent::from).collect())
    }

    /// Verify the hash chain for a session
    fn verify_chain(&self, session_id: &str) -> PyResult<ChainVerification> {
        let verification = self
            .inner
            .verify_chain(session_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to verify: {}", e)))?;

        Ok(ChainVerification::from(verification))
    }

    /// Get event count for a session
    fn get_event_count(&self, session_id: &str) -> PyResult<usize> {
        let events = self
            .inner
            .get_trace(session_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get trace: {}", e)))?;
        Ok(events.len())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert serde_json::Value to Python object
fn json_to_py(py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.into_py(py)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_py(py)),
        serde_json::Value::Array(arr) => {
            let list: Vec<PyObject> = arr.iter()
                .map(|v| json_to_py(py, v))
                .collect::<PyResult<Vec<_>>>()?;
            Ok(list.into_py(py))
        }
        serde_json::Value::Object(obj) => {
            let dict = pyo3::types::PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

// =============================================================================
// Module Functions
// =============================================================================

/// Get the CRA core version
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get the CARP protocol version
#[pyfunction]
fn carp_version() -> &'static str {
    cra_core::CARP_VERSION
}

/// Get the TRACE protocol version
#[pyfunction]
fn trace_version() -> &'static str {
    cra_core::TRACE_VERSION
}

/// Get the Atlas format version
#[pyfunction]
fn atlas_version() -> &'static str {
    cra_core::ATLAS_VERSION
}

/// Get the genesis hash constant
#[pyfunction]
fn genesis_hash() -> &'static str {
    cra_core::trace::GENESIS_HASH
}

// =============================================================================
// Python Module
// =============================================================================

/// CRA - Context Registry Agents
///
/// A governance layer for AI agents providing:
/// - CARP: Context & Action Resolution Protocol
/// - TRACE: Telemetry & Replay Audit Contract
/// - Atlas: Domain context packages
#[pymodule]
fn cra(_py: Python, m: &PyModule) -> PyResult<()> {
    // Classes
    m.add_class::<Resolver>()?;
    m.add_class::<CARPResolution>()?;
    m.add_class::<AllowedAction>()?;
    m.add_class::<DeniedAction>()?;
    m.add_class::<TRACEEvent>()?;
    m.add_class::<ChainVerification>()?;

    // Functions
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(carp_version, m)?)?;
    m.add_function(wrap_pyfunction!(trace_version, m)?)?;
    m.add_function(wrap_pyfunction!(atlas_version, m)?)?;
    m.add_function(wrap_pyfunction!(genesis_hash, m)?)?;

    Ok(())
}
