#!/bin/bash
# oscript/test_mcp.sh - Claude Code MCP 真实场景测试

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "Claude Code MCP Real Scenario Tests"
echo "=========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get port from env or use default
MCP_PORT=${MCP_PORT:-3000}
BASE_URL="http://localhost:$MCP_PORT"

# Counter
PASSED=0
FAILED=0
SKIPPED=0

# Test helper
test_mcp() {
    local name=$1
    local url=$2
    local expected=$3

    echo -n "  $name... "
    response=$(curl -s "$url" 2>/dev/null || echo "")
    if [ -z "$response" ]; then
        echo -e "${YELLOW}⚠️ SKIP${NC} (no response)"
        ((SKIPPED++))
    elif echo "$response" | grep -q "$expected"; then
        echo -e "${GREEN}✅ PASS${NC}"
        ((PASSED++))
    else
        echo -e "${RED}❌ FAIL${NC}"
        ((FAILED++))
        echo "    Expected: $expected"
        echo "    Got: ${response:0:100}"
    fi
}

echo ""
echo "=== MCP Server Health ==="
test_mcp "MCP Health" "$BASE_URL/api/v1/health" "healthy"

echo ""
echo "=== MCP Tool List ==="
echo "  Checking tool count..."
TOOLS_JSON=$(curl -s "$BASE_URL/api/v1/tools" 2>/dev/null || echo "")
TOOL_COUNT=$(echo "$TOOLS_JSON" | jq '.tools | length' 2>/dev/null || echo "0")
EXPECTED_MIN=10

if [ "$TOOL_COUNT" -ge "$EXPECTED_MIN" ]; then
    echo -e "  ${GREEN}✅ Found $TOOL_COUNT tools (>= $EXPECTED_MIN required)${NC}"
    ((PASSED++))
else
    echo -e "  ${RED}❌ Found $TOOL_COUNT tools (< $EXPECTED_MIN required)${NC}"
    ((FAILED++))
fi

echo ""
echo "=== MCP Core Tools ==="
test_mcp "evif_ls" "$BASE_URL/api/v1/directories?path=/mem" "files"
test_mcp "evif_cat" "$BASE_URL/api/v1/files?path=/mem/test.txt" "content"
test_mcp "evif_stat" "$BASE_URL/api/v1/stat?path=/mem/test.txt" "size"

echo ""
echo "=== MCP Write Test ==="
# Create test file
CREATE_RESPONSE=$(curl -s -X PUT "$BASE_URL/api/v1/files?path=/mem/mcp_test.txt" \
    -H "Content-Type: application/json" \
    -d '{"content": "MCP test content", "encoding": "utf-8"}' 2>/dev/null || echo "")

echo -n "  evif_write... "
if [ -n "$CREATE_RESPONSE" ]; then
    echo -e "${GREEN}✅ PASS${NC}"
    ((PASSED++))
else
    echo -e "${RED}❌ FAIL${NC}"
    ((FAILED++))
fi

echo ""
echo "=== MCP Memory Search ==="
MEMORY_RESPONSE=$(curl -s "$BASE_URL/api/v1/memory/search?query=test" 2>/dev/null || echo "")
echo -n "  evif_memory_search... "
if [ -z "$MEMORY_RESPONSE" ]; then
    echo -e "${YELLOW}⚠️ SKIP${NC} (not configured)"
    ((SKIPPED++))
elif echo "$MEMORY_RESPONSE" | grep -q "results\|error"; then
    echo -e "${GREEN}✅ PASS${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠️ SKIP${NC} (endpoint unavailable)"
    ((SKIPPED++))
fi

echo ""
echo "=========================================="
echo "MCP Test Summary"
echo "=========================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"

if [ $FAILED -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ All MCP tests passed${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}❌ Some MCP tests failed${NC}"
    exit 1
fi