---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Update File Write Handlers (write_file, create_file, touch)

## Description

Update three file write handlers (`write_file`, `create_file`, and `touch`) to use `lookup_with_path()` instead of `lookup()`. These handlers create and modify file content.

## Background

Continuing the systematic handler updates from Tasks 05 and 06, we now update the file write handlers. These handlers modify file system state, so thorough testing is important to ensure data integrity.

The three handlers:
- `write_file()` (line 246) - Write content to an existing file
- `create_file()` (line 281) - Create a new file with content
- `touch()` (line 482) - Create an empty file or update timestamp

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 3.2 - REST Handler Updates)
- Read this before implementing to understand the change pattern

**Additional References:**
- specs/vfs-path-translation-fix/context.md (Affected Handlers section)
- specs/vfs-path-translation-fix/plan.md (Step 7 details)

## Technical Requirements

1. Update `write_file()` handler (line 246 in handlers.rs)
2. Update `create_file()` handler (line 281 in handlers.rs)
3. Update `touch()` handler (line 482 in handlers.rs)
4. All three must use `lookup_with_path()` instead of `lookup()`
5. All three must pass the relative path to plugin methods
6. Follow the established change pattern from Tasks 05-06

## Dependencies

- Requires Tasks 05-06 to be complete (pattern well-established)
- `lookup_with_path()` method must be implemented
- Must understand existing handler patterns

## Implementation Approach

**TDD Cycle:**

1. **Write failing integration tests first** (RED):
   ```rust
   #[tokio::test]
   async fn test_create_file_in_nested_path() {
       // Setup mount
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Create file via API
       let response = app
           .oneshot(Request::builder()
               .method("POST")
               .uri("/api/v1/fs/create?path=/mem/new/test.txt")
               .header("Content-Type", "application/octet-stream")
               .body(Body::from("Hello, World!"))
               .unwrap())
           .await
           .unwrap();

       // Assert 201 Created
       assert_eq!(response.status(), 201);

       // Verify file was created
       let content = mem.read("/new/test.txt", 0, None).await.unwrap();
       assert_eq!(String::from_utf8(content).unwrap(), "Hello, World!");
   }

   #[tokio::test]
   async fn test_write_file_in_nested_path() {
       // Setup mount with existing file
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();
       mem.create("/existing.txt", b"Initial".to_vec()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Write to file via API
       let response = app
           .oneshot(Request::builder()
               .method("POST")
               .uri("/api/v1/fs/write?path=/mem/existing.txt")
               .header("Content-Type", "application/octet-stream")
               .body(Body::from("Updated content"))
               .unwrap())
           .await
           .unwrap();

       // Assert 200 OK
       assert!(response.status().is_success());

       // Verify content was updated
       let content = mem.read("/existing.txt", 0, None).await.unwrap();
       assert_eq!(String::from_utf8(content).unwrap(), "Updated content");
   }

   #[tokio::test]
   async fn test_touch_file_in_nested_path() {
       // Setup mount
       let mount_table = Arc::new(RadixMountTable::new());
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

       // Setup test server
       let app = create_test_app(mount_table);

       // Touch new file via API
       let response = app
           .oneshot(Request::builder()
               .method("POST")
               .uri("/api/v1/fs/touch?path=/mem/new.txt")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 201 Created
       assert_eq!(response.status(), 201);

       // Verify empty file was created
       let content = mem.read("/new.txt", 0, None).await.unwrap();
       assert_eq!(content, b"");
   }
   ```

2. **Implement minimal code to pass** (GREEN):
   - Update `write_file()` at line 246:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.write(&relative_path, params.data).await
   ```

   - Update `create_file()` at line 281:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.create(&relative_path, params.data).await
   ```

   - Update `touch()` at line 482:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
   let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(...))?;
   plugin.touch(&relative_path).await
   ```

3. **Refactor while keeping tests green** (REFACTOR):
   - Ensure consistency with previous handler updates
   - Add comments if needed
   - Verify error handling is consistent

## Acceptance Criteria

### 1. create_file() Creates Files with Relative Path

- **Given** a mount point `/mem` → MemFsPlugin
- **When** `POST /api/v1/fs/create?path=/mem/new/test.txt` with content
- **Then** the plugin receives `create("/new/test.txt", ...)` and creates the file

### 2. write_file() Writes to Files with Relative Path

- **Given** a file at `/mem/existing.txt`
- **When** `POST /api/v1/fs/write?path=/mem/existing.txt` with new content
- **Then** the plugin receives `write("/existing.txt", ...)` and updates the file

### 3. touch() Creates Files with Relative Path

- **Given** a mount point `/mem` → MemFsPlugin
- **When** `POST /api/v1/fs/touch?path=/mem/new.txt`
- **Then** the plugin receives `touch("/new.txt")` and creates an empty file

### 4. All Integration Tests Pass

- **Given** all three integration tests are implemented
- **When** running `cargo test -p evif-rest --test api_contract`
- **Then** `test_create_file_in_nested_path`, `test_write_file_in_nested_path`, and `test_touch_file_in_nested_path` all pass

### 5. Data Integrity Verified

- **Given** files created or modified via REST API
- **When** reading the files directly from the plugin
- **Then** the content matches what was written

### 6. No Clippy Warnings

- **Given** the updated handlers
- **When** running `cargo clippy -p evif-rest`
- **Then** there are no clippy warnings related to these handlers

## Metadata

- **Complexity**: Low
- **Labels**: handler-update, phase-2, tdd, file-operations
- **Required Skills**: Rust async handlers, Axum framework, integration testing

## Demo

When this task is complete, you should be able to:
1. Create new files in nested paths via the UI
2. Write content to existing files in nested paths
3. Create empty files with touch command
4. Verify all file operations work correctly

## Connects To

- Previous tasks: Tasks 05-06 (list_directory, file read handlers)
- Next task: Task 08 - Update directory handlers (mkdir, remove)
- Design: Section 3.2 - REST Handler Updates
- Existing code: handlers.rs lines 246, 281, 482

## Note

These handlers modify file system state, so tests verify both the API response and the actual plugin state to ensure data integrity.
