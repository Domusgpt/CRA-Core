# CRA Atlas Schema v2

## Overview

An Atlas is a versioned package of context, policies, and configuration that defines how agents should work within a domain. This document specifies the complete Atlas schema with steward configuration and extensibility support.

---

## Schema Version

```json
{
  "schema_version": "2.0.0"
}
```

---

## Complete Atlas Structure

```json
{
  "schema_version": "2.0.0",

  "atlas": {
    "id": "string",
    "name": "string",
    "description": "string",
    "version": "string",
    "created_at": "ISO 8601",
    "updated_at": "ISO 8601"
  },

  "steward": { ... },
  "contexts": [ ... ],
  "policies": [ ... ],
  "actions": [ ... ],
  "checkpoints": { ... },
  "trace": { ... },
  "integrations": { ... },
  "marketplace": { ... }
}
```

---

## Atlas Identity

```json
{
  "atlas": {
    "id": "vib3-geometry-v2",
    "name": "VIB3 Geometry System",
    "description": "Context and policies for VIB3 geometry index calculations",
    "version": "2.1.0",
    "created_at": "2024-01-15T00:00:00Z",
    "updated_at": "2024-03-20T14:30:00Z",

    "tags": ["geometry", "index", "financial", "vib3"],
    "category": "domain-specific",
    "maturity": "stable",

    "compatibility": {
      "min_cra_version": "1.0.0",
      "platforms": ["claude-code", "openai-agents", "langchain"],
      "required_plugins": []
    },

    "license": {
      "type": "proprietary",
      "terms_url": "https://vib3.io/atlas-terms"
    }
  }
}
```

---

## Steward Configuration

The steward is the creator/owner of the atlas:

```json
{
  "steward": {
    "id": "steward-vib3",
    "name": "VIB3 Team",
    "contact": "atlas@vib3.io",

    "access": {
      "type": "authenticated",
      "api_key_required": true,
      "allowed_domains": ["*.vib3.io"],
      "rate_limits": {
        "requests_per_minute": 100,
        "contexts_per_session": 50
      }
    },

    "delivery": {
      "mode": "api",
      "endpoints": {
        "context": "https://atlas.vib3.io/v2/context",
        "policy": "https://atlas.vib3.io/v2/policy"
      },
      "fallback": {
        "mode": "embedded",
        "contexts": ["essential-facts", "governance-rules"]
      }
    },

    "notifications": {
      "enabled": true,
      "channels": {
        "slack": "https://hooks.slack.com/...",
        "email": "atlas-alerts@vib3.io"
      },
      "triggers": [
        "policy_override",
        "high_risk_action",
        "error_rate_spike"
      ]
    },

    "analytics": {
      "enabled": true,
      "collect": ["context_usage", "policy_hits", "action_frequency"],
      "export": {
        "format": "json",
        "frequency": "daily",
        "destination": "s3://vib3-atlas-analytics/"
      }
    },

    "branding": {
      "logo_url": "https://vib3.io/atlas-logo.png",
      "accent_color": "#FF6B35",
      "support_url": "https://vib3.io/atlas-support"
    }
  }
}
```

---

## Context Blocks

```json
{
  "contexts": [
    {
      "context_id": "geometry-system",
      "name": "Geometry System Overview",
      "description": "Core concepts of VIB3 geometry index",
      "version": "2.0.0",

      "inject": {
        "mode": "on_match",
        "priority": 380,
        "keywords": ["geometry", "index", "formula", "base", "core"],
        "actions": ["calculate_*", "set_geometry"],
        "risk_tiers": []
      },

      "content": "# VIB3 Geometry System\n\nThe geometry index uses...",

      "content_source": {
        "type": "inline"
      },

      "metadata": {
        "author": "VIB3 Team",
        "last_reviewed": "2024-03-15",
        "confidence": "high",
        "related_contexts": ["geometry-formulas", "geometry-examples"]
      }
    },

    {
      "context_id": "geometry-formulas",
      "name": "Geometry Formulas",

      "inject": {
        "mode": "on_demand",
        "priority": 370
      },

      "content_source": {
        "type": "external",
        "url": "https://atlas.vib3.io/contexts/formulas.md",
        "cache_ttl_seconds": 3600,
        "fallback_content": "# Formulas\n\nContact atlas.vib3.io for formula reference."
      }
    },

    {
      "context_id": "high-risk-warning",
      "name": "High Risk Action Warning",

      "inject": {
        "mode": "risk_based",
        "priority": 900,
        "risk_tiers": ["high", "critical"]
      },

      "content": "⚠️ HIGH RISK ACTION\n\nYou are about to perform a high-risk action..."
    }
  ]
}
```

### Content Source Types

| Type | Description | Fields |
|------|-------------|--------|
| `inline` | Content in atlas | `content` field |
| `external` | Fetch from URL | `url`, `cache_ttl_seconds`, `fallback_content` |
| `template` | Render with variables | `template`, `variables` |
| `composite` | Combine multiple contexts | `include_contexts`, `separator` |

### Inject Modes

| Mode | Description |
|------|-------------|
| `always` | Inject at session start |
| `on_match` | Inject when keywords/actions match |
| `on_demand` | Only when explicitly requested |
| `risk_based` | Inject based on risk tier |
| `conditional` | Custom condition |

---

## Policies

```json
{
  "policies": [
    {
      "policy_id": "require-geometry-check",
      "name": "Require Geometry Validation",
      "description": "Index changes must be validated",
      "enabled": true,

      "trigger": {
        "actions": ["update_geometry_index", "set_base_value"],
        "conditions": [
          {
            "field": "params.value",
            "operator": "changed",
            "reference": "previous_value"
          }
        ]
      },

      "rules": [
        {
          "rule_id": "validate-range",
          "condition": {
            "field": "params.value",
            "operator": "between",
            "values": [0, 1000]
          },
          "action": "allow"
        },
        {
          "rule_id": "block-extreme",
          "condition": {
            "field": "params.value",
            "operator": "greater_than",
            "value": 1000
          },
          "action": "deny",
          "message": "Value exceeds maximum allowed (1000)"
        }
      ],

      "default_action": "allow",

      "override": {
        "allowed": true,
        "require_approval": true,
        "approvers": ["admin", "geometry-lead"],
        "log_override": true
      },

      "metadata": {
        "severity": "medium",
        "category": "data-integrity",
        "compliance": ["internal-policy-7.2"]
      }
    },

    {
      "policy_id": "block-production-delete",
      "name": "Block Production Deletions",
      "enabled": true,

      "trigger": {
        "actions": ["delete_*"],
        "conditions": [
          {
            "field": "params.environment",
            "operator": "equals",
            "value": "production"
          }
        ]
      },

      "rules": [
        {
          "rule_id": "block-all",
          "condition": { "type": "always" },
          "action": "deny",
          "message": "Production deletions are not allowed via agent"
        }
      ],

      "override": {
        "allowed": false
      },

      "metadata": {
        "severity": "critical",
        "category": "safety"
      }
    }
  ]
}
```

### Policy Operators

| Operator | Description |
|----------|-------------|
| `equals` | Exact match |
| `not_equals` | Not equal |
| `contains` | String contains |
| `matches` | Regex match |
| `greater_than` | Numeric comparison |
| `less_than` | Numeric comparison |
| `between` | Range check |
| `in` | In list |
| `not_in` | Not in list |
| `changed` | Value changed from reference |
| `exists` | Field exists |
| `type_is` | Type check |

### Policy Actions

| Action | Behavior |
|--------|----------|
| `allow` | Permit action |
| `deny` | Block action |
| `require_approval` | Block until approved |
| `warn` | Allow but record warning |
| `inject_context` | Inject context before action |

---

## Actions

Define available actions and their risk tiers:

```json
{
  "actions": [
    {
      "action_id": "calculate_geometry_index",
      "name": "Calculate Geometry Index",
      "description": "Compute index from input values",

      "risk_tier": "low",

      "params": {
        "base": {
          "type": "number",
          "required": true,
          "description": "Base value for calculation"
        },
        "factors": {
          "type": "array",
          "items": { "type": "number" },
          "required": false,
          "default": []
        }
      },

      "returns": {
        "type": "object",
        "properties": {
          "index": { "type": "number" },
          "confidence": { "type": "number" }
        }
      },

      "contexts_on_use": ["geometry-formulas"],
      "requires_policy_check": false
    },

    {
      "action_id": "update_geometry_index",
      "name": "Update Geometry Index",
      "description": "Modify the stored index value",

      "risk_tier": "medium",

      "params": {
        "index_id": { "type": "string", "required": true },
        "new_value": { "type": "number", "required": true },
        "reason": { "type": "string", "required": true }
      },

      "contexts_on_use": ["geometry-system", "update-guidelines"],
      "requires_policy_check": true
    },

    {
      "action_id": "delete_geometry_data",
      "name": "Delete Geometry Data",
      "description": "Remove geometry data",

      "risk_tier": "high",

      "params": {
        "data_id": { "type": "string", "required": true },
        "confirm": { "type": "boolean", "required": true }
      },

      "contexts_on_use": ["high-risk-warning", "deletion-policy"],
      "requires_policy_check": true,
      "requires_sync_trace": true
    }
  ]
}
```

### Risk Tiers

| Tier | Description | Default Behavior |
|------|-------------|------------------|
| `low` | Read-only, reversible | Async trace, no policy check |
| `medium` | Writes, recoverable | Async trace, policy check |
| `high` | Destructive, hard to reverse | Sync trace, policy check, verification |
| `critical` | Irreversible, high impact | Sync trace, mandatory approval |

---

## Checkpoints Configuration

```json
{
  "checkpoints": {
    "session_start": {
      "enabled": true,
      "inject_contexts": ["essential-facts", "governance-rules"]
    },

    "session_end": {
      "enabled": true,
      "require_sync_trace": true
    },

    "keyword_match": {
      "enabled": true,
      "case_sensitive": false,
      "match_mode": "any",
      "mappings": {
        "geometry|index|formula": ["geometry-system"],
        "deploy|production": ["deployment-checklist"],
        "delete|remove|destroy": ["high-risk-warning"]
      }
    },

    "action_pre": {
      "enabled": true,
      "mappings": {
        "calculate_*": {
          "inject_contexts": ["geometry-formulas"],
          "require_policy_check": false
        },
        "update_*": {
          "inject_contexts": ["update-guidelines"],
          "require_policy_check": true
        },
        "delete_*": {
          "inject_contexts": ["high-risk-warning"],
          "require_policy_check": true,
          "require_confirmation": true
        }
      }
    },

    "risk_threshold": {
      "enabled": true,
      "min_tier": "high",
      "inject_contexts": ["high-risk-warning"],
      "require_sync_trace": true
    },

    "time_interval": {
      "enabled": false,
      "seconds": 300
    },

    "count_interval": {
      "enabled": false,
      "actions": 50
    },

    "custom": [
      {
        "name": "sentiment_check",
        "enabled": false,
        "evaluator": "plugins/sentiment-checkpoint.js",
        "config": {
          "threshold": -0.5
        }
      }
    ]
  }
}
```

---

## TRACE Configuration

```json
{
  "trace": {
    "mode": "async",

    "queue": {
      "max_size": 100,
      "flush_interval_ms": 5000,
      "batch_size": 10
    },

    "sync_required_for": [
      "session_end",
      "wrapper_constructed",
      "policy_checked",
      "action_blocked",
      "risk_detected"
    ],

    "cache": {
      "enabled": true,
      "context_ttl_seconds": 300,
      "policy_ttl_seconds": 60
    },

    "storage": {
      "backend": "default",
      "retention_days": 90,
      "by_event_type": {
        "session_*": 365,
        "action_blocked": 365,
        "policy_override": 730
      }
    },

    "events": {
      "level": "standard",
      "include": [],
      "exclude": []
    }
  }
}
```

### Event Levels

| Level | Events Recorded |
|-------|-----------------|
| `minimal` | session_*, action_blocked, error_occurred |
| `standard` | + action_*, context_*, policy_* |
| `verbose` | + input_*, output_*, checkpoint_* |
| `complete` | All events |

---

## Integrations

```json
{
  "integrations": {
    "mcp": {
      "enabled": true,
      "server_config": {
        "name": "cra-vib3-atlas",
        "version": "1.0.0"
      },
      "exposed_tools": [
        "cra_request_context",
        "cra_report_action",
        "cra_feedback"
      ]
    },

    "plugins": [
      {
        "name": "cra-plugin-metrics",
        "version": "^1.0.0",
        "config": {
          "prometheus_endpoint": "/metrics"
        }
      },
      {
        "name": "cra-plugin-alerts",
        "version": "^1.0.0",
        "config": {
          "slack_webhook": "$SLACK_WEBHOOK_URL"
        }
      }
    ],

    "webhooks": [
      {
        "name": "policy-violations",
        "url": "https://api.vib3.io/webhooks/cra",
        "events": ["action_blocked", "policy_override"],
        "secret": "$WEBHOOK_SECRET"
      }
    ],

    "external_policy": {
      "enabled": false,
      "url": "https://policy.vib3.io/check",
      "timeout_ms": 2000,
      "fallback": "deny"
    }
  }
}
```

---

## Marketplace Configuration

For atlases published to the CRA marketplace:

```json
{
  "marketplace": {
    "published": true,
    "visibility": "public",
    "listing": {
      "title": "VIB3 Geometry System Atlas",
      "short_description": "Context and governance for VIB3 geometry calculations",
      "long_description": "Full markdown description...",
      "screenshots": [
        "https://atlas.vib3.io/screenshots/1.png"
      ],
      "demo_video": "https://vib3.io/atlas-demo.mp4"
    },

    "pricing": {
      "model": "freemium",
      "free_tier": {
        "contexts_per_month": 1000,
        "features": ["basic-contexts", "standard-policies"]
      },
      "paid_tiers": [
        {
          "name": "pro",
          "price_monthly": 49,
          "contexts_per_month": 50000,
          "features": ["all-contexts", "custom-policies", "priority-support"]
        }
      ]
    },

    "support": {
      "documentation_url": "https://docs.vib3.io/atlas",
      "community_url": "https://community.vib3.io",
      "email": "atlas-support@vib3.io",
      "sla": {
        "response_time_hours": 24,
        "uptime_guarantee": 99.9
      }
    },

    "metrics": {
      "display_downloads": true,
      "display_rating": true,
      "display_active_users": false
    }
  }
}
```

---

## Environment Variables

Atlases can reference environment variables:

```json
{
  "steward": {
    "access": {
      "api_key": "$CRA_ATLAS_API_KEY"
    }
  },
  "integrations": {
    "webhooks": [
      {
        "secret": "$WEBHOOK_SECRET"
      }
    ]
  }
}
```

Syntax: `$VARIABLE_NAME` for required, `${VARIABLE_NAME:-default}` for optional with default.

---

## Validation

Atlas files are validated against JSON Schema:

```bash
cra atlas validate path/to/atlas.json
```

### Validation Rules

1. **Required fields**: `schema_version`, `atlas.id`, `atlas.name`, `atlas.version`
2. **Unique IDs**: All `context_id`, `policy_id`, `action_id` must be unique
3. **Valid references**: Context references in policies must exist
4. **Version format**: Semver for `atlas.version`
5. **Risk tier values**: Must be `low`, `medium`, `high`, or `critical`
6. **Inject modes**: Must be valid mode string

---

## Migration from v1

```bash
cra atlas migrate v1-atlas.json --to v2
```

### Breaking Changes from v1

| v1 | v2 | Migration |
|----|----|-----------|
| `contexts[].trigger` | `contexts[].inject` | Renamed, restructured |
| `governance.policies` | `policies` | Top-level, expanded schema |
| `settings.trace` | `trace` | Top-level, expanded config |
| No steward section | `steward` | New required section |
| No marketplace | `marketplace` | New optional section |

---

## Full Example

See `/atlases/examples/complete-atlas-v2.json` for a complete example.

---

## Summary

Atlas v2 adds:

- **Steward configuration** - Full control for atlas creators
- **Enhanced policies** - Richer conditions and overrides
- **Checkpoint configuration** - Fine-grained injection control
- **TRACE configuration** - Async/sync, caching, retention
- **Integrations** - MCP, plugins, webhooks
- **Marketplace** - Publishing, pricing, support

The schema is designed for extensibility - new sections can be added without breaking existing atlases.
