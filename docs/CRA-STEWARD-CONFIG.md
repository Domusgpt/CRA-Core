# Atlas Steward Configuration

## Extending the Existing Manifest

Rather than rebuilding, we add a `steward` block to the existing atlas manifest:

```json
{
  "atlas_version": "1.0",
  "atlas_id": "vib3-development",
  "name": "VIB3+ Development Atlas",
  "version": "6.0.0",

  "steward": {
    "owner": "vib3-team",
    "contact": "steward@vib3.example.com",

    "access": {
      "public": false,
      "requires_auth": true,
      "tiers": ["free", "premium", "enterprise"]
    },

    "delivery": {
      "default_mode": "cached",
      "cache_ttl": "24h",
      "online_required": ["production"]
    },

    "alerts": {
      "on_first_use": true,
      "on_negative_feedback": true,
      "on_violation": true,
      "webhook": "https://vib3.example.com/cra-events"
    },

    "licensing": {
      "free_tier": ["basic-*"],
      "premium_tier": ["advanced-*", "workflow-*"],
      "enterprise_tier": ["*"]
    }
  },

  "context_blocks": [...],
  "policies": [...],
  "actions": [...]
}
```

---

## Steward Block Schema

```rust
/// Steward configuration for an atlas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasSteward {
    /// Who owns/maintains this atlas
    pub owner: String,

    /// Contact for steward
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,

    /// Access control settings
    #[serde(default)]
    pub access: StewardAccess,

    /// Delivery mode settings
    #[serde(default)]
    pub delivery: StewardDelivery,

    /// Alert configuration
    #[serde(default)]
    pub alerts: StewardAlerts,

    /// Licensing tiers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub licensing: Option<StewardLicensing>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StewardAccess {
    /// Is this atlas publicly available?
    #[serde(default)]
    pub public: bool,

    /// Requires authentication to access?
    #[serde(default)]
    pub requires_auth: bool,

    /// Available access tiers
    #[serde(default)]
    pub tiers: Vec<String>,

    /// Allowed agent types (glob patterns)
    #[serde(default)]
    pub allowed_agents: Vec<String>,

    /// Blocked agent types
    #[serde(default)]
    pub blocked_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardDelivery {
    /// Default delivery mode
    #[serde(default = "default_delivery_mode")]
    pub default_mode: DeliveryMode,

    /// Cache TTL for cached mode
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: String,

    /// Contexts that require online verification
    #[serde(default)]
    pub online_required: Vec<String>,

    /// Contexts that can be cached
    #[serde(default)]
    pub cacheable: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// Always fetch from server
    Online,
    /// Cache locally after first fetch
    #[default]
    Cached,
    /// Some blocks cached, some online
    Hybrid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StewardAlerts {
    /// Alert on first use by new custodian
    #[serde(default)]
    pub on_first_use: bool,

    /// Alert on negative feedback
    #[serde(default)]
    pub on_negative_feedback: bool,

    /// Alert on policy violation attempt
    #[serde(default)]
    pub on_violation: bool,

    /// Alert on high usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_high_usage: Option<u64>,

    /// Webhook URL for alerts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StewardLicensing {
    /// Context IDs available in free tier (glob patterns)
    #[serde(default)]
    pub free_tier: Vec<String>,

    /// Context IDs available in premium tier
    #[serde(default)]
    pub premium_tier: Vec<String>,

    /// Context IDs available in enterprise tier
    #[serde(default)]
    pub enterprise_tier: Vec<String>,

    /// Price per tier (optional, for marketplace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<HashMap<String, String>>,
}
```

---

## Per-Context Gating

Individual context blocks can specify their tier:

```json
{
  "context_id": "vib3-basic-overview",
  "name": "VIB3+ Overview",
  "tier": "free",
  "delivery": "cached",
  "content": "..."
}

{
  "context_id": "vib3-advanced-api",
  "name": "Advanced API Reference",
  "tier": "premium",
  "delivery": "cached",
  "content": "..."
}

{
  "context_id": "vib3-production-config",
  "name": "Production Configuration",
  "tier": "enterprise",
  "delivery": "online",
  "audit_access": true,
  "content": "..."
}
```

Add to `AtlasContextBlock`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasContextBlock {
    // ... existing fields ...

    /// Access tier required (free, premium, enterprise, etc.)
    #[serde(default = "default_tier")]
    pub tier: String,

    /// Delivery mode override for this block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<DeliveryMode>,

    /// Log every access to this block
    #[serde(default)]
    pub audit_access: bool,
}
```

---

## TRACE Events for Alerts

Add event types that can trigger steward alerts:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceEventType {
    // ... existing types ...

    /// New custodian started using this atlas
    NewCustodian {
        custodian_id: String,
        tier: String,
    },

    /// Feedback received on context
    ContextFeedback {
        context_id: String,
        helpful: bool,
        reason: Option<String>,
    },

    /// Policy violation attempted
    PolicyViolation {
        policy_id: String,
        action_attempted: String,
        reason: String,
    },

    /// Usage threshold reached
    UsageThreshold {
        threshold: u64,
        current: u64,
        period: String,
    },

    /// Context accessed (for audit_access blocks)
    ContextAccessed {
        context_id: String,
        agent_id: String,
        tier: String,
    },
}
```

---

## Simple Alert System

When a TRACE event is recorded, check if it triggers alerts:

```rust
impl TraceCollector {
    fn maybe_alert(&self, event: &TraceEvent, atlas: &AtlasManifest) {
        if let Some(steward) = &atlas.steward {
            let should_alert = match &event.event_type {
                TraceEventType::NewCustodian { .. } => steward.alerts.on_first_use,
                TraceEventType::ContextFeedback { helpful, .. } => {
                    !helpful && steward.alerts.on_negative_feedback
                }
                TraceEventType::PolicyViolation { .. } => steward.alerts.on_violation,
                TraceEventType::UsageThreshold { .. } => true,
                _ => false,
            };

            if should_alert {
                if let Some(webhook) = &steward.alerts.webhook {
                    self.send_alert(webhook, event);
                }
            }
        }
    }
}
```

---

## Custodian Subscription File

Custodians have a simple config specifying what they've subscribed to:

```json
{
  "custodian_id": "acme-corp",
  "subscriptions": [
    {
      "atlas_id": "vib3-development",
      "tier": "premium",
      "api_key": "cra-vib3-abc123",
      "expires": "2025-12-31"
    },
    {
      "atlas_id": "security-review",
      "tier": "enterprise",
      "api_key": "cra-sec-def456"
    }
  ],
  "agent_permissions": {
    "dev-agent": ["vib3-development"],
    "prod-agent": ["vib3-development", "security-review"]
  }
}
```

---

## Resolution Flow

When an agent requests context:

```
1. Agent asks CRA for context

2. CRA checks:
   - Does custodian have subscription to relevant atlases?
   - Does agent have permission for those atlases?
   - What tier does custodian have?

3. CRA filters context blocks:
   - Only blocks agent's tier can access
   - Check delivery mode (cached OK? or online required?)
   - Check if context is gated for this environment

4. CRA returns appropriate context

5. TRACE records the request

6. If alert conditions met, notify steward
```

---

## What This Adds to Existing Code

### Minimal Changes

1. **AtlasSteward struct** - Add to manifest.rs
2. **tier field** on context blocks - One new field
3. **TraceEventType variants** - A few new event types
4. **Alert check in collector** - Simple match statement
5. **Subscription loader** - Read custodian config file

### What Stays the Same

- CARP resolver logic
- TRACE hash chain
- Context matching
- Policy evaluation
- Everything else

---

## Phased Implementation

### Phase 1: Local Only (No Auth)
- Add `steward` block to manifest schema
- Add `tier` to context blocks
- Steward settings are informational only
- No enforcement yet

### Phase 2: Subscription Enforcement
- Add custodian subscription config
- Filter context by tier
- Enforce agent permissions

### Phase 3: Alerts
- Add alert event types to TRACE
- Implement webhook notifications
- Steward receives alerts

### Phase 4: Online Mode
- Add server component
- Real-time verification for online-required contexts
- Cache management with TTL

### Phase 5: Marketplace (Optional)
- Atlas registry
- Subscription management
- Payment integration

---

## Configuration Examples

### Public Free Atlas (No Auth)
```json
{
  "steward": {
    "owner": "community",
    "access": {
      "public": true,
      "requires_auth": false
    },
    "delivery": {
      "default_mode": "cached",
      "cache_ttl": "7d"
    }
  }
}
```

### Freemium Atlas
```json
{
  "steward": {
    "owner": "vib3-team",
    "access": {
      "public": true,
      "requires_auth": true,
      "tiers": ["free", "premium"]
    },
    "licensing": {
      "free_tier": ["basic-*", "overview-*"],
      "premium_tier": ["*"]
    }
  }
}
```

### Internal/Enterprise Atlas
```json
{
  "steward": {
    "owner": "acme-security",
    "access": {
      "public": false,
      "requires_auth": true
    },
    "delivery": {
      "default_mode": "online",
      "online_required": ["*"]
    },
    "alerts": {
      "on_first_use": true,
      "on_violation": true,
      "webhook": "https://internal.acme.com/cra-alerts"
    }
  }
}
```

---

## Summary

This approach:
1. **Builds on existing code** - Minimal new structs
2. **Steward controls their atlas** - Access, delivery, alerts, licensing
3. **Configurable by steward** - Each atlas can be different
4. **Progressive implementation** - Start simple, add features
5. **Atlases as IP** - Tiers and licensing built in
