# VFS Path Translation Fix - Implementation Plan

## Source: 📋 Planner Hat
**Date**: 2026-02-08
**Task**: Implement `lookup_with_path()` in RadixMountTable and update REST handlers
**Approach**: Test-Driven Development (TDD)

---

## 1. Test Strategy

### 1.1 Unit Tests (Isolated Component Behavior)

**Location**: `crates/evif-core/src/radix_mount_table.rs` (add to existing test module)

**Test Case Matrix**:

| Test Case | Input Setup | Input Path | Expected Output | Purpose |
|-----------|-------------|------------|-----------------|---------|
| `test_lookup_with_path_root` | No mounts | `/` | `(None, "/")` | Root path handling |
| `test_lookup_with_path_simple` | Mount `/hello` → Plugin1 | `/hello` | `(Some(Plugin1), "/")` | Basic mount point |
| `test_lookup_with_path_nested` | Mount `/hello` → Plugin1 | `/hello/world` | `(Some(Plugin1), "/world")` | Prefix stripping |
| `test_lookup_with_path_nonexistent` | No mounts | `/nonexistent` | `(None, "")` | Error case |
| `test_lookup_with_path_deep_nesting` | Mount `/hello` → Plugin1 | `/hello/world/deep/path` | `(Some(Plugin1), "/world/deep/path")` | Deep traversal |
| `test_lookup_with_path_nested_mounts` | Mount `/hello` → P1, `/hello/world` → P2 | `/hello/world/file.txt` | `(Some(P2), "/file.txt")` | Longest prefix matching |

**Test Execution Order**:
1. Write `test_lookup_with_path_root` → FAIL (method doesn't exist)
2. Implement stub `lookup_with_path()` → FAIL (wrong return)
3. Implement root path logic → PASS
4. Write `test_lookup_with_path_simple` → FAIL (no mount matching)
5. Implement mount lookup logic → FAIL (no prefix stripping)
6. Implement prefix stripping → PASS
7. Write `test_lookup_with_path_nested` → PASS (already covered)
8. Write `test_lookup_with_path_nonexistent` → PASS (already covered)
9. Write `test_lookup_with_path_deep_nesting` → PASS (already covered)
10. Write `test_lookup_with_path_nested_mounts` → FAIL (longest prefix not working)
11. Fix longest prefix matching → PASS

**Success Criteria**:
- All 6 unit tests pass
- Code coverage for `lookup_with_path()` > 95%
- Tests run in < 100ms total

### 1.2 Integration Tests (Component Interaction)

**Location**: `crates/evif-rest/tests/api_contract.rs` (add new test module)

**Test Scenarios**:

| Scenario | API Request | Expected Response | Purpose |
|----------|-------------|-------------------|---------|
| `test_list_root_directory` | `GET /api/v1/fs/list?path=/` | 200, mount points list | Root listing special case |
| `test_list_mounted_plugin_root` | `GET /api/v1/fs/list?path=/mem` | 200, plugin contents | Mount point traversal |
| `test_list_nested_directory` | `GET /api/v1/fs/list?path=/mem/nested` | 200, nested files | Nested path handling |
| `test_read_file_in_nested_path` | `GET /api/v1/fs/read?path=/mem/nested/file.txt` | 200, file content | File operation with relative path |
| `test_create_file_in_nested_path` | `POST /api/v1/fs/create?path=/mem/new/file.txt` | 201, created | Write operation with relative path |
| `test_nonexistent_path_returns_404` | `GET /api/v1/fs/list?path=/nonexistent` | 404 NotFound | Error handling |

**Test Execution Order**:
1. Write `test_list_root_directory` → FAIL (no `list_mounts()` integration)
2. Add root path special case in `list_directory()` handler → PASS
3. Write `test_list_mounted_plugin_root` → FAIL (handlers still use `lookup()`)
4. Update `list_directory()` to use `lookup_with_path()` → PASS
5. Write `test_list_nested_directory` → PASS (same implementation)
6. Write `test_read_file_in_nested_path` → FAIL (read handler not updated)
7. Update `read_file()` handler → PASS
8. Write `test_create_file_in_nested_path` → FAIL (create handler not updated)
9. Update `create_file()` handler → PASS
10. Write `test_nonexistent_path_returns_404` → PASS (error handling works)
11. Update remaining 7 handlers → PASS
12. Run full integration test suite → ALL PASS

**Success Criteria**:
- All 6 integration tests pass
- API contracts maintained (no breaking changes)
- Response formats match existing API

### 1.3 E2E Test Scenario (Manual Validation)

**Tool**: Playwright MCP
**Executor**: 🔍 Validator hat

**E2E Scenario: Complete File Operations Workflow**

**Preconditions**:
- Frontend dev server running on port 3000
- Backend REST server running on port 8081
- Playwright MCP browser launched

**Test Steps**:

1. **Navigate to UI**
   - Action: `browser_navigate("http://localhost:3000")`
   - Expected: Page loads successfully, no errors in console
   - Verification: `browser_console_messages(level: "error")` returns empty

2. **Verify Mount Points Display**
   - Action: `browser_snapshot()` and check for mount points
   - Expected: Explorer shows `/hello`, `/mem`, `/local` mount points
   - Verification: Text content contains "hello", "mem", "local"

3. **Expand Mount Point**
   - Action: Click on `/hello` mount point in explorer
   - Expected: Directory tree expands, shows nested contents
   - Verification: Snapshot shows file list under `/hello`

4. **Navigate to Nested Directory**
   - Action: Double-click on nested folder
   - Expected: File list updates to show nested directory contents
   - Verification: Breadcrumb shows `/hello/nested` path

5. **Create New File**
   - Action: Use UI to create file at `/hello/nested/test.txt`
   - Expected: File creation succeeds, no error messages
   - Verification: File appears in explorer, Problems panel shows no errors

6. **Read File Content**
   - Action: Click on newly created file
   - Expected: File content displays in editor tab
   - Verification: Tab shows file name, editor displays content

7. **Verify No Errors**
   - Action: Check console messages and Problems panel
   - Expected: No "Path not found" errors, no 500 errors
   - Verification: Clean console log, empty Problems panel

**Success Criteria**:
- All 7 steps complete without errors
- No "Path not found" errors in console
- File operations work correctly through UI
- Visual verification of directory traversal and file operations

---

## 2. Implementation Plan (TDD Order)

### Phase 1: Core Implementation - `lookup_with_path()`

#### Step 1: Implement Method Stub and Root Path Handling

**Files to Create/Modify**:
- `crates/evif-core/src/radix_mount_table.rs` (add method after line 235)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_lookup_with_path_root() {
    let table = RadixMountTable::new();
    let (plugin, path) = table.lookup_with_path("/").await;
    assert!(plugin.is_none());
    assert_eq!(path, "/");
}
```

**Implementation Steps**:
1. Add method signature: `pub async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String)`
2. Call `normalize_path()` to clean input
3. Add early return: if path is "/", return `(None, "/")`
4. Run test → PASS

**Demo**: Method compiles, root path handled correctly

**Connects to**: Previous work - `normalize_path()` and `lookup()` patterns

---

#### Step 2: Implement Mount Lookup and Prefix Stripping

**Tests to Write**:
```rust
#[tokio::test]
async fn test_lookup_with_path_simple() {
    let table = RadixMountTable::new();
    let plugin = Arc::new(MockPlugin::new("hello"));
    table.mount("/hello".to_string(), plugin.clone()).await.unwrap();

    let (found, rel_path) = table.lookup_with_path("/hello").await;
    assert!(found.is_some());
    assert_eq!(rel_path, "/");
    assert_eq!(found.unwrap().name(), "hello");
}

#[tokio::test]
async fn test_lookup_with_path_nested() {
    let table = RadixMountTable::new();
    let plugin = Arc::new(MockPlugin::new("hello"));
    table.mount("/hello".to_string(), plugin).await.unwrap();

    let (found, rel_path) = table.lookup_with_path("/hello/world").await;
    assert!(found.is_some());
    assert_eq!(rel_path, "/world");
}
```

**Implementation Steps**:
1. Acquire read lock: `self.mounts.read().await`
2. Use longest prefix matching (reuse `lookup()` algorithm)
3. Strip mount prefix from input path
4. Return `(Some(plugin), relative_path)`
5. Run tests → PASS

**Algorithm**:
```rust
// 1. Normalize input path
let normalized = Self::normalize_path(path);

// 2. Find longest matching prefix
let best_key = self.find_longest_prefix(&normalized);

// 3. Strip prefix and return relative path
if let Some(key) = best_key {
    let plugin = self.mounts.get(&key).unwrap();
    let relative_path = if key.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", &normalized[key.len()..])
    };
    (Some(plugin.clone()), relative_path)
} else {
    (None, "".to_string())
}
```

**Demo**: Can look up mount points and strip prefixes correctly

**Connects to**: Step 1's root handling, existing `lookup()` implementation

---

#### Step 3: Handle Edge Cases (Non-existent, Deep Nesting, Nested Mounts)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_lookup_with_path_nonexistent() {
    let table = RadixMountTable::new();
    let (found, path) = table.lookup_with_path("/nonexistent").await;
    assert!(found.is_none());
    assert_eq!(path, "");
}

#[tokio::test]
async fn test_lookup_with_path_deep_nesting() {
    let table = RadixMountTable::new();
    let plugin = Arc::new(MockPlugin::new("hello"));
    table.mount("/hello".to_string(), plugin).await.unwrap();

    let (found, rel_path) = table.lookup_with_path("/hello/world/deep/path").await;
    assert!(found.is_some());
    assert_eq!(rel_path, "/world/deep/path");
}

#[tokio::test]
async fn test_lookup_with_path_nested_mounts() {
    let table = RadixMountTable::new();
    let plugin1 = Arc::new(MockPlugin::new("plugin1"));
    let plugin2 = Arc::new(MockPlugin::new("plugin2"));
    table.mount("/hello".to_string(), plugin1).await.unwrap();
    table.mount("/hello/world".to_string(), plugin2).await.unwrap();

    // Should match /hello/world (longest prefix)
    let (found, rel_path) = table.lookup_with_path("/hello/world/file.txt").await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().name(), "plugin2");
    assert_eq!(rel_path, "/file.txt");
}
```

**Implementation Steps**:
1. Verify non-existent path returns `(None, "")` - should already PASS
2. Verify deep nesting works - should already PASS
3. Fix longest prefix matching if nested mounts test fails
4. Run tests → ALL PASS

**Demo**: All edge cases handled correctly, longest prefix matching works

**Connects to**: Step 2's prefix stripping logic

---

#### Step 4: Verify and Document

**Files to Modify**:
- Add Chinese comments explaining key logic
- Add rustdoc examples to method documentation

**Tests to Run**:
```bash
cargo test -p evif-core --lib radix_mount_table
cargo clippy -p evif-core
```

**Success Criteria**:
- All unit tests pass
- No clippy warnings
- Code documented with Chinese comments
- Examples in rustdoc compile and pass

**Demo**: Complete, tested, documented `lookup_with_path()` method ready for integration

---

### Phase 2: Handler Updates - Systematic Migration

#### Step 5: Update `list_directory()` Handler with Root Special Case

**Files to Modify**:
- `crates/evif-rest/src/handlers.rs` (modify `list_directory()` starting at line 325)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_list_root_directory() {
    // Setup test server with mount table
    // Request GET /api/v1/fs/list?path=/
    // Assert 200 OK with mount points list
}
```

**Implementation Steps**:
1. Replace `lookup()` with `lookup_with_path()`
2. Add root path special case:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;

   if relative_path == "/" && plugin_opt.is_none() {
       // Special case: list mount points
       let mounts = state.mount_table.list_mounts().await;
       let files: Vec<FileInfo> = mounts.into_iter().map(|name| FileInfo {
           name: name.clone(),
           path: format!("/{}", name),
           file_type: "directory".to_string(),
           size: 0,
           modified: Utc::now(),
       }).collect();
       return Ok(Json(files));
   }

   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.readdir(&relative_path).await
   ```
3. Run integration test → PASS

**Demo**: Root listing returns mount points, no more NotFound error

**Connects to**: Phase 1's `lookup_with_path()` implementation

---

#### Step 6: Update File Read Handlers (`read_file`, `stat`, `digest`)

**Files to Modify**:
- `crates/evif-rest/src/handlers.rs` (modify `read_file` at line 218, `stat` at line 417, `digest` at line 441)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_read_file_in_nested_path() {
    // Setup mount with nested file
    // Request GET /api/v1/fs/read?path=/mem/nested/file.txt
    // Assert 200 OK with file content
}

#[tokio::test]
async fn test_stat_file_in_nested_path() {
    // Similar setup, test stat endpoint
}

#[tokio::test]
async fn test_digest_file_in_nested_path() {
    // Similar setup, test digest endpoint
}
```

**Implementation Steps**:
1. Update `read_file()`:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.read(&relative_path, params.offset, params.size).await
   ```
2. Update `stat()` similarly
3. Update `digest()` similarly
4. Run integration tests → ALL PASS

**Demo**: Can read files in nested paths, no more "Path not found" errors

**Connects to**: Step 5's handler pattern

---

#### Step 7: Update File Write Handlers (`write_file`, `create_file`, `touch`)

**Files to Modify**:
- `crates/evif-rest/src/handlers.rs` (modify `write_file` at line 246, `create_file` at line 281, `touch` at line 482)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_create_file_in_nested_path() {
    // Setup mount
    // Request POST /api/v1/fs/create?path=/mem/new/file.txt
    // Assert 201 Created
}

#[tokio::test]
async fn test_write_file_in_nested_path() {
    // Setup mount with file
    // Request POST /api/v1/fs/write?path=/mem/existing/file.txt
    // Assert 200 OK
}

#[tokio::test]
async fn test_touch_file_in_nested_path() {
    // Setup mount
    // Request POST /api/v1/fs/touch?path=/mem/file.txt
    // Assert 201 Created
}
```

**Implementation Steps**:
1. Update `write_file()`
2. Update `create_file()`
3. Update `touch()`
4. Run integration tests → ALL PASS

**Demo**: Can create and write files in nested paths

**Connects to**: Step 6's handler pattern

---

#### Step 8: Update Directory Handlers (`create_directory`, `delete_directory`)

**Files to Modify**:
- `crates/evif-rest/src/handlers.rs` (modify `create_directory` at line 359, `delete_directory` at line 393)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_create_directory_in_nested_path() {
    // Setup mount
    // Request POST /api/v1/fs/mkdir?path=/mem/new/dir
    // Assert 201 Created
}

#[tokio::test]
async fn test_delete_directory_in_nested_path() {
    // Setup mount with directory
    // Request DELETE /api/v1/fs/remove?path=/mem/dir
    // Assert 200 OK
}
```

**Implementation Steps**:
1. Update `create_directory()`
2. Update `delete_directory()`
3. Run integration tests → ALL PASS

**Demo**: Can create and delete directories in nested paths

**Connects to**: Step 7's handler pattern

---

#### Step 9: Update Advanced Handlers (`rename`, `grep`)

**Files to Modify**:
- `crates/evif-rest/src/handlers.rs` (modify `rename` at line 536, `grep` at line 514)

**Tests to Write**:
```rust
#[tokio::test]
async fn test_rename_file_in_nested_path() {
    // Setup mount with file
    // Request POST /api/v1/fs/rename?from=/mem/old.txt&to=/mem/new.txt
    // Assert 200 OK
}

#[tokio::test]
async fn test_grep_in_nested_path() {
    // Setup mount with files containing content
    // Request GET /api/v1/fs/grep?path=/mem/nested&pattern=test
    // Assert 200 OK with results
}
```

**Implementation Steps**:
1. Update `rename()` - needs two path lookups
2. Update `grep()` - needs recursive directory handling
3. Run integration tests → ALL PASS

**Demo**: Advanced operations work with nested paths

**Connects to**: Step 8's handler pattern

---

#### Step 10: Integration Test Suite Verification

**Tests to Run**:
```bash
cargo test -p evif-rest --test api_contract
cargo clippy -p evif-rest
```

**Success Criteria**:
- All 6 integration tests pass
- No clippy warnings
- API contracts maintained
- Response formats unchanged

**Demo**: All REST handlers updated and verified

---

### Phase 3: E2E Validation - Playwright MCP Testing

#### Step 11: Manual E2E Testing with Playwright

**Files to Create**:
- `specs/vfs-path-translation-fix/e2e-test-plan.md` (document test results)

**Tools**: Playwright MCP

**Test Flow**:
1. Start frontend: `npm run dev` (port 3000)
2. Start backend: `cargo run -p evif-rest` (port 8081)
3. Launch Playwright browser
4. Execute E2E scenario from Section 1.3
5. Document results

**Success Criteria**:
- All 7 E2E steps pass
- No "Path not found" errors
- File operations work through UI
- Visual verification confirms functionality

**Demo**: UI works end-to-end, VFS bug fixed

---

## 3. Success Criteria Summary

### Phase 1: Core Implementation
- [ ] All 6 unit tests pass
- [ ] `lookup_with_path()` method implemented
- [ ] Code documented with Chinese comments
- [ ] No clippy warnings in evif-core

### Phase 2: Handler Updates
- [ ] All 10+ handlers updated
- [ ] All 6 integration tests pass
- [ ] Root listing returns mount points
- [ ] No clippy warnings in evif-rest

### Phase 3: E2E Validation
- [ ] E2E test scenario passes
- [ ] No "Path not found" errors in UI
- [ ] All file operations work correctly
- [ ] Visual verification complete

---

## 4. Risk Mitigation

### Risk 1: Path Stripping Logic Error
**Mitigation**: Comprehensive unit tests covering all edge cases
**Confidence**: HIGH (85%)

### Risk 2: Incomplete Handler Updates
**Mitigation**: Systematic update of all 10+ handlers, comprehensive integration tests
**Confidence**: HIGH (95%)

### Risk 3: Breaking Changes
**Mitigation**: API contracts tested, response formats verified
**Confidence**: HIGH (90%)

---

## 5. Next Steps

**Current Hat**: 📋 Planner
**Next Hat**: 📝 Task Writer

**Event to Emit**: `plan.ready`
**Payload**: Test strategy complete, 11 implementation steps defined, success criteria established, risks mitigated

**Task Writer Responsibilities**:
- Convert this plan into structured code task files
- Create Given-When-Then acceptance criteria for each step
- Generate task dependencies graph
- Hand off to 🔨 Builder hat for implementation
