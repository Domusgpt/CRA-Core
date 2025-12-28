#!/bin/bash
# CRA + MINOOTS Integration Demo
#
# This script demonstrates CRA governance over MINOOTS timer actions.
# Run from this directory: ./demo.sh

set -e

CRA_URL="${CRA_URL:-http://localhost:8420}"

echo "=========================================="
echo "  CRA + MINOOTS Integration Demo"
echo "=========================================="
echo ""
echo "CRA Server: $CRA_URL"
echo ""

# Wait for CRA to be ready
echo "[1/6] Waiting for CRA server..."
until curl -s "$CRA_URL/health" > /dev/null 2>&1; do
    sleep 1
done
echo "      CRA server is ready!"
echo ""

# Create a session (simulating MINOOTS action orchestrator)
echo "[2/6] Creating CRA session for MINOOTS orchestrator..."
SESSION_RESPONSE=$(curl -s -X POST "$CRA_URL/v1/sessions" \
    -H "Content-Type: application/json" \
    -d '{
        "agent_id": "minoots-action-orchestrator",
        "goal": "Execute timer-triggered actions"
    }')
SESSION_ID=$(echo "$SESSION_RESPONSE" | jq -r '.session_id')
echo "      Session ID: $SESSION_ID"
echo ""

# Test 1: Allowed webhook (localhost)
echo "[3/6] Test: Internal webhook (should be ALLOWED)..."
RESOLVE_RESPONSE=$(curl -s -X POST "$CRA_URL/v1/resolve" \
    -H "Content-Type: application/json" \
    -d "{
        \"session_id\": \"$SESSION_ID\",
        \"agent_id\": \"minoots-action-orchestrator\",
        \"goal\": \"Execute webhook to localhost\"
    }")
DECISION=$(echo "$RESOLVE_RESPONSE" | jq -r '.decision')
echo "      Decision: $DECISION"
echo "      Allowed actions: $(echo "$RESOLVE_RESPONSE" | jq -r '.allowed_actions | length')"
echo ""

# Test 2: Check for denied actions
echo "[4/6] Test: Checking denied actions..."
DENIED_COUNT=$(echo "$RESOLVE_RESPONSE" | jq -r '.denied_actions | length')
if [ "$DENIED_COUNT" -gt 0 ]; then
    echo "      Denied actions:"
    echo "$RESOLVE_RESPONSE" | jq -r '.denied_actions[] | "        - \(.action_id): \(.reason)"'
else
    echo "      No actions denied"
fi
echo ""

# Test 3: Get trace events
echo "[5/6] Retrieving TRACE audit log..."
TRACE_RESPONSE=$(curl -s "$CRA_URL/v1/traces/$SESSION_ID")
EVENT_COUNT=$(echo "$TRACE_RESPONSE" | jq '. | length')
echo "      Events recorded: $EVENT_COUNT"
echo ""

# Test 4: Verify hash chain integrity
echo "[6/6] Verifying TRACE hash chain..."
VERIFY_RESPONSE=$(curl -s "$CRA_URL/v1/traces/$SESSION_ID/verify")
IS_VALID=$(echo "$VERIFY_RESPONSE" | jq -r '.is_valid')
echo "      Chain valid: $IS_VALID"
echo "      Event count: $(echo "$VERIFY_RESPONSE" | jq -r '.event_count')"
echo ""

echo "=========================================="
echo "  Demo Complete!"
echo "=========================================="
echo ""
echo "Summary:"
echo "  - CRA session created for MINOOTS orchestrator"
echo "  - Policy check performed before action execution"
echo "  - All events recorded in tamper-evident TRACE log"
echo "  - Hash chain verified for audit integrity"
echo ""
echo "In production, MINOOTS Action Orchestrator would:"
echo "  1. Call /v1/resolve before each timer action"
echo "  2. Execute only if decision is 'allow'"
echo "  3. Log denials and skip blocked actions"
echo ""
