# CRA + MINOOTS Integration

This example demonstrates how CRA (Context Registry Agents) provides **three layers of governance** for MINOOTS timer actions.

## The Three-Layer Enforcement Model

```
┌──────────────────────────────────────────────────────────────────────────┐
│                          CRA Enforcement Layers                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │ Layer 1: CONTEXT INJECTION (Soft Governance)                       │  │
│  │                                                                    │  │
│  │ • Atlas context_blocks injected into LLM context window            │  │
│  │ • LLM reads rules and self-governs                                 │  │
│  │ • Works everywhere - no infrastructure changes needed              │  │
│  │ • Example: governance-context.json with embedded rules             │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                    ↓                                      │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │ Layer 2: MCP SERVER (Tool Filtering)                               │  │
│  │                                                                    │  │
│  │ • Self-documenting MCP server via cra-mcp                          │  │
│  │ • Controls which tools the LLM can see                             │  │
│  │ • Hidden tools = can't be called                                   │  │
│  │ • Provides cra_check and cra_help tools for governance awareness   │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                    ↓                                      │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │ Layer 3: WEBHOOK PROXY (Hard Enforcement)                          │  │
│  │                                                                    │  │
│  │ • HTTP proxy gateway via cra-proxy                                 │  │
│  │ • All outbound requests flow through the proxy                     │  │
│  │ • Network-level blocking - unstoppable by LLM                      │  │
│  │ • Blocks: internal IPs, dangerous commands, SSRF attempts          │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## When to Use Each Layer

| Layer | Best For | LLM Can Bypass? |
|-------|----------|-----------------|
| Context Injection | Guidelines, preferences, soft limits | Yes (if it ignores context) |
| MCP Server | Hiding actions, pre-flight checks | No (tool not visible) |
| Webhook Proxy | Absolute blocks, security boundaries | No (network level) |

**Recommended**: Use all three together for defense in depth.

## Quick Start

```bash
# Start full stack with all three layers
docker-compose up -d

# Layer 1 (Context): Check governance context
cat atlases/governance-context.json | jq '.context_blocks'

# Layer 2 (MCP): Connect to self-documenting MCP server
# Port 8421 - Claude/other LLM connects here

# Layer 3 (Proxy): All webhooks route through proxy
# Port 8422 - Hard enforcement gateway
```

## Architecture with MINOOTS

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         MINOOTS + CRA Integration                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   Timer Expires                                                         │
│        ↓                                                                │
│   Action Orchestrator                                                   │
│        ↓                                                                │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │ Layer 1: Context Already Loaded                                 │   │
│   │ LLM has governance rules in context from session start          │   │
│   │ It may self-decline dangerous actions                           │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│        ↓                                                                │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │ Layer 2: MCP Pre-flight Check                                   │   │
│   │ POST /v1/resolve → CRA checks policies                          │   │
│   │ If denied: Stop here, log reason                                │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│        ↓                                                                │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │ Layer 3: Proxy Enforcement                                      │   │
│   │ Webhook routes through CRA Proxy                                │   │
│   │ Hard blocks: internal IPs, dangerous payloads, SSRF             │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│        ↓                                                                │
│   Action Executed (if all layers pass)                                  │
│        ↓                                                                │
│   TRACE Audit Log (immutable, hash-chained)                             │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Layer 1: Context Injection

The `atlases/governance-context.json` contains `context_blocks` with governance rules:

```json
{
  "context_blocks": [
    {
      "block_id": "governance-rules",
      "content_type": "text/markdown",
      "content": "# Timer Governance Rules\n\n1. Never execute commands containing rm -rf...",
      "inject_on": "session_start"
    }
  ]
}
```

When an LLM session starts, these rules are injected into the context window. The LLM reads them and (hopefully) follows them.

**Pros**: Works everywhere, no infrastructure changes, educational for the LLM
**Cons**: LLM can ignore context, prompt injection could override

## Layer 2: MCP Server

The `cra-mcp` service exposes a self-documenting MCP server:

```bash
# Connect and see available tools
$ echo '{"method":"tools/list"}' | nc localhost 8421

# Tools include:
# - cra_check: Verify if action is allowed BEFORE attempting
# - cra_help: Get governance documentation
# - cra_list_actions: See all available governed actions
```

When connecting, the MCP server injects instructions:
```
You are connected to a governance-enabled environment.
1. Check before acting: Use cra_check before any action
2. Read the rules: cra://system/rules contains policies
3. Ask for help: Use cra_help if unsure
```

**Pros**: LLM can't call hidden tools, pre-flight checks, self-documenting
**Cons**: Requires MCP-compatible client, doesn't block direct HTTP

## Layer 3: Webhook Proxy

The `cra-proxy` service is a hard enforcement gateway:

```bash
# Direct webhook (bypasses governance!)
curl https://api.example.com/webhook

# Through proxy (enforced!)
curl -X POST http://localhost:8422/proxy \
  -H "X-Target-URL: https://api.example.com/webhook" \
  -d '{"event": "timer.fired"}'
```

Blocked by default:
- Internal network URLs (10.x, 192.168.x, 172.16-31.x)
- Cloud metadata endpoints (169.254.169.254)
- Dangerous command patterns in body (rm -rf, sudo, curl|sh)

**Pros**: Network-level, LLM cannot bypass, stops SSRF attacks
**Cons**: Requires routing through proxy, slight latency

## Atlas Configuration

### Actions (`minoots-governance.json`)

| Action ID | Description | Risk Tier |
|-----------|-------------|-----------|
| `minoots.webhook.execute` | Send HTTP request | Medium |
| `minoots.webhook.retry` | Retry failed webhook | Medium |
| `minoots.cli.execute` | Run shell command | High |
| `minoots.file.write` | Write file | Medium |
| `minoots.file.append` | Append to file | Low |

### Policies

| Policy | Layer | Effect |
|--------|-------|--------|
| Context rules | 1 | LLM self-governance |
| Rate limits | 2 | Max 100 webhooks/minute |
| Deny dangerous CLI | 2+3 | Block rm -rf, sudo |
| Block internal URLs | 3 | SSRF protection |
| Block cloud metadata | 3 | Cloud security |

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_ENDPOINT` | `http://localhost:8420` | CRA server URL |
| `CRA_MCP_PORT` | `8421` | MCP server port |
| `CRA_PROXY_PORT` | `8422` | Proxy gateway port |
| `CRA_ENABLED` | `true` | Enable/disable CRA checks |
| `CRA_ON_UNAVAILABLE` | `allow` | Behavior when CRA is down |

## Running the Demo

```bash
# 1. Start all services
docker-compose up -d

# 2. Run the demo script
./demo.sh

# 3. See the layers in action:
#    - Context rules loaded ✓
#    - MCP pre-flight checks ✓
#    - Proxy enforcement ✓
```

## Integration with Your MINOOTS Instance

1. Add CRA network to your MINOOTS docker-compose:
```yaml
networks:
  cra-minoots:
    external: true
    name: cra-minoots-network
```

2. Configure Action Orchestrator:
```yaml
action-orchestrator:
  environment:
    - CRA_ENDPOINT=http://cra-server:8420
    - CRA_PROXY=http://cra-proxy:8422
    - CRA_ENABLED=true
```

3. Route webhooks through proxy:
```typescript
// Instead of direct fetch:
// await fetch(webhookUrl, { ... });

// Route through CRA proxy:
await fetch(`${CRA_PROXY}/proxy`, {
  method: 'POST',
  headers: {
    'X-Target-URL': webhookUrl,
    'X-Timer-ID': timer.id,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify(payload)
});
```

## Benefits

1. **Defense in Depth** - Three layers, each catching what others miss
2. **Self-Documenting** - LLMs learn governance by connecting
3. **Complete Audit Trail** - Every action in TRACE log
4. **Flexible Enforcement** - Soft guidance to hard blocks
5. **SSRF Protection** - Network-level security
6. **Rate Limiting** - Prevent runaway timers
