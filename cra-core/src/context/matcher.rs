//! Context Matcher - evaluates conditions for context injection
//!
//! Determines which context packs should be injected based on:
//! - Goal text (keyword matching)
//! - Risk tier
//! - Context hints from request
//! - Custom conditions from pack definition

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::carp::RiskTier;

/// Result of matching a context pack against a request
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Whether the pack should be included
    pub matched: bool,
    /// Score for ranking (higher = more relevant)
    pub score: MatchScore,
    /// Reason for match/no-match
    pub reason: String,
}

/// Scoring for context relevance
#[derive(Debug, Clone, Default)]
pub struct MatchScore {
    /// Base priority from pack definition
    pub priority: i32,
    /// Keyword match score
    pub keyword_score: i32,
    /// Hint match score
    pub hint_score: i32,
    /// Risk tier match score
    pub risk_score: i32,
}

impl MatchScore {
    /// Total score for sorting
    pub fn total(&self) -> i32 {
        self.priority + self.keyword_score + self.hint_score + self.risk_score
    }
}

/// Context matcher for evaluating pack conditions
#[derive(Debug, Default)]
pub struct ContextMatcher {
    /// Minimum score threshold for inclusion
    pub min_score: i32,
    /// Maximum number of blocks to include
    pub max_blocks: usize,
    /// Whether to include packs with no conditions (default: true)
    pub include_unconditional: bool,
}

impl ContextMatcher {
    /// Create a new matcher with defaults
    pub fn new() -> Self {
        Self {
            min_score: 0,
            max_blocks: 10,
            include_unconditional: true,
        }
    }

    /// Set minimum score threshold
    pub fn with_min_score(mut self, score: i32) -> Self {
        self.min_score = score;
        self
    }

    /// Set maximum blocks to return
    pub fn with_max_blocks(mut self, max: usize) -> Self {
        self.max_blocks = max;
        self
    }

    /// Evaluate if a context pack matches the request
    pub fn evaluate(
        &self,
        conditions: Option<&Value>,
        goal: &str,
        risk_tier: Option<RiskTier>,
        context_hints: &[String],
        pack_priority: i32,
    ) -> MatchResult {
        let mut score = MatchScore {
            priority: pack_priority,
            ..Default::default()
        };

        // No conditions = always match (if enabled)
        let Some(conditions) = conditions else {
            return MatchResult {
                matched: self.include_unconditional,
                score,
                reason: "No conditions (unconditional)".to_string(),
            };
        };

        let goal_lower = goal.to_lowercase();
        let mut matched_any = false;

        // Check keyword conditions
        if let Some(keywords) = conditions.get("keywords").and_then(|v| v.as_array()) {
            for keyword in keywords {
                if let Some(kw) = keyword.as_str() {
                    if goal_lower.contains(&kw.to_lowercase()) {
                        score.keyword_score += 20;
                        matched_any = true;
                    }
                }
            }
        }

        // Check risk tier conditions
        if let Some(risk_tiers) = conditions.get("risk_tiers").and_then(|v| v.as_array()) {
            if let Some(request_tier) = risk_tier {
                let tier_str = request_tier.to_string();
                for tier in risk_tiers {
                    if let Some(t) = tier.as_str() {
                        if t.eq_ignore_ascii_case(&tier_str) {
                            score.risk_score += 30;
                            matched_any = true;
                        }
                    }
                }
            }
        }

        // Check context hint conditions
        if let Some(hints) = conditions.get("context_hints").and_then(|v| v.as_array()) {
            for hint in hints {
                if let Some(h) = hint.as_str() {
                    for request_hint in context_hints {
                        if request_hint.eq_ignore_ascii_case(h) {
                            score.hint_score += 25;
                            matched_any = true;
                        }
                    }
                }
            }
        }

        // Check file pattern conditions (for dev tooling)
        if let Some(pattern) = conditions.get("file_pattern").and_then(|v| v.as_str()) {
            // Extract meaningful parts from pattern
            let parts: Vec<&str> = pattern.split(&['/', '*', '.'][..])
                .filter(|p| p.len() > 2)
                .collect();

            for part in parts {
                if goal_lower.contains(&part.to_lowercase()) {
                    score.keyword_score += 15;
                    matched_any = true;
                }
            }
        }

        // Check inject_when conditions (action-based)
        if let Some(actions) = conditions.get("inject_when").and_then(|v| v.as_array()) {
            for action in actions {
                if let Some(act) = action.as_str() {
                    // Check if goal mentions this action type
                    let action_parts: Vec<&str> = act.split('.').collect();
                    for part in action_parts {
                        if goal_lower.contains(&part.to_lowercase()) {
                            score.keyword_score += 10;
                            matched_any = true;
                        }
                    }
                }
            }
        }

        // Determine if matched based on score
        let total_score = score.total();
        let matched = matched_any && total_score >= self.min_score;

        let reason = if matched {
            format!("Matched with score {}", total_score)
        } else if !matched_any {
            "No condition matched".to_string()
        } else {
            format!("Score {} below threshold {}", total_score, self.min_score)
        };

        MatchResult {
            matched,
            score,
            reason,
        }
    }
}

/// Condition builder for creating pack conditions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConditionBuilder {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    keywords: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    risk_tiers: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    context_hints: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_pattern: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    inject_when: Vec<String>,
}

impl ConditionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keyword(mut self, kw: impl Into<String>) -> Self {
        self.keywords.push(kw.into());
        self
    }

    pub fn keywords(mut self, kws: Vec<String>) -> Self {
        self.keywords.extend(kws);
        self
    }

    pub fn risk_tier(mut self, tier: impl Into<String>) -> Self {
        self.risk_tiers.push(tier.into());
        self
    }

    pub fn context_hint(mut self, hint: impl Into<String>) -> Self {
        self.context_hints.push(hint.into());
        self
    }

    pub fn file_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.file_pattern = Some(pattern.into());
        self
    }

    pub fn inject_when(mut self, action: impl Into<String>) -> Self {
        self.inject_when.push(action.into());
        self
    }

    pub fn build(self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_matching() {
        let matcher = ContextMatcher::new();
        let conditions = ConditionBuilder::new()
            .keyword("hash")
            .keyword("trace")
            .build();

        let result = matcher.evaluate(
            Some(&conditions),
            "working on hash chain implementation",
            None,
            &[],
            50,
        );

        assert!(result.matched);
        assert!(result.score.keyword_score > 0);
    }

    #[test]
    fn test_risk_tier_matching() {
        let matcher = ContextMatcher::new();
        let conditions = ConditionBuilder::new()
            .risk_tier("high")
            .risk_tier("critical")
            .build();

        // Should match high risk
        let result = matcher.evaluate(
            Some(&conditions),
            "some goal",
            Some(RiskTier::High),
            &[],
            50,
        );
        assert!(result.matched);
        assert!(result.score.risk_score > 0);

        // Should not match low risk
        let result = matcher.evaluate(
            Some(&conditions),
            "some goal",
            Some(RiskTier::Low),
            &[],
            50,
        );
        assert!(!result.matched);
    }

    #[test]
    fn test_context_hint_matching() {
        let matcher = ContextMatcher::new();
        let conditions = ConditionBuilder::new()
            .context_hint("security")
            .context_hint("audit")
            .build();

        let result = matcher.evaluate(
            Some(&conditions),
            "some goal",
            None,
            &["security".to_string()],
            50,
        );

        assert!(result.matched);
        assert!(result.score.hint_score > 0);
    }

    #[test]
    fn test_unconditional_pack() {
        let matcher = ContextMatcher::new();

        // No conditions = always match
        let result = matcher.evaluate(
            None,
            "any goal",
            None,
            &[],
            100,
        );

        assert!(result.matched);
        assert_eq!(result.score.priority, 100);
    }

    #[test]
    fn test_file_pattern_matching() {
        let matcher = ContextMatcher::new();
        let conditions = ConditionBuilder::new()
            .file_pattern("trace/*.rs")
            .build();

        let result = matcher.evaluate(
            Some(&conditions),
            "editing trace event code",
            None,
            &[],
            50,
        );

        assert!(result.matched);
    }
}
