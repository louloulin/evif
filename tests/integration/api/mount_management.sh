#!/usr/bin/env bash
# EVIF API Mount Management Tests (P0)
# Tests for: GET /api/v1/mounts, POST /api/v1/mount, POST /api/v1/unmount

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"
source "$SCRIPT_DIR/../../lib/server_manager.sh"

# Test configuration
export EVIF_SERVER_PORT="${EVIF_SERVER_PORT:-8080}"
BASE_URL="http://localhost:${EVIF_SERVER_PORT}"

log_section "API Mount Management Tests"

# Start server
start_rest_server "$EVIF_SERVER_PORT" || {
    log_error "Failed to start REST server"
    exit 1
}

# Test: GET /api/v1/mounts
test_mounts_list() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/mounts")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/mounts"
}

# Test: POST /api/v1/mount - mount memfs
test_mount_memfs() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/mount" \
        -H "Content-Type: application/json" \
        -d '{"plugin":"memfs","path":"/test_api_mount"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/mount (memfs)"

    # Verify mount appears in list
    response=$(curl -s "${BASE_URL}/api/v1/mounts")
    assert_output_contains "$response" "/test_api_mount" "mount should appear in mounts list"

    # Cleanup
    curl -s -X POST "${BASE_URL}/api/v1/unmount" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_api_mount"}' >/dev/null || true
}

# Test: POST /api/v1/mount - mount with config
test_mount_with_config() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/mount" \
        -H "Content-Type: application/json" \
        -d '{"plugin":"memfs","path":"/test_api_mount_config","config":{"size":1000}}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/mount with config"

    # Verify mount appears in list
    response=$(curl -s "${BASE_URL}/api/v1/mounts")
    assert_output_contains "$response" "/test_api_mount_config" "configured mount should appear in list"

    # Cleanup
    curl -s -X POST "${BASE_URL}/api/v1/unmount" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_api_mount_config"}' >/dev/null || true
}

# Test: POST /api/v1/unmount
test_unmount() {
    # Mount first
    curl -s -X POST "${BASE_URL}/api/v1/mount" \
        -H "Content-Type: application/json" \
        -d '{"plugin":"memfs","path":"/test_api_unmount"}' >/dev/null

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/unmount" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_api_unmount"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/unmount"

    # Verify mount removed
    response=$(curl -s "${BASE_URL}/api/v1/mounts")
    assert_output_not_contains "$response" "/test_api_unmount" "unmounted path should not appear"
}

# Test: POST /api/v1/mount - invalid plugin
test_mount_invalid_plugin() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/mount" \
        -H "Content-Type: application/json" \
        -d '{"plugin":"fake_plugin_xyz","path":"/fake"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    # Should fail with appropriate status code
    if [ "$status_code" -eq 404 ] || [ "$status_code" -eq 500 ]; then
        log_pass "mount invalid plugin should fail (status: $status_code)"
        return 0
    else
        log_fail "mount invalid plugin" "404 or 500" "status $status_code"
        return 1
    fi
}

# Test: POST /api/v1/unmount - non-existent mount
test_unmount_nonexistent() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/unmount" \
        -H "Content-Type: application/json" \
        -d '{"path":"/nonexistent_mount_xyz"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    # Should fail
    if [ "$status_code" -ne 200 ]; then
        log_pass "unmount non-existent should fail (status: $status_code)"
        return 0
    else
        log_fail "unmount non-existent" "non-200 status" "status $status_code"
        return 1
    fi
}

# Run all tests
main() {
    log_info "Starting API Mount Management Tests..."

    run_test "GET /api/v1/mounts" test_mounts_list
    run_test "POST /api/v1/mount (memfs)" test_mount_memfs
    run_test "POST /api/v1/mount with config" test_mount_with_config
    run_test "POST /api/v1/unmount" test_unmount
    run_test "POST /api/v1/mount (invalid plugin)" test_mount_invalid_plugin
    run_test "POST /api/v1/unmount (non-existent)" test_unmount_nonexistent

    print_test_summary
}

main "$@"