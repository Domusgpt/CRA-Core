use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TRACEEvent {
    pub trace_version: String,
    pub event_id: String,
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub session_id: String,
    pub sequence: u64,
    pub timestamp: String,
    pub event_type: TRACEEventType,
    pub payload: serde_json::Value,
    pub event_hash: String,
    pub previous_event_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TRACEEventType {
    // Session lifecycle
    SessionStarted,
    SessionEnded,

    // CARP events
    CarpRequestReceived,
    CarpResolutionCompleted,

    // Action events
    ActionRequested,
    ActionApproved,
    ActionDenied,
    ActionExecuted,
    ActionFailed,

    // Context events
    ContextInjected,
    ContextExpired,

    // Policy events
    PolicyEvaluated,
    PolicyViolated,

    // Custom
    Custom(String),
}
