# 第六章：FUSE 集成 (Chapter 6: FUSE Integration)

## 目录 (Table of Contents)

1. [FUSE 概述 (FUSE Overview)](#fuse-概述-fuse-overview)
2. [实现状态 (Implementation Status)](#实现状态-implementation-status)
3. [架构设计 (Architecture)](#架构设计-architecture)
4. [核心组件 (Core Components)](#核心组件-core-components)
5. [FUSE 操作映射 (FUSE Operations Mapping)](#fuse-操作映射-fuse-operations-mapping)
6. [挂载配置 (Mount Configuration)](#挂载配置-mount-configuration)
7. [使用示例 (Usage Examples)](#使用示例-usage-examples)
8. [性能优化 (Performance Optimization)](#性能优化-performance-optimization)
9. [平台支持 (Platform Support)](#平台支持-platform-support)
10. [限制与未来工作 (Limitations and Future Work)](#限制与未来工作-limitations-and-future-work)

---

## FUSE 概述 (FUSE Overview)

### 什么是 FUSE？(What is FUSE?)

**FUSE** (Filesystem in Userspace，用户空间文件系统) 是一种软件接口，允许非特权用户创建自己的文件系统，而无需编辑内核代码。

**主要特性：**
- **用户空间实现**：文件系统逻辑运行在用户空间而非内核空间
- **安全性**：崩溃不会影响系统稳定性
- **可移植性**：一次编写，可在 Linux/macOS/FreeBSD 上运行
- **灵活性**：易于开发和调试

### EVIF 的 FUSE 集成 (EVIF's FUSE Integration)

EVIF 通过 `evif-fuse` crate 提供完整的 FUSE 支持，使 EVIF 虚拟文件系统可以作为本地文件系统挂载到 Linux、macOS 和 FreeBSD 上。

**优势：**
- **原生文件访问**：使用标准文件工具（ls、cp、vim 等）访问 EVIF 存储
- **应用集成**：任何应用程序无需修改即可使用 EVIF
- **透明操作**：FUSE 操作转换为 EVIF 插件调用

---

## 实现状态 (Implementation Status)

### 当前状态 (Current Status)

**✅ 完全实现 (Fully Implemented):**

- **核心 FUSE 操作**：完整的 POSIX 文件系统语义
- **文件操作**：read、write、create、delete、truncate
- **目录操作**：readdir、mkdir、rmdir
- **属性操作**：getattr、setattr
- **文件句柄管理**：open、release
- **重命名操作**：rename

**✅ 性能特性 (Performance Features):**

- **Inode 管理**：路径 ↔ inode 双向映射
- **目录缓存**：LRU 缓存 + TTL 超时
- **异步运行时**：基于 Tokio 的异步操作

**✅ 挂载选项 (Mount Options):**

- **只读挂载**：数据保护
- **读写挂载**：完全修改支持
- **允许其他用户**：多用户访问
- **可配置缓存**：可调缓存大小和超时

**平台支持 (Platform Support):**

| 平台 (Platform) | 状态 (Status) | 说明 (Notes) |
|-----------------|---------------|--------------|
| Linux | ✅ 完全支持 | 原生 FUSE 支持 |
| macOS | ✅ 完全支持 | FUSE for macOS (osxfuse) |
| FreeBSD | ✅ 完全支持 | FUSE for FreeBSD |

---

## 架构设计 (Architecture)

### 架构图 (Architecture Diagram)

```
┌─────────────────────────────────────────────────────────────┐
│                     用户空间 (User Space)                    │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐      ┌──────────────┐                    │
│  │   ls / cp    │      │  应用程序     │                    │
│  │   vim / cat  │      │  (Applications)│                   │
│  └──────┬───────┘      └──────┬───────┘                    │
│         │                      │                            │
│         │  系统调用 (System Calls)                           │
│         ▼                      ▼                            │
│  ┌──────────────────────────────────────────────┐           │
│  │            内核 (VFS 层)                       │           │
│  │  (fuser crate 转换为 FUSE 协议)              │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│         ┌───────────┴───────────┐                            │
│         │   /dev/fuse 设备      │                            │
│         └───────────┬───────────┘                            │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │          EvifFuse (evif-fuse crate)           │           │
│  │  ┌────────────────────────────────────────┐  │           │
│  │  │  FUSE 操作 (Filesystem trait)          │  │           │
│  │  │  - getattr, readdir, read, write, etc. │  │           │
│  │  └───────────────┬────────────────────────┘  │           │
│  │                  │                            │           │
│  │  ┌───────────────▼────────────────────────┐  │           │
│  │  │      InodeManager (inode 管理)         │  │           │
│  │  │  - 路径 ↔ Inode 映射                   │  │           │
│  │  │  - Inode 分配与回收                     │  │           │
│  │  └───────────────┬────────────────────────┘  │           │
│  │                  │                            │           │
│  │  ┌───────────────▼────────────────────────┐  │           │
│  │  │      DirCache (目录缓存)                │  │           │
│  │  │  - LRU 缓存 + TTL                       │  │           │
│  │  │  - 缓存失效                             │  │           │
│  │  └───────────────┬────────────────────────┘  │           │
│  └──────────────────┼───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │         RadixMountTable (挂载表)              │           │
│  │    - 最长前缀匹配 (Longest prefix matching)   │           │
│  │    - 插件查找和路由 (Plugin lookup)           │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │       EVIF 插件 (存储插件层)                  │           │
│  │  - MemFS, LocalFS, KVFS, S3FS 等             │           │
│  │  - 云存储 (S3, Azure, GCS 等)                │           │
│  │  - 特殊文件系统 (HTTP, 加密等)                │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │         存储后端 (Storage Backend)            │           │
│  │  - 内存、磁盘、云存储等                       │           │
│  └──────────────────────────────────────────────┘           │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### 数据流 (Data Flow)

**读取操作示例 (Read Operation Example):**

```
1. 用户命令：`cat /mnt/evif/file.txt`

2. 内核 VFS → FUSE → EvifFuse::read()

3. EvifFuse::read()
   - 解析 inode 到路径："/file.txt"
   - 通过 MountTable 查找插件
   - 调用 plugin.read("/file.txt", offset, size)
   - 返回数据到内核

4. 内核 → 用户空间：cat 显示内容
```

**写入操作示例 (Write Operation Example):**

```
1. 用户命令：`echo "hello" > /mnt/evif/file.txt`

2. 内核 VFS → FUSE → EvifFuse::write()

3. EvifFuse::write()
   - 检查 allow_write 标志
   - 解析 inode 到路径："/file.txt"
   - 使目录缓存失效
   - 通过 MountTable 查找插件
   - 调用 plugin.write("/file.txt", data, offset, flags)
   - 返回写入字节数

4. 内核确认写入完成
```

---

## 核心组件 (Core Components)

### 1. EvifFuse (FUSE 文件系统实现)

**位置**：`crates/evif-fuse/src/lib.rs`

**用途**：实现 `fuser::Filesystem` trait 以提供完整的 FUSE 功能。

**主要特性：**

```rust
pub struct EvifFuseFuse {
    /// Tokio 运行时，用于异步操作
    runtime: Arc<Runtime>,

    /// 挂载表，用于插件查找
    mount_table: Arc<RadixMountTable>,

    /// Inode 管理器，用于路径 ↔ inode 映射
    inode_manager: Arc<InodeManager>,

    /// 目录缓存，用于性能优化
    dir_cache: Arc<DirCache>,

    /// EVIF 命名空间中的根路径
    root_path: PathBuf,

    /// 允许写操作
    pub allow_write: bool,

    /// 缓存超时时间（秒）
    cache_timeout: u64,

    /// 文件句柄映射
    file_handles: Arc<RwLock<HashMap<u64, u64>>>,
}
```

**实现的 FUSE 操作：**

| 操作 | 描述 | 方法 |
|-----------|-------------|--------|
| `getattr` | 获取文件属性 | 获取文件元数据（大小、模式、时间戳） |
| `setattr` | 设置文件属性 | 修改文件属性（大小、模式、所有者） |
| `readdir` | 读取目录 | 列出目录内容 |
| `open` | 打开文件 | 检查权限并分配句柄 |
| `read` | 读取文件 | 在偏移量处读取文件数据 |
| `write` | 写入文件 | 在偏移量处写入数据 |
| `create` | 创建文件 | 创建新文件 |
| `unlink` | 删除文件 | 删除文件 |
| `mkdir` | 创建目录 | 创建新目录 |
| `rmdir` | 删除目录 | 删除目录 |
| `rename` | 重命名 | 移动文件/目录 |
| `fsync` | 同步文件 | 刷新文件数据 |
| `fsyncdir` | 同步目录 | 刷新目录数据 |
| `release` | 释放文件 | 关闭文件句柄 |
| `releasedir` | 释放目录 | 关闭目录句柄 |
| `statfs` | 文件系统统计 | 获取文件系统统计信息 |

---

### 2. InodeManager (Inode 管理器)

**位置**：`crates/evif-fuse/src/inode_manager.rs`

**用途**：管理文件路径和 inode 之间的双向映射。

**主要特性：**

- **双向映射**：路径 ↔ Inode
- **Inode 分配**：自动 inode 分配
- **引用计数**：跟踪 inode 使用
- **Inode 回收**：清理已删除文件

**数据结构：**

```rust
/// Inode 信息
pub struct InodeInfo {
    pub inode: Inode,        // inode 编号
    pub path: String,         // 文件路径
    pub is_dir: bool,         // 是否是目录
    pub ref_count: u32,       // 引用计数
}

/// Inode 管理器
pub struct InodeManager {
    next_inode: Arc<Mutex<Inode>>,              // 下一个 inode
    path_to_inode: Arc<RwLock<HashMap<String, Inode>>>,   // 路径 → inode
    inode_to_info: Arc<RwLock<HashMap<Inode, InodeInfo>>>, // inode → 信息
}
```

**使用示例：**

```rust
// 创建 inode 管理器
let manager = InodeManager::new(10000);  // cache_size: 10000

// 获取或创建 inode
let inode = manager.get_or_create("/path/to/file.txt");

// 从 inode 获取路径
let path = manager.get_path(inode);  // Some("/path/to/file.txt")

// 从路径获取 inode
let inode = manager.get_inode("/path/to/file.txt");  // Some(inode)

// 引用计数
manager.incref(inode);  // 增加引用计数
let is_zero = manager.decref(inode);  // 减少引用计数

// 回收 inode
manager.recycle(inode);  // 从映射中删除
```

**特殊 Inode：**

```rust
pub const ROOT_INODE: Inode = 1;    // 根目录 (/)
pub const PARENT_INODE: Inode = 2;   // 父目录 (..)
```

---

### 3. DirCache (目录缓存)

**位置**：`crates/evif-fuse/src/dir_cache.rs`

**用途**：缓存目录内容以提高 `readdir` 性能。

**主要特性：**

- **LRU 淘汰**：最近最少使用策略
- **TTL 过期**：基于时间的缓存失效
- **自动清理**：淘汰过期条目

**数据结构：**

```rust
/// 目录条目
pub struct DirEntry {
    pub inode: u64,        // inode 编号
    pub name: String,      // 文件名
    pub is_dir: bool,      // 是否是目录
}

/// 目录缓存
pub struct DirCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,  // 缓存数据
    lru_head: Arc<RwLock<Option<String>>>,            // LRU 链表头
    lru_tail: Arc<RwLock<Option<String>>>,            // LRU 链表尾
    lru_nodes: Arc<RwLock<HashMap<String, LruNode>>>, // LRU 节点
    max_entries: usize,       // 最大缓存条目数
    ttl: Duration,            // 存活时间
    current_size: Arc<RwLock<usize>>,  // 当前大小
}
```

**使用示例：**

```rust
// 创建目录缓存，TTL 为 60 秒
let cache = DirCache::new(60);

// 缓存目录条目
cache.put("/dir/".to_string(), vec![
    DirEntry::new(10, "file1.txt".to_string(), false),
    DirEntry::new(11, "file2.txt".to_string(), false),
]);

// 获取缓存条目
if let Some(entries) = cache.get("/dir/") {
    for entry in entries {
        println!("{}: {}", entry.inode, entry.name);
    }
}

// 使缓存失效
cache.invalidate("/dir/");

// 清理过期条目
let expired_count = cache.cleanup_expired();

// 获取缓存统计信息
let (current, max, ttl) = cache.stats();
```

---

### 4. Mount Configuration (挂载配置)

**位置**：`crates/evif-fuse/src/mount_config.rs`

**用途**：管理 FUSE 挂载配置选项。

**配置结构：**

```rust
pub struct FuseMountConfig {
    /// 挂载点路径
    pub mount_point: PathBuf,

    /// EVIF 中的根路径
    pub root_path: PathBuf,

    /// 允许写操作
    pub allow_write: bool,

    /// 允许其他用户访问
    pub allow_other: bool,

    /// 缓存大小（inode 缓存条目数）
    pub cache_size: usize,

    /// 缓存超时时间（秒）
    pub cache_timeout: u64,
}
```

**挂载选项：**

```rust
pub enum MountOptions {
    ReadOnly,       // 只读挂载
    ReadWrite,      // 读写挂载
    AllowOther,     // 允许其他用户
    AllowMap,       // 允许映射
    AllowLock,      // 允许锁定
    AllowExec,      // 允许执行
    NoCache,        // 禁用 FUSE 缓存
    NoAttrCache,    // 禁用属性缓存
    NoDirCache,     // 禁用目录缓存
    KernelCache,    // 启用内核缓存
    AsyncRead,      // 异步读取
    WritebackCache, // 写回缓存
}
```

---

## FUSE 操作映射 (FUSE Operations Mapping)

### POSIX 操作 → EVIF 插件调用

**文件操作：**

| POSIX 操作 | FUSE 方法 | EVIF 插件方法 | 描述 |
|----------------|-------------|-------------------|-------------|
| `stat()` | `getattr` | `stat()` | 获取文件属性 |
| `open()` | `open` | 检查存在 | 检查文件存在 |
| `read()` | `read` | `read()` | 读取文件数据 |
| `write()` | `write` | `write()` | 写入文件数据 |
| `create()` | `create` | `create()` | 创建新文件 |
| `unlink()` | `unlink` | `remove()` | 删除文件 |
| `truncate()` | `setattr` | `read()` + `write()` | 调整文件大小 |
| `chmod()` | `setattr` | (无操作) | 修改权限 |
| `chown()` | `setattr` | (无操作) | 修改所有者 |
| `fsync()` | `fsync` | (自动持久化) | 同步到磁盘 |
| `close()` | `release` | (句柄清理) | 关闭文件 |

**目录操作：**

| POSIX 操作 | FUSE 方法 | EVIF 插件方法 | 描述 |
|----------------|-------------|-------------------|-------------|
| `opendir()` | `open` | 检查存在 | 检查目录存在 |
| `readdir()` | `readdir` | `readdir()` | 列出目录内容 |
| `mkdir()` | `mkdir` | `mkdir()` | 创建目录 |
| `rmdir()` | `rmdir` | `remove()` | 删除目录 |
| `rename()` | `rename` | `rename()` | 移动文件/目录 |
| `closedir()` | `releasedir` | (句柄清理) | 关闭目录 |

**属性操作：**

| POSIX 操作 | FUSE 方法 | EVIF 插件方法 | 描述 |
|----------------|-------------|-------------------|-------------|
| `statfs()` | `statfs` | (静态值) | 文件系统统计 |
| `utimens()` | `setattr` | (无操作) | 修改时间戳 |

---

## 挂载配置 (Mount Configuration)

### 挂载配置构建器 (Mount Configuration Builder)

**基本用法：**

```rust
use evif_fuse::{FuseMountBuilder, FuseMountConfig};
use evif_core::RadixMountTable;
use std::path::Path;
use std::sync::Arc;

// 创建挂载表
let mount_table = Arc::new(RadixMountTable::new());

// 使用构建器配置
let config = FuseMountBuilder::new()
    .mount_point(Path::new("/mnt/evif"))      // 必需
    .root_path(Path::new("/"))                 // EVIF 根路径
    .allow_write(false)                        // 只读
    .allow_other(false)                        // 仅所有者
    .cache_size(10000)                         // inode 缓存大小
    .cache_timeout(60)                         // 缓存 TTL 秒数
    .build()?;
```

**预定义配置：**

```rust
// 只读挂载
let config = FuseMountConfig::readonly(
    PathBuf::from("/mnt/evif")
);

// 读写挂载
let config = FuseMountConfig::readwrite(
    PathBuf::from("/mnt/evif")
);

// 默认挂载
let config = FuseMountConfig::default();
// 使用：mount_point=/mnt/evif, readonly, cache_size=10000, ttl=60s
```

### 挂载选项 (Mount Options)

**常用选项：**

| 选项 | 描述 | 示例 |
|--------|-------------|---------|
| `ro` | 只读挂载 | `--readonly` |
| `rw` | 读写挂载 | `--readwrite` |
| `allow_other` | 允许其他用户 | `--allow-other` |
| `no_cache` | 禁用 FUSE 缓存 | (编程方式) |
| `async_read` | 异步读取 | (编程方式) |

**安全考虑：**

- **默认只读**：出于安全考虑，默认挂载为只读
- **允许其他用户**：需要在 `/etc/fuse.conf` 中配置 `user_allow_other`
- **文件权限**：依赖插件，不强制执行文件所有权

---

## 使用示例 (Usage Examples)

### 示例 1：只读挂载 (Read-Only Mount)

```rust
use evif_core::RadixMountTable;
use evif_fuse::{mount_evif, FuseMountConfig};
use evif_plugins::MemFs;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建挂载表
    let mount_table = Arc::new(RadixMountTable::new());

    // 挂载 MemFS 插件
    let memfs = Arc::new(MemFs::new()?);
    mount_table.mount("/mem", memfs).await?;

    // 配置 FUSE 挂载
    let config = FuseMountConfig {
        mount_point: PathBuf::from("/mnt/evif"),
        root_path: PathBuf::from("/"),
        allow_write: false,  // 只读
        allow_other: false,
        cache_size: 10000,
        cache_timeout: 60,
    };

    // 挂载文件系统（阻塞）
    mount_evif(mount_table, Path::new("/mnt/evif"), config)?;

    Ok(())
}
```

### 示例 2：读写挂载 (Read-Write Mount)

```rust
use evif_core::RadixMountTable;
use evif_fuse::{mount_evif_background, FuseMountBuilder};
use evif_plugins::LocalFs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mount_table = Arc::new(RadixMountTable::new());

    // 挂载本地文件系统
    let localfs = Arc::new(LocalFs::new("/data/storage")?);
    mount_table.mount("/local", localfs).await?;

    // 构建读写配置
    let config = FuseMountBuilder::new()
        .mount_point(Path::new("/mnt/evif"))
        .root_path(Path::new("/"))
        .allow_write(true)    // 启用写入
        .allow_other(false)
        .cache_size(10000)
        .cache_timeout(60)
        .build()?;

    // 后台挂载
    let _session = mount_evif_background(
        mount_table,
        Path::new("/mnt/evif"),
        config
    )?;

    println!("EVIF 挂载到 /mnt/evif (读写)");
    println!("按 Ctrl+C 卸载");

    // 等待 Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("正在卸载...");

    Ok(())
}
```

### 示例 3：使用命令行工具 (Using Command-Line Tool)

**构建工具：**

```bash
cargo build --bin evif-fuse-mount --release
```

**基本用法：**

```bash
# 只读挂载
sudo ./target/release/evif-fuse-mount /mnt/evif --readonly

# 读写挂载
sudo ./target/release/evif-fuse-mount /mnt/evif --readwrite

# 允许其他用户
sudo ./target/release/evif-fuse-mount /mnt/evif --readwrite --allow-other

# 自定义缓存设置
sudo ./target/release/evif-fuse-mount /mnt/evif \
    --readonly \
    --cache-size 5000 \
    --cache-timeout 120
```

**卸载：**

```bash
# 方法 1：Ctrl+C（如果在前台运行）
# 在运行 evif-fuse-mount 的终端中按 Ctrl+C

# 方法 2：fusermount
fusermount -u /mnt/evif

# 方法 3：umount
umount /mnt/evif
```

### 示例 4：访问挂载的文件系统 (Accessing Mounted Filesystem)

**挂载到 `/mnt/evif` 后：**

```bash
# 列出文件
ls -la /mnt/evif/

# 读取文件
cat /mnt/evif/mem/test.txt

# 写入文件（如果是读写挂载）
echo "Hello, EVIF!" > /mnt/evif/local/newfile.txt

# 复制文件
cp /tmp/file.txt /mnt/evif/local/

# 创建目录
mkdir /mnt/evif/local/newdir

# 删除文件
rm /mnt/evif/local/oldfile.txt

# 与任何应用一起使用
vim /mnt/evif/mem/config.yaml
rsync -av /data/ /mnt/evif/local/backup/
```

---

## 性能优化 (Performance Optimization)

### 1. Inode 缓存 (Inode Caching)

**目的**：减少路径 → inode 查找开销

**配置：**

```rust
let config = FuseMountBuilder::new()
    .cache_size(10000)  // inode 条目数
    .build()?;
```

**权衡：**
- **更大缓存**：更好性能，更多内存
- **更小缓存**：更低内存使用，更多查找

**推荐值：**
- **小文件系统（<1000 个文件）**：1000-5000
- **中等文件系统（1000-10000 个文件）**：10000-50000
- **大文件系统（>10000 个文件）**：50000+

---

### 2. 目录缓存 (Directory Caching)

**目的**：缓存目录列表以减少 `readdir` 调用

**特性：**
- **LRU 淘汰**：自动淘汰最近最少使用的条目
- **TTL 过期**：基于时间的失效
- **自动失效**：写入操作时

**配置：**

```rust
let config = FuseMountBuilder::new()
    .cache_timeout(60)  // TTL 秒数
    .build()?;
```

**推荐值：**
- **静态内容**：300-600 秒（5-10 分钟）
- **动态内容**：30-60 秒
- **实时更新**：5-10 秒

**手动缓存控制：**

```rust
// 使特定目录失效
fuse.dir_cache.invalidate("/path/to/dir");

// 清理过期条目
let expired_count = fuse.dir_cache.cleanup_expired();

// 清空所有缓存
fuse.dir_cache.clear();
```

---

### 3. 异步运行时优化 (Async Runtime Optimization)

**Tokio 多线程运行时**：所有 FUSE 操作使用异步运行时以实现高效 I/O

**配置：**

```rust
let runtime = Arc::new(
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)  // 工作线程数
        .build()?
);
```

**线程池大小：**
- **CPU 密集型操作**：CPU 核心数
- **I/O 密集型操作**：2-4 倍 CPU 核心数

---

### 4. 挂载选项调优 (Mount Options Tuning)

**内核缓存选项：**

```rust
// 启用内核缓存以提升读性能
let options = vec![
    fuser::MountOption::RO,
    fuser::MountOption::FSName("evif".to_string()),
    fuser::MountOption::KernelCache,  // 启用内核缓存
];
```

**异步读取：**

```rust
// 启用异步读取以提升并发性
let options = vec![
    fuser::MountOption::AsyncRead,
];
```

---

## 平台支持 (Platform Support)

### Linux

**状态**：✅ 完全支持

**要求：**
- `libfuse-dev` 包
- FUSE 内核模块（通常内置）

**安装：**

```bash
# Ubuntu/Debian
sudo apt-get install libfuse-dev fuse

# Fedora/RHEL
sudo dnf install fuse-devel

# Arch Linux
sudo pacman -S fuse3
```

**使用：**

```bash
# 挂载
evif-fuse-mount /mnt/evif --readonly

# 卸载
fusermount -u /mnt/evif
```

---

### macOS

**状态**：✅ 完全支持

**要求：**
- FUSE for macOS (macFUSE) 或 osxfuse

**安装：**

```bash
# 安装 macFUSE
brew install --cask macfuse

# 或安装 osxfuse
brew install osxfuse
```

**使用：**

```bash
# 挂载
sudo evif-fuse-mount /mnt/evif --readonly

# 卸载
umount /mnt/evif
```

**注意事项：**
- 挂载操作需要 `sudo`
- macFUSE 可能需要系统扩展批准

---

### FreeBSD

**状态**：✅ 完全支持

**要求：**
- `fusefs-kmod` 内核模块
- `libfuse` 库

**安装：**

```bash
# 加载内核模块
kldload fusefs

# 安装库
pkg install fusefs-libs
```

**使用：**

```bash
# 挂载
evif-fuse-mount /mnt/evif --readonly

# 卸载
umount /mnt/evif
```

---

## 限制与未来工作 (Limitations and Future Work)

### 当前限制 (Current Limitations)

**❌ 未实现 (Not Implemented):**

1. **扩展属性 (Extended Attributes):**
   - `listxattr`、`getxattr`、`setxattr` 返回错误或无操作
   - EVIF 插件不支持扩展属性

2. **文件锁定 (File Locking):**
   - 不支持 `flock` 或 `lock`
   - 不支持咨询性记录锁定

3. **硬链接 (Hard Links):**
   - 不支持 `link()` 操作
   - EVIF 不支持跨插件的硬链接

4. **符号链接 (Symbolic Links):**
   - 只读符号链接支持（通过 VFS 层）
   - 无法通过 FUSE 创建新符号链接

5. **权限强制 (Permission Enforcement):**
   - 不强制执行 UID/GID
   - 文件模式位仅供参考

6. **访问时间跟踪 (Access Time Tracking):**
   - 不更新 `atime`（性能优化）
   - 仅跟踪 `mtime` 和 `ctime`

---

### 平台特定限制 (Platform-Specific Limitations)

**Linux:**
- `allow_other` 选项需要在 `/etc/fuse.conf` 中配置 `user_allow_other`

**macOS:**
- 挂载操作需要 root 权限
- macOS 安全策略可能阻止 FUSE

**FreeBSD:**
- 需要手动加载内核模块（`kldload fusefs`）

---

### 未来增强 (Future Enhancements)

**计划中的特性：**

1. **写入性能 (Write Performance):**
   - 写回缓存支持
   - 批量写入操作

2. **高级 POSIX 特性 (Advanced POSIX Features):**
   - 扩展属性支持
   - 文件锁定（flock、lock）
   - 符号链接创建（symlink）

3. **性能监控 (Performance Monitoring):**
   - 操作延迟跟踪
   - 缓存命中/未命中统计
   - 吞吐量指标

4. **动态配置 (Dynamic Configuration):**
   - 运行时缓存大小调整
   - 热重载挂载选项
   - 实时统计报告

5. **云原生优化 (Cloud-Native Optimizations):**
   - 云存储预取
   - 自适应缓存大小
   - 带宽感知操作

---

## 总结 (Summary)

**关键要点：**

1. **完整的 FUSE 支持**：EVIF 通过 FUSE 提供完整的 POSIX 文件系统语义
2. **跨平台**：在 Linux、macOS 和 FreeBSD 上运行
3. **性能优化**：inode 缓存、目录缓存和异步运行时
4. **灵活挂载**：只读、读写和多用户模式
5. **插件集成**：与所有 EVIF 插件透明集成
6. **生产就绪**：稳定实现，经过全面测试

**更多信息：**
- **第三章**：[架构设计](chapter-3-architecture.md) - 整体系统架构
- **第四章**：[虚拟文件系统](chapter-4-virtual-filesystem.md) - VFS 层设计
- **第五章**：[插件开发](chapter-5-plugin-development.md) - 插件接口和实现
- **GitHub 仓库**：[EVIF 源代码](https://github.com/your-org/evif)
