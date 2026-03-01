# Chapter 5: Plugin Development

## Table of Contents

- [Plugin System Overview](#plugin-system-overview)
  - [Design Goals](#design-goals)
  - [AGFS Compatibility](#agfs-compatibility)
  - [Architecture Overview](#architecture-overview)
- [Core Interfaces](#core-interfaces)
  - [EvifPlugin Trait](#evifplugin-trait)
  - [File Operation Methods](#file-operation-methods)
  - [Directory Operation Methods](#directory-operation-methods)
  - [Extension Interfaces](#extension-interfaces)
- [Plugin Types](#plugin-types)
  - [Basic Storage Plugins](#basic-storage-plugins)
  - [Cloud Storage Plugins](#cloud-storage-plugins)
  - [Special Function Plugins](#special-function-plugins)
  - [OpenDAL Unified Plugin](#opendal-unified-plugin)
- [Plugin Development Guide](#plugin-development-guide)
  - [Creating a Basic Plugin](#creating-a-basic-plugin)
  - [Implementing EvifPlugin](#implementing-evifplugin)
  - [Configuration Management](#configuration-management)
  - [Error Handling](#error-handling)
  - [Testing Strategy](#testing-strategy)
- [Plugin Registration and Mounting](#plugin-registration-and-mounting)
  - [Mount Table Mechanism](#mount-table-mechanism)
  - [Path Routing](#path-routing)
  - [Symbolic Link Support](#symbolic-link-support)
- [Example Plugins](#example-plugins)
  - [MemFS: Memory Filesystem](#memfs-memory-filesystem)
  - [LocalFS: Local Filesystem](#localfs-local-filesystem)
  - [KVFS: Key-Value Store](#kvfs-key-value-store)
- [Implementation Status](#implementation-status)

---

## Plugin System Overview

### Design Goals

EVIF's plugin system provides a flexible, extensible filesystem abstraction layer that allows developers to support multiple storage backends by implementing a unified interface. The plugin system is a core component of EVIF's current architecture, exposing file operations through the REST API.

**Core Features**:

- **Unified Interface**: All plugins implement the `EvifPlugin` trait, providing standard POSIX file operations
- **Async Support**: Based on `async_trait` for asynchronous operations, suitable for high-concurrency scenarios
- **Configuration-Driven**: Supports dynamic configuration and validation for easy deployment and management
- **Hot-Pluggable**: Mount and unmount plugins at runtime without service restart
- **Extensibility**: Supports optional extension interfaces (HandleFS, Streamer)
- **AGFS Compatibility**: Fully compatible with AGFS plugin interface for easy migration

### AGFS Compatibility

EVIF plugin system is fully compatible with AGFS (Another Graph Filesystem) plugin interface:

| EVIF | AGFS | Description |
|------|------|-------------|
| `EvifPlugin` | `FileSystem` | Core plugin interface |
| `FileInfo` | `FileInfo` | File metadata |
| `OpenFlags` | `OpenFlag` | File open flags |
| `WriteFlags` | `WriteFlag` | File write flags |
| `FileHandle` | `FileHandle` | Stateful file handle |
| `HandleFS` | `HandleFS` | Handle support extension |
| `MountTable` | `MountableFS` | Plugin mount table |
| `validate()` | `Validate()` | Configuration validation |
| `get_readme()` | `GetReadme()` | Documentation generation |
| `get_config_params()` | `GetConfigParams()` | Configuration metadata |

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                     Application Layer                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  REST API    │  │  WebSocket   │  │  CLI Tools   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
├─────────────────────────────────────────────────────────┤
│                  EVIF Core                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ MountTable   │  │ HTTP Server  │  │ Auth Module  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
├─────────────────────────────────────────────────────────┤
│                  Plugin Layer (EvifPlugin)              │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │
│  │MemFS│ │Local│ │ KVFS│ │ S3  │ │Azure│ │ GCS │ ...  │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘      │
│  ┌──────────────────────────────────────────────────┐  │
│  │         OpenDAL Unified Plugin (50+ backends)    │  │
│  └──────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                  Storage Layer                          │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │
│  │Memory│ │Disk │ │Database│ │Object Storage│ │Cloud│...│
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘      │
└─────────────────────────────────────────────────────────┘
```

**Implementation Locations**:
- Core Interface: `crates/evif-core/src/plugin.rs`
- Mount Table: `crates/evif-core/src/mount_table.rs`
- Plugin Implementations: `crates/evif-plugins/src/`

---

## Core Interfaces

### EvifPlugin Trait

The `EvifPlugin` trait is the core interface that all plugins must implement, defining complete filesystem operations.

```rust
use async_trait::async_trait;
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};

#[async_trait]
pub trait EvifPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;

    /// Create file
    async fn create(&self, path: &str, perm: u32) -> EvifResult<()>;

    /// Create directory
    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>;

    /// Read file
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;

    /// Write file
    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
        -> EvifResult<u64>;

    /// Read directory contents
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;

    /// Get file information
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;

    /// Delete file or empty directory
    async fn remove(&self, path: &str) -> EvifResult<()>;

    /// Rename/move file
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;

    /// Recursively delete directory and all contents
    async fn remove_all(&self, path: &str) -> EvifResult<()>;

    // Optional methods (provide default implementations)
    async fn symlink(&self, target_path: &str, link_path: &str) -> EvifResult<()>;
    async fn readlink(&self, link_path: &str) -> EvifResult<String>;
    async fn chmod(&self, path: &str, mode: u32) -> EvifResult<()>;
    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()>;

    // Phase 8: Configuration management methods
    async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()>;
    fn get_readme(&self) -> String;
    fn get_config_params(&self) -> Vec<PluginConfigParam>;
}
```

### File Operation Methods

#### create - Create File

Create an empty file.

```rust
async fn create(&self, path: &str, perm: u32) -> EvifResult<()>
```

**Parameters**:
- `path`: File path
- `perm`: File permissions (octal, e.g., 0o644)

**Example**:
```rust
// Create a new file with 644 permissions
plugin.create("/data/file.txt", 0o644).await?;
```

#### read - Read File

Read file contents with support for offset and size limits.

```rust
async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>
```

**Parameters**:
- `path`: File path
- `offset`: Read offset (0 = from beginning)
- `size`: Read size (0 = read all)

**Example**:
```rust
// Read entire file
let data = plugin.read("/data/file.txt", 0, 0).await?;

// Read 1024 bytes starting from offset 100
let chunk = plugin.read("/data/large.bin", 100, 1024).await?;
```

#### write - Write File

Write data to file with support for multiple write modes.

```rust
async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
    -> EvifResult<u64>
```

**Parameters**:
- `path`: File path
- `data`: Data to write
- `offset`: Write offset (-1 = ignore)
- `flags`: Write flags

**WriteFlags**:
```rust
bitflags::bitflags! {
    pub struct WriteFlags: u32 {
        const APPEND = 1 << 0;      // Append write
        const CREATE = 1 << 1;      // Create file
        const EXCLUSIVE = 1 << 2;   // Exclusive create
        const TRUNCATE = 1 << 3;    // Truncate file
        const SYNC = 1 << 4;        // Sync write
    }
}
```

**Example**:
```rust
use evif_core::WriteFlags;

// Create and write
let written = plugin.write(
    "/data/file.txt",
    b"Hello, EVIF!".to_vec(),
    0,
    WriteFlags::CREATE
).await?;

// Append write
let appended = plugin.write(
    "/data/log.txt",
    b"New log entry\n".to_vec(),
    -1,
    WriteFlags::APPEND
).await?;

// Truncate and write
plugin.write(
    "/data/temp.txt",
    b"Overwritten".to_vec(),
    0,
    WriteFlags::TRUNCATE | WriteFlags::CREATE
).await?;
```

### Directory Operation Methods

#### mkdir - Create Directory

Create a new directory.

```rust
async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>
```

**Example**:
```rust
// Create directory with 755 permissions
plugin.mkdir("/data/subdir", 0o755).await?;
```

#### readdir - Read Directory

List directory contents.

```rust
async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>
```

**FileInfo Structure**:
```rust
pub struct FileInfo {
    pub name: String,       // File/directory name
    pub size: u64,          // Size (bytes)
    pub mode: u32,          // Permission mode
    pub modified: chrono::DateTime<chrono::Utc>,  // Modification time
    pub is_dir: bool,       // Is directory
}
```

**Example**:
```rust
let entries = plugin.readdir("/data").await?;
for entry in entries {
    let type_str = if entry.is_dir { "DIR" } else { "FILE" };
    println!("{} {}: {} bytes", type_str, entry.name, entry.size);
}
```

#### remove - Delete File or Empty Directory

Delete a single file or empty directory.

```rust
async fn remove(&self, path: &str) -> EvifResult<()>
```

**Example**:
```rust
// Delete file
plugin.remove("/data/file.txt").await?;

// Delete empty directory
plugin.remove("/data/empty_dir").await?;
```

#### remove_all - Recursive Delete

Recursively delete a directory and all its contents.

```rust
async fn remove_all(&self, path: &str) -> EvifResult<()>
```

**Example**:
```rust
// Delete entire directory tree
plugin.remove_all("/data/nested_dir").await?;
```

#### rename - Rename/Move

Rename or move files and directories.

```rust
async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>
```

**Example**:
```rust
// Rename file
plugin.rename("/data/old.txt", "/data/new.txt").await?;

// Move file to subdirectory
plugin.rename("/data/file.txt", "/data/subdir/file.txt").await?;
```

### Extension Interfaces

#### FileHandle - Stateful File Handle

For file operations that require maintaining state (e.g., large file chunked transfers), plugins can implement the `FileHandle` interface.

```rust
#[async_trait]
pub trait FileHandle: Send + Sync {
    fn id(&self) -> i64;
    fn path(&self) -> &str;
    async fn read(&mut self, buf: &mut [u8]) -> EvifResult<usize>;
    async fn read_at(&self, buf: &mut [u8], offset: u64) -> EvifResult<usize>;
    async fn write(&mut self, data: &[u8]) -> EvifResult<usize>;
    async fn write_at(&self, data: &[u8], offset: u64) -> EvifResult<usize>;
    async fn seek(&mut self, offset: i64, whence: u8) -> EvifResult<i64>;
    async fn sync(&self) -> EvifResult<()>;
    async fn close(&mut self) -> EvifResult<()>;
    async fn stat(&self) -> EvifResult<FileInfo>;
    fn flags(&self) -> OpenFlags;
}
```

#### HandleFS - Handle Support Extension

Plugins can optionally implement the `HandleFS` extension interface to support stateful file operations.

```rust
#[async_trait]
pub trait HandleFS: EvifPlugin {
    async fn open_handle(&self, path: &str, flags: OpenFlags, mode: u32)
        -> EvifResult<Box<dyn FileHandle>>;
    async fn get_handle(&self, id: i64) -> EvifResult<Box<dyn FileHandle>>;
    async fn close_handle(&self, id: i64) -> EvifResult<()>;
}
```

---

## Plugin Types

EVIF provides various types of plugins covering different storage backends and use cases.

### Basic Storage Plugins

#### MemFS

Memory filesystem plugin where data is stored only in process memory and lost on restart.

**Features**:
- ✅ Complete filesystem functionality
- ✅ High performance (memory operations)
- ✅ No configuration required
- ⚠️ Data volatility (lost on restart)

**Use Cases**:
- Temporary file storage
- Cache layer
- Testing and development

**Configuration**:
```json
{
  "mount": "/mem",
  "plugin": "memfs"
}
```

#### LocalFS

Local filesystem plugin that maps host directories to EVIF.

**Features**:
- ✅ Complete filesystem functionality
- ✅ Data persistence
- ✅ Read-only mode support
- ✅ Path traversal protection

**Use Cases**:
- Local file access
- Data persistence
- Configuration file management

**Configuration**:
```json
{
  "mount": "/local",
  "plugin": "localfs",
  "config": {
    "root": "/tmp/evif-local"
  }
}
```

#### KVFS

Key-value store plugin that maps KV storage as a filesystem.

**Features**:
- ✅ Simple key-value interface
- ✅ Support for arbitrary backends (memory, Redis, etc.)
- ✅ High-performance read/write

**Use Cases**:
- Metadata storage
- Cache systems
- Configuration management

**Configuration**:
```json
{
  "mount": "/kv",
  "plugin": "kvfs",
  "config": {
    "prefix": "app"
  }
}
```

### Cloud Storage Plugins

#### S3FS

AWS S3 object storage plugin.

**Features**:
- ✅ S3 API compatible
- ✅ Directory cache optimization
- ✅ Metadata cache
- ✅ Multi-region support

**Configuration**:
```json
{
  "mount": "/s3",
  "plugin": "s3fs",
  "config": {
    "region": "us-east-1",
    "bucket": "my-bucket",
    "access_key": "AKIA...",
    "secret_key": "...",
    "enable_dir_cache": true,
    "enable_stat_cache": true
  }
}
```

#### AzureBlobFS

Azure Blob Storage plugin.

**Configuration**:
```json
{
  "mount": "/azure",
  "plugin": "azureblobfs",
  "config": {
    "container": "my-container",
    "account": "myaccount",
    "access_key": "...",
    "endpoint": "https://myaccount.blob.core.windows.net"
  }
}
```

#### GcsFS

Google Cloud Storage plugin.

**Configuration**:
```json
{
  "mount": "/gcs",
  "plugin": "gcsfs",
  "config": {
    "bucket": "my-bucket",
    "credentials_path": "/path/to/credentials.json"
  }
}
```

### Special Function Plugins

#### HttpFS

HTTP filesystem plugin for accessing remote files via HTTP.

**Features**:
- ✅ HTTP/HTTPS support
- ✅ Range request support
- ✅ Authentication support

**Configuration**:
```json
{
  "mount": "/http",
  "plugin": "httpfs",
  "config": {
    "base_url": "https://example.com/files",
    "auth_token": "optional-token"
  }
}
```

#### StreamFS

Streaming filesystem plugin for real-time data streams.

**Features**:
- ✅ WebSocket support
- ✅ Server-Sent Events support
- ✅ Real-time data push

**Configuration**:
```json
{
  "mount": "/stream",
  "plugin": "streamfs",
  "config": {
    "buffer_size": 1048576
  }
}
```

#### EncryptedFS

Encrypted filesystem plugin with transparent data encryption.

**Features**:
- ✅ AES-256-GCM encryption
- ✅ Transparent encryption/decryption
- ✅ Key management

**Configuration**:
```json
{
  "mount": "/encrypted",
  "plugin": "encryptedfs",
  "config": {
    "key": "base64-encoded-key",
    "underlying_plugin": "localfs",
    "underlying_config": {
      "root": "/data/encrypted"
    }
  }
}
```

### OpenDAL Unified Plugin

Unified storage plugin based on Apache OpenDAL, supporting 50+ storage backends.

**Supported Services**:
- ✅ Memory
- ✅ Fs (local filesystem)
- ✅ S3 (AWS S3 and compatible storage)
- ✅ Azblob (Azure Blob Storage)
- ✅ Gcs (Google Cloud Storage)
- ✅ Oss (Alibaba Cloud Object Storage)
- ✅ Cos (Tencent Cloud Object Storage)
- ✅ Obs (Huawei Cloud Object Storage)

**Advantages**:
- Unified configuration interface
- Unified error handling
- Automatic retry and timeout
- Built-in cache support

**Configuration Example**:
```json
{
  "mount": "/opendal",
  "plugin": "opendal",
  "config": {
    "service": "s3",
    "bucket": "my-bucket",
    "region": "us-east-1",
    "root": "/prefix",
    "access_key": "...",
    "secret_key": "...",
    "enable_cache": true,
    "cache_max_entries": 1000,
    "cache_ttl_secs": 3600
  }
}
```

---

## Plugin Development Guide

### Creating a Basic Plugin

Let's create a simple "HelloFS" plugin as an example.

**Step 1: Define Plugin Structure**

```rust
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct HelloFsPlugin {
    // Internal storage: filename -> content
    files: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}
```

**Step 2: Implement Constructor**

```rust
impl HelloFsPlugin {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for HelloFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Implement EvifPlugin Trait**

```rust
#[async_trait]
impl EvifPlugin for HelloFsPlugin {
    fn name(&self) -> &str {
        "hellofs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let mut files = self.files.write().await;
        if files.contains_key(path) {
            return Err(EvifError::InvalidInput("File exists".to_string()));
        }
        files.insert(path.to_string(), Vec::new());
        Ok(())
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let files = self.files.read().await;
        files.get(path)
            .cloned()
            .ok_or_else(|| EvifError::NotFound(path.to_string()))
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        let mut files = self.files.write().await;
        files.insert(path.to_string(), data);
        Ok(data.len() as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let files = self.files.read().await;
        let mut entries = Vec::new();

        for (name, data) in files.iter() {
            // Simple path prefix matching
            if name.starts_with(path) {
                entries.push(FileInfo {
                    name: name.clone(),
                    size: data.len() as u64,
                    mode: 0o644,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                });
            }
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let files = self.files.read().await;
        let data = files.get(path)
            .ok_or_else(|| EvifError::NotFound(path.to_string()))?;

        Ok(FileInfo {
            name: path.to_string(),
            size: data.len() as u64,
            mode: 0o644,
            modified: chrono::Utc::now(),
            is_dir: false,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let mut files = self.files.write().await;
        files.remove(path)
            .ok_or_else(|| EvifError::NotFound(path.to_string()))?;
        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let mut files = self.files.write().await;
        let data = files.remove(old_path)
            .ok_or_else(|| EvifError::NotFound(old_path.to_string()))?;
        files.insert(new_path.to_string(), data);
        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let mut files = self.files.write().await;
        // Delete all files with path prefix
        files.retain(|k, _| !k.starts_with(path));
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        // HelloFS doesn't support directories, but return Ok for interface compatibility
        Ok(())
    }
}
```

### Implementing EvifPlugin

#### Required Methods

All plugins must implement the following methods:

1. **`name()`** - Return plugin name
2. **`create()`** - Create file
3. **`mkdir()`** - Create directory
4. **`read()`** - Read file
5. **`write()`** - Write file
6. **`readdir()`** - List directory
7. **`stat()`** - Get file information
8. **`remove()`** - Delete file or empty directory
9. **`rename()`** - Rename/move
10. **`remove_all()`** - Recursive delete

#### Optional Methods

The following methods provide default implementations that plugins can override as needed:

- **`symlink()`** - Create symbolic link
- **`readlink()`** - Read symbolic link
- **`chmod()`** - Change permissions
- **`truncate()`** - Truncate file

### Configuration Management

#### validate Method

Validate plugin configuration before mounting.

```rust
async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()> {
    if let Some(cfg) = config {
        // Check required fields
        if let Some(root) = cfg.get("root") {
            if root.is_null() || root.as_str().map(|s| s.is_empty()).unwrap_or(true) {
                return Err(EvifError::InvalidInput(
                    "config.root cannot be empty".to_string()
                ));
            }
        }

        // Check numeric ranges
        if let Some(cache_size) = cfg.get("cache_size") {
            if let Some(size) = cache_size.as_u64() {
                if size > 10_000_000 {
                    return Err(EvifError::InvalidInput(
                        "cache_size too large (max 10M)".to_string()
                    ));
                }
            }
        }
    }
    Ok(())
}
```

#### get_readme Method

Return the plugin's README documentation.

```rust
fn get_readme(&self) -> String {
    r#"# HelloFS Plugin

A simple example plugin demonstrating EVIF plugin development.

## Configuration

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| message | string | no | Welcome message, default "Hello" |

## Example

```json
{
  "mount": "/hello",
  "plugin": "hellofs",
  "config": {
    "message": "Welcome"
  }
}
```

## Features

- ✅ Basic file operations
- ✅ Memory storage
- ⚠️ Data volatility (lost on restart)
"#.to_string()
}
```

#### get_config_params Method

Return configuration parameter metadata.

```rust
use evif_core::PluginConfigParam;

fn get_config_params(&self) -> Vec<PluginConfigParam> {
    vec![
        PluginConfigParam {
            name: "message".to_string(),
            param_type: "string".to_string(),
            required: false,
            default: Some("Hello".to_string()),
            description: Some("Welcome message".to_string()),
        },
        PluginConfigParam {
            name: "max_size".to_string(),
            param_type: "int".to_string(),
            required: false,
            default: Some("1048576".to_string()),  // 1MB
            description: Some("Maximum file size (bytes)".to_string()),
        },
    ]
}
```

### Error Handling

Use `EvifError` to define clear error types.

```rust
use evif_core::{EvifError, EvifResult};

async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
    // Path validation
    if path.is_empty() {
        return Err(EvifError::InvalidPath("Path cannot be empty".to_string()));
    }

    // Permission check
    if !self.has_read_permission(path).await {
        return Err(EvifError::PermissionDenied("Read access denied".to_string()));
    }

    // File not found
    let file = self.find_file(path).await
        .ok_or_else(|| EvifError::NotFound(path.to_string()))?;

    // Read data
    let data = file.read(offset, size).await
        .map_err(|e| EvifError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Read failed: {}", e)
        )))?;

    Ok(data)
}
```

**Common Error Types**:
- `EvifError::NotFound` - File/directory not found
- `EvifError::InvalidPath` - Invalid path
- `EvifError::PermissionDenied` - Insufficient permissions
- `EvifError::ReadOnly` - Write operation in read-only mode
- `EvifError::NotSupportedGeneric` - Operation not supported
- `EvifError::InvalidInput` - Invalid input parameter
- `EvifError::Io` - I/O error

### Testing Strategy

#### Unit Tests

Write unit tests for each plugin method.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use evif_core::WriteFlags;

    #[tokio::test]
    async fn test_create_and_read() {
        let plugin = HelloFsPlugin::new();

        // Create file
        plugin.create("/test.txt", 0o644).await.unwrap();

        // Write data
        plugin.write(
            "/test.txt",
            b"Hello, World!".to_vec(),
            0,
            WriteFlags::CREATE
        ).await.unwrap();

        // Read data
        let data = plugin.read("/test.txt", 0, 0).await.unwrap();
        assert_eq!(data, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_remove() {
        let plugin = HelloFsPlugin::new();

        plugin.create("/to_remove.txt", 0o644).await.unwrap();
        plugin.remove("/to_remove.txt").await.unwrap();

        let result = plugin.stat("/to_remove.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rename() {
        let plugin = HelloFsPlugin::new();

        plugin.create("/old.txt", 0o644).await.unwrap();
        plugin.rename("/old.txt", "/new.txt").await.unwrap();

        let result = plugin.stat("/old.txt").await;
        assert!(result.is_err());

        let result = plugin.stat("/new.txt").await;
        assert!(result.is_ok());
    }
}
```

#### Integration Tests

Test plugin integration with mount table.

```rust
#[tokio::test]
async fn test_mount_table_integration() {
    use evif_core::MountTable;

    let mount_table = MountTable::new();
    let plugin = Arc::new(HelloFsPlugin::new());

    // Mount plugin
    mount_table.mount("/hello", plugin).await.unwrap();

    // Access through mount table
    mount_table.create("/hello/test.txt", 0o644).await.unwrap();
    mount_table.write(
        "/hello/test.txt",
        b"Mount test".to_vec(),
        0,
        WriteFlags::CREATE
    ).await.unwrap();

    let data = mount_table.read("/hello/test.txt", 0, 0).await.unwrap();
    assert_eq!(data, b"Mount test");
}
```

---

## Plugin Registration and Mounting

### Mount Table Mechanism

`MountTable` manages all plugin mounts and routing.

```rust
use evif_core::MountTable;
use std::sync::Arc;

// Create mount table
let mount_table = MountTable::new();

// Create plugin instances
let memfs = Arc::new(MemFsPlugin::new());
let localfs = Arc::new(LocalFsPlugin::new("/tmp/data"));

// Mount plugins
mount_table.mount("/mem", memfs).await.unwrap();
mount_table.mount("/local", localfs).await.unwrap();
```

### Path Routing

The mount table uses **longest prefix matching** algorithm to route file operations to corresponding plugins.

```rust
// Mount examples
// /mem    -> MemFsPlugin
// /local  -> LocalFsPlugin
// /s3/bucket -> S3FsPlugin

// Path routing examples
"/mem/file.txt"           -> MemFsPlugin
"/local/data/file.txt"    -> LocalFsPlugin
"/s3/bucket/file.txt"     -> S3FsPlugin
"/s3/bucket/nested/file"  -> S3FsPlugin (longest prefix match)
```

**Longest Prefix Matching Algorithm**:

```rust
pub async fn lookup(&self, path: &str) -> Option<(Arc<dyn EvifPlugin>, String)> {
    let mounts = self.mounts.read().await;
    let mut best_match: Option<&str> = None;
    let mut best_len = 0;

    // Iterate all mount points, find longest prefix match
    for mount_point in mounts.keys() {
        if path.starts_with(mount_point) && mount_point.len() > best_len {
            best_match = Some(mount_point);
            best_len = mount_point.len();
        }
    }

    if let Some(mount_point) = best_match {
        let plugin = mounts.get(mount_point)?.clone();
        let relative_path = path[best_len..].trim_start_matches('/');
        Some((plugin, relative_path.to_string()))
    } else {
        None
    }
}
```

**Example**:

```rust
// Mount
mount_table.mount("/s3", s3_plugin).await.unwrap();
mount_table.mount("/s3/bucket", nested_plugin).await.unwrap();

// Request "/s3/bucket/file.txt"
// Match results:
// - "/s3" length 3
// - "/s3/bucket" length 10
// Select "/s3/bucket" (longer)
```

### Symbolic Link Support

The mount table supports virtual symbolic links, allowing cross-filesystem links.

```rust
// Create symbolic link
mount_table.symlink("/local/data", "/data").await.unwrap();

// Resolve symbolic link
let (resolved, is_link) = mount_table.resolve_symlink("/data").await;
assert!(is_link);
assert_eq!(resolved, "/local/data");

// Recursive resolution (supports links pointing to links)
mount_table.symlink("/local/archive", "/archive").await.unwrap();
mount_table.symlink("/archive/2024", "/data2024").await.unwrap();

let final_path = mount_table.resolve_symlink_recursive("/data2024", 10).await.unwrap();
assert_eq!(final_path, "/local/archive/2024");
```

**Cycle Detection**:

```rust
// Create circular links
mount_table.symlink("/a", "/b").await.unwrap();
mount_table.symlink("/b", "/a").await.unwrap();

// Recursive resolution will detect cycle
let result = mount_table.resolve_symlink_recursive("/a", 10).await;
assert!(result.is_err());  // Returns cycle error
```

---

## Example Plugins

### MemFS: Memory Filesystem

Complete memory filesystem implementation supporting directory hierarchy.

**Core Features**:
- Complete file and directory operations
- Recursive directory operations
- File metadata management
- Hierarchical path support

**Implementation Highlights**:

```rust
pub struct MemFsPlugin {
    root: Arc<RwLock<MemNode>>,
}

struct MemNode {
    name: String,
    is_dir: bool,
    data: Vec<u8>,
    mode: u32,
    modified: chrono::DateTime<chrono::Utc>,
    children: Option<HashMap<String, Arc<RwLock<MemNode>>>>,
}

impl MemFsPlugin {
    /// Recursively find node
    async fn find_node(&self, path: &str) -> EvifResult<Arc<RwLock<MemNode>>> {
        let clean_path = path.trim_start_matches('/');
        let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();

        let mut current = Arc::clone(&self.root);
        for part in parts {
            let node_ref = current.read().await;
            if !node_ref.is_dir {
                return Err(EvifError::InvalidPath("Not a directory".to_string()));
            }

            if let Some(children) = &node_ref.children {
                if let Some(child) = children.get(part) {
                    current = Arc::clone(child);
                } else {
                    return Err(EvifError::NotFound(path.to_string()));
                }
            }
        }

        Ok(current)
    }
}
```

### LocalFS: Local Filesystem

Maps host directories to EVIF with security checks and read-only mode support.

**Core Features**:
- Complete filesystem operations
- Path traversal protection
- Read-only mode support
- Permission and metadata preservation

**Security Check**:

```rust
impl LocalFsPlugin {
    fn resolve_path(&self, path: &str) -> EvifResult<PathBuf> {
        let clean_path = path.trim_start_matches('/');
        let full = self.base_path.join(clean_path);

        // Security check: prevent path traversal attacks
        if full.exists() {
            let canonical = full.canonicalize()
                .map_err(|_| EvifError::InvalidPath(path.to_string()))?;

            let base_canonical = self.base_path.canonicalize()
                .map_err(|_| EvifError::InvalidPath("base_path".to_string()))?;

            if !canonical.starts_with(&base_canonical) {
                return Err(EvifError::InvalidPath("Path traversal detected".to_string()));
            }
        }

        Ok(full)
    }
}
```

### KVFS: Key-Value Store

Maps key-value storage as a filesystem.

**Core Features**:
- Simple key-value interface
- Path to key mapping
- Directory enumeration

**Path Mapping**:

```rust
impl KvfsPlugin {
    /// Convert file path to storage key
    fn path_to_key(&self, path: &str) -> EvifResult<String> {
        let clean_path = path.trim_start_matches('/');
        if clean_path.is_empty() {
            return Err(EvifError::InvalidPath("Path cannot be empty".to_string()));
        }
        Ok(format!("{}/{}", self.prefix.trim_end_matches('/'), clean_path))
    }

    /// Convert file path to key prefix (for directory listing)
    fn path_to_prefix(&self, path: &str) -> EvifResult<String> {
        let clean_path = path.trim_start_matches('/');
        let base = self.prefix.trim_end_matches('/');
        if clean_path.is_empty() || clean_path == "/" {
            Ok(format!("{}/", base))
        } else {
            Ok(format!("{}/{}/", base, clean_path.trim_end_matches('/')))
        }
    }
}
```

**Usage Example**:

```rust
// Create KVFS plugin
let kvfs = KvfsPlugin::new("app");

// Write file
kvfs.write("/config.json", config_data, 0, WriteFlags::CREATE).await?;

// Path maps to key: "/app/config.json"
kvfs.store.put("app/config.json".to_string(), config_data).await?;

// Read directory
let entries = kvfs.readdir("/").await?;
// Returns all keys with "app/" prefix
```

---

## Implementation Status

### ✅ Completed

**Core Interfaces**:
- ✅ EvifPlugin trait fully defined (20+ methods)
- ✅ FileHandle trait (stateful file operations)
- ✅ HandleFS extension interface
- ✅ Configuration management (validate, get_readme, get_config_params)
- ✅ Error type definitions (EvifError)

**Basic Plugins (8)**:
- ✅ MemFS - Memory filesystem
- ✅ LocalFS - Local filesystem
- ✅ KVFS - Key-value store
- ✅ QueueFsPlugin - Queue filesystem
- ✅ ServerInfoFsPlugin - Server information
- ✅ HttpFsPlugin - HTTP filesystem
- ✅ StreamFsPlugin - Streaming filesystem
- ✅ ProxyFsPlugin - Proxy filesystem
- ✅ DevFsPlugin - Device filesystem
- ✅ HelloFsPlugin - Example plugin
- ✅ HeartbeatFsPlugin - Heartbeat plugin
- ✅ HandleFsPlugin - Handle filesystem
- ✅ TieredFsPlugin - Tiered storage
- ✅ EncryptedFsPlugin - Encrypted filesystem

**Cloud Storage Plugins (Optional Features)**:
- ✅ S3FsPlugin - AWS S3
- ✅ SqlfsPlugin - SQL storage
- ✅ GptfsPlugin - GPT integration
- ✅ VectorFsPlugin - Vector storage
- ✅ StreamRotateFSPlugin - Stream rotation

**OpenDAL Unified Plugin**:
- ✅ OpendalPlugin - Unified storage interface
- ✅ S3FsPlugin (OpenDAL) - S3 support
- ✅ AzureBlobFsPlugin - Azure Blob
- ✅ GcsFsPlugin - Google Cloud Storage
- ✅ AliyunOssFsPlugin - Alibaba Cloud OSS
- ✅ TencentCosFsPlugin - Tencent Cloud COS
- ✅ HuaweiObsFsPlugin - Huawei Cloud OBS
- ✅ MinioFsPlugin - MinIO

**Mount System**:
- ✅ MountTable - Plugin mount table
- ✅ Longest prefix match routing
- ✅ Virtual symbolic link support
- ✅ Recursive symlink resolution
- ✅ Cycle detection (MAX_SYMLINK_DEPTH = 40)

### ⚠️ Partial Implementation

**OpenDAL Extensions**:
- ⚠️ WebDAV, FTP, SFTP plugins temporarily disabled (awaiting OpenDAL 0.50.x TLS conflict fix)
- ⚠️ Cache layer needs performance optimization

**StreamFsPlugin**:
- ⚠️ WebSocket support needs enhancement
- ⚠️ SSE (Server-Sent Events) partially implemented

### ❌ Not Implemented

**Advanced Features**:
- ❌ Plugin hot reload (requires service restart)
- ❌ Plugin dependency management (dependencies between plugins)
- ❌ Plugin version compatibility checking
- ❌ Plugin sandbox/security isolation

**Performance Optimization**:
- ❌ Batch operation optimization (bulk read/write)
- ❌ Concurrency control (limit concurrent operations per plugin)
- ❌ Connection pool management (database, network connections)

**Monitoring and Diagnostics**:
- ❌ Plugin performance metrics (QPS, latency)
- ❌ Plugin health checks
- ❌ Plugin logging and debugging tools

### Current Architecture Path

**Main Path**: REST API → EvifPlugin → MountTable → Storage Plugins

**Not Used**:
- VFS layer (FileSystem trait defined but not integrated into main path)
- Graph integration (PathResolver's graph query not implemented)

### Recommended Practices

**Developing New Plugins**:
1. Implement `EvifPlugin` trait
2. Implement configuration validation (validate)
3. Provide README documentation (get_readme)
4. Define configuration parameters (get_config_params)
5. Write unit tests
6. Write integration tests (mount table integration)

**Deploying Plugins**:
1. Compile plugin into `evif-plugins` crate
2. Use Cargo features to control optional dependencies
3. Mount via REST API or configuration file
4. Validate configuration parameter effectiveness
5. Monitor plugin performance and errors

---

## Related Chapters

- [Chapter 3: Architecture](chapter-3-architecture.md) - EVIF overall architecture
- [Chapter 4: Virtual Filesystem](chapter-4-virtual-filesystem.md) - VFS abstraction layer
- [Chapter 6: FUSE Integration](chapter-6-fuse.md) - FUSE filesystem support
