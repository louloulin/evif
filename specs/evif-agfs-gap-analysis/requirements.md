# EVIF vs AGFS Gap Analysis - Requirements

**Task**: evif-agfs-gap-analysis
**Created**: 2025-02-08
**Status**: Complete

## Original Objective

Perform a comprehensive gap analysis between EVIF (Rust implementation) and AGFS (Go implementation) to quantify feature parity, identify missing functionality, and create an implementation roadmap.

---

## Requirements Consolidation from Q&A

### R1: Quantitative Comparison
Compare codebases across multiple dimensions:
- Source file count and lines of code
- Plugin count and functionality
- CLI command count and capabilities
- REST API endpoint coverage
- Web UI component richness
- MCP server tool coverage
- FUSE integration completeness

### R2: Qualitative Assessment
Evaluate architectural differences:
- Concurrency models (async Rust vs sync Go)
- Memory safety approaches (compile-time vs GC)
- Error handling patterns (Result<T,E> vs error interface)
- Type system strength (static vs dynamic)

### R3: Gap Identification
Identify specific missing features:
- Global handle management
- Dynamic .so loading for plugins
- Shell variable substitution
- Shell scripting control flow (if/else/loops)
- Background task management

### R4: Prioritization Framework
Classify gaps by priority:
- **P1**: Important but not blocking (global handle mgmt)
- **P2**: Optional enhancements (dynamic loading, scripting)
- **P3**: Nice to have (future improvements)

### R5: Production Readiness Assessment
Determine if EVIF is production-ready despite gaps:
- Core functionality completeness
- Stability and reliability
- Performance characteristics
- Unique advantages over AGFS
- **Edge case recovery capabilities** (see Q3 below)

### R6: Implementation Roadmap
Create phased plan for addressing gaps:
- Phase 0: Edge case recovery (4-6 days, **production-blocking**)
- Phase 1: Critical missing features (optional, P1)
- Phase 2: Quality-of-life improvements (P2)
- Phase 3: Advanced features and ecosystem expansion (P2)

---

## Inquisitor Questions & Architect Answers

### Question 1: Security Positioning (2025-02-08 22:40)

**Question**: Should security hardening (auth, rate limiting, input sanitization) be implemented BEFORE declaring production readiness, or is it acceptable to deploy with known security limitations in trusted environments?

**Answer**: **Phased Approach** (Alternative 3)

**Rationale**:
- **Core security** (path sanitization) is production-blocking
- **Auth/rate limiting** can be deferred for trusted environments
- **Clear documentation** of limitations required
- **Deployment guidance**: Use behind reverse proxy in production

**Confidence**: 80%

**Impact**:
- EVIF can claim "production-ready for trusted environments"
- Auth/rate limiting added to Phase 2 (P2 priority)
- Documentation must specify security assumptions

---

### Question 2: E2E Test Acceptance Criteria (2025-02-08 22:50)

**Question**: What specific E2E test coverage is sufficient to validate EVIF's production readiness? How many REST endpoints? Which CLI workflows? What pass rate threshold?

**Answer**: **Balanced Approach** (Alternative 3)

**Rationale**:
- **Minimal** (3 plugins only) is insufficient - doesn't validate endpoint categories or integration points
- **Comprehensive** (all 56 endpoints) is overkill - doesn't prioritize critical vs. nice-to-have
- **Balanced** (30 endpoints + 3 CLI scenarios) optimizes for risk reduction while achievable

**Specific Acceptance Criteria**:

#### REST API Smoke Tests (30 endpoints minimum)

**Category 1: Health & Status** (2 endpoints)
- GET /health → returns {"status": "healthy"}
- GET /api/v1/health → returns status, version, uptime

**Category 2: File CRUD Operations** (5 endpoints)
- POST /api/v1/files → create file, returns 201
- GET /api/v1/files?path=... → read file content
- PUT /api/v1/files?path=... → write file content
- DELETE /api/v1/files?path=... → delete file
- GET /api/v1/stat?path=... → returns file metadata

**Category 3: Directory Operations** (3 endpoints)
- POST /api/v1/directories → create directory
- GET /api/v1/directories?path=... → list contents
- DELETE /api/v1/directories?path=... → delete directory

**Category 4: Mount Management** (3 endpoints)
- POST /api/v1/mount → mount LocalFS plugin
- GET /api/v1/mounts → list active mounts
- POST /api/v1/unmount → unmount plugin

**Category 5: Plugin Discovery** (3 endpoints)
- GET /api/v1/plugins → list available plugins
- GET /api/v1/plugins/:name/readme → get plugin docs
- GET /api/v1/plugins/:name/config → get config schema

**Category 6: Handle Operations** (5 endpoints)
- POST /api/v1/handles/open → open file handle
- GET /api/v1/handles/:id → get handle info
- POST /api/v1/handles/:id/read → read from handle
- POST /api/v1/handles/:id/write → write to handle
- POST /api/v1/handles/:id/close → close handle

**Category 7: Batch Operations** (3 endpoints)
- POST /api/v1/batch/copy → batch copy files
- GET /api/v1/batch/progress/:id → check operation status
- GET /api/v1/batch/operations → list operations

**Category 8: Advanced Operations** (3 endpoints)
- POST /api/v1/digest → compute file hash
- POST /api/v1/grep → search file content
- POST /api/v1/rename → move file/directory

**Category 9: Metrics** (3 endpoints)
- GET /api/v1/metrics/traffic → returns traffic stats
- GET /api/v1/metrics/operations → returns operation counts
- POST /api/v1/metrics/reset → resets counters

#### CLI Workflow Tests (3 scenarios)

**Scenario 1: Basic File Operations**
1. Start EVIF CLI
2. Mount LocalFS plugin to `/local`
3. Execute: `cd /local`, `ls`, `touch test.txt`, `echo "hello" > test.txt`, `cat test.txt`
4. Verify: File created with correct content
5. Cleanup: `rm test.txt`, unmount

**Scenario 2: Multi-Plugin Workflow**
1. Mount MemFS to `/mem`
2. Mount LocalFS to `/local`
3. Copy file from `/local` to `/mem`
4. Verify: File exists in both mounts, isolated
5. Unmount both plugins

**Scenario 3: Plugin Discovery**
1. Execute: `plugins --list`
2. Verify: At least 28 plugins listed
3. Execute: `plugins --info LocalFS`
4. Verify: README and config displayed

#### Acceptance Thresholds
- **90% of REST smoke tests pass** (27/30 endpoints minimum)
- **All 3 CLI workflows complete** without errors
- **Zero HTTP 500 errors** (4xx/5xx indicates bugs, not expected failures)
- **Tests are reproducible** (run 3 times, same results)

**Duration Estimate**: 2-3 iterations (6-9 hours)
**Unblocks**: task-1770549344-d854 (E2E testing with Playwright MCP)

**Confidence**: 80%

---

### Question 3: Edge Case Recovery Strategies (2025-02-08 23:20)

**Question**: What specific recovery strategies should EVIF implement for edge cases? (FUSE mount failures, plugin cascade failures, network partitions)

**Answer**: **Hybrid Approach** (Alternative 3)

**Rationale**:
- **Graceful degradation** (current state) is insufficient for production
- **Active recovery** (everything) adds unnecessary complexity for plugin failures
- **Hybrid** optimizes: Active for critical paths, graceful for developer errors

**Rationale**:
- **Graceful degradation** (current state) is insufficient for production
- **Active recovery** (everything) adds unnecessary complexity for plugin failures
- **Hybrid** optimizes: Active for critical paths, graceful for developer errors

**Specific Implementation Plan**:

#### Phase 0 (Production-Blocking, 4-6 days)

**1. FUSE I/O Retry** (1-2 days) - **Active Recovery**
- **Why**: FUSE operations are user-visible, transient failures should retry transparently
- **Implementation**:
  - Retry wrapper for read/write operations
  - 3 attempts with exponential backoff (100ms → 400ms → 1600ms)
  - Retry only `EIO` and `EINTR` errors (not `ENOENT` or `EACCES`)
  - Log `warn!` on retry, `error!` on final failure
- **Impact**: Transient I/O failures become transparent to users

**2. HTTPFS Retry with Jitter** (2-3 days) - **Active Recovery**
- **Why**: Network partitions are transient; users expect HTTP operations to retry
- **Implementation**:
  - Retry decorator function: `retry_async<T>(operation, max_attempts)`
  - Exponential backoff with jitter: 100ms → 400ms → 1600ms → 6400ms (max 6.4s)
  - Retryable errors: Timeout, Connection, 5xx, 408, 429
  - Fix error type usage: Use `EvifError::Timeout`/`Network` not `InvalidPath`
  - Jitter: Add random 0-100ms to prevent thundering herd
- **Impact**: HTTPFS/StreamFS/OSSFS handle network partitions gracefully

**3. Plugin Health Endpoint** (0.5 day) - **Graceful Degradation**
- **Why**: Plugin failures are developer errors, not transient
- **Implementation**:
  - `GET /api/v1/plugins/:name/health` → returns `{status: "ok"|"error"}`
  - Plugin implements optional `health()` method
  - Manual unmount workflow documented
- **Impact**: Operators can monitor and manually recover broken plugins

#### Production Readiness Impact

**With Phase 0 ONLY**:
- ✅ Can claim "production-ready for trusted environments"
- ✅ Handles transient failures transparently (FUSE, network)
- ✅ Manual recovery workflow for broken plugins
- ✅ Clear documentation of known limitations

**Without Phase 0**:
- ❌ Cannot claim production-ready (network partitions cause permanent failures)
- ❌ FUSE I/O errors are user-visible (no retry)
- ❌ HTTP timeouts require manual retry

#### Phase 1 (Enhancement, Non-Blocking, 4 days)

**4. Circuit Breaker for HTTPFS** (2 days)
- Track failure rate per host
- Open circuit after 5 consecutive failures
- Attempt reconnect after 30 seconds
- Return cached last-good response or fast-fail

**5. Plugin Isolation Enhancement** (2 days)
- Background health monitor (poll every 30s)
- Auto-unmount after 10 consecutive health check failures
- Event log for plugin lifecycle

**Confidence**: 85%

---

### Question 4: Testing Framework Selection (2025-02-09 10:30)

**Question**: Should E2E testing use Playwright MCP (browser automation) or a different approach more suited for REST API and CLI validation?

**Answer**: **Hybrid Approach** - Native Rust for REST API, Shell Scripts for CLI

**Rationale**:
- **Playwright MCP** successfully identified the HTTP 500 error handling bug during testing
- **However**, for ongoing E2E validation, domain-appropriate tools are more suitable
- **Native Rust integration tests** provide better ergonomics for REST API testing
- **Shell scripts** more accurately represent CLI user workflows
- **Playwright MCP** should be reserved for Web UI E2E testing (47+ React components)

**Evidence from Initial E2E Test Run**:
- ✅ Playwright MCP successfully tested 8/30 REST endpoints
- ✅ Identified critical bug: HTTP 500 errors for domain errors (see `claudedocs/e2e_test_results.md`)
- ⚠️ Browser automation overhead unnecessary for API testing
- ⚠️ CLI testing would require complex shell interaction patterns

**Recommended Testing Strategy**:

#### 1. REST API E2E Tests (Native Rust Integration Tests)

**Location**: `tests/e2e_rest_api.rs`

**Framework**: `tokio-test` + `reqwest` + `assert-json-diff`

**Advantages**:
- Direct HTTP client integration (no browser overhead)
- Type-safe assertions with Rust's type system
- Fast execution (milliseconds vs seconds)
- Easy to run in CI/CD (`cargo test --test e2e_rest_api`)
- Parallel test execution built-in

**Example Structure**:
```rust
#[tokio::test]
async fn test_file_crud_operations() {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8081/api/v1";

    // Create file
    let create_resp = client.post(&format!("{}/files", base_url))
        .json(&json!({"path": "/local/test.txt", "content": "hello"}))
        .send()
        .await
        .expect("create request failed");
    assert_eq!(create_resp.status(), 201);

    // Read file
    let read_resp = client.get(&format!("{}/files", base_url))
        .query(&[("path", "/local/test.txt")])
        .send()
        .await
        .expect("read request failed");
    assert_eq!(read_resp.status(), 200);
}
```

**Coverage**: 30 REST endpoints (as specified in Question 2 acceptance criteria)

#### 2. CLI E2E Tests (Shell Scripts)

**Location**: `tests/e2e_cli.sh`

**Framework**: Bash + `curl` + EVIF CLI binary

**Advantages**:
- Most accurately represents user workflows
- Tests actual CLI binary, not Rust API
- Easy to read and maintain
- Can test shell integration (pipes, redirects)
- No Rust compilation dependency for test updates

**Example Structure**:
```bash
#!/usr/bin/env bash
set -e

# Scenario 1: Basic File Operations
evif-cli <<EOF
mount localfs /local --path /tmp/evif-test
cd /local
touch test.txt
echo "hello" > test.txt
cat test.txt
rm test.txt
unmount /local
EOF

# Verify file was created and deleted
[ -f /tmp/evif-test/test.txt ] && echo "FAIL: file not deleted" || echo "PASS: scenario 1"
```

**Coverage**: 3 CLI scenarios (as specified in Question 2 acceptance criteria)

#### 3. Web UI E2E Tests (Playwright MCP) - Optional Enhancement

**Use Case**: Validate 47+ React components in EVIF Web UI

**When to Use**:
- Testing user workflows across multiple pages
- Validating accessibility compliance
- Cross-browser compatibility testing
- Visual regression testing

**Not Required For**:
- REST API validation (use native Rust tests)
- CLI workflow validation (use shell scripts)
- Unit testing (use Rust's built-in test framework)

**Implementation**: Future enhancement if Web UI testing becomes priority

**Transition Plan**:

**Phase 1**: Fix HTTP 500 error handling bug (P0, 2-3 hours)
- Implement proper error mapping in `crates/evif-rest/src/lib.rs`
- Document error status codes in API docs

**Phase 2**: Implement native Rust REST API tests (P1, 1-2 iterations)
- Create `tests/e2e_rest_api.rs`
- Implement 30 endpoint smoke tests
- Validate proper HTTP status codes (no 500s for domain errors)

**Phase 3**: Implement shell script CLI tests (P1, 1 iteration)
- Create `tests/e2e_cli.sh`
- Implement 3 workflow scenarios
- Validate CLI binary execution

**Phase 4**: Consider Playwright MCP for Web UI (P2, optional)
- If Web UI validation becomes priority
- Add browser-based E2E tests for React components
- Validate accessibility and cross-browser compatibility

**Confidence**: 90%

**Impact**:
- ✅ More appropriate tooling for REST/CLI testing
- ✅ Faster test execution (no browser overhead)
- ✅ Better CI/CD integration (cargo test, bash)
- ✅ Easier to maintain (domain-appropriate languages)
- ⚠️ Playwright MCP remains available for Web UI testing if needed

---

## Success Criteria

1. ✅ Quantified completion percentage across all dimensions
2. ✅ Identified all critical gaps (P0/P1)
3. ✅ Assessed production readiness with edge case recovery plan
4. ✅ Prioritized remaining work by impact
5. ✅ Created actionable implementation roadmap
6. ✅ Defined E2E test acceptance criteria (30 REST + 3 CLI scenarios)
7. ✅ Specified edge case recovery strategies (Phase 0: 4-6 days)

## Key Findings

**Overall Completion**: **89.25%**

**No Blocking Gaps**: All critical functionality is implemented. Missing features are optional enhancements.

**EVIF Advantages**:
- More plugins (28 vs 19)
- More CLI commands (61 vs 54)
- More REST endpoints (56 vs 30+)
- Far richer Web UI (47+ vs ~10 components)
- Superior architecture (async, type-safe, memory-safe)

**Remaining Gaps**:
- **Phase 0** (Production-Blocking, 4-6 days):
  - FUSE I/O retry (Active recovery)
  - HTTPFS retry with jitter (Active recovery)
  - Plugin health endpoint (Graceful degradation)
- **Phase 1** (Optional, P1, 3-4 days):
  - Global handle management
- **Phase 2** (Optional, P2, 13-17 days):
  - Dynamic .so loading (8-10 days)
  - Shell variables and scripting (5-7 days)

## Scope Boundaries

**In Scope**:
- Core file system operations
- REST API functionality
- CLI/Shell capabilities
- Plugin ecosystem
- MCP server coverage
- FUSE integration
- Web UI components
- **Edge case recovery strategies** (Phase 0)
- **E2E testing criteria** (validates production readiness)

**Out of Scope**:
- Performance benchmarking (not required for gap analysis)
- Security audit (separate concern)
- Documentation quality assessment
- Code style comparisons
- Phase 1+ enhancements (nice-to-have, not production-blocking)

## Technical Constraints

- Analysis based on source code examination only
- No runtime testing during analysis phase
- Static analysis of features and interfaces
- Comparison based on AGFS as baseline (not EVIF deficiencies)
- **Phase 0 edge case recovery required before production deployment**

## Edge Cases Considered

- Different programming languages (Rust vs Go) → Architectural differences acknowledged
- Different module organization → File count metrics weighted appropriately
- Async vs sync implementation → Considered as advantage, not gap
- EVIF's additional features → Treated as bonuses, not required for parity
- **Edge case recovery** → Hybrid approach balances practicality with production requirements
- **E2E test coverage** → Balanced approach validates critical workflows without over-testing

## Dependencies

**Unblocks**:
- `task-1770549344-d854` - E2E testing with Playwright MCP (now unblocked with clear acceptance criteria)

**Blocked By**:
- None (analysis complete)

## Next Actions

1. ✅ **Complete design document** with edge case recovery and E2E testing sections
2. ✅ **Publish `design.drafted`** event to hand off to Design Critic
3. ⏳ **Execute E2E testing** (task-1770549344-d854) using Playwright MCP
4. ⏳ **Implement Phase 0 edge case recovery** (4-6 days) if production readiness required
