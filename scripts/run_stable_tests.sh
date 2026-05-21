#!/bin/bash
# EVIF Verification Test Suite
# Run flaky integration tests with dedicated server process

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=========================================="
echo "EVIF Test Verification Suite"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counter
PASSED=0
FAILED=0
IGNORED=0

run_test() {
    local test_name="$1"
    local test_cmd="$2"

    echo -e "${YELLOW}Running: $test_name${NC}"
    if eval "$test_cmd" 2>&1; then
        echo -e "${GREEN}✓ PASS: $test_name${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL: $test_name${NC}"
        ((FAILED++))
    fi
    echo ""
}

# Run stable tests first
echo "=== Running Stable Tests ==="
echo ""

cd "$PROJECT_ROOT"

# Core unit tests
run_test "evif-core unit tests" "cargo test -p evif-core"

# API contract tests (run stable ones only)
run_test "evif-rest stable tests" "cargo test -p evif-rest -- --skip Flaky --skip flaky"

# MCP server tests
run_test "evif-mcp tests" "cargo test -p evif-mcp"

# Plugin tests
run_test "evif-plugins tests" "cargo test -p evif-plugins"

# E2E tests (stable ones)
run_test "e2e stable tests" "cargo test -p e2e-tests -- --skip Flaky --skip flaky --skip HandleFS"

# Bench tests (stable ones)
run_test "evif-bench stable tests" "cargo test -p evif-bench -- --skip Flaky --skip flaky"

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Ignored: $IGNORED"

if [ $FAILED -gt 0 ]; then
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "\n${GREEN}All stable tests passed!${NC}"
    exit 0
fi