# VFS Path Translation Fix - Requirements

## Overview
Fix critical VFS path translation bug where REST handlers pass absolute paths to plugins that expect relative paths. Add `lookup_with_path()` method to RadixMountTable to return both plugin and relative path.

## Questions & Answers

### Q1: Edge Case Handling for Root and Nested Paths

**Question:**
Should `lookup_with_path()` handle edge cases like:
1. **Root path ('/')**: When listing the root directory, should it return all mount points?
2. **Nested mount points**: What happens when a path like `/hello/world` is requested, but `/hello` is a mount point? Should it return `(HelloFsPlugin, "/world")`?
3. **Non-existent mount points**: Should the method return an error or handle it gracefully?

**Answer:**
Based on analysis of the existing RadixMountTable implementation:

1. **Root path ('/')**: Return `None` or empty relative path
   - Root listing is handled separately by `list_mounts()` method
   - `lookup_with_path("/")` should return `None` to signal "no plugin owns root"
   - This keeps consistency with current `lookup()` behavior

2. **Nested mount points**: Strip mount prefix and return relative path
   - `lookup_with_path("/hello")` → `(Some(plugin), "/")`
   - `lookup_with_path("/hello/world")` → `(Some(plugin), "/world")`
   - `lookup_with_path("/hello/world/deep")` → `(Some(plugin), "/world/deep")`
   - The method uses longest prefix matching (already implemented in `lookup()`)
   - Example: If `/hello` and `/hello/world` are both mounted:
     - Request `/hello/world/file.txt` matches `/hello/world` (longest prefix)
     - Returns `(Plugin2, "/file.txt")`

3. **Non-existent mount points**: Return `None`
   - `lookup_with_path("/nonexistent")` → `(None, "")`
   - Consistent with current `lookup()` which returns `Option<Plugin>`
   - REST handlers will convert `None` to `NotFound` error

## Requirements

### REQ-1: Return Plugin and Relative Path
The method MUST return a tuple containing:
- The plugin instance (if mount point found)
- The relative path with mount prefix stripped

Signature:
```rust
pub async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String)
```

### REQ-2: Handle Root Path
When path is "/", the method MUST:
- Return `(None, "/")` or `(None, "")`
- Signal that no plugin owns the root path
- Allow REST handlers to handle root listing separately

### REQ-3: Strip Mount Prefix
For mounted paths, the method MUST:
- Find the longest matching mount point prefix
- Strip that prefix from the input path
- Return the remaining path as relative path
- Preserve leading "/" for relative paths (e.g., "/world" not "world")

Examples:
- Input: `/hello`, Mounted at: `/hello` → Relative: `/`
- Input: `/hello/world`, Mounted at: `/hello` → Relative: `/world`
- Input: `/hello/world/deep`, Mounted at: `/hello` → Relative: `/world/deep`

### REQ-4: Return None for Non-existent Paths
If no mount point matches the path, the method MUST:
- Return `(None, "")` or `(None, original_path)`
- Allow REST handlers to convert to appropriate error

### REQ-5: Preserve Longest Prefix Matching
The method MUST:
- Use the same longest prefix matching logic as existing `lookup()`
- Match most specific mount point when nested mounts exist
- Maintain O(k) performance where k = path length

### REQ-6: Support Nested Mount Points
If both `/hello` and `/hello/world` are mounted:
- Request `/hello` → matches `/hello` → returns `(Plugin1, "/")`
- Request `/hello/world` → matches `/hello/world` → returns `(Plugin2, "/")`
- Request `/hello/world/file.txt` → matches `/hello/world` → returns `(Plugin2, "/file.txt")`

### REQ-7: Path Normalization
The method MUST:
- Use existing `normalize_path()` for consistency
- Handle leading/trailing slashes correctly
- Preserve path normalization behavior from `lookup()`

## Testing Requirements

### TEST-1: Root Path
```rust
lookup_with_path("/") → (None, "/")
```

### TEST-2: Single Mount Point
```rust
mount_table.mount("/hello", plugin).await
lookup_with_path("/hello") → (Some(plugin), "/")
lookup_with_path("/hello/world") → (Some(plugin), "/world")
```

### TEST-3: Non-existent Path
```rust
lookup_with_path("/nonexistent") → (None, "")
```

### TEST-4: Nested Mount Points
```rust
mount_table.mount("/hello", plugin1).await
mount_table.mount("/hello/world", plugin2).await
lookup_with_path("/hello") → (Some(plugin1), "/")
lookup_with_path("/hello/world") → (Some(plugin2), "/")
lookup_with_path("/hello/world/file.txt") → (Some(plugin2), "/file.txt")
```

### TEST-5: Deep Nested Paths
```rust
lookup_with_path("/hello/world/deep/path") → (Some(plugin), "/deep/path")
```
