---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Update Advanced Handlers (rename, grep)

## Description

Update two advanced handlers (`rename` and `grep`) to use `lookup_with_path()` instead of `lookup()`. These handlers perform more complex operations: renaming/moving files and searching file contents.

## Background

The final handler update task covers the most complex operations:
- `rename()` (line 536) - Move or rename a file/directory (requires TWO path lookups)
- `grep()` (line 514) - Search for text patterns in files (recursive directory handling)

These handlers are slightly more complex because:
- `rename()` needs to look up both source and destination paths
- `grep()` needs to handle recursive directory traversal

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 3.2 - REST Handler Updates)
- Read this before implementing to understand the change pattern

**Additional References:**
- specs/vfs-path-translation-fix/context.md (Affected Handlers section)
- specs/vfs-path-translation-fix/plan.md (Step 9 details)

## Technical Requirements

1. Update `rename()` handler (line 536 in handlers.rs)
2. Update `grep()` handler (line 514 in handlers.rs)
3. `rename()` must use `lookup_with_path()` for BOTH source and destination paths
4. `grep()` must use `lookup_with_path()` for the search path
5. Follow the established change pattern from Tasks 05-08

## Dependencies

- Requires Tasks 05-08 to be complete (pattern well-established)
- `lookup_with_path()` method must be implemented
- Must understand existing handler patterns

## Implementation Approach

**TDD Cycle:**

1. **Write failing integration tests first** (RED):
   ```rust
   #[tokio::test]
   async fn test_rename_file_in_nested_path() {
       // Setup mount with file
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();
       mem.create("/old.txt", b"content".to_vec()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Rename file via API
       let response = app
           .oneshot(Request::builder()
               .method("POST")
               .uri("/api/v1/fs/rename?from=/mem/old.txt&to=/mem/new.txt")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 200 OK
       assert!(response.status().is_success());

       // Verify file was renamed
       let result = mem.read("/old.txt", 0, None).await;
       assert!(result.is_err()); // Old path doesn't exist

       let content = mem.read("/new.txt", 0, None).await.unwrap();
       assert_eq!(content, b"content");
   }

   #[tokio::test]
   async fn test_grep_in_nested_path() {
       // Setup mount with files containing content
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();
       mem.mkdir("/nested", true).await.unwrap();
       mem.create("/nested/file1.txt", b"Hello World".to_vec()).await.unwrap();
       mem.create("/nested/file2.txt", b"Goodbye World".to_vec()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Search via API
       let response = app
           .oneshot(Request::builder()
               .uri("/api/v1/fs/grep?path=/mem/nested&pattern=World")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 200 OK with results
       assert!(response.status().is_success());
       let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
       let results: Vec<GrepResult> = serde_json::from_slice(&body).unwrap();
       assert_eq!(results.len(), 2); // Both files contain "World"
   }
   ```

2. **Implement minimal code to pass** (GREEN):
   - Update `rename()` at line 536:
   ```rust
   // Look up both source and destination
   let (src_plugin_opt, src_relative_path) = state.mount_table.lookup_with_path(&params.from).await;
   let (dst_plugin_opt, dst_relative_path) = state.mount_table.lookup_with_path(&params.to).await;

   let src_plugin = src_plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   let dst_plugin = dst_plugin_opt.ok_or_else(|| RestError::NotFound(...))?;

   // Ensure both paths are in the same plugin
   if !Arc::ptr_eq(&src_plugin, &dst_plugin) {
       return Err(RestError::BadRequest("Cannot move across mount points".to_string()));
   }

   src_plugin.rename(&src_relative_path, &dst_relative_path).await
   ```

   - Update `grep()` at line 514:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.grep(&relative_path, &params.pattern, params.recursive).await
   ```

3. **Refactor while keeping tests green** (REFACTOR):
   - Ensure consistency with previous handler updates
   - Add comments explaining the cross-mount check in rename()
   - Verify error handling is consistent

## Acceptance Criteria

### 1. rename() Handles Two Path Lookups

- **Given** a file at `/mem/old.txt`
- **When** `POST /api/v1/fs/rename?from=/mem/old.txt&to=/mem/new.txt` is requested
- **Then** the plugin receives `rename("/old.txt", "/new.txt")` and renames the file

### 2. rename() Rejects Cross-Mount Operations

- **Given** a file at `/mem/file.txt` and mount point `/hello`
- **When** attempting to rename from `/mem/file.txt` to `/hello/file.txt`
- **Then** it returns a BadRequest error explaining cross-mount moves are not allowed

### 3. grep() Searches in Nested Paths

- **Given** files at `/mem/nested/file1.txt` and `/mem/nested/file2.txt` containing "World"
- **When** `GET /api/v1/fs/grep?path=/mem/nested&pattern=World` is requested
- **Then** the plugin receives `grep("/nested", "World", ...)` and returns matching results

### 4. All Integration Tests Pass

- **Given** both integration tests are implemented
- **When** running `cargo test -p evif-rest --test api_contract`
- **Then** `test_rename_file_in_nested_path` and `test_grep_in_nested_path` both pass

### 5. No Clippy Warnings

- **Given** the updated handlers
- **When** running `cargo clippy -p evif-rest`
- **Then** there are no clippy warnings related to these handlers

## Metadata

- **Complexity**: Medium
- **Labels**: handler-update, phase-2, tdd, advanced-operations
- **Required Skills**: Rust async handlers, Axum framework, integration testing

## Demo

When this task is complete, you should be able to:
1. Rename and move files in nested paths via the UI
2. Search for text patterns in files within nested directories
3. Verify cross-mount moves are properly rejected
4. Verify all advanced operations work correctly

## Connects To

- Previous tasks: Tasks 05-08 (list_directory, file handlers, directory handlers)
- Next task: Task 10 - Integration test suite verification
- Design: Section 3.2 - REST Handler Updates
- Existing code: handlers.rs lines 514, 536

## Note

The `rename()` handler is special because it needs to look up two paths and verify they're in the same plugin. This prevents moving files across mount points, which would require complex data transfer logic.
