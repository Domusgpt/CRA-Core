//! Steward Configuration
//!
//! Stewards are the creators/owners of atlases. This module defines
//! the configuration options for controlling access, delivery, and
//! notifications for atlas usage.

use serde::{Deserialize, Serialize};

/// Complete steward configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StewardConfig {
    /// Steward identity
    #[serde(default)]
    pub id: String,

    /// Human-readable name
    #[serde(default)]
    pub name: String,

    /// Contact email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,

    /// Access control configuration
    #[serde(default)]
    pub access: AccessConfig,

    /// Delivery configuration
    #[serde(default)]
    pub delivery: DeliveryConfig,

    /// Notification configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationConfig>,

    /// Analytics configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analytics: Option<AnalyticsConfig>,

    /// Branding configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branding: Option<BrandingConfig>,
}

impl StewardConfig {
    /// Create a new steward config with just an ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set contact
    pub fn with_contact(mut self, contact: impl Into<String>) -> Self {
        self.contact = Some(contact.into());
        self
    }

    /// Set access config
    pub fn with_access(mut self, access: AccessConfig) -> Self {
        self.access = access;
        self
    }

    /// Set delivery config
    pub fn with_delivery(mut self, delivery: DeliveryConfig) -> Self {
        self.delivery = delivery;
        self
    }

    /// Set notification config
    pub fn with_notifications(mut self, notifications: NotificationConfig) -> Self {
        self.notifications = Some(notifications);
        self
    }
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessConfig {
    /// Access type
    #[serde(default)]
    pub access_type: AccessType,

    /// Whether API key is required
    #[serde(default)]
    pub api_key_required: bool,

    /// Allowed domains (for API access)
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Blocked domains
    #[serde(default)]
    pub blocked_domains: Vec<String>,

    /// Rate limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<RateLimitConfig>,
}

impl Default for AccessConfig {
    fn default() -> Self {
        Self {
            access_type: AccessType::Public,
            api_key_required: false,
            allowed_domains: vec![],
            blocked_domains: vec![],
            rate_limits: None,
        }
    }
}

impl AccessConfig {
    /// Create a public access config (no restrictions)
    pub fn public() -> Self {
        Self::default()
    }

    /// Create an authenticated access config
    pub fn authenticated() -> Self {
        Self {
            access_type: AccessType::Authenticated,
            api_key_required: true,
            ..Default::default()
        }
    }

    /// Create a private access config
    pub fn private() -> Self {
        Self {
            access_type: AccessType::Private,
            api_key_required: true,
            ..Default::default()
        }
    }

    /// Add allowed domains
    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = domains;
        self
    }

    /// Add rate limits
    pub fn with_rate_limits(mut self, limits: RateLimitConfig) -> Self {
        self.rate_limits = Some(limits);
        self
    }
}

/// Access type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessType {
    /// No restrictions
    #[default]
    Public,
    /// Requires API key
    Authenticated,
    /// Private (specific users only)
    Private,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Max requests per minute
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,

    /// Max contexts per session
    #[serde(default = "default_contexts_per_session")]
    pub contexts_per_session: u32,

    /// Max sessions per day
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions_per_day: Option<u32>,
}

fn default_requests_per_minute() -> u32 {
    100
}

fn default_contexts_per_session() -> u32 {
    50
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            contexts_per_session: default_contexts_per_session(),
            sessions_per_day: None,
        }
    }
}

/// Delivery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryConfig {
    /// Primary delivery mode
    #[serde(default)]
    pub mode: DeliveryMode,

    /// API endpoints (if mode is API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<DeliveryEndpoints>,

    /// Fallback configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<FallbackConfig>,

    /// Caching settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caching: Option<CachingConfig>,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            mode: DeliveryMode::Embedded,
            endpoints: None,
            fallback: None,
            caching: None,
        }
    }
}

impl DeliveryConfig {
    /// Create an embedded delivery config
    pub fn embedded() -> Self {
        Self {
            mode: DeliveryMode::Embedded,
            ..Default::default()
        }
    }

    /// Create an API delivery config
    pub fn api(context_url: impl Into<String>) -> Self {
        Self {
            mode: DeliveryMode::Api,
            endpoints: Some(DeliveryEndpoints {
                context: Some(context_url.into()),
                policy: None,
            }),
            ..Default::default()
        }
    }

    /// Set fallback configuration
    pub fn with_fallback(mut self, fallback: FallbackConfig) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Set caching configuration
    pub fn with_caching(mut self, caching: CachingConfig) -> Self {
        self.caching = Some(caching);
        self
    }
}

/// Delivery mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// Content embedded in atlas
    #[default]
    Embedded,
    /// Fetch from API endpoints
    Api,
    /// Hybrid - some embedded, some from API
    Hybrid,
}

/// API endpoints for delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryEndpoints {
    /// Context fetch endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    /// Policy check endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
}

/// Fallback configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Fallback mode
    pub mode: DeliveryMode,

    /// Contexts to include in fallback
    #[serde(default)]
    pub contexts: Vec<String>,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    /// Enable caching
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Context TTL in seconds
    #[serde(default = "default_context_ttl")]
    pub context_ttl_seconds: u64,

    /// Policy TTL in seconds
    #[serde(default = "default_policy_ttl")]
    pub policy_ttl_seconds: u64,
}

fn default_true() -> bool {
    true
}

fn default_context_ttl() -> u64 {
    300
}

fn default_policy_ttl() -> u64 {
    60
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            context_ttl_seconds: default_context_ttl(),
            policy_ttl_seconds: default_policy_ttl(),
        }
    }
}

/// Notification configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable notifications
    #[serde(default)]
    pub enabled: bool,

    /// Notification channels
    #[serde(default)]
    pub channels: NotificationChannels,

    /// Events that trigger notifications
    #[serde(default)]
    pub triggers: Vec<NotificationTrigger>,
}

impl NotificationConfig {
    /// Create notification config with a webhook
    pub fn with_webhook(url: impl Into<String>, triggers: Vec<NotificationTrigger>) -> Self {
        Self {
            enabled: true,
            channels: NotificationChannels {
                webhook: Some(url.into()),
                ..Default::default()
            },
            triggers,
        }
    }
}

/// Notification channels
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationChannels {
    /// Slack webhook URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack: Option<String>,

    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Generic webhook URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<String>,
}

/// Notification triggers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTrigger {
    /// Policy was overridden
    PolicyOverride,
    /// High-risk action attempted
    HighRiskAction,
    /// Error rate exceeded threshold
    ErrorRateSpike,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Session started
    SessionStarted,
    /// Session ended
    SessionEnded,
    /// Custom trigger
    Custom(String),
}

/// Analytics configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable analytics collection
    #[serde(default)]
    pub enabled: bool,

    /// What to collect
    #[serde(default)]
    pub collect: Vec<AnalyticsType>,

    /// Export configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<AnalyticsExport>,
}

/// Types of analytics to collect
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsType {
    /// Context usage statistics
    ContextUsage,
    /// Policy hit/miss rates
    PolicyHits,
    /// Action frequency
    ActionFrequency,
    /// Session duration
    SessionDuration,
    /// Error rates
    ErrorRates,
}

/// Analytics export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsExport {
    /// Export format
    pub format: ExportFormat,

    /// Export frequency
    pub frequency: ExportFrequency,

    /// Destination URL or path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
}

/// Export format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Csv,
    Parquet,
}

/// Export frequency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFrequency {
    Hourly,
    Daily,
    Weekly,
}

/// Branding configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Logo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,

    /// Accent color (hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,

    /// Support URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,
}

/// Marketplace configuration for published atlases
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    /// Whether atlas is published
    #[serde(default)]
    pub published: bool,

    /// Visibility
    #[serde(default)]
    pub visibility: Visibility,

    /// Listing information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listing: Option<MarketplaceListing>,

    /// Pricing configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<PricingConfig>,
}

/// Visibility setting
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Visible to everyone
    #[default]
    Public,
    /// Only visible with link
    Unlisted,
    /// Only visible to authorized users
    Private,
}

/// Marketplace listing information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketplaceListing {
    /// Listing title
    pub title: String,

    /// Short description (for cards)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,

    /// Full description (markdown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,

    /// Screenshot URLs
    #[serde(default)]
    pub screenshots: Vec<String>,

    /// Demo video URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_video: Option<String>,

    /// Category tags
    #[serde(default)]
    pub categories: Vec<String>,
}

/// Pricing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Pricing model
    pub model: PricingModel,

    /// Free tier limits (if freemium)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_tier: Option<FreeTierConfig>,

    /// Paid tiers
    #[serde(default)]
    pub paid_tiers: Vec<PaidTier>,
}

/// Pricing model
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingModel {
    /// Completely free
    Free,
    /// Free tier with paid upgrades
    Freemium,
    /// Paid only
    Paid,
    /// Pay per use
    PayPerUse,
}

/// Free tier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeTierConfig {
    /// Max contexts per month
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts_per_month: Option<u32>,

    /// Included features
    #[serde(default)]
    pub features: Vec<String>,
}

/// Paid tier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaidTier {
    /// Tier name
    pub name: String,

    /// Monthly price (in cents)
    pub price_monthly: u32,

    /// Max contexts per month
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts_per_month: Option<u32>,

    /// Included features
    #[serde(default)]
    pub features: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steward_config_builder() {
        let config = StewardConfig::new("steward-vib3")
            .with_name("VIB3 Team")
            .with_contact("atlas@vib3.io")
            .with_access(AccessConfig::authenticated());

        assert_eq!(config.id, "steward-vib3");
        assert_eq!(config.name, "VIB3 Team");
        assert_eq!(config.access.access_type, AccessType::Authenticated);
    }

    #[test]
    fn test_access_config() {
        let public = AccessConfig::public();
        assert_eq!(public.access_type, AccessType::Public);
        assert!(!public.api_key_required);

        let authed = AccessConfig::authenticated()
            .with_allowed_domains(vec!["*.vib3.io".to_string()])
            .with_rate_limits(RateLimitConfig::default());

        assert_eq!(authed.access_type, AccessType::Authenticated);
        assert!(authed.api_key_required);
        assert_eq!(authed.allowed_domains, vec!["*.vib3.io"]);
        assert!(authed.rate_limits.is_some());
    }

    #[test]
    fn test_delivery_config() {
        let embedded = DeliveryConfig::embedded();
        assert_eq!(embedded.mode, DeliveryMode::Embedded);

        let api = DeliveryConfig::api("https://api.vib3.io/context")
            .with_caching(CachingConfig::default())
            .with_fallback(FallbackConfig {
                mode: DeliveryMode::Embedded,
                contexts: vec!["essential".to_string()],
            });

        assert_eq!(api.mode, DeliveryMode::Api);
        assert!(api.endpoints.is_some());
        assert!(api.caching.is_some());
        assert!(api.fallback.is_some());
    }

    #[test]
    fn test_notification_config() {
        let config = NotificationConfig::with_webhook(
            "https://hooks.slack.com/test",
            vec![
                NotificationTrigger::PolicyOverride,
                NotificationTrigger::HighRiskAction,
            ],
        );

        assert!(config.enabled);
        assert!(config.channels.webhook.is_some());
        assert_eq!(config.triggers.len(), 2);
    }

    #[test]
    fn test_serialization() {
        let config = StewardConfig::new("test")
            .with_name("Test Steward")
            .with_access(AccessConfig::authenticated());

        let json = serde_json::to_string(&config).unwrap();
        let parsed: StewardConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.name, "Test Steward");
    }

    #[test]
    fn test_marketplace_config() {
        let config = MarketplaceConfig {
            published: true,
            visibility: Visibility::Public,
            listing: Some(MarketplaceListing {
                title: "Test Atlas".to_string(),
                short_description: Some("A test atlas".to_string()),
                long_description: None,
                screenshots: vec![],
                demo_video: None,
                categories: vec!["test".to_string()],
            }),
            pricing: Some(PricingConfig {
                model: PricingModel::Freemium,
                free_tier: Some(FreeTierConfig {
                    contexts_per_month: Some(1000),
                    features: vec!["basic".to_string()],
                }),
                paid_tiers: vec![PaidTier {
                    name: "pro".to_string(),
                    price_monthly: 4900,
                    contexts_per_month: Some(50000),
                    features: vec!["all".to_string()],
                }],
            }),
        };

        assert!(config.published);
        assert_eq!(config.visibility, Visibility::Public);
    }
}
