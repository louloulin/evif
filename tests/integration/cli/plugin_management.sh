#!/usr/bin/env bash
# EVIF CLI Plugin Management Tests (P0)
# Tests for: mount, mount with config, mounts, unmount

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"
source "$SCRIPT_DIR/../../lib/server_manager.sh"

# Test configuration
export EVIF_CLI="${EVIF_CLI:-cargo run -p evif-cli --}"

log_section "CLI Plugin Management Tests"

# Test: mount memfs /mem
test_mount_memfs() {
    local output
    output=$($EVIF_CLI mount memfs /test_mem 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "mount memfs should succeed"

    # Verify mount appears in mounts list
    output=$($EVIF_CLI mounts 2>&1)
    assert_output_contains "$output" "/test_mem" "mount should appear in mounts list"

    # Cleanup
    $EVIF_CLI unmount /test_mem 2>&1 >/dev/null || true
}

# Test: mount memfs /mem -c '{"size":1000}'
test_mount_with_config() {
    local output
    output=$($EVIF_CLI mount memfs /test_mem_config -c '{"size":1000}' 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "mount with config should succeed"

    # Verify mount appears in mounts list
    output=$($EVIF_CLI mounts 2>&1)
    assert_output_contains "$output" "/test_mem_config" "configured mount should appear in list"

    # Cleanup
    $EVIF_CLI unmount /test_mem_config 2>&1 >/dev/null || true
}

# Test: mounts
test_mounts_list() {
    # Mount a plugin first
    $EVIF_CLI mount memfs /test_mounts_list 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI mounts 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "mounts should succeed"
    assert_output_contains "$output" "/test_mounts_list" "mounts should show mounted plugins"
    assert_output_contains "$output" "memfs" "mounts should show plugin name"

    # Cleanup
    $EVIF_CLI unmount /test_mounts_list 2>&1 >/dev/null || true
}

# Test: unmount /mem
test_unmount() {
    # Mount a plugin first
    $EVIF_CLI mount memfs /test_unmount 2>&1 >/dev/null || true

    local output
    output=$($EVIF_CLI unmount /test_unmount 2>&1)
    local exit_code=$?

    assert_exit_code 0 $exit_code "unmount should succeed"

    # Verify mount removed
    output=$($EVIF_CLI mounts 2>&1)
    assert_output_not_contains "$output" "/test_unmount" "unmounted path should not appear in list"
}

# Test: mount invalid plugin
test_mount_invalid_plugin() {
    local output
    output=$($EVIF_CLI mount fakefs_plugin /fake 2>&1) && exit_code=0 || exit_code=$?

    # Should fail with exit code 126 (command not executable)
    if [ $exit_code -eq 126 ] || [ $exit_code -eq 1 ]; then
        log_pass "mount invalid plugin should fail (exit code: $exit_code)"
        return 0
    else
        log_fail "mount invalid plugin" "exit code 126 or 1" "exit code $exit_code"
        return 1
    fi
}

# Test: unmount non-existent mount
test_unmount_nonexistent() {
    local output
    output=$($EVIF_CLI unmount /nonexistent_mount_xyz 2>&1) && exit_code=0 || exit_code=$?

    # Should fail
    if [ $exit_code -ne 0 ]; then
        log_pass "unmount non-existent should fail (exit code: $exit_code)"
        assert_output_contains "$output" "not" "error message should mention mount not found"
        return 0
    else
        log_fail "unmount non-existent" "non-zero exit code" "exit code 0"
        return 1
    fi
}

# Run all tests
main() {
    log_info "Starting CLI Plugin Management Tests..."

    run_test "mount memfs /mem" test_mount_memfs
    run_test "mount memfs /mem -c '{\"size\":1000}'" test_mount_with_config
    run_test "mounts" test_mounts_list
    run_test "unmount /mem" test_unmount
    run_test "mount invalid plugin" test_mount_invalid_plugin
    run_test "unmount non-existent" test_unmount_nonexistent

    print_test_summary
}

main "$@"
