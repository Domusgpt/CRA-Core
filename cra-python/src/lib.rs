//! CRA Python bindings via PyO3
//!
//! This module provides Python bindings for the CRA Core library,
//! allowing Python applications to use CRA for agent governance.
//!
//! ## Example
//!
//! ```python
//! from cra import Resolver
//!
//! # Create a resolver
//! resolver = Resolver()
//!
//! # Load an atlas from JSON
//! atlas_json = '''
//! {
//!     "atlas_version": "1.0",
//!     "atlas_id": "com.example.test",
//!     "version": "1.0.0",
//!     "name": "Test Atlas",
//!     "description": "A test atlas",
//!     "domains": ["test"],
//!     "capabilities": [],
//!     "policies": [],
//!     "actions": [
//!         {
//!             "action_id": "test.greet",
//!             "name": "Greet",
//!             "description": "Say hello",
//!             "parameters_schema": {"type": "object"},
//!             "risk_tier": "low"
//!         }
//!     ]
//! }
//! '''
//! atlas_id = resolver.load_atlas_json(atlas_json)
//!
//! # Create a session
//! session_id = resolver.create_session("my-agent", "Help the user")
//!
//! # Resolve a request
//! resolution = resolver.resolve(session_id, "my-agent", "I want to greet someone")
//! print(resolution)  # JSON string with allowed actions
//!
//! # Get the trace
//! trace = resolver.get_trace(session_id)
//!
//! # Verify chain integrity
//! verification = resolver.verify_chain(session_id)
//!
//! # End the session
//! resolver.end_session(session_id)
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use cra_core::{
    AtlasManifest, CARPRequest, Resolver as CoreResolver,
};

/// Python wrapper for the CRA Resolver
#[pyclass]
struct Resolver {
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
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse atlas JSON: {}", e)))?;

        self.inner
            .load_atlas(manifest)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to load atlas: {}", e)))
    }

    /// Unload an atlas by ID
    fn unload_atlas(&mut self, atlas_id: &str) -> PyResult<()> {
        self.inner
            .unload_atlas(atlas_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to unload atlas: {}", e)))
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
    /// Returns a JSON string containing the resolution
    fn resolve(&mut self, session_id: &str, agent_id: &str, goal: &str) -> PyResult<String> {
        let request = CARPRequest::new(
            session_id.to_string(),
            agent_id.to_string(),
            goal.to_string(),
        );

        let resolution = self
            .inner
            .resolve(&request)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to resolve: {}", e)))?;

        serde_json::to_string(&resolution)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize resolution: {}", e)))
    }

    /// Execute an action
    ///
    /// Returns a JSON string containing the result
    fn execute(
        &mut self,
        session_id: &str,
        resolution_id: &str,
        action_id: &str,
        parameters_json: Option<&str>,
    ) -> PyResult<String> {
        let params: serde_json::Value = match parameters_json {
            Some(json) => serde_json::from_str(json)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse parameters: {}", e)))?,
            None => serde_json::json!({}),
        };

        let result = self
            .inner
            .execute(session_id, resolution_id, action_id, params)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to execute: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize result: {}", e)))
    }

    /// Get the trace for a session as JSONL
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

    /// Verify the hash chain for a session
    ///
    /// Returns a JSON string containing the verification result
    fn verify_chain(&self, session_id: &str) -> PyResult<String> {
        let verification = self
            .inner
            .verify_chain(session_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to verify chain: {}", e)))?;

        serde_json::to_string(&verification)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize verification: {}", e)))
    }

    /// List all loaded atlas IDs
    fn list_atlases(&self) -> Vec<String> {
        self.inner.list_atlases().iter().map(|s| s.to_string()).collect()
    }
}

/// Get the CRA core version
#[pyfunction]
fn version() -> &'static str {
    "0.1.0"
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

/// CRA Python module
#[pymodule]
fn cra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Resolver>()?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(carp_version, m)?)?;
    m.add_function(wrap_pyfunction!(trace_version, m)?)?;
    m.add_function(wrap_pyfunction!(atlas_version, m)?)?;
    Ok(())
}
