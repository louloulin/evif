# EVIF Functional Integration Tests

Command-based integration testing for EVIF (Everything Is a Virtual Filesystem).

## Overview

This test suite validates EVIF functionality through real command execution rather than unit tests. Tests cover:

- **P0 (Critical)**: CLI file operations, plugin management, system commands, core API endpoints
- **P1 (Important)**: Handle management, batch operations, performance benchmarks
- **P2 (Enhanced)**: Cloud storage (optional), advanced plugins (not yet implemented)

## Quick Start

```bash
# Run all tests
./tests/run_all.sh

# Run specific test suite
./tests/integration/cli/file_operations.sh
./tests/integration/api/core_endpoints.sh
./tests/performance/benchmarks.sh

# Verbose output
./tests/run_all.sh -v
```

## Test Structure

```
tests/
├── run_all.sh                      # Master test runner
├── README.md                       # This file
├── fixtures/                       # Test data
│   ├── text/                       # Text files (small, medium, large, empty)
│   ├── json/                       # JSON files (config, data)
│   ├── binary/                     # Binary files (PNG header)
│   ├── unicode/                    # Unicode test files
│   └── special/                    # Special characters (spaces, quotes)
├── lib/                            # Test helper libraries
│   ├── test_helpers.sh            # Common test functions
│   ├── assertions.sh              # Assertion functions
│   └── server_manager.sh          # REST server management
├── integration/
│   ├── cli/                       # CLI tests
│   │   ├── file_operations.sh     # P0: ls, cat, write, mkdir, rm, mv, cp, etc.
│   │   ├── plugin_management.sh   # P0: mount, unmount, mounts
│   │   └── system_commands.sh     # P0: health, stats
│   └── api/                       # API tests
│       ├── core_endpoints.sh      # P0: /health, /api/v1/files, /api/v1/directories
│       ├── mount_management.sh    # P0: /api/v1/mounts, /api/v1/mount, /api/v1/unmount
│       └── handle_management.sh   # P1: Handle operations (open, read, write, close, etc.)
└── performance/
    └── benchmarks.sh              # Performance tests (latency, throughput, startup)
```

## Environment Variables

```bash
export EVIF_SERVER_PORT=8080          # REST server port (default: 8080)
export EVIF_SERVER_HOST=localhost     # Server host (default: localhost)
export TEST_LOG_DIR=/tmp/evif_tests   # Log directory
export EVIF_VERBOSE=false            # Enable verbose output

# Optional: Cloud testing (opt-in, tests skip gracefully without credentials)
export EVIF_TEST_CLOUD=1              # Enable cloud tests
export AWS_ACCESS_KEY_ID=...          # AWS credentials
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=us-east-1
export AWS_S3_BUCKET=evif-test-bucket
```

## Prerequisites

- **Rust 1.70+** and **cargo** toolchain
- **curl** for API tests
- **jq** for JSON parsing (optional but recommended)
- **bc** for performance calculations
- **bash** 4.0+ for test scripts

### Install dependencies

```bash
# macOS
brew install curl jq bc

# Linux (Debian/Ubuntu)
sudo apt-get install curl jq bc
```

## Test Categories

### P0 CLI Tests (Critical)

| Test Suite | Commands | Test Count |
|-----------|----------|-----------|
| file_operations.sh | ls, cat, write, mkdir, rm, mv, cp, stat, touch, head, tail, tree | 17 |
| plugin_management.sh | mount, mounts, unmount | 6 |
| system_commands.sh | health, stats | 2 |

**Total P0 CLI Tests**: 25

### P0 API Tests (Critical)

| Test Suite | Endpoints | Test Count |
|-----------|----------|-----------|
| core_endpoints.sh | /health, /api/v1/files, /api/v1/directories | 11 |
| mount_management.sh | /api/v1/mounts, /api/v1/mount, /api/v1/unmount | 6 |

**Total P0 API Tests**: 17

### P1 Tests (Important)

| Test Suite | Endpoints | Test Count |
|-----------|----------|-----------|
| handle_management.sh | /api/v1/handles/* (10 endpoints) | 10 |

**Total P1 Tests**: 10

### Performance Tests

| Test Suite | Metrics | Targets |
|-----------|---------|---------|
| benchmarks.sh | API P99 latency, throughput, startup time, suite runtime | P99 < 100ms, > 10 ops/sec, startup < 30s |

## Test Output Interpretation

### Individual Test Output

Each test script produces output like:

```
[INFO] Starting CLI File Operations Tests...
[INFO] Running: ls /
[PASS] ls / (exit code: 0)
[INFO] Running: ls -l /test
[PASS] ls -l should succeed (exit code: 0)

=== Test Summary ===
  Pass: 17
  Fail: 0
  Skip: 0
  Total: 17
```

### Master Test Runner Output

```
=== EVIF Functional Test Suite ===
[INFO] Test directory: /Users/.../tests
[INFO] Server port: 8080

=== P0 CLI Tests ===
[INFO] Running: CLI File Operations
[PASS] CLI File Operations
[INFO] Running: CLI Plugin Management
[PASS] CLI Plugin Management

=== Test Suite Summary ===
  Passed: 8
  Failed: 0
  Skipped: 0
  Total: 8

All tests passed!
```

### Error Messages

When tests fail, you'll see:

```
[FAIL] cat /test/file.txt
  Expected: success
  Actual: failure
```

## Writing New Tests

### Test Script Template

```bash
#!/usr/bin/env bash
# EVIF Your Test Suite
# Description of what this tests

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../lib/test_helpers.sh"
source "$SCRIPT_DIR/../../lib/assertions.sh"

log_section "Your Test Suite"

# Test function
test_your_feature() {
    local output exit_code

    # Run command
    output=$($EVIF_CLI your_command /path 2>&1)
    exit_code=$?

    # Assert
    assert_exit_code 0 $exit_code "your_command should succeed"
    assert_output_contains "$output" "expected" "output should contain 'expected'"

    return 0
}

# Run all tests
main() {
    log_info "Starting Your Test Suite..."
    run_test "Your Feature" test_your_feature
    print_test_summary
}

main "$@"
```

### Available Assertions

- `assert_exit_code <expected> <actual> <message>`
- `assert_http_status <expected> <actual> <message>`
- `assert_output_contains <haystack> <needle> <message>`
- `assert_output_not_contains <haystack> <needle> <message>`
- `assert_output_equals <expected> <actual> <message>`
- `assert_json_field <json> <field> <message>`
- `assert_json_value <json> <field> <expected> <message>`
- `assert_file_exists <path> <message>`
- `assert_dir_exists <path> <message>`
- `assert_file_content <path> <expected> <message>`
- `assert_greater_than <actual> <threshold> <message>`
- `assert_less_than <actual> <threshold> <message>`

## Troubleshooting

### Server won't start

```bash
# Check if port is already in use
lsof -i :8080

# Kill existing server
kill $(lsof -t -i:8080)

# Check server logs
tail -f /tmp/evif_server_*.log
```

### Tests timeout

- Increase timeout in `run_all.sh`: Change `timeout 300` to `timeout 600`
- Check for stuck processes: `ps aux | grep evif`

### Tests skip unexpectedly

- Check if EVIF CLI is available: `cargo run -p evif-cli -- health`
- Check if jq is installed: `jq --version`
- Check environment variables: `echo $EVIF_SERVER_PORT`

## Success Criteria

- [x] All P0 tests pass (100%)
- [ ] At least 80% of P1 tests pass
- [ ] All error cases return correct HTTP status codes (400/404/500)
- [ ] Performance benchmarks meet targets
- [ ] Test suite runs automatically with single command
- [ ] Test results clearly show: PASS, FAIL, TOTAL
- [ ] Failed tests documented with expected vs actual
- [ ] Test execution time < 5 minutes for full suite
- [ ] Cleanup always runs (no leftover mounts or servers)
- [ ] Cloud tests skip gracefully without credentials

## Out of Scope

- **Unit testing**: Individual Rust function tests (use `cargo test` for those)
- **FUSE testing**: Requires system-level permissions (see `evif_fuse_test_spec.md`)
- **Cloud storage plugins**: S3, Azure, GCS require credentials (opt-in, skip gracefully)
- **Security auditing**: Not covered in this test suite
- **UI/E2E testing**: Web interface testing is separate (see `evif-web/`)

## References

- EVIF Functional Test Spec v2.0: `.ralph/agent/evif_functional_test_spec_v2.md`
- EVIF Documentation: `docs/`
- Issue Tracking: GitHub Issues

---

*Test Suite Version: 1.0*
*Created: 2026-02-26*
*Based on EVIF Functional Test Spec v2.0*