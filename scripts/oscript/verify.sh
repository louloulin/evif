#!/bin/bash
# oscript/verify.sh - API 验证脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "oscript Verify - API Verification"
echo "=========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get port from env or use default
SERVER_PORT=${SERVER_PORT:-8080}
BASE_URL="http://localhost:$SERVER_PORT"

# Counter
PASSED=0
FAILED=0

# Test helper
test_api() {
    local name=$1
    local url=$2
    local expected=$3

    echo -n "  $name... "
    response=$(curl -s "$url" 2>/dev/null || echo "")
    if echo "$response" | grep -q "$expected"; then
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
echo "=== Basic Health ==="
test_api "Health Check" "$BASE_URL/api/v1/health" "healthy"
test_api "Root Health" "$BASE_URL/" "version"

echo ""
echo "=== Directory Operations ==="
test_api "List Mounts" "$BASE_URL/api/v1/mounts" "mounts"
test_api "List Directory" "$BASE_URL/api/v1/directories?path=/mem" "files"

echo ""
echo "=== File Operations ==="
# Create test file
curl -s -X PUT "$BASE_URL/api/v1/files?path=/mem/oscript_test.txt" \
    -H "Content-Type: application/json" \
    -d '{"content": "oscript verification test", "encoding": "utf-8"}' > /dev/null

test_api "Read File" "$BASE_URL/api/v1/files?path=/mem/oscript_test.txt" "oscript verification"
test_api "Stat File" "$BASE_URL/api/v1/stat?path=/mem/oscript_test.txt" "size"

echo ""
echo "=== Cleanup ==="
curl -s -X DELETE "$BASE_URL/api/v1/files?path=/mem/oscript_test.txt" > /dev/null || true

echo ""
echo "=========================================="
echo "API Verification Summary"
echo "=========================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ All API tests passed${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}❌ Some API tests failed${NC}"
    exit 1
fi