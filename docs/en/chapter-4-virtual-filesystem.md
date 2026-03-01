# Chapter 4: Virtual Filesystem

## Table of Contents

- [VFS Overview](#vfs-overview)
- [Core Abstractions](#core-abstractions)
  - [FileSystem Trait](#filesystem-trait)
  - [File Abstractions](#file-abstractions)
  - [Directory Abstractions](#directory-abstractions)
- [Path Resolution](#path-resolution)
  - [PathResolver](#pathresolver)
  - [Path Normalization](#path-normalization)
  - [Symbolic Link Handling](#symbolic-link-handling)
- [Storage Backends](#storage-backends)
  - [ContentStore](#contentstore)
  - [INode Management](#inode-management)
- [File Operations](#file-operations)
  - [Open and Close](#open-and-close)
  - [Read and Write](#read-and-write)
  - [Seek Operations](#seek-operations)
- [Mount Table Architecture](#mount-table-architecture)
  - [MountTable](#mounttable)
  - [Longest Prefix Matching](#longest-prefix-matching)
  - [Virtual Symbolic Links](#virtual-symbolic-links)
- [Implementation Status](#implementation-status)

---

## VFS Overview

### Design Goals

EVIF's Virtual Filesystem (VFS) layer provides a POSIX-compliant filesystem abstraction interface supporting unified access to multiple storage backends. The VFS layer is a core component of the "Graph + VFS" technology stack. While the current main path uses the plugin system, the VFS abstraction provides a foundation for future expansion.

**Core Features**:

- **POSIX Compatible**: Provides Unix-like filesystem operation interfaces
- **Async Support**: `async_trait`-based asynchronous operations, suitable for high-concurrency scenarios
- **Multi-backend Support**: Pluggable storage backends (memory, disk, cloud storage, etc.)
- **Path Resolution**: Supports absolute paths, relative paths, symbolic links
- **Concurrent Safety**: Thread-safe using `DashMap` and `Arc`

### Architecture Layers

```
┌─────────────────────────────────────┐
│   Application Layer (REST/FUSE/CLI) │
├─────────────────────────────────────┤
│         VFS Abstraction Layer       │
│  ┌──────────┬──────────┬──────────┐ │
│  │ FileSystem│PathResolver│Handle │ │
│  └──────────┴──────────┴──────────┘ │
├─────────────────────────────────────┤
│         Storage Implementation      │
│  ┌──────────┬──────────┬──────────┐ │
│  │ContentStore│INodeCache│DEntry  │ │
│  └──────────┴──────────┴──────────┘ │
├─────────────────────────────────────┤
│         Graph Structure Layer       │
│  ┌──────────┬──────────┬──────────┐ │
│  │  Graph   │  NodeId  │  VNode  │ │
│  └──────────┴──────────┴──────────┘ │
└─────────────────────────────────────┘
```

### Implementation Location

**Core Code**: `crates/evif-vfs/src/`

- `lib.rs` - Module exports and version information
- `vfs.rs` - VFS main implementation
- `filesystem.rs` - FileSystem trait definition
- `file.rs` - File abstractions and handle management
- `dir.rs` - Directory operations
- `path.rs` - Path resolver
- `inode.rs` - Index node management
- `dentry.rs` - Directory entry cache
- `vnode.rs` - Virtual node
- `error.rs` - Error type definitions

**Implementation Status**: ⚠️ **Partially Implemented** - VFS abstraction layer defined, but current main path (Plugin + REST) doesn't use this layer.

---

## Core Abstractions

### FileSystem Trait

The `FileSystem` trait is the core abstraction of VFS, defining complete filesystem operation interfaces. All filesystem implementations must implement this trait.

#### File Operations

```rust
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Open file
    async fn open(&self, path: &Path, flags: OpenFlags) -> VfsResult<FileHandle>;

    /// Close file
    async fn close(&self, handle: FileHandle) -> VfsResult<()>;

    /// Read from file
    async fn read(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> VfsResult<usize>;

    /// Write to file
    async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize>;

    /// Sync file to disk
    async fn fsync(&self, handle: FileHandle) -> VfsResult<()>;

    /// Get file size
    async fn get_file_size(&self, path: &Path) -> VfsResult<u64>;

    /// Set file size
    async fn set_file_size(&self, path: &Path, size: u64) -> VfsResult<()>;

    /// Create file
    async fn create(&self, path: &Path, mode: FileMode) -> VfsResult<FileHandle>;

    /// Delete file
    async fn unlink(&self, path: &Path) -> VfsResult<()>;

    /// Rename file
    async fn rename(&self, old_path: &Path, new_path: &Path) -> VfsResult<()>;
}
```

#### Directory Operations

```rust
    /// Create directory
    async fn mkdir(&self, path: &Path, mode: FileMode) -> VfsResult<()>;

    /// Remove directory
    async fn rmdir(&self, path: &Path) -> VfsResult<()>;

    /// Read directory contents
    async fn readdir(&self, path: &Path) -> VfsResult<Vec<DirEntry>>;

    /// Open directory
    async fn opendir(&self, path: &Path) -> VfsResult<u64>;

    /// Close directory
    async fn closedir(&self, dir_handle: u64) -> VfsResult<()>;
```

#### Metadata Operations

```rust
    /// Get file attributes
    async fn getattr(&self, path: &Path) -> VfsResult<FileAttributes>;

    /// Set file attributes
    async fn setattr(&self, path: &Path, attrs: FileAttributes) -> VfsResult<()>;

    /// Change file permissions
    async fn chmod(&self, path: &Path, mode: u32) -> VfsResult<()>;

    /// Change file owner
    async fn chown(&self, path: &Path, uid: u32, gid: u32) -> VfsResult<()>;

    /// Change access and modification times
    async fn utime(&self, path: &Path, atime: u64, mtime: u64) -> VfsResult<()>;
```

#### Symbolic Link Operations

```rust
    /// Create symbolic link
    async fn symlink(&self, target: &Path, link_path: &Path) -> VfsResult<()>;

    /// Read symbolic link target
    async fn readlink(&self, path: &Path) -> VfsResult<PathBuf>;
```

#### Filesystem Operations

```rust
    /// Get filesystem statistics
    async fn statfs(&self) -> VfsResult<FileSystemStats>;

    /// Sync filesystem
    async fn sync(&self) -> VfsResult<()>;

    /// Check if path exists
    async fn exists(&self, path: &Path) -> VfsResult<bool>;

    /// Check if path is a file
    async fn is_file(&self, path: &Path) -> VfsResult<bool>;

    /// Check if path is a directory
    async fn is_directory(&self, path: &Path) -> VfsResult<bool>;

    /// Get absolute path
    async fn realpath(&self, path: &Path) -> VfsResult<PathBuf>;
```

#### Default Implementations

The `FileSystem` trait provides convenient default implementations:

```rust
    /// Batch read
    async fn read_full(&self, handle: FileHandle, offset: u64, size: usize) -> VfsResult<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        let total_read = self.read(handle, offset, &mut buffer).await?;
        buffer.truncate(total_read);
        Ok(buffer)
    }

    /// Batch write
    async fn write_full(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize> {
        let mut total_written = 0;
        let mut current_offset = offset;

        while total_written < data.len() {
            let written = self.write(handle, current_offset, &data[total_written..]).await?;
            if written == 0 {
                return Err(VfsError::NoSpaceLeft);
            }
            total_written += written;
            current_offset += written as u64;
        }

        Ok(total_written)
    }

    /// Recursively create directories
    async fn mkdir_all(&self, path: &Path) -> VfsResult<()> {
        if self.exists(path).await? {
            return if self.is_directory(path).await? {
                Ok(())
            } else {
                Err(VfsError::NotADirectory(path.display().to_string()))
            };
        }

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                self.mkdir_all(parent).await?;
            }
        }

        self.mkdir(path, FileMode::default()).await
    }

    /// Recursively remove directories
    async fn remove_dir_all(&self, path: &Path) -> VfsResult<()> {
        if !self.is_directory(path).await? {
            return self.unlink(path).await;
        }

        let entries = self.readdir(path).await?;
        for entry in entries {
            let entry_path = path.join(entry.name);
            if entry.file_type.is_directory() {
                self.remove_dir_all(&entry_path).await?;
            } else {
                self.unlink(&entry_path).await?;
            }
        }

        self.rmdir(path).await
    }
```

### File Abstractions

#### FileHandle

A file handle uniquely identifies an open file:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHandle(u64);

impl FileHandle {
    pub fn new(handle: u64) -> Self;
    pub fn value(&self) -> u64;
    pub fn is_invalid(&self) -> bool;
}
```

**Features**:
- Integer-based lightweight handle
- Serializable, supports cross-thread passing
- Value 0 indicates invalid handle

#### OpenFlags

File open flags define file access modes and special options:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub rw: bool,
    pub create: bool,
    pub exclusive: bool,
    pub append: bool,
    pub truncate: bool,
    pub nonblock: bool,
    pub sync: bool,
    pub nofollow: bool,
}
```

**Predefined Modes**:

```rust
/// Read-only mode
pub const READ_ONLY: OpenFlags = OpenFlags { read: true, ... };

/// Write-only mode
pub const WRITE_ONLY: OpenFlags = OpenFlags { write: true, ... };

/// Read-write mode
pub const READ_WRITE: OpenFlags = OpenFlags { rw: true, ... };

/// Create mode
pub const CREATE: OpenFlags = OpenFlags { create: true, ... };

/// Append mode
pub const APPEND: OpenFlags = OpenFlags { append: true, ... };

/// Truncate mode
pub const TRUNCATE: OpenFlags = OpenFlags { truncate: true, ... };
```

**Usage Examples**:

```rust
// Open file for reading
let handle = fs.open(Path::new("/file.txt"), OpenFlags::READ_ONLY).await?;

// Create file if it doesn't exist
let handle = fs.open(Path::new("/new.txt"), OpenFlags::CREATE).await?;

// Append write
let handle = fs.open(Path::new("/log.txt"), OpenFlags::APPEND).await?;

// Read-write access
let handle = fs.open(Path::new("/data.bin"), OpenFlags::READ_WRITE).await?;
```

#### FileMode

File mode defines file permissions and type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMode {
    pub permissions: u32,      // Permission bits (e.g., 0o644)
    pub file_type: FileType,   // File type
}
```

**Predefined Modes**:

```rust
/// Default file mode (0644)
pub const DEFAULT_FILE: FileMode = FileMode {
    permissions: 0o644,
    file_type: FileType::Regular,
};

/// Default directory mode (0755)
pub const DEFAULT_DIRECTORY: FileMode = FileMode {
    permissions: 0o755,
    file_type: FileType::Directory,
};
```

**Permission Checks**:

```rust
impl FileMode {
    /// Whether readable
    pub fn is_readable(&self) -> bool;

    /// Whether writable
    pub fn is_writable(&self) -> bool;

    /// Whether executable
    pub fn is_executable(&self) -> bool;
}
```

#### File

The `File` struct represents the state of an open file:

```rust
pub struct File {
    handle: FileHandle,
    offset: AtomicU64,
    flags: OpenFlags,
    size: u64,
}
```

**Core Methods**:

```rust
impl File {
    /// Create new file descriptor
    pub fn new(handle: FileHandle, flags: OpenFlags, size: u64) -> Self;

    /// Get file offset
    pub fn offset(&self) -> u64;

    /// Set file offset
    pub fn set_offset(&self, offset: u64);

    /// Increase file offset
    pub fn add_offset(&self, delta: u64) -> u64;

    /// Whether readable
    pub fn can_read(&self) -> bool;

    /// Whether writable
    pub fn can_write(&self) -> bool;

    /// Whether append mode
    pub fn is_append(&self) -> bool;

    /// Read data (update offset)
    pub fn read(&self, buf: &mut [u8], data: &[u8]) -> VfsResult<usize>;

    /// Write data (update offset)
    pub fn write(&self, data: &[u8]) -> VfsResult<u64>;

    /// Seek to position
    pub fn seek(&self, pos: SeekFrom) -> VfsResult<u64>;
}
```

**Seek Operations**:

```rust
pub enum SeekFrom {
    Start(u64),   // From start
    End(i64),     // From end
    Current(i64), // From current position
}

// Usage examples
file.seek(SeekFrom::Start(0))?;      // Seek to file start
file.seek(SeekFrom::End(-10))?;      // Seek to 10 bytes before file end
file.seek(SeekFrom::Current(100))?;  // Move 100 bytes forward from current position
```

#### FileHandleAllocator

The file handle allocator generates unique file handles:

```rust
pub struct FileHandleAllocator {
    next_handle: AtomicU64,
}

impl FileHandleAllocator {
    pub fn new() -> Self;
    pub fn alloc(&self) -> FileHandle;
}
```

**Features**:
- Atomic counter-based, thread-safe
- Starts from 1 and increments
- Ensures handle uniqueness

### Directory Abstractions

#### DirEntry

A directory entry represents one entry in a directory:

```rust
pub struct DirEntry {
    pub name: String,
    pub ino: u64,
    pub file_type: FileType,
}
```

**Field Descriptions**:
- `name` - File or directory name
- `ino` - Index node number
- `file_type` - File type (regular file, directory, symbolic link, etc.)

---

## Path Resolution

### PathResolver

`PathResolver` is responsible for resolving POSIX-style file paths to graph node IDs:

```rust
pub struct PathResolver {
    graph: Arc<Graph>,
    root_id: NodeId,
    cache: DashMap<PathBuf, NodeId>,
    max_symlink_depth: usize,
}
```

**Core Functionality**:

```rust
impl PathResolver {
    /// Create new path resolver
    pub fn new(graph: Arc<Graph>, root_id: NodeId) -> Self;

    /// Resolve path to node ID
    pub fn resolve(&self, path: &Path) -> VfsResult<NodeId>;

    /// Normalize path
    fn normalize_path(&self, path: &Path) -> VfsResult<PathBuf>;

    /// Create path to node mapping
    pub fn create_path(&self, path: &Path, node_id: NodeId) -> VfsResult<()>;

    /// Remove path cache
    pub fn invalidate(&self, path: &Path);

    /// Get parent path
    pub fn parent_path(&self, path: &Path) -> VfsResult<Option<PathBuf>>;

    /// Get basename
    pub fn basename(&self, path: &Path) -> VfsResult<String>;

    /// Clear cache
    pub fn clear_cache(&self);
}
```

### Path Normalization

The `normalize_path` method converts arbitrary paths to canonical form:

**Processing Rules**:

1. **Absolute vs Relative Paths**
   - `/foo/bar` → absolute path
   - `foo/bar` → relative path

2. **Special Directory Components**
   - `.` - current directory (remove)
   - `..` - parent directory (pop previous component)
   - `/` - root directory (keep)

3. **Path Validation**
   - Path length ≤ 4096 bytes
   - Filename length ≤ 255 bytes
   - No null characters (`\0`)
   - No Windows path prefixes (e.g., `C:\`)

**Examples**:

```rust
let resolver = PathResolver::new(graph, root_id);

// Absolute path
assert_eq!(resolver.normalize_path(Path::new("/foo/bar"))?,
           PathBuf::from("/foo/bar"));

// Relative path
assert_eq!(resolver.normalize_path(Path::new("foo/bar"))?,
           PathBuf::from("foo/bar"));

// Parent directory
assert_eq!(resolver.normalize_path(Path::new("/foo/../bar"))?,
           PathBuf::from("/bar"));

// Current directory
assert_eq!(resolver.normalize_path(Path::new("/foo/./bar"))?,
           PathBuf::from("/foo/bar"));

// Complex path
assert_eq!(resolver.normalize_path(Path::new("/a/b/../c/./d"))?,
           PathBuf::from("/a/c/d"));
```

**Error Handling**:

```rust
// Path too long
let result = resolver.normalize_path(Path::new(&"/".repeat(5000)));
assert!(matches!(result, Err(VfsError::PathTooLong)));

// Filename too long
let long_name = "a".repeat(300);
let result = resolver.normalize_path(Path::new(&format!("/{}", long_name)));
assert!(matches!(result, Err(VfsError::NameTooLong)));

// Contains null character
let result = resolver.normalize_path(Path::new("/foo\0bar"));
assert!(matches!(result, Err(VfsError::InvalidPath(_))));
```

### Symbolic Link Handling

`PathResolver` supports recursive symbolic link resolution with cycle detection:

```rust
impl PathResolver {
    /// Recursively resolve path (with depth limit)
    fn resolve_path(&self, path: &Path, depth: usize) -> VfsResult<NodeId> {
        if depth > self.max_symlink_depth {
            return Err(VfsError::SymbolicLinkLoop(path.display().to_string()));
        }

        // ... resolution logic

        // Check if symbolic link
        if node.node_type == NodeType::Symlink {
            let target = self.get_symlink_target(&node)?;
            let target_path = path.parent().unwrap_or(Path::new("/")).join(&target);

            // Recursive resolution
            return self.resolve_path(&target_path, depth + 1);
        }

        Ok(current_id)
    }
}
```

**Features**:
- Maximum recursion depth: 40 levels (`MAX_SYMLINK_DEPTH`)
- Detects circular references
- Automatically resolves symbolic links in path components

**Example Flow**:

```
/path/to/symlink1 → /another/path/symlink2 → /final/target
```

The resolver recursively follows symbolic links until reaching an actual file or hitting the maximum depth.

---

## Storage Backends

### ContentStore

`ContentStore` is responsible for storing and retrieving file content:

```rust
struct ContentStore {
    contents: DashMap<u64, Vec<u8>>,
}
```

**Core Operations**:

```rust
impl ContentStore {
    fn new() -> Self;

    /// Read content
    fn read(&self, node_id: u64, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = self.contents.get(&node_id)?;
        let start = offset as usize;
        if start >= data.len() {
            return Ok(0);
        }
        let end = std::cmp::min(start + buf.len(), data.len());
        let bytes_to_read = end - start;
        buf[..bytes_to_read].copy_from_slice(&data[start..end]);
        Ok(bytes_to_read)
    }

    /// Write content
    fn write(&self, node_id: u64, offset: u64, data: &[u8]) -> VfsResult<usize> {
        let mut existing = self.contents.entry(node_id).or_insert_with(Vec::new);
        let start = offset as usize;

        if offset as usize + data.len() > existing.len() {
            existing.resize(offset as usize + data.len(), 0);
        }

        existing[start..start + data.len()].copy_from_slice(data);
        Ok(data.len())
    }

    /// Get size
    fn size(&self, node_id: u64) -> VfsResult<u64> {
        self.contents.get(&node_id)
            .map(|data| data.len() as u64)
            .ok_or_else(|| VfsError::InternalError(format!("Node not found: {}", node_id)))
    }

    /// Truncate file
    fn truncate(&self, node_id: u64, size: u64) -> VfsResult<()> {
        let mut data = self.contents.entry(node_id).or_insert_with(Vec::new);
        data.resize(size as usize, 0);
        Ok(())
    }

    /// Delete content
    fn delete(&self, node_id: u64) {
        self.contents.remove(&node_id);
    }
}
```

**Features**:
- Concurrent-safe storage based on `DashMap`
- File content indexed by node ID
- Supports partial read/write and truncate operations
- Auto-expansion (when writing beyond current size)

### INode Management

#### INode Structure

Index nodes (INodes) represent metadata for files or directories in the filesystem:

```rust
pub struct INode {
    pub id: NodeId,       // Graph node ID
    pub ino: u64,         // INode number
    pub size: u64,        // File size
    pub blocks: u64,      // Block count
    pub mode: u32,        // Permission mode
    pub nlink: u64,       // Hard link count
    pub uid: u32,         // User ID
    pub gid: u32,         // Group ID
    pub rdev: u64,        // Device ID
    pub atime: u64,       // Access time
    pub mtime: u64,       // Modification time
    pub ctime: u64,       // Creation time
}
```

**Type Check Methods**:

```rust
impl INode {
    /// Whether directory
    pub fn is_directory(&self) -> bool {
        (self.mode & 0o170000) == 0o040000
    }

    /// Whether regular file
    pub fn is_regular_file(&self) -> bool {
        (self.mode & 0o170000) == 0o100000
    }

    /// Whether symbolic link
    pub fn is_symlink(&self) -> bool {
        (self.mode & 0o170000) == 0o120000
    }
}
```

#### INodeCache

`INodeCache` provides cache management for index nodes:

```rust
pub struct INodeCache {
    inodes: Arc<RwLock<HashMap<NodeId, INode>>>,
    ino_map: Arc<RwLock<HashMap<u64, NodeId>>>,
    next_ino: Arc<AtomicU64>,
    max_size: usize,
}
```

**Core Operations**:

```rust
impl INodeCache {
    pub fn new(max_size: usize) -> Self;

    /// Get INode
    pub async fn get(&self, id: &NodeId) -> Option<INode>;

    /// Get by INode number
    pub async fn get_by_ino(&self, ino: u64) -> Option<INode>;

    /// Insert INode
    pub async fn insert(&self, inode: INode) -> VfsResult<()>;

    /// Remove INode
    pub async fn remove(&self, id: &NodeId) -> Option<INode>;

    /// Allocate new INode number
    pub fn alloc_ino(&self) -> u64;

    /// Cache size
    pub async fn len(&self) -> usize;

    /// Whether empty
    pub async fn is_empty(&self) -> bool;

    /// Clear cache
    pub async fn clear(&self);
}
```

**Features**:
- Dual indexing: node ID and INode number
- Auto-eviction: removes old entries when exceeding max size
- Atomic number allocation
- Thread-safe

**Usage Example**:

```rust
let cache = INodeCache::new(1000);

// Create new INode
let id = NodeId::new_v4();
let ino = cache.alloc_ino();
let inode = INode::new(id, ino);

// Insert into cache
cache.insert(inode).await?;

// Query by ID
if let Some(inode) = cache.get(&id).await {
    println!("INode: size={}, mode={}", inode.size, inode.mode);
}

// Query by number
if let Some(inode) = cache.get_by_ino(ino).await {
    println!("Found inode by number: {}", inode.ino);
}

// Cleanup
cache.remove(&id).await;
```

---

## File Operations

### Open and Close

**Open File**:

```rust
async fn open(&self, path: &Path, flags: OpenFlags) -> VfsResult<FileHandle> {
    // Validate path
    self.validate_path(path)?;

    // Check read-only protection
    if self.config.read_only && flags.can_write() {
        return Err(VfsError::ReadOnlyFileSystem);
    }

    // Allocate handle
    let handle = self.handle_allocator.alloc();
    let file = Arc::new(File::new(handle, flags, size));

    // Check open file limit
    if self.open_files.len() >= self.config.max_open_files {
        return Err(VfsError::InternalError("Too many open files".to_string()));
    }

    self.open_files.insert(handle, file);
    Ok(handle)
}
```

**Close File**:

```rust
async fn close(&self, handle: FileHandle) -> VfsResult<()> {
    self.open_files.remove(&handle)
        .ok_or_else(|| VfsError::InvalidFileHandle(handle.value()))?;
    Ok(())
}
```

### Read and Write

**Read File**:

```rust
async fn read(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
    let file = self.open_files.get(&handle)
        .ok_or_else(|| VfsError::InvalidFileHandle(handle.value()))?;

    // Check read permission
    if !file.can_read() {
        return Err(VfsError::PermissionDenied("File not opened for reading".to_string()));
    }

    // Read from content store
    let node_id = handle.value();
    self.content_store.read(node_id, offset, buf)
}
```

**Write File**:

```rust
async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize> {
    // Check read-only protection
    if self.config.read_only {
        return Err(VfsError::ReadOnlyFileSystem);
    }

    let file = self.open_files.get(&handle)
        .ok_or_else(|| VfsError::InvalidFileHandle(handle.value()))?;

    // Check write permission
    if !file.can_write() {
        return Err(VfsError::PermissionDenied("File not opened for writing".to_string()));
    }

    // Write to content store
    let node_id = handle.value();
    self.content_store.write(node_id, offset, data)
}
```

### Seek Operations

File seeking is implemented through `File::seek`:

```rust
pub fn seek(&self, pos: SeekFrom) -> VfsResult<u64> {
    let new_offset = match pos {
        SeekFrom::Start(offset) => offset,
        SeekFrom::End(offset) => {
            if offset < 0 {
                let offset_abs = offset.unsigned_abs();
                if offset_abs > self.size {
                    return Err(VfsError::InvalidOperation("Seek position beyond file range".to_string()));
                }
                self.size - offset_abs
            } else {
                self.size + offset as u64
            }
        }
        SeekFrom::Current(offset) => {
            let current = self.offset();
            if offset < 0 {
                let offset_abs = offset.unsigned_abs();
                if offset_abs > current {
                    return Err(VfsError::InvalidOperation("Seek position beyond file range".to_string()));
                }
                current - offset_abs
            } else {
                current + offset as u64
            }
        }
    };

    self.set_offset(new_offset);
    Ok(new_offset)
}
```

**Usage Examples**:

```rust
// Open file
let handle = fs.open(Path::new("/data.txt"), OpenFlags::READ_WRITE).await?;

// Seek to file start
let file = fs.open_files.get(&handle).unwrap();
file.seek(SeekFrom::Start(0))?;

// Seek to file end
file.seek(SeekFrom::End(0))?;

// Seek to 100 bytes before file end
file.seek(SeekFrom::End(-100))?;

// Move 50 bytes forward from current position
file.seek(SeekFrom::Current(50))?;
```

---

## Mount Table Architecture

### MountTable

`MountTable` implements plugin mounting and path routing:

```rust
pub struct MountTable {
    mounts: Arc<RwLock<HashMap<String, Arc<dyn EvifPlugin>>>>,
    symlinks: Arc<RwLock<HashMap<String, String>>>,  // Virtual symbolic links
}
```

**Core Operations**:

```rust
impl MountTable {
    pub fn new() -> Self;

    /// Mount plugin
    pub async fn mount(&self, path: String, plugin: Arc<dyn EvifPlugin>) -> EvifResult<()>;

    /// Unmount plugin
    pub async fn unmount(&self, path: &str) -> EvifResult<()>;

    /// Lookup plugin (longest prefix match)
    pub async fn lookup(&self, path: &str) -> Option<Arc<dyn EvifPlugin>>;

    /// List all mount points
    pub async fn list_mounts(&self) -> Vec<String>;

    /// Get mount point count
    pub async fn mount_count(&self) -> usize;
}
```

### Longest Prefix Matching

The mount table uses longest prefix matching to find the most appropriate plugin:

```rust
pub async fn lookup(&self, path: &str) -> Option<Arc<dyn EvifPlugin>> {
    let mounts = self.mounts.read().await;
    let normalized_path = Self::normalize_path(path);

    // Find longest matching prefix
    let mut best_match: Option<(&String, Arc<dyn EvifPlugin>)> = None;

    for (mount_point, plugin) in mounts.iter() {
        if normalized_path.starts_with(mount_point) {
            match &best_match {
                None => {
                    best_match = Some((mount_point, plugin.clone()));
                }
                Some((current_point, _)) => {
                    if mount_point.len() > current_point.len() {
                        best_match = Some((mount_point, plugin.clone()));
                    }
                }
            }
        }
    }

    best_match.map(|(_, plugin)| plugin)
}
```

**Example Scenario**:

```rust
let mount_table = MountTable::new();

// Mount multiple plugins
mount_table.mount("/".to_string(), root_plugin).await?;
mount_table.mount("/local".to_string(), local_plugin).await?;
mount_table.mount("/local/documents".to_string(), docs_plugin).await?;

// Lookup tests
mount_table.lookup("/local/documents/file.txt").await?;
// → Returns docs_plugin (longest match)

mount_table.lookup("/local/images/photo.jpg").await?;
// → Returns local_plugin (matches /local)

mount_table.lookup("/tmp/file.tmp").await?;
// → Returns root_plugin (matches root path)
```

### Virtual Symbolic Links

`MountTable` provides virtual symbolic link support without requiring backend storage:

```rust
impl MountTable {
    /// Create symbolic link (virtual)
    pub async fn symlink(&self, target_path: &str, link_path: &str) -> EvifResult<()> {
        let normalized_target = Self::normalize_path(target_path);
        let normalized_link = Self::normalize_path(link_path);

        let mut symlinks = self.symlinks.write().await;
        symlinks.insert(normalized_link.clone(), normalized_target);
        Ok(())
    }

    /// Read symbolic link target (non-recursive)
    pub async fn readlink(&self, link_path: &str) -> EvifResult<String> {
        let normalized_link = Self::normalize_path(link_path);
        let symlinks = self.symlinks.read().await;
        match symlinks.get(&normalized_link) {
            Some(target) => Ok(target.clone()),
            None => Err(EvifError::NotFound(format!("Symlink: {}", link_path))),
        }
    }

    /// Recursively resolve symbolic link (with cycle detection)
    pub async fn resolve_symlink_recursive(&self, path: &str, max_depth: usize) -> EvifResult<String> {
        let mut current_path = Self::normalize_path(path);
        let mut visited = std::collections::HashSet::new();

        for _ in 0..max_depth {
            // Check cycle
            if visited.contains(&current_path) {
                return Err(EvifError::InvalidInput(format!(
                    "Symbolic link cycle detected: {}",
                    path
                )));
            }
            visited.insert(current_path.clone());

            // Try to resolve
            let (resolved, is_link) = self.resolve_symlink(&current_path).await;

            if !is_link {
                return Ok(current_path);
            }

            current_path = resolved;
        }

        Err(EvifError::InvalidInput(format!(
            "Maximum symbolic link depth exceeded: {}",
            max_depth
        )))
    }
}
```

**Features**:
- No backend required, fully in-memory
- Recursive resolution, max 40 levels
- Cycle detection
- Supports cross-filesystem symbolic links

**Usage Example**:

```rust
let mount_table = MountTable::new();

// Create symbolic links
mount_table.symlink("/local/documents", "/docs").await?;
mount_table.symlink("/local/images", "/pics").await?;

// Read symbolic link
let target = mount_table.readlink("/docs").await?;
assert_eq!(target, "/local/documents");

// Recursive resolution (supports symlinks pointing to symlinks)
mount_table.symlink("/pics", "/photos").await?;
mount_table.symlink("/photos", "/images").await?;
let resolved = mount_table.resolve_symlink_recursive("/images", 10).await?;
assert_eq!(resolved, "/local/images");
```

---

## Implementation Status

### Current Implementation

**✅ Completed**:

- VFS core abstraction definition (FileSystem trait)
- File operations (open, close, read, write, seek)
- Directory operations (create, remove, read)
- Path resolution and normalization
- Symbolic link support
- INode management and caching
- ContentStore content storage
- FileHandle handle management

**⚠️ Partially Implemented**:

- PathResolver integration with graph structure (`find_child` method returns error)
- Full DEntry and VNode implementation
- Integration with storage backend (evif-storage)

**❌ Not Integrated**:

- VFS layer integration with main path (Plugin + REST)
- FUSE usage of VFS layer (current FUSE directly uses plugins)
- REST API exposure of VFS

### Current Main Path

**Architecture in Use**:

```
REST API → EvifPlugin → MountTable → Storage Plugins
```

**VFS Layer Position**:

```
VFS Abstraction (defined but unused)
    ↓
EvifPlugin Trait (in use)
    ↓
MountTable + Plugins (running)
```

### Future Plans

1. **VFS and Plugin System Integration**: Establish adaptation layer between plugins and VFS
2. **Graph Query Support**: Complete PathResolver integration with graph structure
3. **Unified Storage Layer**: Provide unified storage abstraction via evif-storage
4. **FUSE VFS Support**: Enable FUSE to use VFS abstraction layer

### Usage Recommendations

**Currently Recommended**:
- Continue using `EvifPlugin` trait for new plugin development
- Use `MountTable` for plugin mount management
- VFS abstraction layer reserved for future use

**Not Recommended**:
- Direct use of VFS layer (not fully integrated)
- Depend on PathResolver graph query functionality

---

## Related Chapters

- **[Chapter 5: Plugin Development](chapter-5-plugin-development.md)** - EvifPlugin trait detailed usage guide
- **[Chapter 6: FUSE Integration](chapter-6-fuse.md)** - FUSE mounting and filesystem access
- **[Chapter 3: Architecture](chapter-3-architecture.md)** - Overall system architecture and dual technology stack

---

**Document Version**: 1.0
**Last Updated**: 2026-03-01
**Author**: EVIF Team
