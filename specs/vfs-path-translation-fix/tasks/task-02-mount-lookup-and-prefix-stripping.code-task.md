---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Implement Mount Lookup and Prefix Stripping Logic

## Description

Implement the core logic of `lookup_with_path()` to find mount points using longest prefix matching and strip the mount prefix from the input path to return the relative path.

## Background

After implementing the root path handling in Task 01, we need to implement the actual mount lookup logic. This involves:
1. Finding the longest matching prefix in the radix trie
2. Stripping that prefix from the input path
3. Returning both the plugin and the relative path

The algorithm reuses the existing longest prefix matching logic from the current `lookup()` method.

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 3.1 - Algorithm)
- Read this before implementing to understand the exact algorithm

**Additional References:**
- specs/vfs-path-translation-fix/context.md (existing code patterns)
- specs/vfs-path-translation-fix/plan.md (Step 2 details)

## Technical Requirements

1. Implement longest prefix matching using the existing radix trie pattern
2. Strip mount prefix from input path using string slicing
3. Preserve leading "/" in relative paths (e.g., "/world" not "world")
4. Return `(Some(plugin), relative_path)` when mount point found
5. Return `(None, "")` when no mount point matches
6. Maintain O(k) performance where k = path length
7. Add Chinese comments explaining the prefix stripping logic

## Dependencies

- Requires Task 01 to be complete (method stub exists)
- Must understand existing `lookup()` method implementation (lines 211-235 in radix_mount_table.rs)
- Must reuse longest prefix matching algorithm from existing code

## Implementation Approach

**TDD Cycle:**

1. **Write failing tests first** (RED):
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

2. **Implement minimal code to pass** (GREEN):
   - Acquire read lock: `self.mounts.read().await`
   - Find longest matching prefix (reuse `lookup()` algorithm)
   - Strip prefix from normalized path
   - Return `(Some(plugin), relative_path)`

3. **Refactor while keeping tests green** (REFACTOR):
   - Extract prefix stripping logic into helper if needed
   - Add comprehensive Chinese comments
   - Ensure code is readable and maintainable

## Algorithm

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

## Acceptance Criteria

### 1. Simple Mount Point

- **Given** a mount point `/hello` → HelloFsPlugin
- **When** `lookup_with_path("/hello")` is called
- **Then** it returns `(Some(HelloFsPlugin), "/")` - entire path stripped

### 2. Nested Path Stripping

- **Given** a mount point `/hello` → HelloFsPlugin
- **When** `lookup_with_path("/hello/world")` is called
- **Then** it returns `(Some(HelloFsPlugin), "/world")` - prefix stripped, leading "/" preserved

### 3. Plugin Matching

- **Given** a mount point with a specific plugin name
- **When** the mount point is found
- **Then** the returned plugin's name() matches the mounted plugin

### 4. Unit Tests Pass

- **Given** the implementation is complete
- **When** running the unit test suite
- **Then** both `test_lookup_with_path_simple` and `test_lookup_with_path_nested` pass

### 5. Performance

- **Given** a mount table with 100 mount points
- **When** performing lookups
- **Then** each lookup completes in < 100μs (maintains O(k) complexity)

## Metadata

- **Complexity**: Medium
- **Labels**: core-implementation, phase-1, tdd, prefix-stripping
- **Required Skills**: Rust string manipulation, trie algorithms, async programming

## Demo

When this task is complete, you should be able to:
1. Mount a plugin at `/hello`
2. Call `lookup_with_path("/hello/world")`
3. Receive the plugin and `"/world"` as the relative path
4. Verify that all tests pass
5. Run `cargo clippy -p evif-core` with no warnings

## Connects To

- Previous task: Task 01 - Method stub and root handling
- Next task: Task 03 - Edge case handling (non-existent paths, deep nesting, nested mounts)
- Existing code: `lookup()` method at lines 211-235 in radix_mount_table.rs
