# CRA MCP Help System

## Dual-Purpose Help

The CRA MCP serves as a help function for two audiences:

1. **Agents** - How to use CRA, troubleshoot issues, understand governance
2. **Custodians** - How to monitor agents, understand traces, configure domains

```
┌─────────────────────────────────────────────────────────────────┐
│                    CRA MCP Functions                             │
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │    Bootstrap    │  │   Governance    │  │      Help       │  │
│  │   & Onboard     │  │   & Trace       │  │    & Support    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│         │                    │                    │              │
│         ▼                    ▼                    ▼              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ cra_bootstrap   │  │ cra_report_     │  │ cra_help        │  │
│  │ cra_discover    │  │   action        │  │ cra_explain     │  │
│  │ cra_acknowledge │  │ cra_feedback    │  │ cra_troubleshoot│  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## For Agents

### cra_help

General help about CRA and how to use it.

```json
{
  "name": "cra_help",
  "description": "Get help about CRA. Ask questions about tools, governance, TRACE, or troubleshooting.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "topic": {
        "type": "string",
        "description": "What you need help with",
        "examples": [
          "how do I report an action",
          "what is TRACE",
          "why was my action denied",
          "how do I request context"
        ]
      },
      "context": {
        "type": "string",
        "description": "Additional context about your situation"
      }
    }
  }
}
```

**Example Responses:**

```
Agent: cra_help(topic: "how do I report an action")

Response:
{
  "answer": "Use cra_report_action before any significant action:\n\n```\ncra_report_action(\n  action: 'write_file',\n  params: {path: '/app/index.html'}\n)\n```\n\nThis creates a TRACE event and may return approved/denied based on policies.",

  "related_topics": [
    "what counts as a significant action",
    "what if my action is denied",
    "how does TRACE work"
  ],

  "examples": [...]
}
```

```
Agent: cra_help(topic: "why was my action denied")

Response:
{
  "answer": "Actions can be denied for several reasons:\n\n1. **Policy violation**: The action type is not allowed for your user type\n2. **Environment restriction**: You're trying to access production when only authorized for staging\n3. **Missing checkpoint**: Destructive actions may require checkpoints first\n4. **Rate limit**: You've exceeded allowed actions for this period\n\nTo see the specific reason, check the 'reason' field in the denial response.",

  "next_steps": [
    "Check your authorization level in your session info",
    "Review the policy that denied you",
    "Request a checkpoint if needed"
  ]
}
```

### cra_explain

Explain a specific CRA concept or output.

```json
{
  "name": "cra_explain",
  "description": "Explain a CRA concept, output, or error in detail.",
  "inputSchema": {
    "type": "object",
    "required": ["what"],
    "properties": {
      "what": {
        "type": "string",
        "description": "What to explain - a concept, an error, or paste a CRA output"
      }
    }
  }
}
```

**Examples:**

```
Agent: cra_explain(what: "hash chain")

Response:
{
  "concept": "Hash Chain",
  "explanation": "A hash chain links events together cryptographically. Each event's hash includes the previous event's hash, creating an unbreakable sequence.\n\n```\nEvent 1 → hash(event1) = H1\nEvent 2 → hash(event2 + H1) = H2\nEvent 3 → hash(event3 + H2) = H3\n```\n\nIf anyone modifies Event 2, H2 changes, which breaks H3's link. This is how TRACE detects tampering.",

  "why_it_matters": "Your actions form a chain that proves exactly what happened, in order. No one can alter history without detection.",

  "for_you": "You don't need to compute hashes. CRA handles it. Just report your actions and the chain builds automatically."
}
```

```
Agent: cra_explain(what: {"error": "chain_verification_failed", "expected": "sha256:abc...", "got": "sha256:def..."})

Response:
{
  "error_type": "Chain Verification Failed",
  "explanation": "The hash chain is broken. The expected hash doesn't match what was computed.\n\nThis can happen if:\n1. An event was modified after creation\n2. An event was inserted or deleted\n3. Events arrived out of order\n4. There's a bug in your wrapper's hash handling",

  "severity": "Critical - your TRACE log may be compromised",

  "recovery": "Contact domain admin. The session may need to be invalidated. Future sessions should work correctly."
}
```

### cra_troubleshoot

Diagnose and fix common issues.

```json
{
  "name": "cra_troubleshoot",
  "description": "Diagnose and get solutions for CRA issues.",
  "inputSchema": {
    "type": "object",
    "required": ["problem"],
    "properties": {
      "problem": {
        "type": "string",
        "description": "Describe what's not working"
      },
      "session_id": {
        "type": "string",
        "description": "Your session ID if you have one"
      }
    }
  }
}
```

**Example:**

```
Agent: cra_troubleshoot(problem: "I'm not getting any context when I request it")

Response:
{
  "diagnosis": [
    {
      "possible_cause": "No atlases loaded for your domain",
      "check": "Call cra_list_atlases() to see what's available",
      "fix": "Ask your custodian to load appropriate atlases"
    },
    {
      "possible_cause": "Your need doesn't match any keywords",
      "check": "Try cra_search_contexts() with different terms",
      "fix": "Be more specific or use different keywords in your request"
    },
    {
      "possible_cause": "Context window too small",
      "check": "Check your discovery answers - did you report a small context window?",
      "fix": "CRA may be conserving your context. Request specific contexts by ID."
    }
  ],

  "quick_test": "Try: cra_search_contexts(query: '*') to see if ANY contexts are available."
}
```

---

## For Custodians

### cra_custodian_help

Help for humans managing agents and CRA domains.

```json
{
  "name": "cra_custodian_help",
  "description": "Help for custodians (humans) managing agents and CRA domains.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "topic": {
        "type": "string",
        "description": "What you need help with as a custodian"
      }
    }
  }
}
```

**Example Topics:**

```
Custodian: cra_custodian_help(topic: "how do I know what my agent is doing")

Response:
{
  "answer": "You can monitor your agent through several methods:\n\n1. **Real-time TRACE**: Watch the `.cra/trace/{session-id}.json` file for live updates\n\n2. **Session summary**: Use `cra_admin_session_summary(session_id)` to get a digest\n\n3. **Action alerts**: Configure alerts for specific action types in your domain config\n\n4. **Dashboard** (future): Visual dashboard showing active sessions and actions",

  "commands": [
    "cra_admin_list_sessions() - See all active sessions",
    "cra_admin_session_trace(session_id) - Get full trace for a session",
    "cra_admin_session_summary(session_id) - Get high-level summary"
  ]
}
```

```
Custodian: cra_custodian_help(topic: "how do I restrict my agent to only staging")

Response:
{
  "answer": "You can restrict an agent to staging in two ways:\n\n1. **Via session key**: Issue a session key with staging-only permissions\n\n```json\ncra_issue_session_key(\n  for_user_type: 'agent',\n  permissions: ['staging'],\n  purpose: 'Staging deployment task'\n)\n```\n\n2. **Via domain policy**: Add a policy that denies production access for agents\n\n```json\n{\n  \"policy_id\": \"agent-staging-only\",\n  \"applies_to\": [\"user_type:agent\"],\n  \"denies\": [\"permission:production\"]\n}\n```",

  "recommendation": "Session keys are better for one-off restrictions. Policies are better for permanent rules."
}
```

### cra_explain_trace

Help custodians understand what happened in a trace.

```json
{
  "name": "cra_explain_trace",
  "description": "Explain a TRACE log in human-readable terms.",
  "inputSchema": {
    "type": "object",
    "required": ["trace"],
    "properties": {
      "trace": {
        "type": "string",
        "description": "Session ID, trace file path, or paste trace JSON"
      },
      "focus": {
        "type": "string",
        "description": "What aspect to focus on (timeline, actions, decisions, context)"
      }
    }
  }
}
```

**Example:**

```
Custodian: cra_explain_trace(trace: "sess-xyz789", focus: "timeline")

Response:
{
  "session_summary": {
    "started": "2024-01-15 12:05:33",
    "ended": "2024-01-15 12:47:22",
    "duration": "42 minutes",
    "agent": "claude-3-sonnet",
    "intent": "Add VIB3 shader background to portfolio site"
  },

  "timeline": [
    {
      "time": "12:05:33",
      "event": "Session started",
      "detail": "Bootstrap completed, governance accepted"
    },
    {
      "time": "12:06:01",
      "event": "Context requested",
      "detail": "Asked for 'how to embed VIB3 in HTML'"
    },
    {
      "time": "12:06:02",
      "event": "Context provided",
      "detail": "vib3-embed-iframe (3 blocks total)"
    },
    {
      "time": "12:08:15",
      "event": "Action reported",
      "detail": "write_file: index.html",
      "decision": "approved"
    },
    {
      "time": "12:15:33",
      "event": "Feedback given",
      "detail": "vib3-embed-iframe marked helpful"
    },
    ...
  ],

  "verdict": "Normal session. Agent followed governance, reported all actions, provided feedback. No policy violations."
}
```

### cra_configure_help

Help custodians configure their domain.

```json
{
  "name": "cra_configure_help",
  "description": "Help configuring CRA domain settings, policies, and atlases.",
  "inputSchema": {
    "type": "object",
    "required": ["what"],
    "properties": {
      "what": {
        "type": "string",
        "description": "What you want to configure"
      }
    }
  }
}
```

---

## Quick Reference Cards

### For New Agents

When an agent first encounters CRA:

```
Welcome to CRA! Here's what you need to know:

1. BOOTSTRAP FIRST
   Call cra_bootstrap(intent: "what you're doing")
   This sets up governance and gives you context.

2. REPORT ACTIONS
   Before significant actions, call cra_report_action(...)
   This creates your audit trail.

3. REQUEST CONTEXT
   When you need information, call cra_request_context(...)
   Be specific about what you need.

4. GIVE FEEDBACK
   After using context, call cra_feedback(...)
   This helps improve the system.

5. GET HELP
   Confused? Call cra_help(topic: "your question")
   We're here to help.
```

### For New Custodians

When a custodian first sets up CRA:

```
Welcome to CRA Management! Here's your quick start:

1. SETUP DOMAIN
   Create .cra/config.json with your domain settings.
   Choose auth mode (open/password/key_based).

2. LOAD ATLASES
   Add atlas files to .cra/atlases/
   These define context and policies for your agents.

3. ISSUE KEYS
   For key-based auth, issue keys to your agents:
   cra_admin_issue_key(for_user_type: "agent", ...)

4. MONITOR
   Watch your agents:
   cra_admin_list_sessions()
   cra_admin_session_trace(session_id)

5. GET HELP
   Questions? Call cra_custodian_help(topic: "...")
```

---

## Onboarding Flows

### Agent Onboarding Flow

```
1. Agent discovers CRA tools are available
         │
         ▼
2. Agent calls cra_help(topic: "how do I get started")
         │
         ▼
3. CRA explains bootstrap process
         │
         ▼
4. Agent calls cra_bootstrap(intent: "...")
         │
         ▼
5. CRA runs interactive discovery
   CRA teaches TRACE
   CRA provides context
         │
         ▼
6. Agent is now onboarded and can work
```

### Custodian Onboarding Flow

```
1. Custodian installs CRA MCP
         │
         ▼
2. Custodian calls cra_custodian_help(topic: "setup")
         │
         ▼
3. CRA walks through domain configuration
         │
         ▼
4. Custodian creates config.json
   Custodian loads atlases
   Custodian sets auth mode
         │
         ▼
5. Custodian issues session keys for agents
         │
         ▼
6. Domain is ready for agents to connect
```

---

## Help Tool Summary

| Tool | For | Purpose |
|------|-----|---------|
| `cra_help` | Agents | General help, how-to questions |
| `cra_explain` | Agents | Explain concepts, outputs, errors |
| `cra_troubleshoot` | Agents | Diagnose and fix issues |
| `cra_custodian_help` | Custodians | Management and monitoring help |
| `cra_explain_trace` | Custodians | Understand what happened in a session |
| `cra_configure_help` | Custodians | Help with domain configuration |

---

## Philosophy

The help system isn't just documentation - it's an **onboarding partner**:

1. **Agents** shouldn't need to read docs. They can just ask.
2. **Custodians** shouldn't need to study CRA. They can get guided.
3. **Errors** should explain themselves. Every error has context.
4. **Learning** is continuous. Help improves from feedback.

The MCP is not just governance infrastructure - it's a helpful guide for everyone interacting with it.
