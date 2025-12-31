# CRA Bootstrap Protocol

## The Core Idea

The CRA wrapper isn't loaded like a normal atlas. It's **built during a handshake process** where:

1. Agent initiates connection
2. CRA streams governance rules, TRACE setup, and context
3. Agent acknowledges and builds its wrapper
4. Hash chain is established from the first message
5. By the time handshake completes, the agent IS governed

```
┌────────────────────────────────────────────────────────────────┐
│                  Bootstrap Handshake                            │
│                                                                 │
│  Agent                              CRA                         │
│    │                                 │                          │
│    │──── INIT (capabilities) ───────▶│                          │
│    │                                 │ Creates session          │
│    │◀─── GOVERNANCE (rules) ────────│ Genesis hash             │
│    │                                 │                          │
│    │──── ACK (understood) ──────────▶│                          │
│    │                                 │ Chains hash              │
│    │◀─── CONTEXT (domain knowledge)─│                          │
│    │                                 │                          │
│    │──── READY (wrapper built) ─────▶│                          │
│    │                                 │ Handshake complete       │
│    │◀─── SESSION (begin work) ──────│                          │
│    │                                 │                          │
│  Agent now has governance baked in   │                          │
└────────────────────────────────────────────────────────────────┘
```

---

## Why This Matters

### Normal Atlas Loading (What We Had)
```
Agent starts → Loads atlas → Reads rules → Tries to follow them
                              ↑
                         Agent could ignore
```

### Bootstrap Protocol (What We Need)
```
Agent starts → Handshake → Rules streamed INTO wrapper creation
                                    ↑
                              Agent IS the rules
```

The difference: With bootstrap, the agent's wrapper is **constructed from** the governance, not just **informed by** it.

---

## Protocol Messages

### 1. INIT (Agent → CRA)

Agent declares who it is and what it can do.

```json
{
  "type": "INIT",
  "agent_id": "claude-code-session-xyz",
  "capabilities": {
    "tools": ["read_file", "write_file", "execute_code", "web_search"],
    "protocols": ["mcp-1.0", "openai-actions"],
    "context_window": 200000
  },
  "intent": "User wants to build a website with VIB3 backgrounds"
}
```

### 2. GOVERNANCE (CRA → Agent)

CRA responds with governance rules that the agent must acknowledge.

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
      "description": "Request context before making domain-specific decisions",
      "enforcement": "soft"
    },
    {
      "rule_id": "feedback.expected",
      "description": "Provide feedback on context usefulness",
      "enforcement": "soft"
    }
  ],
  "policies": [
    {
      "policy_id": "no-destructive-without-confirm",
      "description": "Destructive actions require user confirmation",
      "actions_affected": ["delete_file", "drop_table", "rm -rf"]
    }
  ],
  "acknowledgment_required": true
}
```

### 3. ACK (Agent → CRA)

Agent acknowledges the governance, creating the first chain link.

```json
{
  "type": "ACK",
  "session_id": "sess-abc123",
  "previous_hash": "sha256:e3b0c44298fc...",
  "acknowledgments": [
    {"rule_id": "trace.required", "understood": true},
    {"rule_id": "context.must_request", "understood": true},
    {"rule_id": "feedback.expected", "understood": true}
  ],
  "wrapper_state": "building"
}
```

**Hash chain established:** CRA computes hash of ACK message linked to genesis.

### 4. CONTEXT (CRA → Agent)

CRA streams domain-specific context based on the declared intent.

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
      "content": "# What VIB3+ Is\n\nVIB3+ is a shader-based visualization engine...",
      "digest": "sha256:abc123..."
    },
    {
      "context_id": "vib3-geometry-system",
      "priority": 380,
      "inject_mode": "bootstrap",
      "content": "# Geometry System\n\n24 geometries: 8 base × 3 core...",
      "digest": "sha256:def456..."
    }
  ],
  "more_available": true
}
```

Each context block is hashed and chained. The agent receives:
- The content itself
- Proof that this specific content was provided
- Chain of custody from genesis

### 5. READY (Agent → CRA)

Agent signals wrapper is built and it's ready to work.

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

CRA confirms handshake complete and session begins.

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

---

## The Wrapper

The "wrapper" isn't a separate piece of code the agent writes. It's the **state the agent is in** after bootstrap.

### Before Bootstrap
```
Agent: Generic LLM with tools
       No governance awareness
       No domain knowledge
       No audit trail
```

### After Bootstrap
```
Agent: Governed LLM with:
       ✓ Internalized governance rules
       ✓ Domain context for current task
       ✓ Active hash chain linking all knowledge
       ✓ Tools for continued governance
```

### What "Wrapper Built" Means

When the agent sends `READY`, it's declaring:

1. **I understand the rules** - The governance messages are in my context
2. **I have the domain knowledge** - Context blocks are internalized
3. **I'm part of the chain** - My acknowledgments are hashed and recorded
4. **I can be audited** - Everything from genesis is traceable

---

## Hash Chain During Bootstrap

```
Genesis Hash
     │
     ▼
┌─────────────────┐
│ GOVERNANCE msg  │──hash──▶ H1
│ (rules, policies)│
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ ACK msg         │──hash(msg + H1)──▶ H2
│ (agent accepts) │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ CONTEXT msg 1   │──hash(msg + H2)──▶ H3
│ (domain blocks) │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ READY msg       │──hash(msg + H3)──▶ H4
│ (wrapper done)  │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ SESSION msg     │──hash(msg + H4)──▶ H5
│ (begin work)    │
└─────────────────┘
```

Every message in the bootstrap is chained. If anyone asks "what did the agent know?", the chain proves:
- Exactly what rules were sent
- That the agent acknowledged them
- What context was provided
- When the session began

---

## Implementation: MCP Bootstrap

For MCP-compatible agents (Claude, etc.), bootstrap happens through the tool system:

### Tool: cra_bootstrap

```json
{
  "name": "cra_bootstrap",
  "description": "Initialize CRA governance. MUST be called before any other CRA tools. This establishes governance rules and provides domain context.",
  "inputSchema": {
    "type": "object",
    "required": ["intent"],
    "properties": {
      "intent": {
        "type": "string",
        "description": "What you're trying to accomplish"
      },
      "capabilities": {
        "type": "array",
        "items": {"type": "string"},
        "description": "What tools/abilities you have"
      }
    }
  }
}
```

### Response (Streamed)

The response IS the bootstrap - it contains everything:

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
    },
    ...
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

**Key insight:** The tool response itself IS the wrapper. By receiving and processing this response, the agent has internalized the governance.

---

## Streaming Context During Bootstrap

For large atlases, context is streamed in chunks:

```
GOVERNANCE ─────────▶ Agent receives rules
CONTEXT chunk 1 ────▶ Agent receives essential facts
CONTEXT chunk 2 ────▶ Agent receives workflow guidance
CONTEXT chunk 3 ────▶ Agent receives API reference
CONTEXT chunk 4 ────▶ Agent receives examples
SESSION ────────────▶ Agent ready to work
```

Each chunk is hashed and chained. The agent builds its understanding incrementally, but each step is recorded.

---

## What This Enables

### 1. Proof of What Agent Knew

If an agent makes a mistake, we can prove:
- What context it was given
- When it received it
- That it acknowledged the rules

### 2. No Cheating

The agent can't claim "I didn't know" because:
- Every context block is hashed
- Its acknowledgment is recorded
- The chain proves receipt

### 3. Dynamic Governance

Different intents get different governance:
- "Write a blog post" → light governance
- "Deploy to production" → strict governance
- "Delete user data" → requires explicit policies

### 4. Self-Building Integration

The agent doesn't need pre-built wrappers:
- Starts with bootstrap tool
- Receives governance in response
- Builds its understanding from the stream
- Wrapper is constructed, not loaded

---

## Comparison to Traditional Integration

### Traditional: Load-Then-Run
```
1. Load atlas file
2. Parse JSON
3. Store in memory
4. Agent starts working
5. Hopefully follows rules
```

### Bootstrap: Stream-Build-Prove
```
1. Agent calls bootstrap
2. CRA streams governance
3. Agent acknowledges (hashed)
4. CRA streams context
5. Each piece hashed and chained
6. Agent signals ready (hashed)
7. Work begins with proof of understanding
```

---

## Next Steps

1. **Implement bootstrap protocol in Rust**
   - Add message types to TRACE
   - Create bootstrap flow in resolver
   - Add streaming context delivery

2. **Create MCP bootstrap tool**
   - Single tool that does full handshake
   - Returns streamed governance + context

3. **Test with Claude Code**
   - Verify bootstrap works
   - Confirm chain is properly built
   - Test governance enforcement

4. **Add platform adapters**
   - OpenAI Actions version
   - REST API version
   - WebSocket streaming version
