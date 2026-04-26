#!/bin/bash
# EVIF Agent Workflow Demo - Start Script
#
# This script:
# 1. Starts evif-rest server
# 2. Runs Python SDK smoke test
# 3. Runs the task queue worker demo
# 4. Runs the pipe-triggered agent demo
#
# Usage: ./start_demo.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}EVIF Agent Workflow Demo${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../.."

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
    echo -e "${YELLOW}[Build] Building evif-rest...${NC}"
    cargo build -p evif-rest --quiet

    echo -e "${YELLOW}[Start] Launching evif-rest...${NC}"
    EVIF_REST_AUTH_MODE=disabled ./target/debug/evif-rest --port 8081 &
    REST_PID=$!
    echo "  Started with PID $REST_PID"

    echo -e "${YELLOW}[Wait] Waiting for server...${NC}"
    for i in {1..10}; do
        if curl -sf http://localhost:8081/api/v1/health >/dev/null 2>&1; then
            echo -e "${GREEN}  Server ready!${NC}"
            break
        fi
        if [ $i -eq 10 ]; then
            echo -e "${RED}  Server failed to start${NC}"
            exit 1
        fi
        sleep 1
    done
else
    echo "  Using existing server"
fi

echo ""

# Run Python SDK smoke test
echo -e "${YELLOW}[Test] Python SDK smoke test...${NC}"
PYTHONPATH=crates/evif-python python3 -c "
from evif import Client
client = Client('http://localhost:8081')
print('  Health:', client.health())
print('  Mounts:', len(client.mounts()), 'plugins')
print('  ls /mem:', [f.name for f in client.ls('/mem')][:3])
"
echo -e "${GREEN}  SDK smoke test passed${NC}"
echo ""

# Run Task Queue Worker Demo
echo -e "${YELLOW}[Demo] Task Queue Worker...${NC}"
PYTHONPATH=crates/evif-python python3 demos/agent_workflow/task_queue_worker.py 2>&1 | sed 's/^/  /'
echo ""

# Run Pipe-Triggered Agent Demo
echo -e "${YELLOW}[Demo] Pipe-Triggered Agent...${NC}"
PYTHONPATH=crates/evif-python python3 demos/agent_workflow/pipe_triggered_agent.py 2>&1 | sed 's/^/  /'
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
