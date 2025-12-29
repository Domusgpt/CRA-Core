#!/bin/bash
# CRA + MINOOTS Integration Demo
#
# This script demonstrates CRA's three-layer governance model:
#   Layer 1: Context Injection (governance rules in LLM context)
#   Layer 2: MCP Server (tool filtering, pre-flight checks)
#   Layer 3: Webhook Proxy (hard network-level enforcement)
#
# Run from this directory: ./demo.sh

set -e

CRA_URL="${CRA_URL:-http://localhost:8420}"
CRA_MCP_URL="${CRA_MCP_URL:-http://localhost:8421}"
CRA_PROXY_URL="${CRA_PROXY_URL:-http://localhost:8422}"
WEBHOOK_URL="${WEBHOOK_URL:-http://webhook-receiver:3000}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║         CRA + MINOOTS Three-Layer Governance Demo                    ║"
echo "╠══════════════════════════════════════════════════════════════════════╣"
echo "║  Layer 1: Context Injection (soft governance)                        ║"
echo "║  Layer 2: MCP Server (tool filtering)                                ║"
echo "║  Layer 3: Webhook Proxy (hard enforcement)                           ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""
echo "Endpoints:"
echo "  CRA Server: $CRA_URL"
echo "  CRA MCP:    $CRA_MCP_URL"
echo "  CRA Proxy:  $CRA_PROXY_URL"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Wait for services
# ─────────────────────────────────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Checking services..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo -n "  [1/3] CRA Server... "
until curl -s "$CRA_URL/health" > /dev/null 2>&1; do
    sleep 1
done
echo -e "${GREEN}ready${NC}"

echo -n "  [2/3] CRA Proxy... "
until curl -s "$CRA_PROXY_URL/health" > /dev/null 2>&1; do
    sleep 1
done
echo -e "${GREEN}ready${NC}"

echo -n "  [3/3] Webhook Receiver... "
until curl -s "http://localhost:3000" > /dev/null 2>&1; do
    sleep 1
done
echo -e "${GREEN}ready${NC}"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# LAYER 1: Context Injection Demo
# ─────────────────────────────────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  ${BLUE}LAYER 1: CONTEXT INJECTION${NC}"
echo "  Governance rules embedded in Atlas context_blocks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "  Context blocks in governance-context.json:"
echo "  ─────────────────────────────────────────────"
if [ -f "atlases/governance-context.json" ]; then
    jq -r '.context_blocks[] | "  Block: \(.block_id)\n  Content: \(.content | split("\n")[0])...\n"' atlases/governance-context.json 2>/dev/null || echo "  (Could not parse context blocks)"
else
    echo "  (Atlas file not found)"
fi
echo ""
echo -e "  ${YELLOW}How it works:${NC} When LLM session starts, these rules are"
echo "  injected into the context window. LLM reads and follows them."
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# LAYER 2: MCP Server Demo
# ─────────────────────────────────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  ${BLUE}LAYER 2: MCP SERVER${NC}"
echo "  Pre-flight policy checks via cra_check tool"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Create session
echo "  Creating CRA session..."
SESSION_RESPONSE=$(curl -s -X POST "$CRA_URL/v1/sessions" \
    -H "Content-Type: application/json" \
    -d '{
        "agent_id": "minoots-action-orchestrator",
        "goal": "Execute timer-triggered actions"
    }')
SESSION_ID=$(echo "$SESSION_RESPONSE" | jq -r '.session_id')
echo "  Session: $SESSION_ID"
echo ""

# Test allowed action
echo "  [Test 1] Checking webhook execution (should be allowed)..."
RESOLVE_RESPONSE=$(curl -s -X POST "$CRA_URL/v1/resolve" \
    -H "Content-Type: application/json" \
    -d "{
        \"session_id\": \"$SESSION_ID\",
        \"agent_id\": \"minoots-action-orchestrator\",
        \"goal\": \"minoots.webhook.execute\"
    }")
DECISION=$(echo "$RESOLVE_RESPONSE" | jq -r '.decision')
if [ "$DECISION" = "allow" ]; then
    echo -e "  Result: ${GREEN}ALLOWED${NC} - Webhook execution permitted"
else
    echo -e "  Result: ${RED}DENIED${NC} - $DECISION"
fi
echo ""

# Test denied action (dangerous CLI)
echo "  [Test 2] Checking dangerous CLI (should be denied)..."
RESOLVE_RESPONSE=$(curl -s -X POST "$CRA_URL/v1/resolve" \
    -H "Content-Type: application/json" \
    -d "{
        \"session_id\": \"$SESSION_ID\",
        \"agent_id\": \"minoots-action-orchestrator\",
        \"goal\": \"minoots.cli.execute\"
    }")
DECISION=$(echo "$RESOLVE_RESPONSE" | jq -r '.decision')
DENIED_ACTIONS=$(echo "$RESOLVE_RESPONSE" | jq -r '.denied_actions | length')
if [ "$DENIED_ACTIONS" -gt 0 ] || [ "$DECISION" = "deny" ]; then
    echo -e "  Result: ${GREEN}CORRECTLY BLOCKED${NC} - High-risk CLI denied"
else
    echo -e "  Result: ${YELLOW}ALLOWED${NC} (may need policy update)"
fi
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# LAYER 3: Webhook Proxy Demo
# ─────────────────────────────────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  ${BLUE}LAYER 3: WEBHOOK PROXY${NC}"
echo "  Network-level enforcement - LLM cannot bypass"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Test allowed external webhook
echo "  [Test 3] Safe external webhook (should pass)..."
PROXY_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CRA_PROXY_URL/proxy" \
    -H "Content-Type: application/json" \
    -H "X-Target-URL: $WEBHOOK_URL/timer-event" \
    -H "X-Timer-ID: timer-demo-123" \
    -d '{"event": "timer.fired", "payload": {"message": "Hello from MINOOTS"}}')
HTTP_CODE=$(echo "$PROXY_RESPONSE" | tail -1)
if [ "$HTTP_CODE" = "200" ]; then
    echo -e "  Result: ${GREEN}FORWARDED${NC} (HTTP $HTTP_CODE)"
else
    echo -e "  Result: ${YELLOW}Status $HTTP_CODE${NC}"
fi
echo ""

# Test blocked internal network (SSRF protection)
echo "  [Test 4] Internal network request (SSRF - should be BLOCKED)..."
PROXY_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CRA_PROXY_URL/proxy" \
    -H "Content-Type: application/json" \
    -H "X-Target-URL: http://192.168.1.1/admin" \
    -d '{"event": "malicious"}')
HTTP_CODE=$(echo "$PROXY_RESPONSE" | tail -1)
if [ "$HTTP_CODE" = "403" ]; then
    echo -e "  Result: ${GREEN}CORRECTLY BLOCKED${NC} (HTTP 403 - Internal network blocked)"
else
    echo -e "  Result: ${RED}UNEXPECTED${NC} (HTTP $HTTP_CODE)"
fi
echo ""

# Test blocked cloud metadata (AWS SSRF attack)
echo "  [Test 5] Cloud metadata endpoint (AWS SSRF - should be BLOCKED)..."
PROXY_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CRA_PROXY_URL/proxy" \
    -H "Content-Type: application/json" \
    -H "X-Target-URL: http://169.254.169.254/latest/meta-data" \
    -d '{}')
HTTP_CODE=$(echo "$PROXY_RESPONSE" | tail -1)
if [ "$HTTP_CODE" = "403" ]; then
    echo -e "  Result: ${GREEN}CORRECTLY BLOCKED${NC} (HTTP 403 - Cloud metadata blocked)"
else
    echo -e "  Result: ${RED}UNEXPECTED${NC} (HTTP $HTTP_CODE)"
fi
echo ""

# Test blocked dangerous command in body
echo "  [Test 6] Dangerous command in body (should be BLOCKED)..."
PROXY_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CRA_PROXY_URL/proxy" \
    -H "Content-Type: application/json" \
    -H "X-Target-URL: $WEBHOOK_URL/execute" \
    -d '{"command": "sudo rm -rf /"}')
HTTP_CODE=$(echo "$PROXY_RESPONSE" | tail -1)
if [ "$HTTP_CODE" = "403" ]; then
    echo -e "  Result: ${GREEN}CORRECTLY BLOCKED${NC} (HTTP 403 - Dangerous command blocked)"
else
    echo -e "  Result: ${RED}UNEXPECTED${NC} (HTTP $HTTP_CODE)"
fi
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# TRACE Audit Log
# ─────────────────────────────────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  ${BLUE}TRACE AUDIT LOG${NC}"
echo "  Immutable, hash-chained event log"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TRACE_RESPONSE=$(curl -s "$CRA_URL/v1/traces/$SESSION_ID")
EVENT_COUNT=$(echo "$TRACE_RESPONSE" | jq '. | length')
echo "  Events recorded: $EVENT_COUNT"

VERIFY_RESPONSE=$(curl -s "$CRA_URL/v1/traces/$SESSION_ID/verify")
IS_VALID=$(echo "$VERIFY_RESPONSE" | jq -r '.is_valid')
if [ "$IS_VALID" = "true" ]; then
    echo -e "  Hash chain:     ${GREEN}VERIFIED${NC}"
else
    echo -e "  Hash chain:     ${YELLOW}$IS_VALID${NC}"
fi
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Summary
# ─────────────────────────────────────────────────────────────────────────────
echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                           Demo Complete!                             ║"
echo "╠══════════════════════════════════════════════════════════════════════╣"
echo "║                                                                      ║"
echo "║  Layer 1 (Context):  Governance rules auto-injected to LLM          ║"
echo "║  Layer 2 (MCP):      Pre-flight checks via cra_check tool           ║"
echo "║  Layer 3 (Proxy):    Hard blocks on internal IPs, dangerous cmds    ║"
echo "║  TRACE:              All events logged with hash-chain integrity    ║"
echo "║                                                                      ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Connect an MCP client to localhost:8421"
echo "  2. Route MINOOTS webhooks through localhost:8422/proxy"
echo "  3. Load custom atlases in ./atlases/"
echo ""
