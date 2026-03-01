# 第三章：架构设计

## 目录

- [系统架构概述](#系统架构概述)
- [核心组件](#核心组件)
- [插件系统设计](#插件系统设计)
- [挂载表与路径解析](#挂载表与路径解析)
- [数据流与请求处理](#数据流与请求处理)
- [技术栈](#技术栈)

---

## 系统架构概述

### 双技术栈架构

EVIF 采用了独特的双技术栈架构，两条技术路线并行存在，各自服务于不同的使用场景：

#### 1. 插件 + REST 技术栈（当前主路径）

这是 **EVIF 当前实际运行的主要技术路径**，提供了完整的文件系统操作能力：

- **核心组件**：evif-core、evif-plugins、evif-rest
- **访问方式**：HTTP REST API + WebSocket
- **主要用途**：Web UI（evif-web）、CLI 客户端、第三方集成
- **实现状态**：✅ **完整实现并运行**

**主要特性**：
- 基于插件系统的文件操作（create、read、write、delete、rename 等）
- RESTful API 设计，支持标准 HTTP 方法
- WebSocket 终端，提供交互式命令行体验
- 支持多种存储后端（内存、本地、云存储等）

#### 2. 图 + VFS 技术栈（规划中/部分实现）

这是为未来扩展设计的架构，目前部分组件已实现但未完全集成：

- **核心组件**：evif-graph、evif-vfs、evif-storage、evif-auth、evif-runtime、evif-fuse、evif-grpc
- **设计目标**：图查询引擎、虚拟文件系统抽象、分布式存储、身份认证与授权
- **实现状态**：⚠️ **部分实现，未完全集成**

**当前状态**：
- 图 API 路由存在但返回 "not implemented"
- VFS 抽象层已定义但 REST 主路径未使用
- FUSE 支持已实现（evif-fuse），可独立运行
- gRPC 服务已禁用

### 架构对比表

| 特性 | 插件 + REST 栈 | 图 + VFS 栈 |
|------|---------------|-------------|
| **当前状态** | ✅ 生产就绪 | ⚠️ 规划中 |
| **主要入口** | evif-rest (HTTP 8081) | evif-grpc (已禁用) |
| **文件操作** | EvifPlugin trait | VFS 抽象层 |
| **查询能力** | 路径匹配 | 图查询（未实现） |
| **存储** | 插件自管理 | 统一存储层 |
| **认证** | 无 | evif-auth（未集成） |
| **配置** | 硬编码挂载 | evif-runtime（未使用） |

### 关键设计决策

**为什么采用双技术栈？**

1. **渐进式演进**：插件系统提供即时可用性，图引擎支持复杂查询
2. **灵活性**：不同场景可选择不同技术栈
3. **向后兼容**：保留简单路径的同时支持高级功能

**当前推荐使用**：
- **Web 应用开发**：使用插件 + REST 栈
- **命令行工具**：使用 CLI + REST API
- **FUSE 挂载**：使用 evif-fuse（基于插件栈）
- **图查询**：等待 VFS 栈实现完成

---

## 核心组件

EVIF 由 19 个独立的 crate 组成，每个 crate 负责特定功能。以下是详细分析：

### 1. evif-core

**职责**：核心抽象和基础设施

- **plugin.rs**：定义 `EvifPlugin` trait，包含所有文件系统操作接口
- **radix_mount_table.rs**：`RadixMountTable` 实现，路径挂载与查找
- **handle_manager.rs**：文件句柄全局管理
- **batch_operations.rs**：批量文件操作（复制、删除）
- **cache/**：元数据和目录缓存抽象

**关键类型**：
```rust
pub trait EvifPlugin: Send + Sync {
    fn create(&self, path: &str, typ: FileType) -> Result<()>;
    fn read(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>>;
    fn write(&self, path: &str, offset: u64, data: Vec<u8>) -> Result<u64>;
    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>>;
    fn stat(&self, path: &str) -> Result<FileInfo>;
    fn remove(&self, path: &str) -> Result<()>;
    fn rename(&self, from: &str, to: &str) -> Result<()>;
    // ... 更多方法
}

pub struct RadixMountTable {
    // 基于前缀树的挂载点管理
}
```

**可选特性**：
- `extism_plugin`：WASM 插件支持（需 feature flag）
- 配置验证、监控、审计日志等基础设施

### 2. evif-plugins

**职责**：实现 EvifPlugin trait 的各种存储后端

**默认启用的插件**：
- **MemFsPlugin**：内存文件系统（/mem）
- **HelloFsPlugin**：演示插件（/hello）
- **LocalFsPlugin**：本地文件系统访问（/local）

**其他已实现插件**：
- **云存储**：s3fs、azureblobfs、gcsfs、aliyunossfs、tencentcosfs、huaweiobsfs、miniofs
- **对象存储**：opendal（统一适配层）
- **数据库**：sqlfs、kvfs
- **特殊用途**：streamfs、queuefs、httpfs、proxyfs、devfs、heartbeatfs、handlefs
- **高级功能**：vectorfs（向量存储）、gptfs（AI 集成）、encryptedfs（加密）
- **网络协议**：webdavfs、ftpfs、sftpfs
- **存储分层**：tieredfs（多级缓存）

**插件特性**：
- 所有插件实现相同的 `EvifPlugin` trait
- 支持可选接口：`HandleFS`（文件句柄）、`Streamer`（流式传输）
- 配置验证通过 `Configurable` trait
- 部分插件需要 feature flag 编译

### 3. evif-rest

**职责**：HTTP REST API 服务器

**入口点**：`crates/evif-rest/src/main.rs`

**启动流程**：
```rust
// 1. 创建挂载表
let mount_table = RadixMountTable::new();

// 2. 挂载默认插件（硬编码）
mount_table.mount("/mem", Box::new(MemFsPlugin::new()));
mount_table.mount("/hello", Box::new(HelloFsPlugin::new()));
mount_table.mount("/local", Box::new(LocalFsPlugin::new("/tmp/evif")));

// 3. 启动 HTTP 服务器（默认 8081 端口）
EvifServer::run(mount_table).await?;
```

**API 路由分层**：

#### 3.1 Web 兼容 API (`/api/v1/fs/*`)
- `GET /api/v1/fs/list?path=` - 列出目录
- `GET /api/v1/fs/read?path=` - 读取文件
- `POST /api/v1/fs/write?path=` - 写入文件
- `POST /api/v1/fs/create` - 创建文件/目录
- `DELETE /api/v1/fs/delete?path=` - 删除

**用途**：evif-web 前端使用

#### 3.2 CLI 标准 API (`/api/v1/files`, `/api/v1/directories`)
- `GET /api/v1/directories` - 目录列表
- `GET /api/v1/files` - 读取文件
- `PUT /api/v1/files` - 写入文件
- `POST /api/v1/stat` - 文件元数据
- `POST /api/v1/rename` - 重命名
- `POST /api/v1/grep` - 内容搜索
- `POST /api/v1/digest` - 文件摘要
- `POST /api/v1/touch` - 创建空文件

**用途**：evif-client（CLI）使用

#### 3.3 挂载管理 API
- `GET /api/v1/mounts` - 列出挂载点 ✅ 已实现
- `POST /api/v1/mount` - 动态挂载 ❌ 未实现
- `DELETE /api/v1/unmount` - 卸载 ❌ 未实现

#### 3.4 插件管理 API
- `GET /api/v1/plugins` - 列出插件 ✅
- `GET /api/v1/plugins/list` - 插件详情 ✅
- `POST /api/v1/plugins/load` - 加载插件 ❌
- `POST /api/v1/plugins/wasm/load` - 加载 WASM 插件 ⚠️ 部分
- `DELETE /api/v1/plugins/unload` - 卸载插件 ❌

#### 3.5 句柄 API (`/api/v1/handles/*`)
- `POST /api/v1/handles/open` - 打开文件句柄
- `GET /api/v1/handles/read` - 读取句柄
- `PUT /api/v1/handles/write` - 写入句柄
- `POST /api/v1/handles/seek` - 定位句柄
- `POST /api/v1/handles/sync` - 同步句柄
- `DELETE /api/v1/handles/close` - 关闭句柄

**要求**：插件需实现 `HandleFS` trait

#### 3.6 指标 API (`/api/v1/metrics/*`)
- `GET /api/v1/metrics/traffic` - 流量统计
- `GET /api/v1/metrics/operations` - 操作计数
- `GET /api/v1/metrics/status` - 系统状态
- `POST /api/v1/metrics/reset` - 重置指标

#### 3.7 图 API（占位）
- `GET /nodes/*` - 图节点查询 ❌
- `POST /query` - 图查询 ❌
- `GET /stats` - 图统计 ❌

**返回**："Graph functionality not implemented"

#### 3.8 WebSocket 终端 (`/ws`)
**支持命令**：
- `ls [path]` - 列出目录
- `cat [path]` - 显示文件内容
- `stat [path]` - 文件状态
- `mounts` - 列出挂载点
- `pwd` - 当前路径
- `echo [text]` - 回显文本
- `clear` - 清屏
- `help` - 帮助信息

**用途**：evif-web 终端组件

### 4. evif-web

**职责**：基于 React + TypeScript 的 Web UI

**技术栈**：
- React 18 + TypeScript
- Vite（开发服务器）
- Monaco Editor（代码编辑器）
- TailwindCSS（样式）
- WebSocket API（终端通信）

**已实现功能**：
- 文件树浏览与展开
- Monaco 编辑器集成
- 文件 CRUD 操作
- WebSocket 终端
- 上下文菜单（右键）
- 菜单栏（新建、刷新、保存）

**未接入功能**：
- 插件管理器 UI（PluginManager、MountModal、PluginModal）
- 监控面板（SystemStatus、TrafficChart、LogViewer）
- 协作功能（评论、权限、分享）
- 高级编辑器功能（MiniMap、QuickOpen）
- 搜索与上传

**API 对接**：
- 主要使用 `/api/v1/fs/*` 兼容 API
- WebSocket 连接到 `/ws`
- Vite 代理配置：`/api` → `http://localhost:8081`

### 5. evif-client

**职责**：CLI 的 HTTP 客户端

**用途**：evif-cli 的网络层，通过 HTTP 调用 REST API

**方法映射**：
```rust
// CLI 命令 → HTTP API
ls(path)         → GET /api/v1/directories?path=
cat(path)        → GET /api/v1/files?path=
write(path, data) → PUT /api/v1/files?path=
mkdir(path)      → POST /api/v1/directories
remove(path)     → DELETE /api/v1/files?path=
stat(path)       → POST /api/v1/stat
mounts()         → GET /api/v1/mounts
```

**已知问题**：
- **格式不一致**：client 期望 base64 编码的响应，handlers 返回明文 JSON
- **影响**：CLI `cat` 命令可能无法正确解析文件内容

### 6. evif-cli

**职责**：命令行界面工具

**入口**：`crates/evif-cli/src/main.rs`

**命令解析**：使用 `clap` 框架

**主要命令**：
- `evif ls [path]` - 列出目录
- `evif cat [path]` - 显示文件内容
- `evif cp <src> <dst>` - 复制文件
- `evif stats` - 显示统计信息
- `evif repl` - 进入交互式模式
- `evif mount <path>` - FUSE 挂载
- `evif get <url> <path>` - 下载文件到 EVIF

**默认配置**：
- `--server localhost:50051`（名称类似 gRPC，实际使用 HTTP）
- 实际通过 evif-client 调用 REST API（默认 8081 端口）

### 7. evif-fuse

**职责**：FUSE 文件系统实现

**技术**：基于 `fuser` crate（Rust FUSE 绑定）

**入口**：`evif-fuse-mount` 二进制文件

**工作原理**：
```rust
// FUSE 挂载流程
let mount_table = create_mount_table(); // 与 evif-rest 相同的挂载表
let fuse = EvifFuse::new(mount_table);
fuse.mount("/mnt/evif")?; // 挂载到本地目录
```

**特点**：
- 复用 evif-core 的 `RadixMountTable` 和 `EvifPlugin` 抽象
- 与 evif-rest 共享插件模型，但独立进程运行
- 支持 POSIX 文件系统语义
- 缓存策略：元数据缓存、目录缓存

**使用场景**：
- 将 EVIF 挂载为本地文件系统
- 与标准 Unix 工具集成（ls、cp、mv 等）
- 提供透明文件访问

**限制**：
- 需要单独进程运行
- 不与 evif-rest 共享挂载表状态
- FUSE 权限要求（Linux/macOS）

### 8. evif-graph

**职责**：图数据结构与算法

**核心类型**：
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
    // 图查询 DSL
}

pub struct Executor {
    // 查询执行引擎
}
```

**当前状态**：
- ✅ 图数据结构已实现
- ✅ 查询语言解析器已实现
- ❌ REST API 集成未完成
- ❌ 实际使用场景未明确

**被引用处**：
- evif-storage（图持久化）
- evif-vfs（图查询）
- evif-protocol（图协议）
- evif-client（图客户端类型）
- evif-cli（图命令）
- evif-rest、evif-fuse（依赖但未使用）

**建议**：
- 如果不需要图查询功能，可移除相关依赖
- 如果需要，完成 REST API 集成并提供使用示例

### 9. evif-vfs

**职责**：POSIX 风格虚拟文件系统抽象

**核心概念**：
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

**设计目标**：
- 统一的 VFS 抽象层
- 支持多种文件系统后端
- POSIX 兼容性

**当前状态**：
- ✅ VFS 抽象已定义
- ❌ evif-rest 主路径未使用
- ❌ evif-fuse 直接使用 EvifPlugin，未通过 VFS

**与 EvifPlugin 的关系**：
- `EvifPlugin`：路径字符串操作，更简单
- `Vfs`：基于 INode 的 POSIX 抽象，更底层

### 10. evif-storage

**职责**：可插拔存储后端

**存储类型**：
```rust
pub enum StorageBackend {
    Memory(MemoryStorage),
    Sled(SledStorage),
    RocksDB(RocksDBStorage),
    S3(S3Storage),
    Custom(Box<dyn CustomStorage>),
}
```

**用途**：
- 图数据持久化（evif-graph）
- 元数据存储（插件可选）
- 运行时状态（evif-runtime）

**当前状态**：
- ✅ 多种存储后端已实现
- ⚠️ REST 主路径未直接使用
- ⚠️ 部分插件可能依赖

### 11. evif-auth

**职责**：身份认证与授权

**功能**：
- JWT 令牌验证
- 基于角色的访问控制（RBAC）
- 审计日志
- 能力检查

**当前状态**：
- ✅ 认证框架已实现
- ❌ evif-rest 未集成
- ❌ API 无认证中间件

### 12. evif-runtime

**职责**：运行时配置与编排

**功能**：
- 配置文件加载（TOML/YAML/JSON）
- 插件动态加载
- 服务生命周期管理
- 健康检查

**当前状态**：
- ✅ 运行时框架已实现
- ❌ evif-rest 使用硬编码配置
- ❌ 未与配置文件集成

### 13. evif-protocol

**职责**：消息协议定义

**内容**：
- 请求/响应消息类型
- 编解码（JSON/MessagePack）
- 图查询协议

**使用方**：
- evif-client（图操作客户端）
- evif-grpc（gRPC 服务）

**当前状态**：
- ✅ 协议定义完成
- ⚠️ 主要用于图相关功能
- ❌ REST API 未使用

### 14. evif-grpc

**职责**：gRPC 服务实现

**当前状态**：
- ❌ **服务端已禁用**
- ✅ Protocol Buffers 定义存在
- ✅ gRPC 客户端代码生成

**禁用原因**：
- 可能处于开发早期
- 或优先使用 REST API

**未来用途**：
- 高性能 RPC 通信
- 流式数据传输
- 多语言客户端支持

### 15. evif-mcp

**职责**：Model Context Protocol (MCP) 服务器

**功能**：
- 通过 HTTP 调用 EVIF API
- 为 AI 助手提供文件系统工具
- 支持工具列表、工具调用

**配置**：
- `EVIF_URL` 环境变量（默认 `http://localhost:8080`）
- 应指向 evif-rest 服务（8081）

**工具列表**（17 个）：
- `evif_read_file` - 读取文件
- `evif_write_file` - 写入文件
- `evif_list_directory` - 列出目录
- `evif_create_directory` - 创建目录
- `evif_delete` - 删除文件/目录
- `evif_rename` - 重命名
- `evif_stat` - 文件状态
- `evif_copy` - 复制
- `evif_search` - 搜索文件
- `evif_mount_list` - 列出挂载点
- `evif_mount_create` - 创建挂载
- `evif_mount_delete` - 删除挂载
- `evif_health_check` - 健康检查
- `evif_grep` - 内容搜索
- `evif_digest` - 文件摘要
- `evif_batch_delete` - 批量删除
- `evif_touch` - 创建空文件

### 16. evif-macros

**职责**：过程宏支持

**宏定义**：
- `#[evif_plugin]` - 自动实现 EvifPlugin trait
- `#[configurable]` - 自动实现 Configurable trait
- `#[metric]` - 指标收集

**用途**：
- 简化插件开发
- 减少重复代码
- 编译时类型检查

### 17. evif-metrics

**职责**：指标类型定义

**指标类型**：
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

**用途**：
- `/api/v1/metrics/*` API 响应
- 监控面板数据源
- 性能分析

### 18. evif-python

**职责**：Python 绑定

**功能**：
- Python 客户端库
- 异常类型定义
- 数据模型绑定

**使用 PyO3**：
```python
import evif

client = evif.Client("http://localhost:8081")
files = client.list_directory("/mem")
content = client.read_file("/mem/test.txt")
```

**当前状态**：
- ✅ Python 绑定已实现
- ⚠️ 需验证与 REST API 兼容性

### 19. 辅助 Crate

**example-dynamic-plugin**：动态插件示例
**tests/**：集成测试
- e2e：端到端测试
- cli：CLI 测试
- api：API 测试
- common：测试工具

---

## 插件系统设计

### EvifPlugin Trait

**核心接口**：

```rust
#[async_trait]
pub trait EvifPlugin: Send + Sync + AsAny {
    // 基础信息
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    // 文件操作
    fn create(&self, path: &str, typ: FileType) -> Result<()>;
    fn read(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>>;
    fn write(&self, path: &str, offset: u64, data: Vec<u8>) -> Result<u64>;
    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>>;
    fn stat(&self, path: &str) -> Result<FileInfo>;
    fn remove(&self, path: &str) -> Result<()>;
    fn rename(&self, from: &str, to: &str) -> Result<()>;
    fn remove_all(&self, path: &str) -> Result<()>;

    // 符号链接
    fn symlink(&self, src: &str, dst: &str) -> Result<()>;
    fn readlink(&self, path: &str) -> Result<String>;

    // 可选接口
    fn as_handle_fs(&self) -> Option<&dyn HandleFS> {
        None
    }

    fn as_streamer(&self) -> Option<&dyn Streamer> {
        None
    }
}
```

### 插件类型分类

#### 1. 内存型插件
- **MemFs**：纯内存存储
- **HelloFs**：演示插件
- **DevFs**：开发信息
- **ServerInfoFs**：服务器状态

#### 2. 本地存储插件
- **LocalFs**：本地文件系统
- **TieredFs**：多级缓存

#### 3. 云存储插件
- **S3Fs**：AWS S3
- **AzureBlobFs**：Azure Blob Storage
- **GcsFs**：Google Cloud Storage
- **AliyunOssFs**：阿里云 OSS
- **TencentCosFs**：腾讯云 COS
- **HuaweiObsFs**：华为云 OBS
- **MinioFs**：MinIO

#### 4. 数据库插件
- **SqlFs**：SQL 数据库
- **Kvfs**：键值存储

#### 5. 特殊用途插件
- **StreamFs**：流式处理
- **QueueFs**：消息队列
- **HttpFs**：HTTP 代理
- **ProxyFs**：反向代理
- **VectorFs**：向量存储
- **GptFs**：AI 集成
- **EncryptedFs**：加密存储

#### 6. 网络协议插件
- **WebdavFs**：WebDAV
- **FtpFs**：FTP
- **SftpFs**：SFTP

### 可选接口

#### HandleFS Trait

**用途**：支持文件句柄操作（类似 POSIX file descriptor）

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

**好处**：
- 支持随机访问
- 减少路径解析开销
- 更接近 POSIX 语义

**使用场景**：
- 大文件读写
- 频繁的小量读写
- 需要文件锁的场景

#### Streamer Trait

**用途**：流式数据传输

```rust
#[async_trait]
pub trait Streamer: Send + Sync {
    fn read_stream(&self, path: &str) -> Result<BoxStream<Vec<u8>>>;
    fn write_stream(&self, path: &str) -> Result<BoxSink<Vec<u8>>>;
}
```

**好处**：
- 支持大文件流式传输
- 减少内存占用
- 实时数据处理

**使用场景**：
- 视频流
- 日志流
- 实时数据采集

### 插件配置与验证

```rust
pub trait Configurable: EvifPlugin {
    fn validate_config(&self, config: &Value) -> Result<()>;
    fn apply_config(&mut self, config: &Value) -> Result<()>;
    fn default_config(&self) -> Value;
}
```

**配置示例**（S3Fs）：
```json
{
  "access_key": "AKIAIOSFODNN7EXAMPLE",
  "secret_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
  "bucket": "my-bucket",
  "region": "us-east-1",
  "endpoint": "https://s3.amazonaws.com"
}
```

### 插件开发示例

**简单插件实现**：

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
        // 实现创建逻辑
        Ok(())
    }

    fn read(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>> {
        // 实现读取逻辑
        Ok(Vec::new())
    }

    fn write(&self, path: &str, offset: u64, data: Vec<u8>) -> Result<u64> {
        // 实现写入逻辑
        Ok(data.len() as u64)
    }

    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>> {
        // 实现目录列表
        Ok(Vec::new())
    }

    fn stat(&self, path: &str) -> Result<FileInfo> {
        // 实现状态查询
        Ok(FileInfo {
            path: path.to_string(),
            typ: FileType::File,
            size: 0,
            modified: SystemTime::now(),
        })
    }

    fn remove(&self, path: &str) -> Result<()> {
        // 实现删除逻辑
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> Result<()> {
        // 实现重命名逻辑
        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<()> {
        // 实现递归删除
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

## 挂载表与路径解析

### RadixMountTable 设计

**数据结构**：

```rust
pub struct RadixMountTable {
    mounts: RadixTree<Box<dyn EvifPlugin>>,
    default: Option<Box<dyn EvifPlugin>>,
}
```

**核心概念**：
- **前缀树（Radix Tree）**：高效的路径匹配
- **最长前缀匹配**：选择最具体的挂载点
- **O(k) 复杂度**：k 为路径深度

### 路径规范化

**规范化规则**：

1. **去除冗余分隔符**：`/foo//bar` → `/foo/bar`
2. **解析 `.`**：`/foo/./bar` → `/foo/bar`
3. **解析 `..`**：`/foo/../bar` → `/bar`
4. **尾部斜杠**：`/foo/` → `/foo`（除非是根 `/`）
5. **拒绝非法路径**：空路径、相对路径

**示例**：

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

### 挂载点匹配算法

**最长前缀匹配**：

```rust
impl RadixMountTable {
    pub fn lookup(&self, path: &str) -> Result<&dyn EvifPlugin> {
        let normalized = normalize_path(path)?;

        // 在前缀树中查找最长匹配
        let (mount_point, plugin) = self.mounts
            .longest_prefix(&normalized)
            .unwrap_or((&*"/", self.default.as_ref().unwrap()));

        // 验证挂载点存在
        if plugin.is_none() {
            return Err(Error::not_found(path));
        }

        Ok(plugin)
    }
}
```

**匹配示例**：

| 请求路径 | 挂载点 | 匹配插件 | 相对路径 |
|---------|-------|---------|---------|
| `/mem/foo.txt` | `/mem` | MemFsPlugin | `/foo.txt` |
| `/local/subdir/file.txt` | `/local` | LocalFsPlugin | `/subdir/file.txt` |
| `/hello` | `/hello` | HelloFsPlugin | `/` |
| `/unmounted/path` | - | - | 错误 |

### 挂载管理

**挂载操作**：

```rust
let mount_table = RadixMountTable::new();

// 挂载插件
mount_table.mount("/mem", Box::new(MemFsPlugin::new()))?;
mount_table.mount("/local", Box::new(LocalFsPlugin::new("/data")))?;
mount_table.mount("/s3", Box::new(S3FsPlugin::new(config))?)?;

// 列出挂载点
let mounts = mount_table.list_mounts();
// [("/mem", "MemFsPlugin"), ("/local", "LocalFsPlugin"), ("/s3", "S3FsPlugin")]

// 卸载插件
mount_table.unmount("/mem")?;
```

**限制**：
- **当前版本**：挂载点硬编码在 evif-rest/src/server.rs
- **动态挂载**：API 存在但返回 "not yet supported"
- **未来计划**：通过配置文件或 REST API 管理挂载

---

## 数据流与请求处理

### 启动流程

```mermaid
graph TD
    A[evif-rest::main] --> B[创建 RadixMountTable]
    B --> C[挂载 MemFs 到 /mem]
    B --> D[挂载 HelloFs 到 /hello]
    B --> E[挂载 LocalFs 到 /local]
    C --> F[启动 HTTP 服务器 8081]
    D --> F
    E --> F
    F --> G[监听请求]
```

### Web 请求流

```mermaid
sequenceDiagram
    participant Browser
    participant Vite
    participant RestAPI
    participant MountTable
    participant Plugin

    Browser->>Vite: GET /api/v1/fs/list?path=/mem
    Vite->>RestAPI: 代理到 8081
    RestAPI->>RestAPI: compat_fs::list()
    RestAPI->>MountTable: lookup("/mem")
    MountTable->>Plugin: 返回 MemFsPlugin
    RestAPI->>Plugin: readdir("/mem")
    Plugin->>RestAPI: Vec<FileInfo>
    RestAPI->>Vite: FsListResponse { nodes }
    Vite->>Browser: JSON 响应
```

### CLI 请求流

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
    MountTable->>Plugin: 返回 MemFsPlugin
    RestAPI->>Plugin: readdir("/mem")
    Plugin->>RestAPI: Vec<FileInfo>
    RestAPI->>Client: DirectoryListResponse { files }
    Client->>CLI: 格式化输出
```

### WebSocket 终端流

```mermaid
sequenceDiagram
    participant Terminal
    participant WebSocket
    participant WsHandlers
    participant MountTable
    participant Plugin

    Terminal->>WebSocket: ws://localhost:8081/ws
    WebSocket-->>Terminal: 连接成功

    Terminal->>WebSocket: "ls /mem"
    WebSocket->>WsHandlers: 解析命令
    WsHandlers->>MountTable: lookup("/mem")
    MountTable->>Plugin: 返回 MemFsPlugin
    WsHandlers->>Plugin: readdir("/mem")
    Plugin->>WsHandlers: Vec<FileInfo>
    WsHandlers->>WebSocket: 格式化输出
    WebSocket->>Terminal: "file1.txt\nfile2.txt\n"
```

### FUSE 请求流

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
    MountTable->>Plugin: 返回 MemFsPlugin
    FuseFS->>Plugin: stat("/mem/file.txt")
    Plugin->>FuseFS: FileInfo
    FuseFS->>FUSE: 属性
    FUSE->>FuseFS: read()
    FuseFS->>Plugin: read("/mem/file.txt", 0, 4096)
    Plugin->>FuseFS: Vec<u8>
    FuseFS->>FUSE: 数据
    FUSE->>App: 文件内容
```

### API 路由层次

```
evif-rest (8081)
├─ /health
│  └─ health checker
│
├─ /api/v1/fs/* (Web 兼容 API)
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
├─ /api/v1/handles/* (句柄 API)
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
├─ /nodes/*, /query, /stats (图 API - 占位)
│  └─ "Graph functionality not implemented"
│
└─ /ws (WebSocket 终端)
   └─ ws_handlers
```

---

## 技术栈

### 核心依赖

**异步运行时**：
- **tokio** 1.35 (features = ["full"]) - 异步运行时
- **async-trait** 0.1.77 - 异步 trait 支持

**序列化**：
- **serde** 1.0.196 - 序列化框架
- **serde_json** 1.0.113 - JSON 支持
- **bytes** 1.5.0 - 字节缓冲

**错误处理**：
- **anyhow** 1.0.79 - 错误上下文
- **thiserror** 1.0.56 - 错误类型定义

**图算法**：
- **petgraph** 0.6.4 - 图数据结构与算法

**并发数据结构**：
- **dashmap** 5.5.3 - 并发哈希表
- **ahash** 0.8.7 - 高性能哈希

**工具**：
- **uuid** 1.6.1 - UUID 生成
- **chrono** 0.4.31 - 时间处理
- **config** 0.14.0 - 配置管理

**加密**：
- **blake3** 1.5.0 - 哈希算法
- **ed25519-dalek** 2.1.1 - 签名

**测试**：
- **proptest** 1.4.0 - 属性测试
- **criterion** 0.5.1 - 性能测试
- **tokio-test** 0.4.3 - 异步测试
- **tempfile** 3.8 - 临时文件

### Web 技术

**evif-rest**：
- **axum** (或 actix-web) - Web 框架
- **tokio-tungstenite** - WebSocket 支持
- **tower** - 中间件

**evif-web**：
- **React** 18 - UI 框架
- **TypeScript** - 类型安全
- **Vite** - 构建工具
- **Monaco Editor** - 代码编辑器
- **TailwindCSS** - 样式框架

### FUSE 集成

**fuser** - Rust FUSE 绑定
- 支持 Linux、macOS
- POSIX 文件系统语义
- 高性能内核交互

### 存储后端

**evif-storage**：
- **sled** - 嵌入式数据库
- **rocksdb** - 键值存储
- **rusoto_s3** / **aws-sdk-s3** - AWS S3
- **azure_storage** - Azure Blob
- **google-cloud-storage** - GCS

### 编译配置

**Release 优化**：
```toml
[profile.release]
opt-level = 3        # 最高优化
lto = true          # 链接时优化
codegen-units = 1   # 单编译单元
strip = true        # 移除符号表
```

**Dev 配置**：
```toml
[profile.dev]
opt-level = 0  # 无优化，加快编译
```

---

## 下一章节

- [第四章：虚拟文件系统](./chapter-4-filesystem.md) - VFS 抽象与实现
- [第五章：插件开发](./chapter-5-plugins.md) - 插件开发指南
- [第六章：FUSE 集成](./chapter-6-fuse.md) - FUSE 挂载详解
