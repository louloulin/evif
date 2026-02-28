# Implementation Context - VFS Path Translation Fix

## Source: Explorer Hat Research (2026-02-08)

## Executive Summary

This document provides implementation context for adding `lookup_with_path()` to `RadixMountTable` and updating all REST handlers. The fix addresses a critical bug where handlers pass absolute paths to plugins that expect relative paths.

**Impact**: Affects 10+ REST handlers, blocks all file operations beyond root listing
**Complexity**: Medium - single method addition + systematic handler updates
**Risk**: Low - well-contained change, comprehensive testing possible

---

## 1. Architecture Context

### 1.1 Current Architecture Flow

```
UI Request: GET /api/v1/fs/list?path=/hello/world
    ↓
REST Handler (handlers.rs:329)
    ↓
mount_table.lookup("/hello/world") → HelloFsPlugin
    ↓
plugin.readdir("/hello/world")  ❌ BUG: expects readdir("/")
    ↓
Error: "Path not found: /hello/world"
```

### 1.2 Target Architecture Flow

```
UI Request: GET /api/v1/fs/list?path=/hello/world
    ↓
REST Handler (handlers.rs:329)
    ↓
mount_table.lookup_with_path("/hello/world")
    → (HelloFsPlugin, "/world")  ✅ FIXED: returns relative path
    ↓
plugin.readdir("/world")  ✅ Correct: plugin gets relative path
    ↓
Success: Returns file list
```

### 1.3 Component Relationships

```
RadixMountTable (evif-core)
    ↓ lookup_with_path() returns
    ↓ (Option<Plugin>, String)
    ↓
REST Handlers (evif-rest)
    ↓ use relative path
    ↓
EvifPlugin Trait (evif-core)
    ↓ readdir("/"), read("/"), etc.
    ↓
Plugin Implementations (evif-plugins)
    - MemFsPlugin
    - HelloFsPlugin
    - LocalFsPlugin
```

---

## 2. Integration Points

### 2.1 Primary Integration: RadixMountTable

**File**: `crates/evif-core/src/radix_mount_table.rs`
**Location**: After line 235 (after existing `lookup()` method)

**Dependencies**:
- Uses existing `normalize_path()` (line 254)
- Uses existing `self.mounts.read().await` pattern
- Returns existing types: `Arc<dyn EvifPlugin>`, `String`

**Integration Requirements**:
- MUST NOT modify existing `lookup()` method
- MUST reuse longest prefix matching algorithm
- MUST maintain O(k) performance
- MUST follow existing error patterns

### 2.2 Secondary Integration: REST Handlers

**File**: `crates/evif-rest/src/handlers.rs`

**Affected Handlers** (10 total):
1. `list_directory` (line 325)
2. `read_file` (line 218)
3. `write_file` (line 246)
4. `create_file` (line 281)
5. `create_directory` (line 359)
6. `delete_directory` (line 393)
7. `stat` (line 417)
8. `digest` (line 441)
9. `touch` (line 482)
10. `rename` (line 536)
11. `grep` (line 514)

**Change Pattern**:
```rust
// BEFORE (all 10+ handlers):
let plugin = state.mount_table
    .lookup(&params.path)
    .await
    .ok_or_else(|| RestError::NotFound(...))?;

plugin.method(&params.path).await  // ❌ BUG: absolute path

// AFTER (all 10+ handlers):
let (plugin_opt, relative_path) = state.mount_table
    .lookup_with_path(&params.path)
    .await;

let plugin = plugin_opt
    .ok_or_else(|| RestError::NotFound(...))?;

plugin.method(&relative_path).await  // ✅ FIXED: relative path
```

### 2.3 Special Case: Root Path Handling

**Current Behavior** (handlers.rs:329):
```rust
let plugin = state.mount_table
    .lookup(&params.path)
    .await
    .ok_or_else(|| RestError::NotFound(...))?;
```

When `params.path == "/"`, `lookup()` returns `None`, causing NotFound error.

**Issue**: Root listing (`/`) should list mount points, not return NotFound.

**Current Workaround**: Not currently handled in handlers - crashes UI.

**Target Behavior**: `lookup_with_path("/")` returns `(None, "/")`, allowing handlers to special-case root listing:

```rust
let (plugin_opt, relative_path) = state.mount_table
    .lookup_with_path(&params.path)
    .await;

if relative_path == "/" {
    // Special case: list mount points
    let mounts = state.mount_table.list_mounts().await;
    return Ok(Json(...));
}

let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
// Use plugin for non-root paths
```

---

## 3. Constraints and Considerations

### 3.1 Technical Constraints

**Performance Constraint**:
- MUST maintain O(k) complexity where k = path length
- Radix tree operations are fast, string slicing is O(1)
- No additional allocations beyond return tuple

**API Compatibility Constraint**:
- MUST NOT modify existing `lookup()` method
- MUST NOT change `EvifPlugin` trait
- MUST NOT break existing integrations

**Thread Safety Constraint**:
- Uses existing `Arc<RwLock<Trie>>` pattern
- Read lock acquisition: `self.mounts.read().await`
- No write operations needed (read-only lookup)

### 3.2 Path Semantics

**Normalization** (from `normalize_path()`):
- Input: `"hello"`, `"/hello"`, `"hello/"` → Output: `"/hello"`
- Root: `""`, `"/"` → Output: `"/"`

**Trie Key Format** (internal):
- Keys stored without leading slash: `"hello"`, `"hello/world"`
- Search key also stripped: `"/hello"` → `"hello"`

**Relative Path Format** (output):
- MUST preserve leading slash for consistency: `"/world"` not `"world"`
- Root path returns `"/"`
- Empty path should not occur

### 3.3 Edge Cases

**Case 1: Root Path**
```rust
lookup_with_path("/") → (None, "/")
```
- Rationale: No plugin owns root
- Handler should call `list_mounts()` instead

**Case 2: Simple Mount Point**
```rust
// Mounted: /hello → HelloFsPlugin
lookup_with_path("/hello") → (Some(HelloFsPlugin), "/")
```
- Rationale: Plugin root is "/" relative to mount point
- Strips entire mount prefix

**Case 3: Nested Path**
```rust
// Mounted: /hello → HelloFsPlugin
lookup_with_path("/hello/world/deep") → (Some(HelloFsPlugin), "/world/deep")
```
- Rationale: Plugin sees paths relative to its mount point
- Preserves nested structure

**Case 4: Nested Mount Points**
```rust
// Mounted: /hello → Plugin1, /hello/world → Plugin2
lookup_with_path("/hello") → (Some(Plugin1), "/")
lookup_with_path("/hello/world") → (Some(Plugin2), "/")
lookup_with_path("/hello/world/file.txt") → (Some(Plugin2), "/file.txt")
```
- Rationale: Longest prefix matching
- More specific mount wins

**Case 5: Non-existent Path**
```rust
lookup_with_path("/nonexistent") → (None, "")
```
- Rationale: No mount point matches
- Handler returns 404 NotFound

### 3.4 Error Handling Considerations

**Plugin Lookup Failure**:
```rust
let (plugin_opt, relative_path) = state.mount_table
    .lookup_with_path(&params.path)
    .await;

let plugin = plugin_opt
    .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;
```

**Plugin Method Failure**:
```rust
plugin.readdir(&relative_path)
    .await
    .map_err(|e| RestError::Internal(e.to_string()))?;
```

**Root Path Special Case**:
```rust
if relative_path == "/" && plugin_opt.is_none() {
    // List mount points instead
    return list_mount_points();
}
```

---

## 4. Testing Context

### 4.1 Unit Test Requirements

**Location**: `crates/evif-core/src/radix_mount_table.rs` (add to `#[cfg(test)]` module)

**Required Tests** (from design.md:170-176):
1. `test_lookup_with_path_root` - Verify `(None, "/")` for root
2. `test_lookup_with_path_simple` - Simple mount point stripping
3. `test_lookup_with_path_nested` - Nested path handling
4. `test_lookup_with_path_nonexistent` - No match returns `(None, "")`
5. `test_lookup_with_path_deep_nesting` - Deep path traversal
6. `test_lookup_with_path_nested_mounts` - Multiple nested mount points

**Test Pattern** (from existing tests):
```rust
#[tokio::test]
async fn test_lookup_with_path_simple() {
    let mount_table = RadixMountTable::new();
    let plugin = Arc::new(MockPlugin::new("hello"));

    mount_table.mount("/hello".to_string(), plugin.clone()).await.unwrap();

    let (found, rel_path) = mount_table.lookup_with_path("/hello").await;
    assert!(found.is_some());
    assert_eq!(rel_path, "/");
    assert_eq!(found.unwrap().name(), "hello");
}
```

### 4.2 Integration Test Requirements

**Location**: `crates/evif-rest/tests/api_contract.rs` (add new tests)

**Required Scenarios** (from design.md:182-188):
1. List root directory → returns mount points (special case)
2. List mounted plugin root → lists plugin contents
3. List nested directory → traverses correctly
4. Read file in nested path → succeeds
5. Create file in nested path → succeeds
6. Non-existent path → 404 error

**Test Pattern** (from existing tests):
```rust
#[tokio::test]
async fn test_list_nested_directory() {
    // Setup
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    // Create test server
    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).await.expect("serve");
    });

    // Test nested path
    let url = format!("http://127.0.0.1:{}/api/v1/fs/list?path=/mem/nested", port);
    let res = reqwest::get(&url).await.unwrap();
    assert!(res.status().is_success());
}
```

### 4.3 E2E Test Requirements

**Tool**: Playwright MCP (already in use)

**Test Flows** (from design.md:195-199):
1. Open UI → verify mount points appear
2. Click mount point → expand directory tree
3. Navigate nested folders → verify files appear
4. Create file → verify success
5. Read file → verify content

---

## 5. Implementation Phases

### Phase 1: Core Implementation (Priority 1)
**Files**: `crates/evif-core/src/radix_mount_table.rs`

**Tasks**:
1. Implement `lookup_with_path()` method after `lookup()`
2. Add unit tests to existing test module
3. Run `cargo test -p evif-core` to verify
4. Run `cargo clippy -p evif-core` to check for warnings

**Success Criteria**:
- All unit tests pass
- No clippy warnings
- Implementation matches design specification

### Phase 2: Handler Updates (Priority 1)
**Files**: `crates/evif-rest/src/handlers.rs`

**Tasks**:
1. Update all 10+ handlers to use `lookup_with_path()`
2. Add special case for root path in `list_directory()`
3. Update integration tests in `crates/evif-rest/tests/`
4. Run `cargo test -p evif-rest` to verify

**Success Criteria**:
- All integration tests pass
- API contracts maintained
- Root listing returns mount points

### Phase 3: E2E Validation (Priority 1)
**Tools**: Playwright MCP

**Tasks**:
1. Start frontend and backend servers
2. Run E2E test flows with Playwright
3. Verify all file operations work
4. Document any remaining issues

**Success Criteria**:
- All E2E tests pass
- UI functions correctly for all operations
- No "Path not found" errors for valid paths

---

## 6. Rollout Considerations

### 6.1 Backward Compatibility

**Impact**: Breaking change for handler implementation only
- Plugin API unchanged (good)
- REST API response format unchanged (good)
- Handler signature unchanged (good)
- Handler implementation changes (required)

**Migration**: Atomic - all handlers must update together
- Cannot partially implement (mix old and new)
- Must update all 10+ handlers in one change
- Test thoroughly before committing

### 6.2 Deployment Strategy

**Development**:
1. Implement `lookup_with_path()` first
2. Update one handler as proof of concept
3. Test manually with that handler
4. Update remaining handlers
5. Full test suite run

**Testing**:
1. Unit tests verify core logic
2. Integration tests verify API contracts
3. E2E tests verify UI functionality
4. Manual testing with Playwright MCP

**Rollback**: Simple git revert if critical issues found

### 6.3 Monitoring Considerations

**Metrics to Track**:
- Lookup performance (should remain <100μs for 100 mounts)
- Error rate (should decrease - fix current bug)
- Handler latency (should not increase)

**Logging**: No new logging required
- Existing error handling sufficient
- Add debug logs only if troubleshooting needed

---

## 7. Dependencies and Blocking

### 7.1 No External Dependencies

**Finding**: Implementation requires ZERO new dependencies
- All required types available in `std`
- `radix_trie` already in use
- `tokio::sync::RwLock` already available
- No crate additions needed

### 7.2 No Blocked Tasks

**Status**: Ready to implement
- Design approved
- Research complete
- No external dependencies
- No blocked integrations

### 7.3 Task Dependencies

**From scratchpad**:
- Task 1: Implement `lookup_with_path()` (THIS TASK) ✅ READY
- Task 2: Update REST handlers (BLOCKED by Task 1)
- Task 3: Add WebSocket retry logic (INDEPENDENT)
- Task 4: Add favicon.ico (INDEPENDENT)
- Task 5: E2E testing (BLOCKED by Task 2)

---

## 8. Risk Assessment

### 8.1 Technical Risks

**Risk**: Path stripping logic error
- **Impact**: HIGH - file operations fail
- **Mitigation**: Comprehensive unit + integration tests
- **Confidence**: HIGH (85%) - algorithm well-understood

**Risk**: Performance regression
- **Impact**: MEDIUM - slower API
- **Mitigation**: Benchmark before/after
- **Confidence**: HIGH (95%) - O(k) maintained, only adds O(1) string slice

**Risk**: Breaking existing functionality
- **Impact**: CRITICAL - UI breaks
- **Mitigation**: E2E testing with Playwright
- **Confidence**: HIGH (90%) - isolated change, no plugin API changes

### 8.2 Implementation Risks

**Risk**: Incomplete handler updates
- **Impact**: HIGH - mixed old/new behavior
- **Mitigation**: Update all handlers in one PR, comprehensive testing
- **Confidence**: HIGH (95%) - clear list of affected handlers

**Risk**: Edge case oversight
- **Impact**: MEDIUM - some paths fail
- **Mitigation**: Explicit edge case tests in design
- **Confidence**: HIGH (90%) - edge cases well-documented

---

## 9. Success Metrics

### 9.1 Functional Metrics

**Must Have**:
- [ ] All unit tests pass (6 tests)
- [ ] All integration tests pass (6 scenarios)
- [ ] All E2E tests pass (5 flows)
- [ ] No clippy warnings
- [ ] No new dependencies added
- [ ] All 10+ handlers updated

### 9.2 Quality Metrics

**Must Have**:
- [ ] Code follows existing patterns
- [ ] Comprehensive test coverage
- [ ] Clear documentation with examples
- [ ] Chinese comments for key logic
- [ ] Error handling consistent with codebase

### 9.3 Performance Metrics

**Must Have**:
- [ ] Lookup <100μs for 100 mounts
- [ ] No regression vs current `lookup()`
- [ ] String slicing O(1) (no allocations)

---

## 10. Open Questions

### Q1: Root Path Return Value (RESOLVED)
**Question**: Should `lookup_with_path("/")` return `(None, "/")` or `(None, "")`?

**Answer**: `(None, "/")` - consistent with `normalize_path()` output

**Rationale**: Handlers can check `relative_path == "/"` for special case

### Q2: Error Handling for Non-existent Paths (RESOLVED)
**Question**: Should `lookup_with_path("/nonexistent")` return error or `(None, "")`?

**Answer**: `(None, "")` - consistent with existing `lookup()` returning `None`

**Rationale**: Handlers already have pattern to convert `None` to `NotFound`

### Q3: String Slicing Implementation (RESOLVED)
**Question**: How to slice string after finding longest prefix?

**Answer**:
```rust
let relative_path = if best_key.is_empty() {
    "/".to_string()
} else {
    format!("/{}", &search_key[best_key.len()..])
};
```

**Rationale**: Preserves leading slash, handles root case

---

## 11. Implementation Checklist

### Code Implementation
- [ ] Add `lookup_with_path()` method to `RadixMountTable`
- [ ] Update `list_directory()` handler with root special case
- [ ] Update `read_file()` handler
- [ ] Update `write_file()` handler
- [ ] Update `create_file()` handler
- [ ] Update `create_directory()` handler
- [ ] Update `delete_directory()` handler
- [ ] Update `stat()` handler
- [ ] Update `digest()` handler
- [ ] Update `touch()` handler
- [ ] Update `rename()` handler
- [ ] Update `grep()` handler

### Testing
- [ ] Unit test: `test_lookup_with_path_root`
- [ ] Unit test: `test_lookup_with_path_simple`
- [ ] Unit test: `test_lookup_with_path_nested`
- [ ] Unit test: `test_lookup_with_path_nonexistent`
- [ ] Unit test: `test_lookup_with_path_deep_nesting`
- [ ] Unit test: `test_lookup_with_path_nested_mounts`
- [ ] Integration test: list root directory
- [ ] Integration test: list mounted plugin root
- [ ] Integration test: list nested directory
- [ ] Integration test: read file in nested path
- [ ] Integration test: create file in nested path
- [ ] Integration test: non-existent path returns 404
- [ ] E2E test: verify mount points appear in UI
- [ ] E2E test: navigate nested folders
- [ ] E2E test: create file via UI
- [ ] E2E test: read file via UI

### Verification
- [ ] `cargo test -p evif-core` passes
- [ ] `cargo test -p evif-rest` passes
- [ ] `cargo clippy -p evif-core` clean
- [ ] `cargo clippy -p evif-rest` clean
- [ ] `cargo build` succeeds
- [ ] Manual testing with Playwright MCP succeeds

---

## Summary

**Implementation is ready to proceed**:
- ✅ Design approved (85% confidence)
- ✅ Research complete
- ✅ No external dependencies
- ✅ Clear integration points
- ✅ Comprehensive test strategy
- ✅ Risk mitigations identified

**Next Step**: Hand off to 📋 Planner hat to create implementation plan.
