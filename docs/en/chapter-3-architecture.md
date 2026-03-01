# Chapter 3: Architecture

## Table of Contents

- [System Architecture Overview](#system-architecture-overview)
- [Core Components](#core-components)
- [Plugin System Design](#plugin-system-design)
- [Mount Table & Path Resolution](#mount-table--path-resolution)
- [Data Flow & Request Processing](#data-flow--request-processing)
- [Technology Stack](#technology-stack)

---

## System Architecture Overview

### Dual Technology Stack Architecture

EVIF employs a unique dual technology stack architecture, with two technical paths running in parallel, each serving different use cases:

#### 1. Plugin + REST Stack (Current Main Path)

This is the **primary technical path currently in operation** for EVIF, providing complete file system operation capabilities:

- **Core Components**: evif-core, evif-plugins, evif-rest
- **Access Methods**: HTTP REST API + WebSocket
- **Primary Use**: Web UI (evif-web), CLI clients, third-party integrations
- **Implementation Status**: ✅ **Fully implemented and running**

**Key Features**:
- Plugin-based file operations (create, read, write, delete, rename, etc.)
- RESTful API design supporting standard HTTP methods
- WebSocket terminal for interactive command-line experience
- Multiple storage backend support (memory, local, cloud storage, etc.)

#### 2. Graph + VFS Stack (Planned/Partially Implemented)

This is an architecture designed for future expansion, with some components implemented but not fully integrated:

- **Core Components**: evif-graph, evif-vfs, evif-storage, evif-auth, evif-runtime, evif-fuse, evif-grpc
- **Design Goals**: Graph query engine, virtual filesystem abstraction, distributed storage, authentication and authorization
- **Implementation Status**: ⚠️ **Partially implemented, not fully integrated**

**Current Status**:
- Graph API routes exist but return "not implemented"
- VFS abstraction layer defined but not used by REST main path
- FUSE support implemented (evif-fuse), can run independently
- gRPC service disabled

### Architecture Comparison Table

| Feature | Plugin + REST Stack | Graph + VFS Stack |
|---------|---------------------|-------------------|
| **Current Status** | ✅ Production Ready | ⚠️ Planned |
| **Main Entry** | evif-rest (HTTP 8081) | evif-grpc (disabled) |
| **File Operations** | EvifPlugin trait | VFS abstraction layer |
| **Query Capability** | Path matching | Graph queries (not implemented) |
| **Storage** | Plugin self-managed | Unified storage layer |
| **Authentication** | None | evif-auth (not integrated) |
| **Configuration** | Hardcoded mounts | evif-runtime (not used) |

### Key Design Decisions

**Why a Dual Technology Stack?**

1. **Progressive Evolution**: Plugin system provides immediate usability, graph engine supports complex queries
2. **Flexibility**: Different scenarios can use different technology stacks
3. **Backward Compatibility**: Maintain simple path while supporting advanced features

**Current Recommendation**:
- **Web Application Development**: Use Plugin + REST stack
- **Command Line Tools**: Use CLI + REST API
- **FUSE Mounting**: Use evif-fuse (based on plugin stack)
- **Graph Queries**: Wait for VFS stack implementation completion

---

## Core Components

EVIF consists of 19 independent crates, each responsible for specific functionality. Detailed analysis follows:

### 1. evif-core

**Responsibility**: Core abstractions and infrastructure

- **plugin.rs**: Defines `EvifPlugin` trait containing all filesystem operation interfaces
- **radix_mount_table.rs**: `RadixMountTable` implementation for path mounting and lookup
- **handle_manager.rs**: Global file handle management
- **batch_operations.rs**: Batch file operations (copy, delete)
- **cache/**: Metadata and directory cache abstractions

**Key Types**:
```rust
pub trait EvifPlugin: Send + Sync {
    fn create(&self, path: &str, typ: FileType) -> Result<()>;
    fn read(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>>;
    fn write(&self, path: &str, offset: u64, data: Vec<u8>) -> Result<u64>;
    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>>;
    fn stat(&self, path: &str) -> Result<FileInfo>;
    fn remove(&self, path: &str) -> Result<()>;
    fn rename(&self, from: &str, to: &str) -> Result<()>;
    // ... more methods
}

pub struct RadixMountTable {
    // Radix tree-based mount point management
}
```

**Optional Features**:
- `extism_plugin`: WASM plugin support (requires feature flag)
- Configuration validation, monitoring, audit logging infrastructure

### 2. evif-plugins

**Responsibility**: Implement EvifPlugin trait for various storage backends

**Default Enabled Plugins**:
- **MemFsPlugin**: Memory filesystem (/mem)
- **HelloFsPlugin**: Demo plugin (/hello)
- **LocalFsPlugin**: Local filesystem access (/local)

**Other Implemented Plugins**:
- **Cloud Storage**: s3fs, azureblobfs, gcsfs, aliyunossfs, tencentcosfs, huaweiobsfs, miniofs
- **Object Storage**: opendal (unified adapter layer)
- **Database**: sqlfs, kvfs
- **Special Purpose**: streamfs, queuefs, httpfs, proxyfs, devfs, heartbeatfs, handlefs
- **Advanced Features**: vectorfs (vector storage), gptfs (AI integration), encryptedfs (encryption)
- **Network Protocols**: webdavfs, ftpfs, sftpfs
- **Storage Tiering**: tieredfs (multi-level cache)

**Plugin Features**:
- All plugins implement the same `EvifPlugin` trait
- Support optional interfaces: `HandleFS` (file handles), `Streamer` (streaming)
- Configuration validation via `Configurable` trait
- Some plugins require feature flag compilation

### 3. evif-rest

**Responsibility**: HTTP REST API server

**Entry Point**: `crates/evif-rest/src/main.rs`

**Startup Flow**:
```rust
// 1. Create mount table
let mount_table = RadixMountTable::new();

// 2. Mount default plugins (hardcoded)
mount_table.mount("/mem", Box::new(MemFsPlugin::new()));
mount_table.mount("/hello", Box::new(HelloFsPlugin::new()));
mount_table.mount("/local", Box::new(LocalFsPlugin::new("/tmp/evif")));

// 3. Start HTTP server (default port 8081)
EvifServer::run(mount_table).await?;
```

**API Route Layering**:

#### 3.1 Web Compatible API (`/api/v1/fs/*`)
- `GET /api/v1/fs/list?path=` - List directory
- `GET /api/v1/fs/read?path=` - Read file
- `POST /api/v1/fs/write?path=` - Write file
- `POST /api/v1/fs/create` - Create file/directory
- `DELETE /api/v1/fs/delete?path=` - Delete

**Purpose**: Used by evif-web frontend

#### 3.2 CLI Standard API (`/api/v1/files`, `/api/v1/directories`)
- `GET /api/v1/directories` - Directory list
- `GET /api/v1/files` - Read file
- `PUT /api/v1/files` - Write file
- `POST /api/v1/stat` - File metadata
- `POST /api/v1/rename` - Rename
- `POST /api/v1/grep` - Content search
- `POST /api/v1/digest` - File digest
- `POST /api/v1/touch` - Create empty file

**Purpose**: Used by evif-client (CLI)

#### 3.3 Mount Management API
- `GET /api/v1/mounts` - List mount points ✅ Implemented
- `POST /api/v1/mount` - Dynamic mount ❌ Not implemented
- `DELETE /api/v1/unmount` - Unmount ❌ Not implemented

#### 3.4 Plugin Management API
- `GET /api/v1/plugins` - List plugins ✅
- `GET /api/v1/plugins/list` - Plugin details ✅
- `POST /api/v1/plugins/load` - Load plugin ❌
- `POST /api/v1/plugins/wasm/load` - Load WASM plugin ⚠️ Partial
- `DELETE /api/v1/plugins/unload` - Unload plugin ❌

#### 3.5 Handle API (`/api/v1/handles/*`)
- `POST /api/v1/handles/open` - Open file handle
- `GET /api/v1/handles/read` - Read from handle
- `PUT /api/v1/handles/write` - Write to handle
- `POST /api/v1/handles/seek` - Seek handle
- `POST /api/v1/handles/sync` - Sync handle
- `DELETE /api/v1/handles/close` - Close handle

**Requirement**: Plugin must implement `HandleFS` trait

#### 3.6 Metrics API (`/api/v1/metrics/*`)
- `GET /api/v1/metrics/traffic` - Traffic statistics
- `GET /api/v1/metrics/operations` - Operation counts
- `GET /api/v1/metrics/status` - System status
- `POST /api/v1/metrics/reset` - Reset metrics

#### 3.7 Graph API (Placeholder)
- `GET /nodes/*` - Graph node queries ❌
- `POST /query` - Graph query ❌
- `GET /stats` - Graph statistics ❌

**Returns**: "Graph functionality not implemented"

#### 3.8 WebSocket Terminal (`/ws`)
**Supported Commands**:
- `ls [path]` - List directory
- `cat [path]` - Display file content
- `stat [path]` - File status
- `mounts` - List mount points
- `pwd` - Current path
- `echo [text]` - Echo text
- `clear` - Clear screen
- `help` - Help information

**Purpose**: evif-web terminal component

### 4. evif-web

**Responsibility**: React + TypeScript based Web UI

**Tech Stack**:
- React 18 + TypeScript
- Vite (dev server)
- Monaco Editor (code editor)
- TailwindCSS (styling)
- WebSocket API (terminal communication)

**Implemented Features**:
- File tree browsing and expansion
- Monaco editor integration
- File CRUD operations
- WebSocket terminal
- Context menu (right-click)
- Menu bar (new, refresh, save)

**Unintegrated Features**:
- Plugin manager UI (PluginManager, MountModal, PluginModal)
- Monitoring panel (SystemStatus, TrafficChart, LogViewer)
- Collaboration features (comments, permissions, sharing)
- Advanced editor features (MiniMap, QuickOpen)
- Search and upload

**API Integration**:
- Primarily uses `/api/v1/fs/*` compatible API
- WebSocket connection to `/ws`
- Vite proxy config: `/api` → `http://localhost:8081`

### 5. evif-client

**Responsibility**: HTTP client for CLI

**Purpose**: Network layer for evif-cli, calls REST API via HTTP

**Method Mapping**:
```rust
// CLI commands → HTTP API
ls(path)         → GET /api/v1/directories?path=
cat(path)        → GET /api/v1/files?path=
write(path, data) → PUT /api/v1/files?path=
mkdir(path)      → POST /api/v1/directories
remove(path)     → DELETE /api/v1/files?path=
stat(path)       → POST /api/v1/stat
mounts()         → GET /api/v1/mounts
```

**Known Issues**:
- **Format Inconsistency**: client expects base64-encoded responses, handlers return plain JSON
- **Impact**: CLI `cat` command may fail to parse file content correctly

### 6. evif-cli

**Responsibility**: Command-line interface tool

**Entry**: `crates/evif-cli/src/main.rs`

**Command Parsing**: Uses `clap` framework

**Main Commands**:
- `evif ls [path]` - List directory
- `evif cat [path]` - Display file content
- `evif cp <src> <dst>` - Copy file
- `evif stats` - Display statistics
- `evif repl` - Enter interactive mode
- `evif mount <path>` - FUSE mount
- `evif get <url> <path>` - Download file to EVIF

**Default Configuration**:
- `--server localhost:50051` (name looks like gRPC, actually uses HTTP)
- Actually calls REST API via evif-client (default port 8081)

### 7. evif-fuse

**Responsibility**: FUSE filesystem implementation

**Technology**: Based on `fuser` crate (Rust FUSE binding)

**Entry**: `evif-fuse-mount` binary

**How It Works**:
```rust
// FUSE mount process
let mount_table = create_mount_table(); // Same mount table as evif-rest
let fuse = EvifFuse::new(mount_table);
fuse.mount("/mnt/evif")?; // Mount to local directory
```

**Features**:
- Reuses evif-core's `RadixMountTable` and `EvifPlugin` abstractions
- Shares plugin model with evif-rest but runs as separate process
- Supports POSIX filesystem semantics
- Caching strategy: metadata cache, directory cache

**Use Cases**:
- Mount EVIF as local filesystem
- Integrate with standard Unix tools (ls, cp, mv, etc.)
- Transparent file access

**Limitations**:
- Requires separate process
- Does not share mount table state with evif-rest
- FUSE permission requirements (Linux/macOS)

### 8. evif-graph

**Responsibility**: Graph data structures and algorithms

**Core Types**:
```rust
pub struct Node {
    pub id: NodeId,
    pub labels: Vec<String>,
    pub properties: HashMap<String, Value>,
}

pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
}

pub struct Graph {
    pub nodes: HashMap<NodeId, Node>,
    pub edges: HashMap<EdgeId, Edge>,
}

pub struct Query {
    // Graph query DSL
}

pub struct Executor {
    // Query execution engine
}
```

**Current Status**:
- ✅ Graph data structures implemented
- ✅ Query language parser implemented
- ❌ REST API integration incomplete
- ❌ Actual use cases unclear

**Referenced By**:
- evif-storage (graph persistence)
- evif-vfs (graph queries)
- evif-protocol (graph protocol)
- evif-client (graph client types)
- evif-cli (graph commands)
- evif-rest, evif-fuse (dependency but not used)

**Recommendation**:
- If graph query functionality is not needed, related dependencies can be removed
- If needed, complete REST API integration and provide usage examples

### 9. evif-vfs

**Responsibility**: POSIX-style virtual filesystem abstraction

**Core Concepts**:
```rust
pub trait Vfs {
    fn mount(&mut self, path: &str, backend: Box<dyn VfsBackend>) -> Result<()>;
    fn lookup(&self, path: &str) -> Result<INode>;
    fn create(&mut self, path: &str, typ: FileType) -> Result<INode>;
    fn unlink(&mut self, path: &str) -> Result<()>;
}

pub trait VfsBackend {
    fn read(&self, inode: INode, offset: u64, size: u64) -> Result<Vec<u8>>;
    fn write(&mut self, inode: INode, offset: u64, data: Vec<u8>) -> Result<u64>;
}

pub struct INode(pub u64);

pub struct DEntry {
    pub name: String,
    pub inode: INode,
}
```

**Design Goals**:
- Unified VFS abstraction layer
- Support multiple filesystem backends
- POSIX compatibility

**Current Status**:
- ✅ VFS abstraction defined
- ❌ evif-rest main path does not use it
- ❌ evif-fuse uses EvifPlugin directly, not through VFS

**Relationship with EvifPlugin**:
- `EvifPlugin`: Path string operations, simpler
- `Vfs`: INode-based POSIX abstraction, lower-level

### 10. evif-storage

**Responsibility**: Pluggable storage backends

**Storage Types**:
```rust
pub enum StorageBackend {
    Memory(MemoryStorage),
    Sled(SledStorage),
    RocksDB(RocksDBStorage),
    S3(S3Storage),
    Custom(Box<dyn CustomStorage>),
}
```

**Purpose**:
- Graph data persistence (evif-graph)
- Metadata storage (optional for plugins)
- Runtime state (evif-runtime)

**Current Status**:
- ✅ Multiple storage backends implemented
- ⚠️ REST main path does not use directly
- ⚠️ Some plugins may depend on it

### 11. evif-auth

**Responsibility**: Authentication and authorization

**Features**:
- JWT token validation
- Role-based access control (RBAC)
- Audit logging
- Capability checking

**Current Status**:
- ✅ Authentication framework implemented
- ❌ evif-rest not integrated
- ❌ API has no authentication middleware

### 12. evif-runtime

**Responsibility**: Runtime configuration and orchestration

**Features**:
- Configuration file loading (TOML/YAML/JSON)
- Dynamic plugin loading
- Service lifecycle management
- Health checks

**Current Status**:
- ✅ Runtime framework implemented
- ❌ evif-rest uses hardcoded configuration
- ❌ Not integrated with configuration files

### 13. evif-protocol

**Responsibility**: Message protocol definitions

**Content**:
- Request/response message types
- Encoding/decoding (JSON/MessagePack)
- Graph query protocol

**Used By**:
- evif-client (graph operation client)
- evif-grpc (gRPC service)

**Current Status**:
- ✅ Protocol definitions complete
- ⚠️ Primarily for graph-related features
- ❌ REST API does not use

### 14. evif-grpc

**Responsibility**: gRPC service implementation

**Current Status**:
- ❌ **Server disabled**
- ✅ Protocol Buffers definitions exist
- ✅ gRPC client code generation

**Disable Reason**:
- Possibly in early development
- Or prioritizing REST API

**Future Use**:
- High-performance RPC communication
- Streaming data transfer
- Multi-language client support

### 15. evif-mcp

**Responsibility**: Model Context Protocol (MCP) server

**Features**:
- Call EVIF API via HTTP
- Provide filesystem tools for AI assistants
- Support tool list, tool invocation

**Configuration**:
- `EVIF_URL` environment variable (default `http://localhost:8080`)
- Should point to evif-rest service (8081)

**Tool List** (17 tools):
- `evif_read_file` - Read file
- `evif_write_file` - Write file
- `evif_list_directory` - List directory
- `evif_create_directory` - Create directory
- `evif_delete` - Delete file/directory
- `evif_rename` - Rename
- `evif_stat` - File status
- `evif_copy` - Copy
- `evif_search` - Search file
- `evif_mount_list` - List mount points
- `evif_mount_create` - Create mount
- `evif_mount_delete` - Delete mount
- `evif_health_check` - Health check
- `evif_grep` - Content search
- `evif_digest` - File digest
- `evif_batch_delete` - Batch delete
- `evif_touch` - Create empty file

### 16. evif-macros

**Responsibility**: Procedural macro support

**Macro Definitions**:
- `#[evif_plugin]` - Auto-implement EvifPlugin trait
- `#[configurable]` - Auto-implement Configurable trait
- `#[metric]` - Metric collection

**Purpose**:
- Simplify plugin development
- Reduce repetitive code
- Compile-time type checking

### 17. evif-metrics

**Responsibility**: Metrics type definitions

**Metric Types**:
```rust
pub struct TrafficMetrics {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub requests: u64,
}

pub struct OperationMetrics {
    pub creates: u64,
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
}

pub struct StatusMetrics {
    pub active_mounts: usize,
    pub loaded_plugins: usize,
    pub uptime_seconds: u64,
}
```

**Purpose**:
- `/api/v1/metrics/*` API responses
- Monitoring panel data source
- Performance analysis

### 18. evif-python

**Responsibility**: Python bindings

**Features**:
- Python client library
- Exception type definitions
- Data model bindings

**Uses PyO3**:
```python
import evif

client = evif.Client("http://localhost:8081")
files = client.list_directory("/mem")
content = client.read_file("/mem/test.txt")
```

**Current Status**:
- ✅ Python bindings implemented
- ⚠️ REST API compatibility needs verification

### 19. Auxiliary Crates

**example-dynamic-plugin**: Dynamic plugin example
**tests/**: Integration tests
- e2e: End-to-end tests
- cli: CLI tests
- api: API tests
- common: Test utilities

---

## Plugin System Design

### EvifPlugin Trait

**Core Interface**:

```rust
#[async_trait]
pub trait EvifPlugin: Send + Sync + AsAny {
    // Basic information
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    // File operations
    fn create(&self, path: &str, typ: FileType) -> Result<()>;
    fn read(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>>;
    fn write(&self, path: &str, offset: u64, data: Vec<u8>) -> Result<u64>;
    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>>;
    fn stat(&self, path: &str) -> Result<FileInfo>;
    fn remove(&self, path: &str) -> Result<()>;
    fn rename(&self, from: &str, to: &str) -> Result<()>;
    fn remove_all(&self, path: &str) -> Result<()>;

    // Symbolic links
    fn symlink(&self, src: &str, dst: &str) -> Result<()>;
    fn readlink(&self, path: &str) -> Result<String>;

    // Optional interfaces
    fn as_handle_fs(&self) -> Option<&dyn HandleFS> {
        None
    }

    fn as_streamer(&self) -> Option<&dyn Streamer> {
        None
    }
}
```

### Plugin Type Classification

#### 1. Memory-Based Plugins
- **MemFs**: Pure memory storage
- **HelloFs**: Demo plugin
- **DevFs**: Development information
- **ServerInfoFs**: Server status

#### 2. Local Storage Plugins
- **LocalFs**: Local filesystem
- **TieredFs**: Multi-level cache

#### 3. Cloud Storage Plugins
- **S3Fs**: AWS S3
- **AzureBlobFs**: Azure Blob Storage
- **GcsFs**: Google Cloud Storage
- **AliyunOssFs**: Alibaba Cloud OSS
- **TencentCosFs**: Tencent Cloud COS
- **HuaweiObsFs**: Huawei Cloud OBS
- **MinioFs**: MinIO

#### 4. Database Plugins
- **SqlFs**: SQL database
- **Kvfs**: Key-value storage

#### 5. Special Purpose Plugins
- **StreamFs**: Stream processing
- **QueueFs**: Message queue
- **HttpFs**: HTTP proxy
- **ProxyFs**: Reverse proxy
- **VectorFs**: Vector storage
- **GptFs**: AI integration
- **EncryptedFs**: Encrypted storage

#### 6. Network Protocol Plugins
- **WebdavFs**: WebDAV
- **FtpFs**: FTP
- **SftpFs**: SFTP

### Optional Interfaces

#### HandleFS Trait

**Purpose**: Support file handle operations (similar to POSIX file descriptor)

```rust
#[async_trait]
pub trait HandleFS: Send + Sync {
    fn open(&self, path: &str, flags: OpenFlags) -> Result<FileHandle>;
    fn read(&self, handle: FileHandle, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, handle: FileHandle, buf: &[u8]) -> Result<usize>;
    fn seek(&self, handle: FileHandle, pos: SeekFrom) -> Result<u64>;
    fn sync(&self, handle: FileHandle) -> Result<()>;
    fn close(&self, handle: FileHandle) -> Result<()>;
}
```

**Benefits**:
- Support random access
- Reduce path resolution overhead
- Closer to POSIX semantics

**Use Cases**:
- Large file read/write
- Frequent small read/write operations
- Scenarios requiring file locks

#### Streamer Trait

**Purpose**: Streaming data transfer

```rust
#[async_trait]
pub trait Streamer: Send + Sync {
    fn read_stream(&self, path: &str) -> Result<BoxStream<Vec<u8>>>;
    fn write_stream(&self, path: &str) -> Result<BoxSink<Vec<u8>>>;
}
```

**Benefits**:
- Support large file streaming
- Reduce memory usage
- Real-time data processing

**Use Cases**:
- Video streaming
- Log streaming
- Real-time data collection

### Plugin Configuration and Validation

```rust
pub trait Configurable: EvifPlugin {
    fn validate_config(&self, config: &Value) -> Result<()>;
    fn apply_config(&mut self, config: &Value) -> Result<()>;
    fn default_config(&self) -> Value;
}
```

**Configuration Example** (S3Fs):
```json
{
  "access_key": "AKIAIOSFODNN7EXAMPLE",
  "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
  "bucket": "my-bucket",
  "region": "us-east-1",
  "endpoint": "https://s3.amazonaws.com"
}
```

### Plugin Development Example

**Simple Plugin Implementation**:

```rust
use evif_core::prelude::*;
use async_trait::async_trait;

pub struct MyPlugin {
    config: HashMap<String, String>,
}

impl MyPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn create(&self, path: &str, typ: FileType) -> Result<()> {
        // Implement create logic
        Ok(())
    }

    fn read(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>> {
        // Implement read logic
        Ok(Vec::new())
    }

    fn write(&self, path: &str, offset: u64, data: Vec<u8>) -> Result<u64> {
        // Implement write logic
        Ok(data.len() as u64)
    }

    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>> {
        // Implement directory list
        Ok(Vec::new())
    }

    fn stat(&self, path: &str) -> Result<FileInfo> {
        // Implement status query
        Ok(FileInfo {
            path: path.to_string(),
            typ: FileType::File,
            size: 0,
            modified: SystemTime::now(),
        })
    }

    fn remove(&self, path: &str) -> Result<()> {
        // Implement delete logic
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> Result<()> {
        // Implement rename logic
        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<()> {
        // Implement recursive delete
        Ok(())
    }

    fn symlink(&self, src: &str, dst: &str) -> Result<()> {
        Err(Error::not_supported("Symbolic links"))
    }

    fn readlink(&self, path: &str) -> Result<String> {
        Err(Error::not_supported("Symbolic links"))
    }
}
```

---

## Mount Table & Path Resolution

### RadixMountTable Design

**Data Structure**:

```rust
pub struct RadixMountTable {
    mounts: RadixTree<Box<dyn EvifPlugin>>,
    default: Option<Box<dyn EvifPlugin>>,
}
```

**Core Concepts**:
- **Radix Tree**: Efficient path matching
- **Longest Prefix Match**: Select most specific mount point
- **O(k) Complexity**: k is path depth

### Path Normalization

**Normalization Rules**:

1. **Remove redundant separators**: `/foo//bar` → `/foo/bar`
2. **Resolve `.`**: `/foo/./bar` → `/foo/bar`
3. **Resolve `..`**: `/foo/../bar` → `/bar`
4. **Trailing slash**: `/foo/` → `/foo` (unless root `/`)
5. **Reject invalid paths**: Empty paths, relative paths

**Example**:

```rust
fn normalize_path(path: &str) -> Result<String> {
    if path.is_empty() {
        return Err(Error::invalid_path("Empty path"));
    }

    let components = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .fold(vec![], |mut acc, comp| {
            if comp == ".." {
                acc.pop();
            } else {
                acc.push(comp);
            }
            acc
        });

    if components.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", components.join("/")))
    }
}
```

### Mount Point Matching Algorithm

**Longest Prefix Match**:

```rust
impl RadixMountTable {
    pub fn lookup(&self, path: &str) -> Result<&dyn EvifPlugin> {
        let normalized = normalize_path(path)?;

        // Find longest match in radix tree
        let (mount_point, plugin) = self.mounts
            .longest_prefix(&normalized)
            .unwrap_or((&*"/", self.default.as_ref().unwrap()));

        // Verify mount point exists
        if plugin.is_none() {
            return Err(Error::not_found(path));
        }

        Ok(plugin)
    }
}
```

**Matching Examples**:

| Request Path | Mount Point | Matched Plugin | Relative Path |
|--------------|-------------|----------------|---------------|
| `/mem/foo.txt` | `/mem` | MemFsPlugin | `/foo.txt` |
| `/local/subdir/file.txt` | `/local` | LocalFsPlugin | `/subdir/file.txt` |
| `/hello` | `/hello` | HelloFsPlugin | `/` |
| `/unmounted/path` | - | - | Error |

### Mount Management

**Mount Operations**:

```rust
let mount_table = RadixMountTable::new();

// Mount plugins
mount_table.mount("/mem", Box::new(MemFsPlugin::new()))?;
mount_table.mount("/local", Box::new(LocalFsPlugin::new("/data")))?;
mount_table.mount("/s3", Box::new(S3FsPlugin::new(config))?)?;

// List mount points
let mounts = mount_table.list_mounts();
// [("/mem", "MemFsPlugin"), ("/local", "LocalFsPlugin"), ("/s3", "S3FsPlugin")]

// Unmount plugin
mount_table.unmount("/mem")?;
```

**Limitations**:
- **Current Version**: Mount points hardcoded in evif-rest/src/server.rs
- **Dynamic Mounting**: API exists but returns "not yet supported"
- **Future Plan**: Mount management via config file or REST API

---

## Data Flow & Request Processing

### Startup Flow

```mermaid
graph TD
    A[evif-rest::main] --> B[Create RadixMountTable]
    B --> C[Mount MemFs to /mem]
    B --> D[Mount HelloFs to /hello]
    B --> E[Mount LocalFs to /local]
    C --> F[Start HTTP server 8081]
    D --> F
    E --> F
    F --> G[Listen for requests]
```

### Web Request Flow

```mermaid
sequenceDiagram
    participant Browser
    participant Vite
    participant RestAPI
    participant MountTable
    participant Plugin

    Browser->>Vite: GET /api/v1/fs/list?path=/mem
    Vite->>RestAPI: Proxy to 8081
    RestAPI->>RestAPI: compat_fs::list()
    RestAPI->>MountTable: lookup("/mem")
    MountTable->>Plugin: Return MemFsPlugin
    RestAPI->>Plugin: readdir("/mem")
    Plugin->>RestAPI: Vec<FileInfo>
    RestAPI->>Vite: FsListResponse { nodes }
    Vite->>Browser: JSON response
```

### CLI Request Flow

```mermaid
sequenceDiagram
    participant CLI
    participant Client
    participant RestAPI
    participant MountTable
    participant Plugin

    CLI->>Client: evif ls /mem
    Client->>RestAPI: GET /api/v1/directories?path=/mem
    RestAPI->>RestAPI: handlers::list_directory()
    RestAPI->>MountTable: lookup("/mem")
    MountTable->>Plugin: Return MemFsPlugin
    RestAPI->>Plugin: readdir("/mem")
    Plugin->>RestAPI: Vec<FileInfo>
    RestAPI->>Client: DirectoryListResponse { files }
    Client->>CLI: Formatted output
```

### WebSocket Terminal Flow

```mermaid
sequenceDiagram
    participant Terminal
    participant WebSocket
    participant WsHandlers
    participant MountTable
    participant Plugin

    Terminal->>WebSocket: ws://localhost:8081/ws
    WebSocket-->>Terminal: Connected

    Terminal->>WebSocket: "ls /mem"
    WebSocket->>WsHandlers: Parse command
    WsHandlers->>MountTable: lookup("/mem")
    MountTable->>Plugin: Return MemFsPlugin
    WsHandlers->>Plugin: readdir("/mem")
    Plugin->>WsHandlers: Vec<FileInfo>
    WsHandlers->>WebSocket: Formatted output
    WebSocket->>Terminal: "file1.txt\nfile2.txt\n"
```

### FUSE Request Flow

```mermaid
sequenceDiagram
    participant App
    participant FUSE
    participant FuseFS
    participant MountTable
    participant Plugin

    App->>FUSE: read("/mnt/evif/mem/file.txt")
    FUSE->>FuseFS: getattr()
    FuseFS->>MountTable: lookup("/mem")
    MountTable->>Plugin: Return MemFsPlugin
    FuseFS->>Plugin: stat("/mem/file.txt")
    Plugin->>FuseFS: FileInfo
    FuseFS->>FUSE: Attributes
    FUSE->>FuseFS: read()
    FuseFS->>Plugin: read("/mem/file.txt", 0, 4096)
    Plugin->>FuseFS: Vec<u8>
    FuseFS->>FUSE: Data
    FUSE->>App: File content
```

### API Routing Hierarchy

```
evif-rest (8081)
├─ /health
│  └─ health checker
│
├─ /api/v1/fs/* (Web Compatible API)
│  ├─ /api/v1/fs/list
│  ├─ /api/v1/fs/read
│  ├─ /api/v1/fs/write
│  ├─ /api/v1/fs/create
│  └─ /api/v1/fs/delete
│     └─ compat_fs handlers
│
├─ /api/v1/files, /api/v1/directories (CLI API)
│  ├─ GET /api/v1/directories
│  ├─ GET /api/v1/files
│  ├─ PUT /api/v1/files
│  ├─ DELETE /api/v1/files
│  ├─ POST /api/v1/stat
│  ├─ POST /api/v1/rename
│  ├─ POST /api/v1/grep
│  ├─ POST /api/v1/digest
│  └─ POST /api/v1/touch
│     └─ handlers
│
├─ /api/v1/mounts, /api/v1/mount, /api/v1/unmount
│  └─ mount handlers
│
├─ /api/v1/plugins/*
│  ├─ GET /api/v1/plugins
│  ├─ GET /api/v1/plugins/list
│  ├─ POST /api/v1/plugins/load
│  ├─ POST /api/v1/plugins/wasm/load
│  └─ DELETE /api/v1/plugins/unload
│     └─ plugin handlers
│
├─ /api/v1/handles/* (Handle API)
│  ├─ POST /api/v1/handles/open
│  ├─ GET /api/v1/handles/read
│  ├─ PUT /api/v1/handles/write
│  ├─ POST /api/v1/handles/seek
│  ├─ POST /api/v1/handles/sync
│  └─ DELETE /api/v1/handles/close
│     └─ handle handlers
│
├─ /api/v1/metrics/*
│  ├─ GET /api/v1/metrics/traffic
│  ├─ GET /api/v1/metrics/operations
│  ├─ GET /api/v1/metrics/status
│  └─ POST /api/v1/metrics/reset
│     └─ metrics handlers
│
├─ /nodes/*, /query, /stats (Graph API - Placeholder)
│  └─ "Graph functionality not implemented"
│
└─ /ws (WebSocket Terminal)
   └─ ws_handlers
```

---

## Technology Stack

### Core Dependencies

**Async Runtime**:
- **tokio** 1.35 (features = ["full"]) - Async runtime
- **async-trait** 0.1.77 - Async trait support

**Serialization**:
- **serde** 1.0.196 - Serialization framework
- **serde_json** 1.0.113 - JSON support
- **bytes** 1.5.0 - Byte buffers

**Error Handling**:
- **anyhow** 1.0.79 - Error context
- **thiserror** 1.0.56 - Error type definitions

**Graph Algorithms**:
- **petgraph** 0.6.4 - Graph data structures and algorithms

**Concurrent Data Structures**:
- **dashmap** 5.5.3 - Concurrent hashmap
- **ahash** 0.8.7 - High-performance hashing

**Utilities**:
- **uuid** 1.6.1 - UUID generation
- **chrono** 0.4.31 - Time handling
- **config** 0.14.0 - Configuration management

**Cryptography**:
- **blake3** 1.5.0 - Hash algorithm
- **ed25519-dalek** 2.1.1 - Signatures

**Testing**:
- **proptest** 1.4.0 - Property testing
- **criterion** 0.5.1 - Performance testing
- **tokio-test** 0.4.3 - Async testing
- **tempfile** 3.8 - Temporary files

### Web Technologies

**evif-rest**:
- **axum** (or actix-web) - Web framework
- **tokio-tungstenite** - WebSocket support
- **tower** - Middleware

**evif-web**:
- **React** 18 - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool
- **Monaco Editor** - Code editor
- **TailwindCSS** - Styling framework

### FUSE Integration

**fuser** - Rust FUSE binding
- Support for Linux, macOS
- POSIX filesystem semantics
- High-performance kernel interaction

### Storage Backends

**evif-storage**:
- **sled** - Embedded database
- **rocksdb** - Key-value storage
- **rusoto_s3** / **aws-sdk-s3** - AWS S3
- **azure_storage** - Azure Blob
- **google-cloud-storage** - GCS

### Compilation Configuration

**Release Optimization**:
```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true          # Link-time optimization
codegen-units = 1   # Single compilation unit
strip = true        # Remove symbol table
```

**Dev Configuration**:
```toml
[profile.dev]
opt-level = 0  # No optimization, faster compilation
```

---

## Next Chapters

- [Chapter 4: Virtual Filesystem](./chapter-4-filesystem.md) - VFS abstraction and implementation
- [Chapter 5: Plugin Development](./chapter-5-plugins.md) - Plugin development guide
- [Chapter 6: FUSE Integration](./chapter-6-fuse.md) - FUSE mounting details
