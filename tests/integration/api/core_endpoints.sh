#!/usr/bin/env bash
# EVIF API Core Endpoints Tests (P0)
# Tests for: /health, /api/v1/files (GET/POST/PUT/DELETE), /api/v1/directories (GET/POST/DELETE)

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"
source "$SCRIPT_DIR/../../lib/server_manager.sh"

# Test configuration
export EVIF_SERVER_PORT="${EVIF_SERVER_PORT:-8080}"
BASE_URL="http://localhost:${EVIF_SERVER_PORT}"

log_section "API Core Endpoints Tests"

# Start server
start_rest_server "$EVIF_SERVER_PORT" || {
    log_error "Failed to start REST server"
    exit 1
}

# Test: GET /health
test_health() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/health")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /health"
    assert_output_contains "$response" "ok" "health response should contain 'ok'"
}

# Test: GET /api/v1/files?path=/
test_files_list_root() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/files?path=/")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/files?path=/"
}

# Test: POST /api/v1/files - create file
test_files_create() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_file.txt","content":"Hello API"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/files (create)"

    # Verify file was created
    response=$(curl -s "${BASE_URL}/api/v1/files?path=/test_file.txt")
    assert_output_contains "$response" "Hello API" "created file should contain content"

    # Cleanup
    curl -s -X DELETE "${BASE_URL}/api/v1/files?path=/test_file.txt" >/dev/null || true
}

# Test: PUT /api/v1/files - update file
test_files_update() {
    # Create file first
    curl -s -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_update.txt","content":"original"}' >/dev/null

    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X PUT "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_update.txt","content":"updated"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "PUT /api/v1/files (update)"

    # Verify file was updated
    response=$(curl -s "${BASE_URL}/api/v1/files?path=/test_update.txt")
    assert_output_contains "$response" "updated" "updated file should have new content"

    # Cleanup
    curl -s -X DELETE "${BASE_URL}/api/v1/files?path=/test_update.txt" >/dev/null || true
}

# Test: GET /api/v1/files?path=/test.txt - read file
test_files_read() {
    # Create file first
    curl -s -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_read.txt","content":"read test"}' >/dev/null

    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/files?path=/test_read.txt")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/files?path=/test_read.txt"
    assert_output_contains "$response" "read test" "response should contain file content"

    # Cleanup
    curl -s -X DELETE "${BASE_URL}/api/v1/files?path=/test_read.txt" >/dev/null || true
}

# Test: DELETE /api/v1/files?path=/test.txt
test_files_delete() {
    # Create file first
    curl -s -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_delete.txt","content":"delete me"}' >/dev/null

    local response status_code

    response=$(curl -s -w "\n%{http_code}" -X DELETE "${BASE_URL}/api/v1/files?path=/test_delete.txt")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "DELETE /api/v1/files?path=/test_delete.txt"

    # Verify file was deleted
    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/files?path=/test_delete.txt")
    status_code=$(echo "$response" | tail -n1)
    assert_http_status 404 $status_code "deleted file should return 404"
}

# Test: GET /api/v1/directories?path=/
test_directories_list() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/directories?path=/")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "GET /api/v1/directories?path=/"
}

# Test: POST /api/v1/directories - create directory
test_directories_create() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/directories" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_dir"}')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "POST /api/v1/directories (create)"

    # Cleanup
    curl -s -X DELETE "${BASE_URL}/api/v1/directories?path=/test_dir" >/dev/null || true
}

# Test: DELETE /api/v1/directories?path=/test_dir
test_directories_delete() {
    # Create directory first
    curl -s -X POST "${BASE_URL}/api/v1/directories" \
        -H "Content-Type: application/json" \
        -d '{"path":"/test_dir_delete"}' >/dev/null

    local response status_code

    response=$(curl -s -w "\n%{http_code}" -X DELETE "${BASE_URL}/api/v1/directories?path=/test_dir_delete")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 200 $status_code "DELETE /api/v1/directories?path=/test_dir_delete"
}

# Test: GET /api/v1/files?path=/nonexistent - file not found
test_file_not_found() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/v1/files?path=/nonexistent_file_xyz")
    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 404 $status_code "GET /api/v1/files?path=/nonexistent should return 404"
    assert_json_field "$response" "error" "404 response should have error field"
}

# Test: POST /api/v1/files with invalid JSON
test_invalid_json() {
    local response status_code

    response=$(curl -s -w "\n%{http_code}" \
        -X POST "${BASE_URL}/api/v1/files" \
        -H "Content-Type: application/json" \
        -d '{invalid json')

    status_code=$(echo "$response" | tail -n1)
    response=$(echo "$response" | head -n-1)

    assert_http_status 400 $status_code "invalid JSON should return 400"
}

# Run all tests
main() {
    log_info "Starting API Core Endpoints Tests..."

    run_test "GET /health" test_health
    run_test "GET /api/v1/files?path=/" test_files_list_root
    run_test "POST /api/v1/files (create)" test_files_create
    run_test "PUT /api/v1/files (update)" test_files_update
    run_test "GET /api/v1/files?path=/test.txt (read)" test_files_read
    run_test "DELETE /api/v1/files?path=/test.txt" test_files_delete
    run_test "GET /api/v1/directories?path=/" test_directories_list
    run_test "POST /api/v1/directories (create)" test_directories_create
    run_test "DELETE /api/v1/directories?path=/test_dir" test_directories_delete
    run_test "GET /api/v1/files?path=/nonexistent (404)" test_file_not_found
    run_test "POST /api/v1/files with invalid JSON (400)" test_invalid_json

    print_test_summary
}

main "$@"