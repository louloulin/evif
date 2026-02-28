#!/usr/bin/env bash
# EVIF Integration Test Helpers
# Common functions for all test scripts

set -euo pipefail

# Export EVIF CLI path (assumes built)
export EVIF_CLI="${EVIF_CLI:-cargo run -p evif-cli --}"

# Colors for output
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m' # No Color

# Test counters
export TESTS_PASS=0
export TESTS_FAIL=0
export TESTS_SKIP=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASS++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    echo -e "${RED}  Expected: $2${NC}"
    echo -e "${RED}  Actual: $3${NC}"
    ((TESTS_FAIL++))
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    ((TESTS_SKIP++))
}

log_section() {
    echo ""
    echo -e "${BLUE}=== $1 ===${NC}"
}

# Test execution wrapper
run_test() {
    local test_name="$1"
    local test_func="$2"

    log_info "Running: $test_name"
    if $test_func; then
        log_pass "$test_name"
        return 0
    else
        log_fail "$test_name" "success" "failure"
        return 1
    fi
}

# Cleanup trap
cleanup_on_error() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        log_info "Cleaning up after error (exit code: $exit_code)..."
    fi
}

trap cleanup_on_error EXIT

# Check if REST server is running
check_server() {
    local port="${1:-8080}"
    local max_attempts="${2:-30}"
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "http://localhost:${port}/health" >/dev/null 2>&1; then
            return 0
        fi
        ((attempt++))
        sleep 1
    done

    return 1
}

# Create test file with known content
create_test_file() {
    local path="$1"
    local content="${2:-Hello EVIF}"

    echo "$content" > "$path"
}

# Generate unique test path
gen_test_path() {
    local prefix="${1:-test}"
    echo "/tmp/evif_test_${prefix}_$$"
}

# Check EVIF CLI availability
check_evif_cli() {
    if ! cargo run -p evif-cli -- health >/dev/null 2>&1; then
        log_skip "EVIF CLI not available"
        return 1
    fi
    return 0
}

# Print test summary
print_test_summary() {
    echo ""
    log_section "Test Summary"
    echo "  Pass: $TESTS_PASS"
    echo "  Fail: $TESTS_FAIL"
    echo "  Skip: $TESTS_SKIP"
    local total=$((TESTS_PASS + TESTS_FAIL + TESTS_SKIP))
    echo "  Total: $total"

    if [ $TESTS_FAIL -gt 0 ]; then
        return 1
    fi
    return 0
}
