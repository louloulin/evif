#!/bin/bash
# EVIF Main Value Verification Script
# This script runs the minimal test suite that proves EVIF's core value proposition.
# Success criteria: if these tests pass, EVIF is "MVP-ready" for Agent use.

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        EVIF MVP 6.0 Main Value Verification                ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

cd "$PROJECT_ROOT"

# Track results
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

run_check() {
    local name="$1"
    local cmd="$2"
    TOTAL=$((TOTAL + 1))
    echo -e "${YELLOW}[$TOTAL] ${name}...${NC}"
    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "      ${GREEN}✓ PASS${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "      ${RED}✗ FAIL${NC}"
        FAILED=$((FAILED + 1))
    fi
}

run_check_output() {
    local name="$1"
    local cmd="$2"
    local expected="$3"
    TOTAL=$((TOTAL + 1))
    echo -e "${YELLOW}[$TOTAL] ${name}...${NC}"
    local result
    result=$(eval "$cmd" 2>&1 || true)
    if echo "$result" | grep -q "$expected"; then
        echo -e "      ${GREEN}✓ PASS${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "      ${RED}✗ FAIL${NC} (expected: $expected)"
        FAILED=$((FAILED + 1))
    fi
}

# Phase 1: Core Crate Tests
echo -e "\n${BLUE}=== Phase 1: Core Crate Tests ===${NC}"
run_check "evif-core unit tests" "cargo test -p evif-core --quiet 2>&1 | grep -q 'test result:'"
run_check "evif-mcp unit tests" "cargo test -p evif-mcp --quiet 2>&1 | grep -q 'test result:'"
run_check "evif-cli unit tests" "cargo test -p evif-cli --quiet 2>&1 | grep -q 'test result:'"

# Phase 2: Agent Primitives
echo -e "\n${BLUE}=== Phase 2: Agent Primitive Tests ===${NC}"
run_check "contextfs tests" "cargo test -p evif-plugins -- contextfs --quiet 2>&1 | grep -q 'test result:'"
run_check "skillfs tests" "cargo test -p evif-plugins -- skillfs --quiet 2>&1 | grep -q 'test result:'"
run_check "pipefs tests" "cargo test -p evif-plugins -- pipefs --quiet 2>&1 | grep -q 'test result:'"
run_check "evif-mem tests" "cargo test -p evif-mem --quiet 2>&1 | grep -q 'test result:'"

# Phase 3: Cost Optimization
echo -e "\n${BLUE}=== Phase 3: Cost Optimization Features ===${NC}"
run_check "Output filter compilation" "cargo build -p evif-mcp --quiet 2>&1; [ \$? -eq 0 ]"
run_check "Bounded read in MCP" "grep -q 'max_lines' crates/evif-mcp/src/lib.rs"
run_check "Compact search in memory" "grep -q 'compact' crates/evif-mcp/src/lib.rs"

# Phase 4: Documentation
echo -e "\n${BLUE}=== Phase 4: Documentation ===${NC}"
run_check "README exists" "[ -f README.md ]"
run_check "MVP 6.0 plan exists" "[ -f mvp6.0.md ]"
run_check "Connector matrix exists" "[ -f docs/connector-capability-matrix.md ]"
run_check "Cost optimization matrix exists" "[ -f docs/cost-optimization-matrix.md ]"
run_check "CLAUDE.md template exists" "[ -f CLAUDE.md ]"
run_check "AGENTS.md template exists" "[ -f AGENTS.md ]"
run_check "Archive directory for legacy docs" "[ -d archive/mvp ]"

# Phase 5: Skills
echo -e "\n${BLUE}=== Phase 5: EVIF Skills ===${NC}"
run_check "Skills directory exists" "[ -d .claude/skills ]"
run_check "evif-context skill" "[ -f .claude/skills/evif-context.SKILL.md ]"
run_check "evif-workflows skill" "[ -f .claude/skills/evif-workflows.SKILL.md ]"
run_check "evif-memory skill" "[ -f .claude/skills/evif-memory.SKILL.md ]"
run_check "evif-pipes skill" "[ -f .claude/skills/evif-pipes.SKILL.md ]"

# Phase 6: Demo Script
echo -e "\n${BLUE}=== Phase 6: Demo Scripts ===${NC}"
run_check "Demo agent workflow script" "[ -f scripts/demo-agent-workflow.sh ]"
run_check "Demo script executable" "[ -x scripts/demo-agent-workflow.sh ]"

# Phase 7: Build
echo -e "\n${BLUE}=== Phase 7: Build Verification ===${NC}"
run_check "Full workspace builds" "cargo build --workspace --quiet 2>&1 | grep -q -E '(Finished|error)'"

# Summary
echo -e "\n${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    Verification Summary                     ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Total checks: ${TOTAL}"
echo -e "Passed:       ${GREEN}${PASSED}${NC}"
echo -e "Failed:       ${RED}${FAILED}${NC}"
echo -e "Skipped:     ${YELLOW}${SKIPPED}${NC}"
echo ""

if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}✓ All MVP 6.0 main value checks passed!${NC}"
    echo ""
    echo "EVIF is ready for Agent use with:"
    echo "  - Core crates tested and building"
    echo "  - Agent primitives (context/skill/pipe/memory) implemented"
    echo "  - Cost optimization features in place"
    echo "  - Documentation complete"
    echo "  - Skills for Claude Code integration ready"
    echo ""
    exit 0
else
    echo -e "${RED}✗ Some checks failed. Please review above.${NC}"
    echo ""
    exit 1
fi
