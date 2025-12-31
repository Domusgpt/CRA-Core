//! Checkpoint System
//!
//! Checkpoints are **Steward-controlled intervention points** where CRA can:
//! - Inject context and guidance
//! - Require the LLM to answer questions before proceeding
//! - Enforce policy checks
//! - Gate access to capabilities
//! - Record significant events to TRACE
//!
//! ## Steward Authority
//!
//! **All checkpoint configuration is controlled by the Atlas Steward (publisher).**
//! The Steward defines:
//! - Which checkpoints are active
//! - What questions must be answered
//! - What guidance is injected
//! - What permissions are granted/revoked at each checkpoint
//!
//! This forms the core of the wrapper construction:
//! - **Permissions**: What the wrapper can do (CARP resolution)
//! - **Abilities**: What tools/actions are available
//! - **Context**: What knowledge is injected
//! - **Audit**: What events are recorded to TRACE
//!
//! ## Checkpoint Modes
//!
//! - `blocking` - LLM must answer questions before proceeding
//! - `advisory` - Guidance injected but not blocking
//! - `observational` - Only records to TRACE, no intervention
//!
//! ## Checkpoint Types
//!
//! - `session_start` - Initial context injection
//! - `session_end` - Finalize TRACE
//! - `keyword_match` - Keywords trigger context injection
//! - `action_pre` - Before action execution
//! - `action_post` - After action execution
//! - `risk_threshold` - Risk tier exceeded
//! - `time_interval` - Periodic refresh
//! - `count_interval` - After N actions
//! - `explicit_request` - On-demand from agent
//! - `error_occurred` - Error handling context
//! - `interactive` - Steward-defined interactive gate

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::RiskTier;

/// Checkpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointType {
    /// Session begins
    SessionStart,
    /// Session ends
    SessionEnd,
    /// Keywords matched in input
    KeywordMatch,
    /// Before action execution
    ActionPre,
    /// After action execution
    ActionPost,
    /// Risk tier threshold exceeded
    RiskThreshold,
    /// Time interval elapsed
    TimeInterval,
    /// Action count reached
    CountInterval,
    /// Agent explicitly requested context
    ExplicitRequest,
    /// Error occurred
    ErrorOccurred,
    /// Steward-defined interactive gate (questions + guidance)
    Interactive,
    /// Capability gate - before granting capability access
    CapabilityGate,
}

/// Checkpoint mode - how the checkpoint interacts with the LLM
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointMode {
    /// LLM must answer questions before proceeding
    Blocking,
    /// Guidance injected but not blocking
    #[default]
    Advisory,
    /// Only records to TRACE, no intervention
    Observational,
}

/// Steward-defined checkpoint definition
///
/// This is how the Atlas Steward configures checkpoints in their atlas.
/// The Steward has full control over what happens at each checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardCheckpointDef {
    /// Unique identifier for this checkpoint
    pub checkpoint_id: String,

    /// Human-readable name
    pub name: String,

    /// When this checkpoint triggers
    pub trigger: CheckpointTrigger,

    /// Checkpoint mode
    #[serde(default)]
    pub mode: CheckpointMode,

    /// Questions the LLM must answer (for blocking mode)
    #[serde(default)]
    pub questions: Vec<CheckpointQuestion>,

    /// Guidance text to inject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<GuidanceBlock>,

    /// Context blocks to inject
    #[serde(default)]
    pub inject_contexts: Vec<String>,

    /// Capabilities to unlock after this checkpoint
    #[serde(default)]
    pub unlock_capabilities: Vec<String>,

    /// Capabilities to lock at this checkpoint
    #[serde(default)]
    pub lock_capabilities: Vec<String>,

    /// Actions to allow after this checkpoint
    #[serde(default)]
    pub allow_actions: Vec<String>,

    /// Actions to deny at this checkpoint
    #[serde(default)]
    pub deny_actions: Vec<String>,

    /// Whether to force sync TRACE at this checkpoint
    #[serde(default)]
    pub force_sync_trace: bool,

    /// Priority (higher = evaluated first)
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    500
}

/// Trigger conditions for a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckpointTrigger {
    /// Trigger on session start
    SessionStart,

    /// Trigger on session end
    SessionEnd,

    /// Trigger on keyword match
    Keyword {
        patterns: Vec<String>,
        #[serde(default)]
        case_sensitive: bool,
        #[serde(default)]
        match_mode: MatchMode,
    },

    /// Trigger before specific actions
    ActionPre {
        /// Action patterns (supports wildcards like "ticket.*")
        patterns: Vec<String>,
    },

    /// Trigger after specific actions
    ActionPost {
        patterns: Vec<String>,
    },

    /// Trigger when risk tier is reached
    RiskThreshold {
        min_tier: RiskTier,
    },

    /// Trigger on time interval
    TimeInterval {
        seconds: u64,
    },

    /// Trigger on action count
    CountInterval {
        actions: u64,
    },

    /// Trigger before capability is used
    CapabilityAccess {
        capability_ids: Vec<String>,
    },

    /// Custom trigger (evaluated by steward's API)
    Custom {
        trigger_id: String,
        #[serde(default)]
        params: HashMap<String, Value>,
    },
}

/// A question the Steward requires the LLM to answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointQuestion {
    /// Unique question ID
    pub question_id: String,

    /// The question text
    pub question: String,

    /// Expected response type
    #[serde(default)]
    pub response_type: ResponseType,

    /// Whether this question is required
    #[serde(default = "default_true")]
    pub required: bool,

    /// Validation rules for the answer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<AnswerValidation>,

    /// Hint text for the LLM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,

    /// What happens if validation fails
    #[serde(default)]
    pub on_invalid: InvalidAnswerAction,
}

fn default_true() -> bool {
    true
}

/// Expected response type for a question
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    /// Free-form text
    #[default]
    Text,
    /// Yes/No confirmation
    Boolean,
    /// Selection from options
    Choice { options: Vec<String> },
    /// Numeric value
    Number { min: Option<f64>, max: Option<f64> },
    /// JSON structure
    Json { schema: Option<Value> },
    /// Acknowledgment only (LLM says "understood")
    Acknowledgment,
}

/// Validation rules for answers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerValidation {
    /// Regex pattern the answer must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Minimum length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,

    /// Maximum length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Required keywords
    #[serde(default)]
    pub must_contain: Vec<String>,

    /// Forbidden keywords
    #[serde(default)]
    pub must_not_contain: Vec<String>,

    /// Custom validation (steward API callback)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_validator: Option<String>,
}

/// What happens when an answer is invalid
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidAnswerAction {
    /// Block and ask again
    #[default]
    Retry,
    /// Block completely
    Block,
    /// Allow with warning
    WarnAndContinue,
    /// Record to TRACE and continue
    LogAndContinue,
}

/// Guidance block - bespoke context/instructions from the Steward
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidanceBlock {
    /// Guidance format
    #[serde(default)]
    pub format: GuidanceFormat,

    /// The guidance content
    pub content: String,

    /// Priority for ordering multiple guidance blocks
    #[serde(default)]
    pub priority: i32,

    /// Whether this replaces previous guidance or appends
    #[serde(default)]
    pub append: bool,

    /// Expiry - guidance is removed after this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<String>,

    /// Labels for categorization
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Guidance content format
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceFormat {
    /// Plain text
    #[default]
    Text,
    /// Markdown
    Markdown,
    /// Structured JSON
    Json,
    /// System instruction format
    SystemInstruction,
}

/// LLM's response to a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResponse {
    /// The checkpoint this responds to
    pub checkpoint_id: String,

    /// Answers to questions
    #[serde(default)]
    pub answers: HashMap<String, AnswerValue>,

    /// Whether the LLM acknowledged guidance
    #[serde(default)]
    pub guidance_acknowledged: bool,

    /// Timestamp
    pub responded_at: String,

    /// Session ID for TRACE correlation
    pub session_id: String,
}

/// Value of an answer to a checkpoint question
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnswerValue {
    Text(String),
    Boolean(bool),
    Number(f64),
    Choice(String),
    Json(Value),
    Acknowledged,
}

/// Result of validating checkpoint responses
#[derive(Debug, Clone)]
pub struct CheckpointValidation {
    /// Whether all required questions were answered validly
    pub is_valid: bool,

    /// Individual question results
    pub question_results: HashMap<String, QuestionValidationResult>,

    /// Actions to take based on validation
    pub actions: Vec<CheckpointAction>,

    /// Capabilities unlocked
    pub unlocked_capabilities: Vec<String>,

    /// Capabilities locked
    pub locked_capabilities: Vec<String>,

    /// Guidance to inject
    pub guidance: Option<GuidanceBlock>,

    /// Context blocks to inject
    pub inject_contexts: Vec<String>,
}

/// Result of validating a single question
#[derive(Debug, Clone)]
pub struct QuestionValidationResult {
    pub question_id: String,
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub action: InvalidAnswerAction,
}

/// Actions to take as a result of checkpoint validation
#[derive(Debug, Clone)]
pub enum CheckpointAction {
    /// Proceed with the action/session
    Proceed,
    /// Block and retry the checkpoint
    Retry { message: String },
    /// Block completely
    Block { reason: String },
    /// Inject context
    InjectContext { context_id: String },
    /// Inject guidance
    InjectGuidance { guidance: GuidanceBlock },
    /// Unlock capability
    UnlockCapability { capability_id: String },
    /// Lock capability
    LockCapability { capability_id: String },
    /// Allow action
    AllowAction { action_pattern: String },
    /// Deny action
    DenyAction { action_pattern: String },
    /// Record to TRACE
    RecordTrace { event_type: String, payload: Value },
}

impl CheckpointType {
    /// Get the default priority for this checkpoint type
    pub fn default_priority(&self) -> u32 {
        match self {
            CheckpointType::SessionStart => 1000,
            CheckpointType::Interactive => 950, // Interactive gates are high priority
            CheckpointType::CapabilityGate => 920,
            CheckpointType::RiskThreshold => 900,
            CheckpointType::ActionPre => 800,
            CheckpointType::KeywordMatch => 600,
            CheckpointType::TimeInterval => 500,
            CheckpointType::CountInterval => 400,
            CheckpointType::ExplicitRequest => 300,
            CheckpointType::ActionPost => 100,
            CheckpointType::ErrorOccurred => 50,
            CheckpointType::SessionEnd => 0,
        }
    }

    /// Check if this checkpoint should be synchronous
    pub fn is_sync(&self) -> bool {
        matches!(
            self,
            CheckpointType::SessionEnd
                | CheckpointType::RiskThreshold
                | CheckpointType::ActionPre
                | CheckpointType::Interactive
                | CheckpointType::CapabilityGate
        )
    }

    /// Check if this checkpoint requires LLM response
    pub fn requires_response(&self) -> bool {
        matches!(
            self,
            CheckpointType::Interactive | CheckpointType::CapabilityGate
        )
    }
}

impl StewardCheckpointDef {
    /// Create a new steward checkpoint
    pub fn new(
        checkpoint_id: impl Into<String>,
        name: impl Into<String>,
        trigger: CheckpointTrigger,
    ) -> Self {
        Self {
            checkpoint_id: checkpoint_id.into(),
            name: name.into(),
            trigger,
            mode: CheckpointMode::Advisory,
            questions: vec![],
            guidance: None,
            inject_contexts: vec![],
            unlock_capabilities: vec![],
            lock_capabilities: vec![],
            allow_actions: vec![],
            deny_actions: vec![],
            force_sync_trace: false,
            priority: 500,
        }
    }

    /// Make this a blocking checkpoint with questions
    pub fn blocking(mut self) -> Self {
        self.mode = CheckpointMode::Blocking;
        self
    }

    /// Add a question
    pub fn with_question(mut self, question: CheckpointQuestion) -> Self {
        self.questions.push(question);
        self
    }

    /// Add guidance
    pub fn with_guidance(mut self, guidance: GuidanceBlock) -> Self {
        self.guidance = Some(guidance);
        self
    }

    /// Add contexts to inject
    pub fn inject_contexts(mut self, contexts: Vec<String>) -> Self {
        self.inject_contexts = contexts;
        self
    }

    /// Unlock capabilities after this checkpoint
    pub fn unlock_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.unlock_capabilities = capabilities;
        self
    }

    /// Lock capabilities at this checkpoint
    pub fn lock_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.lock_capabilities = capabilities;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this checkpoint requires LLM response
    pub fn requires_response(&self) -> bool {
        self.mode == CheckpointMode::Blocking && !self.questions.is_empty()
    }
}

impl CheckpointQuestion {
    /// Create a simple text question
    pub fn text(
        question_id: impl Into<String>,
        question: impl Into<String>,
    ) -> Self {
        Self {
            question_id: question_id.into(),
            question: question.into(),
            response_type: ResponseType::Text,
            required: true,
            validation: None,
            hint: None,
            on_invalid: InvalidAnswerAction::Retry,
        }
    }

    /// Create a yes/no question
    pub fn boolean(
        question_id: impl Into<String>,
        question: impl Into<String>,
    ) -> Self {
        Self {
            question_id: question_id.into(),
            question: question.into(),
            response_type: ResponseType::Boolean,
            required: true,
            validation: None,
            hint: None,
            on_invalid: InvalidAnswerAction::Retry,
        }
    }

    /// Create an acknowledgment question
    pub fn acknowledgment(
        question_id: impl Into<String>,
        statement: impl Into<String>,
    ) -> Self {
        Self {
            question_id: question_id.into(),
            question: statement.into(),
            response_type: ResponseType::Acknowledgment,
            required: true,
            validation: None,
            hint: Some("Respond with 'acknowledged' or 'understood'".to_string()),
            on_invalid: InvalidAnswerAction::Retry,
        }
    }

    /// Create a choice question
    pub fn choice(
        question_id: impl Into<String>,
        question: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            question_id: question_id.into(),
            question: question.into(),
            response_type: ResponseType::Choice { options },
            required: true,
            validation: None,
            hint: None,
            on_invalid: InvalidAnswerAction::Retry,
        }
    }

    /// Add validation
    pub fn with_validation(mut self, validation: AnswerValidation) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Add hint
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Set what happens on invalid answer
    pub fn on_invalid(mut self, action: InvalidAnswerAction) -> Self {
        self.on_invalid = action;
        self
    }

    /// Make this question optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

impl GuidanceBlock {
    /// Create a simple text guidance block
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            format: GuidanceFormat::Text,
            content: content.into(),
            priority: 0,
            append: true,
            expires_after: None,
            labels: vec![],
        }
    }

    /// Create a markdown guidance block
    pub fn markdown(content: impl Into<String>) -> Self {
        Self {
            format: GuidanceFormat::Markdown,
            content: content.into(),
            priority: 0,
            append: true,
            expires_after: None,
            labels: vec![],
        }
    }

    /// Create a system instruction guidance block
    pub fn system_instruction(content: impl Into<String>) -> Self {
        Self {
            format: GuidanceFormat::SystemInstruction,
            content: content.into(),
            priority: 100, // System instructions have high priority
            append: false, // Replace previous system instructions
            expires_after: None,
            labels: vec!["system".to_string()],
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add labels
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set expiry
    pub fn expires_after(mut self, checkpoint_id: impl Into<String>) -> Self {
        self.expires_after = Some(checkpoint_id.into());
        self
    }
}

/// Checkpoint trigger result
#[derive(Debug, Clone)]
pub struct TriggeredCheckpoint {
    /// The checkpoint type that triggered
    pub checkpoint_type: CheckpointType,
    /// Priority for ordering
    pub priority: u32,
    /// Context IDs to inject
    pub inject_contexts: Vec<String>,
    /// Whether this is a sync checkpoint
    pub is_sync: bool,
    /// Additional data from the trigger
    pub trigger_data: Option<TriggerData>,
    /// Steward checkpoint definition (if from steward config)
    pub steward_def: Option<StewardCheckpointDef>,
    /// Questions that must be answered (for interactive checkpoints)
    pub questions: Vec<CheckpointQuestion>,
    /// Guidance to inject
    pub guidance: Option<GuidanceBlock>,
    /// Checkpoint mode
    pub mode: CheckpointMode,
}

impl TriggeredCheckpoint {
    /// Check if this checkpoint requires LLM response
    pub fn requires_response(&self) -> bool {
        self.mode == CheckpointMode::Blocking && !self.questions.is_empty()
    }

    /// Get all capabilities to unlock
    pub fn unlocked_capabilities(&self) -> Vec<String> {
        self.steward_def
            .as_ref()
            .map(|def| def.unlock_capabilities.clone())
            .unwrap_or_default()
    }

    /// Get all capabilities to lock
    pub fn locked_capabilities(&self) -> Vec<String> {
        self.steward_def
            .as_ref()
            .map(|def| def.lock_capabilities.clone())
            .unwrap_or_default()
    }
}

/// Validates checkpoint responses against questions
pub struct CheckpointValidator;

impl CheckpointValidator {
    /// Validate a response against a checkpoint definition
    pub fn validate(
        checkpoint: &TriggeredCheckpoint,
        response: &CheckpointResponse,
    ) -> CheckpointValidation {
        let mut question_results = HashMap::new();
        let mut is_valid = true;
        let mut actions = vec![];

        for question in &checkpoint.questions {
            let result = Self::validate_question(question, response.answers.get(&question.question_id));

            if question.required && !result.is_valid {
                is_valid = false;
                match result.action {
                    InvalidAnswerAction::Retry => {
                        actions.push(CheckpointAction::Retry {
                            message: result.error_message.clone().unwrap_or_else(|| {
                                format!("Please answer: {}", question.question)
                            }),
                        });
                    }
                    InvalidAnswerAction::Block => {
                        actions.push(CheckpointAction::Block {
                            reason: result.error_message.clone().unwrap_or_else(|| {
                                "Required question not answered".to_string()
                            }),
                        });
                    }
                    InvalidAnswerAction::WarnAndContinue | InvalidAnswerAction::LogAndContinue => {
                        // Continue despite invalid answer
                    }
                }
            }

            question_results.insert(question.question_id.clone(), result);
        }

        // If valid, add proceed action and capability/context actions
        if is_valid {
            actions.push(CheckpointAction::Proceed);

            // Add context injections
            for context_id in &checkpoint.inject_contexts {
                actions.push(CheckpointAction::InjectContext {
                    context_id: context_id.clone(),
                });
            }

            // Add guidance injection
            if let Some(guidance) = &checkpoint.guidance {
                actions.push(CheckpointAction::InjectGuidance {
                    guidance: guidance.clone(),
                });
            }

            // Add capability unlocks
            for cap in checkpoint.unlocked_capabilities() {
                actions.push(CheckpointAction::UnlockCapability {
                    capability_id: cap,
                });
            }

            // Add capability locks
            for cap in checkpoint.locked_capabilities() {
                actions.push(CheckpointAction::LockCapability {
                    capability_id: cap,
                });
            }
        }

        CheckpointValidation {
            is_valid,
            question_results,
            actions,
            unlocked_capabilities: checkpoint.unlocked_capabilities(),
            locked_capabilities: checkpoint.locked_capabilities(),
            guidance: checkpoint.guidance.clone(),
            inject_contexts: checkpoint.inject_contexts.clone(),
        }
    }

    fn validate_question(
        question: &CheckpointQuestion,
        answer: Option<&AnswerValue>,
    ) -> QuestionValidationResult {
        let Some(answer) = answer else {
            return QuestionValidationResult {
                question_id: question.question_id.clone(),
                is_valid: false,
                error_message: Some("No answer provided".to_string()),
                action: question.on_invalid.clone(),
            };
        };

        // Type validation
        let type_valid = match (&question.response_type, answer) {
            (ResponseType::Text, AnswerValue::Text(_)) => true,
            (ResponseType::Boolean, AnswerValue::Boolean(_)) => true,
            (ResponseType::Number { min, max }, AnswerValue::Number(n)) => {
                let in_min = min.map_or(true, |m| *n >= m);
                let in_max = max.map_or(true, |m| *n <= m);
                in_min && in_max
            }
            (ResponseType::Choice { options }, AnswerValue::Choice(c)) => {
                options.contains(c)
            }
            (ResponseType::Json { .. }, AnswerValue::Json(_)) => true,
            (ResponseType::Acknowledgment, AnswerValue::Acknowledged) => true,
            (ResponseType::Acknowledgment, AnswerValue::Text(t)) => {
                let t_lower = t.to_lowercase();
                t_lower.contains("acknowledged") || t_lower.contains("understood")
            }
            _ => false,
        };

        if !type_valid {
            return QuestionValidationResult {
                question_id: question.question_id.clone(),
                is_valid: false,
                error_message: Some("Answer type does not match expected type".to_string()),
                action: question.on_invalid.clone(),
            };
        }

        // Custom validation
        if let Some(validation) = &question.validation {
            if let AnswerValue::Text(text) = answer {
                // Length validation
                if let Some(min) = validation.min_length {
                    if text.len() < min {
                        return QuestionValidationResult {
                            question_id: question.question_id.clone(),
                            is_valid: false,
                            error_message: Some(format!("Answer too short (min {} chars)", min)),
                            action: question.on_invalid.clone(),
                        };
                    }
                }
                if let Some(max) = validation.max_length {
                    if text.len() > max {
                        return QuestionValidationResult {
                            question_id: question.question_id.clone(),
                            is_valid: false,
                            error_message: Some(format!("Answer too long (max {} chars)", max)),
                            action: question.on_invalid.clone(),
                        };
                    }
                }

                // Must contain
                for keyword in &validation.must_contain {
                    if !text.to_lowercase().contains(&keyword.to_lowercase()) {
                        return QuestionValidationResult {
                            question_id: question.question_id.clone(),
                            is_valid: false,
                            error_message: Some(format!("Answer must contain: {}", keyword)),
                            action: question.on_invalid.clone(),
                        };
                    }
                }

                // Must not contain
                for keyword in &validation.must_not_contain {
                    if text.to_lowercase().contains(&keyword.to_lowercase()) {
                        return QuestionValidationResult {
                            question_id: question.question_id.clone(),
                            is_valid: false,
                            error_message: Some(format!("Answer must not contain: {}", keyword)),
                            action: question.on_invalid.clone(),
                        };
                    }
                }

                // Pattern validation
                if let Some(pattern) = &validation.pattern {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if !re.is_match(text) {
                            return QuestionValidationResult {
                                question_id: question.question_id.clone(),
                                is_valid: false,
                                error_message: Some("Answer does not match required pattern".to_string()),
                                action: question.on_invalid.clone(),
                            };
                        }
                    }
                }
            }
        }

        QuestionValidationResult {
            question_id: question.question_id.clone(),
            is_valid: true,
            error_message: None,
            action: question.on_invalid.clone(),
        }
    }
}

/// Additional data from checkpoint triggers
#[derive(Debug, Clone)]
pub enum TriggerData {
    /// Keywords that matched
    Keywords(Vec<String>),
    /// Action being performed
    Action {
        action_id: String,
        params: Option<Value>,
    },
    /// Risk tier that triggered
    Risk {
        tier: RiskTier,
        action_id: Option<String>,
    },
    /// Interval data
    Interval {
        elapsed: Duration,
        count: u64,
    },
    /// Error data
    Error {
        error_type: String,
        message: String,
    },
}

/// Checkpoint configuration from atlas
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Session start configuration
    #[serde(default)]
    pub session_start: SessionStartConfig,

    /// Session end configuration
    #[serde(default)]
    pub session_end: SessionEndConfig,

    /// Keyword matching configuration
    #[serde(default)]
    pub keyword_match: KeywordMatchConfig,

    /// Action pre-execution configuration
    #[serde(default)]
    pub action_pre: ActionPreConfig,

    /// Risk threshold configuration
    #[serde(default)]
    pub risk_threshold: RiskThresholdConfig,

    /// Time interval configuration
    #[serde(default)]
    pub time_interval: TimeIntervalConfig,

    /// Count interval configuration
    #[serde(default)]
    pub count_interval: CountIntervalConfig,
}

/// Session start checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartConfig {
    pub enabled: bool,
    #[serde(default)]
    pub inject_contexts: Vec<String>,
}

impl Default for SessionStartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            inject_contexts: vec![],
        }
    }
}

/// Session end checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndConfig {
    pub enabled: bool,
    pub require_sync_trace: bool,
}

impl Default for SessionEndConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_sync_trace: true,
        }
    }
}

/// Keyword match checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatchConfig {
    pub enabled: bool,
    pub case_sensitive: bool,
    #[serde(default)]
    pub match_mode: MatchMode,
    /// Map from keyword patterns to context IDs
    #[serde(default)]
    pub mappings: HashMap<String, Vec<String>>,
}

impl Default for KeywordMatchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            case_sensitive: false,
            match_mode: MatchMode::Any,
            mappings: HashMap::new(),
        }
    }
}

/// Match mode for keywords
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// Match if any keyword found
    #[default]
    Any,
    /// Match only if all keywords found
    All,
    /// Match exact phrase
    Phrase,
    /// Use regex patterns
    Regex,
}

/// Action pre-execution checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPreConfig {
    pub enabled: bool,
    /// Map from action patterns to config
    #[serde(default)]
    pub mappings: HashMap<String, ActionCheckpointConfig>,
}

impl Default for ActionPreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mappings: HashMap::new(),
        }
    }
}

/// Config for individual action checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCheckpointConfig {
    #[serde(default)]
    pub inject_contexts: Vec<String>,
    #[serde(default)]
    pub require_policy_check: bool,
    #[serde(default)]
    pub require_confirmation: bool,
}

/// Risk threshold checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskThresholdConfig {
    pub enabled: bool,
    pub min_tier: RiskTier,
    #[serde(default)]
    pub inject_contexts: Vec<String>,
    #[serde(default)]
    pub require_sync_trace: bool,
}

impl Default for RiskThresholdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_tier: RiskTier::High,
            inject_contexts: vec![],
            require_sync_trace: true,
        }
    }
}

/// Time interval checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeIntervalConfig {
    pub enabled: bool,
    pub seconds: u64,
    #[serde(default)]
    pub inject_contexts: Vec<String>,
}

impl Default for TimeIntervalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            seconds: 300,
            inject_contexts: vec![],
        }
    }
}

/// Count interval checkpoint config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountIntervalConfig {
    pub enabled: bool,
    pub actions: u64,
    #[serde(default)]
    pub inject_contexts: Vec<String>,
}

impl Default for CountIntervalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            actions: 50,
            inject_contexts: vec![],
        }
    }
}

/// Session state for checkpoint evaluation
#[derive(Debug)]
pub struct SessionCheckpointState {
    /// Session start time
    pub started_at: Instant,
    /// Last checkpoint time
    pub last_checkpoint: Instant,
    /// Action count since last checkpoint
    pub action_count: u64,
    /// Total action count
    pub total_actions: u64,
    /// Keywords already matched (to avoid duplicates)
    pub matched_keywords: HashSet<String>,
}

impl SessionCheckpointState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_checkpoint: now,
            action_count: 0,
            total_actions: 0,
            matched_keywords: HashSet::new(),
        }
    }

    /// Record an action
    pub fn record_action(&mut self) {
        self.action_count += 1;
        self.total_actions += 1;
    }

    /// Reset after checkpoint
    pub fn checkpoint_complete(&mut self) {
        self.last_checkpoint = Instant::now();
        self.action_count = 0;
    }
}

impl Default for SessionCheckpointState {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint evaluator
#[derive(Debug)]
pub struct CheckpointEvaluator {
    config: CheckpointConfig,
}

impl CheckpointEvaluator {
    /// Create a new evaluator with config
    pub fn new(config: CheckpointConfig) -> Self {
        Self { config }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(CheckpointConfig::default())
    }

    /// Evaluate session start checkpoint
    pub fn on_session_start(&self) -> Option<TriggeredCheckpoint> {
        if !self.config.session_start.enabled {
            return None;
        }

        Some(TriggeredCheckpoint {
            checkpoint_type: CheckpointType::SessionStart,
            priority: CheckpointType::SessionStart.default_priority(),
            inject_contexts: self.config.session_start.inject_contexts.clone(),
            is_sync: false,
            trigger_data: None,
            steward_def: None,
            questions: vec![],
            guidance: None,
            mode: CheckpointMode::Advisory,
        })
    }

    /// Evaluate session end checkpoint
    pub fn on_session_end(&self) -> Option<TriggeredCheckpoint> {
        if !self.config.session_end.enabled {
            return None;
        }

        Some(TriggeredCheckpoint {
            checkpoint_type: CheckpointType::SessionEnd,
            priority: CheckpointType::SessionEnd.default_priority(),
            inject_contexts: vec![],
            is_sync: self.config.session_end.require_sync_trace,
            trigger_data: None,
            steward_def: None,
            questions: vec![],
            guidance: None,
            mode: CheckpointMode::Observational,
        })
    }

    /// Evaluate keyword match checkpoint
    pub fn on_input(
        &self,
        input: &str,
        state: &mut SessionCheckpointState,
    ) -> Vec<TriggeredCheckpoint> {
        let mut checkpoints = vec![];

        // Keyword matching
        if self.config.keyword_match.enabled {
            if let Some(checkpoint) = self.evaluate_keywords(input, state) {
                checkpoints.push(checkpoint);
            }
        }

        // Time interval
        if self.config.time_interval.enabled {
            if let Some(checkpoint) = self.evaluate_time_interval(state) {
                checkpoints.push(checkpoint);
            }
        }

        // Sort by priority (highest first)
        checkpoints.sort_by(|a, b| b.priority.cmp(&a.priority));

        checkpoints
    }

    /// Evaluate action pre checkpoint
    pub fn on_action_pre(
        &self,
        action_id: &str,
        params: Option<&Value>,
        risk_tier: RiskTier,
        state: &mut SessionCheckpointState,
    ) -> Vec<TriggeredCheckpoint> {
        let mut checkpoints = vec![];

        // Action pre config
        if self.config.action_pre.enabled {
            if let Some(checkpoint) = self.evaluate_action_pre(action_id, params) {
                checkpoints.push(checkpoint);
            }
        }

        // Risk threshold
        if self.config.risk_threshold.enabled {
            if let Some(checkpoint) = self.evaluate_risk_threshold(risk_tier, action_id) {
                checkpoints.push(checkpoint);
            }
        }

        // Count interval
        state.record_action();
        if self.config.count_interval.enabled {
            if let Some(checkpoint) = self.evaluate_count_interval(state) {
                checkpoints.push(checkpoint);
            }
        }

        // Sort by priority
        checkpoints.sort_by(|a, b| b.priority.cmp(&a.priority));

        checkpoints
    }

    /// Evaluate action post checkpoint
    pub fn on_action_post(
        &self,
        action_id: &str,
        _success: bool,
    ) -> Option<TriggeredCheckpoint> {
        // Basic action post - could be extended with config
        Some(TriggeredCheckpoint {
            checkpoint_type: CheckpointType::ActionPost,
            priority: CheckpointType::ActionPost.default_priority(),
            inject_contexts: vec![],
            is_sync: false,
            trigger_data: Some(TriggerData::Action {
                action_id: action_id.to_string(),
                params: None,
            }),
            steward_def: None,
            questions: vec![],
            guidance: None,
            mode: CheckpointMode::Observational,
        })
    }

    /// Evaluate error checkpoint
    pub fn on_error(&self, error_type: &str, message: &str) -> Option<TriggeredCheckpoint> {
        Some(TriggeredCheckpoint {
            checkpoint_type: CheckpointType::ErrorOccurred,
            priority: CheckpointType::ErrorOccurred.default_priority(),
            inject_contexts: vec![],
            is_sync: false,
            trigger_data: Some(TriggerData::Error {
                error_type: error_type.to_string(),
                message: message.to_string(),
            }),
            steward_def: None,
            questions: vec![],
            guidance: None,
            mode: CheckpointMode::Observational,
        })
    }

    /// Evaluate explicit request checkpoint
    pub fn on_explicit_request(&self, context_ids: Vec<String>) -> TriggeredCheckpoint {
        TriggeredCheckpoint {
            checkpoint_type: CheckpointType::ExplicitRequest,
            priority: CheckpointType::ExplicitRequest.default_priority(),
            inject_contexts: context_ids,
            is_sync: false,
            trigger_data: None,
            steward_def: None,
            questions: vec![],
            guidance: None,
            mode: CheckpointMode::Advisory,
        }
    }

    /// Evaluate a steward-defined checkpoint
    pub fn evaluate_steward_checkpoint(
        &self,
        checkpoint_def: &StewardCheckpointDef,
        trigger_data: Option<TriggerData>,
    ) -> TriggeredCheckpoint {
        TriggeredCheckpoint {
            checkpoint_type: CheckpointType::Interactive,
            priority: checkpoint_def.priority,
            inject_contexts: checkpoint_def.inject_contexts.clone(),
            is_sync: checkpoint_def.force_sync_trace || checkpoint_def.mode == CheckpointMode::Blocking,
            trigger_data,
            steward_def: Some(checkpoint_def.clone()),
            questions: checkpoint_def.questions.clone(),
            guidance: checkpoint_def.guidance.clone(),
            mode: checkpoint_def.mode,
        }
    }

    // Internal evaluation methods

    fn evaluate_keywords(
        &self,
        input: &str,
        state: &mut SessionCheckpointState,
    ) -> Option<TriggeredCheckpoint> {
        let input_normalized = if self.config.keyword_match.case_sensitive {
            input.to_string()
        } else {
            input.to_lowercase()
        };

        let mut matched_keywords = vec![];
        let mut contexts_to_inject = vec![];

        for (pattern, contexts) in &self.config.keyword_match.mappings {
            // Pattern can be "keyword1|keyword2" for OR matching
            let keywords: Vec<&str> = pattern.split('|').collect();

            let matches = match self.config.keyword_match.match_mode {
                MatchMode::Any => keywords.iter().any(|k| {
                    let kw = if self.config.keyword_match.case_sensitive {
                        k.to_string()
                    } else {
                        k.to_lowercase()
                    };
                    input_normalized.contains(&kw)
                }),
                MatchMode::All => keywords.iter().all(|k| {
                    let kw = if self.config.keyword_match.case_sensitive {
                        k.to_string()
                    } else {
                        k.to_lowercase()
                    };
                    input_normalized.contains(&kw)
                }),
                MatchMode::Phrase => {
                    let phrase = if self.config.keyword_match.case_sensitive {
                        pattern.clone()
                    } else {
                        pattern.to_lowercase()
                    };
                    input_normalized.contains(&phrase)
                }
                MatchMode::Regex => {
                    // Try to compile as regex
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(&input_normalized))
                        .unwrap_or(false)
                }
            };

            if matches {
                // Check if we've already matched this pattern
                if !state.matched_keywords.contains(pattern) {
                    state.matched_keywords.insert(pattern.clone());
                    matched_keywords.extend(keywords.iter().map(|s| s.to_string()));
                    contexts_to_inject.extend(contexts.clone());
                }
            }
        }

        if matched_keywords.is_empty() {
            return None;
        }

        Some(TriggeredCheckpoint {
            checkpoint_type: CheckpointType::KeywordMatch,
            priority: CheckpointType::KeywordMatch.default_priority(),
            inject_contexts: contexts_to_inject,
            is_sync: false,
            trigger_data: Some(TriggerData::Keywords(matched_keywords)),
            steward_def: None,
            questions: vec![],
            guidance: None,
            mode: CheckpointMode::Advisory,
        })
    }

    fn evaluate_action_pre(
        &self,
        action_id: &str,
        params: Option<&Value>,
    ) -> Option<TriggeredCheckpoint> {
        // Try exact match first
        if let Some(config) = self.config.action_pre.mappings.get(action_id) {
            return Some(TriggeredCheckpoint {
                checkpoint_type: CheckpointType::ActionPre,
                priority: CheckpointType::ActionPre.default_priority(),
                inject_contexts: config.inject_contexts.clone(),
                is_sync: config.require_policy_check,
                trigger_data: Some(TriggerData::Action {
                    action_id: action_id.to_string(),
                    params: params.cloned(),
                }),
                steward_def: None,
                questions: vec![],
                guidance: None,
                mode: if config.require_confirmation {
                    CheckpointMode::Blocking
                } else {
                    CheckpointMode::Advisory
                },
            });
        }

        // Try wildcard patterns
        for (pattern, config) in &self.config.action_pre.mappings {
            if pattern.ends_with('*') {
                let prefix = pattern.trim_end_matches('*');
                if action_id.starts_with(prefix) {
                    return Some(TriggeredCheckpoint {
                        checkpoint_type: CheckpointType::ActionPre,
                        priority: CheckpointType::ActionPre.default_priority(),
                        inject_contexts: config.inject_contexts.clone(),
                        is_sync: config.require_policy_check,
                        trigger_data: Some(TriggerData::Action {
                            action_id: action_id.to_string(),
                            params: params.cloned(),
                        }),
                        steward_def: None,
                        questions: vec![],
                        guidance: None,
                        mode: if config.require_confirmation {
                            CheckpointMode::Blocking
                        } else {
                            CheckpointMode::Advisory
                        },
                    });
                }
            }
        }

        None
    }

    fn evaluate_risk_threshold(
        &self,
        risk_tier: RiskTier,
        action_id: &str,
    ) -> Option<TriggeredCheckpoint> {
        // Compare risk tiers
        let current_level = match risk_tier {
            RiskTier::Low => 0,
            RiskTier::Medium => 1,
            RiskTier::High => 2,
            RiskTier::Critical => 3,
        };

        let min_level = match self.config.risk_threshold.min_tier {
            RiskTier::Low => 0,
            RiskTier::Medium => 1,
            RiskTier::High => 2,
            RiskTier::Critical => 3,
        };

        if current_level >= min_level {
            return Some(TriggeredCheckpoint {
                checkpoint_type: CheckpointType::RiskThreshold,
                priority: CheckpointType::RiskThreshold.default_priority(),
                inject_contexts: self.config.risk_threshold.inject_contexts.clone(),
                is_sync: self.config.risk_threshold.require_sync_trace,
                trigger_data: Some(TriggerData::Risk {
                    tier: risk_tier,
                    action_id: Some(action_id.to_string()),
                }),
                steward_def: None,
                questions: vec![],
                guidance: None,
                mode: CheckpointMode::Advisory,
            });
        }

        None
    }

    fn evaluate_time_interval(
        &self,
        state: &SessionCheckpointState,
    ) -> Option<TriggeredCheckpoint> {
        let elapsed = state.last_checkpoint.elapsed();
        let threshold = Duration::from_secs(self.config.time_interval.seconds);

        if elapsed >= threshold {
            return Some(TriggeredCheckpoint {
                checkpoint_type: CheckpointType::TimeInterval,
                priority: CheckpointType::TimeInterval.default_priority(),
                inject_contexts: self.config.time_interval.inject_contexts.clone(),
                is_sync: false,
                trigger_data: Some(TriggerData::Interval {
                    elapsed,
                    count: state.action_count,
                }),
                steward_def: None,
                questions: vec![],
                guidance: None,
                mode: CheckpointMode::Advisory,
            });
        }

        None
    }

    fn evaluate_count_interval(
        &self,
        state: &SessionCheckpointState,
    ) -> Option<TriggeredCheckpoint> {
        if state.action_count >= self.config.count_interval.actions {
            return Some(TriggeredCheckpoint {
                checkpoint_type: CheckpointType::CountInterval,
                priority: CheckpointType::CountInterval.default_priority(),
                inject_contexts: self.config.count_interval.inject_contexts.clone(),
                is_sync: false,
                trigger_data: Some(TriggerData::Interval {
                    elapsed: state.last_checkpoint.elapsed(),
                    count: state.action_count,
                }),
                steward_def: None,
                questions: vec![],
                guidance: None,
                mode: CheckpointMode::Advisory,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_start_checkpoint() {
        let mut config = CheckpointConfig::default();
        config.session_start.inject_contexts = vec!["intro".to_string()];

        let evaluator = CheckpointEvaluator::new(config);
        let checkpoint = evaluator.on_session_start();

        assert!(checkpoint.is_some());
        let cp = checkpoint.unwrap();
        assert_eq!(cp.checkpoint_type, CheckpointType::SessionStart);
        assert_eq!(cp.inject_contexts, vec!["intro"]);
    }

    #[test]
    fn test_keyword_matching() {
        let mut config = CheckpointConfig::default();
        config.keyword_match.mappings.insert(
            "geometry|index".to_string(),
            vec!["geometry-system".to_string()],
        );

        let evaluator = CheckpointEvaluator::new(config);
        let mut state = SessionCheckpointState::new();

        // Should match
        let checkpoints = evaluator.on_input("Tell me about the geometry system", &mut state);
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].checkpoint_type, CheckpointType::KeywordMatch);
        assert_eq!(checkpoints[0].inject_contexts, vec!["geometry-system"]);

        // Should not match again (already matched)
        let checkpoints = evaluator.on_input("More about geometry", &mut state);
        assert!(checkpoints.is_empty());
    }

    #[test]
    fn test_risk_threshold() {
        let mut config = CheckpointConfig::default();
        config.risk_threshold.min_tier = RiskTier::High;
        config.risk_threshold.inject_contexts = vec!["high-risk-warning".to_string()];

        let evaluator = CheckpointEvaluator::new(config);
        let mut state = SessionCheckpointState::new();

        // Low risk - should not trigger
        let checkpoints = evaluator.on_action_pre("read_file", None, RiskTier::Low, &mut state);
        assert!(checkpoints.iter().all(|c| c.checkpoint_type != CheckpointType::RiskThreshold));

        // High risk - should trigger
        let checkpoints = evaluator.on_action_pre("delete_file", None, RiskTier::High, &mut state);
        let risk_checkpoint = checkpoints
            .iter()
            .find(|c| c.checkpoint_type == CheckpointType::RiskThreshold);
        assert!(risk_checkpoint.is_some());
        assert!(risk_checkpoint.unwrap().is_sync);
    }

    #[test]
    fn test_action_pre_wildcard() {
        let mut config = CheckpointConfig::default();
        config.action_pre.mappings.insert(
            "delete_*".to_string(),
            ActionCheckpointConfig {
                inject_contexts: vec!["deletion-warning".to_string()],
                require_policy_check: true,
                require_confirmation: true,
            },
        );

        let evaluator = CheckpointEvaluator::new(config);
        let mut state = SessionCheckpointState::new();

        let checkpoints = evaluator.on_action_pre("delete_user", None, RiskTier::High, &mut state);
        let action_checkpoint = checkpoints
            .iter()
            .find(|c| c.checkpoint_type == CheckpointType::ActionPre);
        assert!(action_checkpoint.is_some());
        assert_eq!(
            action_checkpoint.unwrap().inject_contexts,
            vec!["deletion-warning"]
        );
    }

    #[test]
    fn test_count_interval() {
        let mut config = CheckpointConfig::default();
        config.count_interval.enabled = true;
        config.count_interval.actions = 3;

        let evaluator = CheckpointEvaluator::new(config);
        let mut state = SessionCheckpointState::new();

        // Actions 1-2: no checkpoint
        for _ in 0..2 {
            let checkpoints = evaluator.on_action_pre("test", None, RiskTier::Low, &mut state);
            assert!(checkpoints
                .iter()
                .all(|c| c.checkpoint_type != CheckpointType::CountInterval));
        }

        // Action 3: should trigger
        let checkpoints = evaluator.on_action_pre("test", None, RiskTier::Low, &mut state);
        assert!(checkpoints
            .iter()
            .any(|c| c.checkpoint_type == CheckpointType::CountInterval));
    }

    #[test]
    fn test_checkpoint_priority_ordering() {
        let mut config = CheckpointConfig::default();
        config.keyword_match.mappings.insert(
            "test".to_string(),
            vec!["test-context".to_string()],
        );
        config.risk_threshold.min_tier = RiskTier::Low;

        let evaluator = CheckpointEvaluator::new(config);
        let mut state = SessionCheckpointState::new();

        // First, trigger keyword match
        let _ = evaluator.on_input("test input", &mut state);

        // Now trigger multiple checkpoints
        let checkpoints = evaluator.on_action_pre("test", None, RiskTier::High, &mut state);

        // Should be ordered by priority (risk > action_pre > keyword)
        if checkpoints.len() >= 2 {
            assert!(checkpoints[0].priority >= checkpoints[1].priority);
        }
    }

    #[test]
    fn test_steward_checkpoint_definition() {
        let checkpoint = StewardCheckpointDef::new(
            "onboarding",
            "Onboarding Checkpoint",
            CheckpointTrigger::SessionStart,
        )
        .blocking()
        .with_question(CheckpointQuestion::boolean(
            "agree-terms",
            "Do you agree to the terms of service?",
        ))
        .with_guidance(GuidanceBlock::text("Welcome to the system!"))
        .unlock_capabilities(vec!["basic-access".to_string()]);

        assert_eq!(checkpoint.mode, CheckpointMode::Blocking);
        assert!(checkpoint.requires_response());
        assert_eq!(checkpoint.questions.len(), 1);
        assert!(checkpoint.guidance.is_some());
        assert_eq!(checkpoint.unlock_capabilities, vec!["basic-access"]);
    }

    #[test]
    fn test_checkpoint_question_types() {
        // Text question
        let text_q = CheckpointQuestion::text("q1", "What is your name?");
        assert_eq!(text_q.response_type, ResponseType::Text);
        assert!(text_q.required);

        // Boolean question
        let bool_q = CheckpointQuestion::boolean("q2", "Are you ready?");
        assert_eq!(bool_q.response_type, ResponseType::Boolean);

        // Acknowledgment question
        let ack_q = CheckpointQuestion::acknowledgment("q3", "You must follow these guidelines.");
        assert_eq!(ack_q.response_type, ResponseType::Acknowledgment);
        assert!(ack_q.hint.is_some());

        // Choice question
        let choice_q = CheckpointQuestion::choice(
            "q4",
            "Select your role",
            vec!["admin".to_string(), "user".to_string()],
        );
        assert!(matches!(choice_q.response_type, ResponseType::Choice { .. }));
    }

    #[test]
    fn test_guidance_block() {
        let guidance = GuidanceBlock::system_instruction("Always be helpful.");
        assert_eq!(guidance.format, GuidanceFormat::SystemInstruction);
        assert_eq!(guidance.priority, 100);
        assert!(!guidance.append);

        let text_guidance = GuidanceBlock::text("Some helpful info")
            .with_priority(50)
            .with_labels(vec!["important".to_string()]);
        assert_eq!(text_guidance.format, GuidanceFormat::Text);
        assert_eq!(text_guidance.priority, 50);
        assert_eq!(text_guidance.labels, vec!["important"]);
    }

    #[test]
    fn test_checkpoint_validator_valid_response() {
        let checkpoint = TriggeredCheckpoint {
            checkpoint_type: CheckpointType::Interactive,
            priority: 500,
            inject_contexts: vec!["intro".to_string()],
            is_sync: true,
            trigger_data: None,
            steward_def: None,
            questions: vec![
                CheckpointQuestion::boolean("agree", "Do you agree?"),
            ],
            guidance: Some(GuidanceBlock::text("Welcome!")),
            mode: CheckpointMode::Blocking,
        };

        let mut answers = HashMap::new();
        answers.insert("agree".to_string(), AnswerValue::Boolean(true));

        let response = CheckpointResponse {
            checkpoint_id: "test".to_string(),
            answers,
            guidance_acknowledged: true,
            responded_at: "2024-01-01T00:00:00Z".to_string(),
            session_id: "session-1".to_string(),
        };

        let validation = CheckpointValidator::validate(&checkpoint, &response);
        assert!(validation.is_valid);
        assert!(validation.actions.iter().any(|a| matches!(a, CheckpointAction::Proceed)));
    }

    #[test]
    fn test_checkpoint_validator_invalid_response() {
        let checkpoint = TriggeredCheckpoint {
            checkpoint_type: CheckpointType::Interactive,
            priority: 500,
            inject_contexts: vec![],
            is_sync: true,
            trigger_data: None,
            steward_def: None,
            questions: vec![
                CheckpointQuestion::text("name", "What is your name?")
                    .with_validation(AnswerValidation {
                        pattern: None,
                        min_length: Some(3),
                        max_length: Some(100),
                        must_contain: vec![],
                        must_not_contain: vec!["forbidden".to_string()],
                        custom_validator: None,
                    }),
            ],
            guidance: None,
            mode: CheckpointMode::Blocking,
        };

        // Too short
        let mut answers = HashMap::new();
        answers.insert("name".to_string(), AnswerValue::Text("ab".to_string()));

        let response = CheckpointResponse {
            checkpoint_id: "test".to_string(),
            answers,
            guidance_acknowledged: false,
            responded_at: "2024-01-01T00:00:00Z".to_string(),
            session_id: "session-1".to_string(),
        };

        let validation = CheckpointValidator::validate(&checkpoint, &response);
        assert!(!validation.is_valid);
    }

    #[test]
    fn test_checkpoint_validator_forbidden_words() {
        let checkpoint = TriggeredCheckpoint {
            checkpoint_type: CheckpointType::Interactive,
            priority: 500,
            inject_contexts: vec![],
            is_sync: true,
            trigger_data: None,
            steward_def: None,
            questions: vec![
                CheckpointQuestion::text("response", "Describe your approach")
                    .with_validation(AnswerValidation {
                        pattern: None,
                        min_length: None,
                        max_length: None,
                        must_contain: vec![],
                        must_not_contain: vec!["hack".to_string(), "exploit".to_string()],
                        custom_validator: None,
                    }),
            ],
            guidance: None,
            mode: CheckpointMode::Blocking,
        };

        let mut answers = HashMap::new();
        answers.insert("response".to_string(), AnswerValue::Text("I will hack the system".to_string()));

        let response = CheckpointResponse {
            checkpoint_id: "test".to_string(),
            answers,
            guidance_acknowledged: false,
            responded_at: "2024-01-01T00:00:00Z".to_string(),
            session_id: "session-1".to_string(),
        };

        let validation = CheckpointValidator::validate(&checkpoint, &response);
        assert!(!validation.is_valid);
    }

    #[test]
    fn test_steward_checkpoint_evaluation() {
        let checkpoint_def = StewardCheckpointDef::new(
            "capability-gate",
            "Access Sensitive Data",
            CheckpointTrigger::CapabilityAccess {
                capability_ids: vec!["sensitive-data".to_string()],
            },
        )
        .blocking()
        .with_question(CheckpointQuestion::acknowledgment(
            "ack-sensitive",
            "You are about to access sensitive data. I will handle it responsibly.",
        ))
        .unlock_capabilities(vec!["sensitive-data".to_string()])
        .with_priority(900);

        let evaluator = CheckpointEvaluator::with_defaults();
        let triggered = evaluator.evaluate_steward_checkpoint(&checkpoint_def, None);

        assert_eq!(triggered.checkpoint_type, CheckpointType::Interactive);
        assert_eq!(triggered.priority, 900);
        assert!(triggered.is_sync);
        assert!(triggered.requires_response());
        assert_eq!(triggered.questions.len(), 1);
        assert_eq!(triggered.unlocked_capabilities(), vec!["sensitive-data"]);
    }
}
