# Question 4: Testing Framework Selection - Answer Summary

**Date**: 2025-02-09 10:30
**Question**: Should E2E testing use Playwright MCP or a different approach?
**Answer**: Hybrid Approach - Native Rust for REST, Shell Scripts for CLI

---

## Executive Summary

**Recommendation**: Use domain-appropriate tools for E2E testing instead of browser automation:
- **REST API**: Native Rust integration tests (`reqwest` + `tokio-test`)
- **CLI**: Shell scripts (Bash + EVIF CLI binary)
- **Web UI**: Playwright MCP (optional, future enhancement)

**Confidence**: 90%

---

## Evidence from Initial Testing

**Playwright MCP Test Results** (2025-02-09):
- ✅ Successfully tested 8/30 REST endpoints
- ✅ Identified critical bug: HTTP 500 errors for domain errors
- ⚠️ Browser automation overhead unnecessary for API testing
- ⚠️ CLI testing would require complex shell interaction patterns

**Bug Discovered**: EVIF REST API returns HTTP 500 for domain errors instead of proper status codes
- Missing file → 500 instead of 404
- Invalid path → 500 instead of 400
- Root cause: `crates/evif-rest/src/lib.rs:66-67`

---

## Recommended Testing Strategy

### 1. REST API E2E Tests (Native Rust)

**Framework**: `tokio-test` + `reqwest` + `assert-json-diff`
**Location**: `tests/e2e_rest_api.rs`

**Advantages**:
- Direct HTTP client integration (no browser overhead)
- Type-safe assertions with Rust's type system
- Fast execution (milliseconds vs seconds)
- Easy to run in CI/CD (`cargo test --test e2e_rest_api`)
- Parallel test execution built-in

**Coverage**: 30 REST endpoints
- Health & Status (2)
- File CRUD (5)
- Directory Operations (3)
- Mount Management (3)
- Plugin Discovery (3)
- Handle Operations (5)
- Batch Operations (3)
- Advanced Operations (3)
- Metrics (3)

**Example**:
```rust
#[tokio::test]
async fn test_file_crud_operations() {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8081/api/v1";

    let create_resp = client.post(&format!("{}/files", base_url))
        .json(&json!({"path": "/local/test.txt", "content": "hello"}))
        .send()
        .await
        .expect("create request failed");
    assert_eq!(create_resp.status(), 201);
}
```

### 2. CLI E2E Tests (Shell Scripts)

**Framework**: Bash + `curl` + EVIF CLI binary
**Location**: `tests/e2e_cli.sh`

**Advantages**:
- Most accurately represents user workflows
- Tests actual CLI binary, not Rust API
- Easy to read and maintain
- Can test shell integration (pipes, redirects)
- No Rust compilation dependency for test updates

**Coverage**: 3 CLI scenarios
1. Basic File Operations (mount, cd, touch, echo, cat, rm)
2. Multi-Plugin Workflow (MemFS + LocalFS, copy between mounts)
3. Plugin Discovery (list plugins, get README/config)

**Example**:
```bash
#!/usr/bin/env bash
set -e

evif-cli <<EOF
mount localfs /local --path /tmp/evif-test
cd /local
touch test.txt
echo "hello" > test.txt
cat test.txt
rm test.txt
unmount /local
EOF

[ -f /tmp/evif-test/test.txt ] && echo "FAIL" || echo "PASS"
```

### 3. Web UI E2E Tests (Playwright MCP) - Optional

**Use Case**: Validate 47+ React components in EVIF Web UI
**Priority**: P2 (future enhancement)
**When to Use**:
- Testing user workflows across multiple pages
- Validating accessibility compliance
- Cross-browser compatibility testing
- Visual regression testing

**Not Required For**:
- REST API validation (use native Rust tests)
- CLI workflow validation (use shell scripts)
- Unit testing (use Rust's built-in test framework)

---

## Implementation Plan

### Phase 1: Fix Error Handling Bug (P0, 2-3 hours)
**Task**: `task-1770604284-4e7d`

Implement proper error mapping in `crates/evif-rest/src/lib.rs`:
- `EvifError::NotFound` → HTTP 404
- `EvifError::InvalidPath` → HTTP 400
- `EvifError::PermissionDenied` → HTTP 403
- `EvifError::AlreadyExists` → HTTP 409
- `EvifError::Timeout` → HTTP 504
- `EvifError::Network` → HTTP 503

### Phase 2: Implement REST API Tests (P1, 1-2 iterations)
**Task**: `task-1770604327-8294` (blocked by Phase 1)

Create `tests/e2e_rest_api.rs` with 30 endpoint smoke tests.

### Phase 3: Implement CLI Tests (P1, 1 iteration)
**Task**: `task-1770604334-ea87` (blocked by Phase 1)

Create `tests/e2e_cli.sh` with 3 workflow scenarios.

### Phase 4: Web UI Testing (P2, optional)
Consider Playwright MCP if Web UI validation becomes priority.

---

## Impact

✅ **More appropriate tooling** for REST/CLI testing
✅ **Faster test execution** (no browser overhead)
✅ **Better CI/CD integration** (cargo test, bash)
✅ **Easier to maintain** (domain-appropriate languages)
⚠️ **Playwright MCP remains available** for Web UI testing if needed

---

## Acceptance Criteria (from Question 2)

**REST API Smoke Tests**: 30 endpoints minimum
- 90% pass rate required (27/30 endpoints)
- Zero HTTP 500 errors (domain errors return proper status codes)

**CLI Workflow Tests**: 3 scenarios
- All scenarios complete without errors
- Reproducible results (run 3 times, same outcomes)

**Duration Estimate**: 2-3 iterations (6-9 hours) for Phases 2-3

---

## Documentation

**Requirements Updated**: `specs/evif-agfs-gap-analysis/requirements.md`
- Added Question 4 Q&A
- Documented testing framework rationale
- Specified implementation plan

**Test Results**: `claudedocs/e2e_test_results.md`
- Playwright MCP test execution log
- HTTP 500 bug details and fix requirements
- Root cause analysis

---

## Next Actions

1. ✅ Complete requirements update with Question 4 answer
2. ✅ Create tasks for Phases 1-3
3. ⏳ Execute Phase 1: Fix error handling bug
4. ⏳ Execute Phase 2: Implement REST API tests
5. ⏳ Execute Phase 3: Implement CLI tests
6. ⏳ Validate production readiness with E2E test results

---

**Event Emitted**: `answer.proposed`
**Status**: Requirements complete, tasks created, ready for execution
**Confidence**: 90%
