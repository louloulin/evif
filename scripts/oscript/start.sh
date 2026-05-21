#!/bin/bash
# oscript/start.sh - 服务启动脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "oscript Start - Server Startup"
echo "=========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get port from env or use default
MCP_PORT=${MCP_PORT:-3000}
SERVER_PORT=${SERVER_PORT:-8080}

echo "Starting servers on ports MCP=$MCP_PORT, Server=$SERVER_PORT..."

# Kill any existing processes on these ports
kill_port() {
    local port=$1
    local pid=$(lsof -ti:$port 2>/dev/null || true)
    if [ -n "$pid" ]; then
        echo "  Killing existing process on port $port (PID: $pid)"
        kill $pid 2>/dev/null || true
        sleep 1
    fi
}

kill_port $MCP_PORT
kill_port $SERVER_PORT

# Start EVIF REST Server
echo "Starting EVIF REST Server..."
cd "$PROJECT_ROOT"
cargo run -p evif-rest -- --port $SERVER_PORT &
SERVER_PID=$!
echo "  REST Server PID: $SERVER_PID"

# Wait for server to be ready
echo "Waiting for REST Server to be ready..."
for i in {1..30}; do
    if curl -s "http://localhost:$SERVER_PORT/api/v1/health" | grep -q "healthy"; then
        echo -e "  ${GREEN}✅ REST Server ready${NC}"
        break
    fi
    sleep 0.5
done

# Start MCP Server
echo "Starting EVIF MCP Server..."
cargo run -p evif-mcp -- --port $MCP_PORT &
MCP_PID=$!
echo "  MCP Server PID: $MCP_PID"

# Wait for MCP server to be ready
echo "Waiting for MCP Server to be ready..."
for i in {1..30}; do
    if curl -s "http://localhost:$MCP_PORT/api/v1/health" | grep -q "healthy"; then
        echo -e "  ${GREEN}✅ MCP Server ready${NC}"
        break
    fi
    sleep 0.5
done

# Export PIDs for teardown
echo $SERVER_PID > /tmp/evif_server_pid
echo $MCP_PID > /tmp/evif_mcp_pid

echo ""
echo -e "${GREEN}✅ All servers started${NC}"
echo ""

exit 0