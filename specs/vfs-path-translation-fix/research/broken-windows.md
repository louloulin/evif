# Broken Windows - VFS Path Translation Fix

## Source: Code Smell Analysis (2026-02-08)

## Files to be Modified:
1. `crates/evif-core/src/radix_mount_table.rs` - Add `lookup_with_path()` method
2. `crates/evif-rest/src/handlers.rs` - Update 10+ handlers to use new method

## Code Smells Found: LOW RISK

### 1. radix_mount_table.rs:211 - Inconsistent Loop Variable Naming
**Type**: Minor - naming inconsistency
**Risk**: Low
**Location**: `radix_mount_table.rs:222-230`

**Current Code**:
```rust
for i in (0..=search_key.len()).rev() {
    let prefix = &search_key[..i];
    if let Some(plugin) = mounts.get(prefix) {
        if prefix.len() > best_len {
            best_match = Some(plugin.clone());
            best_len = prefix.len();
        }
    }
}
```

**Issue**: Variable `i` is not descriptive - should be `prefix_end_idx` or similar.

**Fix**: Not recommended - this is idiomatic Rust for range iteration.

---

### 2. handlers.rs:329 - Missing Comment for Path Translation Bug
**Type**: Documentation - missing comment explaining the bug
**Risk**: Low (no behavior change)
**Location**: `handlers.rs:329-338`

**Current Code**:
```rust
let plugin = state.mount_table
    .lookup(&params.path)
    .await
    .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

// List files using readdir
let evif_file_infos = plugin
    .readdir(&params.path)  // BUG: passes absolute path instead of relative
    .await
    .map_err(|e| RestError::Internal(e.to_string()))?;
```

**Issue**: No comment indicating this is the buggy behavior being fixed.

**Fix**: Add TODO comment before implementation:
```rust
// TODO: Pass relative path after lookup_with_path() is implemented
// Current bug: passes absolute path instead of plugin-relative path
```

---

### 3. handlers.rs:344 - Redundant `trim_end_matches('/')`
**Type**: Minor - unnecessary operation
**Risk**: Low (no behavior change)
**Location**: `handlers.rs:340-350`

**Current Code**:
```rust
let files = evif_file_infos.into_iter().map(|info| {
    FileInfo {
        id: None,
        name: info.name.clone(),
        path: format!("{}/{}", params.path.trim_end_matches('/'), info.name),
        is_dir: info.is_dir,
        size: info.size,
        modified: info.modified.to_rfc3339(),
        created: info.modified.to_rfc3339(),
    }
}).collect();
```

**Issue**: `params.path` is already normalized by `normalize_path()`, making `trim_end_matches('/')` redundant.

**Fix**: Remove redundant operation:
```rust
path: format!("{}/{}", params.path, info.name),
```

---

### 4. radix_mount_table.rs:253 - Private Function Could Be Public
**Type**: API design - function visibility
**Risk**: Low (no behavior change)
**Location**: `radix_mount_table.rs:254-262`

**Current Code**:
```rust
/// 标准化路径
fn normalize_path(path: &str) -> String {
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path)
    }
}
```

**Issue**: `normalize_path()` is private but could be useful for external users.

**Fix**: Not recommended - keep private as it's an implementation detail. External users should use the public API.

---

### 5. handlers.rs:583 - Redundant `lookup()` in `list_mounts()`
**Type**: Performance - unnecessary lookup
**Risk**: Low (minor performance impact)
**Location**: `handlers.rs:576-592`

**Current Code**:
```rust
pub async fn list_mounts(
    State(state): State<AppState>,
) -> RestResult<Json<ListMountsResponse>> {
    let mount_paths = state.mount_table.list_mounts().await;

    let mut mounts = Vec::new();
    for path in mount_paths {
        if let Some(plugin) = state.mount_table.lookup(&path).await {
            mounts.push(MountInfo {
                plugin: plugin.name().to_string(),
                path,
            });
        }
    }

    Ok(Json(ListMountsResponse { mounts }))
}
```

**Issue**: `list_mounts()` returns mount paths, then `lookup()` is called for each path. But `list_mounts()` could return `(path, plugin)` pairs directly.

**Fix**: This would require changing `RadixMountTable::list_mounts()` signature. Not recommended for this fix - out of scope.

---

## Clean Code Observations (NOT broken windows)

### 1. Good: Consistent Error Handling Pattern
**Location**: Throughout `handlers.rs`

```rust
.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;
```

**Assessment**: Excellent pattern - consistent and descriptive.

---

### 2. Good: Comprehensive Unit Tests
**Location**: `radix_mount_table.rs:302-406`

**Assessment**: Good test coverage including:
- Basic mount/lookup/unmount
- Longest prefix matching
- Performance benchmarks
- Path normalization

**Recommendation**: Maintain this quality for new `lookup_with_path()` tests.

---

### 3. Good: Type Aliases for Readability
**Location**: `radix_mount_table.rs`, `handlers.rs`

```rust
type EvifResult<T> = Result<T, EvifError>;
type RestResult<T> = Result<T, RestError>;
```

**Assessment**: Good practice - use type alias for `lookup_with_path()` return type if complex.

---

## Recommendations for Implementation

### DO NOT Fix During This Implementation:
1. Loop variable naming (`i` → `prefix_end_idx`) - idiomatic Rust
2. Private `normalize_path()` visibility - correct design
3. `list_mounts()` redundant lookup - out of scope

### CAN Fix Safely (Low Risk):
1. Add TODO comment explaining the bug at handlers.rs:336
2. Remove redundant `trim_end_matches('/')` at handlers.rs:344

### SHOULD Fix:
1. Write clean, well-commented code for `lookup_with_path()`
2. Add comprehensive unit tests matching existing quality
3. Follow existing patterns for error handling and path normalization

## Code Quality Checklist for New Code

- [ ] Use async fn with &self receiver
- [ ] Follow longest prefix matching algorithm
- [ ] Return (Option<Arc<dyn EvifPlugin>>, String)
- [ ] Handle root path "/" correctly
- [ ] Use descriptive variable names
- [ ] Add doc comment with examples
- [ ] Write unit tests for all edge cases
- [ ] Use consistent error handling
- [ ] Follow existing naming conventions
- [ ] Add Chinese comments for key logic

## Summary

**Broken Windows Found**: 3 minor issues
- 2 worth fixing (redundant operations)
- 1 documentation improvement (TODO comment)

**Risk Level**: LOW
- All issues are cosmetic or minor performance
- No behavior changes required
- Implementation can proceed without fixing

**Codebase Quality**: GOOD
- Consistent patterns throughout
- Good error handling
- Comprehensive test coverage
- Clean, readable code

**Recommendation**: Focus on implementing `lookup_with_path()` cleanly. Optional to fix minor issues during refactor phase.
