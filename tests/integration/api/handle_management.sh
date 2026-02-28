#!/usr/bin/env bash
# EVIF API Handle Management Tests (P1)
# Tests for handle endpoints: open, get, read, write, seek, sync, close, renew, list, stats

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"
source "$SCRIPT_DIR/../../lib/server_manager.sh"

# Test configuration
export EVIF_SERVER_PORT="${EVIF_SERVER_PORT:-8080}"
BASE_URL="http://localhost:${EVIF_SERVER_PORT}"

log_section "API Handle Management Tests (P1)"

# Start server
start_rest_server "$EVIF_SERVER_PORT" || {
    log_error "Failed to start REST server"
    exit 1
}

# Create test file for handle operations
setup_test_file() {
    curl -s -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","content":"Hello Handle World"}' >/dev/null
}

cleanup_test_file() {
    curl -s -X DELETE "${BASE_URL}/api/v1/files?path=/test_handle.txt" >/dev/null || true
}

# Test: POST /api/v1/handles/open
test_handle_open() {
    setup_test_file

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/open"
    assert_json_field "$response" "id" "response should have handle ID"

    # Close handle
    local handle_id
    handle_id=$(echo "$response" | jq -r '.id')
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    cleanup_test_file
}

# Test: GET /api/v1/handles/:id
test_handle_get() {
    setup_test_file

    # Open handle first
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/handles/${handle_id}")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/handles/:id"
    assert_json_value "$response" "id" "$handle_id" "handle ID should match"

    # Close handle
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    cleanup_test_file
}

# Test: POST /api/v1/handles/:id/read
test_handle_read() {
    setup_test_file

    # Open handle first
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/${handle_id}/read" \
        -H "Content-Type: application/json" \
        -d '{"offset":0,"length":100}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/:id/read"
    assert_output_contains "$response" "Hello Handle World" "read should return file content"

    # Close handle
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    cleanup_test_file
}

# Test: POST /api/v1/handles/:id/write
test_handle_write() {
    # Create file for writing
    curl -s -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle_write.txt","content":""}' >/dev/null

    # Open handle for writing
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle_write.txt","mode":"w"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/${handle_id}/write" \
        -H "Content-Type: application/json" \
        -d '{"data":"Written via handle","offset":0}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/:id/write"

    # Close handle
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    # Verify content
    local file_content
    file_content=$(curl -s "${BASE_URL}/api/v1/files?path=/test_handle_write.txt")
    assert_output_contains "$file_content" "Written via handle" "file should have written content"

    # Cleanup
    curl -s -X DELETE "${BASE_URL}/api/v1/files?path=/test_handle_write.txt" >/dev/null || true
}

# Test: POST /api/v1/handles/:id/seek
test_handle_seek() {
    setup_test_file

    # Open handle first
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/${handle_id}/seek" \
        -H "Content-Type: application/json" \
        -d '{"position":6,"whence":0}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/:id/seek"

    # Close handle
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    cleanup_test_file
}

# Test: POST /api/v1/handles/:id/sync
test_handle_sync() {
    setup_test_file

    # Open handle first
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/${handle_id}/sync")

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/:id/sync"

    # Close handle
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    cleanup_test_file
}

# Test: POST /api/v1/handles/:id/close
test_handle_close() {
    setup_test_file

    # Open handle first
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close")

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/:id/close"

    cleanup_test_file
}

# Test: POST /api/v1/handles/:id/renew
test_handle_renew() {
    setup_test_file

    # Open handle first
    local open_response handle_id
    open_response=$(curl -s -X POST "${BASE_URL}/api/v1/handles/open" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_handle.txt","mode":"r"}')
    handle_id=$(echo "$open_response" | jq -r '.id')

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/handles/${handle_id}/renew")

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/handles/:id/renew"

    # Close handle
    curl -s -X POST "${BASE_URL}/api/v1/handles/${handle_id}/close" >/dev/null || true

    cleanup_test_file
}

# Test: GET /api/v1/handles
test_handles_list() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/handles")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/handles"
}

# Test: GET /api/v1/handles/stats
test_handles_stats() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/handles/stats")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/handles/stats"
}

# Run all tests
main() {
    log_info "Starting API Handle Management Tests (P1)..."

    run_test "POST /api/v1/handles/open" test_handle_open
    run_test "GET /api/v1/handles/:id" test_handle_get
    run_test "POST /api/v1/handles/:id/read" test_handle_read
    run_test "POST /api/v1/handles/:id/write" test_handle_write
    run_test "POST /api/v1/handles/:id/seek" test_handle_seek
    run_test "POST /api/v1/handles/:id/sync" test_handle_sync
    run_test "POST /api/v1/handles/:id/close" test_handle_close
    run_test "POST /api/v1/handles/:id/renew" test_handle_renew
    run_test "GET /api/v1/handles" test_handles_list
    run_test "GET /api/v1/handles/stats" test_handles_stats

    print_test_summary
}

main "$@"