//! Policy evaluation engine for CARP
//!
//! Policies are evaluated in a specific order:
//! 1. Deny policies (immediate rejection)
//! 2. Approval policies (require human approval)
//! 3. Rate limit policies (throttle if exceeded)
//! 4. Allow policies (explicit allowance)
//!
//! If no policy matches, the default behavior is to allow the action.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::atlas::{AtlasPolicy, PolicyType};

/// Result of evaluating a policy against an action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    /// Action is allowed
    Allow,
    /// Action is allowed with constraints
    AllowWithConstraints(Vec<String>),
    /// Action is denied
    Deny { policy_id: String, reason: String },
    /// Action requires approval
    RequiresApproval { policy_id: String },
    /// Rate limit exceeded
    RateLimitExceeded { policy_id: String, retry_after: u64 },
    /// No matching policy
    NoMatch,
}

impl PolicyResult {
    /// Check if this result allows the action
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyResult::Allow | PolicyResult::AllowWithConstraints(_))
    }

    /// Check if this result denies the action
    pub fn is_denied(&self) -> bool {
        matches!(self, PolicyResult::Deny { .. })
    }
}

/// Policy evaluator that processes policies in the correct order
#[derive(Debug)]
pub struct PolicyEvaluator {
    /// Policies grouped by type
    policies: Vec<AtlasPolicy>,

    /// Rate limit state (action_id -> (count, window_start))
    rate_limit_state: HashMap<String, RateLimitState>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    count: u64,
    window_start: Instant,
    max_calls: u64,
    window_seconds: u64,
}

/// Check if an action matches any of the policy patterns
fn matches_action(patterns: &[String], action_id: &str) -> bool {
    patterns.iter().any(|pattern| pattern_matches(pattern, action_id))
}

/// Match a pattern against an action ID
///
/// Supports:
/// - Exact match: "ticket.get"
/// - Wildcard suffix: "ticket.*"
/// - Wildcard prefix: "*.delete"
/// - Full wildcard: "*"
fn pattern_matches(pattern: &str, action_id: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern == action_id {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix(".*") {
        return action_id.starts_with(prefix) && action_id[prefix.len()..].starts_with('.');
    }

    if let Some(suffix) = pattern.strip_prefix("*.") {
        return action_id.ends_with(suffix) && action_id[..action_id.len() - suffix.len()].ends_with('.');
    }

    false
}

impl PolicyEvaluator {
    /// Create a new policy evaluator
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            rate_limit_state: HashMap::new(),
        }
    }

    /// Add policies from an atlas
    pub fn add_policies(&mut self, policies: Vec<AtlasPolicy>) {
        self.policies.extend(policies);
    }

    /// Clear all policies
    pub fn clear_policies(&mut self) {
        self.policies.clear();
        self.rate_limit_state.clear();
    }

    /// Evaluate all policies for a given action
    ///
    /// Returns the first matching result in priority order:
    /// deny -> requires_approval -> rate_limit -> allow -> no_match
    pub fn evaluate(&mut self, action_id: &str) -> PolicyResult {
        // Phase 1: Check deny policies
        for policy in self.policies.iter().filter(|p| p.policy_type == PolicyType::Deny) {
            if matches_action(&policy.actions, action_id) {
                return PolicyResult::Deny {
                    policy_id: policy.policy_id.clone(),
                    reason: policy.reason.clone().unwrap_or_else(|| "Denied by policy".to_string()),
                };
            }
        }

        // Phase 2: Check approval policies
        for policy in self.policies.iter().filter(|p| p.policy_type == PolicyType::RequiresApproval) {
            if matches_action(&policy.actions, action_id) {
                return PolicyResult::RequiresApproval {
                    policy_id: policy.policy_id.clone(),
                };
            }
        }

        // Phase 3: Check rate limit policies
        // Collect matching rate limit policies first to avoid borrow issues
        let rate_limit_matches: Vec<_> = self
            .policies
            .iter()
            .filter(|p| p.policy_type == PolicyType::RateLimit)
            .filter(|p| matches_action(&p.actions, action_id))
            .cloned()
            .collect();

        for policy in rate_limit_matches {
            if let Some(result) = self.check_rate_limit(action_id, &policy) {
                return result;
            }
        }

        // Phase 4: Check allow policies (explicit allow)
        for policy in self.policies.iter().filter(|p| p.policy_type == PolicyType::Allow) {
            if matches_action(&policy.actions, action_id) {
                return PolicyResult::Allow;
            }
        }

        // Default: no matching policy means allow
        PolicyResult::NoMatch
    }

    /// Match a pattern against an action ID
    ///
    /// Supports:
    /// - Exact match: "ticket.get"
    /// - Wildcard suffix: "ticket.*"
    /// - Wildcard prefix: "*.delete"
    /// - Full wildcard: "*"
    pub fn pattern_matches(&self, pattern: &str, action_id: &str) -> bool {
        pattern_matches(pattern, action_id)
    }

    /// Check rate limit for an action
    fn check_rate_limit(&mut self, action_id: &str, policy: &AtlasPolicy) -> Option<PolicyResult> {
        let params = policy.parameters.as_ref()?;
        let max_calls = params.get("max_calls")?.as_u64()?;
        let window_seconds = params.get("window_seconds")?.as_u64()?;

        let now = Instant::now();
        let key = format!("{}:{}", policy.policy_id, action_id);

        let state = self.rate_limit_state.entry(key.clone()).or_insert_with(|| {
            RateLimitState {
                count: 0,
                window_start: now,
                max_calls,
                window_seconds,
            }
        });

        // Check if window has expired
        let window = Duration::from_secs(state.window_seconds);
        if now.duration_since(state.window_start) > window {
            // Reset window
            state.count = 0;
            state.window_start = now;
        }

        // Check if limit exceeded
        if state.count >= state.max_calls {
            let elapsed = now.duration_since(state.window_start);
            let retry_after = state.window_seconds.saturating_sub(elapsed.as_secs());
            return Some(PolicyResult::RateLimitExceeded {
                policy_id: policy.policy_id.clone(),
                retry_after,
            });
        }

        // Increment counter
        state.count += 1;

        None
    }

    /// Reset rate limit state for testing or session end
    pub fn reset_rate_limits(&mut self) {
        self.rate_limit_state.clear();
    }

    /// Get the current count for a rate-limited action
    pub fn get_rate_limit_count(&self, policy_id: &str, action_id: &str) -> Option<u64> {
        let key = format!("{}:{}", policy_id, action_id);
        self.rate_limit_state.get(&key).map(|s| s.count)
    }
}

impl Default for PolicyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct for serializing policy evaluation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub policy_id: String,
    pub policy_type: String,
    pub action_id: String,
    pub result: String,
    pub reason: Option<String>,
}

impl PolicyEvaluation {
    pub fn from_result(action_id: &str, result: &PolicyResult) -> Option<Self> {
        match result {
            PolicyResult::Deny { policy_id, reason } => Some(Self {
                policy_id: policy_id.clone(),
                policy_type: "deny".to_string(),
                action_id: action_id.to_string(),
                result: "denied".to_string(),
                reason: Some(reason.clone()),
            }),
            PolicyResult::RequiresApproval { policy_id } => Some(Self {
                policy_id: policy_id.clone(),
                policy_type: "requires_approval".to_string(),
                action_id: action_id.to_string(),
                result: "requires_approval".to_string(),
                reason: None,
            }),
            PolicyResult::RateLimitExceeded { policy_id, retry_after } => Some(Self {
                policy_id: policy_id.clone(),
                policy_type: "rate_limit".to_string(),
                action_id: action_id.to_string(),
                result: "rate_limit_exceeded".to_string(),
                reason: Some(format!("Retry after {} seconds", retry_after)),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_policies() -> Vec<AtlasPolicy> {
        vec![
            AtlasPolicy {
                policy_id: "deny-delete".to_string(),
                policy_type: PolicyType::Deny,
                actions: vec!["*.delete".to_string()],
                reason: Some("Deletion requires manual approval".to_string()),
                parameters: None,
            },
            AtlasPolicy {
                policy_id: "approve-high-risk".to_string(),
                policy_type: PolicyType::RequiresApproval,
                actions: vec!["payment.*".to_string()],
                reason: None,
                parameters: None,
            },
            AtlasPolicy {
                policy_id: "rate-limit-api".to_string(),
                policy_type: PolicyType::RateLimit,
                actions: vec!["ticket.*".to_string()],
                reason: None,
                parameters: Some(json!({
                    "max_calls": 5,
                    "window_seconds": 60
                })),
            },
        ]
    }

    #[test]
    fn test_deny_policy() {
        let mut evaluator = PolicyEvaluator::new();
        evaluator.add_policies(create_test_policies());

        let result = evaluator.evaluate("ticket.delete");
        assert!(matches!(result, PolicyResult::Deny { .. }));

        if let PolicyResult::Deny { policy_id, reason } = result {
            assert_eq!(policy_id, "deny-delete");
            assert!(reason.contains("manual approval"));
        }
    }

    #[test]
    fn test_approval_policy() {
        let mut evaluator = PolicyEvaluator::new();
        evaluator.add_policies(create_test_policies());

        let result = evaluator.evaluate("payment.process");
        assert!(matches!(result, PolicyResult::RequiresApproval { .. }));
    }

    #[test]
    fn test_rate_limit_policy() {
        let mut evaluator = PolicyEvaluator::new();
        evaluator.add_policies(create_test_policies());

        // First 5 calls should succeed
        for _ in 0..5 {
            let result = evaluator.evaluate("ticket.get");
            assert!(!matches!(result, PolicyResult::RateLimitExceeded { .. }));
        }

        // 6th call should be rate limited
        let result = evaluator.evaluate("ticket.get");
        assert!(matches!(result, PolicyResult::RateLimitExceeded { .. }));
    }

    #[test]
    fn test_pattern_matching() {
        let evaluator = PolicyEvaluator::new();

        // Wildcard suffix
        assert!(evaluator.pattern_matches("ticket.*", "ticket.get"));
        assert!(evaluator.pattern_matches("ticket.*", "ticket.delete"));
        assert!(!evaluator.pattern_matches("ticket.*", "user.get"));

        // Wildcard prefix
        assert!(evaluator.pattern_matches("*.delete", "ticket.delete"));
        assert!(evaluator.pattern_matches("*.delete", "user.delete"));
        assert!(!evaluator.pattern_matches("*.delete", "ticket.get"));

        // Full wildcard
        assert!(evaluator.pattern_matches("*", "anything"));

        // Exact match
        assert!(evaluator.pattern_matches("ticket.get", "ticket.get"));
        assert!(!evaluator.pattern_matches("ticket.get", "ticket.list"));
    }

    #[test]
    fn test_no_matching_policy() {
        let mut evaluator = PolicyEvaluator::new();
        evaluator.add_policies(create_test_policies());

        // Action with no matching policy
        let result = evaluator.evaluate("unknown.action");
        assert!(matches!(result, PolicyResult::NoMatch));
    }

    #[test]
    fn test_policy_priority() {
        let mut evaluator = PolicyEvaluator::new();

        // Add policies that could match the same action
        evaluator.add_policies(vec![
            AtlasPolicy {
                policy_id: "allow-all".to_string(),
                policy_type: PolicyType::Allow,
                actions: vec!["*".to_string()],
                reason: None,
                parameters: None,
            },
            AtlasPolicy {
                policy_id: "deny-delete".to_string(),
                policy_type: PolicyType::Deny,
                actions: vec!["*.delete".to_string()],
                reason: Some("No deletes".to_string()),
                parameters: None,
            },
        ]);

        // Deny should take precedence over allow
        let result = evaluator.evaluate("ticket.delete");
        assert!(matches!(result, PolicyResult::Deny { .. }));
    }
}
