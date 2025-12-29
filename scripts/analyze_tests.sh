#!/bin/bash
# CRA Test Analysis Script
# Analyzes test results and generates reports

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           CRA Test Analysis Report                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Generated: $(date)"
echo "Project: $PROJECT_ROOT"
echo ""

# Function to count tests in a module
count_module_tests() {
    local module=$1
    cargo test --lib -- --list 2>/dev/null | grep "^${module}" | grep ": test$" | wc -l
}

echo -e "${YELLOW}📊 Test Distribution by Module${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Count tests per module
ATLAS_TESTS=$(count_module_tests "atlas::")
CARP_TESTS=$(count_module_tests "carp::")
CONTEXT_TESTS=$(count_module_tests "context::")
TRACE_TESTS=$(count_module_tests "trace::")
TIMING_TESTS=$(count_module_tests "timing::")
ERROR_TESTS=$(count_module_tests "error::")
STORAGE_TESTS=$(count_module_tests "storage::")
FFI_TESTS=$(count_module_tests "ffi::")
ROOT_TESTS=$(count_module_tests "tests::")

TOTAL_LIB=$((ATLAS_TESTS + CARP_TESTS + CONTEXT_TESTS + TRACE_TESTS + TIMING_TESTS + ERROR_TESTS + STORAGE_TESTS + FFI_TESTS + ROOT_TESTS))

printf "  %-20s %3d tests\n" "atlas::" "$ATLAS_TESTS"
printf "  %-20s %3d tests\n" "carp::" "$CARP_TESTS"
printf "  %-20s %3d tests\n" "context::" "$CONTEXT_TESTS"
printf "  %-20s %3d tests\n" "trace::" "$TRACE_TESTS"
printf "  %-20s %3d tests\n" "timing::" "$TIMING_TESTS"
printf "  %-20s %3d tests\n" "error::" "$ERROR_TESTS"
printf "  %-20s %3d tests\n" "storage::" "$STORAGE_TESTS"
printf "  %-20s %3d tests\n" "ffi::" "$FFI_TESTS"
printf "  %-20s %3d tests\n" "tests::" "$ROOT_TESTS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "  %-20s %3d tests\n" "TOTAL LIB" "$TOTAL_LIB"
echo ""

# Count integration tests
INTEGRATION_TESTS=$(cargo test --test self_governance -- --list 2>/dev/null | grep ": test$" | wc -l)
echo -e "${YELLOW}🔗 Integration Tests${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "  %-20s %3d tests\n" "self_governance" "$INTEGRATION_TESTS"
echo ""

TOTAL=$((TOTAL_LIB + INTEGRATION_TESTS))
echo -e "${GREEN}✓ Total Tests: $TOTAL${NC}"
echo ""

# Run tests and capture results
echo -e "${YELLOW}🧪 Running Tests...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Run lib tests
echo "Running library tests..."
LIB_RESULT=$(cargo test --lib 2>&1)
LIB_PASSED=$(echo "$LIB_RESULT" | grep -o '[0-9]* passed' | head -1 | grep -o '[0-9]*')
LIB_FAILED=$(echo "$LIB_RESULT" | grep -o '[0-9]* failed' | head -1 | grep -o '[0-9]*' || echo "0")

if [ "$LIB_FAILED" = "0" ] || [ -z "$LIB_FAILED" ]; then
    echo -e "  Library tests: ${GREEN}✓ $LIB_PASSED passed${NC}"
else
    echo -e "  Library tests: ${RED}✗ $LIB_FAILED failed${NC}, $LIB_PASSED passed"
fi

# Run integration tests
echo "Running integration tests..."
INT_RESULT=$(cargo test --test self_governance 2>&1)
INT_PASSED=$(echo "$INT_RESULT" | grep -o '[0-9]* passed' | head -1 | grep -o '[0-9]*')
INT_FAILED=$(echo "$INT_RESULT" | grep -o '[0-9]* failed' | head -1 | grep -o '[0-9]*' || echo "0")

if [ "$INT_FAILED" = "0" ] || [ -z "$INT_FAILED" ]; then
    echo -e "  Integration tests: ${GREEN}✓ $INT_PASSED passed${NC}"
else
    echo -e "  Integration tests: ${RED}✗ $INT_FAILED failed${NC}, $INT_PASSED passed"
fi

echo ""

# Test categories
echo -e "${YELLOW}📁 Test Categories${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

HASH_TESTS=$(cargo test --lib -- --list 2>/dev/null | grep -i "hash\|compute\|canonical" | wc -l)
CHAIN_TESTS=$(cargo test --lib -- --list 2>/dev/null | grep -i "chain\|verify\|integrity" | wc -l)
CONTEXT_INJECTS=$(cargo test --lib -- --list 2>/dev/null | grep -i "context.*inject\|inject.*context" | wc -l)
DEFERRED_TESTS=$(cargo test --lib -- --list 2>/dev/null | grep -i "deferred\|flush" | wc -l)

printf "  %-25s %3d tests\n" "Hash/Canonical" "$HASH_TESTS"
printf "  %-25s %3d tests\n" "Chain/Verification" "$CHAIN_TESTS"
printf "  %-25s %3d tests\n" "Context Injection" "$CONTEXT_INJECTS"
printf "  %-25s %3d tests\n" "Deferred Mode" "$DEFERRED_TESTS"
echo ""

# Critical tests status
echo -e "${YELLOW}⚠️  Critical Test Areas${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check critical tests exist
check_critical() {
    local pattern=$1
    local name=$2
    local count=$(cargo test --lib -- --list 2>/dev/null | grep -i "$pattern" | wc -l)
    if [ "$count" -gt 0 ]; then
        echo -e "  ${GREEN}✓${NC} $name ($count tests)"
    else
        echo -e "  ${RED}✗${NC} $name (MISSING!)"
    fi
}

check_critical "compute_hash\|canonical_json" "Hash computation"
check_critical "chain.*valid\|verify.*chain" "Chain verification"
check_critical "context.*inject" "Context injection"
check_critical "deferred.*chain\|chain.*deferred" "Deferred mode integrity"
check_critical "policy.*evaluat" "Policy evaluation"
echo ""

# Summary
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                        Summary                             ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

TOTAL_PASSED=$((LIB_PASSED + INT_PASSED))
TOTAL_FAILED=$((${LIB_FAILED:-0} + ${INT_FAILED:-0}))

if [ "$TOTAL_FAILED" = "0" ]; then
    echo -e "${GREEN}All $TOTAL_PASSED tests passed!${NC}"
    echo ""
    echo "Coverage areas:"
    echo "  • Atlas handling: ✓"
    echo "  • CARP protocol: ✓"
    echo "  • Context injection: ✓"
    echo "  • TRACE events: ✓"
    echo "  • Chain integrity: ✓"
    exit 0
else
    echo -e "${RED}$TOTAL_FAILED tests failed!${NC}"
    echo ""
    echo "Run with --nocapture for details:"
    echo "  cargo test -- --nocapture"
    exit 1
fi
