//! Atlas Validator
//!
//! Provides comprehensive validation for atlas manifests including:
//! - Schema validation
//! - Semantic validation
//! - Dependency resolution
//! - Cross-reference checking

use super::manifest::{AtlasManifest, AtlasPolicy, PolicyType};

/// Validation result with detailed findings
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,

    /// Error-level issues that must be fixed
    pub errors: Vec<ValidationIssue>,

    /// Warning-level issues that should be addressed
    pub warnings: Vec<ValidationIssue>,

    /// Informational notes
    pub info: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            info: vec![],
        }
    }

    /// Add an error
    pub fn add_error(&mut self, issue: ValidationIssue) {
        self.is_valid = false;
        self.errors.push(issue);
    }

    /// Add a warning
    pub fn add_warning(&mut self, issue: ValidationIssue) {
        self.warnings.push(issue);
    }

    /// Add info
    pub fn add_info(&mut self, issue: ValidationIssue) {
        self.info.push(issue);
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "{}: {} errors, {} warnings, {} info",
            if self.is_valid { "VALID" } else { "INVALID" },
            self.errors.len(),
            self.warnings.len(),
            self.info.len()
        )
    }
}

/// A single validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Issue code
    pub code: String,

    /// Human-readable message
    pub message: String,

    /// Path to the problematic element (e.g., "actions[0].action_id")
    pub path: Option<String>,

    /// Suggested fix
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    /// Create a new issue
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            path: None,
            suggestion: None,
        }
    }

    /// Set the path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Atlas validator
pub struct AtlasValidator {
    /// Whether to check for recommended practices
    check_recommendations: bool,

    /// Known action ID patterns (for cross-referencing)
    known_patterns: Vec<String>,
}

impl AtlasValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            check_recommendations: true,
            known_patterns: vec![],
        }
    }

    /// Disable recommendation checks
    pub fn skip_recommendations(mut self) -> Self {
        self.check_recommendations = false;
        self
    }

    /// Add known patterns for validation
    pub fn with_known_patterns(mut self, patterns: Vec<String>) -> Self {
        self.known_patterns = patterns;
        self
    }

    /// Validate an atlas manifest
    pub fn validate(&self, manifest: &AtlasManifest) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Core validations
        self.validate_version(&manifest, &mut result);
        self.validate_identifiers(&manifest, &mut result);
        self.validate_actions(&manifest, &mut result);
        self.validate_policies(&manifest, &mut result);
        self.validate_capabilities(&manifest, &mut result);
        self.validate_context_packs(&manifest, &mut result);

        // Recommendations
        if self.check_recommendations {
            self.check_recommendations(&manifest, &mut result);
        }

        result
    }

    fn validate_version(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        if manifest.atlas_version != super::VERSION {
            result.add_error(
                ValidationIssue::new(
                    "E001",
                    format!(
                        "Unsupported atlas_version: expected {}, got {}",
                        super::VERSION,
                        manifest.atlas_version
                    ),
                )
                .with_path("atlas_version"),
            );
        }

        // Validate semantic version format
        if !is_valid_semver(&manifest.version) {
            result.add_error(
                ValidationIssue::new(
                    "E002",
                    format!("Invalid version format: {}", manifest.version),
                )
                .with_path("version")
                .with_suggestion("Use semantic versioning (e.g., 1.0.0)"),
            );
        }
    }

    fn validate_identifiers(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        // Atlas ID format
        if manifest.atlas_id.is_empty() {
            result.add_error(
                ValidationIssue::new("E003", "atlas_id cannot be empty").with_path("atlas_id"),
            );
        } else if !is_valid_atlas_id(&manifest.atlas_id) {
            result.add_warning(
                ValidationIssue::new(
                    "W001",
                    format!(
                        "atlas_id '{}' doesn't follow reverse-domain notation",
                        manifest.atlas_id
                    ),
                )
                .with_path("atlas_id")
                .with_suggestion("Use reverse-domain notation (e.g., com.company.atlas-name)"),
            );
        }

        // Name
        if manifest.name.is_empty() {
            result.add_error(
                ValidationIssue::new("E004", "name cannot be empty").with_path("name"),
            );
        }
    }

    fn validate_actions(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        let mut seen_ids = std::collections::HashSet::new();

        for (i, action) in manifest.actions.iter().enumerate() {
            let path = format!("actions[{}]", i);

            // Check for duplicates
            if !seen_ids.insert(&action.action_id) {
                result.add_error(
                    ValidationIssue::new(
                        "E005",
                        format!("Duplicate action_id: {}", action.action_id),
                    )
                    .with_path(&format!("{}.action_id", path)),
                );
            }

            // Validate action ID format
            if !is_valid_action_id(&action.action_id) {
                result.add_warning(
                    ValidationIssue::new(
                        "W002",
                        format!(
                            "action_id '{}' doesn't follow recommended format",
                            action.action_id
                        ),
                    )
                    .with_path(&format!("{}.action_id", path))
                    .with_suggestion("Use dotted notation (e.g., resource.verb)"),
                );
            }

            // Validate parameters schema
            if !action.parameters_schema.is_object() {
                result.add_error(
                    ValidationIssue::new("E006", "parameters_schema must be an object")
                        .with_path(&format!("{}.parameters_schema", path)),
                );
            }

            // Validate risk tier
            if !["low", "medium", "high", "critical"].contains(&action.risk_tier.as_str()) {
                result.add_warning(
                    ValidationIssue::new(
                        "W003",
                        format!("Unknown risk_tier: {}", action.risk_tier),
                    )
                    .with_path(&format!("{}.risk_tier", path))
                    .with_suggestion("Use one of: low, medium, high, critical"),
                );
            }
        }
    }

    fn validate_policies(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        let mut seen_ids = std::collections::HashSet::new();

        for (i, policy) in manifest.policies.iter().enumerate() {
            let path = format!("policies[{}]", i);

            // Check for duplicates
            if !seen_ids.insert(&policy.policy_id) {
                result.add_error(
                    ValidationIssue::new(
                        "E007",
                        format!("Duplicate policy_id: {}", policy.policy_id),
                    )
                    .with_path(&format!("{}.policy_id", path)),
                );
            }

            // Validate policy has actions
            if policy.actions.is_empty() {
                result.add_error(
                    ValidationIssue::new("E008", "Policy must have at least one action pattern")
                        .with_path(&format!("{}.actions", path)),
                );
            }

            // Validate rate limit parameters
            if policy.policy_type == PolicyType::RateLimit {
                self.validate_rate_limit_params(policy, &path, result);
            }

            // Validate action patterns
            for (j, pattern) in policy.actions.iter().enumerate() {
                if !is_valid_action_pattern(pattern) {
                    result.add_warning(
                        ValidationIssue::new(
                            "W004",
                            format!("Invalid action pattern: {}", pattern),
                        )
                        .with_path(&format!("{}.actions[{}]", path, j)),
                    );
                }
            }
        }
    }

    fn validate_rate_limit_params(
        &self,
        policy: &AtlasPolicy,
        path: &str,
        result: &mut ValidationResult,
    ) {
        match &policy.parameters {
            Some(params) => {
                if params.get("max_calls").is_none() {
                    result.add_error(
                        ValidationIssue::new(
                            "E009",
                            "Rate limit policy must have max_calls parameter",
                        )
                        .with_path(&format!("{}.parameters.max_calls", path)),
                    );
                }
                if params.get("window_seconds").is_none() {
                    result.add_error(
                        ValidationIssue::new(
                            "E010",
                            "Rate limit policy must have window_seconds parameter",
                        )
                        .with_path(&format!("{}.parameters.window_seconds", path)),
                    );
                }
            }
            None => {
                result.add_error(
                    ValidationIssue::new(
                        "E011",
                        "Rate limit policy must have parameters",
                    )
                    .with_path(&format!("{}.parameters", path)),
                );
            }
        }
    }

    fn validate_capabilities(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        let action_ids: std::collections::HashSet<&str> = manifest
            .actions
            .iter()
            .map(|a| a.action_id.as_str())
            .collect();

        for (i, capability) in manifest.capabilities.iter().enumerate() {
            let path = format!("capabilities[{}]", i);

            // Check referenced actions exist
            for (j, action_id) in capability.actions.iter().enumerate() {
                if !action_ids.contains(action_id.as_str()) {
                    result.add_error(
                        ValidationIssue::new(
                            "E012",
                            format!("Capability references unknown action: {}", action_id),
                        )
                        .with_path(&format!("{}.actions[{}]", path, j)),
                    );
                }
            }

            // Empty capability warning
            if capability.actions.is_empty() {
                result.add_warning(
                    ValidationIssue::new("W005", "Capability has no actions")
                        .with_path(&format!("{}.actions", path)),
                );
            }
        }
    }

    fn validate_context_packs(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        for (i, pack) in manifest.context_packs.iter().enumerate() {
            let path = format!("context_packs[{}]", i);

            if pack.files.is_empty() {
                result.add_warning(
                    ValidationIssue::new("W006", "Context pack has no files")
                        .with_path(&format!("{}.files", path)),
                );
            }
        }
    }

    fn check_recommendations(&self, manifest: &AtlasManifest, result: &mut ValidationResult) {
        // License
        if manifest.license.is_none() {
            result.add_info(
                ValidationIssue::new("I001", "No license specified")
                    .with_path("license")
                    .with_suggestion("Add an SPDX license identifier"),
            );
        }

        // Description
        if manifest.description.is_empty() {
            result.add_info(
                ValidationIssue::new("I002", "No description provided")
                    .with_path("description")
                    .with_suggestion("Add a description to help users understand this atlas"),
            );
        }

        // Authors
        if manifest.authors.is_empty() {
            result.add_info(
                ValidationIssue::new("I003", "No authors specified")
                    .with_path("authors")
                    .with_suggestion("Add author information"),
            );
        }

        // Domains
        if manifest.domains.is_empty() {
            result.add_info(
                ValidationIssue::new("I004", "No domains specified")
                    .with_path("domains")
                    .with_suggestion("Add domain tags for better discoverability"),
            );
        }
    }
}

impl Default for AtlasValidator {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u32>().is_ok())
}

fn is_valid_atlas_id(id: &str) -> bool {
    // Should be reverse-domain notation: com.company.name
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

fn is_valid_action_id(id: &str) -> bool {
    // Should be dotted notation: resource.verb
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

fn is_valid_action_pattern(pattern: &str) -> bool {
    // Patterns can include wildcards: *.delete, ticket.*, *
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" {
        return true;
    }
    pattern
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '*' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::{AtlasAction, AtlasCapability, AtlasPolicy};

    fn create_valid_manifest() -> AtlasManifest {
        AtlasManifest::builder("com.test.valid".to_string(), "Valid Atlas".to_string())
            .version("1.0.0")
            .description("A valid test atlas")
            .license("MIT")
            .domains(vec!["test".to_string()])
            .add_action(AtlasAction::new(
                "test.action".to_string(),
                "Test".to_string(),
                "Test action".to_string(),
            ))
            .add_policy(AtlasPolicy::deny(
                "deny-delete".to_string(),
                vec!["*.delete".to_string()],
                "No deletes".to_string(),
            ))
            .build()
    }

    #[test]
    fn test_validate_valid_manifest() {
        let manifest = create_valid_manifest();
        let validator = AtlasValidator::new();
        let result = validator.validate(&manifest);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_missing_fields() {
        let manifest = AtlasManifest {
            atlas_version: "1.0".to_string(),
            atlas_id: "".to_string(), // Invalid: empty
            version: "1.0.0".to_string(),
            name: "".to_string(), // Invalid: empty
            description: String::new(),
            authors: vec![],
            license: None,
            domains: vec![],
            capabilities: vec![],
            context_packs: vec![],
            policies: vec![],
            actions: vec![],
            dependencies: None,
        };

        let validator = AtlasValidator::new();
        let result = validator.validate(&manifest);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "E003"));
        assert!(result.errors.iter().any(|e| e.code == "E004"));
    }

    #[test]
    fn test_validate_duplicate_action_ids() {
        let mut manifest = create_valid_manifest();
        manifest.actions.push(AtlasAction::new(
            "test.action".to_string(), // Duplicate
            "Test 2".to_string(),
            "Test".to_string(),
        ));

        let validator = AtlasValidator::new();
        let result = validator.validate(&manifest);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "E005"));
    }

    #[test]
    fn test_validate_unknown_capability_action() {
        let mut manifest = create_valid_manifest();
        manifest.capabilities.push(AtlasCapability::new(
            "test.cap".to_string(),
            "Test Cap".to_string(),
            vec!["unknown.action".to_string()],
        ));

        let validator = AtlasValidator::new();
        let result = validator.validate(&manifest);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "E012"));
    }

    #[test]
    fn test_validate_rate_limit_params() {
        let mut manifest = create_valid_manifest();
        manifest.policies.push(AtlasPolicy {
            policy_id: "rate-limit-broken".to_string(),
            policy_type: PolicyType::RateLimit,
            actions: vec!["api.*".to_string()],
            reason: None,
            parameters: None, // Missing required params
        });

        let validator = AtlasValidator::new();
        let result = validator.validate(&manifest);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "E011"));
    }

    #[test]
    fn test_helper_functions() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.2"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.0"));

        assert!(is_valid_atlas_id("com.example.atlas"));
        assert!(is_valid_atlas_id("org.test.my-atlas"));
        assert!(!is_valid_atlas_id("single"));
        assert!(!is_valid_atlas_id(""));

        assert!(is_valid_action_id("ticket.get"));
        assert!(is_valid_action_id("api.v1.users.list"));
        assert!(!is_valid_action_id("single"));

        assert!(is_valid_action_pattern("*"));
        assert!(is_valid_action_pattern("*.delete"));
        assert!(is_valid_action_pattern("ticket.*"));
        assert!(is_valid_action_pattern("ticket.get"));
        assert!(!is_valid_action_pattern(""));
    }
}
