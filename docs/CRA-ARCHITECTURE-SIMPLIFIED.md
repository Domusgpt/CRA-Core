# CRA Architecture - Simplified

## The Three Roles

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                  │
│  ATLAS STEWARD                                                   │
│  (Creates & controls the atlas)                                  │
│  ├── Defines content (context blocks, policies)                 │
│  ├── Sets access rules (who, how, when)                         │
│  ├── Configures delivery mode (online/cached/gated)             │
│  ├── Receives usage data & feedback                             │
│  └── Controls licensing/pricing                                  │
│                           │                                      │
│                           ▼                                      │
│  CUSTODIAN                                                       │
│  (Human overseeing agents, subscribes to atlases)               │
│  ├── Subscribes to atlases they need                            │
│  ├── Configures which agents get which atlases                  │
│  ├── Monitors their agents' activity                            │
│  └── Can add local restrictions on top                          │
│                           │                                      │
│                           ▼                                      │
│  AGENT                                                           │
│  (LLM doing work)                                                │
│  ├── Gets context from atlases custodian subscribed to          │
│  ├── Reports actions (TRACE)                                     │
│  ├── Gives feedback                                              │
│  └── Operates within governance                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## What An Atlas Actually Is

An atlas is **proprietary context and rules** for a specific domain:

| Atlas Type | Example | Value |
|------------|---------|-------|
| **Internal tools** | Company's custom API atlas | Agents can use internal systems correctly |
| **Specialized domain** | Medical coding atlas | Agents handle ICD-10 codes properly |
| **Product knowledge** | VIB3+ development atlas | Agents build with VIB3 correctly |
| **Compliance** | HIPAA atlas | Agents follow regulations |
| **Best practices** | Security review atlas | Agents catch common vulnerabilities |

**Atlases have value.** They could be:
- Free (open source knowledge)
- Subscription (access while paying)
- Licensed (one-time purchase)
- Internal only (company proprietary)

---

## Atlas Steward Controls

The steward decides everything about their atlas:

### 1. Access Control

```json
{
  "access": {
    "public": false,
    "requires_subscription": true,
    "allowed_custodians": ["org-id-1", "org-id-2"],
    "allowed_agent_types": ["claude-*", "gpt-4-*"],
    "denied_agent_types": ["*-mini", "*-lite"]
  }
}
```

### 2. Delivery Mode

```json
{
  "delivery": {
    "mode": "hybrid",
    "cacheable": true,
    "cache_ttl": "24h",
    "online_required_for": ["high_risk_actions", "production_env"],
    "offline_allowed_for": ["development", "read_only"]
  }
}
```

Options:
- **Online only**: Every request hits CRA (sensitive, changing content)
- **Cacheable**: Agent/custodian can cache locally (stable content)
- **Hybrid**: Some parts cached, some online-only
- **Gated**: Each context block requires individual auth
- **Open**: Once subscribed, full access

### 3. Alert Configuration

```json
{
  "alerts": {
    "notify_steward_on": [
      "first_use_by_new_custodian",
      "negative_feedback",
      "policy_violation_attempt",
      "high_volume_usage"
    ],
    "notify_custodian_on": [
      "agent_policy_violation",
      "suspicious_pattern",
      "session_anomaly"
    ],
    "webhooks": {
      "steward": "https://steward.example.com/cra-events",
      "custodian": "configurable_per_custodian"
    }
  }
}
```

### 4. Gating Specific Content

```json
{
  "context_blocks": [
    {
      "context_id": "basic-overview",
      "gated": false,
      "tier": "free"
    },
    {
      "context_id": "advanced-api",
      "gated": true,
      "tier": "premium",
      "requires_online": true
    },
    {
      "context_id": "production-secrets",
      "gated": true,
      "tier": "enterprise",
      "requires_online": true,
      "audit_every_access": true
    }
  ]
}
```

---

## Custodian Controls

Custodians subscribe to atlases and configure their agents:

### 1. Atlas Subscriptions

```json
{
  "custodian_id": "acme-corp",
  "subscriptions": [
    {
      "atlas_id": "vib3-development",
      "tier": "premium",
      "expires": "2025-12-31"
    },
    {
      "atlas_id": "security-review",
      "tier": "enterprise",
      "expires": null
    }
  ]
}
```

### 2. Agent Permissions

```json
{
  "agents": [
    {
      "agent_id": "dev-claude",
      "atlases_allowed": ["vib3-development"],
      "environment": "development",
      "supervision": "human_supervised"
    },
    {
      "agent_id": "prod-agent",
      "atlases_allowed": ["vib3-development", "security-review"],
      "environment": "production",
      "supervision": "automated",
      "extra_restrictions": ["no_destructive_actions"]
    }
  ]
}
```

### 3. Local Overrides

Custodian can add restrictions on top of steward's rules:

```json
{
  "local_policies": [
    {
      "policy_id": "no-external-apis",
      "applies_to": ["all_agents"],
      "denies": ["action:external_api_call"]
    },
    {
      "policy_id": "staging-only",
      "applies_to": ["agent:junior-*"],
      "restricts_to": ["environment:staging"]
    }
  ]
}
```

---

## Agent Experience

From the agent's perspective, it's simple:

1. **Bootstrap**: "I'm claude-3-sonnet, working on VIB3 website"
2. **Get context**: CRA returns what's available based on custodian's subscriptions
3. **Report actions**: Tell CRA what you're doing
4. **Get feedback**: If something's wrong, CRA tells you

The agent doesn't need to know about stewards, subscriptions, tiers. It just gets what it's allowed to get.

---

## Alert/Event System

TRACE events can trigger actions:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Event Flow                                    │
│                                                                  │
│  Agent reports action                                            │
│         │                                                        │
│         ▼                                                        │
│  ┌─────────────┐                                                │
│  │   TRACE     │──────┬──────────────────────────┐              │
│  │   Event     │      │                          │              │
│  └─────────────┘      │                          │              │
│         │             │                          │              │
│         ▼             ▼                          ▼              │
│  ┌───────────┐  ┌───────────┐           ┌───────────┐          │
│  │  Record   │  │  Evaluate │           │  Trigger  │          │
│  │  to chain │  │  policies │           │  alerts   │          │
│  └───────────┘  └─────┬─────┘           └─────┬─────┘          │
│                       │                       │                 │
│                       ▼                       ▼                 │
│                 ┌───────────┐          ┌───────────┐           │
│                 │ Allow or  │          │  Webhook  │           │
│                 │  deny     │          │  to roles │           │
│                 └───────────┘          └───────────┘           │
│                                              │                  │
│                       ┌──────────────────────┼──────────┐      │
│                       ▼                      ▼          ▼      │
│                 ┌──────────┐          ┌──────────┐ ┌─────────┐ │
│                 │ Steward  │          │Custodian │ │  Agent  │ │
│                 │ notified │          │ notified │ │ alerted │ │
│                 └──────────┘          └──────────┘ └─────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Event Types That Can Alert

| Event | Steward Gets | Custodian Gets | Agent Gets |
|-------|--------------|----------------|------------|
| New custodian uses atlas | Yes | - | - |
| Negative feedback | Yes | - | - |
| High usage | Yes | Yes | - |
| Policy violation attempt | Optional | Yes | Warning |
| Suspicious pattern | Optional | Yes | May terminate |
| Context cache expired | Optional | Optional | Refresh prompt |
| Session anomaly | Optional | Yes | - |

---

## Online/Offline/Cached Modes

Steward configures, custodian can further restrict:

### Online Only
```
Every request → CRA server → Fresh response
```
- Most secure, most controlled
- Steward sees all usage in real-time
- No stale data
- Requires connectivity

### Cacheable
```
First request → CRA server → Cache locally
Subsequent → Local cache (until TTL)
```
- Works offline after initial fetch
- Reduces latency
- Steward sets TTL (1 hour, 24 hours, 7 days, etc.)
- Good for stable content

### Hybrid
```
Some blocks → Cached (stable reference content)
Some blocks → Online only (sensitive or changing)
```
- Best of both worlds
- Steward marks each block's mode
- Production actions might require online verification

### Gated Per Block
```
Basic blocks → Free, cacheable
Premium blocks → Subscription, cacheable
Enterprise blocks → Online only, audit logged
```
- Tiered access within single atlas
- Upsell path for stewards

---

## Marketplace Concept (Future)

Atlases as licensable products:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Atlas Marketplace                             │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ VIB3+ Dev       │  │ Medical Coding  │  │ AWS Best        │ │
│  │ Atlas           │  │ Atlas           │  │ Practices       │ │
│  │                 │  │                 │  │                 │ │
│  │ By: VIB3 Team   │  │ By: HealthTech  │  │ By: CloudExpert │ │
│  │ $29/mo          │  │ $199/mo         │  │ Free            │ │
│  │ ★★★★☆ (47)      │  │ ★★★★★ (203)     │  │ ★★★★☆ (892)     │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│                                                                  │
│  Categories: Development | Healthcare | DevOps | Legal | ...    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

Stewards can:
- Publish atlases to marketplace
- Set pricing (free, subscription, one-time)
- Track usage and revenue
- Get feedback and improve

Custodians can:
- Browse atlases by category
- Read reviews from other custodians
- Subscribe with one click
- Assign to their agents

---

## What This Simplifies

### Before (Over-Complicated)
- Complex bootstrap handshake
- Agent builds its own wrapper
- Multiple auth modes
- Lots of moving parts

### After (Simpler)
- Steward controls their atlas (they're the IP owner)
- Custodian subscribes and assigns to agents
- Agent just uses what it's given
- Events flow up (agent → custodian → steward)
- Steward configures online/offline/cached

### The Core Loop
```
1. Steward creates atlas with content + rules + delivery config
2. Custodian subscribes to atlas
3. Agent connects, gets context based on subscription
4. Agent works, reports to TRACE
5. Events notify steward and custodian as configured
6. Feedback improves atlas
```

---

## Questions To Decide

1. **Where does CRA run?**
   - Hosted service (stewards upload atlases)?
   - Self-hosted (custodians run their own)?
   - Both (hybrid)?

2. **How do stewards get paid?**
   - Through CRA marketplace?
   - Direct billing?
   - Usage-based vs subscription?

3. **What's the minimum viable version?**
   - Probably: Local CRA + file-based atlases + no marketplace
   - Then: Add online mode + subscriptions
   - Then: Add marketplace

4. **What events are most important to alert on?**
   - Policy violations (security)
   - Negative feedback (quality)
   - Usage patterns (value/abuse)

---

## Next Steps

1. **Define atlas manifest schema** - What a steward needs to specify
2. **Define subscription schema** - How custodians subscribe
3. **Define event types** - What triggers alerts
4. **Build simple local version** - File-based, no auth
5. **Add online mode** - Server-based with auth
6. **Add marketplace** - If there's demand
