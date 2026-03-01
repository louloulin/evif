# 第五章：插件开发

## 目录

- [插件系统概述](#插件系统概述)
  - [设计目标](#设计目标)
  - [对标 AGFS](#对标-agfs)
  - [架构概览](#架构概览)
- [核心接口](#核心接口)
  - [EvifPlugin Trait](#evifplugin-trait)
  - [文件操作方法](#文件操作方法)
  - [目录操作方法](#目录操作方法)
  - [扩展接口](#扩展接口)
- [插件类型](#插件类型)
  - [基础存储插件](#基础存储插件)
  - [云存储插件](#云存储插件)
  - [特殊功能插件](#特殊功能插件)
  - [OpenDAL 统一插件](#opendal-统一插件)
- [插件开发指南](#插件开发指南)
  - [创建基本插件](#创建基本插件)
  - [实现 EvifPlugin](#实现-evifplugin)
  - [配置管理](#配置管理)
  - [错误处理](#错误处理)
  - [测试策略](#测试策略)
- [插件注册与挂载](#插件注册与挂载)
  - [挂载表机制](#挂载表机制)
  - [路径路由](#路径路由)
  - [符号链接支持](#符号链接支持)
- [示例插件](#示例插件)
  - [MemFS：内存文件系统](#memfs内存文件系统)
  - [LocalFS：本地文件系统](#localfs本地文件系统)
  - [KVFS：键值存储](#kvfs键值存储)
- [实现状态](#实现状态)

---

## 插件系统概述

### 设计目标

EVIF 的插件系统提供了一个灵活、可扩展的文件系统抽象层，允许开发者通过实现统一的接口来支持多种存储后端。插件系统是 EVIF 当前架构的核心组件，通过 REST API 暴露文件操作能力。

**核心特性**：

- **统一接口**：所有插件实现 `EvifPlugin` trait，提供标准的 POSIX 文件操作
- **异步支持**：基于 `async_trait` 的异步操作，适合高并发场景
- **配置驱动**：支持动态配置和验证，方便部署和管理
- **热插拔**：运行时挂载和卸载插件，无需重启服务
- **扩展性**：支持可选扩展接口（HandleFS、Streamer）
- **对标 AGFS**：完全对标 AGFS 插件接口，便于迁移和兼容

### 对标 AGFS

EVIF 插件系统完全对标 AGFS（Another Graph Filesystem）的插件接口：

| EVIF | AGFS | 说明 |
|------|------|------|
| `EvifPlugin` | `FileSystem` | 核心插件接口 |
| `FileInfo` | `FileInfo` | 文件元信息 |
| `OpenFlags` | `OpenFlag` | 文件打开标志 |
| `WriteFlags` | `WriteFlag` | 文件写入标志 |
| `FileHandle` | `FileHandle` | 有状态文件句柄 |
| `HandleFS` | `HandleFS` | 句柄支持扩展 |
| `MountTable` | `MountableFS` | 插件挂载表 |
| `validate()` | `Validate()` | 配置验证 |
| `get_readme()` | `GetReadme()` | 文档生成 |
| `get_config_params()` | `GetConfigParams()` | 配置元数据 |

### 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                     应用层                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  REST API    │  │  WebSocket   │  │  CLI Tools   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
├─────────────────────────────────────────────────────────┤
│                  EVIF Core                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ MountTable   │  │ HTTP Server  │  │ Auth Module  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
├─────────────────────────────────────────────────────────┤
│                  插件层 (EvifPlugin)                     │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │
│  │MemFS│ │Local│ │ KVFS│ │ S3  │ │Azure│ │ GCS │ ...  │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘      │
│  ┌──────────────────────────────────────────────────┐  │
│  │         OpenDAL 统一插件 (50+ 后端)               │  │
│  └──────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                  存储层                                  │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │
│  │内存 │ │磁盘 │ │数据库│ │ 对象存储│ │云存储│ │...  │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘      │
└─────────────────────────────────────────────────────────┘
```

**实现位置**：
- 核心接口：`crates/evif-core/src/plugin.rs`
- 挂载表：`crates/evif-core/src/mount_table.rs`
- 插件实现：`crates/evif-plugins/src/`

---

## 核心接口

### EvifPlugin Trait

`EvifPlugin` trait 是所有插件必须实现的核心接口，定义了完整的文件系统操作。

```rust
use async_trait::async_trait;
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};

#[async_trait]
pub trait EvifPlugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;

    /// 创建文件
    async fn create(&self, path: &str, perm: u32) -> EvifResult<()>;

    /// 创建目录
    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>;

    /// 读取文件
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;

    /// 写入文件
    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
        -> EvifResult<u64>;

    /// 读取目录内容
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;

    /// 获取文件信息
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;

    /// 删除文件或空目录
    async fn remove(&self, path: &str) -> EvifResult<()>;

    /// 重命名/移动文件
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;

    /// 递归删除目录及其所有内容
    async fn remove_all(&self, path: &str) -> EvifResult<()>;

    // 可选方法（提供默认实现）
    async fn symlink(&self, target_path: &str, link_path: &str) -> EvifResult<()>;
    async fn readlink(&self, link_path: &str) -> EvifResult<String>;
    async fn chmod(&self, path: &str, mode: u32) -> EvifResult<()>;
    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()>;

    // Phase 8: 配置管理方法
    async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()>;
    fn get_readme(&self) -> String;
    fn get_config_params(&self) -> Vec<PluginConfigParam>;
}
```

### 文件操作方法

#### create - 创建文件

创建一个空文件。

```rust
async fn create(&self, path: &str, perm: u32) -> EvifResult<()>
```

**参数**：
- `path`: 文件路径
- `perm`: 文件权限（八进制，如 0o644）

**示例**：
```rust
// 创建一个新文件，权限 644
plugin.create("/data/file.txt", 0o644).await?;
```

#### read - 读取文件

读取文件内容，支持偏移量和大小限制。

```rust
async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>
```

**参数**：
- `path`: 文件路径
- `offset`: 读取偏移量（0 = 从头开始）
- `size`: 读取大小（0 = 读取全部）

**示例**：
```rust
// 读取整个文件
let data = plugin.read("/data/file.txt", 0, 0).await?;

// 从偏移量 100 开始读取 1024 字节
let chunk = plugin.read("/data/large.bin", 100, 1024).await?;
```

#### write - 写入文件

写入数据到文件，支持多种写入模式。

```rust
async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
    -> EvifResult<u64>
```

**参数**：
- `path`: 文件路径
- `data`: 写入数据
- `offset`: 写入偏移量（-1 = 忽略）
- `flags`: 写入标志

**WriteFlags**：
```rust
bitflags::bitflags! {
    pub struct WriteFlags: u32 {
        const APPEND = 1 << 0;      // 追加写入
        const CREATE = 1 << 1;      // 创建文件
        const EXCLUSIVE = 1 << 2;   // 排他创建
        const TRUNCATE = 1 << 3;    // 截断文件
        const SYNC = 1 << 4;        // 同步写入
    }
}
```

**示例**：
```rust
use evif_core::WriteFlags;

// 创建并写入
let written = plugin.write(
    "/data/file.txt",
    b"Hello, EVIF!".to_vec(),
    0,
    WriteFlags::CREATE
).await?;

// 追加写入
let appended = plugin.write(
    "/data/log.txt",
    b"New log entry\n".to_vec(),
    -1,
    WriteFlags::APPEND
).await?;

// 截断并写入
plugin.write(
    "/data/temp.txt",
    b"Overwritten".to_vec(),
    0,
    WriteFlags::TRUNCATE | WriteFlags::CREATE
).await?;
```

### 目录操作方法

#### mkdir - 创建目录

创建一个新目录。

```rust
async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>
```

**示例**：
```rust
// 创建目录，权限 755
plugin.mkdir("/data/subdir", 0o755).await?;
```

#### readdir - 读取目录

列出目录内容。

```rust
async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>
```

**FileInfo 结构**：
```rust
pub struct FileInfo {
    pub name: String,       // 文件/目录名
    pub size: u64,          // 大小（字节）
    pub mode: u32,          // 权限模式
    pub modified: chrono::DateTime<chrono::Utc>,  // 修改时间
    pub is_dir: bool,       // 是否为目录
}
```

**示例**：
```rust
let entries = plugin.readdir("/data").await?;
for entry in entries {
    let type_str = if entry.is_dir { "DIR" } else { "FILE" };
    println!("{} {}: {} bytes", type_str, entry.name, entry.size);
}
```

#### remove - 删除文件或空目录

删除单个文件或空目录。

```rust
async fn remove(&self, path: &str) -> EvifResult<()>
```

**示例**：
```rust
// 删除文件
plugin.remove("/data/file.txt").await?;

// 删除空目录
plugin.remove("/data/empty_dir").await?;
```

#### remove_all - 递归删除

递归删除目录及其所有内容。

```rust
async fn remove_all(&self, path: &str) -> EvifResult<()>
```

**示例**：
```rust
// 删除整个目录树
plugin.remove_all("/data/nested_dir").await?;
```

#### rename - 重命名/移动

重命名或移动文件和目录。

```rust
async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>
```

**示例**：
```rust
// 重命名文件
plugin.rename("/data/old.txt", "/data/new.txt").await?;

// 移动文件到子目录
plugin.rename("/data/file.txt", "/data/subdir/file.txt").await?;
```

### 扩展接口

#### FileHandle - 有状态文件句柄

对于需要保持状态的文件操作（如大文件分块传输），插件可以实现 `FileHandle` 接口。

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

#### HandleFS - 句柄支持扩展

插件可以选择实现 `HandleFS` 扩展接口来支持有状态文件操作。

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

## 插件类型

EVIF 提供了多种类型的插件，覆盖不同的存储后端和使用场景。

### 基础存储插件

#### MemFS

内存文件系统插件，数据仅存储在进程内存中，重启后丢失。

**特性**：
- ✅ 完整的文件系统功能
- ✅ 高性能（内存操作）
- ✅ 无需配置
- ⚠️ 数据易失（重启丢失）

**使用场景**：
- 临时文件存储
- 缓存层
- 测试和开发

**配置**：
```json
{
  "mount": "/mem",
  "plugin": "memfs"
}
```

#### LocalFS

本地文件系统插件，将宿主机目录映射到 EVIF。

**特性**：
- ✅ 完整的文件系统功能
- ✅ 数据持久化
- ✅ 支持只读模式
- ✅ 路径遍历保护

**使用场景**：
- 本地文件访问
- 数据持久化
- 配置文件管理

**配置**：
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

键值存储插件，将键值存储映射为文件系统。

**特性**：
- ✅ 简单的键值接口
- ✅ 支持任意后端（内存、Redis 等）
- ✅ 高性能读写

**使用场景**：
- 元数据存储
- 缓存系统
- 配置管理

**配置**：
```json
{
  "mount": "/kv",
  "plugin": "kvfs",
  "config": {
    "prefix": "app"
  }
}
```

### 云存储插件

#### S3FS

AWS S3 对象存储插件。

**特性**：
- ✅ S3 API 兼容
- ✅ 目录缓存优化
- ✅ 元数据缓存
- ✅ 多区域支持

**配置**：
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

Azure Blob Storage 插件。

**配置**：
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

Google Cloud Storage 插件。

**配置**：
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

### 特殊功能插件

#### HttpFS

HTTP 文件系统插件，通过 HTTP 访问远程文件。

**特性**：
- ✅ HTTP/HTTPS 支持
- ✅ Range 请求支持
- ✅ 认证支持

**配置**：
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

流式文件系统插件，用于实时数据流。

**特性**：
- ✅ WebSocket 支持
- ✅ Server-Sent Events 支持
- ✅ 实时数据推送

**配置**：
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

加密文件系统插件，透明的数据加密。

**特性**：
- ✅ AES-256-GCM 加密
- ✅ 透明的加密/解密
- ✅ 密钥管理

**配置**：
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

### OpenDAL 统一插件

基于 Apache OpenDAL 的统一存储插件，支持 50+ 存储后端。

**支持的服务**：
- ✅ Memory (内存)
- ✅ Fs (本地文件系统)
- ✅ S3 (AWS S3 及兼容存储)
- ✅ Azblob (Azure Blob Storage)
- ✅ Gcs (Google Cloud Storage)
- ✅ Oss (阿里云对象存储)
- ✅ Cos (腾讯云对象存储)
- ✅ Obs (华为云对象存储)

**优势**：
- 统一的配置接口
- 统一的错误处理
- 自动重试和超时
- 内置缓存支持

**配置示例**：
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

## 插件开发指南

### 创建基本插件

让我们创建一个简单的 "HelloFS" 插件作为示例。

**步骤 1：定义插件结构**

```rust
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct HelloFsPlugin {
    // 内部存储：文件名 -> 内容
    files: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}
```

**步骤 2：实现构造函数**

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

**步骤 3：实现 EvifPlugin trait**

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
            // 简单的路径前缀匹配
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
        // 删除所有以 path 为前缀的文件
        files.retain(|k, _| !k.starts_with(path));
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        // HelloFS 不支持目录，但返回 Ok 以兼容接口
        Ok(())
    }
}
```

### 实现 EvifPlugin

#### 必需方法

所有插件必须实现以下方法：

1. **`name()`** - 返回插件名称
2. **`create()`** - 创建文件
3. **`mkdir()`** - 创建目录
4. **`read()`** - 读取文件
5. **`write()`** - 写入文件
6. **`readdir()`** - 列出目录
7. **`stat()`** - 获取文件信息
8. **`remove()`** - 删除文件或空目录
9. **`rename()`** - 重命名/移动
10. **`remove_all()`** - 递归删除

#### 可选方法

以下方法提供默认实现，插件可以按需覆盖：

- **`symlink()`** - 创建符号链接
- **`readlink()`** - 读取符号链接
- **`chmod()`** - 修改权限
- **`truncate()`** - 截断文件

### 配置管理

#### validate 方法

在挂载前验证插件配置。

```rust
async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()> {
    if let Some(cfg) = config {
        // 检查必需字段
        if let Some(root) = cfg.get("root") {
            if root.is_null() || root.as_str().map(|s| s.is_empty()).unwrap_or(true) {
                return Err(EvifError::InvalidInput(
                    "config.root cannot be empty".to_string()
                ));
            }
        }

        // 检查数值范围
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

#### get_readme 方法

返回插件的 README 文档。

```rust
fn get_readme(&self) -> String {
    r#"# HelloFS Plugin

一个简单的示例插件，用于演示 EVIF 插件开发。

## 配置

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| message | string | 否 | 欢迎消息，默认 "Hello" |

## 示例

```json
{
  "mount": "/hello",
  "plugin": "hellofs",
  "config": {
    "message": "Welcome"
  }
}
```

## 特性

- ✅ 基本文件操作
- ✅ 内存存储
- ⚠️ 数据易失（重启丢失）
"#.to_string()
}
```

#### get_config_params 方法

返回配置参数的元数据。

```rust
use evif_core::PluginConfigParam;

fn get_config_params(&self) -> Vec<PluginConfigParam> {
    vec![
        PluginConfigParam {
            name: "message".to_string(),
            param_type: "string".to_string(),
            required: false,
            default: Some("Hello".to_string()),
            description: Some("欢迎消息".to_string()),
        },
        PluginConfigParam {
            name: "max_size".to_string(),
            param_type: "int".to_string(),
            required: false,
            default: Some("1048576".to_string()),  // 1MB
            description: Some("最大文件大小（字节）".to_string()),
        },
    ]
}
```

### 错误处理

使用 `EvifError` 定义清晰的错误类型。

```rust
use evif_core::{EvifError, EvifResult};

async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
    // 路径验证
    if path.is_empty() {
        return Err(EvifError::InvalidPath("Path cannot be empty".to_string()));
    }

    // 权限检查
    if !self.has_read_permission(path).await {
        return Err(EvifError::PermissionDenied("Read access denied".to_string()));
    }

    // 文件不存在
    let file = self.find_file(path).await
        .ok_or_else(|| EvifError::NotFound(path.to_string()))?;

    // 读取数据
    let data = file.read(offset, size).await
        .map_err(|e| EvifError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Read failed: {}", e)
        )))?;

    Ok(data)
}
```

**常用错误类型**：
- `EvifError::NotFound` - 文件/目录不存在
- `EvifError::InvalidPath` - 无效路径
- `EvifError::PermissionDenied` - 权限不足
- `EvifError::ReadOnly` - 只读模式下的写操作
- `EvifError::NotSupportedGeneric` - 操作不支持
- `EvifError::InvalidInput` - 无效输入参数
- `EvifError::Io` - I/O 错误

### 测试策略

#### 单元测试

为每个插件方法编写单元测试。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use evif_core::WriteFlags;

    #[tokio::test]
    async fn test_create_and_read() {
        let plugin = HelloFsPlugin::new();

        // 创建文件
        plugin.create("/test.txt", 0o644).await.unwrap();

        // 写入数据
        plugin.write(
            "/test.txt",
            b"Hello, World!".to_vec(),
            0,
            WriteFlags::CREATE
        ).await.unwrap();

        // 读取数据
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

#### 集成测试

测试插件与挂载表的集成。

```rust
#[tokio::test]
async fn test_mount_table_integration() {
    use evif_core::MountTable;

    let mount_table = MountTable::new();
    let plugin = Arc::new(HelloFsPlugin::new());

    // 挂载插件
    mount_table.mount("/hello", plugin).await.unwrap();

    // 通过挂载表访问
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

## 插件注册与挂载

### 挂载表机制

`MountTable` 负责管理所有插件的挂载和路由。

```rust
use evif_core::MountTable;
use std::sync::Arc;

// 创建挂载表
let mount_table = MountTable::new();

// 创建插件实例
let memfs = Arc::new(MemFsPlugin::new());
let localfs = Arc::new(LocalFsPlugin::new("/tmp/data"));

// 挂载插件
mount_table.mount("/mem", memfs).await.unwrap();
mount_table.mount("/local", localfs).await.unwrap();
```

### 路径路由

挂载表使用**最长前缀匹配**算法路由文件操作到对应的插件。

```rust
// 挂载示例
// /mem    -> MemFsPlugin
// /local  -> LocalFsPlugin
// /s3/bucket -> S3FsPlugin

// 路径路由示例
"/mem/file.txt"           -> MemFsPlugin
"/local/data/file.txt"    -> LocalFsPlugin
"/s3/bucket/file.txt"     -> S3FsPlugin
"/s3/bucket/nested/file"  -> S3FsPlugin (最长前缀匹配)
```

**最长前缀匹配算法**：

```rust
pub async fn lookup(&self, path: &str) -> Option<(Arc<dyn EvifPlugin>, String)> {
    let mounts = self.mounts.read().await;
    let mut best_match: Option<&str> = None;
    let mut best_len = 0;

    // 遍历所有挂载点，找最长的前缀匹配
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

**示例**：

```rust
// 挂载
mount_table.mount("/s3", s3_plugin).await.unwrap();
mount_table.mount("/s3/bucket", nested_plugin).await.unwrap();

// 请求 "/s3/bucket/file.txt"
// 匹配结果：
// - "/s3" 长度 3
// - "/s3/bucket" 长度 10
// 选择 "/s3/bucket"（更长）
```

### 符号链接支持

挂载表支持虚拟符号链接，允许跨文件系统的链接。

```rust
// 创建符号链接
mount_table.symlink("/local/data", "/data").await.unwrap();

// 解析符号链接
let (resolved, is_link) = mount_table.resolve_symlink("/data").await;
assert!(is_link);
assert_eq!(resolved, "/local/data");

// 递归解析（支持链接指向链接）
mount_table.symlink("/local/archive", "/archive").await.unwrap();
mount_table.symlink("/archive/2024", "/data2024").await.unwrap();

let final_path = mount_table.resolve_symlink_recursive("/data2024", 10).await.unwrap();
assert_eq!(final_path, "/local/archive/2024");
```

**循环检测**：

```rust
// 创建循环链接
mount_table.symlink("/a", "/b").await.unwrap();
mount_table.symlink("/b", "/a").await.unwrap();

// 递归解析会检测到循环
let result = mount_table.resolve_symlink_recursive("/a", 10).await;
assert!(result.is_err());  // 返回循环错误
```

---

## 示例插件

### MemFS：内存文件系统

完整的内存文件系统实现，支持目录层次结构。

**核心特性**：
- 完整的文件和目录操作
- 递归目录操作
- 文件元数据管理
- 层次化路径支持

**实现要点**：

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
    /// 递归查找节点
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

### LocalFS：本地文件系统

将宿主机目录映射到 EVIF，支持安全检查和只读模式。

**核心特性**：
- 完整的文件系统操作
- 路径遍历保护
- 只读模式支持
- 权限和元数据保留

**安全检查**：

```rust
impl LocalFsPlugin {
    fn resolve_path(&self, path: &str) -> EvifResult<PathBuf> {
        let clean_path = path.trim_start_matches('/');
        let full = self.base_path.join(clean_path);

        // 安全检查：防止路径遍历攻击
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

### KVFS：键值存储

将键值存储映射为文件系统。

**核心特性**：
- 简单的键值接口
- 路径到键的映射
- 目录枚举

**路径映射**：

```rust
impl KvfsPlugin {
    /// 将文件路径转换为存储key
    fn path_to_key(&self, path: &str) -> EvifResult<String> {
        let clean_path = path.trim_start_matches('/');
        if clean_path.is_empty() {
            return Err(EvifError::InvalidPath("Path cannot be empty".to_string()));
        }
        Ok(format!("{}/{}", self.prefix.trim_end_matches('/'), clean_path))
    }

    /// 将文件路径转换为key前缀（用于列出目录）
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

**使用示例**：

```rust
// 创建 KVFS 插件
let kvfs = KvfsPlugin::new("app");

// 写入文件
kvfs.write("/config.json", config_data, 0, WriteFlags::CREATE).await?;

// 路径映射到键："/app/config.json"
kvfs.store.put("app/config.json".to_string(), config_data).await?;

// 读取目录
let entries = kvfs.readdir("/").await?;
// 返回所有以 "app/" 为前缀的键
```

---

## 实现状态

### ✅ 已完成

**核心接口**：
- ✅ EvifPlugin trait 完整定义（20+ 方法）
- ✅ FileHandle trait（有状态文件操作）
- ✅ HandleFS 扩展接口
- ✅ 配置管理（validate、get_readme、get_config_params）
- ✅ 错误类型定义（EvifError）

**基础插件（8个）**：
- ✅ MemFS - 内存文件系统
- ✅ LocalFS - 本地文件系统
- ✅ KVFS - 键值存储
- ✅ QueueFsPlugin - 队列文件系统
- ✅ ServerInfoFsPlugin - 服务器信息
- ✅ HttpFsPlugin - HTTP 文件系统
- ✅ StreamFsPlugin - 流式文件系统
- ✅ ProxyFsPlugin - 代理文件系统
- ✅ DevFsPlugin - 设备文件系统
- ✅ HelloFsPlugin - 示例插件
- ✅ HeartbeatFsPlugin - 心跳插件
- ✅ HandleFsPlugin - 句柄文件系统
- ✅ TieredFsPlugin - 分层存储
- ✅ EncryptedFsPlugin - 加密文件系统

**云存储插件（可选特性）**：
- ✅ S3FsPlugin - AWS S3
- ✅ SqlfsPlugin - SQL 存储
- ✅ GptfsPlugin - GPT 集成
- ✅ VectorFsPlugin - 向量存储
- ✅ StreamRotateFSPlugin - 流轮转

**OpenDAL 统一插件**：
- ✅ OpendalPlugin - 统一存储接口
- ✅ S3FsPlugin (OpenDAL) - S3 支持
- ✅ AzureBlobFsPlugin - Azure Blob
- ✅ GcsFsPlugin - Google Cloud Storage
- ✅ AliyunOssFsPlugin - 阿里云 OSS
- ✅ TencentCosFsPlugin - 腾讯云 COS
- ✅ HuaweiObsFsPlugin - 华为云 OBS
- ✅ MinioFsPlugin - MinIO

**挂载系统**：
- ✅ MountTable - 插件挂载表
- ✅ 最长前缀匹配路由
- ✅ 虚拟符号链接支持
- ✅ 递归符号链接解析
- ✅ 循环检测（MAX_SYMLINK_DEPTH = 40）

### ⚠️ 部分实现

**OpenDAL 扩展**：
- ⚠️ WebDAV、FTP、SFTP 插件暂时禁用（等待 OpenDAL 0.50.x TLS 冲突修复）
- ⚠️ 缓存层需要性能优化

**StreamFsPlugin**：
- ⚠️ WebSocket 支持需要增强
- ⚠️ SSE (Server-Sent Events) 部分实现

### ❌ 未实现

**高级特性**：
- ❌ 插件热重载（需要重启服务）
- ❌ 插件依赖管理（插件间的依赖关系）
- ❌ 插件版本兼容性检查
- ❌ 插件沙箱/安全隔离

**性能优化**：
- ❌ 批量操作优化（批量读写）
- ❌ 并发控制（限制单个插件的并发操作数）
- ❌ 连接池管理（数据库、网络连接）

**监控和诊断**：
- ❌ 插件性能指标（QPS、延迟）
- ❌ 插件健康检查
- ❌ 插件日志和调试工具

### 当前架构路径

**主路径**：REST API → EvifPlugin → MountTable → Storage Plugins

**未使用**：
- VFS 层（FileSystem trait 已定义但未集成到主路径）
- 图集成（PathResolver 的 graph 查询未实现）

### 推荐实践

**开发新插件**：
1. 实现 `EvifPlugin` trait
2. 实现配置验证（validate）
3. 提供 README 文档（get_readme）
4. 定义配置参数（get_config_params）
5. 编写单元测试
6. 编写集成测试（挂载表集成）

**部署插件**：
1. 将插件编译到 `evif-plugins` crate
2. 使用 Cargo features 控制可选依赖
3. 通过 REST API 或配置文件挂载
4. 验证配置参数有效性
5. 监控插件性能和错误

---

## 相关章节

- [第三章：架构设计](chapter-3-architecture.md) - EVIF 整体架构
- [第四章：虚拟文件系统](chapter-4-virtual-filesystem.md) - VFS 抽象层
- [第六章：FUSE 集成](chapter-6-fuse.md) - FUSE 文件系统支持
