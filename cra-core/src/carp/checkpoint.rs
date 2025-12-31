//! Checkpoint System
//!
//! Checkpoints are moments when CRA intervenes - injecting context,
//! checking policies, or recording significant events.
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
}

impl CheckpointType {
    /// Get the default priority for this checkpoint type
    pub fn default_priority(&self) -> u32 {
        match self {
            CheckpointType::SessionStart => 1000,
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
        )
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
}
