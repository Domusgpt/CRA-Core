//! TRACE Event types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::VERSION;

/// A single TRACE event in the audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TRACEEvent {
    /// TRACE protocol version (always "1.0")
    pub trace_version: String,

    /// Unique identifier for this event
    pub event_id: String,

    /// Trace ID grouping related events
    pub trace_id: String,

    /// Span ID for this operation
    pub span_id: String,

    /// Parent span ID for nested operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,

    /// Session this event belongs to
    pub session_id: String,

    /// Monotonically increasing sequence number
    pub sequence: u64,

    /// ISO 8601 timestamp with microsecond precision
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: EventType,

    /// Event-specific payload data
    pub payload: Value,

    /// SHA-256 hash of this event
    pub event_hash: String,

    /// SHA-256 hash of the preceding event
    pub previous_event_hash: String,
}

impl TRACEEvent {
    /// Create a new TRACE event
    pub fn new(
        session_id: String,
        trace_id: String,
        event_type: EventType,
        payload: Value,
    ) -> Self {
        Self {
            trace_version: VERSION.to_string(),
            event_id: Uuid::new_v4().to_string(),
            trace_id,
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            session_id,
            sequence: 0, // Will be set by collector
            timestamp: Utc::now(),
            event_type,
            payload,
            event_hash: String::new(),   // Will be computed by collector
            previous_event_hash: String::new(), // Will be set by collector
        }
    }

    /// Create the genesis event for a session
    pub fn genesis(session_id: String, trace_id: String, payload: Value) -> Self {
        let mut event = Self::new(
            session_id,
            trace_id,
            EventType::SessionStarted,
            payload,
        );
        event.sequence = 0;
        event.previous_event_hash = super::GENESIS_HASH.to_string();
        event.event_hash = event.compute_hash();
        event
    }

    /// Set the parent span
    pub fn with_parent_span(mut self, parent_span_id: String) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// Set the sequence and previous hash, then compute this event's hash
    pub fn chain(mut self, sequence: u64, previous_event_hash: String) -> Self {
        self.sequence = sequence;
        self.previous_event_hash = previous_event_hash;
        self.event_hash = self.compute_hash();
        self
    }

    /// Compute the SHA-256 hash of this event
    ///
    /// Hash is computed over:
    /// trace_version || event_id || trace_id || span_id || parent_span_id ||
    /// session_id || sequence || timestamp || event_type || canonical_json(payload) ||
    /// previous_event_hash
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(self.trace_version.as_bytes());
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.trace_id.as_bytes());
        hasher.update(self.span_id.as_bytes());
        hasher.update(self.parent_span_id.as_deref().unwrap_or("").as_bytes());
        hasher.update(self.session_id.as_bytes());
        hasher.update(self.sequence.to_string().as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.event_type.as_str().as_bytes());
        hasher.update(canonical_json(&self.payload).as_bytes());
        hasher.update(self.previous_event_hash.as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Verify this event's hash
    pub fn verify_hash(&self) -> bool {
        self.event_hash == self.compute_hash()
    }
}

/// Canonical JSON serialization (sorted keys)
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<_> = map.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            let contents: Vec<String> = pairs
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, canonical_json(v)))
                .collect();
            format!("{{{}}}", contents.join(","))
        }
        Value::Array(arr) => {
            let contents: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", contents.join(","))
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Event types defined in TRACE/1.0
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Session events
    #[serde(rename = "session.started")]
    SessionStarted,
    #[serde(rename = "session.ended")]
    SessionEnded,

    // CARP events
    #[serde(rename = "carp.request.received")]
    CARPRequestReceived,
    #[serde(rename = "carp.resolution.completed")]
    CARPResolutionCompleted,
    #[serde(rename = "carp.resolution.cached")]
    CARPResolutionCached,

    // Action events
    #[serde(rename = "action.requested")]
    ActionRequested,
    #[serde(rename = "action.approved")]
    ActionApproved,
    #[serde(rename = "action.denied")]
    ActionDenied,
    #[serde(rename = "action.executed")]
    ActionExecuted,
    #[serde(rename = "action.failed")]
    ActionFailed,

    // Policy events
    #[serde(rename = "policy.evaluated")]
    PolicyEvaluated,
    #[serde(rename = "policy.violated")]
    PolicyViolated,

    // Context events
    #[serde(rename = "context.injected")]
    ContextInjected,
    #[serde(rename = "context.redacted")]
    ContextRedacted,
    #[serde(rename = "context.stale")]
    ContextStale,

    // Checkpoint events
    #[serde(rename = "checkpoint.triggered")]
    CheckpointTriggered,
    #[serde(rename = "checkpoint.question_presented")]
    CheckpointQuestionPresented,
    #[serde(rename = "checkpoint.response_received")]
    CheckpointResponseReceived,
    #[serde(rename = "checkpoint.validated")]
    CheckpointValidated,
    #[serde(rename = "checkpoint.passed")]
    CheckpointPassed,
    #[serde(rename = "checkpoint.failed")]
    CheckpointFailed,
    #[serde(rename = "checkpoint.skipped")]
    CheckpointSkipped,
    #[serde(rename = "checkpoint.guidance_injected")]
    CheckpointGuidanceInjected,

    // Error events
    #[serde(rename = "error.occurred")]
    ErrorOccurred,
}

impl EventType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::SessionStarted => "session.started",
            EventType::SessionEnded => "session.ended",
            EventType::CARPRequestReceived => "carp.request.received",
            EventType::CARPResolutionCompleted => "carp.resolution.completed",
            EventType::CARPResolutionCached => "carp.resolution.cached",
            EventType::ActionRequested => "action.requested",
            EventType::ActionApproved => "action.approved",
            EventType::ActionDenied => "action.denied",
            EventType::ActionExecuted => "action.executed",
            EventType::ActionFailed => "action.failed",
            EventType::PolicyEvaluated => "policy.evaluated",
            EventType::PolicyViolated => "policy.violated",
            EventType::ContextInjected => "context.injected",
            EventType::ContextRedacted => "context.redacted",
            EventType::ContextStale => "context.stale",
            EventType::CheckpointTriggered => "checkpoint.triggered",
            EventType::CheckpointQuestionPresented => "checkpoint.question_presented",
            EventType::CheckpointResponseReceived => "checkpoint.response_received",
            EventType::CheckpointValidated => "checkpoint.validated",
            EventType::CheckpointPassed => "checkpoint.passed",
            EventType::CheckpointFailed => "checkpoint.failed",
            EventType::CheckpointSkipped => "checkpoint.skipped",
            EventType::CheckpointGuidanceInjected => "checkpoint.guidance_injected",
            EventType::ErrorOccurred => "error.occurred",
        }
    }

    /// Check if this is a session event
    pub fn is_session_event(&self) -> bool {
        matches!(self, EventType::SessionStarted | EventType::SessionEnded)
    }

    /// Check if this is a CARP event
    pub fn is_carp_event(&self) -> bool {
        matches!(
            self,
            EventType::CARPRequestReceived
                | EventType::CARPResolutionCompleted
                | EventType::CARPResolutionCached
        )
    }

    /// Check if this is an action event
    pub fn is_action_event(&self) -> bool {
        matches!(
            self,
            EventType::ActionRequested
                | EventType::ActionApproved
                | EventType::ActionDenied
                | EventType::ActionExecuted
                | EventType::ActionFailed
        )
    }

    /// Check if this is a checkpoint event
    pub fn is_checkpoint_event(&self) -> bool {
        matches!(
            self,
            EventType::CheckpointTriggered
                | EventType::CheckpointQuestionPresented
                | EventType::CheckpointResponseReceived
                | EventType::CheckpointValidated
                | EventType::CheckpointPassed
                | EventType::CheckpointFailed
                | EventType::CheckpointSkipped
                | EventType::CheckpointGuidanceInjected
        )
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for EventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "session.started" => Ok(EventType::SessionStarted),
            "session.ended" => Ok(EventType::SessionEnded),
            "carp.request.received" => Ok(EventType::CARPRequestReceived),
            "carp.resolution.completed" => Ok(EventType::CARPResolutionCompleted),
            "carp.resolution.cached" => Ok(EventType::CARPResolutionCached),
            "action.requested" => Ok(EventType::ActionRequested),
            "action.approved" => Ok(EventType::ActionApproved),
            "action.denied" => Ok(EventType::ActionDenied),
            "action.executed" => Ok(EventType::ActionExecuted),
            "action.failed" => Ok(EventType::ActionFailed),
            "policy.evaluated" => Ok(EventType::PolicyEvaluated),
            "policy.violated" => Ok(EventType::PolicyViolated),
            "context.injected" => Ok(EventType::ContextInjected),
            "context.redacted" => Ok(EventType::ContextRedacted),
            "context.stale" => Ok(EventType::ContextStale),
            "checkpoint.triggered" => Ok(EventType::CheckpointTriggered),
            "checkpoint.question_presented" => Ok(EventType::CheckpointQuestionPresented),
            "checkpoint.response_received" => Ok(EventType::CheckpointResponseReceived),
            "checkpoint.validated" => Ok(EventType::CheckpointValidated),
            "checkpoint.passed" => Ok(EventType::CheckpointPassed),
            "checkpoint.failed" => Ok(EventType::CheckpointFailed),
            "checkpoint.skipped" => Ok(EventType::CheckpointSkipped),
            "checkpoint.guidance_injected" => Ok(EventType::CheckpointGuidanceInjected),
            "error.occurred" => Ok(EventType::ErrorOccurred),
            _ => Err(format!("Unknown event type: {}", s)),
        }
    }
}

/// Type-safe event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPayload {
    SessionStarted(SessionStartedPayload),
    SessionEnded(SessionEndedPayload),
    CARPRequest(CARPRequestPayload),
    CARPResolution(CARPResolutionPayload),
    ActionRequested(ActionRequestedPayload),
    ActionExecuted(ActionExecutedPayload),
    ActionDenied(ActionDeniedPayload),
    ActionFailed(ActionFailedPayload),
    PolicyEvaluated(PolicyEvaluatedPayload),
    ContextStale(ContextStalePayload),
    CheckpointTriggered(CheckpointTriggeredPayload),
    CheckpointQuestionPresented(CheckpointQuestionPresentedPayload),
    CheckpointResponseReceived(CheckpointResponseReceivedPayload),
    CheckpointValidated(CheckpointValidatedPayload),
    CheckpointPassed(CheckpointPassedPayload),
    CheckpointFailed(CheckpointFailedPayload),
    CheckpointSkipped(CheckpointSkippedPayload),
    CheckpointGuidanceInjected(CheckpointGuidanceInjectedPayload),
    Generic(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartedPayload {
    pub agent_id: String,
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndedPayload {
    pub reason: String,
    pub duration_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPRequestPayload {
    pub request_id: String,
    pub operation: String,
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CARPResolutionPayload {
    pub resolution_id: String,
    pub decision_type: String,
    pub allowed_count: usize,
    pub denied_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_block_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequestedPayload {
    pub action_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecutedPayload {
    pub action_id: String,
    pub execution_id: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDeniedPayload {
    pub action_id: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFailedPayload {
    pub action_id: String,
    pub error_code: String,
    pub error_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluatedPayload {
    pub policy_id: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStalePayload {
    pub context_id: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified: Option<String>,
}

/// Payload for checkpoint.triggered event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointTriggeredPayload {
    /// The checkpoint ID from the Atlas
    pub checkpoint_id: String,
    /// Human-readable checkpoint name
    pub checkpoint_name: String,
    /// What triggered this checkpoint
    pub trigger_type: String,
    /// Checkpoint mode (blocking, advisory, observational)
    pub mode: String,
    /// Number of questions to be presented
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_count: Option<usize>,
    /// Whether guidance will be injected
    pub has_guidance: bool,
    /// Action that triggered this (if action-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_action_id: Option<String>,
    /// Keyword that triggered this (if keyword-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_keyword: Option<String>,
}

/// Payload for checkpoint.question_presented event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointQuestionPresentedPayload {
    pub checkpoint_id: String,
    pub question_id: String,
    pub question_text: String,
    pub response_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Payload for checkpoint.response_received event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResponseReceivedPayload {
    pub checkpoint_id: String,
    pub question_id: String,
    /// Type of answer provided
    pub answer_type: String,
    /// Hash of the answer (for privacy - actual answer not logged)
    pub answer_hash: String,
    /// Whether the response was empty
    pub is_empty: bool,
}

/// Payload for checkpoint.validated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointValidatedPayload {
    pub checkpoint_id: String,
    pub question_id: String,
    pub validation_passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<Vec<String>>,
    /// Action taken if validation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_on_invalid: Option<String>,
}

/// Payload for checkpoint.passed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPassedPayload {
    pub checkpoint_id: String,
    /// Number of questions answered
    pub questions_answered: usize,
    /// Capabilities unlocked by this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities_unlocked: Option<Vec<String>>,
    /// Capabilities locked by this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities_locked: Option<Vec<String>>,
    /// Actions allowed after this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions_allowed: Option<Vec<String>>,
    /// Actions denied after this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions_denied: Option<Vec<String>>,
    /// Duration to process checkpoint in milliseconds
    pub duration_ms: u64,
}

/// Payload for checkpoint.failed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointFailedPayload {
    pub checkpoint_id: String,
    /// Reason for failure
    pub failure_reason: String,
    /// Question that caused the failure (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_question_id: Option<String>,
    /// Number of retry attempts (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
    /// Action taken due to failure
    pub action_taken: String,
}

/// Payload for checkpoint.skipped event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSkippedPayload {
    pub checkpoint_id: String,
    /// Reason for skipping (e.g., "advisory_mode", "already_completed", "not_applicable")
    pub skip_reason: String,
    /// Original trigger that would have activated the checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_trigger: Option<String>,
}

/// Payload for checkpoint.guidance_injected event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointGuidanceInjectedPayload {
    pub checkpoint_id: String,
    /// Format of the injected guidance
    pub guidance_format: String,
    /// Size of the guidance in bytes
    pub guidance_size_bytes: usize,
    /// Hash of the guidance content
    pub guidance_hash: String,
    /// Context IDs also injected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injected_context_ids: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_creation() {
        let event = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::SessionStarted,
            json!({"agent_id": "agent-1"}),
        );

        assert_eq!(event.session_id, "session-1");
        assert!(matches!(event.event_type, EventType::SessionStarted));
    }

    #[test]
    fn test_genesis_event() {
        let event = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        assert_eq!(event.sequence, 0);
        assert_eq!(event.previous_event_hash, super::super::GENESIS_HASH);
        assert!(!event.event_hash.is_empty());
        assert!(event.verify_hash());
    }

    #[test]
    fn test_hash_verification() {
        let event = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        assert!(event.verify_hash());

        // Modify event - hash should no longer verify
        let mut modified = event.clone();
        modified.payload = json!({"agent_id": "agent-2", "goal": "test"});
        assert!(!modified.verify_hash());
    }

    #[test]
    fn test_event_chaining() {
        let first = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        let second = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CARPRequestReceived,
            json!({"request_id": "req-1", "operation": "resolve", "goal": "test"}),
        )
        .chain(1, first.event_hash.clone());

        assert_eq!(second.sequence, 1);
        assert_eq!(second.previous_event_hash, first.event_hash);
        assert!(second.verify_hash());
    }

    #[test]
    fn test_canonical_json() {
        let value = json!({"b": 2, "a": 1, "c": {"y": 2, "x": 1}});
        let canonical = canonical_json(&value);

        // Keys should be sorted
        assert!(canonical.starts_with("{\"a\":1"));
        assert!(canonical.contains("\"c\":{\"x\":1,\"y\":2}"));
    }

    #[test]
    fn test_event_type_parsing() {
        assert_eq!(
            "session.started".parse::<EventType>().unwrap(),
            EventType::SessionStarted
        );
        assert_eq!(
            "action.executed".parse::<EventType>().unwrap(),
            EventType::ActionExecuted
        );
        assert!("unknown.event".parse::<EventType>().is_err());
    }

    #[test]
    fn test_context_stale_event() {
        let payload = json!({
            "context_id": "ctx-123",
            "reason": "source_file_changed",
            "source_file": "/path/to/file.rs",
            "last_verified": "2025-12-29T10:00:00Z"
        });

        let event = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::ContextStale,
            payload.clone(),
        );

        assert_eq!(event.session_id, "session-1");
        assert!(matches!(event.event_type, EventType::ContextStale));
        assert_eq!(event.payload, payload);
    }

    #[test]
    fn test_context_stale_event_parsing() {
        assert_eq!(
            "context.stale".parse::<EventType>().unwrap(),
            EventType::ContextStale
        );
        assert_eq!(EventType::ContextStale.as_str(), "context.stale");
    }

    #[test]
    fn test_context_stale_event_hash() {
        let payload = json!({
            "context_id": "ctx-123",
            "reason": "ttl_expired"
        });

        let event = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            payload,
        );

        let stale_event = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::ContextStale,
            json!({
                "context_id": "ctx-456",
                "reason": "source_file_changed",
                "source_file": "/updated/file.rs"
            }),
        )
        .chain(1, event.event_hash.clone());

        assert_eq!(stale_event.sequence, 1);
        assert_eq!(stale_event.previous_event_hash, event.event_hash);
        assert!(stale_event.verify_hash());
    }

    #[test]
    fn test_context_stale_payload_serialization() {
        let payload = ContextStalePayload {
            context_id: "ctx-789".to_string(),
            reason: "source_file_changed".to_string(),
            source_file: Some("/path/to/changed.rs".to_string()),
            last_verified: Some("2025-12-29T12:00:00Z".to_string()),
        };

        let json_value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json_value["context_id"], "ctx-789");
        assert_eq!(json_value["reason"], "source_file_changed");
        assert_eq!(json_value["source_file"], "/path/to/changed.rs");
        assert_eq!(json_value["last_verified"], "2025-12-29T12:00:00Z");
    }

    #[test]
    fn test_checkpoint_event_types_parsing() {
        // Test all checkpoint event type parsing
        assert_eq!(
            "checkpoint.triggered".parse::<EventType>().unwrap(),
            EventType::CheckpointTriggered
        );
        assert_eq!(
            "checkpoint.question_presented".parse::<EventType>().unwrap(),
            EventType::CheckpointQuestionPresented
        );
        assert_eq!(
            "checkpoint.response_received".parse::<EventType>().unwrap(),
            EventType::CheckpointResponseReceived
        );
        assert_eq!(
            "checkpoint.validated".parse::<EventType>().unwrap(),
            EventType::CheckpointValidated
        );
        assert_eq!(
            "checkpoint.passed".parse::<EventType>().unwrap(),
            EventType::CheckpointPassed
        );
        assert_eq!(
            "checkpoint.failed".parse::<EventType>().unwrap(),
            EventType::CheckpointFailed
        );
        assert_eq!(
            "checkpoint.skipped".parse::<EventType>().unwrap(),
            EventType::CheckpointSkipped
        );
        assert_eq!(
            "checkpoint.guidance_injected".parse::<EventType>().unwrap(),
            EventType::CheckpointGuidanceInjected
        );
    }

    #[test]
    fn test_checkpoint_event_types_as_str() {
        assert_eq!(EventType::CheckpointTriggered.as_str(), "checkpoint.triggered");
        assert_eq!(EventType::CheckpointQuestionPresented.as_str(), "checkpoint.question_presented");
        assert_eq!(EventType::CheckpointResponseReceived.as_str(), "checkpoint.response_received");
        assert_eq!(EventType::CheckpointValidated.as_str(), "checkpoint.validated");
        assert_eq!(EventType::CheckpointPassed.as_str(), "checkpoint.passed");
        assert_eq!(EventType::CheckpointFailed.as_str(), "checkpoint.failed");
        assert_eq!(EventType::CheckpointSkipped.as_str(), "checkpoint.skipped");
        assert_eq!(EventType::CheckpointGuidanceInjected.as_str(), "checkpoint.guidance_injected");
    }

    #[test]
    fn test_is_checkpoint_event() {
        assert!(EventType::CheckpointTriggered.is_checkpoint_event());
        assert!(EventType::CheckpointQuestionPresented.is_checkpoint_event());
        assert!(EventType::CheckpointPassed.is_checkpoint_event());
        assert!(EventType::CheckpointFailed.is_checkpoint_event());
        assert!(EventType::CheckpointGuidanceInjected.is_checkpoint_event());

        // Non-checkpoint events should return false
        assert!(!EventType::SessionStarted.is_checkpoint_event());
        assert!(!EventType::ActionExecuted.is_checkpoint_event());
        assert!(!EventType::PolicyEvaluated.is_checkpoint_event());
    }

    #[test]
    fn test_checkpoint_triggered_event() {
        let payload = CheckpointTriggeredPayload {
            checkpoint_id: "onboarding-ack".to_string(),
            checkpoint_name: "Onboarding Acknowledgment".to_string(),
            trigger_type: "session_start".to_string(),
            mode: "blocking".to_string(),
            question_count: Some(2),
            has_guidance: true,
            trigger_action_id: None,
            trigger_keyword: None,
        };

        let json_value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json_value["checkpoint_id"], "onboarding-ack");
        assert_eq!(json_value["mode"], "blocking");
        assert_eq!(json_value["question_count"], 2);
        assert_eq!(json_value["has_guidance"], true);
    }

    #[test]
    fn test_checkpoint_triggered_event_chain() {
        let first = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        let checkpoint_event = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CheckpointTriggered,
            json!({
                "checkpoint_id": "safety-check",
                "checkpoint_name": "Safety Check",
                "trigger_type": "action_pre",
                "mode": "blocking",
                "has_guidance": false,
                "trigger_action_id": "user.delete"
            }),
        )
        .chain(1, first.event_hash.clone());

        assert_eq!(checkpoint_event.sequence, 1);
        assert!(checkpoint_event.verify_hash());
        assert!(checkpoint_event.event_type.is_checkpoint_event());
    }

    #[test]
    fn test_checkpoint_passed_payload() {
        let payload = CheckpointPassedPayload {
            checkpoint_id: "data-access-approval".to_string(),
            questions_answered: 3,
            capabilities_unlocked: Some(vec!["sensitive-data-read".to_string()]),
            capabilities_locked: None,
            actions_allowed: Some(vec!["database.query".to_string()]),
            actions_denied: None,
            duration_ms: 1250,
        };

        let json_value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json_value["checkpoint_id"], "data-access-approval");
        assert_eq!(json_value["questions_answered"], 3);
        assert_eq!(json_value["duration_ms"], 1250);
        assert!(json_value.get("capabilities_locked").is_none()); // Should be skipped
    }

    #[test]
    fn test_checkpoint_failed_payload() {
        let payload = CheckpointFailedPayload {
            checkpoint_id: "compliance-check".to_string(),
            failure_reason: "validation_failed".to_string(),
            failed_question_id: Some("q-compliance-ack".to_string()),
            retry_count: Some(3),
            action_taken: "block_session".to_string(),
        };

        let json_value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json_value["checkpoint_id"], "compliance-check");
        assert_eq!(json_value["failure_reason"], "validation_failed");
        assert_eq!(json_value["action_taken"], "block_session");
    }

    #[test]
    fn test_checkpoint_guidance_injected_payload() {
        let payload = CheckpointGuidanceInjectedPayload {
            checkpoint_id: "pii-handling".to_string(),
            guidance_format: "markdown".to_string(),
            guidance_size_bytes: 2048,
            guidance_hash: "abc123def456".to_string(),
            injected_context_ids: Some(vec![
                "ctx-pii-policy".to_string(),
                "ctx-data-handling".to_string(),
            ]),
        };

        let json_value = serde_json::to_value(&payload).unwrap();
        assert_eq!(json_value["checkpoint_id"], "pii-handling");
        assert_eq!(json_value["guidance_format"], "markdown");
        assert_eq!(json_value["guidance_size_bytes"], 2048);
    }

    #[test]
    fn test_checkpoint_event_full_chain() {
        // Simulate a complete checkpoint flow in TRACE
        let genesis = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "process customer data"}),
        );

        let triggered = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CheckpointTriggered,
            json!({
                "checkpoint_id": "data-consent",
                "checkpoint_name": "Data Consent Verification",
                "trigger_type": "session_start",
                "mode": "blocking",
                "question_count": 1,
                "has_guidance": true
            }),
        )
        .chain(1, genesis.event_hash.clone());

        let question = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CheckpointQuestionPresented,
            json!({
                "checkpoint_id": "data-consent",
                "question_id": "q-consent",
                "question_text": "Do you acknowledge the data handling requirements?",
                "response_type": "boolean",
                "required": true
            }),
        )
        .chain(2, triggered.event_hash.clone());

        let response = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CheckpointResponseReceived,
            json!({
                "checkpoint_id": "data-consent",
                "question_id": "q-consent",
                "answer_type": "boolean",
                "answer_hash": "c3ab8ff13720e8ad9047dd39466b3c8974e592c2fa383d4a3960714caef0c4f2",
                "is_empty": false
            }),
        )
        .chain(3, question.event_hash.clone());

        let validated = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CheckpointValidated,
            json!({
                "checkpoint_id": "data-consent",
                "question_id": "q-consent",
                "validation_passed": true
            }),
        )
        .chain(4, response.event_hash.clone());

        let passed = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            EventType::CheckpointPassed,
            json!({
                "checkpoint_id": "data-consent",
                "questions_answered": 1,
                "capabilities_unlocked": ["customer-data-access"],
                "duration_ms": 850
            }),
        )
        .chain(5, validated.event_hash.clone());

        // Verify the chain
        assert!(genesis.verify_hash());
        assert!(triggered.verify_hash());
        assert!(question.verify_hash());
        assert!(response.verify_hash());
        assert!(validated.verify_hash());
        assert!(passed.verify_hash());

        // Verify chain linkage
        assert_eq!(triggered.previous_event_hash, genesis.event_hash);
        assert_eq!(question.previous_event_hash, triggered.event_hash);
        assert_eq!(response.previous_event_hash, question.event_hash);
        assert_eq!(validated.previous_event_hash, response.event_hash);
        assert_eq!(passed.previous_event_hash, validated.event_hash);
    }
}
