# EVIF Functional Verification Test Specification (v2.0)

## Summary
Command-based integration testing for EVIF (Everything Is a Virtual Filesystem), validating CLI commands, REST API endpoints, and plugin functionality through real execution rather than unit tests.

---

## 1. Test Data Specifications

### 1.1 Test Fixtures Directory Structure
```
tests/fixtures/
├── text/
│   ├── small.txt              # 10 bytes: "Hello EVIF\n"
│   ├── medium.txt             # 1 KB: Lorem ipsum repeated
│   ├── large.txt              # 1 MB: Random bytes
│   └── empty.txt              # 0 bytes
├── json/
│   ├── config.json            # {"name":"test","version":"1.0"}
│   └── data.json              # Array of 100 objects
├── binary/
│   └── image.png              # 1 KB PNG header + random data
├── unicode/
│   └── mixed.txt              # "中文日本語🎉emoji test"
└── special/
    ├── spaces in name.txt     # File with spaces
    └── "quote'test".txt       # Quote characters
```

### 1.2 Test Data Generation Commands
```bash
# Generate test files
echo "Hello EVIF" > tests/fixtures/text/small.txt
dd if=/dev/urandom bs=1024 count=1 of=tests/fixtures/text/medium.txt
dd if=/dev/urandom bs=1048576 count=1 of=tests/fixtures/text/large.txt
touch tests/fixtures/text/empty.txt

# Create JSON fixtures
echo '{"name":"test","version":"1.0","enabled":true}' > tests/fixtures/json/config.json
jq -n '[range(100)|{id:.,name:"item\(. )",value:.}]' > tests/fixtures/json/data.json
```

---

## 2. Error Code/Message Mappings

### 2.1 CLI Exit Codes
| Exit Code | Meaning | Test Validation |
|-----------|---------|-----------------|
| 0 | Success | Command executed without error |
| 1 | General error | Invalid arguments, file not found |
| 2 | Misuse of command | Missing required arguments |
| 126 | Command not executable | Plugin not found |
| 127 | Command not found | Unknown subcommand |

### 2.2 API HTTP Status Codes
| Status Code | Meaning | Error Message Pattern | Test Validation |
|-------------|---------|----------------------|-----------------|
| 200 | OK | N/A | Success response |
| 400 | Bad Request | "invalid.*path\|missing.*parameter" | Invalid input |
| 401 | Unauthorized | "unauthorized\|authentication.*required" | Missing auth token |
| 403 | Forbidden | "permission denied\|access denied" | Insufficient permissions |
| 404 | Not Found | "file.*not.*found\|path.*not.*exist" | Missing resource |
| 409 | Conflict | "already.*exists\|conflict" | Duplicate resource |
| 422 | Unprocessable | "invalid.*format\|parse.*error" | Malformed request |
| 500 | Internal Error | "internal.*error\|unexpected" | Server error |
| 503 | Service Unavailable | "unavailable\|starting" | Server not ready |

### 2.3 Error Response Format
```json
{
  "error": {
    "code": "FILE_NOT_FOUND",
    "message": "File '/nonexistent.txt' does not exist",
    "details": {
      "path": "/nonexistent.txt",
      "operation": "read"
    }
  }
}
```

### 2.4 Given-When-Then with Error Cases
| Test | Given | When | Then |
|------|-------|------|------|
| `test_cat_missing_file` | No file at /test/missing.txt | Run `evif cat /test/missing.txt` | Exit code 1, stderr contains "not found" |
| `test_api_file_not_found` | No file at /missing | GET `/api/v1/files?path=/missing` | HTTP 404, error.code = "FILE_NOT_FOUND" |
| `test_api_invalid_json` | Server running | POST `/api/v1/files` with `{invalid` | HTTP 400, error.code = "INVALID_REQUEST" |
| `test_mount_invalid_plugin` | No plugin "fakefs" | `evif mount fakefs /fake` | Exit code 126, stderr contains "not found" |
| `test_unmount_nonexistent` | Nothing at /nonexistent | `evif unmount /nonexistent` | Exit code 1, stderr contains "not mounted" |

---

## 3. Performance Test Methodology

### 3.1 Performance Test Scripts
```bash
# tests/performance/benchmarks.sh

#!/usr/bin/env bash
set -euo pipefail

export EVIF_SERVER_PORT=8080
source "$(dirname "$0")/../lib/server_manager.sh"

# Timing helper
time_cmd() {
    local start end elapsed
    start=$(date +%s.%N)
    "$@"
    end=$(date +%s.%N)
    elapsed=$(echo "$end - $start" | bc)
    echo "$elapsed"
}

# API Latency Tests
test_api_latency() {
    echo "=== API Latency Tests ==="

    # Warmup
    curl -s http://localhost:8080/health > /dev/null

    # Measure 10 requests, calculate P50/P95/P99
    local times=()
    for i in {1..10}; do
        times+=($(time_cmd curl -s http://localhost:8080/health > /dev/null))
    done

    # Calculate percentiles (simplified)
    echo "P50: $(echo "${times[4]}" | bc)ms"
    echo "P95: $(echo "${times[8]}" | bc)ms"
    echo "P99: $(echo "${times[9]}" | bc)ms"

    # PASS/FAIL criteria
    # P99 must be < 100ms
    local p99="${times[9]}"
    if (( $(echo "$p99 < 0.1" | bc -l) )); then
        echo "✓ PASS: API P99 latency < 100ms"
    else
        echo "✗ FAIL: API P99 latency ${p99}s >= 100ms"
    fi
}

# File Operation Throughput
test_file_throughput() {
    echo "=== File Operation Throughput ==="

    local count=100
    local start end ops_per_sec

    start=$(date +%s)
    for i in $(seq 1 $count); do
        curl -s -X POST "http://localhost:8080/api/v1/files" \
            -H "Content-Type: application/json" \
            -d "{\"path\":\"/perf/test${i}.txt\",\"content\":\"test\"}" > /dev/null
    done
    end=$(date +%s)

    ops_per_sec=$(echo "scale=2; $count / ($end - $start)" | bc)
    echo "Throughput: $ops_per_sec ops/sec"

    # PASS/FAIL criteria
    # Must achieve > 10 ops/sec
    if (( $(echo "$ops_per_sec > 10" | bc -l) )); then
        echo "✓ PASS: Throughput > 10 ops/sec"
    else
        echo "✗ FAIL: Throughput ${ops_per_sec} <= 10 ops/sec"
    fi
}

# Server Startup Time
test_server_startup() {
    echo "=== Server Startup Time ==="

    local start end elapsed
    start=$(date +%s.%N)

    cargo run -p evif-rest -- --port 8081 > /tmp/evif_startup.log 2>&1 &
    local pid=$!

    while ! curl -s http://localhost:8081/health > /dev/null 2>&1; do
        sleep 0.5
        if ! kill -0 $pid 2>/dev/null; then
            echo "✗ FAIL: Server failed to start"
            return 1
        fi
    done

    end=$(date +%s.%N)
    elapsed=$(echo "$end - $start" | bc)
    echo "Startup time: ${elapsed}s"

    kill $pid 2>/dev/null || true

    # PASS/FAIL criteria
    # Must start within 30 seconds
    if (( $(echo "$elapsed < 30" | bc -l) )); then
        echo "✓ PASS: Server startup < 30s"
    else
        echo "✗ FAIL: Server startup ${elapsed}s >= 30s"
    fi
}

main() {
    start_rest_server 8080
    test_api_latency
    test_file_throughput
    stop_rest_server
}
```

### 3.2 Performance Acceptance Criteria
| Metric | Target | Measurement Method | Pass Criteria |
|--------|--------|-------------------|---------------|
| API P99 Latency | < 100ms | 10 sequential requests | P99 calculated from times |
| File Write Throughput | > 10 ops/sec | 100 sequential writes | Operations / elapsed time |
| Server Startup | < 30 seconds | Time to /health responds | Wall clock time |
| Test Suite Runtime | < 5 minutes | `time ./tests/run_all.sh` | Total execution time |
| Memory Usage | < 500 MB | `ps -o rss` during tests | Peak RSS |

---

## 4. Cloud Storage Test Setup

### 4.1 Prerequisites for Cloud Tests
```bash
# Cloud storage tests are OPTIONAL and REQUIRE credentials
# They are skipped by default unless explicitly enabled

export EVIF_TEST_CLOUD=1              # Enable cloud tests
export AWS_ACCESS_KEY_ID=...          # AWS credentials
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=us-east-1
export AWS_S3_BUCKET=evif-test-bucket

export AZURE_STORAGE_ACCOUNT=...
export AZURE_STORAGE_KEY=...
export AZURE_CONTAINER=evif-test

export GOOGLE_APPLICATION_CREDENTIALS=/path/to/creds.json
export GCS_BUCKET=evif-test-bucket
```

### 4.2 Cloud Test Skeleton
```bash
# tests/integration/plugins/cloud_storage_test.sh

#!/usr/bin/env bash
set -euo pipefail

SKIP_REASON=""

# Skip if cloud tests not enabled
if [ -z "${EVIF_TEST_CLOUD:-}" ]; then
    SKIP_REASON="Cloud tests disabled (set EVIF_TEST_CLOUD=1 to enable)"
    echo "⊘ SKIP: $SKIP_REASON"
    exit 0
fi

# Check for required credentials
check_aws_credentials() {
    if [ -z "${AWS_ACCESS_KEY_ID:-}" ] || [ -z "${AWS_SECRET_ACCESS_KEY:-}" ]; then
        echo "⊘ SKIP: AWS credentials not set"
        exit 0
    fi
}

source "$(dirname "$0")/../lib/test_helpers.sh"

test_s3fs_mount() {
    check_aws_credentials

    # Create bucket if not exists (requires AWS CLI)
    aws s3 mb s3://${AWS_S3_BUCKET} 2>/dev/null || true

    # Mount S3 plugin
    local response
    response=$(curl -s -X POST http://localhost:8080/api/v1/mount \
        -H "Content-Type: application/json" \
        -d "{
            \"plugin\": \"s3fs\",
            \"path\": \"/s3\",
            \"config\": {
                \"bucket\": \"${AWS_S3_BUCKET}\",
                \"region\": \"${AWS_REGION}\"
            }
        }")

    # Verify mount succeeded
    assert_http_status 200 $? "S3 mount should succeed"

    # Write and read test
    curl -s -X PUT http://localhost:8080/api/v1/files \
        -H "Content-Type: application/json" \
        -d '{"path":"/s3/test.txt","content":"S3 test content"}' > /dev/null

    local read_response
    read_response=$(curl -s "http://localhost:8080/api/v1/files?path=/s3/test.txt")
    assert_output_contains "$read_response" "S3 test content" "S3 read should return written content"

    # Cleanup
    curl -s -X POST http://localhost:8080/api/v1/unmount \
        -H "Content-Type: application/json" \
        -d '{"path":"/s3"}' > /dev/null

    echo "✓ PASS: S3FS mount/read/write/cleanup"
}

test_s3fs_with_minio() {
    # Alternative: Use MinIO container for local testing
    # Requires: docker run -d -p 9000:9000 -p 9001:9000 minio/minio server /data --console-address ":9001"

    if ! curl -s http://localhost:9000/minio/health/live > /dev/null 2>&1; then
        echo "⊘ SKIP: MinIO not running at localhost:9000"
        exit 0
    fi

    # Use MinIO as S3-compatible endpoint
    export AWS_ACCESS_KEY_ID=minioadmin
    export AWS_SECRET_ACCESS_KEY=minioadmin
    export AWS_S3_BUCKET=evif-test
    # Point to local MinIO
    export AWS_S3_ENDPOINT=http://localhost:9000
    export AWS_S3_PATH_STYLE=true

    # Run same S3 tests with MinIO endpoint
    test_s3fs_mount
}
```

### 4.3 Test Execution Matrix
| Cloud Provider | Prerequisites | Default Behavior |
|----------------|---------------|------------------|
| AWS S3 | AWS credentials env vars | SKIP (opt-in) |
| Azure Blob | Azure credentials env vars | SKIP (opt-in) |
| GCS | Google credentials JSON | SKIP (opt-in) |
| MinIO | Docker container running | SKIP (if no container) |
| All | No credentials | SKIP gracefully |

---

## 5. FUSE Dependency Documentation

### 5.1 FUSE Testing Prerequisites
```bash
# FUSE testing requires:
# 1. FUSE kernel module loaded
# 2. fusermount utility installed
# 3. Appropriate user permissions

# Check FUSE availability
check_fuse() {
    if ! modprobe fuse 2>/dev/null && [ ! -e /dev/fuse ]; then
        echo "FUSE not available: /dev/fuse not found"
        return 1
    fi
    if ! command -v fusermount >/dev/null 2>&1; then
        echo "FUSE not available: fusermount not found"
        return 1
    fi
    return 0
}

# Check user can mount FUSE
check_fuse_permissions() {
    if ! fusermount -V >/dev/null 2>&1; then
        echo "FUSE permission denied: user not in 'fuse' group"
        return 1
    fi
    return 0
}
```

### 5.2 FUSE Test Specification (Out of Scope)
```markdown
## FUSE Testing - SEPARATE SPECIFICATION

FUSE testing is OUT OF SCOPE for this functional test plan due to:
1. Requires root/kernel-level permissions
2. Different test infrastructure needed
3. Platform-specific (Linux/FreeBSD/macOS FUSE differences)

If FUSE testing is required, create separate spec: `evif_fuse_test_spec.md`
```

---

## 6. Measurable Acceptance Criteria

### 6.1 CLI Command Tests (P0)
| Command | Test Scenario | Input | Expected Output | Exit Code |
|---------|--------------|-------|-----------------|-----------|
| `ls /` | List root | `evif ls /` | Contains mount points | 0 |
| `ls -l /test` | Long format | `evif ls -l /test` | Shows size, date, permissions | 0 |
| `ls -r /test` | Recursive | `evif ls -r /test` | Lists all nested files | 0 |
| `cat /test/file.txt` | Read file | `evif cat /test/file.txt` | Exact file content | 0 |
| `write /test/new.txt -c "hi"` | Write file | `evif write /test/new.txt -c "hi"` | File created | 0 |
| `write /test/append.txt -c "a" -a` | Append | `evif write /test/append.txt -c "b" -a` | Content "ab" | 0 |
| `mkdir /test/dir` | Make dir | `evif mkdir /test/dir` | Directory created | 0 |
| `mkdir -p /a/b/c` | Recursive mkdir | `evif mkdir -p /a/b/c` | All dirs created | 0 |
| `rm /test/file.txt` | Remove file | `evif rm /test/file.txt` | File removed | 0 |
| `rm -r /test/dir` | Recursive rm | `evif rm -r /test/dir` | Dir and contents removed | 0 |
| `mv /a /b` | Move/rename | `evif mv /test/a /test/b` | File at new location | 0 |
| `cp /src /dst` | Copy | `evif cp /test/src /test/dst` | Both files exist | 0 |
| `stat /test/file` | File stats | `evif stat /test/file.txt` | Shows size, mtime, mode | 0 |
| `touch /test/empty` | Touch file | `evif touch /test/empty` | Empty file created | 0 |
| `head -n 5 /test/file` | Head lines | `evif head -n 5 /test/file` | First 5 lines | 0 |
| `tail -n 5 /test/file` | Tail lines | `evif tail -n 5 /test/file` | Last 5 lines | 0 |
| `tree -d 2 /test` | Tree view | `evif tree -d 2 /test` | 2-level directory tree | 0 |

### 6.2 Plugin Management Tests (P0)
| Command | Test Scenario | Input | Expected Output | Exit Code |
|---------|--------------|-------|-----------------|-----------|
| `mount memfs /mem` | Mount memfs | `evif mount memfs /mem` | Mount succeeds | 0 |
| `mount memfs /mem -c '{"size":1000}'` | Mount with config | `evif mount memfs /mem -c '{"size":1000}'` | Mount with config | 0 |
| `mounts` | List mounts | `evif mounts` | Shows all mounts | 0 |
| `unmount /mem` | Unmount | `evif unmount /mem` | Unmount succeeds | 0 |

### 6.3 System Commands Tests (P0)
| Command | Test Scenario | Input | Expected Output | Exit Code |
|---------|--------------|-------|-----------------|-----------|
| `health` | Health check | `evif health` | Shows status, version | 0 |
| `stats` | Statistics | `evif stats` | Shows connection info | 0 |

### 6.4 REST API Tests (P0)
| Endpoint | Method | Test Input | Expected Response | Status |
|----------|--------|------------|-------------------|--------|
| `/health` | GET | - | `{"status":"ok","version":"...","uptime":...}` | 200 |
| `/api/v1/files` | GET | `?path=/test` | File list or content | 200 |
| `/api/v1/files` | POST | `{"path":"/test.txt","content":"hi"}` | Created file | 200 |
| `/api/v1/files` | PUT | `{"path":"/test.txt","content":"updated"}` | Updated file | 200 |
| `/api/v1/files` | DELETE | `?path=/test.txt` | Deleted | 200 |
| `/api/v1/directories` | GET | `?path=/` | Directory listing | 200 |
| `/api/v1/directories` | POST | `{"path":"/newdir"}` | Created directory | 200 |
| `/api/v1/directories` | DELETE | `?path=/dir` | Deleted directory | 200 |
| `/api/v1/mounts` | GET | - | Mount list | 200 |
| `/api/v1/mount` | POST | `{"plugin":"memfs","path":"/m"}` | Mounted | 200 |
| `/api/v1/unmount` | POST | `{"path":"/m"}` | Unmounted | 200 |

### 6.5 Handle Management Tests (P1)
| Endpoint | Method | Test Input | Expected Response | Status |
|----------|--------|------------|-------------------|--------|
| `/api/v1/handles/open` | POST | `{"path":"/test.txt","mode":"r"}` | Handle ID | 200 |
| `/api/v1/handles/:id` | GET | - | Handle details | 200 |
| `/api/v1/handles/:id/read` | POST | `{"offset":0,"length":100}` | Data | 200 |
| `/api/v1/handles/:id/write` | POST | `{"data":"test","offset":0}` | Bytes written | 200 |
| `/api/v1/handles/:id/seek` | POST | `{"position":10,"whence":0}` | New position | 200 |
| `/api/v1/handles/:id/sync` | POST | - | Synced | 200 |
| `/api/v1/handles/:id/close` | POST | - | Closed | 200 |
| `/api/v1/handles/:id/renew` | POST | - | TTL extended | 200 |
| `/api/v1/handles` | GET | - | Handle list | 200 |
| `/api/v1/handles/stats` | GET | - | Handle statistics | 200 |

---

## 7. Test Framework & Structure

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
│   │   └── cloud_storage_test.sh     # P1: S3, Azure, GCS (optional)
│   ├── fixtures/                     # Test data (see Section 1)
│   ├── lib/
│   │   ├── test_helpers.sh           # Common functions
│   │   ├── server_manager.sh         # Start/stop REST server
│   │   └── assertions.sh             # Test assertions
│   └── performance/
│       └── benchmarks.sh             # Performance tests
├── run_all.sh                        # Master test runner
└── README.md                         # Test documentation
```

---

## 8. Non-Functional Requirements

### Performance
- Test suite must complete within 5 minutes
- Each test must complete within 10 seconds
- REST server startup must complete within 30 seconds

### Reliability
- Tests must be deterministic (no race conditions)
- Cleanup must run even if tests fail
- Server must be stopped even on SIGINT/SIGTERM

### Maintainability
- Test output must clearly indicate pass/fail
- Failed tests must show expected vs actual values
- Test helpers must be reusable across test suites

---

## 9. Out of Scope

- **Unit testing**: Don't test individual Rust functions
- **FUSE testing**: Requires system-level permissions and kernel modules (separate spec)
- **Cloud storage plugins**: S3, Azure, GCS require credentials (opt-in, skip gracefully)
- **Security auditing**: Not covered in this test suite
- **UI/E2E testing**: Web interface testing separate from functional testing

---

## 10. Success Criteria

- [ ] All P0 tests pass (100%)
- [ ] At least 80% of P1 tests pass
- [ ] All error cases return correct HTTP status codes (400/404/500 as specified)
- [ ] Performance benchmarks meet targets (P99 < 100ms, throughput > 10 ops/sec)
- [ ] Test suite runs automatically with single command: `./tests/run_all.sh`
- [ ] Test results clearly show: PASS: X, FAIL: Y, TOTAL: Z
- [ ] Failed tests are documented with expected vs actual output
- [ ] Test execution time < 5 minutes for full suite
- [ ] Cleanup always runs (no leftover mounts or servers)
- [ ] Cloud tests skip gracefully when credentials not provided

---

## 11. Environment Variables

```bash
export EVIF_SERVER_PORT=8080          # REST server port (default: 8080)
export EVIF_SERVER_HOST=localhost     # Server host (default: localhost)
export TEST_LOG_DIR=/tmp/evif_tests   # Log directory
export EVIF_VERBOSE=false            # Enable verbose output

# Optional: Cloud testing
export EVIF_TEST_CLOUD=1              # Enable cloud tests (default: disabled)
export AWS_ACCESS_KEY_ID=...          # AWS credentials
export AWS_SECRET_ACCESS_KEY=...
export AZURE_STORAGE_ACCOUNT=...      # Azure credentials
export GOOGLE_APPLICATION_CREDENTIALS=...  # GCS credentials
```

---

*Specification Version: 2.0*
*Created: 2026-02-26*
*Author: Spec Writer (Ralph Loop)*
*Addresses spec.rejected issues: measurable criteria, error mappings, performance methodology, cloud setup, test data, FUSE docs*
