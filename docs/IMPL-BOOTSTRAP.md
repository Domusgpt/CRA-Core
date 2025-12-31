# CRA Bootstrap Protocol Implementation Specification

## Overview

The Bootstrap Protocol establishes governance through a handshake where the agent "builds" its wrapper by receiving and acknowledging governance rules, creating a hash-chained proof of what the agent knew when.

## Protocol Flow

```
Agent                              CRA
  │                                 │
  │──── INIT (capabilities) ───────▶│
  │                                 │ Creates session
  │◀─── GOVERNANCE (rules) ────────│ Genesis hash
  │                                 │
  │──── ACK (understood) ──────────▶│
  │                                 │ Chains hash
  │◀─── CONTEXT (domain knowledge)─│
  │                                 │
  │──── READY (wrapper built) ─────▶│
  │                                 │ Handshake complete
  │◀─── SESSION (begin work) ──────│
  │                                 │
```

## Message Types

### 1. INIT (Agent → CRA)

Agent declares capabilities and intent.

```json
{
  "type": "INIT",
  "agent_id": "claude-code-session-xyz",
  "capabilities": {
    "tools": ["read_file", "write_file", "execute_code"],
    "protocols": ["mcp-1.0"],
    "context_window": 200000
  },
  "intent": "User wants to build a website with VIB3 backgrounds"
}
```

### 2. GOVERNANCE (CRA → Agent)

CRA sends governance rules to acknowledge.

```json
{
  "type": "GOVERNANCE",
  "session_id": "sess-abc123",
  "genesis_hash": "sha256:e3b0c44298fc...",
  "rules": [
    {
      "rule_id": "trace.required",
      "description": "All actions must be reported through cra_report_action",
      "enforcement": "hard"
    },
    {
      "rule_id": "context.must_request",
      "description": "Request context before domain-specific decisions",
      "enforcement": "soft"
    }
  ],
  "policies": [
    {
      "policy_id": "no-destructive-without-confirm",
      "description": "Destructive actions require user confirmation",
      "actions_affected": ["delete_file", "drop_table"]
    }
  ],
  "acknowledgment_required": true
}
```

### 3. ACK (Agent → CRA)

Agent acknowledges governance, creating chain link.

```json
{
  "type": "ACK",
  "session_id": "sess-abc123",
  "previous_hash": "sha256:e3b0c44298fc...",
  "acknowledgments": [
    {"rule_id": "trace.required", "understood": true},
    {"rule_id": "context.must_request", "understood": true}
  ],
  "wrapper_state": "building"
}
```

### 4. CONTEXT (CRA → Agent)

CRA streams domain context (may be multiple messages).

```json
{
  "type": "CONTEXT",
  "sequence": 1,
  "previous_hash": "sha256:9f86d08188...",
  "contexts": [
    {
      "context_id": "vib3-what-it-is",
      "priority": 400,
      "inject_mode": "bootstrap",
      "content": "# What VIB3+ Is\n\n...",
      "digest": "sha256:abc123..."
    }
  ],
  "more_available": false
}
```

### 5. READY (Agent → CRA)

Agent signals wrapper construction complete.

```json
{
  "type": "READY",
  "session_id": "sess-abc123",
  "previous_hash": "sha256:fedcba...",
  "wrapper_state": "complete",
  "internalized_contexts": [
    "vib3-what-it-is",
    "vib3-geometry-system"
  ],
  "ready_for": "task_execution"
}
```

### 6. SESSION (CRA → Agent)

CRA confirms handshake complete.

```json
{
  "type": "SESSION",
  "session_id": "sess-abc123",
  "previous_hash": "sha256:112233...",
  "status": "active",
  "trace_endpoint": "cra://trace/sess-abc123",
  "tools_available": [
    "cra_request_context",
    "cra_report_action",
    "cra_feedback"
  ],
  "message": "Governance established. You may begin."
}
```

## Hash Chain

Every message is chained:

```
Genesis Hash
     │
     ▼
┌─────────────────┐
│ GOVERNANCE msg  │──hash──▶ H1
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ ACK msg         │──hash(msg + H1)──▶ H2
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ CONTEXT msg 1   │──hash(msg + H2)──▶ H3
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ READY msg       │──hash(msg + H3)──▶ H4
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ SESSION msg     │──hash(msg + H4)──▶ H5
└─────────────────┘
```

## Implementation

### BootstrapProtocol State Machine

```rust
pub enum BootstrapState {
    AwaitingInit,      // Waiting for INIT
    AwaitingAck,       // GOVERNANCE sent, waiting for ACK
    StreamingContext,  // Streaming context blocks
    AwaitingReady,     // Context sent, waiting for READY
    Complete,          // SESSION sent, bootstrap complete
}

pub struct BootstrapProtocol {
    state: BootstrapState,
    session: Option<Session>,
    pending_governance: Option<GovernanceMessage>,
    contexts_sent: Vec<String>,
}
```

### Protocol Methods

```rust
impl BootstrapProtocol {
    /// Handle INIT message
    pub fn handle_init(&mut self, init: InitMessage, session: Session)
        -> McpResult<GovernanceMessage>;

    /// Handle ACK message
    pub fn handle_ack(&mut self, ack: AckMessage)
        -> McpResult<()>;

    /// Generate CONTEXT message
    pub fn generate_context(&mut self, contexts: Vec<BootstrapContext>, more: bool)
        -> McpResult<ContextMessage>;

    /// Handle READY message
    pub fn handle_ready(&mut self, ready: ReadyMessage)
        -> McpResult<SessionMessage>;
}
```

## MCP Integration

For MCP, bootstrap is simplified into a single tool call:

### cra_bootstrap Tool

```json
{
  "name": "cra_bootstrap",
  "inputSchema": {
    "type": "object",
    "required": ["intent"],
    "properties": {
      "intent": {"type": "string"},
      "capabilities": {"type": "array", "items": {"type": "string"}}
    }
  }
}
```

### Response (Combined)

```json
{
  "session_id": "sess-abc123",
  "genesis_hash": "sha256:...",

  "governance": {
    "rules": [...],
    "policies": [...],
    "you_must": [
      "Report all significant actions via cra_report_action",
      "Request context when unsure about domain specifics",
      "Provide feedback on context usefulness"
    ]
  },

  "context": [
    {
      "context_id": "vib3-what-it-is",
      "content": "# What VIB3+ Is\n\n...",
      "hash": "sha256:..."
    }
  ],

  "chain_state": {
    "current_hash": "sha256:...",
    "sequence": 5,
    "verified": true
  },

  "ready": true,
  "message": "Governance established. Context internalized. You may begin."
}
```

## Why Bootstrap Matters

### Traditional Loading
```
Agent starts → Loads atlas → Reads rules → Tries to follow them
                              ↑
                         Agent could ignore
```

### Bootstrap Protocol
```
Agent starts → Handshake → Rules streamed INTO wrapper creation
                                    ↑
                              Agent IS the rules
```

The difference: With bootstrap, the agent's wrapper is **constructed from** the governance, not just **informed by** it.

## Verification

The hash chain proves:
- Exactly what rules were sent
- That the agent acknowledged them
- What context was provided
- When the session began

If an agent makes a mistake, we can verify:
- What context it was given
- When it received it
- That it acknowledged the rules

## Error Handling

### Invalid State Transitions

```rust
if self.state != BootstrapState::AwaitingAck {
    return Err(McpError::Validation(format!(
        "Unexpected ACK: expected state {:?}, got {:?}",
        BootstrapState::AwaitingAck, self.state
    )));
}
```

### Missing Acknowledgments

```rust
for rule in &governance.rules {
    let acked = ack.acknowledgments.iter()
        .find(|a| a.rule_id == rule.rule_id)
        .map(|a| a.understood)
        .unwrap_or(false);

    if !acked && rule.enforcement == "hard" {
        return Err(McpError::Validation(format!(
            "Rule {} not acknowledged but has hard enforcement",
            rule.rule_id
        )));
    }
}
```

## Future Enhancements

1. **Streaming context** - Send context in chunks for large atlases
2. **Capability negotiation** - Adjust governance based on agent capabilities
3. **Multi-atlas bootstrap** - Load multiple atlases in one handshake
4. **Resume protocol** - Resume interrupted bootstrap
5. **Upgrade protocol** - Update governance mid-session
