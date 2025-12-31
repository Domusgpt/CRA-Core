# CRA Wrapper System

## Core Concept

The agent **builds its own wrapper**. This is not just setup - it's a unified process where:

1. Agent constructs the integration code
2. While building, agent learns the system
3. Construction IS the onboarding
4. Construction IS the authentication
5. Construction creates verified identity for TRACE
6. Result: Working wrapper + understanding + security + access

---

## Why Agent Builds It Themselves

### 1. Tailored Integration

The agent creates a wrapper specific to:
- Its model/capabilities (detected automatically)
- Its environment (Claude Code, OpenAI, custom)
- Its custodian's needs
- The atlases it will use

No generic wrapper - built for this specific agent.

### 2. Learning Through Building

By constructing the wrapper, the agent:
- Understands how CRA works (not just told about it)
- Knows what the wrapper does (because it built it)
- Can troubleshoot issues (understands the code)
- Internalizes governance (embedded in what it built)

### 3. Secure Handshake

The build process IS the security handshake:
- Agent agrees to terms during construction
- Agent authenticates as part of building
- Agent's identity is verified and recorded
- Wrapper creation is itself a TRACE event

### 4. Access Through Construction

By building the wrapper, the agent:
- Makes its I/O accessible to CRA (intentionally)
- Understands what access it's granting
- Can't claim it didn't know (it built it)

---

## What CRA Knows Automatically

CRA should detect most things without asking:

| Auto-Detected | How |
|---------------|-----|
| Model identity | From API headers, environment, or system prompt |
| Context window | Standard for each model |
| Available tools | From MCP tool list or environment |
| Environment | Claude Code vs API vs custom |
| Custodian | From config or session origin |

**Only ask essential questions:**
- What is your intent for this session?
- What authorization level? (dev/staging/prod)
- Any special constraints?

Don't interrogate - detect and confirm.

---

## The Build Process

```
┌─────────────────────────────────────────────────────────────────────┐
│                     WRAPPER BUILD PROCESS                            │
│                                                                      │
│  1. INITIATE                                                         │
│     Agent: "I need to integrate with CRA"                           │
│     CRA: Detects model, environment, capabilities automatically      │
│                                                                      │
│  2. ESSENTIAL QUESTIONS                                              │
│     CRA: "What's your intent? Authorization level?"                  │
│     Agent: Answers (minimal questions)                               │
│                                                                      │
│  3. TERMS & AUTH                                                     │
│     CRA: Presents governance terms based on detected + answered info │
│     Agent: Agrees (recorded in TRACE as genesis event)              │
│                                                                      │
│  4. WRAPPER CONSTRUCTION                                             │
│     CRA: Provides wrapper template tailored to this agent           │
│     Agent: Builds/implements the wrapper                             │
│     CRA: Guides, explains each part as agent builds                  │
│                                                                      │
│  5. VERIFICATION                                                     │
│     CRA: Verifies wrapper is correctly constructed                   │
│     CRA: Tests that I/O access works                                 │
│     CRA: Records agent identity verification in TRACE               │
│                                                                      │
│  6. ACTIVATION                                                       │
│     Wrapper is now live                                              │
│     CRA has access to agent I/O                                      │
│     TRACE is recording                                               │
│     Context injection is active                                      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## What Gets Created

### 1. CRA Integration MD

A markdown file tailored to THIS agent:

```markdown
# CRA Integration for claude-3-opus @ acme-corp

Session: sess-abc123
Model: claude-3-opus (auto-detected)
Environment: Claude Code
Authorization: staging
Custodian: acme-corp

## Your Wrapper

You built this wrapper during bootstrap. It:
- Intercepts your inputs (you made this accessible)
- Injects context from loaded atlases
- Records your outputs to TRACE
- Enforces policies you agreed to

## Governance You Agreed To

- All significant actions reported to TRACE
- Context injection on relevant prompts
- Policy checks before destructive actions
- Feedback on context usefulness

## Your TRACE Identity

Your identity in TRACE is verified:
- Model: claude-3-opus
- Session: sess-abc123
- Genesis hash: sha256:abc123...
- Verification: Wrapper construction recorded

This cannot be spoofed - you built the wrapper that reports as you.
```

### 2. Wrapper Code

Actual integration code the agent creates. For Claude Code:

```python
# Built by agent during bootstrap
class CRAWrapper:
    def __init__(self, session_id, cra_client):
        self.session_id = session_id
        self.cra = cra_client

    def wrap_input(self, prompt):
        # Get relevant context from CRA
        context = self.cra.get_context(prompt)
        # Inject into prompt
        return self.inject_context(prompt, context)

    def wrap_output(self, response):
        # Record to TRACE
        self.cra.trace_record(response)
        return response

    def before_action(self, action, params):
        # Policy check
        decision = self.cra.check_policy(action, params)
        if decision.denied:
            return decision.reason
        # Record to TRACE
        self.cra.trace_record(action, params)
        return None  # Proceed
```

The agent BUILDS this - understands every line.

### 3. TRACE Genesis Record

The wrapper construction itself is recorded:

```json
{
  "event_type": "wrapper_construction",
  "session_id": "sess-abc123",
  "timestamp": "2024-01-15T12:05:33Z",
  "agent": {
    "model": "claude-3-opus",
    "detected_capabilities": ["read", "write", "execute", "search"],
    "environment": "claude-code"
  },
  "custodian": "acme-corp",
  "terms_agreed": [
    "trace.all_actions",
    "context.injection",
    "policy.enforcement"
  ],
  "wrapper_hash": "sha256:def456...",
  "genesis_hash": "sha256:abc123..."
}
```

This proves:
- Which agent this is
- What it agreed to
- When it built the wrapper
- That the wrapper is authentic

---

## Security Through Construction

### Identity Verification

The agent's identity is verified BY the build process:

1. CRA detects model from environment
2. Agent constructs wrapper with specific patterns
3. Wrapper construction is hashed
4. Hash becomes part of TRACE identity

**You can't impersonate another agent** because:
- The wrapper YOU built has YOUR construction hash
- Every TRACE event links back to this genesis
- Attempting to claim you're someone else breaks the chain

### Tiered Access

Different content can require different verification:

```json
{
  "context_id": "basic-usage",
  "tier": "open",
  "requires": []
}

{
  "context_id": "production-config",
  "tier": "verified",
  "requires": ["wrapper_verified", "custodian_approved"]
}

{
  "context_id": "security-credentials",
  "tier": "restricted",
  "requires": ["wrapper_verified", "custodian_approved", "mfa_token"]
}
```

The wrapper can handle different security levels for different content.

### Custodian Controls

Custodians can require:
- Wrapper verification before any access
- Additional auth for certain atlases
- Approval workflow for high-risk actions
- Audit logging for sensitive contexts

All enforced through the wrapper the agent built.

---

## What The Wrapper Enables

Once built and active:

### Automatic Context Injection

```
Prompt arrives → Wrapper intercepts
                      ↓
              Wrapper calls CRA
                      ↓
              CRA returns relevant context
                      ↓
              Wrapper injects into prompt
                      ↓
              Agent sees prompt + context
```

Agent doesn't call a tool - wrapper handles it.

### Automatic TRACE Recording

```
Agent produces output → Wrapper captures
                              ↓
                        Wrapper records to TRACE
                              ↓
                        Hash chain extended
                              ↓
                        Output passed through
```

Agent doesn't report manually - wrapper handles it.

### Policy Enforcement

```
Agent attempts action → Wrapper intercepts
                              ↓
                        Wrapper checks policy via CRA
                              ↓
                        If denied: Block + explain
                        If allowed: Proceed + record
```

Policies enforced automatically.

### Identity in Every Event

```
Every TRACE event includes:
- Session ID (from wrapper construction)
- Agent identity (verified at build)
- Wrapper hash (proves authentic wrapper)
- Chain link (proves sequence)
```

Can't fake identity - wrapper construction proves who you are.

---

## For Different Agent Types

### Claude Code

Wrapper integrates with Claude Code's hook system:
- `pre_tool_use` hook for context injection
- `post_tool_use` hook for TRACE recording
- MCP tools for explicit CRA calls

### OpenAI Agents

Wrapper as middleware in the API call chain:
- Intercept completion requests
- Inject context into system prompt
- Capture and record responses

### Custom Agents

Wrapper as Python/JS decorator or middleware:
- Wrap the agent's main function
- Handle I/O interception
- Connect to CRA-Core via API or library

---

## The Full Picture

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   CUSTODIAN                                                          │
│   - Configures what atlases are available                           │
│   - Sets authorization requirements                                  │
│   - Monitors agent activity                                          │
│                                                                      │
│        │                                                             │
│        ▼                                                             │
│   ┌────────────────────────────────────────────────────────────┐   │
│   │  CRA MCP                                                    │   │
│   │  - Guides agent through wrapper construction               │   │
│   │  - Provides tailored wrapper template                      │   │
│   │  - Verifies wrapper is correct                             │   │
│   │  - Records genesis to TRACE                                │   │
│   └────────────────────────────────────────────────────────────┘   │
│        │                                                             │
│        ▼                                                             │
│   ┌────────────────────────────────────────────────────────────┐   │
│   │  AGENT (builds wrapper during onboarding)                   │   │
│   │                                                             │   │
│   │  ┌──────────────────────────────────────────────────────┐  │   │
│   │  │  WRAPPER (agent-built)                                │  │   │
│   │  │  - Intercepts I/O                                     │  │   │
│   │  │  - Injects context                                    │  │   │
│   │  │  - Records to TRACE                                   │  │   │
│   │  │  - Enforces policies                                  │  │   │
│   │  │  - Carries verified identity                          │  │   │
│   │  └──────────────────────────────────────────────────────┘  │   │
│   │                         │                                   │   │
│   │                         ▼                                   │   │
│   │               Agent does work (governed)                    │   │
│   │                                                             │   │
│   └────────────────────────────────────────────────────────────┘   │
│        │                                                             │
│        ▼                                                             │
│   ┌────────────────────────────────────────────────────────────┐   │
│   │  CRA-CORE (Rust)                                            │   │
│   │  - CARP: Context matching, policy evaluation               │   │
│   │  - TRACE: Hash chain, event recording                      │   │
│   │  - Atlas: Context blocks, governance rules                 │   │
│   └────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Summary

| Aspect | What Happens |
|--------|--------------|
| **Detection** | CRA auto-detects model, capabilities, environment |
| **Questions** | Only essential: intent, authorization level |
| **Construction** | Agent builds wrapper with CRA guidance |
| **Learning** | Agent understands system by building it |
| **Terms** | Agent agrees during construction (recorded) |
| **Auth** | Construction IS the authentication |
| **Identity** | Verified by wrapper construction hash |
| **Access** | Agent grants access by building the wrapper |
| **Injection** | Wrapper injects context automatically |
| **Recording** | Wrapper records to TRACE automatically |
| **Policies** | Wrapper enforces policies automatically |
| **Security** | Tiered access, custodian controls, verifiable identity |

**The wrapper is real code that the agent builds, giving CRA access to its I/O. Building it IS the onboarding, authentication, and terms agreement. The result is a governed agent with verified identity and automatic context/governance.**
