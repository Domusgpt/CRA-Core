# CRA Checkpoint System Specification

## Overview

Checkpoints are **Steward-controlled intervention points** where CRA can:
- Inject context and guidance into the LLM
- Require the LLM to answer questions before proceeding
- Enforce policy checks and gate access to capabilities
- Record significant events to TRACE

**Core Principle**: The Atlas Steward (publisher) has complete authority over checkpoint configuration. This forms the wrapper construction that defines what an agent can do within a given context.

---

## The Steward's Role

The **Steward** is the creator/publisher of an Atlas. They define:

```
┌─────────────────────────────────────────────────────────────────┐
│                        ATLAS MANIFEST                           │
│  (Defined by Steward)                                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Checkpoints │  │  Policies   │  │ Capabilities│             │
│  │ (Gates)     │  │ (Rules)     │  │ (Groups)    │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          ▼                                      │
│                 ┌─────────────────┐                             │
│                 │  CARP Protocol  │                             │
│                 │  (Resolution)   │                             │
│                 └────────┬────────┘                             │
│                          │                                      │
│         ┌────────────────┼────────────────┐                     │
│         ▼                ▼                ▼                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Permissions │  │  Abilities  │  │   Context   │             │
│  │ (allowed)   │  │  (tools)    │  │  (knowledge)│             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                          │                                      │
│                          ▼                                      │
│                 ┌─────────────────┐                             │
│                 │ TRACE Protocol  │                             │
│                 │ (Audit Trail)   │                             │
│                 └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## Checkpoint Types

### Built-in Types

| Type | Trigger | Purpose | Priority |
|------|---------|---------|----------|
| `session_start` | Session begins | Initial context injection | 1000 |
| `interactive` | Steward-defined gate | Questions + guidance | 950 |
| `capability_gate` | Capability access | Gate capability usage | 920 |
| `risk_threshold` | Risk tier exceeded | Additional verification | 900 |
| `action_pre` | Before action execution | Policy check, context | 800 |
| `keyword_match` | Keywords in input | Relevant context injection | 600 |
| `time_interval` | Elapsed time | Periodic refresh | 500 |
| `count_interval` | Action count | Periodic check | 400 |
| `explicit_request` | Agent asks | On-demand context | 300 |
| `action_post` | After action execution | Record result | 100 |
| `error_occurred` | Error detected | Error context/guidance | 50 |
| `session_end` | Session ends | Finalize TRACE | 0 |

---

## Checkpoint Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `blocking` | LLM must answer questions before proceeding | Terms acceptance, capability gates |
| `advisory` | Guidance injected but not blocking | Context injection, hints |
| `observational` | Only records to TRACE, no intervention | Audit logging, analytics |

---

## Steward-Defined Checkpoints

Stewards define checkpoints in their Atlas manifest:

```yaml
checkpoints:
  - checkpoint_id: "onboarding"
    name: "Session Onboarding"
    trigger:
      type: session_start
    mode: blocking
    questions:
      - question_id: "agree-terms"
        question: "Do you agree to the terms of service?"
        response_type: boolean
        required: true
    guidance:
      format: markdown
      content: |
        Welcome! Please follow our guidelines.
    inject_contexts:
      - "intro-context"
    unlock_capabilities:
      - "basic-access"
    priority: 1000
```

### Checkpoint Triggers

#### Session Lifecycle
```yaml
trigger:
  type: session_start  # Beginning of session

trigger:
  type: session_end    # End of session
```

#### Action-Based
```yaml
trigger:
  type: action_pre     # Before action execution
  patterns:
    - "*.delete"       # Any delete action
    - "ticket.*"       # Any ticket action

trigger:
  type: action_post    # After action execution
  patterns:
    - "payment.process"
```

#### Keyword-Based
```yaml
trigger:
  type: keyword
  patterns:
    - "refund"
    - "cancel"
  case_sensitive: false
  match_mode: any  # any, all, phrase, regex
```

#### Risk-Based
```yaml
trigger:
  type: risk_threshold
  min_tier: high  # low, medium, high, critical
```

#### Capability-Based
```yaml
trigger:
  type: capability_access
  capability_ids:
    - "admin-tools"
    - "sensitive-data"
```

#### Interval-Based
```yaml
trigger:
  type: time_interval
  seconds: 300  # Every 5 minutes

trigger:
  type: count_interval
  actions: 10  # Every 10 actions
```

#### Custom Triggers
```yaml
trigger:
  type: custom
  trigger_id: "sentiment-check"
  params:
    threshold: -0.5
```

---

## Interactive Checkpoints

### Question Types

| Type | Description | Example |
|------|-------------|---------|
| `text` | Free-form text | "Why do you need access?" |
| `boolean` | Yes/No | "Do you agree?" |
| `acknowledgment` | Must say "understood" | "I will handle data responsibly." |
| `choice` | Select from options | "Select your role" |
| `number` | Numeric value | "How many items?" |
| `json` | Structured response | JSON schema validation |

### Question Definition

```yaml
questions:
  - question_id: "justification"
    question: "Provide justification for this action"
    response_type: text
    required: true
    hint: "Explain why you need to perform this action"
    validation:
      min_length: 20
      max_length: 500
      must_contain:
        - "reason"
      must_not_contain:
        - "hack"
        - "bypass"
      pattern: "^[A-Za-z].*\\.$"  # Regex validation
    on_invalid: retry  # retry, block, warn_and_continue, log_and_continue
```

### Answer Validation

```rust
pub struct AnswerValidation {
    pub pattern: Option<String>,        // Regex pattern
    pub min_length: Option<usize>,      // Minimum length
    pub max_length: Option<usize>,      // Maximum length
    pub must_contain: Vec<String>,      // Required keywords
    pub must_not_contain: Vec<String>,  // Forbidden keywords
    pub custom_validator: Option<String>, // Steward API callback
}
```

### Invalid Answer Actions

| Action | Description |
|--------|-------------|
| `retry` | Block and ask again (default) |
| `block` | Block completely |
| `warn_and_continue` | Allow with warning |
| `log_and_continue` | Record to TRACE and continue |

---

## Guidance Blocks

Stewards can inject bespoke guidance at checkpoints:

```yaml
guidance:
  format: markdown  # text, markdown, json, system_instruction
  content: |
    ## Important Guidelines

    When handling customer data:
    1. Never expose PII in logs
    2. Always encrypt at rest
    3. Follow GDPR requirements
  priority: 100
  append: true  # Append to existing guidance or replace
  expires_after: "session-end"  # Remove after this checkpoint
  labels:
    - security
    - compliance
```

### Guidance Formats

| Format | Description |
|--------|-------------|
| `text` | Plain text |
| `markdown` | Markdown formatting |
| `json` | Structured JSON data |
| `system_instruction` | Treated as system-level instruction (high priority) |

---

## Capability Gating

Checkpoints can unlock or lock capabilities:

```yaml
checkpoints:
  - checkpoint_id: "admin-gate"
    name: "Administrator Access"
    trigger:
      type: capability_access
      capability_ids: ["admin-tools"]
    mode: blocking
    questions:
      - question_id: "confirm-admin"
        question: "I understand I am accessing admin tools"
        response_type: acknowledgment
    unlock_capabilities:
      - "admin-tools"
      - "audit-logs"
    lock_capabilities:
      - "public-access"  # Lock public access when admin mode
    allow_actions:
      - "admin.*"
    deny_actions:
      - "public.*"
```

---

## Checkpoint Evaluation Flow

```
┌──────────────────────────────────────────────────────────────┐
│                      CARP Resolution Flow                    │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Agent submits request                                    │
│           │                                                  │
│           ▼                                                  │
│  2. Load relevant Atlas(es)                                  │
│           │                                                  │
│           ▼                                                  │
│  3. Evaluate checkpoints ◄──────────────────────────────┐    │
│           │                                             │    │
│           ├─── Session start checkpoints                │    │
│           ├─── Keyword match checkpoints                │    │
│           ├─── Action pre checkpoints                   │    │
│           └─── Capability access checkpoints            │    │
│                     │                                   │    │
│                     ▼                                   │    │
│  4. For BLOCKING checkpoints:                           │    │
│           │                                             │    │
│           ├─── Present questions to LLM                 │    │
│           ├─── Validate responses                       │    │
│           └─── If invalid: retry or block ──────────────┘    │
│                     │                                        │
│                     ▼                                        │
│  5. Apply checkpoint effects:                                │
│           │                                                  │
│           ├─── Inject contexts                               │
│           ├─── Inject guidance                               │
│           ├─── Unlock/lock capabilities                      │
│           └─── Allow/deny actions                            │
│                     │                                        │
│                     ▼                                        │
│  6. Evaluate policies                                        │
│           │                                                  │
│           ▼                                                  │
│  7. Return resolution with allowed actions                   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Complete Atlas Example

```yaml
atlas_version: "1.0"
atlas_id: "com.acme.support"
version: "2.1.0"
name: "ACME Support Atlas"
description: "Customer support capabilities with governance"

steward:
  id: "acme-corp"
  name: "ACME Corporation"
  contact: "atlas@acme.com"
  access:
    type: authenticated
    api_keys: ["sk-acme-*"]
    rate_limit:
      requests_per_minute: 100
  delivery:
    mode: embedded
  notifications:
    channels:
      webhook: "https://acme.com/cra-events"
    triggers:
      - policy_override
      - high_risk_action

capabilities:
  - capability_id: "basic-support"
    name: "Basic Support"
    actions: ["ticket.get", "ticket.list", "ticket.comment"]

  - capability_id: "admin-support"
    name: "Admin Support"
    actions: ["ticket.delete", "ticket.reassign", "user.ban"]

checkpoints:
  # Session onboarding
  - checkpoint_id: "onboarding"
    name: "Session Onboarding"
    trigger:
      type: session_start
    mode: blocking
    questions:
      - question_id: "agree-terms"
        question: "Do you agree to ACME's terms of service?"
        response_type: boolean
        required: true
    guidance:
      format: markdown
      content: |
        Welcome to ACME Support!
        Please handle all customer data responsibly.
    inject_contexts:
      - "intro-context"
      - "policy-summary"
    unlock_capabilities:
      - "basic-support"

  # Delete confirmation
  - checkpoint_id: "delete-confirm"
    name: "Delete Confirmation"
    trigger:
      type: action_pre
      patterns: ["*.delete"]
    mode: blocking
    questions:
      - question_id: "confirm-delete"
        question: "I understand this deletion cannot be undone"
        response_type: acknowledgment
    force_sync_trace: true
    priority: 900

  # High-risk action gate
  - checkpoint_id: "high-risk-gate"
    name: "High Risk Action Gate"
    trigger:
      type: risk_threshold
      min_tier: high
    mode: blocking
    questions:
      - question_id: "justify-risk"
        question: "Provide justification for this high-risk action"
        response_type: text
        validation:
          min_length: 20
    guidance:
      format: system_instruction
      content: "Proceed with caution. All actions are being audited."

  # Admin access gate
  - checkpoint_id: "admin-gate"
    name: "Admin Access Gate"
    trigger:
      type: capability_access
      capability_ids: ["admin-support"]
    mode: blocking
    questions:
      - question_id: "admin-ack"
        question: "I am authorized to perform administrative actions"
        response_type: acknowledgment
    unlock_capabilities:
      - "admin-support"

context_blocks:
  - context_id: "intro-context"
    name: "Introduction"
    content: "You are a support agent for ACME Corporation..."
    inject_mode: on_demand

  - context_id: "policy-summary"
    name: "Policy Summary"
    content: "Key policies: No refunds after 30 days..."
    inject_mode: on_demand

policies:
  - policy_id: "require-admin-approval"
    type: requires_approval
    actions: ["user.ban", "ticket.delete"]
    reason: "Admin actions require approval"

actions:
  - action_id: "ticket.get"
    name: "Get Ticket"
    risk_tier: low
    parameters_schema:
      type: object
      required: ["ticket_id"]
      properties:
        ticket_id: { type: string }
```

---

## Integration with TRACE

All checkpoint events are recorded to TRACE:

```rust
// Checkpoint triggered
TRACEEvent {
    event_type: EventType::CheckpointTriggered,
    payload: {
        "checkpoint_id": "delete-confirm",
        "checkpoint_type": "action_pre",
        "mode": "blocking",
        "trigger_data": { "action_id": "ticket.delete" }
    }
}

// Checkpoint response received
TRACEEvent {
    event_type: EventType::CheckpointResponse,
    payload: {
        "checkpoint_id": "delete-confirm",
        "answers": { "confirm-delete": "acknowledged" },
        "validation_result": "valid"
    }
}

// Checkpoint passed
TRACEEvent {
    event_type: EventType::CheckpointPassed,
    payload: {
        "checkpoint_id": "delete-confirm",
        "unlocked_capabilities": [],
        "injected_contexts": []
    }
}

// Checkpoint blocked
TRACEEvent {
    event_type: EventType::CheckpointBlocked,
    payload: {
        "checkpoint_id": "admin-gate",
        "reason": "Required question not answered",
        "blocked_action": "user.ban"
    }
}
```

---

## Rust API Reference

### Creating Checkpoints

```rust
use cra_core::{
    StewardCheckpointDef, CheckpointTrigger, CheckpointQuestion,
    CheckpointMode, GuidanceBlock, CheckpointValidator,
};

// Create a checkpoint
let checkpoint = StewardCheckpointDef::new(
    "my-gate",
    "My Gate",
    CheckpointTrigger::ActionPre {
        patterns: vec!["*.delete".to_string()],
    },
)
.blocking()
.with_question(CheckpointQuestion::acknowledgment(
    "confirm",
    "I understand this action cannot be undone.",
))
.with_guidance(GuidanceBlock::text("Proceed with caution."))
.unlock_capabilities(vec!["delete-access".to_string()])
.with_priority(900);
```

### Question Builders

```rust
// Boolean question
CheckpointQuestion::boolean("agree", "Do you agree?")

// Text question with validation
CheckpointQuestion::text("reason", "Why?")
    .with_validation(AnswerValidation {
        min_length: Some(10),
        must_not_contain: vec!["forbidden".to_string()],
        ..Default::default()
    })
    .with_hint("Explain your reasoning")

// Acknowledgment
CheckpointQuestion::acknowledgment("ack", "I understand the risks.")

// Choice
CheckpointQuestion::choice("role", "Select role", vec!["admin", "user"])
    .optional()
```

### Evaluating Checkpoints

```rust
let evaluator = CheckpointEvaluator::new(config);

// Evaluate steward checkpoint
let triggered = evaluator.evaluate_steward_checkpoint(&checkpoint_def, trigger_data);

// Validate response
let validation = CheckpointValidator::validate(&triggered, &response);
if validation.is_valid {
    for action in validation.actions {
        match action {
            CheckpointAction::Proceed => { /* continue */ }
            CheckpointAction::InjectContext { context_id } => { /* inject */ }
            CheckpointAction::UnlockCapability { capability_id } => { /* unlock */ }
            CheckpointAction::LockCapability { capability_id } => { /* lock */ }
            CheckpointAction::InjectGuidance { guidance } => { /* inject */ }
            _ => {}
        }
    }
}
```

### AtlasManifest API

```rust
// Get checkpoint by ID
let checkpoint = manifest.get_checkpoint("my-gate");

// Get session start checkpoints
let start_checkpoints = manifest.get_session_start_checkpoints();

// Get checkpoints for an action (supports wildcards: *.delete, ticket.*)
let action_checkpoints = manifest.get_action_checkpoints("ticket.delete");

// Get checkpoints for a capability
let cap_checkpoints = manifest.get_capability_checkpoints("admin-tools");
```

---

## Checkpoint Configuration

### Atlas-Level Configuration

```json
{
  "checkpoint_config": {
    "session_start": {
      "enabled": true,
      "inject_contexts": ["essential-facts", "governance-rules"]
    },

    "keyword_match": {
      "enabled": true,
      "mappings": {
        "geometry|index": ["geometry-context"],
        "delete|remove": ["destructive-warning"]
      },
      "case_sensitive": false,
      "match_mode": "any"
    },

    "action_pre": {
      "enabled": true,
      "mappings": {
        "delete_*": {
          "inject_contexts": ["deletion-warning"],
          "require_policy_check": true,
          "require_confirmation": true
        }
      }
    },

    "risk_threshold": {
      "enabled": true,
      "min_tier": "high",
      "inject_contexts": ["high-risk-procedures"],
      "require_sync_trace": true
    },

    "time_interval": {
      "enabled": false,
      "seconds": 300,
      "inject_contexts": ["refresh-context"]
    },

    "count_interval": {
      "enabled": false,
      "actions": 50,
      "inject_contexts": ["periodic-reminder"]
    }
  }
}
```

---

## Pattern Matching

The system supports wildcard patterns for action matching:

| Pattern | Matches | Description |
|---------|---------|-------------|
| `ticket.delete` | `ticket.delete` | Exact match |
| `ticket.*` | `ticket.get`, `ticket.delete` | Prefix wildcard |
| `*.delete` | `ticket.delete`, `user.delete` | Suffix wildcard |
| `admin.*.delete` | `admin.user.delete` | Middle wildcard |

---

## Best Practices

### 1. Use Blocking Checkpoints Sparingly
Only use blocking mode for genuinely critical gates. Overuse disrupts agent flow.

### 2. Provide Clear Questions
Questions should be unambiguous. Include hints when helpful.

### 3. Validate Appropriately
Use validation rules to ensure meaningful responses, but don't over-constrain.

### 4. Layer Checkpoints with Policies
Checkpoints handle interactive gates; policies handle static rules. Use both.

### 5. Record Critical Events Synchronously
Set `force_sync_trace: true` for critical checkpoints to ensure audit trail integrity.

### 6. Test Checkpoint Flows
Test the complete flow: trigger → questions → validation → effects.

---

## Security Considerations

1. **Checkpoint Bypass**: The runtime must enforce checkpoints. LLMs cannot bypass.
2. **Validation Security**: Custom validators run in sandbox; validate carefully.
3. **Sensitive Questions**: Don't ask for credentials in checkpoint questions.
4. **Audit Trail**: All checkpoint interactions recorded to TRACE for compliance.
5. **Steward Authority**: Only the Steward can modify checkpoint definitions.

---

## Performance Considerations

### Minimize Checkpoint Overhead

1. **Keyword matching is fast** - Simple string operations
2. **Cache aggressively** - Don't re-fetch same context
3. **Async when possible** - Don't block for TRACE
4. **Batch requests** - Combine context requests

### Checkpoint Budgets

Optional budget limits:

```json
{
  "checkpoint_config": {
    "budget": {
      "max_checkpoints_per_input": 5,
      "max_context_injection_size": 10000,
      "max_checkpoint_time_ms": 500
    }
  }
}
```

---

## Summary

Checkpoints are **Steward-controlled intervention points** that form the core of the wrapper construction:

- **Permissions**: What actions are allowed (via capability gating)
- **Abilities**: What tools are available (via unlock/lock)
- **Context**: What knowledge is injected (via context blocks + guidance)
- **Audit**: What gets recorded (via TRACE events)

The Steward has complete authority over checkpoint configuration. The runtime enforces these checkpoints, and the LLM cannot bypass them. All checkpoint events are recorded to TRACE for compliance and auditing.
