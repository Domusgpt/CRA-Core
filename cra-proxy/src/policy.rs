//! Policy checking with parameter inspection
//!
//! Unlike basic CRA which only matches action names, the proxy
//! can inspect actual request content for policy decisions.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Policy decision
#[derive(Debug, Clone, Serialize)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

/// Request context for policy evaluation
#[derive(Debug, Clone, Deserialize)]
pub struct RequestContext {
    pub target_url: String,
    pub method: String,
    pub body: Option<serde_json::Value>,
    pub timer_id: Option<String>,
    pub session_id: Option<String>,
}

/// Parameter-aware policy checker
pub struct PolicyChecker {
    /// Deny patterns for URLs
    url_deny_patterns: Vec<(Regex, String)>,
    /// Deny patterns for request bodies
    body_deny_patterns: Vec<(Regex, String)>,
    /// Allowed URL patterns (if empty, all non-denied are allowed)
    url_allow_patterns: Vec<Regex>,
}

impl PolicyChecker {
    pub fn new() -> Self {
        Self {
            url_deny_patterns: vec![],
            body_deny_patterns: vec![],
            url_allow_patterns: vec![],
        }
    }

    /// Create a checker with default security rules
    pub fn with_defaults() -> Self {
        let mut checker = Self::new();

        // Deny dangerous command patterns in body
        checker.add_body_deny_pattern(
            r#"(rm\s+-rf|sudo\s+|chmod\s+777|curl.*\|\s*sh)"#,
            "Dangerous command pattern detected",
        );

        // Deny internal network URLs (SSRF protection)
        checker.add_url_deny_pattern(
            r#"^https?://(10\.|172\.(1[6-9]|2[0-9]|3[01])\.|192\.168\.)"#,
            "Internal network URLs not allowed",
        );

        // Deny metadata endpoints (cloud SSRF)
        checker.add_url_deny_pattern(
            r#"169\.254\.169\.254"#,
            "Cloud metadata endpoint blocked",
        );

        checker
    }

    /// Add a URL deny pattern
    pub fn add_url_deny_pattern(&mut self, pattern: &str, reason: &str) {
        if let Ok(regex) = Regex::new(pattern) {
            self.url_deny_patterns.push((regex, reason.to_string()));
        }
    }

    /// Add a body deny pattern
    pub fn add_body_deny_pattern(&mut self, pattern: &str, reason: &str) {
        if let Ok(regex) = Regex::new(pattern) {
            self.body_deny_patterns.push((regex, reason.to_string()));
        }
    }

    /// Add an allowed URL pattern
    pub fn add_url_allow_pattern(&mut self, pattern: &str) {
        if let Ok(regex) = Regex::new(pattern) {
            self.url_allow_patterns.push(regex);
        }
    }

    /// Check a request against policies
    pub fn check(&self, ctx: &RequestContext) -> PolicyDecision {
        // Check URL deny patterns
        for (pattern, reason) in &self.url_deny_patterns {
            if pattern.is_match(&ctx.target_url) {
                return PolicyDecision::Deny {
                    reason: reason.clone(),
                };
            }
        }

        // Check body deny patterns
        if let Some(body) = &ctx.body {
            let body_str = serde_json::to_string(body).unwrap_or_default();
            for (pattern, reason) in &self.body_deny_patterns {
                if pattern.is_match(&body_str) {
                    return PolicyDecision::Deny {
                        reason: reason.clone(),
                    };
                }
            }
        }

        // If allow patterns exist, check them
        if !self.url_allow_patterns.is_empty() {
            let allowed = self.url_allow_patterns.iter().any(|p| p.is_match(&ctx.target_url));
            if !allowed {
                return PolicyDecision::Deny {
                    reason: "URL does not match any allowed pattern".to_string(),
                };
            }
        }

        PolicyDecision::Allow
    }
}

impl Default for PolicyChecker {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_command_blocked() {
        let checker = PolicyChecker::with_defaults();

        let ctx = RequestContext {
            target_url: "http://localhost:3000/execute".to_string(),
            method: "POST".to_string(),
            body: Some(serde_json::json!({"command": "sudo rm -rf /"})),
            timer_id: None,
            session_id: None,
        };

        match checker.check(&ctx) {
            PolicyDecision::Deny { reason } => {
                assert!(reason.contains("Dangerous"));
            }
            PolicyDecision::Allow => panic!("Should have been denied"),
        }
    }

    #[test]
    fn test_internal_network_blocked() {
        let checker = PolicyChecker::with_defaults();

        let ctx = RequestContext {
            target_url: "http://192.168.1.1/admin".to_string(),
            method: "GET".to_string(),
            body: None,
            timer_id: None,
            session_id: None,
        };

        match checker.check(&ctx) {
            PolicyDecision::Deny { reason } => {
                assert!(reason.contains("Internal network"));
            }
            PolicyDecision::Allow => panic!("Should have been denied"),
        }
    }

    #[test]
    fn test_safe_request_allowed() {
        let checker = PolicyChecker::with_defaults();

        let ctx = RequestContext {
            target_url: "https://api.example.com/webhook".to_string(),
            method: "POST".to_string(),
            body: Some(serde_json::json!({"event": "timer.fired"})),
            timer_id: Some("timer-123".to_string()),
            session_id: None,
        };

        match checker.check(&ctx) {
            PolicyDecision::Allow => {}
            PolicyDecision::Deny { reason } => panic!("Should have been allowed: {}", reason),
        }
    }
}
