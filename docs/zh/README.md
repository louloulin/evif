# EVIF 文档

> 万物皆为虚拟文件系统 — 为 AI 智能体提供持久化上下文、可复用技能和多智能体协同。

## 快速开始

```bash
# 1. 安装
cargo build --release

# 2. 启动服务器 (AI 智能体的 Docker)
./target/release/evif-rest --port 8081 --auth-mode disabled

# 3. 使用 CLI (类似 docker run for AI)
evif health
evif mkdir /mem/demo
evif write -c "Hello EVIF" /mem/demo/intro.txt

# 4. 通过 SDK 访问
pip install -e crates/evif-python
python -c "from evif import Client; c = Client('http://localhost:8081'); print(c.health())"
```

## 文档章节

### [1. 架构概览](00-overview.md)
系统架构、插件系统、挂载表和项目统计。

**从这里开始** 了解 EVIF 如何工作。

### [2. 核心模块](01-core-modules.md)
EVIF Core 引擎内部 — 26 个模块涵盖：
- 插件 trait 和生命周期
- Radix Mount Table (O(k) 路径路由)
- Handle Manager Lease 资源管理
- Cache Manager、Circuit Breaker、Batch Operations

**深入技术理解请阅读此章节**。

### [3. 插件系统](02-plugin-system.md)
40+ 插件实现：
- **智能体原语**: ContextFS, SkillFS, PipeFS, QueueFS, VectorFS
- **存储**: memfs, localfs, S3FS, GCSFS, AzureFS, SQLiteFS, PostgreSQLFS
- **增强**: EncryptedFS, TieredFS, StreamRotateFS

**包含 SKILL.md 格式规范** 用于可复用工作流。

### [4. REST API 参考](03-rest-api.md)
106 个端点，14 个类别：
- 文件操作、Handle 操作、挂载管理
- 内存操作、上下文操作、技能操作
- 系统操作、监控、加密、协作

**开发者集成 EVIF 的 API 参考**。

### [5. SDK 集成](04-sdk-integration.md)
- **Python SDK**: 完整 async/sync 客户端、Memory/Queue mixins
- **TypeScript SDK**: 类型安全客户端
- **Go SDK**: 符合 Go 习惯的接口
- **MCP Server**: 13 个工具用于 Claude Code 原生集成

**SDK 开发从这里开始**。

### [6. 智能体集成](05-agent-integration.md)
AI 智能体平台集成指南：
- **Claude Code**: MCP Server + preSession hook 配置
- **Codex**: Python SDK 插件
- **OpenClaw**: PipeFS 用于多智能体协同

**包含智能体项目的 CLAUDE.md 配置**。

### [7. 部署与运维](06-deployment.md)
- 本地、Docker、Docker Compose、Kubernetes 部署
- 配置、健康检查、监控
- 安全、备份恢复、性能调优

**DevOps 和生产部署**。

### [8. 开发者指南](07-developer-guide.md)
- 构建、测试、lint 命令
- 创建新插件
- 添加 REST 端点
- 测试策略
- 性能基准测试
- 发布流程

**贡献者和插件作者**。

## 快速参考

### CLI 命令

| 命令 | 描述 | 示例 |
|------|------|------|
| `evif health` | 检查服务器状态 | `evif health` |
| `evif ls <路径>` | 列出目录 | `evif ls /mem` |
| `evif cat <路径>` | 读取文件 | `evif cat /context/L0/current` |
| `evif write -c <文本> <路径>` | 写入文件 | `evif write -c "task" /context/L0/current` |
| `evif mkdir <路径>` | 创建目录 | `evif mkdir /pipes/my-task` |
| `evif rm <路径>` | 删除文件/目录 | `evif rm /mem/test.txt` |
| `evif stat <路径>` | 文件元数据 | `evif stat /mem/test.txt` |
| `evif mounts` | 列出挂载 | `evif mounts` |

### REST API 基础

```
http://localhost:8081/api/v1
```

### 默认挂载

| 挂载 | 插件 | 用途 |
|------|------|------|
| `/mem` | memfs | 内存文件系统 |
| `/context` | contextfs | 持久化上下文 (L0/L1/L2) |
| `/skills` | skillfs | 可复用工作流 |
| `/pipes` | pipefs | 多智能体协同 |
| `/queue` | queuefs | 任务队列 |
| `/data` | localfs | 本地磁盘存储 |

### 环境变量

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `EVIF_REST_PORT` | `8081` | HTTP 端口 |
| `EVIF_REST_AUTH_MODE` | `api-key` | 认证模式 |
| `EVIF_API_KEY` | - | API 密钥 |
| `EVIF_LOG_DIR` | `logs` | 日志目录 |

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      消费层                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  CLI     │  │   SDK    │  │  MCP     │  │   REST   │     │
│  │  evif    │  │ Python   │  │ Server   │  │   API    │     │
│  └──────────┘  │ TypeScript│  │          │  │  (106)   │     │
│                │ Go       │  │ Claude   │  │          │     │
│                └──────────┘  │ Code     │  │          │     │
│                              └──────────┘  └──────────┘     │
└───────────────────────────────┬───────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────┐
│                       API 层                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    evif-rest                          │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐   │  │
│  │  │  File   │ │ Handle  │ │ Mount   │ │ System  │   │  │
│  │  │ Handler │ │ Manager │ │ Manager │ │ Handlers│   │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────────────────────────┬───────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────┐
│                      EVIF Core                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Radix Mount Table (O(k))                │  │
│  │                                                       │  │
│  │  /mem ──────► memfs                                 │  │
│  │  /context ──► contextfs                             │  │
│  │  /skills ───► skillfs                               │  │
│  │  /pipes ────► pipefs                                │  │
│  │  /data ─────► localfs                               │  │
│  │  /s3 ───────► s3fs (可配置)                          │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          │
│  │ Handle  │ │  Cache  │ │Circuit  │ │  Batch  │          │
│  │ Manager │ │ Manager │ │ Breaker │ │   Ops   │          │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘          │
└───────────────────────────────┬───────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────┐
│                    插件层 (40+)                            │
│                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          │
│  │ Agent   │ │ Storage │ │ Cloud   │ │Enhance  │          │
│  │ContextFS│ │ memfs   │ │ S3FS    │ │Encrypted│          │
│  │SkillFS  │ │localfs  │ │ GCSFS   │ │ Tiered  │          │
│  │PipeFS   │ │ SQLite  │ │ AzureFS │ │Snapshot │          │
│  │QueueFS  │ │Postgres │ │ FTP/SFTP│ │ Quota   │          │
│  │VectorFS │ │         │ │         │ │         │          │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘          │
└─────────────────────────────────────────────────────────────┘
```

## 代码示例

### Python SDK

```python
from evif import Client

client = Client("http://localhost:8081")

# 文件操作
client.mkdir("/mem/demo")
client.write("/mem/demo/data.txt", "Hello!")
print(client.cat("/mem/demo/data.txt"))

# 智能体上下文
client.write("/context/L0/current", "Implementing auth module")
decisions = client.cat("/context/L1/decisions.md")

# 内存搜索
client.memory_store("User prefers dark mode", modality="preference")
results = client.memory_search("editor theme")

# 任务队列
import json
client.write("/queue/tasks/enqueue", json.dumps({"type": "review"}))
```

### CLI 工作流

```bash
# 智能体会话开始
evif cat /context/L0/current          # 我在做什么？
evif cat /context/L1/decisions.md     # 做了哪些决定？

# 智能体工作
evif write -c "JWT implementation" /context/L0/current
evif mkdir /pipes/review-task
evif write -c "Review auth module" /pipes/review-task/input

# 智能体会话结束
evif write -c "Auth complete, testing" /context/L0/current
```

### MCP Server 工具 (Claude Code)

```json
// Claude Code 自动获得这些工具
evif_context_get(layer: "L0" | "L1" | "L2")
evif_context_set(layer, content, append?)
evif_ls(path)
evif_cat(path)
evif_write(path, content)
evif_skill_list()
evif_skill_run(name, input)
evif_memory_search(query, limit?)
evif_pipe_create(name)
evif_pipe_send(name, data)
```

## 相关文档

- [快速开始](../GETTING_STARTED.md)
- [CLI 参考](../cli-mode.md)
- [MCP Server 配置](../mcp-server.md)
- [指标指南](../metrics.md)
- [生产部署](../production-deployment.md)

## 项目信息

- **仓库**: https://github.com/evif/evif
- **版本**: 0.1.0
- **语言**: Rust
- **代码行数**: ~45,000
- **Crate 数量**: 10+
- **插件数量**: 40+

### 目录结构

```
evif/
├── crates/
│   ├── evif-core/         # 核心引擎 (26 个模块)
│   ├── evif-plugins/       # 插件实现 (40+)
│   ├── evif-rest/          # REST API 服务器
│   ├── evif-cli/           # CLI 工具
│   ├── evif-python/        # Python SDK
│   ├── evif-sdk-ts/        # TypeScript SDK
│   ├── evif-sdk-go/        # Go SDK
│   └── evif-mcp/           # MCP 服务器
├── tests/                  # 集成测试
├── demos/                  # 演示应用
└── docs/                   # 文档 (含 zh/ 中文)
    └── zh/                 # 中文文档
```

---

**问题?** 在 https://github.com/evif/evif/issues 提出 issue