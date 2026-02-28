---
status: completed
created: 2026-02-08
started: 2026-02-08
completed: 2026-02-08
---
# Task: Update list_directory() Handler with Root Special Case

## Description

Update the `list_directory()` REST handler to use `lookup_with_path()` instead of `lookup()`. Add special handling for the root path ("/") to list mount points instead of returning a NotFound error.

## Background

Currently, the `list_directory()` handler at line 325 in `crates/evif-rest/src/handlers.rs` uses `lookup()` which fails for the root path. This is the first handler to be updated as part of Phase 2 - systematic migration of all REST handlers.

The root path ("/") is a special case:
- No plugin owns the root path
- Instead, we should list all available mount points
- This allows the UI to show the top-level directory structure

## Reference Documentation

**Required:**
- Design: specs/vfs-path-translation-fix/design.md (Section 3.2 - REST Handler Updates)
- Read this before implementing to understand the change pattern

**Additional References:**
- specs/vfs-path-translation-fix/context.md (Integration Points section)
- specs/vfs-path-translation-fix/plan.md (Step 5 details)

## Technical Requirements

1. Replace `lookup()` with `lookup_with_path()` in `list_directory()` handler
2. Add root path special case handling:
   - Check if `relative_path == "/"` and `plugin_opt.is_none()`
   - Call `state.mount_table.list_mounts().await` to get mount points
   - Convert mount points to `FileInfo` objects
   - Return JSON response with mount points as directories
3. For non-root paths, use the plugin with the relative path
4. Follow existing error handling patterns

## Dependencies

- Requires Phase 1 to be complete (Tasks 01-04)
- `lookup_with_path()` method must be implemented and tested
- Must understand existing handler patterns in handlers.rs

## Implementation Approach

**TDD Cycle:**

1. **Write failing integration test first** (RED):
   ```rust
   #[tokio::test]
   async fn test_list_root_directory() {
       // Setup test server with mount table
       let mount_table = Arc::new(RadixMountTable::new());
       let app = create_test_app(mount_table.clone());

       // Mount a test plugin
       let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
       mount_table.mount("/mem".to_string(), mem).await.unwrap();

       // Request root listing
       let response = app
           .oneshot(Request::builder()
               .uri("/api/v1/fs/list?path=/")
               .body(Body::empty())
               .unwrap())
           .await
           .unwrap();

       // Assert 200 OK with mount points
       assert!(response.status().is_success());
       let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
       let files: Vec<FileInfo> = serde_json::from_slice(&body).unwrap();
       assert_eq!(files.len(), 1);
       assert_eq!(files[0].name, "mem");
   }
   ```

2. **Implement minimal code to pass** (GREEN):
   - Modify `list_directory()` handler at line 325
   - Replace `lookup()` with `lookup_with_path()`
   - Add root path special case:
   ```rust
   let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;

   // Special case: root path lists mount points
   if relative_path == "/" && plugin_opt.is_none() {
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

3. **Refactor while keeping tests green** (REFACTOR):
   - Extract mount point listing logic if needed
   - Add comments explaining root path handling
   - Ensure code follows existing patterns

## Acceptance Criteria

### 1. Root Path Returns Mount Points

- **Given** a mount table with mounts at `/mem` and `/hello`
- **When** `GET /api/v1/fs/list?path=/` is requested
- **Then** it returns 200 OK with a list of mount points as directories

### 2. Mounted Plugin Root Lists Contents

- **Given** a mount point `/mem` → MemFsPlugin with files
- **When** `GET /api/v1/fs/list?path=/mem` is requested
- **Then** it returns 200 OK with the plugin's root directory contents

### 3. Integration Test Passes

- **Given** the integration test is implemented
- **When** running `cargo test -p evif-rest --test api_contract`
- **Then** `test_list_root_directory` passes

### 4. No Clippy Warnings

- **Given** the updated handler
- **When** running `cargo clippy -p evif-rest`
- **Then** there are no clippy warnings related to this handler

## Metadata

- **Complexity**: Medium
- **Labels**: handler-update, phase-2, tdd, root-handling
- **Required Skills**: Rust async handlers, Axum framework, integration testing

## Demo

When this task is complete, you should be able to:
1. Request `GET /api/v1/fs/list?path=/` and receive mount points
2. Request `GET /api/v1/fs/list?path=/mem` and receive plugin contents
3. Verify that the UI can now display the root directory without errors
4. Run integration tests with 100% pass rate

## Connects To

- Previous phase: Tasks 01-04 (Phase 1 - Core Implementation)
- Next task: Task 06 - Update file read handlers (read, stat, digest)
- Design: Section 3.2 - REST Handler Updates
- Existing code: handlers.rs line 325

## Change Pattern

This task establishes the pattern that will be used for all remaining handlers:
```rust
// BEFORE (broken):
let plugin = state.mount_table.lookup(&params.path).await.ok_or(...)?;
plugin.method(&params.path).await

// AFTER (fixed):
let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
let plugin = plugin_opt.ok_or(...)?;
plugin.method(&relative_path).await
```
