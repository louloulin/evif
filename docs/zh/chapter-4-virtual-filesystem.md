# 第四章：虚拟文件系统

## 目录

- [VFS 概述](#vfs-概述)
- [核心抽象](#核心抽象)
  - [FileSystem Trait](#filesystem-trait)
  - [文件抽象](#文件抽象)
  - [目录抽象](#目录抽象)
- [路径解析](#路径解析)
  - [PathResolver](#pathresolver)
  - [路径规范化](#路径规范化)
  - [符号链接处理](#符号链接处理)
- [存储后端](#存储后端)
  - [ContentStore](#contentstore)
  - [INode 管理](#inode-管理)
- [文件操作](#文件操作)
  - [打开与关闭](#打开与关闭)
  - [读写操作](#读写操作)
  - [定位操作](#定位操作)
- [挂载表架构](#挂载表架构)
  - [MountTable](#mounttable)
  - [最长前缀匹配](#最长前缀匹配)
  - [虚拟符号链接](#虚拟符号链接)
- [实现状态](#实现状态)

---

## VFS 概述

### 设计目标

EVIF 的虚拟文件系统（VFS）层提供了一个符合 POSIX 标准的文件系统抽象接口，支持多种存储后端的统一访问。VFS 层是"图 + VFS"技术栈的核心组件，虽然当前主路径使用插件系统，但 VFS 抽象为未来扩展提供了基础。

**核心特性**：

- **POSIX 兼容**：提供类似 Unix 文件系统的操作接口
- **异步支持**：基于 `async_trait` 的异步操作，适合高并发场景
- **多后端支持**：可插拔的存储后端（内存、磁盘、云存储等）
- **路径解析**：支持绝对路径、相对路径、符号链接
- **并发安全**：使用 `DashMap` 和 `Arc` 实现线程安全

### 架构层次

```
┌─────────────────────────────────────┐
│     应用层（REST/FUSE/CLI）         │
├─────────────────────────────────────┤
│         VFS 抽象层                   │
│  ┌──────────┬──────────┬──────────┐ │
│  │ FileSystem│ PathResolver│Handle│ │
│  └──────────┴──────────┴──────────┘ │
├─────────────────────────────────────┤
│         存储实现层                   │
│  ┌──────────┬──────────┬──────────┐ │
│  │ContentStore│ INodeCache│DEntry │ │
│  └──────────┴──────────┴──────────┘ │
├─────────────────────────────────────┤
│         图结构层                     │
│  ┌──────────┬──────────┬──────────┐ │
│  │  Graph   │  NodeId  │ VNode   │ │
│  └──────────┴──────────┴──────────┘ │
└─────────────────────────────────────┘
```

### 实现位置

**核心代码**：`crates/evif-vfs/src/`

- `lib.rs` - 模块导出与版本信息
- `vfs.rs` - VFS 主实现
- `filesystem.rs` - FileSystem trait 定义
- `file.rs` - 文件抽象与句柄管理
- `dir.rs` - 目录操作
- `path.rs` - 路径解析器
- `inode.rs` - 索引节点管理
- `dentry.rs` - 目录项缓存
- `vnode.rs` - 虚拟节点
- `error.rs` - 错误类型定义

**实现状态**：⚠️ **部分实现** - VFS 抽象层已定义，但当前主路径（Plugin + REST）未使用此层。

---

## 核心抽象

### FileSystem Trait

`FileSystem` trait 是 VFS 的核心抽象，定义了完整的文件系统操作接口。所有文件系统实现都需要实现此 trait。

#### 文件操作

```rust
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// 打开文件
    async fn open(&self, path: &Path, flags: OpenFlags) -> VfsResult<FileHandle>;

    /// 关闭文件
    async fn close(&self, handle: FileHandle) -> VfsResult<()>;

    /// 读取文件
    async fn read(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> VfsResult<usize>;

    /// 写入文件
    async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize>;

    /// 同步文件到磁盘
    async fn fsync(&self, handle: FileHandle) -> VfsResult<()>;

    /// 获取文件大小
    async fn get_file_size(&self, path: &Path) -> VfsResult<u64>;

    /// 设置文件大小
    async fn set_file_size(&self, path: &Path, size: u64) -> VfsResult<()>;

    /// 创建文件
    async fn create(&self, path: &Path, mode: FileMode) -> VfsResult<FileHandle>;

    /// 删除文件
    async fn unlink(&self, path: &Path) -> VfsResult<()>;

    /// 重命名文件
    async fn rename(&self, old_path: &Path, new_path: &Path) -> VfsResult<()>;
}
```

#### 目录操作

```rust
    /// 创建目录
    async fn mkdir(&self, path: &Path, mode: FileMode) -> VfsResult<()>;

    /// 删除目录
    async fn rmdir(&self, path: &Path) -> VfsResult<()>;

    /// 读取目录内容
    async fn readdir(&self, path: &Path) -> VfsResult<Vec<DirEntry>>;

    /// 打开目录
    async fn opendir(&self, path: &Path) -> VfsResult<u64>;

    /// 关闭目录
    async fn closedir(&self, dir_handle: u64) -> VfsResult<()>;
```

#### 元数据操作

```rust
    /// 获取文件属性
    async fn getattr(&self, path: &Path) -> VfsResult<FileAttributes>;

    /// 设置文件属性
    async fn setattr(&self, path: &Path, attrs: FileAttributes) -> VfsResult<()>;

    /// 更改文件权限
    async fn chmod(&self, path: &Path, mode: u32) -> VfsResult<()>;

    /// 更改文件所有者
    async fn chown(&self, path: &Path, uid: u32, gid: u32) -> VfsResult<()>;

    /// 更改访问和修改时间
    async fn utime(&self, path: &Path, atime: u64, mtime: u64) -> VfsResult<()>;
```

#### 符号链接操作

```rust
    /// 创建符号链接
    async fn symlink(&self, target: &Path, link_path: &Path) -> VfsResult<()>;

    /// 读取符号链接目标
    async fn readlink(&self, path: &Path) -> VfsResult<PathBuf>;
```

#### 文件系统操作

```rust
    /// 获取文件系统统计信息
    async fn statfs(&self) -> VfsResult<FileSystemStats>;

    /// 同步文件系统
    async fn sync(&self) -> VfsResult<()>;

    /// 检查路径是否存在
    async fn exists(&self, path: &Path) -> VfsResult<bool>;

    /// 检查是否为文件
    async fn is_file(&self, path: &Path) -> VfsResult<bool>;

    /// 检查是否为目录
    async fn is_directory(&self, path: &Path) -> VfsResult<bool>;

    /// 获取绝对路径
    async fn realpath(&self, path: &Path) -> VfsResult<PathBuf>;
```

#### 默认实现

`FileSystem` trait 提供了一些便捷的默认实现：

```rust
    /// 批量读取
    async fn read_full(&self, handle: FileHandle, offset: u64, size: usize) -> VfsResult<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        let total_read = self.read(handle, offset, &mut buffer).await?;
        buffer.truncate(total_read);
        Ok(buffer)
    }

    /// 批量写入
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

    /// 递归创建目录
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

    /// 递归删除目录
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

### 文件抽象

#### FileHandle

文件句柄唯一标识一个打开的文件：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHandle(u64);

impl FileHandle {
    pub fn new(handle: u64) -> Self;
    pub fn value(&self) -> u64;
    pub fn is_invalid(&self) -> bool;
}
```

**特性**：
- 基于整数的轻量级句柄
- 可序列化，支持跨线程传递
- 0 值表示无效句柄

#### OpenFlags

文件打开标志定义了文件的访问模式和特殊选项：

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

**预定义模式**：

```rust
/// 只读模式
pub const READ_ONLY: OpenFlags = OpenFlags { read: true, ... };

/// 只写模式
pub const WRITE_ONLY: OpenFlags = OpenFlags { write: true, ... };

/// 读写模式
pub const READ_WRITE: OpenFlags = OpenFlags { rw: true, ... };

/// 创建模式
pub const CREATE: OpenFlags = OpenFlags { create: true, ... };

/// 追加模式
pub const APPEND: OpenFlags = OpenFlags { append: true, ... };

/// 截断模式
pub const TRUNCATE: OpenFlags = OpenFlags { truncate: true, ... };
```

**使用示例**：

```rust
// 打开文件进行读取
let handle = fs.open(Path::new("/file.txt"), OpenFlags::READ_ONLY).await?;

// 创建文件（如果不存在）
let handle = fs.open(Path::new("/new.txt"), OpenFlags::CREATE).await?;

// 追加写入
let handle = fs.open(Path::new("/log.txt"), OpenFlags::APPEND).await?;

// 读写访问
let handle = fs.open(Path::new("/data.bin"), OpenFlags::READ_WRITE).await?;
```

#### FileMode

文件模式定义了文件的权限和类型：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMode {
    pub permissions: u32,      // 权限位（如 0o644）
    pub file_type: FileType,   // 文件类型
}
```

**预定义模式**：

```rust
/// 默认文件模式 (0644)
pub const DEFAULT_FILE: FileMode = FileMode {
    permissions: 0o644,
    file_type: FileType::Regular,
};

/// 默认目录模式 (0755)
pub const DEFAULT_DIRECTORY: FileMode = FileMode {
    permissions: 0o755,
    file_type: FileType::Directory,
};
```

**权限检查**：

```rust
impl FileMode {
    /// 是否可读
    pub fn is_readable(&self) -> bool;

    /// 是否可写
    pub fn is_writable(&self) -> bool;

    /// 是否可执行
    pub fn is_executable(&self) -> bool;
}
```

#### File

`File` 结构体表示一个打开的文件的状态：

```rust
pub struct File {
    handle: FileHandle,
    offset: AtomicU64,
    flags: OpenFlags,
    size: u64,
}
```

**核心方法**：

```rust
impl File {
    /// 创建新的文件描述符
    pub fn new(handle: FileHandle, flags: OpenFlags, size: u64) -> Self;

    /// 获取文件偏移量
    pub fn offset(&self) -> u64;

    /// 设置文件偏移量
    pub fn set_offset(&self, offset: u64);

    /// 增加文件偏移量
    pub fn add_offset(&self, delta: u64) -> u64;

    /// 是否可读
    pub fn can_read(&self) -> bool;

    /// 是否可写
    pub fn can_write(&self) -> bool;

    /// 是否为追加模式
    pub fn is_append(&self) -> bool;

    /// 读取数据（更新偏移量）
    pub fn read(&self, buf: &mut [u8], data: &[u8]) -> VfsResult<usize>;

    /// 写入数据（更新偏移量）
    pub fn write(&self, data: &[u8]) -> VfsResult<u64>;

    /// 定位到指定位置
    pub fn seek(&self, pos: SeekFrom) -> VfsResult<u64>;
}
```

**Seek 操作**：

```rust
pub enum SeekFrom {
    Start(u64),   // 从开始位置
    End(i64),     // 从结束位置
    Current(i64), // 从当前位置
}

// 使用示例
file.seek(SeekFrom::Start(0))?;      // 定位到文件开头
file.seek(SeekFrom::End(-10))?;      // 定位到文件末尾前 10 字节
file.seek(SeekFrom::Current(100))?;  // 从当前位置向前移动 100 字节
```

#### FileHandleAllocator

文件句柄分配器负责生成唯一的文件句柄：

```rust
pub struct FileHandleAllocator {
    next_handle: AtomicU64,
}

impl FileHandleAllocator {
    pub fn new() -> Self;
    pub fn alloc(&self) -> FileHandle;
}
```

**特性**：
- 基于原子计数器，线程安全
- 从 1 开始递增分配
- 确保句柄唯一性

### 目录抽象

#### DirEntry

目录项表示目录中的一个条目：

```rust
pub struct DirEntry {
    pub name: String,
    pub ino: u64,
    pub file_type: FileType,
}
```

**字段说明**：
- `name` - 文件或目录名称
- `ino` - 索引节点编号
- `file_type` - 文件类型（普通文件、目录、符号链接等）

---

## 路径解析

### PathResolver

`PathResolver` 负责将 POSIX 风格的文件路径解析为图节点 ID：

```rust
pub struct PathResolver {
    graph: Arc<Graph>,
    root_id: NodeId,
    cache: DashMap<PathBuf, NodeId>,
    max_symlink_depth: usize,
}
```

**核心功能**：

```rust
impl PathResolver {
    /// 创建新的路径解析器
    pub fn new(graph: Arc<Graph>, root_id: NodeId) -> Self;

    /// 解析路径到节点 ID
    pub fn resolve(&self, path: &Path) -> VfsResult<NodeId>;

    /// 规范化路径
    fn normalize_path(&self, path: &Path) -> VfsResult<PathBuf>;

    /// 创建路径到节点的映射
    pub fn create_path(&self, path: &Path, node_id: NodeId) -> VfsResult<()>;

    /// 移除路径缓存
    pub fn invalidate(&self, path: &Path);

    /// 获取父路径
    pub fn parent_path(&self, path: &Path) -> VfsResult<Option<PathBuf>>;

    /// 获取基础名称
    pub fn basename(&self, path: &Path) -> VfsResult<String>;

    /// 清空缓存
    pub fn clear_cache(&self);
}
```

### 路径规范化

`normalize_path` 方法将任意路径转换为规范形式：

**处理规则**：

1. **绝对路径与相对路径**
   - `/foo/bar` → 绝对路径
   - `foo/bar` → 相对路径

2. **特殊目录组件**
   - `.` - 当前目录（移除）
   - `..` - 父目录（弹出前一个组件）
   - `/` - 根目录（保留）

3. **路径验证**
   - 路径长度 ≤ 4096 字节
   - 文件名长度 ≤ 255 字节
   - 不包含空字符（`\0`）
   - 不支持 Windows 路径前缀（如 `C:\`）

**示例**：

```rust
let resolver = PathResolver::new(graph, root_id);

// 绝对路径
assert_eq!(resolver.normalize_path(Path::new("/foo/bar"))?,
           PathBuf::from("/foo/bar"));

// 相对路径
assert_eq!(resolver.normalize_path(Path::new("foo/bar"))?,
           PathBuf::from("foo/bar"));

// 父目录
assert_eq!(resolver.normalize_path(Path::new("/foo/../bar"))?,
           PathBuf::from("/bar"));

// 当前目录
assert_eq!(resolver.normalize_path(Path::new("/foo/./bar"))?,
           PathBuf::from("/foo/bar"));

// 复杂路径
assert_eq!(resolver.normalize_path(Path::new("/a/b/../c/./d"))?,
           PathBuf::from("/a/c/d"));
```

**错误处理**：

```rust
// 路径过长
let result = resolver.normalize_path(Path::new(&"/".repeat(5000)));
assert!(matches!(result, Err(VfsError::PathTooLong)));

// 文件名过长
let long_name = "a".repeat(300);
let result = resolver.normalize_path(Path::new(&format!("/{}", long_name)));
assert!(matches!(result, Err(VfsError::NameTooLong)));

// 包含空字符
let result = resolver.normalize_path(Path::new("/foo\0bar"));
assert!(matches!(result, Err(VfsError::InvalidPath(_))));
```

### 符号链接处理

`PathResolver` 支持符号链接的递归解析，带有循环检测：

```rust
impl PathResolver {
    /// 递归解析路径（带深度限制）
    fn resolve_path(&self, path: &Path, depth: usize) -> VfsResult<NodeId> {
        if depth > self.max_symlink_depth {
            return Err(VfsError::SymbolicLinkLoop(path.display().to_string()));
        }

        // ... 解析逻辑

        // 检查是否为符号链接
        if node.node_type == NodeType::Symlink {
            let target = self.get_symlink_target(&node)?;
            let target_path = path.parent().unwrap_or(Path::new("/")).join(&target);

            // 递归解析
            return self.resolve_path(&target_path, depth + 1);
        }

        Ok(current_id)
    }
}
```

**特性**：
- 最大递归深度：40 层（`MAX_SYMLINK_DEPTH`）
- 检测循环引用
- 自动解析路径中间的符号链接

**示例流程**：

```
/path/to/symlink1 → /another/path/symlink2 → /final/target
```

解析器会递归跟随符号链接，直到到达实际文件或达到最大深度。

---

## 存储后端

### ContentStore

`ContentStore` 负责存储和检索文件内容：

```rust
struct ContentStore {
    contents: DashMap<u64, Vec<u8>>,
}
```

**核心操作**：

```rust
impl ContentStore {
    fn new() -> Self;

    /// 读取内容
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

    /// 写入内容
    fn write(&self, node_id: u64, offset: u64, data: &[u8]) -> VfsResult<usize> {
        let mut existing = self.contents.entry(node_id).or_insert_with(Vec::new);
        let start = offset as usize;

        if offset as usize + data.len() > existing.len() {
            existing.resize(offset as usize + data.len(), 0);
        }

        existing[start..start + data.len()].copy_from_slice(data);
        Ok(data.len())
    }

    /// 获取大小
    fn size(&self, node_id: u64) -> VfsResult<u64> {
        self.contents.get(&node_id)
            .map(|data| data.len() as u64)
            .ok_or_else(|| VfsError::InternalError(format!("Node not found: {}", node_id)))
    }

    /// 截断文件
    fn truncate(&self, node_id: u64, size: u64) -> VfsResult<()> {
        let mut data = self.contents.entry(node_id).or_insert_with(Vec::new);
        data.resize(size as usize, 0);
        Ok(())
    }

    /// 删除内容
    fn delete(&self, node_id: u64) {
        self.contents.remove(&node_id);
    }
}
```

**特性**：
- 基于 `DashMap` 的并发安全存储
- 按节点 ID 索引文件内容
- 支持部分读写和截断操作
- 自动扩容（写入超出当前大小时）

### INode 管理

#### INode 结构

索引节点（INode）代表文件系统中的文件或目录的元数据：

```rust
pub struct INode {
    pub id: NodeId,       // 图节点 ID
    pub ino: u64,         // INode 编号
    pub size: u64,        // 文件大小
    pub blocks: u64,      // 块数量
    pub mode: u32,        // 权限模式
    pub nlink: u64,       // 硬链接数
    pub uid: u32,         // 用户 ID
    pub gid: u32,         // 组 ID
    pub rdev: u64,        // 设备 ID
    pub atime: u64,       // 访问时间
    pub mtime: u64,       // 修改时间
    pub ctime: u64,       // 创建时间
}
```

**类型检查方法**：

```rust
impl INode {
    /// 是否为目录
    pub fn is_directory(&self) -> bool {
        (self.mode & 0o170000) == 0o040000
    }

    /// 是否为普通文件
    pub fn is_regular_file(&self) -> bool {
        (self.mode & 0o170000) == 0o100000
    }

    /// 是否为符号链接
    pub fn is_symlink(&self) -> bool {
        (self.mode & 0o170000) == 0o120000
    }
}
```

#### INodeCache

`INodeCache` 提供索引节点的缓存管理：

```rust
pub struct INodeCache {
    inodes: Arc<RwLock<HashMap<NodeId, INode>>>,
    ino_map: Arc<RwLock<HashMap<u64, NodeId>>>,
    next_ino: Arc<AtomicU64>,
    max_size: usize,
}
```

**核心操作**：

```rust
impl INodeCache {
    pub fn new(max_size: usize) -> Self;

    /// 获取 INode
    pub async fn get(&self, id: &NodeId) -> Option<INode>;

    /// 通过 INode 编号获取
    pub async fn get_by_ino(&self, ino: u64) -> Option<INode>;

    /// 插入 INode
    pub async fn insert(&self, inode: INode) -> VfsResult<()>;

    /// 移除 INode
    pub async fn remove(&self, id: &NodeId) -> Option<INode>;

    /// 分配新的 INode 编号
    pub fn alloc_ino(&self) -> u64;

    /// 缓存大小
    pub async fn len(&self) -> usize;

    /// 是否为空
    pub async fn is_empty(&self) -> bool;

    /// 清空缓存
    pub async fn clear(&self);
}
```

**特性**：
- 双索引：节点 ID 和 INode 编号
- 自动驱逐：超过最大大小时移除旧条目
- 原子编号分配
- 线程安全

**使用示例**：

```rust
let cache = INodeCache::new(1000);

// 创建新 INode
let id = NodeId::new_v4();
let ino = cache.alloc_ino();
let inode = INode::new(id, ino);

// 插入缓存
cache.insert(inode).await?;

// 通过 ID 查询
if let Some(inode) = cache.get(&id).await {
    println!("INode: size={}, mode={}", inode.size, inode.mode);
}

// 通过编号查询
if let Some(inode) = cache.get_by_ino(ino).await {
    println!("Found inode by number: {}", inode.ino);
}

// 清理
cache.remove(&id).await;
```

---

## 文件操作

### 打开与关闭

**打开文件**：

```rust
async fn open(&self, path: &Path, flags: OpenFlags) -> VfsResult<FileHandle> {
    // 验证路径
    self.validate_path(path)?;

    // 检查只读保护
    if self.config.read_only && flags.can_write() {
        return Err(VfsError::ReadOnlyFileSystem);
    }

    // 分配句柄
    let handle = self.handle_allocator.alloc();
    let file = Arc::new(File::new(handle, flags, size));

    // 检查打开文件数限制
    if self.open_files.len() >= self.config.max_open_files {
        return Err(VfsError::InternalError("打开文件数超过限制".to_string()));
    }

    self.open_files.insert(handle, file);
    Ok(handle)
}
```

**关闭文件**：

```rust
async fn close(&self, handle: FileHandle) -> VfsResult<()> {
    self.open_files.remove(&handle)
        .ok_or_else(|| VfsError::InvalidFileHandle(handle.value()))?;
    Ok(())
}
```

### 读写操作

**读取文件**：

```rust
async fn read(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
    let file = self.open_files.get(&handle)
        .ok_or_else(|| VfsError::InvalidFileHandle(handle.value()))?;

    // 检查读取权限
    if !file.can_read() {
        return Err(VfsError::PermissionDenied("文件未以读模式打开".to_string()));
    }

    // 从内容存储读取
    let node_id = handle.value();
    self.content_store.read(node_id, offset, buf)
}
```

**写入文件**：

```rust
async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize> {
    // 检查只读保护
    if self.config.read_only {
        return Err(VfsError::ReadOnlyFileSystem);
    }

    let file = self.open_files.get(&handle)
        .ok_or_else(|| VfsError::InvalidFileHandle(handle.value()))?;

    // 检查写入权限
    if !file.can_write() {
        return Err(VfsError::PermissionDenied("文件未以写模式打开".to_string()));
    }

    // 写入到内容存储
    let node_id = handle.value();
    self.content_store.write(node_id, offset, data)
}
```

### 定位操作

文件内定位通过 `File::seek` 实现：

```rust
pub fn seek(&self, pos: SeekFrom) -> VfsResult<u64> {
    let new_offset = match pos {
        SeekFrom::Start(offset) => offset,
        SeekFrom::End(offset) => {
            if offset < 0 {
                let offset_abs = offset.unsigned_abs();
                if offset_abs > self.size {
                    return Err(VfsError::InvalidOperation("定位位置超出文件范围".to_string()));
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
                    return Err(VfsError::InvalidOperation("定位位置超出文件范围".to_string()));
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

**使用示例**：

```rust
// 打开文件
let handle = fs.open(Path::new("/data.txt"), OpenFlags::READ_WRITE).await?;

// 定位到文件开头
let file = fs.open_files.get(&handle).unwrap();
file.seek(SeekFrom::Start(0))?;

// 定位到文件末尾
file.seek(SeekFrom::End(0))?;

// 定位到文件倒数 100 字节
file.seek(SeekFrom::End(-100))?;

// 从当前位置向前移动 50 字节
file.seek(SeekFrom::Current(50))?;
```

---

## 挂载表架构

### MountTable

`MountTable` 实现了插件挂载和路径路由功能：

```rust
pub struct MountTable {
    mounts: Arc<RwLock<HashMap<String, Arc<dyn EvifPlugin>>>>,
    symlinks: Arc<RwLock<HashMap<String, String>>>,  // 虚拟符号链接
}
```

**核心操作**：

```rust
impl MountTable {
    pub fn new() -> Self;

    /// 挂载插件
    pub async fn mount(&self, path: String, plugin: Arc<dyn EvifPlugin>) -> EvifResult<()>;

    /// 卸载插件
    pub async fn unmount(&self, path: &str) -> EvifResult<()>;

    /// 查找插件（最长前缀匹配）
    pub async fn lookup(&self, path: &str) -> Option<Arc<dyn EvifPlugin>>;

    /// 列出所有挂载点
    pub async fn list_mounts(&self) -> Vec<String>;

    /// 获取挂载点数量
    pub async fn mount_count(&self) -> usize;
}
```

### 最长前缀匹配

挂载表使用最长前缀匹配算法来查找最合适的插件：

```rust
pub async fn lookup(&self, path: &str) -> Option<Arc<dyn EvifPlugin>> {
    let mounts = self.mounts.read().await;
    let normalized_path = Self::normalize_path(path);

    // 找到最长的匹配前缀
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

**示例场景**：

```rust
let mount_table = MountTable::new();

// 挂载多个插件
mount_table.mount("/".to_string(), root_plugin).await?;
mount_table.mount("/local".to_string(), local_plugin).await?;
mount_table.mount("/local/documents".to_string(), docs_plugin).await?;

// 查找测试
mount_table.lookup("/local/documents/file.txt").await?;
// → 返回 docs_plugin（最长匹配）

mount_table.lookup("/local/images/photo.jpg").await?;
// → 返回 local_plugin（匹配 /local）

mount_table.lookup("/tmp/file.tmp").await?;
// → 返回 root_plugin（匹配根路径）
```

### 虚拟符号链接

`MountTable` 提供了虚拟符号链接支持，无需后端存储：

```rust
impl MountTable {
    /// 创建符号链接（虚拟）
    pub async fn symlink(&self, target_path: &str, link_path: &str) -> EvifResult<()> {
        let normalized_target = Self::normalize_path(target_path);
        let normalized_link = Self::normalize_path(link_path);

        let mut symlinks = self.symlinks.write().await;
        symlinks.insert(normalized_link.clone(), normalized_target);
        Ok(())
    }

    /// 读取符号链接目标（非递归）
    pub async fn readlink(&self, link_path: &str) -> EvifResult<String> {
        let normalized_link = Self::normalize_path(link_path);
        let symlinks = self.symlinks.read().await;
        match symlinks.get(&normalized_link) {
            Some(target) => Ok(target.clone()),
            None => Err(EvifError::NotFound(format!("Symlink: {}", link_path))),
        }
    }

    /// 递归解析符号链接（带循环检测）
    pub async fn resolve_symlink_recursive(&self, path: &str, max_depth: usize) -> EvifResult<String> {
        let mut current_path = Self::normalize_path(path);
        let mut visited = std::collections::HashSet::new();

        for _ in 0..max_depth {
            // 检查循环
            if visited.contains(&current_path) {
                return Err(EvifError::InvalidInput(format!(
                    "Symbolic link cycle detected: {}",
                    path
                )));
            }
            visited.insert(current_path.clone());

            // 尝试解析
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

**特性**：
- 无需后端支持，完全在内存中实现
- 递归解析，最多 40 层
- 循环检测
- 支持跨文件系统的符号链接

**使用示例**：

```rust
let mount_table = MountTable::new();

// 创建符号链接
mount_table.symlink("/local/documents", "/docs").await?;
mount_table.symlink("/local/images", "/pics").await?;

// 读取符号链接
let target = mount_table.readlink("/docs").await?;
assert_eq!(target, "/local/documents");

// 递归解析（支持符号链接指向符号链接）
mount_table.symlink("/pics", "/photos").await?;
mount_table.symlink("/photos", "/images").await?;
let resolved = mount_table.resolve_symlink_recursive("/images", 10).await?;
assert_eq!(resolved, "/local/images");
```

---

## 实现状态

### 当前实现

**✅ 已完成**：

- VFS 核心抽象定义（FileSystem trait）
- 文件操作（打开、关闭、读、写、定位）
- 目录操作（创建、删除、读取）
- 路径解析与规范化
- 符号链接支持
- INode 管理与缓存
- ContentStore 内容存储
- FileHandle 句柄管理

**⚠️ 部分实现**：

- PathResolver 与图结构的集成（`find_child` 方法返回错误）
- DEntry 和 VNode 的完整实现
- 与存储后端（evif-storage）的集成

**❌ 未集成**：

- VFS 层与主路径（Plugin + REST）的集成
- FUSE 对 VFS 层的使用（当前 FUSE 直接使用插件）
- REST API 对 VFS 的暴露

### 当前主路径

**实际使用的架构**：

```
REST API → EvifPlugin → MountTable → Storage Plugins
```

**VFS 层的位置**：

```
VFS 抽象（已定义但未使用）
    ↓
EvifPlugin Trait（实际使用）
    ↓
MountTable + Plugins（运行中）
```

### 未来计划

1. **VFS 与插件系统集成**：在插件和 VFS 之间建立适配层
2. **图查询支持**：完成 PathResolver 与图结构的集成
3. **统一存储层**：通过 evif-storage 提供统一的存储抽象
4. **FUSE VFS 支持**：让 FUSE 可以使用 VFS 抽象层

### 使用建议

**当前推荐**：
- 继续使用 `EvifPlugin` trait 开发新插件
- 使用 `MountTable` 管理插件挂载
- VFS 抽象层供未来使用

**不推荐**：
- 直接使用 VFS 层（未完全集成）
- 依赖 PathResolver 的图查询功能

---

## 相关章节

- **[第五章：插件开发](chapter-5-plugin-development.md)** - EvifPlugin trait 详细使用指南
- **[第六章：FUSE 集成](chapter-6-fuse.md)** - FUSE 挂载与文件系统访问
- **[第三章：架构设计](chapter-3-architecture.md)** - 系统整体架构与双技术栈

---

**文档版本**: 1.0
**最后更新**: 2026-03-01
**作者**: EVIF Team
