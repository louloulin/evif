#!/bin/bash
# oscript/verify_pipeline.sh - 主验证管道

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "EVIF oscript Verification Pipeline"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Track results
PHASE_RESULTS=()

# Phase 1: Setup
echo -e "${YELLOW}[1/5] Setting up environment...${NC}"
bash "$SCRIPT_DIR/setup.sh"
if [ $? -eq 0 ]; then
    PHASE_RESULTS+=("setup:✅")
    echo -e "  ${GREEN}✅ Setup complete${NC}"
else
    PHASE_RESULTS+=("setup:❌")
    echo -e "  ${RED}❌ Setup failed${NC}"
    exit 1
fi

# Phase 2: Start Servers
echo ""
echo -e "${YELLOW}[2/5] Starting servers...${NC}"
bash "$SCRIPT_DIR/start.sh"
if [ $? -eq 0 ]; then
    PHASE_RESULTS+=("start:✅")
    echo -e "  ${GREEN}✅ Servers started${NC}"
else
    PHASE_RESULTS+=("start:❌")
    echo -e "  ${RED}❌ Server start failed${NC}"
    bash "$SCRIPT_DIR/teardown.sh"
    exit 1
fi

# Phase 3: API Verification
echo ""
echo -e "${YELLOW}[3/5] Running API verification...${NC}"
bash "$SCRIPT_DIR/verify.sh"
VERIFY_RESULT=$?
if [ $VERIFY_RESULT -eq 0 ]; then
    PHASE_RESULTS+=("verify:✅")
else
    PHASE_RESULTS+=("verify:❌")
fi

# Phase 4: MCP Tests
echo ""
echo -e "${YELLOW}[4/5] Running MCP tests...${NC}"
bash "$SCRIPT_DIR/test_mcp.sh"
MCP_RESULT=$?
if [ $MCP_RESULT -eq 0 ]; then
    PHASE_RESULTS+=("mcp:✅")
else
    PHASE_RESULTS+=("mcp:❌")
fi

# Phase 5: Teardown
echo ""
echo -e "${YELLOW}[5/5] Cleaning up...${NC}"
bash "$SCRIPT_DIR/teardown.sh"
PHASE_RESULTS+=("teardown:✅")

# Summary
echo ""
echo "=========================================="
echo "Verification Pipeline Summary"
echo "=========================================="
echo ""

for result in "${PHASE_RESULTS[@]}"; do
    IFS=':' read -r phase status <<< "$result"
    if [ "$status" = "✅" ]; then
        echo -e "  $phase: ${GREEN}$status${NC}"
    else
        echo -e "  $phase: ${RED}$status${NC}"
    fi
done

echo ""

# Final verdict
if [[ " ${PHASE_RESULTS[*]} " =~ "verify:❌" ]] || [[ " ${PHASE_RESULTS[*]} " =~ "mcp:❌" ]]; then
    echo -e "${RED}❌ Verification FAILED${NC}"
    echo ""
    echo "Failed phases:"
    for result in "${PHASE_RESULTS[@]}"; do
        if [[ "$result" == *":❌" ]]; then
            phase="${result%:❌}"
            echo "  - $phase"
        fi
    done
    exit 1
else
    echo -e "${GREEN}✅ All verification passed${NC}"
    echo ""
    echo "Pipeline completed successfully!"
    exit 0
fi