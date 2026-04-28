# EVIF Core Modules

## 1. Overview

The `evif-core` crate provides the foundational infrastructure for the EVIF virtual filesystem. It contains ~7,500 lines of Rust code across 26 modules.

## 2. Module Structure

```
crates/evif-core/src/
├── lib.rs                    # Public exports
├── plugin.rs                 # Plugin trait definition
├── error.rs                  # Error types
├── config.rs                 # Configuration
├── server.rs                 # Server implementation
├── radix_mount_table.rs       # Path routing
├── mount_table.rs            # Mount table trait
├── radix_benchmarks.rs       # Performance benchmarks
├── cache.rs                  # Caching infrastructure
├── handle_manager.rs          # File handle management
├── memory_handle.rs           # Memory-mapped handles
├── file_lock.rs              # File locking
├── file_monitor.rs           # File watching
├── batch_operations.rs        # Bulk operations
├── circuit_breaker.rs         # Resilience pattern
├── streaming.rs               # Streaming utilities
├── monitoring.rs              # Metrics infrastructure
├── dynamic_loader.rs          # Dynamic plugin loading
├── plugin_registry.rs         # Plugin lifecycle management
├── acl.rs                     # Access control
├── wasm.rs                    # WASM runtime
├── extism_plugin.rs           # Extism integration
├── config_validation.rs        # Configuration validation
└── cross_fs_copy.rs          # Cross-filesystem copy
```

## 3. Plugin Trait

The central abstraction is the `EvifPlugin` trait:

```rust
use async_trait::async_trait;
use bytes::Bytes;
use std::path::Path;

#[async_trait]
pub trait EvifPlugin: Send + Sync + 'static {
    // Lifecycle
    async fn init(&self, config: Option<&Value>) -> Result<(), EvifError>;
    async fn shutdown(&self) -> Result<(), EvifError>;

    // File operations
    async fn create(&self, path: &Path, options: CreateOptions) -> Result<FileHandle, EvifError>;
    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> Result<Bytes, EvifError>;
    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> Result<u64, EvifError>;
    async fn flush(&self, handle: &FileHandle) -> Result<(), EvifError>;

    // Directory operations
    async fn readdir(&self, path: &Path, offset: u64) -> Result<Vec<DirEntry>, EvifError>;

    // Metadata
    async fn stat(&self, path: &Path) -> Result<Metadata, EvifError>;

    // Mutation
    async fn remove(&self, path: &Path) -> Result<(), EvifError>;
    async fn rename(&self, old: &Path, new: &Path) -> Result<(), EvifError>;
    async fn remove_all(&self, path: &Path) -> Result<u32, EvifError>;

    // Optional capabilities
    fn capabilities(&self) -> PluginCapabilities;
}
```

## 4. Radix Mount Table

### 4.1 Purpose

Provides O(k) path routing where k is the number of path segments, not the total path length.

### 4.2 Implementation

```rust
pub struct RadixMountTable {
    root: Box<RadixNode>,
    mounts: DashMap<String, MountEntry>,
}

struct RadixNode {
    plugin: Option<Arc<dyn EvifPlugin>>,
    children: BTreeMap<u8, Box<RadixNode>>,  // First byte of segment
    wildcard: Option<Box<RadixNode>>,          // ** wildcard
}
```

### 4.3 Mount Operations

```rust
impl RadixMountTable {
    /// Mount a plugin at a path
    pub async fn mount(
        &self,
        path: &str,
        plugin: Arc<dyn EvifPlugin>,
        name: String,
    ) -> Result<(), EvifError>;

    /// Unmount a plugin
    pub async fn unmount(&self, path: &str) -> Result<(), EvifError>;

    /// Resolve a path to its plugin
    pub async fn resolve(&self, path: &Path) -> Result<ResolvedPath, EvifError>;

    /// List all mount points
    pub fn list(&self) -> Vec<MountInfo>;
}
```

### 4.4 Resolution Algorithm

```
Input: /context/L0/current
Step 1: Split into segments: ["context", "L0", "current"]
Step 2: Start at root, find "context" child → contextfs plugin
Step 3: Pass remaining path "/L0/current" to plugin
```

## 5. Handle Manager

### 5.1 Purpose

Manages file handles with lease-based resource management.

### 5.2 Handle Types

```rust
pub enum FileHandle {
    ReadOnly(u64),        // handle_id
    ReadWrite(u64),        // handle_id
    Append(u64),           // handle_id
    Directory(u64),        // handle_id
}

pub struct HandleState {
    pub id: u64,
    pub path: PathBuf,
    pub flags: OpenFlags,
    pub offset: u64,
    pub created_at: Instant,
    pub last_accessed: AtomicU64,
    pub lease_expires: Instant,
    pub references: AtomicU32,
}
```

### 5.3 Lease Management

```rust
impl GlobalHandleManager {
    /// Open a file and get a handle
    pub async fn open(
        &self,
        path: &Path,
        flags: OpenFlags,
        lease_seconds: u64,
    ) -> Result<FileHandle, EvifError>;

    /// Renew a handle's lease
    pub async fn renew(&self, handle: &FileHandle, seconds: u64) -> Result<(), EvifError>;

    /// Close a handle
    pub async fn close(&self, handle: &FileHandle) -> Result<(), EvifError>;

    /// Background cleanup of expired leases
    fn cleanup_loop(&self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                self.cleanup_expired().await;
            }
        });
    }
}
```

## 6. Cache Manager

### 6.1 Cache Types

```rust
pub enum EvifCache {
    /// LRU cache for file metadata
    Metadata(MetadataCache),

    /// Directory listing cache
    Directory(DirectoryCache),

    /// Content cache (optional)
    Content(Arc<dyn KvStore>),
}

pub struct MetadataCache {
    inner: RwLock<LruCache<String, Metadata>>,
    max_size: usize,
}

pub struct DirectoryCache {
    inner: RwLock<LruCache<String, Vec<DirEntry>>>,
    ttl: Duration,
}
```

### 6.2 Cache Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// Enable metadata caching
    pub metadata: Option<bool>,

    /// Enable directory listing caching
    pub directory: Option<bool>,

    /// Maximum cache size in entries
    pub max_entries: usize,

    /// Cache TTL in seconds
    pub ttl_seconds: u64,
}
```

## 7. Circuit Breaker

### 7.1 State Machine

```rust
pub enum CircuitState {
    /// Normal operation, requests pass through
    Closed,

    /// Too many failures, reject requests immediately
    Open,

    /// Testing if the service recovered
    HalfOpen,
}

pub struct CircuitBreaker {
    state: AtomicU8,
    failure_count: AtomicU32,
    last_failure: AtomicU64,
    success_count: AtomicU32,
    threshold: u32,
    timeout: Duration,
}
```

### 7.2 Usage

```rust
impl CircuitBreaker {
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T, CircuitError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, EvifError>>,
    {
        match self.state() {
            CircuitState::Closed => {
                match operation().await {
                    Ok(v) => {
                        self.on_success();
                        Ok(v)
                    }
                    Err(e) => {
                        self.on_failure();
                        Err(CircuitError::Upstream(e))
                    }
                }
            }
            CircuitState::Open => Err(CircuitError::CircuitOpen),
            CircuitState::HalfOpen => {
                // Allow limited requests through
            }
        }
    }
}
```

## 8. Error Handling

### 8.1 Error Types

```rust
#[derive(Debug, Error)]
pub enum EvifError {
    #[error("plugin not found: {0}")]
    PluginNotFound(String),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("is directory: {0}")]
    IsDirectory(String),

    #[error("not a directory: {0}")]
    NotDirectory(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("plugin error: {0}")]
    Plugin(String),

    #[error("circuit breaker open")]
    CircuitBreakerOpen,

    #[error("timeout")]
    Timeout,

    #[error("cancelled")]
    Cancelled,
}
```

### 8.2 Result Type

```rust
pub type EvifResult<T> = Result<T, EvifError>;
```

## 9. Batch Operations

### 9.1 Parallel Copy

```rust
pub struct BatchExecutor {
    max_concurrent: usize,
    error_strategy: ErrorStrategy,
}

pub enum ErrorStrategy {
    /// Stop on first error
    FailFast,

    /// Continue with remaining items, collect errors
    Continue,

    /// Retry failed items up to N times
    Retry { max_attempts: u32 },
}

impl BatchExecutor {
    pub async fn copy(
        &self,
        operations: Vec<CopyOperation>,
        progress: Option<Box<dyn ProgressCallback>>,
    ) -> BatchResult {
        // Uses semaphore for concurrency control
        // Reports progress via callback
        // Collects results and errors
    }
}
```

### 9.2 Copy Operation

```rust
pub struct CopyOperation {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub overwrite: bool,
    pub recursive: bool,
}
```

## 10. Streaming

### 10.1 Line Reader

```rust
pub struct LineReader<S> {
    stream: S,
    buffer: BytesMut,
    delimiter: u8,
}

impl<S: AsyncRead + Unpin> Stream for LineReader<S> {
    type Item = Result<String, std::io::Error>;
}
```

### 10.2 Stream Reader

```rust
pub trait StreamReader: AsyncRead + AsyncSeek + Send + Sync {
    fn content_length(&self) -> Option<u64>;
    fn content_type(&self) -> Option<&str>;
}

pub struct MemoryStreamReader {
    data: Bytes,
    position: u64,
}
```

## 11. Dynamic Loader

### 11.1 Plugin Loading

```rust
pub struct DynamicLoader {
    search_paths: Vec<PathBuf>,
    loaded: DashMap<String, Arc<dyn EvifPlugin>>,
}

impl DynamicLoader {
    /// Load a plugin from a .so/.dylib file
    pub async fn load(&self, path: &Path) -> Result<Arc<dyn EvifPlugin>, LoadError>;

    /// Unload a plugin
    pub async fn unload(&self, name: &str) -> Result<(), UnloadError>;

    /// Reload a plugin (unload + load)
    pub async fn reload(&self, name: &str) -> Result<Arc<dyn EvifPlugin>, LoadError>;
}
```

## 12. WASM Support

### 12.1 Extism Integration

```rust
pub struct ExtismPlugin {
    manifest: Manifest,
    plugin: Plugin,
}

#[async_trait]
impl EvifPlugin for ExtismPlugin {
    // Maps filesystem operations to Extism host functions
}
```

### 12.2 Wasmtime Integration

```rust
pub struct WasmtimePlugin {
    engine: Engine,
    module: Module,
    linker: Linker,
    store: Store<PluginState>,
}
```

## 13. Configuration

### 13.1 Server Config

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub workers: usize,
    pub max_connections: usize,
    pub request_timeout: Duration,
    pub shutdown_timeout: Duration,
}
```

### 13.2 Mount Config

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MountConfig {
    pub path: String,
    pub plugin: String,
    pub config: Option<Value>,
    pub read_only: bool,
}
```

## 14. Related Documents

- [Plugin Development Guide](plugin-development.md)
- [REST API Reference](API.md)
- [Production Deployment](production-deployment.md)
