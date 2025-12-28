//! TRACE Replay Engine
//!
//! Provides deterministic replay of trace events and diff generation
//! for comparing traces.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::atlas::AtlasManifest;
use crate::error::{CRAError, Result};

use super::event::{EventType, TRACEEvent};
use super::chain::ChainVerifier;

/// Result of replaying a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Whether replay was successful
    pub success: bool,

    /// Number of events replayed
    pub events_replayed: usize,

    /// Events that failed to replay
    pub failures: Vec<ReplayFailure>,

    /// State reconstructed from the trace
    pub final_state: ReplayState,

    /// Statistics about the replay
    pub stats: ReplayStats,
}

impl ReplayResult {
    /// Create a successful replay result
    pub fn success(events_replayed: usize, state: ReplayState, stats: ReplayStats) -> Self {
        Self {
            success: true,
            events_replayed,
            failures: vec![],
            final_state: state,
            stats,
        }
    }

    /// Create a failed replay result
    pub fn failure(
        events_replayed: usize,
        failures: Vec<ReplayFailure>,
        state: ReplayState,
        stats: ReplayStats,
    ) -> Self {
        Self {
            success: false,
            events_replayed,
            failures,
            final_state: state,
            stats,
        }
    }
}

/// A failure that occurred during replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFailure {
    /// Event index where failure occurred
    pub event_index: usize,

    /// Event that failed
    pub event_type: String,

    /// Error message
    pub error: String,

    /// Whether this failure is recoverable
    pub recoverable: bool,
}

/// Reconstructed state from replay
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayState {
    /// Session information
    pub session: Option<SessionState>,

    /// Resolutions created during the session
    pub resolutions: Vec<ResolutionState>,

    /// Actions executed during the session
    pub actions: Vec<ActionState>,

    /// Policies that were evaluated
    pub policy_evaluations: Vec<PolicyEvaluationState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub agent_id: String,
    pub goal: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub end_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionState {
    pub resolution_id: String,
    pub decision_type: String,
    pub allowed_count: usize,
    pub denied_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionState {
    pub action_id: String,
    pub execution_id: Option<String>,
    pub status: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationState {
    pub policy_id: String,
    pub action_id: Option<String>,
    pub result: String,
}

/// Statistics from replay
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayStats {
    /// Total events processed
    pub total_events: usize,

    /// Events by type
    pub events_by_type: HashMap<String, usize>,

    /// Total duration in milliseconds
    pub total_duration_ms: u64,

    /// Number of successful actions
    pub successful_actions: usize,

    /// Number of failed actions
    pub failed_actions: usize,

    /// Number of denied actions
    pub denied_actions: usize,
}

/// Difference between two traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayDiff {
    /// Whether the traces are identical
    pub identical: bool,

    /// Events only in the first trace
    pub only_in_first: Vec<EventSummary>,

    /// Events only in the second trace
    pub only_in_second: Vec<EventSummary>,

    /// Events that differ between traces
    pub differences: Vec<EventDifference>,

    /// Summary of changes
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub index: usize,
    pub event_type: String,
    pub event_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDifference {
    pub index: usize,
    pub event_type: String,
    pub field: String,
    pub first_value: Value,
    pub second_value: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffSummary {
    pub first_count: usize,
    pub second_count: usize,
    pub common_prefix_length: usize,
    pub divergence_point: Option<usize>,
}

/// TRACE Replay Engine
pub struct ReplayEngine {
    /// Loaded atlases for action validation
    atlases: Vec<AtlasManifest>,
}

impl ReplayEngine {
    /// Create a new replay engine
    pub fn new() -> Self {
        Self { atlases: vec![] }
    }

    /// Add an atlas for action validation
    pub fn with_atlas(mut self, atlas: AtlasManifest) -> Self {
        self.atlases.push(atlas);
        self
    }

    /// Replay a trace and reconstruct state
    pub fn replay(&self, events: &[TRACEEvent]) -> Result<ReplayResult> {
        // First verify chain integrity
        let chain_verification = ChainVerifier::verify(events);
        if !chain_verification.is_valid {
            return Err(CRAError::TraceChainIntegrityError {
                reason: chain_verification
                    .error_message
                    .unwrap_or_else(|| "Chain verification failed".to_string()),
            });
        }

        let mut state = ReplayState::default();
        let mut stats = ReplayStats::default();
        let mut failures = vec![];

        for (i, event) in events.iter().enumerate() {
            stats.total_events += 1;
            *stats
                .events_by_type
                .entry(event.event_type.to_string())
                .or_insert(0) += 1;

            match self.process_event(event, &mut state) {
                Ok(()) => {}
                Err(e) => {
                    failures.push(ReplayFailure {
                        event_index: i,
                        event_type: event.event_type.to_string(),
                        error: e.to_string(),
                        recoverable: e.is_recoverable(),
                    });
                }
            }
        }

        // Calculate stats from state
        stats.successful_actions = state
            .actions
            .iter()
            .filter(|a| a.status == "executed")
            .count();
        stats.failed_actions = state
            .actions
            .iter()
            .filter(|a| a.status == "failed")
            .count();
        stats.denied_actions = state
            .actions
            .iter()
            .filter(|a| a.status == "denied")
            .count();

        if failures.is_empty() {
            Ok(ReplayResult::success(events.len(), state, stats))
        } else {
            Ok(ReplayResult::failure(events.len(), failures, state, stats))
        }
    }

    /// Process a single event during replay
    fn process_event(&self, event: &TRACEEvent, state: &mut ReplayState) -> Result<()> {
        match event.event_type {
            EventType::SessionStarted => {
                let agent_id = event
                    .payload
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let goal = event
                    .payload
                    .get("goal")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                state.session = Some(SessionState {
                    session_id: event.session_id.clone(),
                    agent_id,
                    goal,
                    started_at: event.timestamp.to_rfc3339(),
                    ended_at: None,
                    end_reason: None,
                });
            }
            EventType::SessionEnded => {
                if let Some(ref mut session) = state.session {
                    session.ended_at = Some(event.timestamp.to_rfc3339());
                    session.end_reason = event
                        .payload
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
            EventType::CARPResolutionCompleted => {
                let resolution_id = event
                    .payload
                    .get("resolution_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let decision_type = event
                    .payload
                    .get("decision_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let allowed_count = event
                    .payload
                    .get("allowed_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let denied_count = event
                    .payload
                    .get("denied_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                state.resolutions.push(ResolutionState {
                    resolution_id,
                    decision_type,
                    allowed_count,
                    denied_count,
                });
            }
            EventType::ActionRequested => {
                let action_id = event
                    .payload
                    .get("action_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                state.actions.push(ActionState {
                    action_id,
                    execution_id: event
                        .payload
                        .get("execution_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    status: "requested".to_string(),
                    duration_ms: None,
                });
            }
            EventType::ActionExecuted => {
                let action_id = event
                    .payload
                    .get("action_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let duration_ms = event
                    .payload
                    .get("duration_ms")
                    .and_then(|v| v.as_u64());

                if let Some(action) = state.actions.iter_mut().rev().find(|a| a.action_id == action_id) {
                    action.status = "executed".to_string();
                    action.duration_ms = duration_ms;
                }
            }
            EventType::ActionDenied => {
                let action_id = event
                    .payload
                    .get("action_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if let Some(action) = state.actions.iter_mut().rev().find(|a| a.action_id == action_id) {
                    action.status = "denied".to_string();
                }
            }
            EventType::ActionFailed => {
                let action_id = event
                    .payload
                    .get("action_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if let Some(action) = state.actions.iter_mut().rev().find(|a| a.action_id == action_id) {
                    action.status = "failed".to_string();
                }
            }
            EventType::PolicyEvaluated => {
                let policy_id = event
                    .payload
                    .get("policy_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let result = event
                    .payload
                    .get("result")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let action_id = event
                    .payload
                    .get("action_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                state.policy_evaluations.push(PolicyEvaluationState {
                    policy_id,
                    action_id,
                    result,
                });
            }
            _ => {
                // Other event types don't affect state reconstruction
            }
        }

        Ok(())
    }

    /// Compare two traces and generate a diff
    pub fn diff(&self, first: &[TRACEEvent], second: &[TRACEEvent]) -> ReplayDiff {
        let divergence = ChainVerifier::find_divergence(first, second);

        let common_prefix_length = divergence.unwrap_or(first.len().min(second.len()));

        let only_in_first: Vec<EventSummary> = first
            .iter()
            .enumerate()
            .skip(common_prefix_length)
            .map(|(i, e)| EventSummary {
                index: i,
                event_type: e.event_type.to_string(),
                event_hash: e.event_hash.clone(),
            })
            .collect();

        let only_in_second: Vec<EventSummary> = second
            .iter()
            .enumerate()
            .skip(common_prefix_length)
            .map(|(i, e)| EventSummary {
                index: i,
                event_type: e.event_type.to_string(),
                event_hash: e.event_hash.clone(),
            })
            .collect();

        // Find differences in common events
        let mut differences = vec![];
        for i in 0..common_prefix_length {
            if first[i].event_hash != second[i].event_hash {
                // Find specific field differences
                if first[i].payload != second[i].payload {
                    differences.push(EventDifference {
                        index: i,
                        event_type: first[i].event_type.to_string(),
                        field: "payload".to_string(),
                        first_value: first[i].payload.clone(),
                        second_value: second[i].payload.clone(),
                    });
                }
            }
        }

        let identical = only_in_first.is_empty()
            && only_in_second.is_empty()
            && differences.is_empty();

        ReplayDiff {
            identical,
            only_in_first,
            only_in_second,
            differences,
            summary: DiffSummary {
                first_count: first.len(),
                second_count: second.len(),
                common_prefix_length,
                divergence_point: divergence,
            },
        }
    }
}

impl Default for ReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_trace() -> Vec<TRACEEvent> {
        let first = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        let second = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CARPResolutionCompleted,
            json!({
                "resolution_id": "res-1",
                "decision_type": "allow",
                "allowed_count": 3,
                "denied_count": 1
            }),
        )
        .chain(1, first.event_hash.clone());

        let third = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::ActionExecuted,
            json!({
                "action_id": "test.get",
                "execution_id": "exec-1",
                "duration_ms": 100
            }),
        )
        .chain(2, second.event_hash.clone());

        let fourth = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::SessionEnded,
            json!({"reason": "completed", "duration_ms": 1000}),
        )
        .chain(3, third.event_hash.clone());

        vec![first, second, third, fourth]
    }

    #[test]
    fn test_replay() {
        let trace = create_test_trace();
        let engine = ReplayEngine::new();

        let result = engine.replay(&trace).unwrap();

        assert!(result.success);
        assert_eq!(result.events_replayed, 4);

        let state = &result.final_state;
        assert!(state.session.is_some());
        assert_eq!(state.session.as_ref().unwrap().agent_id, "agent-1");
        assert_eq!(state.resolutions.len(), 1);
        assert_eq!(state.resolutions[0].decision_type, "allow");
    }

    #[test]
    fn test_diff_identical() {
        let trace = create_test_trace();
        let engine = ReplayEngine::new();

        let diff = engine.diff(&trace, &trace);

        assert!(diff.identical);
        assert!(diff.only_in_first.is_empty());
        assert!(diff.only_in_second.is_empty());
        assert!(diff.differences.is_empty());
    }

    #[test]
    fn test_diff_different() {
        let trace1 = create_test_trace();
        let mut trace2 = trace1.clone(); // Clone to get identical traces

        // Modify second trace
        trace2[2].payload = json!({
            "action_id": "test.create",
            "execution_id": "exec-2",
            "duration_ms": 200
        });
        trace2[2].event_hash = trace2[2].compute_hash();

        let engine = ReplayEngine::new();
        let diff = engine.diff(&trace1, &trace2);

        assert!(!diff.identical);
        assert_eq!(diff.summary.divergence_point, Some(2));
    }

    #[test]
    fn test_replay_stats() {
        let trace = create_test_trace();
        let engine = ReplayEngine::new();

        let result = engine.replay(&trace).unwrap();

        assert_eq!(result.stats.total_events, 4);
        assert!(result.stats.events_by_type.contains_key("session.started"));
        assert!(result.stats.events_by_type.contains_key("session.ended"));
    }
}
