---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Implement lookup_with_path() Method Stub and Root Path Handling

## Description

Implement a new method `lookup_with_path()` in the `RadixMountTable` struct that will return both the plugin and the relative path with mount prefix stripped. This initial implementation will handle the method stub and root path special case.

## Background

The current implementation has a critical bug where REST handlers pass absolute paths (e.g., `/hello`) to plugins, but plugins expect paths relative to their mount point (e.g., `/`). This new method will fix this by stripping the mount prefix from the path and returning both the plugin and the relative path.

This is the first step in a three-phase implementation plan:
- Phase 1: Core implementation of `lookup_with_path()` (Steps 1-4)
- Phase 2: Update all REST handlers (Steps 5-10)
- Phase 3: E2E validation (Step 11)

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md
- Read this before implementing to understand the complete architecture

**Additional References:**
- specs/vfs-path-translation-fix/context.md (codebase patterns)
- specs/vfs-path-translation-fix/plan.md (overall strategy)

## Technical Requirements

1. Add `lookup_with_path()` method to `RadixMountTable` struct in `crates/evif-core/src/radix_mount_table.rs`
2. Method signature must be: `pub async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String)`
3. Method must call existing `normalize_path()` to clean input
4. Method must handle root path ("/") as a special case, returning `(None, "/")`
5. Follow existing code patterns: use async/await, acquire read lock with `self.mounts.read().await`
6. Add Chinese comments explaining key logic

## Dependencies

- Must read existing `RadixMountTable` implementation to understand patterns
- Must reuse existing `normalize_path()` method
- Must follow existing `lookup()` method pattern for acquiring locks

## Implementation Approach

**TDD Cycle:**

1. **Write failing test first** (RED):
   ```rust
   #[tokio::test]
   async fn test_lookup_with_path_root() {
       let table = RadixMountTable::new();
       let (plugin, path) = table.lookup_with_path("/").await;
       assert!(plugin.is_none());
       assert_eq!(path, "/");
   }
   ```

2. **Implement minimal code to pass** (GREEN):
   - Add method stub after line 235 (after existing `lookup()` method)
   - Implement root path early return
   - Make test pass

3. **Refactor while keeping tests green** (REFACTOR):
   - Add rustdoc documentation
   - Add Chinese comments explaining root path logic
   - Ensure code follows existing patterns

## Acceptance Criteria

### 1. Root Path Handling

- **Given** a `RadixMountTable` instance with no mounts
- **When** `lookup_with_path("/")` is called
- **Then** it returns `(None, "/")` - plugin is None, path is "/"

### 2. Method Signature

- **Given** the `RadixMountTable` implementation
- **When** viewing the method signature
- **Then** it matches: `pub async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String)`

### 3. Path Normalization

- **Given** any path input (with or without leading slash, trailing slashes)
- **When** the method processes the input
- **Then** it calls `Self::normalize_path()` to clean the input first

### 4. Unit Tests Pass

- **Given** the implementation is complete
- **When** running `cargo test -p evif-core --lib radix_mount_table::tests::test_lookup_with_path_root`
- **Then** the test passes with no errors

## Metadata

- **Complexity**: Low
- **Labels**: core-implementation, phase-1, tdd
- **Required Skills**: Rust async/await, Arc, RwLock, unit testing with tokio

## Demo

When this task is complete, you should be able to:
1. Call `lookup_with_path("/")` on any `RadixMountTable` instance
2. Receive `(None, "/")` as the result
3. Verify that the unit test passes
4. Run `cargo clippy -p evif-core` with no warnings related to this method

## Connects To

- Next task: Task 02 - Implement mount lookup and prefix stripping logic
- Previous work: Existing `lookup()` and `normalize_path()` methods in `RadixMountTable`
