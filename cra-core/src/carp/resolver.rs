//! CARP Resolver - The core resolution engine
//!
//! The Resolver is the main entry point for CARP operations. It:
//! - Manages atlases and their actions
//! - Creates and tracks sessions
//! - Resolves CARP requests into resolutions
//! - Executes actions and tracks results
//! - Emits TRACE events for all operations

use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::atlas::{AtlasAction, AtlasContextBlock, AtlasManifest, InjectMode};
use crate::context::{ContextRegistry, ContextMatcher, LoadedContext, ContextSource};
use crate::error::{CRAError, Result};
use crate::trace::{DeferredConfig, EventType, TraceCollector, TRACEEvent};

use super::{
    AllowedAction, CARPRequest, CARPResolution, ContextBlock, Constraint, Decision, DeniedAction,
    PolicyEvaluator, PolicyResult,
};

/// Session state
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub session_id: String,
    /// Agent that owns this session
    pub agent_id: String,
    /// Initial goal for the session
    pub goal: String,
    /// When the session was created
    pub created_at: chrono::DateTime<Utc>,
    /// When the session ended (if it has)
    pub ended_at: Option<chrono::DateTime<Utc>>,
    /// Whether the session is still active
    pub is_active: bool,
    /// Number of resolutions in this session
    pub resolution_count: u64,
    /// Number of actions executed in this session
    pub action_count: u64,
}

impl Session {
    /// Create a new session
    pub fn new(session_id: String, agent_id: String, goal: String) -> Self {
        Self {
            session_id,
            agent_id,
            goal,
            created_at: Utc::now(),
            ended_at: None,
            is_active: true,
            resolution_count: 0,
            action_count: 0,
        }
    }

    /// End the session
    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
        self.is_active = false;
    }

    /// Get session duration in milliseconds
    pub fn duration_ms(&self) -> i64 {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        (end - self.created_at).num_milliseconds()
    }
}

/// The main CRA Resolver
///
/// Manages atlases, sessions, and provides CARP resolution.
#[derive(Debug)]
pub struct Resolver {
    /// Loaded atlases by ID
    atlases: HashMap<String, AtlasManifest>,

    /// Active sessions by ID
    sessions: HashMap<String, Session>,

    /// Policy evaluator
    policy_evaluator: PolicyEvaluator,

    /// Context registry for context injection
    context_registry: ContextRegistry,

    /// Context matcher for evaluating conditions
    context_matcher: ContextMatcher,

    /// TRACE collector for audit events
    trace_collector: TraceCollector,

    /// Default TTL for resolutions in seconds
    default_ttl: u64,
}

impl Resolver {
    /// Create a new resolver
    pub fn new() -> Self {
        Self {
            atlases: HashMap::new(),
            sessions: HashMap::new(),
            policy_evaluator: PolicyEvaluator::new(),
            context_registry: ContextRegistry::new(),
            context_matcher: ContextMatcher::new(),
            trace_collector: TraceCollector::new(),
            default_ttl: 300, // 5 minutes
        }
    }

    /// Set the default TTL for resolutions
    pub fn with_default_ttl(mut self, ttl: u64) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Enable deferred tracing mode
    ///
    /// In deferred mode, trace events are queued without computing hashes,
    /// achieving <1µs per event instead of ~15µs. Call `flush_traces()` before
    /// `get_trace()` or `verify_chain()` to ensure all events are processed.
    ///
    /// This is recommended for high-throughput scenarios (agent swarms, benchmarks).
    pub fn with_deferred_tracing(mut self, config: DeferredConfig) -> Self {
        self.trace_collector = TraceCollector::with_deferred(config);
        self
    }

    /// Check if deferred tracing is enabled
    pub fn is_deferred(&self) -> bool {
        self.trace_collector.is_deferred()
    }

    /// Get the number of pending (unprocessed) trace events
    pub fn pending_trace_count(&self) -> usize {
        self.trace_collector.pending_count()
    }

    /// Flush all pending trace events (deferred mode)
    ///
    /// Processes all events in the buffer, computing hashes and chaining them.
    /// In immediate mode, this is a no-op.
    ///
    /// Call this before `get_trace()` or `verify_chain()` when using deferred mode.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let resolver = Resolver::new()
    ///     .with_deferred_tracing(DeferredConfig::default());
    ///
    /// // Fast resolution (no hash computation)
    /// resolver.resolve(&request)?;
    ///
    /// // Flush before querying trace
    /// resolver.flush_traces()?;
    /// let trace = resolver.get_trace(&session_id)?;
    /// ```
    pub fn flush_traces(&mut self) -> Result<()> {
        self.trace_collector.flush()
    }

    /// Check if all trace events have been processed
    pub fn is_traces_flushed(&self) -> bool {
        self.trace_collector.is_flushed()
    }

    /// Load an atlas into the resolver
    pub fn load_atlas(&mut self, atlas: AtlasManifest) -> Result<String> {
        let atlas_id = atlas.atlas_id.clone();

        if self.atlases.contains_key(&atlas_id) {
            return Err(CRAError::AtlasAlreadyLoaded {
                atlas_id: atlas_id.clone(),
            });
        }

        // Add policies from the atlas to the evaluator
        self.policy_evaluator.add_policies(atlas.policies.clone());

        // Load inline context_blocks into the registry
        for block in &atlas.context_blocks {
            // Build conditions from block fields
            let conditions = if block.inject_when.is_empty()
                && block.keywords.is_empty()
                && block.risk_tiers.is_empty()
            {
                None
            } else {
                Some(serde_json::json!({
                    "inject_when": block.inject_when,
                    "keywords": block.keywords,
                    "risk_tiers": block.risk_tiers,
                }))
            };

            let loaded = LoadedContext {
                pack_id: block.context_id.clone(),
                source: ContextSource::Atlas(atlas_id.clone()),
                content: block.content.clone(),
                content_type: block.content_type.clone(),
                priority: block.priority,
                keywords: block.keywords.clone(),
                conditions,
            };

            self.context_registry.add_context(loaded);
        }

        // Load file-based context_packs (placeholder - would need file loader)
        // For now, context_packs with files are not loaded automatically
        // In production, you'd use ContextRegistry::load_from_pack() with a file loader

        self.atlases.insert(atlas_id.clone(), atlas);
        Ok(atlas_id)
    }

    /// Unload an atlas from the resolver
    pub fn unload_atlas(&mut self, atlas_id: &str) -> Result<()> {
        if !self.atlases.contains_key(atlas_id) {
            return Err(CRAError::AtlasNotFound {
                atlas_id: atlas_id.to_string(),
            });
        }

        self.atlases.remove(atlas_id);
        // Note: policies remain - in production you'd want to rebuild
        Ok(())
    }

    /// Get a loaded atlas by ID
    pub fn get_atlas(&self, atlas_id: &str) -> Option<&AtlasManifest> {
        self.atlases.get(atlas_id)
    }

    /// List all loaded atlas IDs
    pub fn list_atlases(&self) -> Vec<&str> {
        self.atlases.keys().map(|s| s.as_str()).collect()
    }

    /// Create a new session
    pub fn create_session(&mut self, agent_id: &str, goal: &str) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();

        if self.sessions.contains_key(&session_id) {
            return Err(CRAError::SessionAlreadyExists {
                session_id: session_id.clone(),
            });
        }

        let session = Session::new(session_id.clone(), agent_id.to_string(), goal.to_string());

        // Emit session.started event
        self.trace_collector.emit(
            &session_id,
            EventType::SessionStarted,
            serde_json::json!({
                "agent_id": agent_id,
                "goal": goal,
                "atlas_ids": self.list_atlases(),
            }),
        )?;

        self.sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }

    /// End a session
    pub fn end_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            CRAError::SessionNotFound {
                session_id: session_id.to_string(),
            }
        })?;

        if !session.is_active {
            return Err(CRAError::SessionAlreadyEnded {
                session_id: session_id.to_string(),
            });
        }

        session.end();

        // Emit session.ended event
        self.trace_collector.emit(
            session_id,
            EventType::SessionEnded,
            serde_json::json!({
                "reason": "completed",
                "duration_ms": session.duration_ms(),
                "resolution_count": session.resolution_count,
                "action_count": session.action_count,
            }),
        )?;

        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Resolve a CARP request
    ///
    /// This is the core resolution function that:
    /// 1. Validates the request
    /// 2. Loads relevant atlases
    /// 3. Evaluates policies for each action
    /// 4. Assembles the resolution with allowed/denied actions
    /// 5. Emits TRACE events
    pub fn resolve(&mut self, request: &CARPRequest) -> Result<CARPResolution> {
        // Validate request
        request.validate().map_err(|e| CRAError::InvalidCARPRequest { reason: e })?;

        // Check session exists and is active
        let session = self.sessions.get_mut(&request.session_id).ok_or_else(|| {
            CRAError::SessionNotFound {
                session_id: request.session_id.clone(),
            }
        })?;

        if !session.is_active {
            return Err(CRAError::SessionAlreadyEnded {
                session_id: request.session_id.clone(),
            });
        }

        // Generate trace ID for this resolution
        let trace_id = Uuid::new_v4().to_string();

        // Emit carp.request.received event
        self.trace_collector.emit(
            &request.session_id,
            EventType::CARPRequestReceived,
            serde_json::json!({
                "request_id": trace_id,
                "operation": "resolve",
                "goal": request.goal,
                "agent_id": request.agent_id,
            }),
        )?;

        // Collect all actions from loaded atlases
        let all_actions: Vec<&AtlasAction> = self
            .atlases
            .values()
            .flat_map(|a| a.actions.iter())
            .collect();

        let mut allowed_actions = Vec::new();
        let mut denied_actions = Vec::new();
        let mut constraints = Vec::new();

        // Evaluate each action against policies
        for action in all_actions {
            let result = self.policy_evaluator.evaluate(&action.action_id);

            // Emit policy.evaluated event
            self.trace_collector.emit(
                &request.session_id,
                EventType::PolicyEvaluated,
                serde_json::json!({
                    "action_id": action.action_id,
                    "result": format!("{:?}", result),
                }),
            )?;

            match result {
                PolicyResult::Deny { policy_id, reason } => {
                    denied_actions.push(DeniedAction::new(
                        action.action_id.clone(),
                        policy_id,
                        reason,
                    ));
                }
                PolicyResult::RequiresApproval { policy_id } => {
                    denied_actions.push(DeniedAction::new(
                        action.action_id.clone(),
                        policy_id,
                        "Requires human approval".to_string(),
                    ));
                }
                PolicyResult::RateLimitExceeded { policy_id, retry_after } => {
                    denied_actions.push(DeniedAction::new(
                        action.action_id.clone(),
                        policy_id,
                        format!("Rate limit exceeded, retry after {} seconds", retry_after),
                    ));
                }
                PolicyResult::Allow | PolicyResult::AllowWithConstraints(_) | PolicyResult::NoMatch => {
                    allowed_actions.push(AllowedAction {
                        action_id: action.action_id.clone(),
                        name: action.name.clone(),
                        description: Some(action.description.clone()),
                        parameters_schema: action.parameters_schema.clone(),
                        risk_tier: action.risk_tier.clone(),
                    });

                    // Add constraints if any
                    if let PolicyResult::AllowWithConstraints(constraint_ids) = result {
                        for constraint_id in constraint_ids {
                            constraints.push(Constraint::new(
                                constraint_id.clone(),
                                crate::carp::ConstraintType::Custom,
                                format!("Constraint from {}", constraint_id),
                            ));
                        }
                    }
                }
            }
        }

        // Determine overall decision
        let decision = if denied_actions.is_empty() && !allowed_actions.is_empty() {
            Decision::Allow
        } else if allowed_actions.is_empty() {
            Decision::Deny
        } else if !constraints.is_empty() {
            Decision::AllowWithConstraints
        } else {
            Decision::Partial
        };

        // Update session stats
        session.resolution_count += 1;

        // Query context registry for matching context based on goal
        let context_hints: Vec<String> = request.context_hints.clone().unwrap_or_default();
        let matching_contexts = self.context_registry.query(&request.goal, None);

        // Convert matching context to ContextBlocks and emit TRACE events
        let mut context_blocks: Vec<ContextBlock> = Vec::new();
        for ctx in matching_contexts {
            // Evaluate conditions with the matcher for fine-grained matching
            let match_result = self.context_matcher.evaluate(
                ctx.conditions.as_ref(),
                &request.goal,
                None, // TODO: Parse risk tier from request if provided
                &context_hints,
                ctx.priority,
            );

            if match_result.matched {
                let block = ctx.to_context_block();

                // Emit context.injected TRACE event
                self.trace_collector.emit(
                    &request.session_id,
                    EventType::ContextInjected,
                    serde_json::json!({
                        "context_id": block.block_id,
                        "source_atlas": block.source_atlas,
                        "priority": block.priority,
                        "content_type": block.content_type,
                        "token_estimate": ctx.token_estimate(),
                        "match_score": match_result.score.total(),
                    }),
                )?;

                context_blocks.push(block);
            }
        }

        // Build resolution with injected context
        let resolution = CARPResolution::builder(request.session_id.clone())
            .trace_id(trace_id.clone())
            .decision(decision)
            .allowed_actions(allowed_actions.clone())
            .denied_actions(denied_actions.clone())
            .constraints(constraints)
            .context_blocks(context_blocks.clone())
            .ttl_seconds(self.default_ttl)
            .build();

        // Emit carp.resolution.completed event
        self.trace_collector.emit(
            &request.session_id,
            EventType::CARPResolutionCompleted,
            serde_json::json!({
                "resolution_id": trace_id,
                "decision_type": resolution.decision.to_string(),
                "allowed_count": allowed_actions.len(),
                "denied_count": denied_actions.len(),
                "context_count": context_blocks.len(),
                "ttl_seconds": self.default_ttl,
            }),
        )?;

        Ok(resolution)
    }

    /// Execute an action within a session
    pub fn execute(
        &mut self,
        session_id: &str,
        resolution_id: &str,
        action_id: &str,
        parameters: Value,
    ) -> Result<Value> {
        // Check session exists and is active
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            CRAError::SessionNotFound {
                session_id: session_id.to_string(),
            }
        })?;

        if !session.is_active {
            return Err(CRAError::SessionAlreadyEnded {
                session_id: session_id.to_string(),
            });
        }

        let execution_id = Uuid::new_v4().to_string();

        // Emit action.requested event
        self.trace_collector.emit(
            session_id,
            EventType::ActionRequested,
            serde_json::json!({
                "action_id": action_id,
                "resolution_id": resolution_id,
                "execution_id": execution_id,
                "parameters_hash": hash_value(&parameters),
            }),
        )?;

        // Re-evaluate policy for this action
        let policy_result = self.policy_evaluator.evaluate(action_id);

        if let PolicyResult::Deny { policy_id, reason } = policy_result {
            // Emit action.denied event
            self.trace_collector.emit(
                session_id,
                EventType::ActionDenied,
                serde_json::json!({
                    "action_id": action_id,
                    "reason": reason,
                    "policy_id": policy_id,
                }),
            )?;

            return Err(CRAError::ActionDenied { policy_id, reason });
        }

        // Find the action definition
        let action = self
            .atlases
            .values()
            .flat_map(|a| a.actions.iter())
            .find(|a| a.action_id == action_id)
            .ok_or_else(|| CRAError::ActionNotFound {
                action_id: action_id.to_string(),
            })?;

        // In a real implementation, you would validate parameters against schema
        // and execute the actual action here. For now, we just record the execution.

        // Emit action.approved event
        self.trace_collector.emit(
            session_id,
            EventType::ActionApproved,
            serde_json::json!({
                "action_id": action_id,
                "resolution_id": resolution_id,
            }),
        )?;

        // Simulate execution
        let start = std::time::Instant::now();

        // Placeholder result - in reality this would come from actual action execution
        let result = serde_json::json!({
            "status": "success",
            "action_id": action_id,
            "message": format!("Action {} executed successfully", action.name),
        });

        let duration_ms = start.elapsed().as_millis() as u64;

        // Update session stats
        session.action_count += 1;

        // Emit action.executed event
        self.trace_collector.emit(
            session_id,
            EventType::ActionExecuted,
            serde_json::json!({
                "action_id": action_id,
                "execution_id": execution_id,
                "duration_ms": duration_ms,
                "result_hash": hash_value(&result),
            }),
        )?;

        Ok(result)
    }

    /// Get the TRACE for a session
    pub fn get_trace(&self, session_id: &str) -> Result<Vec<TRACEEvent>> {
        self.trace_collector.get_events(session_id)
    }

    /// Verify the hash chain integrity for a session
    pub fn verify_chain(&self, session_id: &str) -> Result<crate::trace::ChainVerification> {
        self.trace_collector.verify_chain(session_id)
    }

    /// Get the trace collector (for advanced operations)
    pub fn trace_collector(&self) -> &TraceCollector {
        &self.trace_collector
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a JSON value for audit purposes
fn hash_value(value: &Value) -> String {
    use sha2::{Digest, Sha256};

    let canonical = serde_json::to_string(value).unwrap_or_default();
    let hash = Sha256::digest(canonical.as_bytes());
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_atlas() -> AtlasManifest {
        serde_json::from_value(json!({
            "atlas_version": "1.0",
            "atlas_id": "com.test.resolver",
            "version": "1.0.0",
            "name": "Test Resolver Atlas",
            "description": "Atlas for testing the resolver",
            "domains": ["test"],
            "capabilities": [],
            "policies": [
                {
                    "policy_id": "deny-delete",
                    "type": "deny",
                    "actions": ["*.delete"],
                    "reason": "Deletion not allowed"
                }
            ],
            "actions": [
                {
                    "action_id": "test.get",
                    "name": "Get Test",
                    "description": "Get a test resource",
                    "parameters_schema": { "type": "object" },
                    "risk_tier": "low"
                },
                {
                    "action_id": "test.create",
                    "name": "Create Test",
                    "description": "Create a test resource",
                    "parameters_schema": { "type": "object" },
                    "risk_tier": "medium"
                },
                {
                    "action_id": "test.delete",
                    "name": "Delete Test",
                    "description": "Delete a test resource",
                    "parameters_schema": { "type": "object" },
                    "risk_tier": "high"
                }
            ]
        }))
        .unwrap()
    }

    #[test]
    fn test_load_atlas() {
        let mut resolver = Resolver::new();
        let atlas = create_test_atlas();

        let result = resolver.load_atlas(atlas);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "com.test.resolver");
        assert!(resolver.get_atlas("com.test.resolver").is_some());
    }

    #[test]
    fn test_create_session() {
        let mut resolver = Resolver::new();
        let session_id = resolver.create_session("test-agent", "Test goal").unwrap();

        let session = resolver.get_session(&session_id).unwrap();
        assert_eq!(session.agent_id, "test-agent");
        assert_eq!(session.goal, "Test goal");
        assert!(session.is_active);
    }

    #[test]
    fn test_resolve_request() {
        let mut resolver = Resolver::new();
        resolver.load_atlas(create_test_atlas()).unwrap();

        let session_id = resolver.create_session("test-agent", "Test goal").unwrap();

        let request = CARPRequest::new(
            session_id.clone(),
            "test-agent".to_string(),
            "I want to test things".to_string(),
        );

        let resolution = resolver.resolve(&request).unwrap();

        // test.get and test.create should be allowed
        // test.delete should be denied
        assert!(resolution.is_action_allowed("test.get"));
        assert!(resolution.is_action_allowed("test.create"));
        assert!(!resolution.is_action_allowed("test.delete"));

        // Check denied reason
        let denial = resolution.denied_actions.iter()
            .find(|d| d.action_id == "test.delete");
        assert!(denial.is_some());
    }

    #[test]
    fn test_execute_action() {
        let mut resolver = Resolver::new();
        resolver.load_atlas(create_test_atlas()).unwrap();

        let session_id = resolver.create_session("test-agent", "Test goal").unwrap();

        // Execute allowed action
        let result = resolver.execute(
            &session_id,
            "resolution-1",
            "test.get",
            json!({}),
        );
        assert!(result.is_ok());

        // Execute denied action should fail
        let result = resolver.execute(
            &session_id,
            "resolution-1",
            "test.delete",
            json!({}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_trace_chain() {
        let mut resolver = Resolver::new();
        resolver.load_atlas(create_test_atlas()).unwrap();

        let session_id = resolver.create_session("test-agent", "Test goal").unwrap();

        let request = CARPRequest::new(
            session_id.clone(),
            "test-agent".to_string(),
            "Test goal".to_string(),
        );
        resolver.resolve(&request).unwrap();

        // Verify chain
        let verification = resolver.verify_chain(&session_id).unwrap();
        assert!(verification.is_valid);
    }

    #[test]
    fn test_end_session() {
        let mut resolver = Resolver::new();
        let session_id = resolver.create_session("test-agent", "Test goal").unwrap();

        assert!(resolver.get_session(&session_id).unwrap().is_active);

        resolver.end_session(&session_id).unwrap();

        assert!(!resolver.get_session(&session_id).unwrap().is_active);

        // Trying to end again should fail
        let result = resolver.end_session(&session_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_context_injection() {
        use crate::atlas::AtlasContextBlock;

        // Create an atlas with context_blocks
        let mut atlas: AtlasManifest = serde_json::from_value(json!({
            "atlas_version": "1.0",
            "atlas_id": "com.test.context",
            "version": "1.0.0",
            "name": "Context Test Atlas",
            "description": "Atlas for testing context injection",
            "domains": ["test"],
            "capabilities": [],
            "policies": [],
            "actions": [
                {
                    "action_id": "test.action",
                    "name": "Test Action",
                    "description": "A test action",
                    "parameters_schema": { "type": "object" },
                    "risk_tier": "low"
                }
            ]
        }))
        .unwrap();

        // Add inline context_blocks
        atlas.context_blocks = vec![
            AtlasContextBlock {
                context_id: "hash-rules".to_string(),
                name: "Hash Computation Rules".to_string(),
                priority: 100,
                content: "CRITICAL: Use TRACEEvent::compute_hash()".to_string(),
                content_type: "text/markdown".to_string(),
                inject_mode: InjectMode::OnMatch,
                also_inject: vec![],
                inject_when: vec![],
                keywords: vec!["hash".to_string(), "trace".to_string()],
                risk_tiers: vec![],
            },
            AtlasContextBlock {
                context_id: "test-rules".to_string(),
                name: "Testing Rules".to_string(),
                priority: 50,
                content: "Always run cargo test before committing".to_string(),
                content_type: "text/markdown".to_string(),
                inject_mode: InjectMode::OnMatch,
                also_inject: vec![],
                inject_when: vec![],
                keywords: vec!["test".to_string(), "testing".to_string()],
                risk_tiers: vec![],
            },
        ];

        let mut resolver = Resolver::new();
        resolver.load_atlas(atlas).unwrap();

        let session_id = resolver.create_session("test-agent", "Test context").unwrap();

        // Resolve with goal that matches "hash" keyword
        let request = CARPRequest::new(
            session_id.clone(),
            "test-agent".to_string(),
            "Working on hash chain implementation".to_string(),
        );
        let resolution = resolver.resolve(&request).unwrap();

        // Should have injected hash-rules context
        assert!(!resolution.context_blocks.is_empty(), "Should have injected context");
        assert!(
            resolution.context_blocks.iter().any(|b| b.block_id == "hash-rules"),
            "Should have hash-rules context block"
        );

        // Verify the content
        let hash_block = resolution.context_blocks.iter()
            .find(|b| b.block_id == "hash-rules")
            .unwrap();
        assert!(hash_block.content.contains("compute_hash"));

        // Verify trace includes context.injected event
        let trace = resolver.get_trace(&session_id).unwrap();
        let context_events: Vec<_> = trace.iter()
            .filter(|e| e.event_type == crate::trace::EventType::ContextInjected)
            .collect();
        assert!(!context_events.is_empty(), "Should have context.injected trace events");
    }
}
