#!/bin/bash
# EVIF Agent Workflow Demo - Start Script
#
# This script:
# 1. Starts evif-rest server
# 2. Runs Python SDK smoke test
# 3. Runs the task queue worker demo
#
# Usage: ./start_demo.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}EVIF Agent Workflow Demo${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Kill any existing evif-rest on port 8081
echo -e "${YELLOW}[Setup] Checking for existing evif-rest...${NC}"
if lsof -i :8081 >/dev/null 2>&1; then
    echo "  Port 8081 already in use, will use existing server"
    USE_EXISTING=true
else
    USE_EXISTING=false
fi

# Start evif-rest if not running
REST_PID=""
if [ "$USE_EXISTING" = false ]; then
    echo -e "${YELLOW}[Start] Launching evif-rest...${NC}"

    # Build first
    echo "  Building evif-rest..."
    cargo build -p evif-rest --quiet 2>/dev/null || cargo build -p evif-rest

    # Start server with disabled auth for demo
    cargo run -p evif-rest -- \
        --port 8081 \
        --auth-mode disabled &
    REST_PID=$!

    echo "  Started with PID $REST_PID"

    # Wait for server to be ready
    echo -e "${YELLOW}[Wait] Waiting for server to start...${NC}"
    sleep 3

    # Check if server is running
    for i in {1..10}; do
        if curl -sf http://localhost:8081/api/v1/health >/dev/null 2>&1; then
            echo -e "${GREEN}  Server is ready!${NC}"
            break
        fi
        if [ $i -eq 10 ]; then
            echo -e "${RED}  Server failed to start in time${NC}"
            exit 1
        fi
        sleep 1
    done
else
    echo "  Using existing server on port 8081"
fi

echo ""

# Run Python SDK smoke test
echo -e "${YELLOW}[Test] Python SDK smoke test...${NC}"
cd "$(dirname "$SCRIPT_DIR")"  # Go to repo root

# Check if evif package is installed
if ! python3 -c "import evif" 2>/dev/null; then
    echo "  Installing evif Python SDK..."
    cd crates/evif-python
    pip install -e . --quiet
    cd ../..
fi

# Run smoke test
python3 -c "
from evif import Client
import asyncio

async def test():
    client = Client('http://localhost:8081', api_key='write-key')
    print('  Client created')
    health = client.health()
    print(f'  Health: {health}')
    return True

result = asyncio.run(test())
print('  ✓ SDK smoke test passed')
"
echo ""

# Run task queue worker demo
echo -e "${YELLOW}[Demo] Running Task Queue Worker Demo...${NC}"
python3 demos/agent_workflow/task_queue_worker.py

echo ""

# Cleanup
if [ -n "$REST_PID" ]; then
    echo -e "${YELLOW}[Cleanup] Stopping evif-rest (PID $REST_PID)${NC}"
    kill $REST_PID 2>/dev/null || true
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Demo completed successfully!${NC}"
echo -e "${GREEN}========================================${NC}"