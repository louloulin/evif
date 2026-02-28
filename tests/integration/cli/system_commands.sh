#!/usr/bin/env bash
# EVIF CLI System Commands Tests (P0)
# Tests for: health, stats

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"

# Test configuration
export EVIF_CLI="${EVIF_CLI:-cargo run -p evif-cli --}"

log_section "CLI System Commands Tests"

# Test: health
test_health() {
    local output
    output=$($EVIF_CLI health 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "health should succeed"
    assert_output_contains "$output" "status" "health should show status"
    assert_output_contains "$output" "version" "health should show version"
}

# Test: stats
test_stats() {
    local output
    output=$($EVIF_CLI stats 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "stats should succeed"
    # Stats should show connection/server information
    assert_output_contains "$output" "connected" "stats should show connection status"
}

# Run all tests
main() {
    log_info "Starting CLI System Commands Tests..."

    run_test "health" test_health
    run_test "stats" test_stats

    print_test_summary
}

main "$@"