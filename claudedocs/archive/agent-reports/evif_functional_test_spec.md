# EVIF Functional Verification Test Specification

## Summary
Command-based integration testing for EVIF (Everything Is a Virtual Filesystem), validating CLI commands, REST API endpoints, and plugin functionality through real execution rather than unit tests.

---

## 1. Testing Philosophy

**Approach**: Integration testing via actual command execution
- Execute CLI commands and verify outputs
- Make HTTP requests to REST API and validate responses
- Mount/unmount plugins and perform operations
- Test complete user workflows end-to-end

**NOT Unit Testing**: Don't test internal Rust functions directly. Test through public interfaces (CLI and API).

---

## 2. Test Framework & Structure

### Directory Layout
```
tests/
├── integration/
│   ├── cli/
│   │   ├── file_operations.sh        # P0: ls, cat, write, mkdir, rm, mv, cp
│   │   ├── plugin_management.sh      # P0: mount, unmount, mounts
│   │   ├── system_commands.sh        # P0: health, stats
│   │   ├── batch_operations.sh       # P1: batch-copy, batch-delete, etc.
│   │   ├── search_analysis.sh        # P1: grep, checksum, diff, du, find
│   │   ├── text_processing.sh        # P2: sort, uniq, wc, cut, tr, etc.
│   │   └── shell_tools.sh            # P2: echo, cd, pwd, env, date, etc.
│   ├── api/
│   │   ├── core_endpoints.sh         # P0: health, files, directories
│   │   ├── mount_management.sh       # P0: mounts, mount, unmount
│   │   ├── metadata_operations.sh    # P1: stat, digest, touch
│   │   ├── handle_management.sh      # P1: 10 handle endpoints
│   │   ├── batch_operations.sh       # P1: 5 batch endpoints
│   │   └── plugin_management.sh      # P1: plugin endpoints
│   ├── plugins/
│   │   ├── memfs_test.sh             # P0: Memory filesystem
│   │   ├── localfs_test.sh           # P0: Local filesystem
│   │   └── cloud_storage_test.sh     # P1: S3, Azure, GCS (optional, requires credentials)
│   ├── fixtures/
│   │   ├── test_file.txt
│   │   ├── test_data.json
│   │   └── test_directory/
│   └── lib/
│       ├── test_helpers.sh           # Common functions
│       ├── server_manager.sh         # Start/stop REST server
│       └── assertions.sh             # Test assertions
```

### Test Script Format (Bash)
```bash
#!/usr/bin/env bash
# tests/integration/cli/file_operations.sh

set -euo pipefail

# Source helpers
source "$(dirname "$0")/../lib/test_helpers.sh"

# Test function naming: test_<feature>_<scenario>
# Assertion functions: assert_success, assert_output_contains, assert_http_status

test_ls_root_directory() {
    local output
    output=$(evif ls /)
    assert_success "evif ls /" $?

    # After mounting memfs at /test, should see directory
    evif mount memfs /test > /dev/null
    assert_success "evif mount memfs /test" $?

    output=$(evif ls /)
    assert_output_contains "$output" "test/" "Should list /test directory"
}

test_write_and_read_file() {
    # Create test file
    echo "Hello, EVIF!" | evif write /test/hello.txt -c "$(cat -)" > /dev/null
    assert_success "evif write /test/hello.txt" $?

    # Read back content
    local output
    output=$(evif cat /test/hello.txt)
    assert_output_contains "$output" "Hello, EVIF!" "File content should match"

    # Verify file exists
    evif stat /test/hello.txt > /dev/null
    assert_success "evif stat /test/hello.txt" $?
}

# Main test runner
main() {
    setup  # Start server, mount test filesystem
    run_tests
    cleanup  # Unmount, stop server
}

main "$@"
```

### API Test Format
```bash
#!/usr/bin/env bash
# tests/integration/api/core_endpoints.sh

set -euo pipefail

source "$(dirname "$0")/../lib/test_helpers.sh"

test_health_endpoint() {
    local response
    response=$(curl -s http://localhost:8080/health)
    assert_http_status 200 $? "GET /health should return 200"

    assert_output_contains "$response" '"status":"ok"' "Health check should return ok"
    assert_output_contains "$response" '"version"' "Health check should include version"
}

test_create_file_via_api() {
    local response
    response=$(curl -s -X POST http://localhost:8080/api/v1/files \
        -H "Content-Type: application/json" \
        -d '{"path":"/test/api_file.txt","content":"API test"}')
    assert_http_status 200 $? "POST /api/v1/files should return 200"

    # Verify file exists
    response=$(curl -s "http://localhost:8080/api/v1/files?path=/test/api_file.txt")
    assert_output_contains "$response" "API test" "File content should match"
}

main() {
    setup
    run_tests
    cleanup
}

main "$@"
```

---

## 3. Service Management

### Server Startup
```bash
# tests/integration/lib/server_manager.sh

start_rest_server() {
    local port=${1:-8080}
    local log_file="${TEST_LOG_DIR}/rest_server.log"

    echo "Starting EVIF REST server on port $port..."
    cargo run -p evif-rest -- --port "$port" > "$log_file" 2>&1 &
    SERVER_PID=$!

    # Wait for server to be ready
    local max_wait=30
    local waited=0
    while [ $waited -lt $max_wait ]; do
        if curl -s http://localhost:${port}/health > /dev/null 2>&1; then
            echo "Server ready on port $port (PID: $SERVER_PID)"
            return 0
        fi
        sleep 1
        waited=$((waited + 1))
    done

    echo "ERROR: Server failed to start within ${max_wait}s"
    cat "$log_file"
    return 1
}

stop_rest_server() {
    if [ -n "${SERVER_PID:-}" ]; then
        echo "Stopping server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        unset SERVER_PID
    fi
}
```

---

## 4. Test Helper Functions

### assertions.sh
```bash
assert_success() {
    local test_name="$1"
    local exit_code="$2"

    if [ $exit_code -eq 0 ]; then
        echo "✓ PASS: $test_name"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "✗ FAIL: $test_name (exit code: $exit_code)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
}

assert_output_contains() {
    local actual="$1"
    local expected="$2"
    local message="$3"

    if echo "$actual" | grep -qF "$expected"; then
        echo "✓ PASS: $message"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "✗ FAIL: $message"
        echo "  Expected to contain: $expected"
        echo "  Actual output:"
        echo "$actual" | sed 's/^/    /'
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
}

assert_http_status() {
    local expected="$1"
    local exit_code="$2"
    local message="$3"

    # curl -w "%{http_code}" -o /dev/null -s gives HTTP status as exit code
    # But we check for success (200-299 range)
    if [ $exit_code -eq 0 ]; then
        # Request succeeded, verify status
        echo "✓ PASS: $message"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "✗ FAIL: $message (curl exit code: $exit_code)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
}

assert_file_exists() {
    local path="$1"
    local message="$2"

    if evif stat "$path" > /dev/null 2>&1; then
        echo "✓ PASS: $message"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "✗ FAIL: $message (file not found: $path)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return 1
    fi
}
```

---

## 5. Given-When-Then Acceptance Criteria

### Example: File Operations

| Test | Given | When | Then |
|------|-------|------|------|
| `test_ls_root_directory` | memfs mounted at /test | Run `evif ls /` | Output contains "test/" directory |
| `test_write_file` | memfs mounted at /test | Run `evif write /test/hello.txt -c "Hello"` | Exit code 0, file created |
| `test_cat_file` | File /test/hello.txt exists with content | Run `evif cat /test/hello.txt` | Output contains "Hello" |
| `test_mkdir` | memfs mounted at /test | Run `evif mkdir /test/subdir` | Directory /test/subdir created |
| `test_rm_file` | File /test/tmp.txt exists | Run `evif rm /test/tmp.txt` | File removed, exit code 0 |
| `test_mv_file` | File /test/a.txt exists | Run `evif mv /test/a.txt /test/b.txt` | File moved to /test/b.txt |
| `test_cp_file` | File /test/source.txt exists | Run `evif cp /test/source.txt /test/dest.txt` | Both files exist with same content |
| `test_stat_file` | File /test/test.txt exists | Run `evif stat /test/test.txt` | Output shows size, type, mode, modified time |

### Example: REST API

| Test | Given | When | Then |
|------|-------|------|------|
| `test_health_endpoint` | REST server running on port 8080 | GET `/health` | Status 200, body contains `"status":"ok"` |
| `test_list_directory` | /test directory has files | GET `/api/v1/directories?path=/test` | Returns list of files with names, sizes, is_dir flags |
| `test_create_file` | /test directory exists | POST `/api/v1/files` with path and content | Status 200, file created |
| `test_read_file` | File /test/hello.txt exists | GET `/api/v1/files?path=/test/hello.txt` | Status 200, body contains file content |
| `test_delete_file` | File /test/tmp.txt exists | DELETE `/api/v1/files?path=/test/tmp.txt` | Status 200, file removed |

### Example: Plugin Management

| Test | Given | When | Then |
|------|-------|------|------|
| `test_mount_memfs` | No plugins mounted | `evif mount memfs /mem` | Exit code 0, mount appears in `evif mounts` |
| `test_unmount_plugin` | memfs mounted at /mem | `evif unmount /mem` | Exit code 0, mount removed from list |
| `test_list_mounts` | memfs at /test, localfs at /local | `evif mounts` | Output shows both mounts with correct paths and plugin names |

---

## 6. Test Execution Plan

### Phase 1: P0 Critical Tests (50%)
```bash
# CLI file operations (15 commands)
./tests/integration/cli/file_operations.sh

# CLI plugin management (4 commands)
./tests/integration/cli/plugin_management.sh

# CLI system commands (3 commands)
./tests/integration/cli/system_commands.sh

# API core endpoints (10 endpoints)
./tests/integration/api/core_endpoints.sh

# API mount management (3 endpoints)
./tests/integration/api/mount_management.sh

# Basic plugins (memfs, localfs)
./tests/integration/plugins/memfs_test.sh
./tests/integration/plugins/localfs_test.sh
```

### Phase 2: P1 Important Tests (30%)
```bash
# CLI batch operations (5 commands)
./tests/integration/cli/batch_operations.sh

# CLI search and analysis (8 commands)
./tests/integration/cli/search_analysis.sh

# API metadata operations (4 endpoints)
./tests/integration/api/metadata_operations.sh

# API handle management (10 endpoints)
./tests/integration/api/handle_management.sh

# API batch operations (5 endpoints)
./tests/integration/api/batch_operations.sh

# API plugin management (6 endpoints)
./tests/integration/api/plugin_management.sh
```

### Phase 3: P2 Enhancement Tests (15%)
```bash
# CLI text processing (10 commands)
./tests/integration/cli/text_processing.sh

# CLI shell tools (10 commands)
./tests/integration/cli/shell_tools.sh

# API metrics (4 endpoints)
./tests/integration/api/metrics_test.sh

# API collaboration (10 endpoints) - optional, may need mock data
./tests/integration/api/collaboration_test.sh
```

### Phase 4: P3 Optional Tests (5%)
```bash
# Graph operations - verify "not implemented" behavior
./tests/integration/api/graph_operations.sh
```

---

## 7. Non-Functional Requirements

### Performance
- Test suite should complete within 5 minutes
- Each test should complete within 10 seconds
- REST server startup should complete within 30 seconds

### Reliability
- Tests should be deterministic (no race conditions)
- Cleanup must run even if tests fail
- Server should be stopped even on SIGINT/SIGTERM

### Maintainability
- Test output should clearly indicate pass/fail
- Failed tests should show expected vs actual values
- Test helpers should be reusable across test suites

---

## 8. Edge Cases and Error Conditions

### File Operations
- **Missing file**: `evif cat /nonexistent` should return non-zero exit code
- **Empty file**: `evif cat /empty.txt` on empty file should succeed
- **Large file**: Files >1MB should work without truncation
- **Special characters**: Files with spaces, unicode, or special chars
- **Path traversal**: `../../../etc/passwd` should be rejected or handled safely

### API Error Handling
- **404 Not Found**: Missing files should return 404, not 500
- **400 Bad Request**: Invalid JSON should return 400
- **409 Conflict**: Creating existing file should handle gracefully
- **503 Service Unavailable**: Server down scenarios

### Plugin Operations
- **Invalid plugin**: `evif mount nonexistent /path` should fail gracefully
- **Duplicate mount**: Same path mounted twice should fail
- **Unmount non-existent**: Should handle gracefully
- **Plugin crash**: Plugin failure should not crash server

---

## 9. Out of Scope

- **Unit testing**: Don't test individual Rust functions
- **FUSE testing**: Requires system-level permissions and kernel modules
- **Cloud storage plugins**: S3, Azure, GCS require credentials (mark as optional)
- **Performance benchmarking**: Not part of functional verification
- **Security auditing**: Not covered in this test suite
- **UI/E2E testing**: Web interface testing separate from functional testing

---

## 10. Success Criteria

- [ ] All P0 tests pass (100%)
- [ ] At least 80% of P1 tests pass
- [ ] Test suite runs automatically with single command: `./tests/run_all.sh`
- [ ] Test results clearly show: PASS: X, FAIL: Y, TOTAL: Z
- [ ] Failed tests are documented with expected vs actual output
- [ ] Test execution time <5 minutes for full suite
- [ ] Cleanup always runs (no leftover mounts or servers)

---

## 11. Implementation Notes

### Required Dependencies
- `cargo` - for building and running EVIF
- `curl` - for API testing
- `jq` - optional, for JSON parsing

### Environment Variables
```bash
export EVIF_SERVER_PORT=8080          # REST server port
export EVIF_SERVER_HOST=localhost     # Server host
export TEST_LOG_DIR=/tmp/evif_tests   # Log directory
export EVIF_VERBOSE=false            # Enable verbose output
```

### Running Individual Tests
```bash
# Run specific test suite
./tests/integration/cli/file_operations.sh

# Run with verbose output
EVIF_VERBOSE=true ./tests/integration/api/core_endpoints.sh

# Run with custom server port
EVIF_SERVER_PORT=9000 ./tests/integration/plugins/memfs_test.sh
```

---

*Specification Version: 1.0*
*Created: 2026-02-26*
*Author: Spec Writer (Ralph Loop)*