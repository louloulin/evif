# VFS Path Translation Fix - Rough Idea

## Initial Prompt
Critical VFS path translation bug fix needed: REST handlers pass absolute paths to plugins that expect relative paths. Need to design lookup_with_path() method for RadixMountTable to return (plugin, relative_path) tuple.

## Problem Context
- **Symptom**: `Path not found: /hello`, `/mem`, `/local` when listing subdirectories
- **Root Cause**: REST handlers pass absolute paths to plugins, but plugins expect relative paths
- **Example**: When UI requests `/hello`, handler calls `plugin.readdir("/hello")`
  - Plugin HelloFsPlugin mounted at `/hello` expects `readdir("/")` or `readdir("")`
  - But receives `readdir("/hello")` which returns NotFound
- **File**: `crates/evif-rest/src/handlers.rs:336`

## Proposed Solution
Add `lookup_with_path()` method to RadixMountTable that returns both the plugin and the relative path:
- Returns: `(plugin, relative_path)`
- Strips mount prefix from path
- Example: `lookup_with_path("/hello")` → `(HelloFsPlugin, "/")`

## Impact
- Severity: CRITICAL - Blocks all file operations
- Scope: Affects every plugin operation (read, write, readdir, stat, etc.)
- Broken Features:
  - Browsing mount points
  - Creating files
  - Reading/writing files
  - All file system operations
