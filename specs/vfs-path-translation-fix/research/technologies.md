# Technologies and Dependencies - VFS Path Translation Fix

## Source: Cargo.toml Analysis (2026-02-08)

## 1. Core Dependencies

### 1.1 Radix Tree Library
**Crate**: `radix_trie`
**Location**: `crates/evif-core/Cargo.toml`

**Usage in Codebase**:
```rust
use radix_trie::{Trie, TrieCommon};
```

**Key Types**:
- `Trie<K, V>`: Radix tree data structure
- `TrieCommon`: Common trait for trie operations

**Relevant Methods**:
- `trie.get(key)`: Get value by key
- `trie.insert(key, value)`: Insert key-value pair
- `trie.remove(key)`: Remove key
- `trie.iter()`: Iterate over entries

**Pattern for Path Matching**:
```rust
// From: radix_mount_table.rs:225-230
for i in (0..=search_key.len()).rev() {
    let prefix = &search_key[..i];
    if let Some(plugin) = mounts.get(prefix) {
        // Found match
    }
}
```

### 1.2 Async Runtime
**Crate**: `tokio`
**Version**: Latest compatible
**Features**: `sync`, `rt-multi-thread`

**Key Types**:
- `tokio::sync::RwLock`: Async read-write lock
- `tokio::sync::Mutex`: Async mutex
- `tokio::spawn`: Spawn async tasks

**Usage Pattern**:
```rust
let mounts = self.mounts.read().await;
let mut mounts = self.mounts.write().await;
```

### 1.3 Thread Safety
**Crate**: `std::sync`
**Key Types**:
- `Arc<T>`: Atomic reference counting
- `Arc::clone(&item)`: Clone Arc pointer

**Usage Pattern**:
```rust
pub struct RadixMountTable {
    mounts: Arc<RwLock<Trie<String, Arc<dyn EvifPlugin>>>>,
}
```

## 2. Plugin System

### 2.1 Plugin Trait
**Location**: `crates/evif-core/src/plugin.rs`

**Key Methods**:
```rust
pub trait EvifPlugin: Send + Sync {
    fn name(&self) -> &str;
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;
    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<u64>;
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;
    async fn create(&self, path: &str, perm: u32) -> EvifResult<()>;
    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>;
    async fn remove(&self, path: &str) -> EvifResult<()>;
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;
    // ... more methods
}
```

**Type Alias**:
```rust
type EvifResult<T> = Result<T, EvifError>;
```

### 2.2 Available Plugin Implementations
**Crate**: `evif-plugins`

**Plugins**:
1. `MemFsPlugin`: In-memory filesystem
2. `HelloFsPlugin`: Hello world demo plugin
3. `LocalFsPlugin`: Local filesystem wrapper

**Usage in Handlers**:
```rust
use evif_plugins::{MemFsPlugin, HelloFsPlugin, LocalFsPlugin};

let plugin: Arc<dyn EvifPlugin> = match plugin_name.as_str() {
    "mem" => Arc::new(MemFsPlugin::new()),
    "hello" => Arc::new(HelloFsPlugin::new()),
    "local" => Arc::new(LocalFsPlugin::new(&root)),
    _ => return Err(...),
};
```

## 3. Error Handling

### 3.1 Error Types
**Crate**: `thiserror` (implied by usage pattern)
**Location**: `crates/evif-core/src/error.rs`

**Core Error Type**:
```rust
pub enum EvifError {
    #[error("Path not found: {0}")]
    NotFound(String),

    #[error("Already mounted at: {0}")]
    AlreadyMounted(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // ... more variants
}
```

**REST Error Type**:
**Location**: `crates/evif-rest/src/error.rs`

```rust
pub enum RestError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}
```

### 3.2 Error Conversion Patterns

**Option to Result**:
```rust
let plugin = state.mount_table
    .lookup(&params.path)
    .await
    .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;
```

**Plugin Error to REST Error**:
```rust
plugin.readdir(&params.path)
    .await
    .map_err(|e| RestError::Internal(e.to_string()))?;
```

## 4. REST API Framework

### 4.1 Web Framework
**Crate**: `axum`
**Version**: Latest compatible
**Features**: Full feature set

**Key Extractors**:
```rust
use axum::{
    Json,
    extract::{Path, State, Query},
};

// Extract state
State(state): State<AppState>

// Extract path parameters
Path(id): Path<String>

// Extract query parameters
Query(params): Query<FileQueryParams>

// Return JSON
Json(response_data)
```

### 4.2 Response Types
**Location**: `crates/evif-rest/src/handlers.rs`

```rust
pub type RestResult<T> = Result<T, RestError>;

// Usage
pub async fn handler() -> RestResult<Json<Response>> {
    Ok(Json(Response { ... }))
}
```

## 5. Testing Infrastructure

### 5.1 Test Dependencies
**Crate**: `tokio` (test features)

**Test Attribute**:
```rust
#[tokio::test]
async fn test_name() {
    // Test code
}
```

### 5.2 Integration Test Dependencies
**Location**: `crates/evif-rest/tests/api_contract.rs`

```toml
[dev-dependencies]
reqwest = { version = "*", features = ["json"] }
tokio = { version = "*", features = ["full"] }
```

**Test Server Pattern**:
```rust
let app = create_routes(mount_table);
let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap();
let port = addr.port();

tokio::spawn(async move {
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve");
});
```

### 5.3 HTTP Client
**Crate**: `reqwest`
**Features**: `json`

**Usage**:
```rust
let client = reqwest::Client::new();
let res = client.get(&url).send().await?;
let json: serde_json::Value = res.json().await?;
```

## 6. Serialization

### 6.1 JSON Serialization
**Crate**: `serde`
**Features**: `derive`

**Usage**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileQueryParams {
    pub path: String,
}
```

### 6.2 Base64 Encoding
**Crate**: `base64`
**Usage**:
```rust
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

let encoded = STANDARD.encode(&data);
let decoded = STANDARD.decode(&encoded)?;
```

## 7. Time Handling

### 7.1 Timestamps
**Crate**: `chrono`

**Usage**:
```rust
use chrono::Utc;

let timestamp = Utc::now().to_rfc3339();
```

## 8. No Additional Dependencies Needed

**Key Finding**: Implementation requires NO new dependencies
- All required types are already available
- `radix_trie` provides needed trie operations
- `tokio::sync::RwLock` provides thread safety
- Standard library provides string manipulation
- Existing `EvifPlugin` trait provides plugin interface

## 9. String Operations

### 9.1 Path Manipulation
**Library**: `std::string::String`
**Methods Available**:
- `str::trim_start_matches('/')`: Remove prefix
- `str::len()`: Get length
- `String::from("/")`: Create from string literal
- `format!("/{}", path)`: Format with leading slash

**Example**:
```rust
// Strip mount prefix
let relative_path = if mount_key.is_empty() {
    "/".to_string()
} else {
    format!("/{}", &search_key[mount_key.len()..])
};
```

## 10. Build System

### 10.1 Build Commands
```bash
# Build all crates
cargo build

# Build with output
cargo build --verbose

# Build release
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test test_lookup_with_path

# Run tests in package
cargo test -p evif-core

# Run integration tests
cargo test --test api_contract
```

### 10.2 Development Commands
```bash
# Check compilation (faster than build)
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Run documentation generation
cargo doc --open
```

## Summary

**Available Technologies**:
- ✅ Radix tree: `radix_trie` crate
- ✅ Async runtime: `tokio` with `RwLock`
- ✅ Thread safety: `Arc<T>` for shared ownership
- ✅ Error handling: `thiserror` for custom errors
- ✅ Web framework: `axum` for REST API
- ✅ Testing: `tokio::test` for async tests
- ✅ HTTP client: `reqwest` for integration tests
- ✅ Serialization: `serde` for JSON
- ✅ Path operations: Standard library

**No New Dependencies Required** ✅
