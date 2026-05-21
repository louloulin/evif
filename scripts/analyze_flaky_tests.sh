#!/bin/bash
# EVIF Flaky Test Analysis
# Run ignored tests and analyze patterns

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=========================================="
echo "EVIF Flaky Test Analysis"
echo "=========================================="
echo ""

cd "$PROJECT_ROOT"

echo "=== Flaky Tests by File ==="
echo ""

# Count ignored tests per file
for file in crates/evif-rest/tests/*.rs tests/e2e/tests/*.rs crates/evif-bench/src/*.rs; do
    if [ -f "$file" ]; then
        ignored=$(grep -c "ignore.*Flaky" "$file" 2>/dev/null || echo "0")
        total=$(grep -c "#\[tokio::test\]" "$file" 2>/dev/null || echo "0")
        if [ "$ignored" -gt 0 ]; then
            filename=$(basename "$file")
            echo "  $filename: $ignored/$total tests marked flaky"
        fi
    fi
done

echo ""
echo "=== All Flaky Tests ==="
echo ""

# List all ignored tests
cargo test --workspace -- --list 2>/dev/null | grep "ignored" | grep -v "^$" | head -50

echo ""
echo "=========================================="
echo "Analysis Complete"
echo "=========================================="
echo ""
echo "Root causes identified:"
echo "  1. Server startup race - tests start server in multi-threaded tokio runtime"
echo "  2. Test isolation issues - shared state between tests"
echo "  3. Environment variable race - concurrent test access"
echo ""
echo "Recommended fixes:"
echo "  1. Use dedicated server process for integration tests"
echo "  2. Add proper test isolation (temp directories, unique ports)"
echo "  3. Use thread-local or mutex-protected env vars"