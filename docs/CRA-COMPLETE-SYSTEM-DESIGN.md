# CRA - Complete System Design

## What CRA Is

**CRA = Context Registry for Agents**

A system that:
1. Gives agents domain knowledge (from atlases)
2. Records what happened (TRACE audit trail)
3. Enforces policies when needed (CARP)

**Principle:** "If it wasn't emitted by the runtime, it didn't happen."

---

## The Stack

```
ATLASES (JSON files)
   │
   │  Context blocks, policies, actions
   │  Mostly FREE (paid = future potential)
   │
   ▼
CRA-CORE (Rust library)
   │
   │  CARP: Context matching, policy evaluation
   │  TRACE: Hash-chained audit trail (async by default)
   │  Atlas loader
   │
   ▼
MCP SERVER (Bridge)
   │
   │  Guides wrapper construction
   │  Exposes CRA tools to agents
   │  Auth/verification
   │  Help system
   │
   ▼
WRAPPER (Agent-built)
   │
   │  Agent builds this during onboarding
   │  Thin collector by default
   │  Injects context at checkpoints
   │  Collects TRACE, uploads when makes sense
   │
   ▼
AGENT (Does work)
   │
   │  Mostly doesn't notice wrapper
   │  Gets helpful context when relevant
   │  Has audit trail without thinking about it
```

---

## The Three Roles

**STEWARD** (creates atlas)
- Writes context, policies, actions
- Configures access (public/auth/tiered)
- Configures context injection rules and checkpoints
- Integrates other tools or external systems into the atlas
- Gets alerts (optional)
- Owns the IP

**CUSTODIAN** (uses atlas)
- Subscribes to atlases
- Assigns to their agents
- Monitors activity
- Can add local restrictions

**AGENT** (does work)
- Builds wrapper during onboarding
- Gets context automatically
- Actions recorded to TRACE
- Doesn't think about governance

---

## The Wrapper

### Why Agent Builds It

| Reason | Explanation |
|--------|-------------|
| Building = Learning | Understands by constructing |
| Building = Consent | Active agreement, can't claim ignorance |
| Building = Auth | Construction hash = identity |
| Building = Tailored | Custom for this agent/environment |

### What Wrapper Does (Minimal Default)

1. **Collect TRACE data** - Queue events locally
2. **Upload when makes sense** - Batch, intervals, session end
3. **Inject context at checkpoints** - Not every prompt
4. **That's it**

### What Wrapper Can Do (When Atlas Requires)

- Sync verification for specific actions
- Policy blocking
- Real-time checks
- Extra integration hooks

**Most wrappers are thin collectors. Heavy governance is opt-in.**

---

## TRACE

### Default: Async, Non-Blocking

```
Agent does something
        │
        ▼
Wrapper captures event
        │
        ▼
Event → LOCAL QUEUE (instant, non-blocking)
        │
        ▼
Agent continues (not waiting)

        ... background ...

Queue uploads when:
  - Size threshold reached
  - Time interval passes
  - Session ends
  - Atlas requires sync for this event
```

### Why Async Default

Most CRAs are low governance:
- Just want context injection
- Just want basic audit trail
- Don't need real-time verification

Sync only when atlas explicitly requires it.

### Caching

Wrapper caches:
- Context blocks (don't re-fetch)
- Policy decisions (same action = same answer)
- Session state

TTL configured by atlas. Repeat requests are instant.

---

## Checkpoints

### When Context Injection Happens

| Inject Mode | Trigger |
|-------------|---------|
| `always` | Session start |
| `on_match` | Keywords in prompt |
| `on_demand` | Agent asks |
| `risk_based` | High-risk action |

**NOT every prompt.** Atlas defines when CRA intervenes.

### Example Atlas Config

```json
{
  "context_blocks": [
    {
      "context_id": "essentials",
      "inject_mode": "always"
    },
    {
      "context_id": "geometry-help",
      "inject_mode": "on_match",
      "keywords": ["geometry", "index"]
    },
    {
      "context_id": "troubleshooting",
      "inject_mode": "on_demand"
    }
  ]
}
```

---

## CRA Auto-Detects

CRA knows without asking:
- Model identity (from environment)
- Capabilities (from tool list)
- Context window (standard per model)
- Environment (Claude Code, API, etc.)

**Only asks essential questions:**
- What's your intent?
- Authorization level? (dev/staging/prod)
- Special constraints?

No interrogation. Detect and confirm.

---

## Bootstrap Flow

```
1. Agent: "I need CRA"

2. CRA MCP: Auto-detects model, environment, capabilities

3. CRA MCP: "Intent? Auth level?" (minimal questions)

4. Agent: Answers

5. CRA MCP: Guides wrapper construction
   - Provides template
   - Agent builds/implements
   - CRA explains as agent builds
   - Terms agreed by building

6. CRA MCP: Verifies wrapper correct

7. Wrapper active:
   - I/O accessible to CRA
   - Context injection at checkpoints
   - TRACE collection (async)
   - Agent is governed
```

**Building the wrapper IS the onboarding.**

---

## What Gets Generated

### CRA Integration MD

Custom markdown for this agent:

```markdown
# CRA Integration for claude-3-opus @ acme-corp

Session: sess-abc123
Model: claude-3-opus (detected)
Environment: Claude Code
Authorization: staging

## Your Wrapper
- Collects TRACE (async)
- Injects context at checkpoints
- Uploads when: [batch size, interval, session end]

## Governance
- Actions recorded to TRACE
- Context injected on keyword match
- Policies: [list from atlas]
```

### Wrapper Code

Agent builds actual code/hooks:
- For Claude Code: hooks in `.claude/`
- For API agents: middleware
- For custom: whatever fits

---

## Steward Configuration

Stewards configure the atlas, its context injections and rules, and integrate other tools or external systems:

```json
{
  "steward": {
    "owner": "vib3-team",
    "access": {
      "public": true,
      "requires_auth": false
    },
    "delivery": {
      "default_mode": "cached",
      "cache_ttl": "24h"
    },
    "alerts": {
      "on_first_use": true,
      "on_negative_feedback": true
    },
    "integrations": {
      "external_tools": ["api.vib3.com/validate"],
      "webhooks": ["https://vib3.example.com/cra-events"]
    }
  },
  "trace_config": {
    "mode": "async",
    "sync_required_for": ["policy_check"]
  }
}
```

---

## Why Agents Won't Avoid This

1. **Low overhead** - Async, cached, checkpoints only
2. **Actually helpful** - Gets good context when relevant
3. **Invisible** - Wrapper handles everything
4. **No interrogation** - Auto-detects
5. **Doesn't block** - Syncs only when required

Wrapper is so thin agents don't notice it.

---

## Summary Table

| Aspect | Design |
|--------|--------|
| Wrapper | Agent builds it (thin collector default) |
| TRACE | Async, queue locally, upload when makes sense |
| Context | Checkpoints only, not every prompt |
| Cache | Yes, repeat requests instant |
| Detection | Auto-detect model/env, minimal questions |
| Overhead | Low - agents won't avoid |
| Sync | Only when atlas requires |
| Building | IS the onboarding, consent, auth |
| Atlases | Mostly free, paid is future potential |
| Stewards | Configure atlas, rules, and external integrations |
