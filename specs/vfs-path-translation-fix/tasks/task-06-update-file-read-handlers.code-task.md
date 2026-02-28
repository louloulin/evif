---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Update File Read Handlers (read_file, stat, digest)

## Description

Update three file read handlers (`read_file`, `stat`, and `digest`) to use `lookup_with_path()` instead of `lookup()`. These handlers read file content, metadata, and checksums respectively.

## Background

After updating `list_directory()` in Task 05, we continue with the file read handlers. These handlers all follow the same pattern:
1. Look up the plugin
2. Call the plugin's method with a path
3. Return the result

The fix is identical for all three: use `lookup_with_path()` and pass the relative path to the plugin.

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 3.2 - REST Handler Updates)
- Read this before implementing to understand the change pattern

**Additional References:**
- specs/vfs-path-translation-fix/context.md (Affected Handlers section)
- specs/vfs-path-translation-fix/plan.md (Step 6 details)

## Technical Requirements

1. Update `read_file()` handler (line 218 in handlers.rs)
2. Update `stat()` handler (line 417 in handlers.rs)
3. Update `digest()` handler (line 441 in handlers.rs)
4. All three must use `lookup_with_path()` instead of `lookup()`
5. All three must pass the relative path to plugin methods
6. Follow the established change pattern from Task 05

## Dependencies

- Requires Task 05 to be complete (pattern established)
- `lookup_with_path()` method must be implemented
- Must understand existing handler patterns

## Implementation Approach

**TDD Cycle:**

1. **Write failing integration tests first** (RED):
   ```rust
   #[tokio::test]
   async fn test_read_file_in_nested_path() {
       // Setup mount with nested file
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

       // Create test file
       mem.create("/nested/test.txt", b"Hello, World!".to_vec()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Request file read
       let response = app
           .oneshot(Request::builder()
               .uri("/api/v1/fs/read?path=/mem/nested/test.txt")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 200 OK with file content
       assert!(response.status().is_success());
   }

   #[tokio::test]
   async fn test_stat_file_in_nested_path() {
       // Similar setup for stat endpoint
       // Request: GET /api/v1/fs/stat?path=/mem/nested/test.txt
       // Assert: 200 OK with file metadata
   }

   #[tokio::test]
   async fn test_digest_file_in_nested_path() {
       // Similar setup for digest endpoint
       // Request: GET /api/v1/fs/digest?path=/mem/nested/test.txt
       // Assert: 200 OK with checksum
   }
   ```

2. **Implement minimal code to pass** (GREEN):
   - Update `read_file()` at line 218:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.read(&relative_path, params.offset, params.size).await
   ```

   - Update `stat()` at line 417:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.stat(&relative_path).await
   ```

   - Update `digest()` at line 441:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.digest(&relative_path).await
   ```

3. **Refactor while keeping tests green** (REFACTOR):
   - Ensure consistency across all three handlers
   - Add comments if needed
   - Verify error handling is consistent

## Acceptance Criteria

### 1. read_file() Uses Relative Path

- **Given** a file at `/mem/nested/test.txt`
- **When** `GET /api/v1/fs/read?path=/mem/nested/test.txt` is requested
- **Then** the plugin receives `read("/nested/test.txt")` and returns file content

### 2. stat() Uses Relative Path

- **Given** a file at `/mem/nested/test.txt`
- **When** `GET /api/v1/fs/stat?path=/mem/nested/test.txt` is requested
- **Then** the plugin receives `stat("/nested/test.txt")` and returns metadata

### 3. digest() Uses Relative Path

- **Given** a file at `/mem/nested/test.txt`
- **When** `GET /api/v1/fs/digest?path=/mem/nested/test.txt` is requested
- **Then** the plugin receives `digest("/nested/test.txt")` and returns checksum

### 4. All Integration Tests Pass

- **Given** all three integration tests are implemented
- **When** running `cargo test -p evif-rest --test api_contract`
- **Then** `test_read_file_in_nested_path`, `test_stat_file_in_nested_path`, and `test_digest_file_in_nested_path` all pass

### 5. No Clippy Warnings

- **Given** the updated handlers
- **When** running `cargo clippy -p evif-rest`
- **Then** there are no clippy warnings related to these handlers

## Metadata

- **Complexity**: Low
- **Labels**: handler-update, phase-2, tdd, file-operations
- **Required Skills**: Rust async handlers, Axum framework, integration testing

## Demo

When this task is complete, you should be able to:
1. Read files in nested paths via REST API
2. Get file metadata for nested paths
3. Calculate checksums for files in nested paths
4. Verify all operations work correctly through the UI

## Connects To

- Previous task: Task 05 - Update list_directory() handler
- Next task: Task 07 - Update file write handlers (write, create, touch)
- Design: Section 3.2 - REST Handler Updates
- Existing code: handlers.rs lines 218, 417, 441

## Note

This task is straightforward because all three handlers follow the exact same pattern established in Task 05. The key is consistency and thorough testing.
