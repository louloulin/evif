# Chapter 6: FUSE Integration (FUSE集成)

## Table of Contents (目录)

1. [FUSE Overview (FUSE概述)](#fuse-overview-fuse概述)
2. [Implementation Status (实现状态)](#implementation-status-实现状态)
3. [Architecture (架构设计)](#architecture-架构设计)
4. [Core Components (核心组件)](#core-components-核心组件)
5. [FUSE Operations Mapping (FUSE操作映射)](#fuse-operations-mapping-fuse操作映射)
6. [Mount Configuration (挂载配置)](#mount-configuration-挂载配置)
7. [Usage Examples (使用示例)](#usage-examples-使用示例)
8. [Performance Optimization (性能优化)](#performance-optimization-性能优化)
9. [Platform Support (平台支持)](#platform-support-平台支持)
10. [Limitations and Future Work (限制与未来工作)](#limitations-and-future-work-限制与未来工作)

---

## FUSE Overview (FUSE概述)

### What is FUSE? (什么是FUSE?)

**FUSE** (Filesystem in Userspace) is a software interface that enables non-privileged users to create their own file systems without editing kernel code. FUSE 是一种软件接口，允许非特权用户创建自己的文件系统，而无需编辑内核代码。

**Key Features:**
- **Userspace Implementation**: Filesystem logic runs in user space, not kernel space (用户空间实现：文件系统逻辑运行在用户空间而非内核空间)
- **Safety**: Crashes don't compromise system stability (安全性：崩溃不会影响系统稳定性)
- **Portability**: Write once, run on Linux/macOS/FreeBSD (可移植性：一次编写，可在 Linux/macOS/FreeBSD 上运行)
- **Flexibility**: Easy to develop and debug (灵活性：易于开发和调试)

### EVIF's FUSE Integration (EVIF的FUSE集成)

EVIF provides complete FUSE support through the `evif-fuse` crate, enabling EVIF virtual filesystems to be mounted as local filesystems on Linux, macOS, and FreeBSD.
EVIF 通过 `evif-fuse` crate 提供完整的 FUSE 支持，使 EVIF 虚拟文件系统可以作为本地文件系统挂载到 Linux、macOS 和 FreeBSD 上。

**Benefits:**
- **Native File Access**: Access EVIF storage using standard file tools (ls, cp, vim, etc.) (原生文件访问：使用标准文件工具访问 EVIF 存储)
- **Application Integration**: Any application can work with EVIF without modification (应用集成：任何应用程序无需修改即可使用 EVIF)
- **Transparent Operation**: FUSE operations translate to EVIF plugin calls (透明操作：FUSE 操作转换为 EVIF 插件调用)

---

## Implementation Status (实现状态)

### Current Status (当前状态)

**✅ Fully Implemented (完全实现):**

- **Core FUSE Operations**: Complete POSIX filesystem semantics (核心 FUSE 操作：完整的 POSIX 文件系统语义)
- **File Operations**: read, write, create, delete, truncate (文件操作：读、写、创建、删除、截断)
- **Directory Operations**: readdir, mkdir, rmdir (目录操作：读取目录、创建目录、删除目录)
- **Attribute Operations**: getattr, setattr (属性操作：获取属性、设置属性)
- **File Handle Management**: open, release (文件句柄管理：打开、释放)
- **Rename Operations**: rename (重命名操作)

**✅ Performance Features (性能特性):**

- **Inode Management**: Path ↔ inode bidirectional mapping (Inode 管理：路径 ↔ inode 双向映射)
- **Directory Caching**: LRU cache with TTL (目录缓存：LRU 缓存 + TTL 超时)
- **Async Runtime**: Tokio-based async operations (异步运行时：基于 Tokio 的异步操作)

**✅ Mount Options (挂载选项):**

- **Read-Only Mount**: Data protection (只读挂载：数据保护)
- **Read-Write Mount**: Full modification support (读写挂载：完全修改支持)
- **Allow Other**: Multi-user access (允许其他用户：多用户访问)
- **Configurable Cache**: Adjustable cache size and timeout (可配置缓存：可调缓存大小和超时)

**Platform Support (平台支持):**

| Platform (平台) | Status (状态) | Notes (说明) |
|-----------------|---------------|--------------|
| Linux | ✅ Fully Supported | Native FUSE support |
| macOS | ✅ Fully Supported | FUSE for macOS (osxfuse) |
| FreeBSD | ✅ Fully Supported | FUSE for FreeBSD |

---

## Architecture (架构设计)

### Architecture Diagram (架构图)

```
┌─────────────────────────────────────────────────────────────┐
│                     User Space (用户空间)                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐      ┌──────────────┐                    │
│  │   ls / cp    │      │  Applications│                    │
│  │   vim / cat  │      │   (应用层)     │                    │
│  └──────┬───────┘      └──────┬───────┘                    │
│         │                      │                            │
│         │  System Calls        │                            │
│         ▼                      ▼                            │
│  ┌──────────────────────────────────────────────┐           │
│  │            Kernel (VFS Layer)                │           │
│  │  (fuser crate translates to FUSE protocol)   │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│         ┌───────────┴───────────┐                            │
│         │   /dev/fuse device    │                            │
│         └───────────┬───────────┘                            │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │          EvifFuse (evif-fuse crate)           │           │
│  │  ┌────────────────────────────────────────┐  │           │
│  │  │  FUSE Operations (Filesystem trait)    │  │           │
│  │  │  - getattr, readdir, read, write, etc. │  │           │
│  │  └───────────────┬────────────────────────┘  │           │
│  │                  │                            │           │
│  │  ┌───────────────▼────────────────────────┐  │           │
│  │  │      InodeManager (inode 管理)         │  │           │
│  │  │  - Path ↔ Inode mapping               │  │           │
│  │  │  - Inode allocation & recycling       │  │           │
│  │  └───────────────┬────────────────────────┘  │           │
│  │                  │                            │           │
│  │  ┌───────────────▼────────────────────────┐  │           │
│  │  │      DirCache (目录缓存)                │  │           │
│  │  │  - LRU cache with TTL                  │  │           │
│  │  │  - Cache invalidation                  │  │           │
│  │  └───────────────┬────────────────────────┘  │           │
│  └──────────────────┼───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │         RadixMountTable (挂载表)              │           │
│  │    - Longest prefix matching                  │           │
│  │    - Plugin lookup and routing                │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │       EVIF Plugins (存储插件层)               │           │
│  │  - MemFS, LocalFS, KVFS, S3FS, etc.          │           │
│  │  - Cloud storage (S3, Azure, GCS, etc.)      │           │
│  │  - Special filesystems (HTTP, Encrypted, etc.)│           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │         Storage Backend (存储后端)            │           │
│  │  - Memory, Disk, Cloud Storage, etc.         │           │
│  └──────────────────────────────────────────────┘           │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow (数据流)

**Read Operation Example (读取操作示例):**

```
1. User command: `cat /mnt/evif/file.txt`
   (用户命令：`cat /mnt/evif/file.txt`)

2. Kernel VFS → FUSE → EvifFuse::read()
   (内核 VFS → FUSE → EvifFuse::read())

3. EvifFuse::read()
   - Resolve inode to path: "/file.txt"
     (解析 inode 到路径："/file.txt")
   - Lookup plugin via MountTable
     (通过 MountTable 查找插件)
   - Call plugin.read("/file.txt", offset, size)
     (调用插件读取)
   - Return data to kernel
     (返回数据到内核)

4. Kernel → User space: cat displays content
   (内核 → 用户空间：cat 显示内容)
```

**Write Operation Example (写入操作示例):**

```
1. User command: `echo "hello" > /mnt/evif/file.txt`
   (用户命令：`echo "hello" > /mnt/evif/file.txt`)

2. Kernel VFS → FUSE → EvifFuse::write()
   (内核 VFS → FUSE → EvifFuse::write())

3. EvifFuse::write()
   - Check allow_write flag
     (检查 allow_write 标志)
   - Resolve inode to path: "/file.txt"
     (解析 inode 到路径："/file.txt")
   - Invalidate directory cache
     (使目录缓存失效)
   - Lookup plugin via MountTable
     (通过 MountTable 查找插件)
   - Call plugin.write("/file.txt", data, offset, flags)
     (调用插件写入)
   - Return bytes written
     (返回写入字节数)

4. Kernel acknowledges write completion
   (内核确认写入完成)
```

---

## Core Components (核心组件)

### 1. EvifFuse (FUSE文件系统实现)

**Location**: `crates/evif-fuse/src/lib.rs`

**Purpose**: Implements the `fuser::Filesystem` trait to provide complete FUSE functionality.

**Key Features (主要特性):**

```rust
pub struct EvifFuseFuse {
    /// Tokio runtime for async operations
    /// Tokio 运行时，用于异步操作
    runtime: Arc<Runtime>,

    /// Mount table for plugin lookup
    /// 挂载表，用于插件查找
    mount_table: Arc<RadixMountTable>,

    /// Inode manager for path ↔ inode mapping
    /// Inode 管理器，用于路径 ↔ inode 映射
    inode_manager: Arc<InodeManager>,

    /// Directory cache for performance
    /// 目录缓存，用于性能优化
    dir_cache: Arc<DirCache>,

    /// Root path in EVIF namespace
    /// EVIF 命名空间中的根路径
    root_path: PathBuf,

    /// Allow write operations
    /// 允许写操作
    pub allow_write: bool,

    /// Cache timeout in seconds
    /// 缓存超时时间（秒）
    cache_timeout: u64,

    /// File handle mapping
    /// 文件句柄映射
    file_handles: Arc<RwLock<HashMap<u64, u64>>>,
}
```

**Implemented FUSE Operations (实现的 FUSE 操作):**

| Operation | Description | Method |
|-----------|-------------|--------|
| `getattr` | Get file attributes | Get file metadata (size, mode, timestamps) |
| `setattr` | Set file attributes | Modify file properties (size, mode, owner) |
| `readdir` | Read directory | List directory contents |
| `open` | Open file | Check permissions and allocate handle |
| `read` | Read file | Read file data at offset |
| `write` | Write file | Write data at offset |
| `create` | Create file | Create new file |
| `unlink` | Delete file | Remove file |
| `mkdir` | Create directory | Create new directory |
| `rmdir` | Remove directory | Remove directory |
| `rename` | Rename | Move file/directory |
| `fsync` | Sync file | Flush file data |
| `fsyncdir` | Sync directory | Flush directory data |
| `release` | Release file | Close file handle |
| `releasedir` | Release directory | Close directory handle |
| `statfs` | Filesystem stats | Get filesystem statistics |

---

### 2. InodeManager (Inode管理器)

**Location**: `crates/evif-fuse/src/inode_manager.rs`

**Purpose**: Manages bidirectional mapping between file paths and inodes.

**Key Features (主要特性):**

- **Bidirectional Mapping**: Path ↔ Inode (双向映射：路径 ↔ Inode)
- **Inode Allocation**: Automatic inode assignment (Inode 分配：自动 inode 分配)
- **Reference Counting**: Track inode usage (引用计数：跟踪 inode 使用)
- **Inode Recycling**: Cleanup deleted files (Inode 回收：清理已删除文件)

**Data Structures (数据结构):**

```rust
/// Inode information (inode 信息)
pub struct InodeInfo {
    pub inode: Inode,        // Inode number (inode 编号)
    pub path: String,         // File path (文件路径)
    pub is_dir: bool,         // Is directory (是否是目录)
    pub ref_count: u32,       // Reference count (引用计数)
}

/// Inode Manager (inode 管理器)
pub struct InodeManager {
    next_inode: Arc<Mutex<Inode>>,              // Next inode number (下一个 inode)
    path_to_inode: Arc<RwLock<HashMap<String, Inode>>>,   // Path → inode
    inode_to_info: Arc<RwLock<HashMap<Inode, InodeInfo>>>, // inode → info
}
```

**Usage Examples (使用示例):**

```rust
// Create inode manager (创建 inode 管理器)
let manager = InodeManager::new(10000);  // cache_size: 10000

// Get or create inode (获取或创建 inode)
let inode = manager.get_or_create("/path/to/file.txt");

// Get path from inode (从 inode 获取路径)
let path = manager.get_path(inode);  // Some("/path/to/file.txt")

// Get inode from path (从路径获取 inode)
let inode = manager.get_inode("/path/to/file.txt");  // Some(inode)

// Reference counting (引用计数)
manager.incref(inode);  // Increase ref count (增加引用计数)
let is_zero = manager.decref(inode);  // Decrease ref count (减少引用计数)

// Recycle inode (回收 inode)
manager.recycle(inode);  // Remove from mappings (从映射中删除)
```

**Special Inodes (特殊 Inode):**

```rust
pub const ROOT_INODE: Inode = 1;    // Root directory (/)
pub const PARENT_INODE: Inode = 2;   // Parent directory (..)
```

---

### 3. DirCache (目录缓存)

**Location**: `crates/evif-fuse/src/dir_cache.rs`

**Purpose**: Caches directory contents to improve `readdir` performance.

**Key Features (主要特性):**

- **LRU Eviction**: Least Recently Used strategy (LRU 淘汰：最近最少使用策略)
- **TTL Expiration**: Time-based cache invalidation (TTL 过期：基于时间的缓存失效)
- **Automatic Cleanup**: Evict expired entries (自动清理：淘汰过期条目)

**Data Structures (数据结构):**

```rust
/// Directory entry (目录条目)
pub struct DirEntry {
    pub inode: u64,        // Inode number (inode 编号)
    pub name: String,      // File name (文件名)
    pub is_dir: bool,      // Is directory (是否是目录)
}

/// Directory cache (目录缓存)
pub struct DirCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,  // Cache data (缓存数据)
    lru_head: Arc<RwLock<Option<String>>>,            // LRU list head (LRU 链表头)
    lru_tail: Arc<RwLock<Option<String>>>,            // LRU list tail (LRU 链表尾)
    lru_nodes: Arc<RwLock<HashMap<String, LruNode>>>, // LRU nodes (LRU 节点)
    max_entries: usize,       // Maximum cache entries (最大缓存条目数)
    ttl: Duration,            // Time-to-live (存活时间)
    current_size: Arc<RwLock<usize>>,  // Current size (当前大小)
}
```

**Usage Examples (使用示例):**

```rust
// Create directory cache with 60 second TTL (创建目录缓存，TTL 为 60 秒)
let cache = DirCache::new(60);

// Cache directory entries (缓存目录条目)
cache.put("/dir/".to_string(), vec![
    DirEntry::new(10, "file1.txt".to_string(), false),
    DirEntry::new(11, "file2.txt".to_string(), false),
]);

// Get cached entries (获取缓存条目)
if let Some(entries) = cache.get("/dir/") {
    for entry in entries {
        println!("{}: {}", entry.inode, entry.name);
    }
}

// Invalidate cache (使缓存失效)
cache.invalidate("/dir/");

// Clean up expired entries (清理过期条目)
let expired_count = cache.cleanup_expired();

// Get cache statistics (获取缓存统计信息)
let (current, max, ttl) = cache.stats();
```

---

### 4. Mount Configuration (挂载配置)

**Location**: `crates/evif-fuse/src/mount_config.rs`

**Purpose**: Manages FUSE mount configuration options.

**Configuration Structure (配置结构):**

```rust
pub struct FuseMountConfig {
    /// Mount point path (挂载点路径)
    pub mount_point: PathBuf,

    /// Root path in EVIF (EVIF 中的根路径)
    pub root_path: PathBuf,

    /// Allow write operations (允许写操作)
    pub allow_write: bool,

    /// Allow other users to access (允许其他用户访问)
    pub allow_other: bool,

    /// Cache size (inode cache entries) (缓存大小)
    pub cache_size: usize,

    /// Cache timeout in seconds (缓存超时秒数)
    pub cache_timeout: u64,
}
```

**Mount Options (挂载选项):**

```rust
pub enum MountOptions {
    ReadOnly,       // Read-only mount (只读挂载)
    ReadWrite,      // Read-write mount (读写挂载)
    AllowOther,     // Allow other users (允许其他用户)
    AllowMap,       // Allow mapping (允许映射)
    AllowLock,      // Allow locking (允许锁定)
    AllowExec,      // Allow execution (允许执行)
    NoCache,        // Disable FUSE cache (禁用 FUSE 缓存)
    NoAttrCache,    // Disable attribute cache (禁用属性缓存)
    NoDirCache,     // Disable directory cache (禁用目录缓存)
    KernelCache,    // Enable kernel cache (启用内核缓存)
    AsyncRead,      // Async read operations (异步读取)
    WritebackCache, // Writeback cache (写回缓存)
}
```

---

## FUSE Operations Mapping (FUSE操作映射)

### POSIX Operations → EVIF Plugin Calls

**File Operations (文件操作):**

| POSIX Operation | FUSE Method | EVIF Plugin Method | Description |
|----------------|-------------|-------------------|-------------|
| `stat()` | `getattr` | `stat()` | Get file attributes (获取文件属性) |
| `open()` | `open` | Check existence | Check file exists (检查文件存在) |
| `read()` | `read` | `read()` | Read file data (读取文件数据) |
| `write()` | `write` | `write()` | Write file data (写入文件数据) |
| `create()` | `create` | `create()` | Create new file (创建新文件) |
| `unlink()` | `unlink` | `remove()` | Delete file (删除文件) |
| `truncate()` | `setattr` | `read()` + `write()` | Resize file (调整文件大小) |
| `chmod()` | `setattr` | (No-op) | Change permissions (修改权限) |
| `chown()` | `setattr` | (No-op) | Change owner (修改所有者) |
| `fsync()` | `fsync` | (Auto-persist) | Sync to disk (同步到磁盘) |
| `close()` | `release` | (Handle cleanup) | Close file (关闭文件) |

**Directory Operations (目录操作):**

| POSIX Operation | FUSE Method | EVIF Plugin Method | Description |
|----------------|-------------|-------------------|-------------|
| `opendir()` | `open` | Check exists | Check directory exists (检查目录存在) |
| `readdir()` | `readdir` | `readdir()` | List directory contents (列出目录内容) |
| `mkdir()` | `mkdir` | `mkdir()` | Create directory (创建目录) |
| `rmdir()` | `rmdir` | `remove()` | Remove directory (删除目录) |
| `rename()` | `rename` | `rename()` | Move file/directory (移动文件/目录) |
| `closedir()` | `releasedir` | (Handle cleanup) | Close directory (关闭目录) |

**Attribute Operations (属性操作):**

| POSIX Operation | FUSE Method | EVIF Plugin Method | Description |
|----------------|-------------|-------------------|-------------|
| `statfs()` | `statfs` | (Static values) | Filesystem stats (文件系统统计) |
| `utimens()` | `setattr` | (No-op) | Change timestamps (修改时间戳) |

---

## Mount Configuration (挂载配置)

### Mount Configuration Builder (挂载配置构建器)

**Basic Usage (基本用法):**

```rust
use evif_fuse::{FuseMountBuilder, FuseMountConfig};
use evif_core::RadixMountTable;
use std::path::Path;
use std::sync::Arc;

// Create mount table (创建挂载表)
let mount_table = Arc::new(RadixMountTable::new());

// Build configuration using builder (使用构建器配置)
let config = FuseMountBuilder::new()
    .mount_point(Path::new("/mnt/evif"))      // Required (必需)
    .root_path(Path::new("/"))                 // EVIF root path (EVIF 根路径)
    .allow_write(false)                        // Read-only (只读)
    .allow_other(false)                        // Owner only (仅所有者)
    .cache_size(10000)                         // Inode cache size (inode 缓存大小)
    .cache_timeout(60)                         // Cache TTL in seconds (缓存 TTL 秒数)
    .build()?;
```

**Predefined Configurations (预定义配置):**

```rust
// Read-only mount (只读挂载)
let config = FuseMountConfig::readonly(
    PathBuf::from("/mnt/evif")
);

// Read-write mount (读写挂载)
let config = FuseMountConfig::readwrite(
    PathBuf::from("/mnt/evif")
);

// Default mount (默认挂载)
let config = FuseMountConfig::default();
// Uses: mount_point=/mnt/evif, readonly, cache_size=10000, ttl=60s
```

### Mount Options (挂载选项)

**Common Options (常用选项):**

| Option | Description | Example |
|--------|-------------|---------|
| `ro` | Read-only mount (只读挂载) | `--readonly` |
| `rw` | Read-write mount (读写挂载) | `--readwrite` |
| `allow_other` | Allow other users (允许其他用户) | `--allow-other` |
| `no_cache` | Disable FUSE cache (禁用 FUSE 缓存) | (Programmatic) |
| `async_read` | Async read operations (异步读取) | (Programmatic) |

**Security Considerations (安全考虑):**

- **Read-Only by Default**: Default mount is read-only for safety (默认只读：安全考虑)
- **Allow Other**: Requires user_allow_other in `/etc/fuse.conf` (允许其他用户：需要 `/etc/fuse.conf` 配置)
- **File Permissions**: Plugin-dependent (file ownership not enforced) (文件权限：依赖插件，不强制执行文件所有权)

---

## Usage Examples (使用示例)

### Example 1: Read-Only Mount (只读挂载)

```rust
use evif_core::RadixMountTable;
use evif_fuse::{mount_evif, FuseMountConfig};
use evif_plugins::MemFs;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create mount table (创建挂载表)
    let mount_table = Arc::new(RadixMountTable::new());

    // Mount MemFS plugin (挂载 MemFS 插件)
    let memfs = Arc::new(MemFs::new()?);
    mount_table.mount("/mem", memfs).await?;

    // Configure FUSE mount (配置 FUSE 挂载)
    let config = FuseMountConfig {
        mount_point: PathBuf::from("/mnt/evif"),
        root_path: PathBuf::from("/"),
        allow_write: false,  // Read-only (只读)
        allow_other: false,
        cache_size: 10000,
        cache_timeout: 60,
    };

    // Mount filesystem (blocking) (挂载文件系统，阻塞)
    mount_evif(mount_table, Path::new("/mnt/evif"), config)?;

    Ok(())
}
```

### Example 2: Read-Write Mount (读写挂载)

```rust
use evif_core::RadixMountTable;
use evif_fuse::{mount_evif_background, FuseMountBuilder};
use evif_plugins::LocalFs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mount_table = Arc::new(RadixMountTable::new());

    // Mount local filesystem (挂载本地文件系统)
    let localfs = Arc::new(LocalFs::new("/data/storage")?);
    mount_table.mount("/local", localfs).await?;

    // Build read-write configuration (构建读写配置)
    let config = FuseMountBuilder::new()
        .mount_point(Path::new("/mnt/evif"))
        .root_path(Path::new("/"))
        .allow_write(true)    // Enable writes (启用写入)
        .allow_other(false)
        .cache_size(10000)
        .cache_timeout(60)
        .build()?;

    // Mount in background (后台挂载)
    let _session = mount_evif_background(
        mount_table,
        Path::new("/mnt/evif"),
        config
    )?;

    println!("EVIF mounted at /mnt/evif (read-write)");
    println!("Press Ctrl+C to unmount");

    // Wait for Ctrl+C (等待 Ctrl+C)
    tokio::signal::ctrl_c().await?;
    println!("Unmounting...");

    Ok(())
}
```

### Example 3: Using the Command-Line Tool (使用命令行工具)

**Build the tool (构建工具):**

```bash
cargo build --bin evif-fuse-mount --release
```

**Basic Usage (基本用法):**

```bash
# Read-only mount (只读挂载)
sudo ./target/release/evif-fuse-mount /mnt/evif --readonly

# Read-write mount (读写挂载)
sudo ./target/release/evif-fuse-mount /mnt/evif --readwrite

# Allow other users (允许其他用户)
sudo ./target/release/evif-fuse-mount /mnt/evif --readwrite --allow-other

# Custom cache settings (自定义缓存设置)
sudo ./target/release/evif-fuse-mount /mnt/evif \
    --readonly \
    --cache-size 5000 \
    --cache-timeout 120
```

**Unmounting (卸载):**

```bash
# Method 1: Ctrl+C (if running in foreground) (方法 1：Ctrl+C)
# Press Ctrl+C in the terminal running evif-fuse-mount

# Method 2: fusermount (方法 2：fusermount)
fusermount -u /mnt/evif

# Method 3: umount (方法 3：umount)
umount /mnt/evif
```

### Example 4: Accessing Mounted Filesystem (访问挂载的文件系统)

**After mounting at `/mnt/evif` (挂载到 `/mnt/evif` 后):**

```bash
# List files (列出文件)
ls -la /mnt/evif/

# Read file (读取文件)
cat /mnt/evif/mem/test.txt

# Write file (if mounted read-write) (写入文件，如果是读写挂载)
echo "Hello, EVIF!" > /mnt/evif/local/newfile.txt

# Copy files (复制文件)
cp /tmp/file.txt /mnt/evif/local/

# Create directory (创建目录)
mkdir /mnt/evif/local/newdir

# Delete file (删除文件)
rm /mnt/evif/local/oldfile.txt

# Use with any application (与任何应用一起使用)
vim /mnt/evif/mem/config.yaml
rsync -av /data/ /mnt/evif/local/backup/
```

---

## Performance Optimization (性能优化)

### 1. Inode Caching (Inode 缓存)

**Purpose**: Reduce path → inode lookup overhead (目的：减少路径 → inode 查找开销)

**Configuration (配置):**

```rust
let config = FuseMountBuilder::new()
    .cache_size(10000)  // Number of inode entries (inode 条目数)
    .build()?;
```

**Trade-offs (权衡):**
- **Larger Cache**: Better performance, more memory (更大缓存：更好性能，更多内存)
- **Smaller Cache**: Lower memory usage, more lookups (更小缓存：更低内存使用，更多查找)

**Recommended Values (推荐值):**
- **Small filesystems (<1000 files)**: 1000-5000 (小文件系统)
- **Medium filesystems (1000-10000 files)**: 10000-50000 (中文件系统)
- **Large filesystems (>10000 files)**: 50000+ (大文件系统)

---

### 2. Directory Caching (目录缓存)

**Purpose**: Cache directory listings to reduce `readdir` calls (目的：缓存目录列表以减少 `readdir` 调用)

**Features (特性):**
- **LRU Eviction**: Automatically evict least recently used entries (LRU 淘汰：自动淘汰最近最少使用的条目)
- **TTL Expiration**: Time-based invalidation (TTL 过期：基于时间的失效)
- **Automatic Invalidation**: On write operations (自动失效：写入操作时)

**Configuration (配置):**

```rust
let config = FuseMountBuilder::new()
    .cache_timeout(60)  // TTL in seconds (TTL 秒数)
    .build()?;
```

**Recommended Values (推荐值):**
- **Static content**: 300-600 seconds (5-10 minutes) (静态内容)
- **Dynamic content**: 30-60 seconds (动态内容)
- **Real-time updates**: 5-10 seconds (实时更新)

**Manual Cache Control (手动缓存控制):**

```rust
// Invalidate specific directory (使特定目录失效)
fuse.dir_cache.invalidate("/path/to/dir");

// Clean up expired entries (清理过期条目)
let expired_count = fuse.dir_cache.cleanup_expired();

// Clear all cache (清空所有缓存)
fuse.dir_cache.clear();
```

---

### 3. Async Runtime Optimization (异步运行时优化)

**Tokio Multi-threaded Runtime**: All FUSE operations use async runtime for efficient I/O (Tokio 多线程运行时：所有 FUSE 操作使用异步运行时以实现高效 I/O)

**Configuration (配置):**

```rust
let runtime = Arc::new(
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)  // Number of worker threads (工作线程数)
        .build()?
);
```

**Thread Pool Sizing (线程池大小):**
- **CPU-bound operations**: Number of CPU cores (CPU 密集型操作)
- **I/O-bound operations**: 2-4x CPU cores (I/O 密集型操作)

---

### 4. Mount Options Tuning (挂载选项调优)

**Kernel Cache Options (内核缓存选项):**

```rust
// Enable kernel cache for better read performance (启用内核缓存以提升读性能)
let options = vec![
    fuser::MountOption::RO,
    fuser::MountOption::FSName("evif".to_string()),
    fuser::MountOption::KernelCache,  // Enable kernel cache (启用内核缓存)
];
```

**Async Read (异步读取):**

```rust
// Enable async read for better concurrency (启用异步读取以提升并发性)
let options = vec![
    fuser::MountOption::AsyncRead,
];
```

---

## Platform Support (平台支持)

### Linux (Linux)

**Status**: ✅ Fully supported (完全支持)

**Requirements (要求):**
- `libfuse-dev` package (libfuse-dev 包)
- FUSE kernel module (usually built-in) (FUSE 内核模块，通常内置)

**Installation (安装):**

```bash
# Ubuntu/Debian (Ubuntu/Debian)
sudo apt-get install libfuse-dev fuse

# Fedora/RHEL (Fedora/RHEL)
sudo dnf install fuse-devel

# Arch Linux (Arch Linux)
sudo pacman -S fuse3
```

**Usage (使用):**

```bash
# Mount (挂载)
evif-fuse-mount /mnt/evif --readonly

# Unmount (卸载)
fusermount -u /mnt/evif
```

---

### macOS (macOS)

**Status**: ✅ Fully supported (完全支持)

**Requirements (要求):**
- FUSE for macOS (macFUSE) or osxfuse

**Installation (安装):**

```bash
# Install macFUSE (安装 macFUSE)
brew install --cask macfuse

# Or install osxfuse (或安装 osxfuse)
brew install osxfuse
```

**Usage (使用):**

```bash
# Mount (挂载)
sudo evif-fuse-mount /mnt/evif --readonly

# Unmount (卸载)
umount /mnt/evif
```

**Notes (注意事项):**
- Requires `sudo` for mount operations (挂载操作需要 `sudo`)
- macFUSE may require system extension approval (macFUSE 可能需要系统扩展批准)

---

### FreeBSD (FreeBSD)

**Status**: ✅ Fully supported (完全支持)

**Requirements (要求):**
- `fusefs-kmod` kernel module (fusefs-kmod 内核模块)
- `libfuse` library

**Installation (安装):**

```bash
# Load kernel module (加载内核模块)
kldload fusefs

# Install library (安装库)
pkg install fusefs-libs
```

**Usage (使用):**

```bash
# Mount (挂载)
evif-fuse-mount /mnt/evif --readonly

# Unmount (卸载)
umount /mnt/evif
```

---

## Limitations and Future Work (限制与未来工作)

### Current Limitations (当前限制)

**❌ Not Implemented (未实现):**

1. **Extended Attributes (扩展属性):**
   - `listxattr`, `getxattr`, `setxattr` return errors or no-op
   - Extended attributes not supported by EVIF plugins

2. **File Locking (文件锁定):**
   - No `flock` or `lock` support
   - No advisory record locking

3. **Hard Links (硬链接):**
   - No `link()` operation support
   - EVIF doesn't support hard links across plugins

4. **Symbolic Links (符号链接):**
   - Read-only symlink support (through VFS layer)
   - Cannot create new symlinks via FUSE

5. **Permission Enforcement (权限强制):**
   - No UID/GID enforcement
   - File mode bits are informational only

6. **Access Time Tracking (访问时间跟踪):**
   - No `atime` updates (performance optimization)
   - Only `mtime` and `ctime` are tracked

---

### Platform-Specific Limitations (平台特定限制)

**Linux (Linux):**
- Requires `user_allow_other` in `/etc/fuse.conf` for `allow_other` option

**macOS (macOS):**
- Requires root privileges for mount operations
- macOS security policies may block FUSE

**FreeBSD (FreeBSD):**
- Manual kernel module loading required (`kldload fusefs`)

---

### Future Enhancements (未来增强)

**Planned Features (计划中的特性):**

1. **Write Performance (写入性能):**
   - Writeback cache support (writeback caching)
   - Batch write operations

2. **Advanced POSIX Features (高级 POSIX 特性):**
   - Extended attributes support
   - File locking (flock, lock)
   - Symbolic link creation (symlink)

3. **Performance Monitoring (性能监控):**
   - Operation latency tracking
   - Cache hit/miss statistics
   - Throughput metrics

4. **Dynamic Configuration (动态配置):**
   - Runtime cache size adjustment
   - Hot-reload mount options
   - Live statistics reporting

5. **Cloud-Native Optimizations (云原生优化):**
   - Prefetching for cloud storage
   - Adaptive cache sizing
   - Bandwidth-aware operations

---

## Summary (总结)

**Key Takeaways (关键要点):**

1. **Complete FUSE Support**: EVIF provides full POSIX filesystem semantics via FUSE (完整 FUSE 支持：EVIF 通过 FUSE 提供完整的 POSIX 文件系统语义)

2. **Cross-Platform**: Works on Linux, macOS, and FreeBSD (跨平台：在 Linux、macOS 和 FreeBSD 上运行)

3. **Performance Optimized**: Inode caching, directory caching, and async runtime (性能优化：inode 缓存、目录缓存和异步运行时)

4. **Flexible Mounting**: Read-only, read-write, and multi-user modes (灵活挂载：只读、读写和多用户模式)

5. **Plugin Integration**: Transparent integration with all EVIF plugins (插件集成：与所有 EVIF 插件透明集成)

6. **Production Ready**: Stable implementation with comprehensive testing (生产就绪：稳定实现，经过全面测试)

**For more information (更多信息):**
- **Chapter 3**: [Architecture](chapter-3-architecture.md) - Overall system architecture (整体系统架构)
- **Chapter 4**: [Virtual Filesystem](chapter-4-virtual-filesystem.md) - VFS layer design (VFS 层设计)
- **Chapter 5**: [Plugin Development](chapter-5-plugin-development.md) - Plugin interface and implementation (插件接口和实现)
- **GitHub Repository**: [EVIF Source Code](https://github.com/your-org/evif) (EVIF 源代码)
