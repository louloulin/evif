#!/bin/bash
# oscript/teardown.sh - 环境清理脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "oscript Teardown - Cleanup"
echo "=========================================="

# Colors
GREEN='\033[0;32m'
NC='\033[0m'

# Kill servers
kill_server() {
    local name=$1
    local pid_file=$2

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file" 2>/dev/null || echo "")
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "Stopping $name (PID: $pid)..."
            kill "$pid" 2>/dev/null || true
            sleep 1
            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
        fi
        rm -f "$pid_file"
    fi
}

kill_server "EVIF Server" "/tmp/evif_server_pid"
kill_server "MCP Server" "/tmp/evif_mcp_pid"

# Kill any remaining processes on test ports
for port in 3000 8080; do
    pid=$(lsof -ti:$port 2>/dev/null || true)
    if [ -n "$pid" ]; then
        echo "Cleaning up port $port (PID: $pid)..."
        kill $pid 2>/dev/null || true
    fi
done

# Clean up test directory
if [ -n "$TEST_DIR" ] && [ -d "$TEST_DIR" ]; then
    echo "Removing test directory: $TEST_DIR"
    rm -rf "$TEST_DIR"
fi

echo ""
echo -e "${GREEN}✅ Cleanup complete${NC}"

exit 0