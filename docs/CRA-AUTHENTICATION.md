# CRA Authentication System

## Overview

CRA admins can configure authentication requirements for their domain:

1. **Open** - No auth required (local development)
2. **Key-Based** - API keys for different user types
3. **Password** - Simple password protection
4. **Session-Bound** - CRA issues session tokens

---

## Domain Configuration

```json
{
  "domain": "my-org-project",
  "auth": {
    "mode": "key_based",
    "require_auth": true,
    "allow_anonymous": false,

    "user_types": [
      {
        "type": "admin",
        "permissions": ["all"],
        "can_issue_keys": true,
        "governance_level": "minimal"
      },
      {
        "type": "developer",
        "permissions": ["development", "staging"],
        "can_issue_keys": false,
        "governance_level": "standard"
      },
      {
        "type": "agent",
        "permissions": ["development"],
        "governance_level": "strict",
        "requires_session_key": true
      },
      {
        "type": "external",
        "permissions": ["read_only"],
        "governance_level": "maximum",
        "rate_limit": "100/hour"
      }
    ],

    "session_settings": {
      "issue_session_keys": true,
      "session_key_expiry": "24h",
      "session_key_format": "cra-{domain}-{timestamp}-{random}"
    }
  }
}
```

---

## Authentication Modes

### 1. Open Mode (No Auth)

Best for: Local development, personal projects

```json
{
  "auth": {
    "mode": "open",
    "require_auth": false
  }
}
```

Any agent can connect. No credentials required.

### 2. Password Mode

Best for: Simple protection, small teams

```json
{
  "auth": {
    "mode": "password",
    "require_auth": true,
    "password_hash": "sha256:..."
  }
}
```

Agent provides password during bootstrap:
```json
{
  "cra_bootstrap": {
    "intent": "...",
    "auth": {
      "password": "secret123"
    }
  }
}
```

### 3. Key-Based Mode

Best for: Organizations, multiple users/agents

```json
{
  "auth": {
    "mode": "key_based",
    "require_auth": true,
    "keys": {
      "cra-key-abc123": {
        "user_type": "developer",
        "owner": "alice@company.com",
        "created": "2024-01-15",
        "expires": "2025-01-15"
      },
      "cra-key-def456": {
        "user_type": "agent",
        "owner": "claude-agent-1",
        "created": "2024-01-15",
        "permissions_override": ["staging"]
      }
    }
  }
}
```

Agent provides key:
```json
{
  "cra_bootstrap": {
    "intent": "...",
    "auth": {
      "api_key": "cra-key-abc123"
    }
  }
}
```

### 4. Session-Bound Mode

Best for: High-security, audit-focused environments

```json
{
  "auth": {
    "mode": "session_bound",
    "require_auth": true,
    "issue_session_keys": true,
    "session_key_single_use": true
  }
}
```

Flow:
1. User/admin generates session key via CRA admin panel
2. Session key given to agent
3. Agent uses key for bootstrap
4. Key bound to that session only

---

## Session Key System

### Issuing Session Keys

Admin or authorized user can issue session keys:

```json
{
  "cra_issue_session_key": {
    "for_user_type": "agent",
    "permissions": ["development"],
    "expires_in": "4h",
    "single_use": true,
    "purpose": "VIB3 website build task"
  }
}
```

Response:
```json
{
  "session_key": "cra-myorg-20240115-a7f3b2c1",
  "expires_at": "2024-01-15T16:00:00Z",
  "permissions": ["development"],
  "single_use": true,
  "usage": "Give this to the agent. It will be bound to one session."
}
```

### Using Session Keys

Agent uses the key during bootstrap:
```json
{
  "cra_bootstrap": {
    "intent": "Build VIB3 website",
    "auth": {
      "session_key": "cra-myorg-20240115-a7f3b2c1"
    }
  }
}
```

CRA verifies:
- Key exists and not expired
- Key not already used (if single_use)
- Permissions match intent

### Key Binding

Once used, the session key is bound:
```json
{
  "session_key": "cra-myorg-20240115-a7f3b2c1",
  "bound_to_session": "sess-xyz789",
  "bound_at": "2024-01-15T12:05:33Z",
  "agent_identity": "claude-3-sonnet",
  "intent": "Build VIB3 website"
}
```

If someone tries to use it again:
```json
{
  "error": "Session key already used",
  "bound_to_session": "sess-xyz789",
  "bound_at": "2024-01-15T12:05:33Z"
}
```

---

## User Type Permissions

### Permission Levels

| Permission | Description |
|------------|-------------|
| `all` | Full access to everything |
| `production` | Can access production resources |
| `staging` | Can access staging resources |
| `development` | Can access development resources |
| `read_only` | Can only request context, cannot report actions |

### Governance Levels

| Level | Description |
|-------|-------------|
| `minimal` | Light touch - only log major actions |
| `standard` | Normal governance - all actions logged |
| `strict` | Every action requires explicit approval |
| `maximum` | All actions logged + rate limited + require checkpoint |

### Example: Different Users Get Different Treatment

```
Admin (Alice):
  - All permissions
  - Minimal governance
  - No rate limits
  - Can issue keys

Developer (Bob):
  - Development + staging
  - Standard governance
  - Normal rate limits
  - Cannot issue keys

Claude Agent (via session key):
  - Development only
  - Strict governance
  - Rate limited
  - Single session only
```

---

## Bootstrap With Auth

### Full Bootstrap Request

```json
{
  "cra_bootstrap": {
    "intent": "Add VIB3 shader background to portfolio",
    "auth": {
      "type": "session_key",
      "session_key": "cra-myorg-20240115-a7f3b2c1"
    },
    "discovery_answers": {
      "model_identity": "claude-3-sonnet",
      "tool_capabilities": ["read_file", "write_file"],
      "context_window": 200000,
      "authorization_level": "development",
      "supervision": "human_supervised"
    }
  }
}
```

### Response Includes Auth Status

```json
{
  "session_id": "sess-xyz789",
  "genesis_hash": "sha256:...",

  "auth_status": {
    "authenticated": true,
    "user_type": "agent",
    "permissions": ["development"],
    "governance_level": "strict",
    "session_key_bound": true,
    "expires_at": "2024-01-15T16:00:00Z"
  },

  "governance": {
    "rules": [...],
    "tailored_for": {
      "user_type": "agent",
      "governance_level": "strict"
    }
  },

  "context": [...],
  "ready": true
}
```

---

## Admin Operations

### Issue API Key

```json
{
  "cra_admin_issue_key": {
    "admin_key": "cra-admin-master",
    "for_user_type": "developer",
    "owner": "newdev@company.com",
    "expires": "2025-01-15"
  }
}
```

### Revoke Key

```json
{
  "cra_admin_revoke_key": {
    "admin_key": "cra-admin-master",
    "key_to_revoke": "cra-key-abc123",
    "reason": "Employee departed"
  }
}
```

### List Active Sessions

```json
{
  "cra_admin_list_sessions": {
    "admin_key": "cra-admin-master",
    "filter": "active"
  }
}
```

Response:
```json
{
  "sessions": [
    {
      "session_id": "sess-xyz789",
      "user_type": "agent",
      "started": "2024-01-15T12:05:33Z",
      "actions_reported": 17,
      "intent": "Build VIB3 website"
    }
  ]
}
```

### Terminate Session

```json
{
  "cra_admin_terminate_session": {
    "admin_key": "cra-admin-master",
    "session_id": "sess-xyz789",
    "reason": "Suspicious activity"
  }
}
```

---

## Security Considerations

### Key Storage
- API keys should be stored hashed, not plaintext
- Session keys are ephemeral and can be plaintext
- Admin keys should use strong secrets

### Key Transmission
- Keys travel in the bootstrap request
- For MCP, this goes through the tool call
- For REST API, use HTTPS

### Expiry
- Session keys: 1-24 hours typical
- API keys: 1 year typical
- Admin keys: No expiry but can be rotated

### Revocation
- Any key can be revoked immediately
- Revoked session keys terminate active sessions
- Revocation is logged in TRACE

---

## Roadmap

### Phase 1: Basic Auth
- Password mode
- Single admin key
- Session-level governance

### Phase 2: User Types
- Multiple API keys
- User type permissions
- Governance level per type

### Phase 3: Session Keys
- Issuable session keys
- Single-use keys
- Key binding

### Phase 4: Enterprise
- SSO integration
- Team management
- Audit logs for auth events
