//! Bootstrap Protocol implementation
//!
//! The bootstrap protocol establishes governance through a handshake where:
//! 1. Agent declares capabilities (INIT)
//! 2. CRA sends governance rules (GOVERNANCE)
//! 3. Agent acknowledges (ACK)
//! 4. CRA streams context (CONTEXT)
//! 5. Agent signals ready (READY)
//! 6. CRA confirms session (SESSION)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{McpError, McpResult};
use crate::session::Session;

/// Bootstrap protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum BootstrapMessage {
    /// Agent → CRA: Initial capabilities and intent
    Init(InitMessage),

    /// CRA → Agent: Governance rules to acknowledge
    Governance(GovernanceMessage),

    /// Agent → CRA: Acknowledge governance rules
    Ack(AckMessage),

    /// CRA → Agent: Context blocks (may be streamed)
    Context(ContextMessage),

    /// Agent → CRA: Wrapper built, ready to work
    Ready(ReadyMessage),

    /// CRA → Agent: Session confirmed, begin work
    Session(SessionMessage),
}

/// INIT message - agent declares itself
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitMessage {
    /// Agent identifier
    pub agent_id: String,

    /// Agent capabilities
    pub capabilities: AgentCapabilities,

    /// What the agent intends to do
    pub intent: String,
}

/// Agent capabilities declared during INIT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Available tools
    #[serde(default)]
    pub tools: Vec<String>,

    /// Supported protocols
    #[serde(default)]
    pub protocols: Vec<String>,

    /// Context window size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,

    /// Additional capability metadata
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// GOVERNANCE message - CRA sends rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceMessage {
    /// Session ID for this governance context
    pub session_id: String,

    /// Genesis hash (start of the chain)
    pub genesis_hash: String,

    /// Governance rules the agent must follow
    pub rules: Vec<GovernanceRule>,

    /// Policies that will be enforced
    pub policies: Vec<PolicySummary>,

    /// Whether acknowledgment is required
    #[serde(default = "default_true")]
    pub acknowledgment_required: bool,
}

fn default_true() -> bool { true }

/// A governance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRule {
    /// Rule identifier
    pub rule_id: String,

    /// Human-readable description
    pub description: String,

    /// Enforcement level: "hard" (enforced), "soft" (encouraged)
    #[serde(default = "default_soft")]
    pub enforcement: String,
}

fn default_soft() -> String { "soft".to_string() }

/// Summary of a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    /// Policy identifier
    pub policy_id: String,

    /// Description of what the policy does
    pub description: String,

    /// Actions affected by this policy
    #[serde(default)]
    pub actions_affected: Vec<String>,
}

/// ACK message - agent acknowledges governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    /// Session ID
    pub session_id: String,

    /// Previous hash (links to governance message hash)
    pub previous_hash: String,

    /// Individual acknowledgments
    pub acknowledgments: Vec<RuleAcknowledgment>,

    /// Wrapper construction state
    pub wrapper_state: String,
}

/// Acknowledgment of a single rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAcknowledgment {
    /// Rule being acknowledged
    pub rule_id: String,

    /// Whether the rule is understood
    pub understood: bool,
}

/// CONTEXT message - CRA streams context blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    /// Sequence number for streaming
    pub sequence: u64,

    /// Previous hash in chain
    pub previous_hash: String,

    /// Context blocks in this chunk
    pub contexts: Vec<BootstrapContext>,

    /// Whether more context is coming
    #[serde(default)]
    pub more_available: bool,
}

/// A context block sent during bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapContext {
    /// Context identifier
    pub context_id: String,

    /// Priority (higher = more important)
    pub priority: i32,

    /// Injection mode for this context
    pub inject_mode: String,

    /// The actual content
    pub content: String,

    /// Hash of the content
    pub digest: String,
}

/// READY message - agent signals wrapper complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyMessage {
    /// Session ID
    pub session_id: String,

    /// Previous hash in chain
    pub previous_hash: String,

    /// Wrapper construction state (should be "complete")
    pub wrapper_state: String,

    /// Context IDs that have been internalized
    pub internalized_contexts: Vec<String>,

    /// What the agent is ready for
    pub ready_for: String,
}

/// SESSION message - CRA confirms handshake complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Session ID
    pub session_id: String,

    /// Previous hash in chain
    pub previous_hash: String,

    /// Session status (should be "active")
    pub status: String,

    /// TRACE endpoint for this session
    pub trace_endpoint: String,

    /// Tools available for the session
    pub tools_available: Vec<String>,

    /// Message to the agent
    pub message: String,
}

/// Bootstrap protocol handler
pub struct BootstrapProtocol {
    /// Current state in the protocol
    state: BootstrapState,

    /// Session being bootstrapped
    session: Option<Session>,

    /// Pending governance to acknowledge
    pending_governance: Option<GovernanceMessage>,

    /// Contexts sent so far
    contexts_sent: Vec<String>,
}

/// State of the bootstrap handshake
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapState {
    /// Waiting for INIT
    AwaitingInit,

    /// GOVERNANCE sent, waiting for ACK
    AwaitingAck,

    /// Streaming context
    StreamingContext,

    /// Context sent, waiting for READY
    AwaitingReady,

    /// SESSION sent, bootstrap complete
    Complete,
}

impl BootstrapProtocol {
    /// Create a new bootstrap protocol handler
    pub fn new() -> Self {
        Self {
            state: BootstrapState::AwaitingInit,
            session: None,
            pending_governance: None,
            contexts_sent: Vec::new(),
        }
    }

    /// Get current state
    pub fn state(&self) -> BootstrapState {
        self.state
    }

    /// Check if bootstrap is complete
    pub fn is_complete(&self) -> bool {
        self.state == BootstrapState::Complete
    }

    /// Process an INIT message and generate GOVERNANCE response
    pub fn handle_init(&mut self, init: InitMessage, session: Session) -> McpResult<GovernanceMessage> {
        if self.state != BootstrapState::AwaitingInit {
            return Err(McpError::Validation(format!(
                "Unexpected INIT: expected state {:?}, got {:?}",
                BootstrapState::AwaitingInit, self.state
            )));
        }

        // Generate governance message with standard rules
        let governance = GovernanceMessage {
            session_id: session.session_id.clone(),
            genesis_hash: session.genesis_hash.clone(),
            rules: vec![
                GovernanceRule {
                    rule_id: "trace.required".to_string(),
                    description: "All actions must be reported through cra_report_action".to_string(),
                    enforcement: "hard".to_string(),
                },
                GovernanceRule {
                    rule_id: "context.must_request".to_string(),
                    description: "Request context before making domain-specific decisions".to_string(),
                    enforcement: "soft".to_string(),
                },
                GovernanceRule {
                    rule_id: "feedback.expected".to_string(),
                    description: "Provide feedback on context usefulness".to_string(),
                    enforcement: "soft".to_string(),
                },
            ],
            policies: Vec::new(), // Would be populated from loaded atlases
            acknowledgment_required: true,
        };

        self.session = Some(session);
        self.pending_governance = Some(governance.clone());
        self.state = BootstrapState::AwaitingAck;

        Ok(governance)
    }

    /// Process an ACK message
    pub fn handle_ack(&mut self, ack: AckMessage) -> McpResult<()> {
        if self.state != BootstrapState::AwaitingAck {
            return Err(McpError::Validation(format!(
                "Unexpected ACK: expected state {:?}, got {:?}",
                BootstrapState::AwaitingAck, self.state
            )));
        }

        // Verify session ID matches
        let session = self.session.as_ref()
            .ok_or_else(|| McpError::Internal("No session in bootstrap".to_string()))?;

        if ack.session_id != session.session_id {
            return Err(McpError::InvalidSession(ack.session_id));
        }

        // Verify all rules acknowledged
        let governance = self.pending_governance.as_ref()
            .ok_or_else(|| McpError::Internal("No pending governance".to_string()))?;

        for rule in &governance.rules {
            let acked = ack.acknowledgments.iter()
                .find(|a| a.rule_id == rule.rule_id)
                .map(|a| a.understood)
                .unwrap_or(false);

            if !acked && rule.enforcement == "hard" {
                return Err(McpError::Validation(format!(
                    "Rule {} not acknowledged but has hard enforcement",
                    rule.rule_id
                )));
            }
        }

        self.state = BootstrapState::StreamingContext;
        Ok(())
    }

    /// Generate a context message
    pub fn generate_context(&mut self, contexts: Vec<BootstrapContext>, more_available: bool) -> McpResult<ContextMessage> {
        if self.state != BootstrapState::StreamingContext {
            return Err(McpError::Validation(format!(
                "Cannot send context: expected state {:?}, got {:?}",
                BootstrapState::StreamingContext, self.state
            )));
        }

        let session = self.session.as_ref()
            .ok_or_else(|| McpError::Internal("No session in bootstrap".to_string()))?;

        let sequence = self.contexts_sent.len() as u64;

        // Record sent contexts
        for ctx in &contexts {
            self.contexts_sent.push(ctx.context_id.clone());
        }

        if !more_available {
            self.state = BootstrapState::AwaitingReady;
        }

        Ok(ContextMessage {
            sequence,
            previous_hash: session.current_hash.clone(),
            contexts,
            more_available,
        })
    }

    /// Process a READY message and generate SESSION response
    pub fn handle_ready(&mut self, ready: ReadyMessage) -> McpResult<SessionMessage> {
        if self.state != BootstrapState::AwaitingReady {
            return Err(McpError::Validation(format!(
                "Unexpected READY: expected state {:?}, got {:?}",
                BootstrapState::AwaitingReady, self.state
            )));
        }

        let session = self.session.as_ref()
            .ok_or_else(|| McpError::Internal("No session in bootstrap".to_string()))?;

        if ready.session_id != session.session_id {
            return Err(McpError::InvalidSession(ready.session_id));
        }

        if ready.wrapper_state != "complete" {
            return Err(McpError::Validation(format!(
                "Wrapper not complete: {}",
                ready.wrapper_state
            )));
        }

        self.state = BootstrapState::Complete;

        Ok(SessionMessage {
            session_id: session.session_id.clone(),
            previous_hash: session.current_hash.clone(),
            status: "active".to_string(),
            trace_endpoint: format!("cra://trace/{}", session.session_id),
            tools_available: vec![
                "cra_request_context".to_string(),
                "cra_report_action".to_string(),
                "cra_feedback".to_string(),
                "cra_end_session".to_string(),
            ],
            message: "Governance established. Context internalized. You may begin.".to_string(),
        })
    }
}

impl Default for BootstrapProtocol {
    fn default() -> Self {
        Self::new()
    }
}

/// Simplified bootstrap result for MCP tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Session ID
    pub session_id: String,

    /// Genesis hash
    pub genesis_hash: String,

    /// Governance rules
    pub governance: GovernanceSection,

    /// Initial context blocks
    pub context: Vec<BootstrapContext>,

    /// Chain state
    pub chain_state: ChainState,

    /// Whether bootstrap is complete
    pub ready: bool,

    /// Message to the agent
    pub message: String,
}

/// Governance section of bootstrap result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceSection {
    /// Rules to follow
    pub rules: Vec<GovernanceRule>,

    /// Policies in effect
    pub policies: Vec<PolicySummary>,

    /// Things the agent MUST do
    pub you_must: Vec<String>,
}

/// Chain state in bootstrap result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    /// Current hash
    pub current_hash: String,

    /// Sequence number
    pub sequence: u64,

    /// Whether chain is verified
    pub verified: bool,
}
