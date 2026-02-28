# Existing Patterns - VFS Path Translation Fix

## Source: Codebase Analysis (2026-02-08)

## 1. RadixMountTable Implementation Patterns

### Location: `crates/evif-core/src/radix_mount_table.rs`

### Pattern 1.1: Async Method Signature with RwLock
**File**: `radix_mount_table.rs:211-235`

```rust
pub async fn lookup(&self, path: &str) -> Option<Arc<dyn EvifPlugin>> {
    let mounts = self.mounts.read().await;
    let normalized_path = Self::normalize_path(path);
    let search_key = normalized_path.trim_start_matches('/').to_string();

    // Radix Tree最长前缀匹配
    let mut best_match: Option<Arc<dyn EvifPlugin>> = None;
    let mut best_len = 0;

    // 检查所有可能的前缀 (从最长到最短)
    for i in (0..=search_key.len()).rev() {
        let prefix = &search_key[..i];

        if let Some(plugin) = mounts.get(prefix) {
            if prefix.len() > best_len {
                best_match = Some(plugin.clone());
                best_len = prefix.len();
            }
        }
    }

    best_match
}
```

**Pattern Characteristics**:
- All methods are `async fn` with `&self` receiver
- Use `Arc<RwLock<Trie>>` for thread-safe access
- Read lock acquisition: `self.mounts.read().await`
- Normalize path before processing
- Return `Arc<dyn EvifPlugin>` for shared ownership
- Longest prefix matching algorithm

### Pattern 1.2: Path Normalization
**File**: `radix_mount_table.rs:254-262`

```rust
fn normalize_path(path: &str) -> String {
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path)
    }
}
```

**Pattern Characteristics**:
- Private helper function (`fn` not `pub`)
- Ensures consistent path format
- Handles root "/" as special case
- Removes leading slash then re-adds it
- Used internally before Trie operations

### Pattern 1.3: Error Handling
**File**: `radix_mount_table.rs:187-199`

```rust
pub async fn unmount(&self, path: &str) -> EvifResult<()> {
    let mut mounts = self.mounts.write().await;
    let normalized_path = Self::normalize_path(path);
    let key = normalized_path.trim_start_matches('/').to_string();

    if mounts.get(&key).is_none() {
        return Err(EvifError::NotFound(format!("Mount point: {}", path)));
    }

    mounts.remove(&key);
    Ok(())
}
```

**Pattern Characteristics**:
- Returns `EvifResult<T>` (alias for `Result<T, EvifError>)`
- Early return on error with descriptive message
- Use write lock for mutations: `self.mounts.write().await`
- Type-specific error variants: `EvifError::NotFound`, `EvifError::AlreadyMounted`

## 2. REST Handler Patterns

### Location: `crates/evif-rest/src/handlers.rs`

### Pattern 2.1: Plugin Lookup and Error Conversion
**File**: `handlers.rs:329-338`

```rust
let plugin = state.mount_table
    .lookup(&params.path)
    .await
    .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

// List files using readdir
let evif_file_infos = plugin
    .readdir(&params.path)
    .await
    .map_err(|e| RestError::Internal(e.to_string()))?;
```

**Pattern Characteristics**:
- Use `?` operator for error propagation
- Convert `Option` to `Result` with `ok_or_else()`
- Map plugin errors to REST errors with `.map_err()`
- Descriptive error messages include the path
- Plugin method calls after successful lookup

### Pattern 2.2: State Access Pattern
**File**: `handlers.rs:326-328`

```rust
pub async fn list_directory(
    State(state): State<AppState>,
    Query(params): Query<FileQueryParams>,
) -> RestResult<Json<DirectoryListResponse>> {
```

**Pattern Characteristics**:
- Extract state with axum's `State` extractor
- Extract query parameters with `Query` extractor
- Return `RestResult<Json<T>>` for JSON responses
- All handlers are `async fn`
- Use `pub async fn` for public API handlers

### Pattern 2.3: Handler Update Pattern
**File**: `handlers.rs:218-244`

```rust
pub async fn read_file(
    State(state): State<AppState>,
    Query(params): Query<FileQueryParams>,
) -> RestResult<Json<FileReadResponse>> {
    let plugin = state.mount_table
        .lookup(&params.path)
        .await
        .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

    let data = plugin
        .read(&params.path, params.offset.unwrap_or(0), params.size.unwrap_or(0))
        .await
        .map_err(|e| RestError::Internal(e.to_string()))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(Json(FileReadResponse {
        content: String::from_utf8_lossy(&data).to_string(),
        data: encoded,
        size: data.len(),
    }))
}
```

**Pattern Characteristics**:
- Lookup plugin first
- Call plugin method with absolute path (BUG - this is the issue!)
- Transform plugin response to REST response
- Base64 encode binary data
- Return structured JSON response

## 3. Testing Patterns

### Location: `crates/evif-core/src/radix_mount_table.rs:302-406`

### Pattern 3.1: Unit Test Structure
**File**: `radix_mount_table.rs:306-316`

```rust
#[tokio::test]
async fn test_radix_mount_and_lookup() {
    let mount_table = RadixMountTable::new();
    let plugin = Arc::new(MockPlugin::new("test"));

    mount_table.mount("/test".to_string(), plugin.clone()).await.unwrap();

    let found = mount_table.lookup("/test/file.txt").await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().name(), "test");
}
```

**Pattern Characteristics**:
- Use `#[tokio::test]` for async tests
- Create new instance for each test
- Use `MockPlugin` for testing
- Assert on `Option` with `is_some()`, `is_none()`
- Use `unwrap()` for infallible operations in tests
- Test both positive and negative cases

### Pattern 3.2: Longest Prefix Matching Test
**File**: `radix_mount_table.rs:343-361`

```rust
#[tokio::test]
async fn test_radix_longest_prefix_match() {
    let mount_table = RadixMountTable::new();
    let plugin1 = Arc::new(MockPlugin::new("root"));
    let plugin2 = Arc::new(MockPlugin::new("sub"));

    mount_table.mount("/root".to_string(), plugin1).await.unwrap();
    mount_table.mount("/root/sub".to_string(), plugin2).await.unwrap();

    // 应匹配更具体的 /root/sub
    let found = mount_table.lookup("/root/sub/file.txt").await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().name(), "sub");

    // 应匹配 /root
    let found = mount_table.lookup("/root/other/file.txt").await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().name(), "root");
}
```

**Pattern Characteristics**:
- Test nested mount points
- Verify longest prefix matching
- Use descriptive assert messages
- Chinese comments for expected behavior
- Test multiple scenarios in one test

### Pattern 3.3: Integration Test Pattern
**Location**: `crates/evif-rest/tests/api_contract.rs:14-43`

```rust
#[tokio::test]
async fn test_list_mounts_returns_mounts_key() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    for _ in 0..50 {
        let url = format!("http://127.0.0.1:{}/api/v1/mounts", port);
        if let Ok(res) = reqwest::get(&url).await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("mounts").is_some());
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not become ready in time");
}
```

**Pattern Characteristics**:
- Spin up test server on random port
- Use `reqwest` for HTTP client
- Retry logic with timeout (50 iterations × 50ms)
- Parse JSON and validate structure
- Panic on failure with descriptive message

## 4. Error Type Patterns

### Location: `crates/evif-core/src/error.rs`

### Pattern 4.1: Error Variants
**File**: `error.rs:10-30`

```rust
pub enum EvifError {
    #[error("Path not found: {0}")]
    NotFound(String),

    #[error("Already mounted at: {0}")]
    AlreadyMounted(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    // ... more variants
}
```

**Pattern Characteristics**:
- Use `#[error()]` attribute for display messages
- Include context in error payload (String)
- Specific error types for different failure modes
- Derive `thiserror::Error` for error handling

### Pattern 4.2: REST Error Conversion
**File**: `handlers.rs:332`

```rust
.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;
```

**Pattern Characteristics**:
- Convert `Option` to `Result` at layer boundaries
- Preserve context in error messages
- Use domain-specific error types (REST vs Core)

## 5. Naming Conventions

### Pattern 5.1: Method Naming
- Lookup methods: `lookup()`, `lookup_with_path()`
- CRUD operations: `mount()`, `unmount()`, `create()`, `read()`, `write()`, `remove()`
- List operations: `list_mounts()`, `list_directory()`
- Path operations: `normalize_path()`, `resolve_symlink()`

### Pattern 5.2: Variable Naming
- Plugin instances: `plugin`, `plugin_from`, `plugin_to`
- Path variables: `path`, `normalized_path`, `relative_path`, `search_key`
- Error variables: `e`, `err`
- Result variables: `found`, `result`, `res`

## 6. Thread Safety Patterns

### Pattern 6.1: Arc + RwLock Usage
**File**: `radix_mount_table.rs:39-43`

```rust
pub struct RadixMountTable {
    mounts: Arc<RwLock<Trie<String, Arc<dyn EvifPlugin>>>>,
    symlinks: Arc<RwLock<HashMap<String, String>>>,
}
```

**Pattern Characteristics**:
- Wrap shared state in `Arc` for shared ownership
- Use `RwLock` for read-write locking
- Multiple readers, single writer semantics
- `.read().await` for read access
- `.write().await` for write access
- Clone `Arc` for cheap references

## Summary for Implementation

**Key Patterns to Follow**:
1. Use `async fn` with `&self` for new method
2. Acquire read lock: `self.mounts.read().await`
3. Reuse `normalize_path()` for consistency
4. Return tuple `(Option<Arc<dyn EvifPlugin>>, String)`
5. Use longest prefix matching algorithm from `lookup()`
6. Write comprehensive unit tests with `#[tokio::test]`
7. Test nested mount points and edge cases
8. Use descriptive error messages with context
