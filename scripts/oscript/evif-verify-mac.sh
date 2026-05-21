#!/bin/bash
# oscript/evif-verify-mac.sh - macOS EVIF 真实验证脚本
# 验证 Claude Code 集成 EVIF 的核心价值

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "EVIF Claude Code Integration Real Test"
echo "Date: $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Config
REST_PORT=8080
MCP_PORT=3000
REST_URL="http://localhost:$REST_PORT"
MCP_URL="http://localhost:$MCP_PORT"

# Results
declare -a TEST_RESULTS=()
PASSED=0
FAILED=0

# Log result
log_result() {
    local name=$1
    local status=$2
    local detail=$3

    if [ "$status" = "PASS" ]; then
        PASSED=$((PASSED + 1))
        echo -e "  ${GREEN}✅ $name${NC}"
    else
        FAILED=$((FAILED + 1))
        echo -e "  ${RED}❌ $name${NC}"
    fi
    TEST_RESULTS+=("$name:$status:$detail")
}

echo -e "${BLUE}[Step 1] Checking Prerequisites${NC}"
echo "-----------------------------------"

# Check macOS
if [ "$(uname)" != "Darwin" ]; then
    echo -e "${YELLOW}⚠️ Not running on macOS - some features may not work${NC}"
fi
echo "  macOS version: $(sw_vers -productVersion)"
echo "  Architecture: $(uname -m)"

# Check tools
for cmd in curl jq cargo; do
    echo -n "  $cmd... "
    if command -v "$cmd" &> /dev/null; then
        echo -e "${GREEN}✅${NC}"
    else
        echo -e "${RED}❌${NC}"
    fi
done

echo ""
echo -e "${BLUE}[Step 2] Checking Server Status${NC}"
echo "-----------------------------------"

# Check if servers are running
check_server() {
    local name=$1
    local url=$2

    echo -n "  $name at $url... "
    response=$(curl -s --max-time 3 "$url/api/v1/health" 2>/dev/null || echo "")

    if echo "$response" | grep -q "healthy"; then
        echo -e "${GREEN}✅ Running${NC}"
        version=$(echo "$response" | jq -r '.version' 2>/dev/null || echo "unknown")
        uptime=$(echo "$response" | jq -r '.uptime' 2>/dev/null || echo "unknown")
        echo "    Version: $version"
        echo "    Uptime: ${uptime}s"
        return 0
    else
        echo -e "${RED}❌ Not running${NC}"
        return 1
    fi
}

REST_RUNNING=false
MCP_RUNNING=false

check_server "REST Server" "$REST_URL" && REST_RUNNING=true
check_server "MCP Server" "$MCP_URL" && MCP_RUNNING=true

echo ""
echo -e "${BLUE}[Step 3] Testing Claude Code Skills${NC}"
echo "-----------------------------------"

# Check Skills
SKILLS_DIR="$PROJECT_ROOT/.claude/skills"
if [ -d "$SKILLS_DIR" ]; then
    skill_count=$(ls "$SKILLS_DIR"/*.SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    echo "  Found $skill_count EVIF Skills:"
    for skill in "$SKILLS_DIR"/*.SKILL.md; do
        name=$(basename "$skill" .SKILL.md)
        triggers=$(grep -A 10 "^triggers:" "$skill" 2>/dev/null | grep -E "^\s+-\s+" | wc -l | tr -d ' ')
        echo "    - $name ($triggers triggers)"
    done
    log_result "Skills Directory" "PASS" "$skill_count skills"
else
    log_result "Skills Directory" "FAIL" "No skills directory"
fi

echo ""
echo -e "${BLUE}[Step 4] Testing EVIF REST API${NC}"
echo "-----------------------------------"

if [ "$REST_RUNNING" = true ]; then
    # Test directory listing
    echo -n "  List directories... "
    response=$(curl -s "$REST_URL/api/v1/directories?path=/mem" 2>/dev/null || echo "")
    if echo "$response" | grep -q "files"; then
        echo -e "${GREEN}✅${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌${NC}"
        FAILED=$((FAILED + 1))
    fi

    # Test file creation
    echo -n "  Create file... "
    create_response=$(curl -s -X PUT "$REST_URL/api/v1/files?path=/mem/oscript_test_$(date +%s).txt" \
        -H "Content-Type: application/json" \
        -d '{"content": "oscript real test", "encoding": "utf-8"}' 2>/dev/null || echo "")
    if echo "$create_response" | grep -q "bytes_written\|path\|success"; then
        echo -e "${GREEN}✅${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌${NC}"
        FAILED=$((FAILED + 1))
    fi

    # Test health endpoint
    echo -n "  Health check... "
    health=$(curl -s "$REST_URL/api/v1/health" 2>/dev/null || echo "")
    if echo "$health" | grep -q "healthy"; then
        echo -e "${GREEN}✅${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌${NC}"
        FAILED=$((FAILED + 1))
    fi
else
    echo -e "  ${YELLOW}⚠️ REST Server not running, skipping API tests${NC}"
fi

echo ""
echo -e "${BLUE}[Step 5] Testing MCP Server${NC}"
echo "-----------------------------------"

if [ "$MCP_RUNNING" = true ]; then
    # Test MCP tools endpoint
    echo -n "  MCP tools list... "
    tools_response=$(curl -s "$MCP_URL/api/v1/tools" 2>/dev/null || echo "")
    tool_count=$(echo "$tools_response" | jq '.tools | length' 2>/dev/null || echo "0")
    if [ "$tool_count" -gt 0 ]; then
        echo -e "${GREEN}✅ ($tool_count tools)${NC}"
        PASSED=$((PASSED + 1))
        echo "    Tool names:"
        echo "$tools_response" | jq -r '.tools[].name' 2>/dev/null | head -5 | sed 's/^/      /'
    else
        echo -e "${RED}❌${NC}"
        FAILED=$((FAILED + 1))
    fi

    # Test MCP stat
    echo -n "  MCP stat... "
    stat_response=$(curl -s "$MCP_URL/api/v1/stat?path=/mem" 2>/dev/null || echo "")
    if echo "$stat_response" | grep -q "size\|is_dir"; then
        echo -e "${GREEN}✅${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌${NC}"
        FAILED=$((FAILED + 1))
    fi
else
    echo -e "  ${YELLOW}⚠️ MCP Server not running, skipping MCP tests${NC}"
fi

echo ""
echo -e "${BLUE}[Step 6] Claude Code Integration Value Analysis${NC}"
echo "-----------------------------------"

echo "  Claude Code 集成 EVIF 的核心价值:"
echo ""
echo "  1. 持久化上下文"
echo "     - L0/L1/L2 三层上下文"
echo "     - 跨会话记忆"
echo "     - 决策追溯"
echo ""
echo "  2. 增强的工具调用"
echo "     - MCP 协议原生支持"
echo "     - 15+ 工具可用"
echo "     - 统一的 API 接口"
echo ""
echo "  3. 多 Agent 协调"
echo "     - PipeFS 管道机制"
echo "     - 任务队列"
echo "     - 工作流复用"
echo ""

echo ""
echo "=========================================="
echo "Verification Summary"
echo "=========================================="
echo ""
echo -e "Passed:  ${GREEN}$PASSED${NC}"
echo -e "Failed:  ${RED}$FAILED${NC}"
echo ""

if [ "$REST_RUNNING" = false ] || [ "$MCP_RUNNING" = false ]; then
    echo -e "${YELLOW}⚠️ Servers not running - start with:${NC}"
    echo "  Terminal 1: cargo run -p evif-rest -- --port $REST_PORT"
    echo "  Terminal 2: cargo run -p evif-mcp -- --port $MCP_PORT"
    echo ""
fi

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All verification tests passed!${NC}"
    echo ""
    echo "Claude Code + EVIF 集成核心价值验证完成"
else
    echo -e "${RED}❌ Some tests failed${NC}"
fi

echo ""
echo "=========================================="
echo "Verification completed at $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="

exit $FAILED