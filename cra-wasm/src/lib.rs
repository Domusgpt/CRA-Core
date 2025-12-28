//! CRA WebAssembly bindings via wasm-bindgen
//!
//! This module provides WebAssembly bindings for the CRA Core library,
//! allowing browser and edge applications to use CRA for agent governance.
//!
//! ## Example
//!
//! ```javascript
//! import init, { Resolver, version, carp_version } from '@cra/wasm';
//!
//! async function main() {
//!   // Initialize the WASM module
//!   await init();
//!
//!   // Create a resolver
//!   const resolver = new Resolver();
//!
//!   // Load an atlas from JSON
//!   const atlasJson = JSON.stringify({
//!     atlas_version: "1.0",
//!     atlas_id: "com.example.test",
//!     version: "1.0.0",
//!     name: "Test Atlas",
//!     description: "A test atlas",
//!     domains: ["test"],
//!     capabilities: [],
//!     policies: [],
//!     actions: [
//!       {
//!         action_id: "test.greet",
//!         name: "Greet",
//!         description: "Say hello",
//!         parameters_schema: { type: "object" },
//!         risk_tier: "low"
//!       }
//!     ]
//!   });
//!
//!   const atlasId = resolver.load_atlas_json(atlasJson);
//!
//!   // Create a session
//!   const sessionId = resolver.create_session("my-agent", "Help the user");
//!
//!   // Resolve a request
//!   const resolution = resolver.resolve(sessionId, "my-agent", "I want to greet someone");
//!   console.log(JSON.parse(resolution));
//!
//!   // Get the trace
//!   const trace = resolver.get_trace(sessionId);
//!
//!   // Verify chain integrity
//!   const verification = resolver.verify_chain(sessionId);
//!
//!   // End the session
//!   resolver.end_session(sessionId);
//!
//!   // Check versions
//!   console.log("CRA version:", version());
//!   console.log("CARP version:", carp_version());
//! }
//!
//! main();
//! ```

use wasm_bindgen::prelude::*;

use cra_core::{AtlasManifest, CARPRequest, Resolver as CoreResolver};

// Set up panic hook for better error messages
#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// CRA Resolver for WebAssembly
#[wasm_bindgen]
pub struct Resolver {
    inner: CoreResolver,
}

#[wasm_bindgen]
impl Resolver {
    /// Create a new resolver
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Resolver {
            inner: CoreResolver::new(),
        }
    }

    /// Load an atlas from a JSON string
    ///
    /// Returns the atlas ID on success
    #[wasm_bindgen]
    pub fn load_atlas_json(&mut self, json: &str) -> Result<String, JsError> {
        let manifest: AtlasManifest = serde_json::from_str(json)
            .map_err(|e| JsError::new(&format!("Failed to parse atlas JSON: {}", e)))?;

        self.inner
            .load_atlas(manifest)
            .map_err(|e| JsError::new(&format!("Failed to load atlas: {}", e)))
    }

    /// Unload an atlas by ID
    #[wasm_bindgen]
    pub fn unload_atlas(&mut self, atlas_id: &str) -> Result<(), JsError> {
        self.inner
            .unload_atlas(atlas_id)
            .map_err(|e| JsError::new(&format!("Failed to unload atlas: {}", e)))
    }

    /// Create a new session
    ///
    /// Returns the session ID
    #[wasm_bindgen]
    pub fn create_session(&mut self, agent_id: &str, goal: &str) -> Result<String, JsError> {
        self.inner
            .create_session(agent_id, goal)
            .map_err(|e| JsError::new(&format!("Failed to create session: {}", e)))
    }

    /// End a session
    #[wasm_bindgen]
    pub fn end_session(&mut self, session_id: &str) -> Result<(), JsError> {
        self.inner
            .end_session(session_id)
            .map_err(|e| JsError::new(&format!("Failed to end session: {}", e)))
    }

    /// Resolve a CARP request
    ///
    /// Returns a JSON string containing the resolution
    #[wasm_bindgen]
    pub fn resolve(
        &mut self,
        session_id: &str,
        agent_id: &str,
        goal: &str,
    ) -> Result<String, JsError> {
        let request = CARPRequest::new(
            session_id.to_string(),
            agent_id.to_string(),
            goal.to_string(),
        );

        let resolution = self
            .inner
            .resolve(&request)
            .map_err(|e| JsError::new(&format!("Failed to resolve: {}", e)))?;

        serde_json::to_string(&resolution)
            .map_err(|e| JsError::new(&format!("Failed to serialize: {}", e)))
    }

    /// Execute an action
    ///
    /// Returns a JSON string containing the result
    #[wasm_bindgen]
    pub fn execute(
        &mut self,
        session_id: &str,
        resolution_id: &str,
        action_id: &str,
        parameters_json: Option<String>,
    ) -> Result<String, JsError> {
        let params: serde_json::Value = match parameters_json {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| JsError::new(&format!("Failed to parse parameters: {}", e)))?,
            None => serde_json::json!({}),
        };

        let result = self
            .inner
            .execute(session_id, resolution_id, action_id, params)
            .map_err(|e| JsError::new(&format!("Failed to execute: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Failed to serialize: {}", e)))
    }

    /// Get the trace for a session as JSONL
    #[wasm_bindgen]
    pub fn get_trace(&self, session_id: &str) -> Result<String, JsError> {
        let events = self
            .inner
            .get_trace(session_id)
            .map_err(|e| JsError::new(&format!("Failed to get trace: {}", e)))?;

        let lines: Vec<String> = events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect();

        Ok(lines.join("\n"))
    }

    /// Verify the hash chain for a session
    ///
    /// Returns a JSON string containing the verification result
    #[wasm_bindgen]
    pub fn verify_chain(&self, session_id: &str) -> Result<String, JsError> {
        let verification = self
            .inner
            .verify_chain(session_id)
            .map_err(|e| JsError::new(&format!("Failed to verify: {}", e)))?;

        serde_json::to_string(&verification)
            .map_err(|e| JsError::new(&format!("Failed to serialize: {}", e)))
    }

    /// List all loaded atlas IDs
    #[wasm_bindgen]
    pub fn list_atlases(&self) -> Vec<String> {
        self.inner.list_atlases().iter().map(|s| s.to_string()).collect()
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the CRA core version
#[wasm_bindgen]
pub fn version() -> String {
    "0.1.0".to_string()
}

/// Get the CARP protocol version
#[wasm_bindgen]
pub fn carp_version() -> String {
    cra_core::CARP_VERSION.to_string()
}

/// Get the TRACE protocol version
#[wasm_bindgen]
pub fn trace_version() -> String {
    cra_core::TRACE_VERSION.to_string()
}

/// Get the Atlas format version
#[wasm_bindgen]
pub fn atlas_version() -> String {
    cra_core::ATLAS_VERSION.to_string()
}
