# CRA Interactive Onboarding Protocol

## The Core Insight

Bootstrap isn't just CRA talking AT the agent. It's a **conversation** where:
1. CRA asks questions
2. Agent answers honestly
3. CRA configures based on answers
4. TRACE is taught and protected
5. Domain owner gets full visibility

```
┌─────────────────────────────────────────────────────────────────┐
│                   Interactive Onboarding                         │
│                                                                  │
│  CRA                                    Agent                    │
│   │                                       │                      │
│   │◀── "I want to help with X" ───────────│  Intent declared     │
│   │                                       │                      │
│   │──── "What tools do you have?" ───────▶│  CRA asks            │
│   │◀── "read, write, execute, search" ────│  Agent answers       │
│   │                                       │                      │
│   │──── "What's your context limit?" ────▶│  CRA asks            │
│   │◀── "200k tokens" ─────────────────────│  Agent answers       │
│   │                                       │                      │
│   │──── "Are you authorized for prod?" ──▶│  CRA asks            │
│   │◀── "No, staging only" ────────────────│  Agent answers       │
│   │                                       │                      │
│   │  [CRA configures based on answers]    │                      │
│   │                                       │                      │
│   │──── GOVERNANCE (tailored) ───────────▶│  Rules for this      │
│   │                                       │  agent's permissions │
│   │──── TRACE TUTORIAL ──────────────────▶│  How TRACE works     │
│   │──── TRACE PROTECTION ────────────────▶│  How to NOT break it │
│   │──── CONTEXT (right-sized) ───────────▶│  Only what's needed  │
│   │                                       │                      │
│   │◀── READY ─────────────────────────────│  Agent understands   │
│   │                                       │                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Identity & Capability Discovery

### CRA Asks, Agent Answers

```json
{
  "type": "DISCOVERY",
  "questions": [
    {
      "id": "model_identity",
      "question": "What model are you?",
      "why": "Different models have different capabilities and trustworthiness",
      "options": ["claude-3", "gpt-4", "llama", "other"],
      "required": true
    },
    {
      "id": "tool_capabilities",
      "question": "What tools/actions can you perform?",
      "why": "We only govern actions you can actually take",
      "type": "array",
      "examples": ["read_file", "write_file", "execute_code", "web_search", "api_call"]
    },
    {
      "id": "context_window",
      "question": "What is your context window size?",
      "why": "We'll adjust how much context we provide",
      "type": "number",
      "unit": "tokens"
    },
    {
      "id": "authorization_level",
      "question": "What environments are you authorized for?",
      "why": "Policies differ by environment",
      "options": ["development", "staging", "production", "all"],
      "required": true
    },
    {
      "id": "user_context",
      "question": "Who is supervising you? (human, automated, unsupervised)",
      "why": "Affects risk tolerance and approval requirements"
    },
    {
      "id": "task_scope",
      "question": "Is this task bounded or open-ended?",
      "why": "Bounded tasks get tighter governance"
    }
  ]
}
```

### Agent Response

```json
{
  "type": "DISCOVERY_RESPONSE",
  "answers": {
    "model_identity": "claude-3-sonnet",
    "tool_capabilities": ["read_file", "write_file", "execute_code", "web_search"],
    "context_window": 200000,
    "authorization_level": "staging",
    "user_context": "human_supervised",
    "task_scope": "bounded"
  }
}
```

### What CRA Does With Answers

```
IF model_identity is trusted AND authorization_level is "production"
  → Enable production policies
  → Allow destructive actions with confirmation

IF context_window < 50000
  → Send only high-priority context blocks
  → Offer incremental context on request

IF user_context is "unsupervised"
  → Stricter action approval
  → More frequent checkpoints

IF task_scope is "bounded"
  → Tighter governance
  → Explicit boundaries
```

---

## Phase 2: Tailored Governance

Based on discovery, CRA sends governance **specific to this agent**:

### For a Staging-Only Agent

```json
{
  "type": "GOVERNANCE",
  "tailored_for": {
    "model": "claude-3-sonnet",
    "authorization": "staging",
    "supervised": true
  },
  "rules": [
    {
      "rule_id": "env.staging_only",
      "description": "You may only modify resources in staging environments",
      "enforcement": "hard",
      "check": "all paths must contain 'staging' or not contain 'prod'"
    },
    {
      "rule_id": "trace.required",
      "description": "All file operations must be reported",
      "enforcement": "hard"
    }
  ],
  "excluded_rules": [
    {
      "rule_id": "prod.approval_required",
      "reason": "Not applicable - agent is staging-only"
    }
  ]
}
```

### For an Unsupervised Production Agent

```json
{
  "type": "GOVERNANCE",
  "tailored_for": {
    "model": "claude-3-opus",
    "authorization": "production",
    "supervised": false
  },
  "rules": [
    {
      "rule_id": "prod.checkpoint",
      "description": "Must checkpoint before any destructive action",
      "enforcement": "hard"
    },
    {
      "rule_id": "prod.no_force",
      "description": "No --force flags on any command",
      "enforcement": "hard"
    },
    {
      "rule_id": "prod.rollback_plan",
      "description": "Must have rollback plan before changes",
      "enforcement": "soft"
    }
  ],
  "elevated_logging": true,
  "audit_level": "comprehensive"
}
```

---

## Phase 3: TRACE Tutorial

CRA teaches the agent how TRACE works:

```json
{
  "type": "TRACE_TUTORIAL",
  "sections": [
    {
      "topic": "what_trace_is",
      "content": "TRACE is a cryptographic audit log. Every action you report becomes a hash-linked event. This creates a tamper-evident record of your session."
    },
    {
      "topic": "why_trace_matters",
      "content": "TRACE proves:\n- What you knew (context received)\n- What you did (actions reported)\n- When you did it (timestamps)\n- The order of events (hash chain)"
    },
    {
      "topic": "how_to_use_trace",
      "content": "Simply use cra_report_action before significant actions. CRA handles the hashing. Example:\n\ncra_report_action(action: 'write_file', params: {path: '/app/config.js'})\n\nThis creates a TRACE event linked to all previous events."
    },
    {
      "topic": "what_not_to_do",
      "content": "DO NOT:\n- Report false actions (will be detected via outcome mismatch)\n- Skip reporting (breaks governance)\n- Attempt to forge hashes (cryptographically impossible)\n- Claim you didn't receive context (bootstrap is hashed)"
    }
  ]
}
```

---

## Phase 4: TRACE Protection

CRA explains what it watches for:

```json
{
  "type": "TRACE_PROTECTION",
  "monitoring": [
    {
      "detection": "action_outcome_mismatch",
      "description": "If you report 'write_file' but the file doesn't change, we detect it",
      "consequence": "Session flagged for review"
    },
    {
      "detection": "unreported_actions",
      "description": "If external monitoring shows actions not in TRACE, we detect it",
      "consequence": "Governance violation logged"
    },
    {
      "detection": "hash_manipulation",
      "description": "Any attempt to alter previous events breaks the chain",
      "consequence": "Chain invalidated, session terminated"
    },
    {
      "detection": "context_denial",
      "description": "Claiming you didn't receive context when bootstrap proves you did",
      "consequence": "Bad faith flag"
    },
    {
      "detection": "feedback_gaming",
      "description": "Systematic false feedback to manipulate context matching",
      "consequence": "Feedback weight reduced for this agent"
    }
  ],
  "transparency": "We're telling you this so you understand the system, not to threaten you. Honest operation means none of this matters."
}
```

---

## Phase 5: Right-Sized Context

Based on answers, CRA sends **only relevant context**:

### For Small Context Window (50k tokens)

```json
{
  "type": "CONTEXT",
  "strategy": "minimal",
  "reason": "Agent has 50k context - sending only essentials",
  "contexts": [
    {
      "context_id": "vib3-essential-facts",
      "priority": 500,
      "content": "[Condensed 500 words]"
    }
  ],
  "available_on_request": [
    {"context_id": "vib3-geometry-system", "summary": "24 geometry formulas"},
    {"context_id": "vib3-api-reference", "summary": "Full API documentation"}
  ],
  "instruction": "Request additional context by ID if needed. We're conserving your context window."
}
```

### For Large Context Window (200k tokens)

```json
{
  "type": "CONTEXT",
  "strategy": "comprehensive",
  "reason": "Agent has 200k context - sending full relevant context",
  "contexts": [
    {"context_id": "vib3-what-it-is", "priority": 400, "content": "[Full content]"},
    {"context_id": "vib3-geometry-system", "priority": 380, "content": "[Full content]"},
    {"context_id": "vib3-javascript-api", "priority": 360, "content": "[Full content]"},
    {"context_id": "vib3-audio-reactivity", "priority": 340, "content": "[Full content]"}
  ]
}
```

---

## Domain Owner Visibility

Everything is logged for the domain owner:

### Session Log Entry

```json
{
  "session_id": "sess-abc123",
  "domain": "vib3-development",
  "started_at": "2024-01-15T10:30:00Z",
  "agent": {
    "model": "claude-3-sonnet",
    "capabilities": ["read", "write", "execute"],
    "authorization": "staging",
    "supervised": true
  },
  "governance_applied": [
    "env.staging_only",
    "trace.required"
  ],
  "context_provided": [
    {"id": "vib3-what-it-is", "hash": "sha256:..."},
    {"id": "vib3-geometry-system", "hash": "sha256:..."}
  ],
  "trace_events": 47,
  "feedback_given": [
    {"context_id": "vib3-geometry-system", "helpful": true}
  ],
  "outcome": "completed",
  "ended_at": "2024-01-15T11:45:00Z"
}
```

### Domain Dashboard (Conceptual)

```
┌─────────────────────────────────────────────────────────────────┐
│  VIB3 Development Domain - CRA Dashboard                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Active Sessions: 3                                              │
│  ├─ claude-3-opus (production, unsupervised) - 2 actions        │
│  ├─ gpt-4 (staging, supervised) - 17 actions                    │
│  └─ claude-3-sonnet (development, supervised) - 5 actions       │
│                                                                  │
│  Today's Stats:                                                  │
│  ├─ Sessions: 24                                                │
│  ├─ Actions traced: 487                                          │
│  ├─ Context requests: 156                                        │
│  ├─ Feedback received: 89                                        │
│  └─ Violations: 0                                                │
│                                                                  │
│  Context Effectiveness:                                          │
│  ├─ vib3-geometry-system: 94% helpful                           │
│  ├─ vib3-what-it-is: 78% helpful                                 │
│  └─ vib3-embed-iframe: 100% helpful                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Interactive Onboarding

### Without Interactive Onboarding
```
- Generic governance for all agents
- Context that may overwhelm or underwhelm
- No visibility into agent capabilities
- No tailoring to authorization level
- One-size-fits-all TRACE
```

### With Interactive Onboarding
```
+ Governance matches agent's actual permissions
+ Context sized for agent's window
+ Domain owner sees exactly who's accessing
+ TRACE protection taught explicitly
+ Agent understands consequences
+ Appropriate policies for environment
```

---

## Implementation Notes

### Keep It Simple

As you said, don't make it "cloud infrastructure confusing." The onboarding should be:

1. **Quick** - A few questions, not an interrogation
2. **Clear** - Agent understands what's happening
3. **Practical** - Answers directly affect behavior
4. **Transparent** - Both sides know what's logged

### Minimum Viable Onboarding

For MVP, the discovery can be as simple as:

```
CRA: What's your intent?
Agent: Add VIB3 background to website

CRA: What can you do? (tools)
Agent: read, write files

CRA: What environment?
Agent: development

CRA: [Sends tailored governance + relevant context]
```

### Future: Dynamic Policies

Later, domain owners could configure their own discovery questions:

```json
{
  "domain_questions": [
    {
      "id": "team_membership",
      "question": "What team are you working for?",
      "maps_to_policy": "team_access_control"
    },
    {
      "id": "data_classification",
      "question": "Will you be handling PII?",
      "if_yes": "enable pii_protection policy"
    }
  ]
}
```

---

## Summary

Interactive onboarding transforms CRA from a static system to a dynamic one:

1. **Ask** - CRA learns about the agent
2. **Configure** - Governance tailored to answers
3. **Teach** - Agent understands TRACE
4. **Protect** - TRACE monitoring explained
5. **Provide** - Right-sized context delivered
6. **Track** - Domain owner sees everything

The agent builds its "wrapper" by answering honestly and receiving appropriate governance. The wrapper isn't code - it's the configured state of the governed session.
