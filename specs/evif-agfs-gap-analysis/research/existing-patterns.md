# Existing Patterns - EVIF Phase 0 Implementation

**Research Date**: 2025-02-08
**Task**: evif-agfs-gap-analysis
**Focus**: Phase 0 edge case recovery implementation

## 1. Error Handling Patterns

### EvifError Type Hierarchy
**Source**: `crates/evif-core/src/error.rs:9-109`

```rust
pub enum EvifError {
    Io(io::Error),                          // Line 11-12
    NotFound(String),                       // Line 14-15
    AlreadyExists(String),                  // Line 17-18
    AlreadyMounted(String),                 // Line 20-21
    NotMounted(String),                     // Line 23-24
    InvalidPath(String),                    // Line 26-27  ⚠️ OVERUSED in HTTPFS
    InvalidArgument(String),                // Line 29-30
    InvalidInput(String),                   // Line 32-33
    PermissionDenied(String),               // Line 35-36
    ReadOnly,                               // Line 38-39
    NotSupported { ... },                   // Line 41-42
    NotSupportedGeneric,                    // Line 44-45
    Http(String),                           // Line 59-60
    Network(String),                        // Line 63-64  ⚠️ UNDERUSED in HTTPFS
    Timeout(u64),                           // Line 65-66  ⚠️ UNDERUSED in HTTPFS
    // ... other variants
}
```

**Type Alias**: `EvifResult<T> = Result<T, EvifError>` (Line 7)

**Pattern**:
- All errors use `thiserror` derive macro (Line 9)
- String-based error messages with context
- Automatic conversions via `From` trait implementations (Lines 112-137)

**Issue Found**:
- HTTPFS uses `EvifError::InvalidPath` for ALL HTTP errors (see `httpfs.rs:62,88,110`)
- Should use `EvifError::Timeout` for timeouts (Line 65-66)
- Should use `EvifError::Network` for connection errors (Line 63-64)

---

## 2. Async/Timeout Patterns

### HTTPFS Timeout Implementation
**Source**: `crates/evif-plugins/src/httpfs.rs:53-74`

```rust
async fn http_get(&self, path: &str) -> EvifResult<reqwest::Response> {
    let url = self.build_url(path);
    let timeout = std::time::Duration::from_secs(self.timeout_seconds);  // Line 55

    let response = self.client
        .get(&url)
        .timeout(timeout)                          // Line 59 - reqwest timeout
        .send()
        .await
        .map_err(|e| EvifError::InvalidPath(      // ⚠️ WRONG ERROR TYPE
            format!("HTTP GET failed: {}", e)))?; // Line 62

    if response.status().is_success() {
        Ok(response)
    } else if response.status() == 404 {
        Err(EvifError::NotFound(path.to_string()))
    } else {
        Err(EvifError::InvalidPath(format!(
            "HTTP error: {}",
            response.status()
        )))
    }
}
```

**Pattern**:
- Uses `reqwest::Client` with `.timeout()` method (Line 59)
- Timeout is configurable per-instance (Line 23: `timeout_seconds: u64`)
- All HTTP operations (GET, PUT, DELETE, HEAD) follow same pattern:
  - Build URL → Set timeout → Send request → Check status → Return/error

**Dependency**: `reqwest = { version = "0.12", features = ["json"] }` (Cargo.toml:37)

**Issues Found**:
1. No retry logic - single attempt only
2. Error type misclassification - all errors → `InvalidPath`
3. No jitter for thundering herd prevention
4. No exponential backoff

---

## 3. FUSE Error Handling Patterns

### FUSE read() Implementation
**Source**: `crates/evif-fuse/src/lib.rs:559-590`

```rust
fn read(
    &mut self,
    _req: &Request<'_>,
    ino: u64,
    _fh: u64,
    offset: i64,
    size: u32,
    _flags: i32,
    _lock_owner: Option<u64>,
    reply: ReplyData,
) {
    let path = self.inode_manager.get_path(ino);
    if path.is_none() {
        reply.error(libc::ENOENT);          // Line 574 - immediate error return
        return;
    }

    let path = PathBuf::from(path.unwrap());
    let rt = self.runtime.clone();

    match rt.block_on(self.read_async(&path, offset as u64, size)) {
        Ok(data) => {
            reply.data(&data);               // Line 583 - success
        }
        Err(e) => {
            error!("read error: {}", e);
            reply.error(libc::EIO);          // Line 587 - ⚠️ GENERIC ERROR
        }
    }
}
```

**Pattern**:
- FUSE operations use `runtime.block_on()` for async → sync bridging (Line 581)
- Errors mapped to libc error codes:
  - `ENOENT` for not found (Line 574)
  - `EIO` for all other errors (Line 587) - **too generic**
- Uses `tracing` crate: `error!`, `debug!`, `info!`, `warn!` (Line 30)

**FUSE write() Pattern** (Lines 593-629):
- Similar structure with `reply.error(libc::EIO)` on failure (Line 626)
- Checks `allow_write` flag before writes (Line 606-609)

**Issues Found**:
1. No retry logic for transient failures (EIO, EINTR)
2. Generic `EIO` for all errors loses context
3. No distinction between transient (retryable) and permanent errors

---

## 4. Plugin Trait Pattern

### EvifPlugin Trait Definition
**Source**: `crates/evif-core/src/plugin.rs:151-270+`

```rust
#[async_trait]
pub trait EvifPlugin: Send + Sync {
    fn name(&self) -> &str;

    // Core operations
    async fn create(&self, path: &str, perm: u32) -> EvifResult<()>;
    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>;
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;
    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
        -> EvifResult<u64>;
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;
    async fn remove(&self, path: &str) -> EvifResult<()>;
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;
    async fn remove_all(&self, path: &str) -> EvifResult<()>;

    // Optional methods with default implementations
    async fn symlink(&self, src: &str, dst: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn readlink(&self, path: &str) -> EvifResult<String> {
        Err(EvifError::NotSupportedGeneric)
    }
    // ... more optional methods
}
```

**Pattern**:
- Uses `#[async_trait]` macro for async trait methods
- All methods return `EvifResult<T>`
- Optional methods provide default `Err(EvifError::NotSupportedGeneric)` implementation
- No health check method currently exists

**Integration Point**:
- Add `async fn health(&self) -> EvifResult<()>` as optional method
- Default implementation: `Ok(())` (healthy)
- Plugins can override for custom health checks

---

## 5. REST Handler Pattern

### Plugin Handlers Structure
**Source**: `crates/evif-rest/src/plugin_handlers.rs:66-339`

```rust
pub struct PluginHandlers;

impl PluginHandlers {
    /// GET /api/v1/plugins
    pub async fn list_plugins(
        State(state): State<PluginState>,
    ) -> Result<Json<Vec<PluginInfo>>, PluginError> {
        let mount_paths = state.mount_table.list_mounts().await;
        // ... implementation
    }

    /// GET /api/v1/plugins/{name}/config
    pub async fn get_plugin_config(
        State(_state): State<PluginState>,
        Path(name): Path<String>,
    ) -> Result<Json<PluginConfigSchema>, PluginError> {
        match name.as_str() {
            "localfs" => Ok(Json(PluginConfigSchema { ... })),
            // ... hardcoded schemas
            _ => Err(PluginError::NotFound(format!("Plugin not found: {}", name))),
        }
    }
}
```

**Pattern**:
- Uses `axum` framework with `State`, `Path`, `Query` extractors
- Returns `Result<Json<T>, PluginError>` for automatic error conversion
- `PluginError` enum implements `IntoResponse` (Lines 344-365)
- Error mapping:
  - `PluginError::NotFound` → 404
  - `PluginError::BadRequest` → 400
  - `PluginError::Internal` → 500

**Integration Point**:
- Add new handler: `GET /api/v1/plugins/{name}/health`
- Returns: `{"status": "ok"|"error", "message": "..."}`
- Call `plugin.health()` method
- Map `Ok(())` → `{"status": "ok"}`
- Map `Err(e)` → `{"status": "error", "message": e}`

---

## 6. Testing Patterns

### Test Directory Structure
```
crates/
├── evif-fuse/tests/
│   └── fuse_integration_test.rs
├── evif-rest/tests/
│   └── api_contract.rs
├── evif-plugins/tests/
│   ├── sqlfs_tests.rs
│   └── (other plugin tests)
└── evif-cli/tests/
```

**Dependencies** (workspace Cargo.toml:67-71):
```toml
[workspace.dependencies]
proptest = "1.4.0"      # Property-based testing
criterion = "0.5.1"     # Benchmarking
tokio-test = "0.4.3"    # Async testing
tempfile = "3.8"        # Temporary files/folders
```

**Test Pattern** (from `httpfs.rs:277-305`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_httpfs_basic() {
        let plugin = HttpFsPlugin::new("https://httpbin.org", 30);

        match plugin.read("/get", 0, 1024).await {
            Ok(data) => {
                assert!(!data.is_empty());
                println!("HTTP read success: {} bytes", data.len());
            }
            Err(e) => {
                println!("HTTP read failed (expected in offline mode): {:?}", e);
            }
        }
    }

    #[test]
    fn test_httpfs_url_building() {
        let plugin = HttpFsPlugin::new("http://localhost:8080", 10);
        assert_eq!(plugin.build_url(""), "http://localhost:8080/");
        // ... assertions
    }
}
```

**Pattern**:
- Unit tests in same file under `#[cfg(test)]` module
- Integration tests in `tests/` directory
- Uses `tokio::test` for async tests
- Accepts failures in offline mode (defensive testing)

---

## 7. Logging Patterns

**Source**: `crates/evif-fuse/src/lib.rs:30`
```rust
use tracing::{debug, error, info, warn};
```

**Usage Pattern**:
- `debug!()` - Detailed operation info (Lines 113, 217, 269, etc.)
- `info!()` - High-level events (Lines 1132, 1157, etc.)
- `warn!()` - Recoverable issues (NOT CURRENTLY USED)
- `error!()` - Failures (Lines 270, 284, 446, etc.)

**Integration Point**:
- Phase 0 retry logic should use `warn!()` on retry attempts
- Phase 0 retry logic should use `error!()` on final failure

---

## 8. Async Runtime Pattern

### Runtime Creation
**Source**: `crates/evif-fuse/src/lib.rs:72-77`

```rust
let runtime = Arc::new(
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| EvifError::Internal(format!("Failed to create runtime: {}", e)))?
);
```

**Usage**:
- `rt.block_on(async move { ... })` for async → sync bridging (Lines 278, 328, 454)
- Clone `Arc<Runtime>` before moving into async blocks (Line 276)
- Pattern: `let rt = self.runtime.clone();`

---

## 9. Dependencies Summary

### Already Available:
- `tokio = { version = "1.35", features = ["full"] }` ✅ (async runtime, includes time/sleep)
- `reqwest = { version = "0.12", features = ["json"] }` ✅ (HTTP client with timeout)
- `tracing = "0.1.40"` ✅ (logging)
- `thiserror = "1.0.56"` ✅ (error handling)

### Need to Add:
- **None** - all Phase 0 dependencies already available!

**Key Finding**: No additional dependencies required for Phase 0 implementation.
