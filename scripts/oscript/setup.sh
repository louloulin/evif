#!/bin/bash
# oscript/setup.sh - 环境准备脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "oscript Setup - Environment Preparation"
echo "=========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check prerequisites
check_prerequisite() {
    local cmd=$1
    local name=$2
    echo -n "Checking $name... "
    if command -v "$cmd" &> /dev/null; then
        echo -e "${GREEN}✅${NC}"
        return 0
    else
        echo -e "${RED}❌${NC}"
        echo "  Please install $name"
        return 1
    fi
}

# Check required tools
MISSING=0
check_prerequisite "cargo" "Rust/Cargo" || MISSING=1
check_prerequisite "curl" "curl" || MISSING=1
check_prerequisite "jq" "jq" || MISSING=1

if [ $MISSING -eq 1 ]; then
    echo -e "${RED}❌ Missing prerequisites${NC}"
    exit 1
fi

# Create test directories
echo "Creating test directories..."
TEST_DIR="/tmp/evif-oscript-test-$$"
mkdir -p "$TEST_DIR"
echo "  Test directory: $TEST_DIR"

# Create artifacts directory
ARTIFACTS_DIR="$PROJECT_ROOT/artifacts"
mkdir -p "$ARTIFACTS_DIR"
echo "  Artifacts directory: $ARTIFACTS_DIR"

# Generate unique test port
MCP_PORT=$((30000 + RANDOM % 1000))
echo "  MCP Port: $MCP_PORT"

# Create test config
cat > "$TEST_DIR/test-config.env" << EOF
MCP_PORT=$MCP_PORT
TEST_DIR=$TEST_DIR
ARTIFACTS_DIR=$ARTIFACTS_DIR
PROJECT_ROOT=$PROJECT_ROOT
EOF

echo ""
echo -e "${GREEN}✅ Setup complete${NC}"
echo "Config written to: $TEST_DIR/test-config.env"
echo ""

# Export for child scripts
export MCP_PORT
export TEST_DIR
export ARTIFACTS_DIR
export PROJECT_ROOT

exit 0