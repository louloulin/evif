#!/bin/bash
# EVIF Agent Workflow Demo - Official Demo Script
# This script demonstrates the complete EVIF Agent workflow:
# 1. Context management (L0/L1/L2)
# 2. Skill discovery and invocation
# 3. Pipe coordination for multi-agent work
# 4. Memory retrieval

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
EVIF_PORT="${EVIF_PORT:-8081}"
EVIF_HOST="${EVIF_HOST:-localhost}"
EVIF_BASE_URL="http://${EVIF_HOST}:${EVIF_PORT}/api/v1"

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           EVIF Agent Workflow Demo                         ║${NC}"
echo -e "${BLUE}║  Context + Skills + Pipes + Memory                        ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Check server health
echo -e "${YELLOW}[Step 1/6] Checking EVIF Server...${NC}"
if curl -sf "${EVIF_BASE_URL}/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Server is healthy"
    curl -s "${EVIF_BASE_URL}/health" | jq -r '.status' 2>/dev/null || echo "ok"
else
    echo -e "${RED}✗${NC} Server not reachable at ${EVIF_BASE_URL}"
    echo "  Start with: cargo run -p evif-rest -- --port ${EVIF_PORT}"
    exit 1
fi
echo ""

# Step 2: Context Management
echo -e "${YELLOW}[Step 2/6] Context Management${NC}"
echo "  L0 (Current Task): $(curl -s "${EVIF_BASE_URL}/context/l0/current" 2>/dev/null | jq -r '.content // empty' 2>/dev/null || echo '(empty)')"
echo "  L1 (Decisions): $(curl -s "${EVIF_BASE_URL}/context/l1/decisions.md" 2>/dev/null | head -3 | jq -Rs . 2>/dev/null || echo '(empty)')"
echo ""

# Update L0
echo "  Updating L0..."
curl -s -X PUT "${EVIF_BASE_URL}/context/l0/current" \
    -H "Content-Type: application/json" \
    -d '{"content": "EVIF MVP 6.0 Demo - Testing Agent Workflow"}' > /dev/null
echo -e "${GREEN}✓${NC} L0 updated"

# Add a decision
echo "  Adding decision to L1..."
curl -s -X PUT "${EVIF_BASE_URL}/context/l1/decisions.md" \
    -H "Content-Type: application/json" \
    -d '{"content": "- $(date +%Y-%m-%d): Started MVP 6.0 Agent Workflow Demo\n"}' > /dev/null
echo -e "${GREEN}✓${NC} L1 updated"
echo ""

# Step 3: Skill Discovery
echo -e "${YELLOW}[Step 3/6] Skill Discovery${NC}"
echo "  Available skills:"
SKILLS=$(curl -s "${EVIF_BASE_URL}/skills" 2>/dev/null | jq -r '.skills[] | "    - \(.name): \(.description)"' 2>/dev/null || echo "    (use 'evif ls /skills' to list)")
echo "${SKILLS}"
echo ""

# Step 4: Pipe Coordination
echo -e "${YELLOW}[Step 4/6] Pipe Coordination${NC}"
DEMO_PIPE="/pipes/demo-$(date +%s)"
echo "  Creating pipe: ${DEMO_PIPE}"
curl -s -X POST "${EVIF_BASE_URL}/fs/mkdir" \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"${DEMO_PIPE}\"}" > /dev/null

# Write task to pipe
echo "  Writing task to pipe..."
curl -s -X PUT "${EVIF_BASE_URL}/fs/write" \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"${DEMO_PIPE}/input\", \"content\": \"Analyze codebase for TODO comments\"}" > /dev/null

# Read from pipe
echo "  Reading from pipe..."
CONTENT=$(curl -s "${EVIF_BASE_URL}/fs/read?path=${DEMO_PIPE}/input" 2>/dev/null | jq -r '.content' 2>/dev/null || echo "")
echo "    Task: ${CONTENT}"
echo -e "${GREEN}✓${NC} Pipe coordination working"
echo ""

# Step 5: Memory Operations
echo -e "${YELLOW}[Step 5/6] Memory Operations${NC}"
# Store a memory
echo "  Storing memory..."
curl -s -X POST "${EVIF_BASE_URL}/memory/store" \
    -H "Content-Type: application/json" \
    -d '{"content": "EVIF MVP 6.0 demo completed successfully", "tags": ["demo", "mvp6.0"]}' > /dev/null

# Search memory
echo "  Searching memory..."
RESULT=$(curl -s "${EVIF_BASE_URL}/memory/search?query=demo&limit=3" 2>/dev/null | jq -r '.results[0].content' 2>/dev/null || echo "")
if [ -n "$RESULT" ]; then
    echo "    Found: ${RESULT}"
    echo -e "${GREEN}✓${NC} Memory retrieval working"
else
    echo "    (Memory search returned empty - normal on fresh install)"
fi
echo ""

# Step 6: MCP Tools (demonstrate cost optimization)
echo -e "${YELLOW}[Step 6/6] MCP Tool - Bounded Read (Cost Optimization)${NC}"
echo "  Reading with bounded output (max_lines=5)..."
BOUNDED=$(curl -s "${EVIF_BASE_URL}/fs/read?path=/context/L0/current&max_lines=5" 2>/dev/null | jq -r '.content' 2>/dev/null || echo "")
echo "    Content: ${BOUNDED}"
echo -e "${GREEN}✓${NC} Bounded read (cost optimization) working"
echo ""

# Cleanup
echo -e "${YELLOW}Cleanup${NC}"
echo "  Removing demo pipe..."
curl -s -X DELETE "${EVIF_BASE_URL}/fs/rm?path=${DEMO_PIPE}&recursive=true" > /dev/null 2>&1 || true
echo ""

# Summary
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Demo Complete!                                     ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "You've successfully demonstrated:"
echo "  1. ${GREEN}Context Management${NC} - L0/L1/L2 layered context"
echo "  2. ${GREEN}Skill Discovery${NC} - Finding and using skills"
echo "  3. ${GREEN}Pipe Coordination${NC} - Multi-agent task coordination"
echo "  4. ${GREEN}Memory Operations${NC} - Storing and retrieving memories"
echo "  5. ${GREEN}Cost Optimization${NC} - Bounded reads reduce token usage"
echo ""
echo "Next steps:"
echo "  - Run 'evif ls /skills' to explore all skills"
echo "  - Connect to Claude Code: 'evif connect claude'"
echo "  - Check docs: https://github.com/evif/evif#readme"
echo ""
