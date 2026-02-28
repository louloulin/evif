# VFS Path Translation Fix - Questions & Answers

## Q1: Edge Case Handling for Root and Nested Paths (2026-02-08)

### Question
Should `lookup_with_path()` handle edge cases like:
1. **Root path ('/')**: When listing the root directory, should it return all mount points?
2. **Nested mount points**: What happens when a path like `/hello/world` is requested, but `/hello` is a mount point? Should it return `(HelloFsPlugin, "/world")`?
3. **Non-existent mount points**: Should the method return an error or handle it gracefully?

### Context
The current bug shows that plugins receive `/hello` when they expect `/`, but we need to clarify:
- How to handle the root listing (currently works: returns `/hello`, `/local`, `/mem`)
- How to strip the mount prefix correctly for nested paths
- Whether mount points can be nested (e.g., `/hello` and `/hello/world` both mounted)

### Answer (2026-02-08)

Based on analysis of the existing RadixMountTable implementation:

1. **Root path ('/')**: Return `(None, "/")` or `(None, "")`
   - Root listing is handled separately by `list_mounts()` method
   - `lookup_with_path("/")` should return `None` to signal "no plugin owns root"
   - This keeps consistency with current `lookup()` behavior

2. **Nested mount points**: Strip mount prefix and return relative path
   - `lookup_with_path("/hello")` → `(Some(plugin), "/")`
   - `lookup_with_path("/hello/world")` → `(Some(plugin), "/world")`
   - The method uses longest prefix matching (already implemented in `lookup()`)
   - Example: If `/hello` and `/hello/world` are both mounted:
     - Request `/hello/world/file.txt` matches `/hello/world` (longest prefix)
     - Returns `(Plugin2, "/file.txt")`

3. **Non-existent mount points**: Return `(None, "")`
   - `lookup_with_path("/nonexistent")` → `(None, "")`
   - Consistent with current `lookup()` which returns `Option<Plugin>`
   - REST handlers will convert `None` to `NotFound` error

**Key Design Decisions:**
- Return tuple `(Option<Plugin>, String)` for clear error handling
- Preserve leading "/" in relative paths for consistency
- Use existing longest prefix matching algorithm
- Maintain O(k) performance complexity
