//! CRA Node.js bindings via napi-rs
//!
//! This module provides Node.js bindings for the CRA Core library,
//! allowing Node.js applications to use CRA for agent governance.
//!
//! ## Example
//!
//! ```javascript
//! const { Resolver } = require('@cra/core');
//!
//! // Create a resolver
//! const resolver = new Resolver();
//!
//! // Load an atlas from JSON
//! const atlasJson = JSON.stringify({
//!   atlas_version: "1.0",
//!   atlas_id: "com.example.test",
//!   version: "1.0.0",
//!   name: "Test Atlas",
//!   description: "A test atlas",
//!   domains: ["test"],
//!   capabilities: [],
//!   policies: [],
//!   actions: [
//!     {
//!       action_id: "test.greet",
//!       name: "Greet",
//!       description: "Say hello",
//!       parameters_schema: { type: "object" },
//!       risk_tier: "low"
//!     }
//!   ]
//! });
//!
//! const atlasId = resolver.loadAtlasJson(atlasJson);
//!
//! // Create a session
//! const sessionId = resolver.createSession("my-agent", "Help the user");
//!
//! // Resolve a request
//! const resolution = resolver.resolve(sessionId, "my-agent", "I want to greet someone");
//! console.log(JSON.parse(resolution));
//!
//! // Get the trace
//! const trace = resolver.getTrace(sessionId);
//!
//! // End the session
//! resolver.endSession(sessionId);
//! ```

#[macro_use]
extern crate napi_derive;

use napi::{Error, Result, Status};

use cra_core::{AtlasManifest, CARPRequest, Resolver as CoreResolver};

/// CRA Resolver for Node.js
#[napi]
pub struct Resolver {
    inner: CoreResolver,
}

#[napi]
impl Resolver {
    /// Create a new resolver
    #[napi(constructor)]
    pub fn new() -> Self {
        Resolver {
            inner: CoreResolver::new(),
        }
    }

    /// Load an atlas from a JSON string
    ///
    /// Returns the atlas ID on success
    #[napi]
    pub fn load_atlas_json(&mut self, json: String) -> Result<String> {
        let manifest: AtlasManifest = serde_json::from_str(&json)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to parse atlas JSON: {}", e)))?;

        self.inner
            .load_atlas(manifest)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to load atlas: {}", e)))
    }

    /// Unload an atlas by ID
    #[napi]
    pub fn unload_atlas(&mut self, atlas_id: String) -> Result<()> {
        self.inner
            .unload_atlas(&atlas_id)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to unload atlas: {}", e)))
    }

    /// Create a new session
    ///
    /// Returns the session ID
    #[napi]
    pub fn create_session(&mut self, agent_id: String, goal: String) -> Result<String> {
        self.inner
            .create_session(&agent_id, &goal)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to create session: {}", e)))
    }

    /// End a session
    #[napi]
    pub fn end_session(&mut self, session_id: String) -> Result<()> {
        self.inner
            .end_session(&session_id)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to end session: {}", e)))
    }

    /// Resolve a CARP request
    ///
    /// Returns a JSON string containing the resolution
    #[napi]
    pub fn resolve(&mut self, session_id: String, agent_id: String, goal: String) -> Result<String> {
        let request = CARPRequest::new(session_id, agent_id, goal);

        let resolution = self
            .inner
            .resolve(&request)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to resolve: {}", e)))?;

        serde_json::to_string(&resolution)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize: {}", e)))
    }

    /// Execute an action
    ///
    /// Returns a JSON string containing the result
    #[napi]
    pub fn execute(
        &mut self,
        session_id: String,
        resolution_id: String,
        action_id: String,
        parameters_json: Option<String>,
    ) -> Result<String> {
        let params: serde_json::Value = match parameters_json {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to parse parameters: {}", e)))?,
            None => serde_json::json!({}),
        };

        let result = self
            .inner
            .execute(&session_id, &resolution_id, &action_id, params)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to execute: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize: {}", e)))
    }

    /// Get the trace for a session as JSONL
    #[napi]
    pub fn get_trace(&self, session_id: String) -> Result<String> {
        let events = self
            .inner
            .get_trace(&session_id)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get trace: {}", e)))?;

        let lines: Vec<String> = events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect();

        Ok(lines.join("\n"))
    }

    /// Verify the hash chain for a session
    ///
    /// Returns a JSON string containing the verification result
    #[napi]
    pub fn verify_chain(&self, session_id: String) -> Result<String> {
        let verification = self
            .inner
            .verify_chain(&session_id)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to verify: {}", e)))?;

        serde_json::to_string(&verification)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize: {}", e)))
    }

    /// List all loaded atlas IDs
    #[napi]
    pub fn list_atlases(&self) -> Vec<String> {
        self.inner.list_atlases().iter().map(|s| s.to_string()).collect()
    }
}

/// Get the CRA core version
#[napi]
pub fn version() -> &'static str {
    "0.1.0"
}

/// Get the CARP protocol version
#[napi]
pub fn carp_version() -> &'static str {
    cra_core::CARP_VERSION
}

/// Get the TRACE protocol version
#[napi]
pub fn trace_version() -> &'static str {
    cra_core::TRACE_VERSION
}

/// Get the Atlas format version
#[napi]
pub fn atlas_version() -> &'static str {
    cra_core::ATLAS_VERSION
}
