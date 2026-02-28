---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Handle Edge Cases - Non-existent Paths, Deep Nesting, and Nested Mounts

## Description

Implement edge case handling for `lookup_with_path()` including non-existent paths, deeply nested paths, and nested mount point scenarios (longest prefix matching).

## Background

The basic mount lookup and prefix stripping logic is implemented in Task 02. Now we need to verify that edge cases are handled correctly:
1. **Non-existent paths**: Return `(None, "")` when no mount point matches
2. **Deep nesting**: Handle paths like `/hello/world/deep/path` correctly
3. **Nested mounts**: When multiple mount points exist (e.g., `/hello` and `/hello/world`), use longest prefix matching

Most of these cases should already work with the prefix stripping implementation, but we need to write tests to verify and fix any issues.

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 5 - Error Handling)
- Read this before implementing to understand error handling strategy

**Additional References:**
- specs/vfs-path-translation-fix/context.md (edge case considerations)
- specs/vfs-path-translation-fix/plan.md (Step 3 details)

## Technical Requirements

1. Non-existent paths must return `(None, "")`
2. Deep nesting must work correctly (e.g., `/hello/world/deep/path` → `/world/deep/path`)
3. Nested mount points must use longest prefix matching
4. All edge cases must have comprehensive unit tests
5. Add Chinese comments explaining edge case handling

## Dependencies

- Requires Task 02 to be complete (basic prefix stripping works)
- Must verify that existing implementation handles edge cases correctly
- May need to fix bugs if tests fail

## Implementation Approach

**TDD Cycle:**

1. **Write failing tests first** (RED):
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

2. **Verify tests pass or fix implementation** (GREEN):
   - Run tests - most should pass if Task 02 was done correctly
   - Fix longest prefix matching if nested mounts test fails
   - Ensure all edge cases return correct values

3. **Refactor while keeping tests green** (REFACTOR):
   - Add comprehensive Chinese comments for edge cases
   - Improve code clarity if needed
   - Ensure all edge cases are documented

## Acceptance Criteria

### 1. Non-existent Path

- **Given** a mount table with no mounts
- **When** `lookup_with_path("/nonexistent")` is called
- **Then** it returns `(None, "")` - no plugin, empty path

### 2. Deep Nesting

- **Given** a mount point `/hello` → HelloFsPlugin
- **When** `lookup_with_path("/hello/world/deep/path")` is called
- **Then** it returns `(Some(HelloFsPlugin), "/world/deep/path")` - entire nested structure preserved

### 3. Longest Prefix Matching

- **Given** two mount points: `/hello` → Plugin1 and `/hello/world` → Plugin2
- **When** `lookup_with_path("/hello/world/file.txt")` is called
- **Then** it returns `(Some(Plugin2), "/file.txt")` - matches longest prefix `/hello/world`

### 4. Shortest Prefix When No Longer Match

- **Given** two mount points: `/hello` → Plugin1 and `/hello/world` → Plugin2
- **When** `lookup_with_path("/hello/other")` is called
- **Then** it returns `(Some(Plugin1), "/other")` - matches shorter prefix `/hello`

### 5. Unit Tests Pass

- **Given** all three edge case tests are implemented
- **When** running `cargo test -p evif-core --lib radix_mount_table`
- **Then** all 6 unit tests pass (root, simple, nested, nonexistent, deep_nesting, nested_mounts)

## Metadata

- **Complexity**: Medium
- **Labels**: core-implementation, phase-1, tdd, edge-cases
- **Required Skills**: Rust testing, edge case analysis, trie algorithms

## Demo

When this task is complete, you should be able to:
1. Handle non-existent paths gracefully
2. Process deeply nested paths correctly
3. Use longest prefix matching for nested mount points
4. Verify all 6 unit tests pass
5. Run `cargo clippy -p evif-core` with no warnings

## Connects To

- Previous tasks: Task 01 (stub), Task 02 (basic logic)
- Next task: Task 04 - Verification and documentation
- Design reference: Section 5 - Error Handling in design.md

## Note

Most edge cases should already work if Task 02 was implemented correctly. This task is primarily about:
1. Writing comprehensive tests to verify edge cases
2. Fixing any bugs that tests reveal
3. Adding documentation for edge case behavior
