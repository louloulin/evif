#!/bin/bash
# scripts/verify_skills_mcp.sh - Skills + MCP 真实接入验证

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=========================================="
echo "Skills + MCP Integration Verification"
echo "=========================================="

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0
SKIPPED=0

# 1. Check Skills
echo ""
echo -e "${YELLOW}[1/4] Checking EVIF Skills...${NC}"
SKILLS_DIR="$PROJECT_ROOT/.claude/skills"
if [ -d "$SKILLS_DIR" ]; then
    count=$(ls "$SKILLS_DIR"/*.SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    echo "  Found $count skills:"
    for skill in "$SKILLS_DIR"/*.SKILL.md; do
        name=$(basename "$skill" .SKILL.md)
        echo "    - $name"
    done
    if [ "${count:-0}" -ge 5 ]; then
        echo -e "  ${GREEN}✅ Skills check passed ($count skills)${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "  ${RED}❌ Insufficient skills (found $count, need >= 5)${NC}"
        FAILED=$((FAILED + 1))
    fi
else
    echo -e "  ${RED}❌ Skills directory not found${NC}"
    FAILED=$((FAILED + 1))
fi

# 2. Check MCP Server
echo ""
echo -e "${YELLOW}[2/4] Checking MCP Server...${NC}"
MCP_RESPONSE=$(curl -s --max-time 5 "http://localhost:3000/api/v1/health" 2>/dev/null || echo "")
if echo "$MCP_RESPONSE" | grep -q "healthy"; then
    echo -e "  ${GREEN}✅ MCP Server is healthy${NC}"
    PASSED=$((PASSED + 1))

    # Get tool count
    TOOLS_RESPONSE=$(curl -s --max-time 5 "http://localhost:3000/api/v1/tools" 2>/dev/null || echo "")
    TOOL_COUNT=$(echo "$TOOLS_RESPONSE" | jq '.tools | length' 2>/dev/null || echo "0")
    echo "  Tool count: $TOOL_COUNT"
else
    echo -e "  ${YELLOW}⚠️ MCP Server not running${NC}"
    echo "    Start with: cargo run -p evif-mcp -- --port 3000"
    SKIPPED=$((SKIPPED + 1))
fi

# 3. Check REST Server
echo ""
echo -e "${YELLOW}[3/4] Checking REST Server...${NC}"
REST_RESPONSE=$(curl -s --max-time 5 "http://localhost:8080/api/v1/health" 2>/dev/null || echo "")
if echo "$REST_RESPONSE" | grep -q "healthy"; then
    echo -e "  ${GREEN}✅ REST Server is healthy${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "  ${YELLOW}⚠️ REST Server not running${NC}"
    echo "    Start with: cargo run -p evif-rest -- --port 8080"
    SKIPPED=$((SKIPPED + 1))
fi

# 4. Check Skill Triggers
echo ""
echo -e "${YELLOW}[4/4] Checking Skill Triggers...${NC}"
TRIGGER_COUNT=0
for skill in "$SKILLS_DIR"/*.SKILL.md; do
    triggers=$(grep -c "triggers:" "$skill" 2>/dev/null || echo "0")
    TRIGGER_COUNT=$((TRIGGER_COUNT + triggers))
done
if [ "${TRIGGER_COUNT:-0}" -ge 5 ]; then
    echo -e "  ${GREEN}✅ All skills have triggers ($TRIGGER_COUNT triggers)${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "  ${YELLOW}⚠️ Some skills missing triggers${NC}"
    SKIPPED=$((SKIPPED + 1))
fi

# Summary
echo ""
echo "=========================================="
echo "Verification Summary"
echo "=========================================="
echo -e "Passed:  ${GREEN}$PASSED${NC}"
echo -e "Failed:  ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"

if [ "$FAILED" -eq 0 ]; then
    echo ""
    if [ "$SKIPPED" -eq 0 ]; then
        echo -e "${GREEN}✅ All checks passed - Skills + MCP fully integrated!${NC}"
    else
        echo -e "${YELLOW}⚠️ Checks passed but servers not running${NC}"
        echo "  Start servers to complete verification"
    fi
    exit 0
else
    echo ""
    echo -e "${RED}❌ Some checks failed${NC}"
    exit 1
fi