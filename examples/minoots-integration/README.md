# CRA + MINOOTS Integration

This example demonstrates how CRA (Context Registry Agents) provides governance for MINOOTS timer actions.

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    MINOOTS Timer System                      │
├─────────────────────────────────────────────────────────────┤
│  Timer expires                                               │
│       ↓                                                      │
│  Action Orchestrator                                         │
│       ↓                                                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  POST /v1/resolve to CRA                             │    │
│  │  → Check if action is allowed                        │    │
│  │  → Get policy constraints                            │    │
│  └─────────────────────────────────────────────────────┘    │
│       ↓                                                      │
│  If allowed: Execute webhook/CLI/file                        │
│  If denied:  Log and skip                                    │
│       ↓                                                      │
│  All actions recorded in CRA TRACE                           │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Start CRA server
docker-compose up -d

# Run demo
chmod +x demo.sh
./demo.sh
```

## Atlas Configuration

The `atlases/minoots-governance.json` file defines:

### Actions
| Action ID | Description | Risk Tier |
|-----------|-------------|-----------|
| `minoots.webhook.execute` | Send HTTP request | Medium |
| `minoots.webhook.retry` | Retry failed webhook | Medium |
| `minoots.cli.execute` | Run shell command | High |
| `minoots.file.write` | Write file | Medium |
| `minoots.file.append` | Append to file | Low |

### Policies
| Policy | Type | Effect |
|--------|------|--------|
| `rate-limit-webhooks` | Rate Limit | Max 100 webhooks/minute |
| `deny-dangerous-cli` | Deny | Block rm -rf, sudo, etc. |
| `require-approval-external-webhooks` | Approval | External URLs need approval |
| `business-hours-cli` | Temporal | CLI only 9am-6pm weekdays |
| `file-write-paths` | Allow | Only /tmp/minoots, /var/log/minoots |

## Integration Points

### MINOOTS Action Orchestrator

```typescript
// Before executing any timer action:
async function executeTimerAction(timer: Timer, action: Action) {
  // 1. Check with CRA
  const resolution = await fetch(`${CRA_URL}/v1/resolve`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      session_id: timer.metadata.cra_session_id,
      agent_id: 'minoots-action-orchestrator',
      goal: `${action.type}.execute`
    })
  }).then(r => r.json());

  // 2. Check decision
  if (resolution.decision === 'deny') {
    logger.warn(`Action blocked by CRA: ${resolution.denied_actions[0].reason}`);
    return { executed: false, reason: 'policy_denied' };
  }

  // 3. Execute if allowed
  return executeAction(action);
}
```

### Timer Metadata

Add CRA session to timer creation:

```json
{
  "name": "daily-report",
  "duration": "24h",
  "action": {
    "type": "webhook",
    "url": "https://api.example.com/reports"
  },
  "metadata": {
    "cra_session_id": "sess_abc123",
    "cra_policy_context": {
      "tenant_id": "acme-corp",
      "environment": "production"
    }
  }
}
```

## API Reference

### CRA Endpoints Used

```bash
# Create session for orchestrator
POST /v1/sessions
{
  "agent_id": "minoots-action-orchestrator",
  "goal": "Execute timer actions"
}

# Check if action is allowed
POST /v1/resolve
{
  "session_id": "...",
  "agent_id": "minoots-action-orchestrator",
  "goal": "minoots.webhook.execute"
}

# Get audit trail
GET /v1/traces/:session_id

# Verify integrity
GET /v1/traces/:session_id/verify
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_ENDPOINT` | `http://localhost:8420` | CRA server URL |
| `CRA_ENABLED` | `true` | Enable/disable CRA checks |
| `CRA_TIMEOUT_MS` | `5000` | Request timeout |
| `CRA_ON_UNAVAILABLE` | `allow` | Behavior when CRA is down |

## Network Setup

Both services should be on the same Docker network:

```yaml
# In MINOOTS docker-compose.yml
networks:
  cra-minoots:
    external: true
    name: cra-minoots-network
```

## Benefits

1. **Unified Governance** - Timer actions follow same policies as real-time agent actions
2. **Complete Audit Trail** - Every timer execution recorded in TRACE
3. **Rate Limiting** - Prevent runaway timers from overwhelming systems
4. **Dangerous Action Blocking** - Block shell injection, external webhooks
5. **Temporal Policies** - Restrict actions to business hours
