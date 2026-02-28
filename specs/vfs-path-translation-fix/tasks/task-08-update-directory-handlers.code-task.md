---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Update Directory Handlers (create_directory, delete_directory)

## Description

Update two directory handlers (`create_directory` and `delete_directory`) to use `lookup_with_path()` instead of `lookup()`. These handlers create and delete directories.

## Background

Continuing the systematic handler updates, we now update the directory manipulation handlers. These handlers are similar to file handlers but operate on directories instead.

The two handlers:
- `create_directory()` (line 359) - Create a new directory
- `delete_directory()` (line 393) - Delete a directory or file

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 3.2 - REST Handler Updates)
- Read this before implementing to understand the change pattern

**Additional References:**
- specs/vfs-path-translation-fix/context.md (Affected Handlers section)
- specs/vfs-path-translation-fix/plan.md (Step 8 details)

## Technical Requirements

1. Update `create_directory()` handler (line 359 in handlers.rs)
2. Update `delete_directory()` handler (line 393 in handlers.rs)
3. Both must use `lookup_with_path()` instead of `lookup()`
4. Both must pass the relative path to plugin methods
5. Follow the established change pattern from Tasks 05-07

## Dependencies

- Requires Tasks 05-07 to be complete (pattern well-established)
- `lookup_with_path()` method must be implemented
- Must understand existing handler patterns

## Implementation Approach

**TDD Cycle:**

1. **Write failing integration tests first** (RED):
   ```rust
   #[tokio::test]
   async fn test_create_directory_in_nested_path() {
       // Setup mount
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Create directory via API
       let response = app
           .oneshot(Request::builder()
               .method("POST")
               .uri("/api/v1/fs/mkdir?path=/mem/new/dir")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 201 Created
       assert_eq!(response.status(), 201);

       // Verify directory was created
       let files = mem.readdir("/new").await.unwrap();
       assert!(!files.is_empty());
   }

   #[tokio::test]
   async fn test_delete_directory_in_nested_path() {
       // Setup mount with directory
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();
       mem.mkdir("/dir/to/delete", true).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Delete directory via API
       let response = app
           .oneshot(Request::builder()
               .method("DELETE")
               .uri("/api/v1/fs/remove?path=/mem/dir/to/delete")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 200 OK
       assert!(response.status().is_success());

       // Verify directory was deleted
       let result = mem.readdir("/dir").await;
       assert!(result.is_err() || result.unwrap().is_empty());
   }
   ```

2. **Implement minimal code to pass** (GREEN):
   - Update `create_directory()` at line 359:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.mkdir(&relative_path, params.recursive).await
   ```

   - Update `delete_directory()` at line 393:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.remove(&relative_path, params.recursive).await
   ```

3. **Refactor while keeping tests green** (REFACTOR):
   - Ensure consistency with previous handler updates
   - Add comments if needed
   - Verify error handling is consistent

## Acceptance Criteria

### 1. create_directory() Creates Directories with Relative Path

- **Given** a mount point `/mem` → MemFsPlugin
- **When** `POST /api/v1/fs/mkdir?path=/mem/new/dir` is requested
- **Then** the plugin receives `mkdir("/new/dir", ...)` and creates the directory

### 2. delete_directory() Deletes Directories with Relative Path

- **Given** a directory at `/mem/dir/to/delete`
- **When** `DELETE /api/v1/fs/remove?path=/mem/dir/to/delete` is requested
- **Then** the plugin receives `remove("/dir/to/delete", ...)` and deletes the directory

### 3. All Integration Tests Pass

- **Given** both integration tests are implemented
- **When** running `cargo test -p evif-rest --test api_contract`
- **Then** `test_create_directory_in_nested_path` and `test_delete_directory_in_nested_path` both pass

### 4. Recursive Operations Work

- **Given** nested directory structures
- **When** creating or deleting with `recursive=true`
- **Then** the operation completes successfully

### 5. No Clippy Warnings

- **Given** the updated handlers
- **When** running `cargo clippy -p evif-rest`
- **Then** there are no clippy warnings related to these handlers

## Metadata

- **Complexity**: Low
- **Labels**: handler-update, phase-2, tdd, directory-operations
- **Required Skills**: Rust async handlers, Axum framework, integration testing

## Demo

When this task is complete, you should be able to:
1. Create directories in nested paths via the UI
2. Delete directories and files in nested paths
3. Perform recursive directory operations
4. Verify all directory operations work correctly

## Connects To

- Previous tasks: Tasks 05-07 (list_directory, file handlers)
- Next task: Task 09 - Update advanced handlers (rename, grep)
- Design: Section 3.2 - REST Handler Updates
- Existing code: handlers.rs lines 359, 393

## Note

These handlers follow the same pattern as file handlers. The key difference is that they operate on directories and may support recursive operations.
