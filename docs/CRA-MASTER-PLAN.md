# CRA System Design - Master Plan

## What We're Building

CRA (Context Registry for Agents) is a governance and context system for AI agents.

**Core principle:** "If it wasn't emitted by the runtime, it didn't happen."

---

## The Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CRA SYSTEM                                   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  CRA-CORE (Rust)                                             │   │
│  │  - CARP: Context matching, policy evaluation                │   │
│  │  - TRACE: Async event recording, hash chain                 │   │
│  │  - Atlas: Load and validate atlas manifests                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                       │
│                              │                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  MCP SERVER                                                  │   │
│  │  - Guides wrapper construction                              │   │
│  │  - Exposes CRA tools to agents                              │   │
│  │  - Handles auth/verification                                │   │
│  │  - Help system for agents and custodians                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                       │
│                              │                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  WRAPPER (Agent-Built)                                       │   │
│  │  - Intercepts agent I/O                                     │   │
│  │  - Injects context at checkpoints                           │   │
│  │  - Records to TRACE (async)                                 │   │
│  │  - Enforces policies                                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                       │
│                              │                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  ATLASES (JSON)                                              │   │
│  │  - Context blocks with inject modes                         │   │
│  │  - Policies (allow, deny, rate limit, etc.)                 │   │
│  │  - Actions definitions                                       │   │
│  │  - Steward configuration                                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Design Principles

### 1. Most CRAs Are Low Governance

The typical use case:
- Context injection for domain knowledge
- Basic audit trail
- No blocking verification
- Async everything

High governance (real-time verification, blocking policies) is the exception, not the rule.

### 2. TRACE Is Async By Default

TRACE records events but doesn't block:
- Events queued and processed async
- Cached results accepted
- Hash chain built in background
- Only blocks when atlas requires it

### 3. Checkpoints, Not Every Prompt

Context injection happens at checkpoints defined by atlas:
- `inject_mode: always` - session start
- `inject_mode: on_match` - when keywords match
- `inject_mode: on_demand` - when agent asks
- `inject_mode: risk_based` - for risky actions

Not every prompt goes through CRA.

### 4. Agent Builds Its Own Wrapper

The wrapper is:
- Built by the agent during onboarding
- Tailored to agent's environment
- The build process IS the onboarding
- Verified by CRA MCP

### 5. CRA Auto-Detects, Doesn't Interrogate

CRA should know:
- Model identity (from environment)
- Capabilities (from tool list)
- Context window (standard per model)

Only ask essential questions:
- Intent
- Authorization level
- Special constraints

---

## Documents To Create

### Core System

| Document | Purpose | Status |
|----------|---------|--------|
| `CRA-WRAPPER-SYSTEM.md` | How wrapper works, agent builds it | ✅ Done |
| `CRA-TRACE-ASYNC.md` | Async TRACE with caching | 🔄 Needed |
| `CRA-CHECKPOINT-SYSTEM.md` | When/how context injection happens | 🔄 Needed |
| `CRA-WRAPPER-PROTOCOL.md` | Spec for wrapper construction | 🔄 Needed |

### Integration

| Document | Purpose | Status |
|----------|---------|--------|
| `MCP-INTEGRATION-DESIGN.md` | MCP server design | ✅ Done |
| `CRA-BOOTSTRAP-PROTOCOL.md` | Bootstrap handshake | ✅ Done |
| `CRA-AGENT-MD-GENERATION.md` | Generated .md for agents | ✅ Done |

### Configuration

| Document | Purpose | Status |
|----------|---------|--------|
| `CRA-STEWARD-CONFIG.md` | Steward controls on atlas | ✅ Done |
| `CRA-AUTHENTICATION.md` | Auth modes and keys | ✅ Done |
| `CRA-ARCHITECTURE-SIMPLIFIED.md` | Three roles overview | ✅ Done |

### Help & Onboarding

| Document | Purpose | Status |
|----------|---------|--------|
| `CRA-MCP-HELP-SYSTEM.md` | Help for agents and custodians | ✅ Done |
| `CRA-INTERACTIVE-ONBOARDING.md` | Discovery questions | ✅ Done |

---

## TRACE Async Design

### Default: Non-Blocking

```
Agent action
     │
     ▼
Wrapper captures ─────► Queue event ─────► Async processor
     │                                           │
     ▼                                           ▼
Response continues                     TRACE writes to chain
(not blocked)                          (in background)
```

### When Blocking Is Needed

Atlas can require sync for specific cases:

```json
{
  "trace_config": {
    "default_mode": "async",
    "sync_required_for": [
      "policy_check",
      "high_risk_action",
      "production_write"
    ]
  }
}
```

### Caching

TRACE can cache:
- Context blocks (already fetched)
- Policy decisions (same action pattern)
- Session state (don't re-fetch)

Cache invalidation:
- TTL from atlas config
- Explicit invalidation on changes
- Session boundary clears cache

---

## Checkpoint System

### What Triggers Checkpoints

| Trigger | Example | Inject Mode |
|---------|---------|-------------|
| Session start | Bootstrap | `always` |
| Keyword match | "VIB3" in prompt | `on_match` |
| Risk tier | Destructive action | `risk_based` |
| Explicit request | Agent asks | `on_demand` |
| Time interval | Every N minutes | (configurable) |
| Action count | Every N actions | (configurable) |

### What Happens At Checkpoint

1. Wrapper pauses (briefly)
2. Calls CRA with current context
3. CRA returns relevant context blocks
4. Wrapper injects into prompt/context
5. TRACE records checkpoint (async)
6. Wrapper continues

### Atlas Configures Checkpoints

```json
{
  "checkpoint_config": {
    "on_session_start": true,
    "on_keyword_match": true,
    "keywords": ["deploy", "production", "delete"],
    "on_risk_tier": ["high", "critical"],
    "time_interval_minutes": null,
    "action_count_interval": null
  }
}
```

---

## Wrapper Protocol

### Standard Parts (Every Wrapper)

1. **Session management** - Start/end governed session
2. **I/O hooks** - Intercept input/output
3. **CRA client** - Call CRA-Core
4. **TRACE queue** - Buffer events for async send
5. **Identity** - Wrapper construction hash

### Customizable Parts

1. **Checkpoint triggers** - When to call CRA
2. **Injection points** - Where to put context
3. **Policy handling** - What to do on deny
4. **Error handling** - Wrapper failure modes

### Verification

CRA MCP verifies wrapper:
- Matches expected patterns
- Has required components
- Construction hash recorded
- I/O hooks functional

---

## Implementation Order

### Phase 1: Documentation (Current)
- [x] Core system docs
- [ ] TRACE async design
- [ ] Checkpoint system
- [ ] Wrapper protocol spec

### Phase 2: Rust Core Updates
- [ ] TRACE async mode
- [ ] Checkpoint evaluation
- [ ] Cache layer
- [ ] Webhook/event triggers

### Phase 3: MCP Server
- [ ] Wrapper construction guidance
- [ ] Verification tools
- [ ] Help system
- [ ] Auth handling

### Phase 4: Integration Testing
- [ ] Claude Code wrapper
- [ ] OpenAI wrapper
- [ ] End-to-end tests

---

## Key Decisions Made

1. **Agent builds wrapper** - Not pre-built, constructed during onboarding
2. **Building = onboarding** - Learn by building
3. **TRACE async by default** - Most CRAs are low governance
4. **Checkpoints not every prompt** - Atlas defines when
5. **CRA auto-detects** - Minimal questions
6. **Atlases mostly free** - Marketplace is future potential
7. **Trust is case-by-case** - Not assuming worst case

---

## Next Steps

1. Document TRACE async design
2. Document checkpoint system
3. Document wrapper protocol
4. Update Rust code for async TRACE
5. Build MCP server skeleton
