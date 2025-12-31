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
}
