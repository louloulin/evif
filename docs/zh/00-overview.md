# EVIF 架构概览

## 1. 核心概念

EVIF (Everything Is a Virtual Filesystem) 是一个基于 Rust 构建的插件驱动虚拟文件系统，为 AI 智能体提供：

- **持久化上下文**: 跨会话保持智能体工作状态
- **可复用技能**: 标准化工作流程
- **多智能体协同**: 基于 PipeFS 的任务分发

核心设计原则：**一切皆为路径**。文件、队列、内存、上下文都通过统一路径接口访问。

## 2. 系统架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         消费层                                       │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐        │
│  │  CLI   │  │ Python │  │  Go    │  │ TS/JS  │  │  MCP   │        │
│  │ evif   │  │  SDK   │  │  SDK   │  │  SDK   │  │ Server │        │
│  └────────┘  └────────┘  └────────┘  └────────┘  └────────┘        │
└─────────────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────────┐
│                         API 层                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                      evif-rest (Axum)                       │    │
│  │                                                              │    │
│  │  文件操作 │ Handle管理 │ 挂载管理 │ 系统操作 │ 内存操作      │    │
│  │  技能执行 │ 管道通信 │ 监控 │ 加密 │ 协作                       │    │
│  └─────────────────────────────────────────────────────────────┘    │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────────┐
│                         核心层                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Radix Mount Table                        │    │
│  │                     O(k) 路径路由                           │    │
│  │                                                              │    │
│  │  /mem ───► memfs     /context ──► contextfs                │    │
│  │  /skills ─► skillfs  /pipes ───► pipefs                    │    │
│  │  /queue ───► queuefs /data ─────► localfs                  │    │
│  │  /memories ─► vectorfs                                    │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  Handle Manager │ Cache Manager │ Circuit Breaker │ Batch Operations │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────────┐
│                        插件层 (40+)                                  │
│                                                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │
│  │  智能体原语  │ │    存储     │ │    云存储    │ │    增强     │   │
│  │  ContextFS   │ │   memfs     │ │    S3FS     │ │  EncryptedFS │   │
│  │  SkillFS    │ │   localfs    │ │    GCSFS    │ │   TieredFS   │   │
│  │  PipeFS     │ │   SQLiteFS   │ │   AzureFS   │ │  StreamRotate│   │
│  │  QueueFS    │ │ PostgreSQLFS  │ │   FTP/SFTP  │ │   SnapshotFS │   │
│  │  VectorFS   │ │             │ │             │ │    QuotaFS    │   │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │
└────────────────────────────────────────────────────────────────────┘
```

## 3. Radix Mount Table

路径路由使用基数树（Radix Tree）实现，复杂度 O(k)，k 为路径深度：

```rust
// 挂载点示例
/mem       → memfs        (内存文件系统)
/context/* → contextfs    (持久化上下文)
/skills/*  → skillfs      (可复用技能)
/pipes/*   → pipefs       (多智能体管道)
/queue/*   → queuefs      (任务队列)
/data      → localfs      (本地存储)
/s3        → s3fs         (S3 云存储)

# 自定义挂载
/mongodb   → mongo://localhost:27017
/postgres  → postgresql://user:pass@localhost/db
/sftp      → sftp://remote.host/path
```

### 路径解析流程

```
请求: /context/L0/current
        │
        ▼
  ┌─────────────┐
  │ Radix Tree  │
  │ /context/* │ ──► contextfs.resolve("L0/current")
  └─────────────┘
        │
        ▼
  返回文件内容或操作结果
```

## 4. 插件类型

### 4.1 智能体原语

| 插件 | 路径前缀 | 功能 |
|------|----------|------|
| ContextFS | `/context/` | 三层上下文 (L0/L1/L2) |
| SkillFS | `/skills/` | 可复用 SKILL.md 工作流 |
| PipeFS | `/pipes/` | 多智能体消息传递 |
| QueueFS | `/queue/` | 任务队列 (enqueue/dequeue) |
| VectorFS | `/memories/` | 向量内存搜索 |

### 4.2 存储插件

| 插件 | 类型 | 描述 |
|------|------|------|
| memfs | 内存 | 进程内存储，临时数据 |
| localfs | 本地磁盘 | 持久化到文件系统 |
| SQLiteFS | 数据库 | SQLite 数据库存储 |
| PostgreSQLFS | 数据库 | PostgreSQL 存储 |
| S3FS | 云 | Amazon S3 兼容存储 |
| GCSFS | 云 | Google Cloud Storage |
| AzureFS | 云 | Azure Blob Storage |
| FTPFS/SFTPFS | 远程 | FTP/SFTP 协议 |

### 4.3 增强插件

| 插件 | 功能 |
|------|------|
| EncryptedFS | 透明加密 |
| TieredFS | 自动分层存储 |
| StreamRotateFS | 日志轮转 |
| SnapshotFS | 快照备份 |
| QuotaFS | 存储配额 |

## 5. 插件架构

```rust
// 插件 trait 定义
#[async_trait]
pub trait EvifPlugin: Send + Sync {
    async fn create(&self, path: &Path, options: CreateOptions) -> EvifResult<FileHandle>;
    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes>;
    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> EvifResult<u64>;
    async fn readdir(&self, path: &Path, offset: u64) -> EvifResult<Vec<DirEntry>>;
    async fn stat(&self, path: &Path) -> EvifResult<Metadata>;
    async fn remove(&self, path: &Path) -> EvifResult<()>;
    async fn rename(&self, old: &Path, new: &Path) -> EvifResult<()>;
    async fn remove_all(&self, path: &Path) -> EvifResult<u32>;
}
```

### 生命周期

```
启动 → 注册插件 → 解析挂载点 → 处理请求 → 优雅关闭
         │              │
         ▼              ▼
    PluginRegistry   MountTable
```

## 6. Handle 系统

Handle 是文件操作的轻量级句柄，支持租约机制：

```rust
// Handle 结构
struct FileHandle {
    id: u64,
    plugin: Arc<dyn EvifPlugin>,
    path: PathBuf,
    flags: OpenFlags,
    lease: Lease,
    position: u64,
}

// 租约机制
struct Lease {
    expires: Instant,
    duration: Duration,
}
```

## 7. REST API

106 个端点覆盖所有操作：

| 类别 | 端点数 | 主要操作 |
|------|--------|----------|
| 文件操作 | 20+ | read, write, mkdir, rm, mv, cp, stat |
| Handle 操作 | 10+ | open, read, write, seek, close, renew |
| 挂载管理 | 5+ | list, mount, unmount |
| 内存操作 | 5+ | store, search, list, delete |
| 上下文 | 5+ | get, set, search (L0/L1/L2) |
| 技能 | 5+ | list, run, get |
| 管道 | 8+ | create, send, receive, status, claim |
| 系统 | 15+ | health, metrics, status, reset |

## 8. 智能体集成

### 8.1 Claude Code

```bash
# MCP Server 方式 (推荐)
claude mcp add @evif/mcp-server

# preSession hook 方式
# 在 ~/.claude/settings.json 配置
```

### 8.2 上下文工作流

```
会话开始 → 读取 /context/L0/current → 读取 /context/L1/decisions.md
                │                              │
                ▼                              ▼
         恢复上次任务                    查看历史决策
                │
                ▼
         [执行工作]
                │
                ▼
会话结束 → 写入 /context/L0/current → 追加 /context/L1/decisions.md
```

### 8.3 技能执行

```
写入任务 → /skills/code-review/input
                      │
                      ▼
               SkillFS 检测到写入
                      │
                      ▼
              执行 SKILL.md 中的指令
                      │
                      ▼
              输出结果 → /skills/code-review/output
```

## 9. 性能特性

| 特性 | 实现 |
|------|------|
| 路径解析 | O(k) Radix Tree |
| 缓存 | 多级缓存 + 一致性 |
| 并发 | Tokio 异步运行时 |
| 连接池 | HTTP 连接复用 |
| 限流 | Circuit Breaker 模式 |

## 10. 项目统计

| 指标 | 数值 |
|------|------|
| 代码行数 (Rust) | ~45,000 |
| Crate 数量 | 12 |
| 核心模块 | 26 |
| 插件数量 | 40+ |
| REST 端点 | 106 |
| SDK 语言 | Python, TypeScript, Go |

### 目录结构

```
evif/
├── Cargo.toml              # 工作空间定义
├── crates/
│   ├── evif-core/         # 核心引擎 (~26 模块, ~7,500 LOC)
│   ├── evif-plugins/      # 插件实现 (~40 插件, ~19,000 LOC)
│   ├── evif-rest/         # REST API 服务器 (~4,500 LOC)
│   ├── evif-cli/          # CLI 工具 (~3,000 LOC)
│   ├── evif-python/       # Python SDK (~2,000 LOC)
│   ├── evif-sdk-ts/       # TypeScript SDK
│   ├── evif-sdk-go/       # Go SDK
│   └── evif-mcp/          # MCP 服务器
├── tests/                  # 集成测试
├── demos/                  # 演示应用
└── docs/                   # 文档 (含 zh/ 中文)
    └── zh/                 # 中文文档
```

## 11. 相关文档

- [核心模块](01-core-modules.md)
- [插件系统](02-plugin-system.md)
- [REST API 参考](03-rest-api.md)
- [智能体集成](05-agent-integration.md)