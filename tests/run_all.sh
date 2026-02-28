#!/usr/bin/env bash
# EVIF Master Test Runner
# Orchestrates all integration tests

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
export EVIF_SERVER_PORT="${EVIF_SERVER_PORT:-8080}"
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Test counters
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0
SKIPPED_SUITES=0

log_section() {
    echo ""
    echo -e "${BLUE}=== $1 ===${NC}"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASSED_SUITES++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAILED_SUITES++))
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    ((SKIPPED_SUITES++))
}

# Cleanup on exit
cleanup() {
    log_info "Cleaning up..."
    # Stop any running servers
    if [ -n "${EVIF_SERVER_PID:-}" ]; then
        kill $EVIF_SERVER_PID 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Run a test suite
run_suite() {
    local suite_name="$1"
    local suite_path="$2"

    ((TOTAL_SUITES++))
    log_info "Running: $suite_name"

    if [ ! -x "$suite_path" ]; then
        log_skip "$suite_name (not executable)"
        return 0
    fi

    if timeout 300 "$suite_path"; then
        log_pass "$suite_name"
        return 0
    else
        log_fail "$suite_name"
        return 1
    fi
}

# Print final summary
print_summary() {
    echo ""
    log_section "Test Suite Summary"
    echo "  Passed: $PASSED_SUITES"
    echo "  Failed: $FAILED_SUITES"
    echo "  Skipped: $SKIPPED_SUITES"
    echo "  Total: $TOTAL_SUITES"
    echo ""

    if [ $FAILED_SUITES -gt 0 ]; then
        echo -e "${RED}Some tests failed!${NC}"
        return 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    fi
}

# Main
main() {
    log_section "EVIF Functional Test Suite"
    log_info "Test directory: $TEST_DIR"
    log_info "Server port: $EVIF_SERVER_PORT"

    # P0 CLI Tests
    log_section "P0 CLI Tests"
    run_suite "CLI File Operations" "$TEST_DIR/integration/cli/file_operations.sh"
    run_suite "CLI Plugin Management" "$TEST_DIR/integration/cli/plugin_management.sh"
    run_suite "CLI System Commands" "$TEST_DIR/integration/cli/system_commands.sh"

    # P0 API Tests
    log_section "P0 API Tests"
    run_suite "API Core Endpoints" "$TEST_DIR/integration/api/core_endpoints.sh"
    run_suite "API Mount Management" "$TEST_DIR/integration/api/mount_management.sh"

    # P1 Tests
    log_section "P1 Tests"
    run_suite "API Handle Management" "$TEST_DIR/integration/api/handle_management.sh"

    # Performance Tests
    log_section "Performance Tests"
    run_suite "Performance Benchmarks" "$TEST_DIR/performance/benchmarks.sh"

    # Summary
    print_summary
}

# Parse arguments
VERBOSE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            export EVIF_VERBOSE=true
            shift
            ;;
        *)
            echo "Usage: $0 [-v|--verbose]"
            exit 1
            ;;
    esac
done

main "$@"