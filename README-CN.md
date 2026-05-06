# EVIF - 万物皆为虚拟文件系统

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)
[![Tests](https://img.shields.io/badge/Tests-600+-green.svg)](#测试)
[![Crate](https://img.shields.io/badge/Crates-13-orange.svg)](#核心组件)

> 基于 Rust 构建的上下文导向虚拟文件系统，遵循 Plan 9 "万物皆为文件" 理念，为 AI Agent 提供持久化上下文、可复用技能和多智能体协同。

**文档**: [中文](README-CN.md) | [English](README.md)

## 概览

EVIF 从 "Everything Is a File" 演进为 AI Agent 的 "Context Is a File"：

- **ContextFS** - 分层 `L0/L1/L2` 工作上下文
- **SkillFS** - 标准 `SKILL.md` 技能发现和调用
- **PipeFS** - 轻量级多智能体协同
- **传统 EVIF 插件基础设施**

### Agent 定位

EVIF 为 Agent 提供统一的文件接口：

- `/context` - 活跃上下文 (L0: 当前任务, L1: 会话决策, L2: 项目知识)
- `/skills` - 可复用工作流 (SKILL.md 格式)
- `/pipes` - 任务协同 (多智能体通信)
- `/memories` - 向量记忆 (语义搜索)

### 关键特性

| 特性 | 描述 |
|------|------|
| **13 个核心 Crate** | 120K 行 Rust 代码，架构清晰 |
| **40+ 插件** | 云存储、数据库、AI 服务 |
| **63 个 MCP 工具** | 完整的 AI Agent 工具生态 |
| **150 个 REST 端点** | 全面的 HTTP API |
| **600+ 测试** | 高测试覆盖率 |
| **多智能体协同** | PipeFS 支持 wait_for_result、原子 claim |

## 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         访问层                                    │
│  CLI (60+ 命令) │ REST API (150 端点) │ MCP Server │ FUSE         │
├─────────────────────────────────────────────────────────────────┤
│                         核心层                                    │
│  Mount Table (Radix Tree) │ 插件生命周期 │ Handle 管理            │
├─────────────────────────────────────────────────────────────────┤
│                         插件层                                    │
│  ContextFS │ SkillFS │ PipeFS │ QueueFS │ VectorFS │ Storage     │
└─────────────────────────────────────────────────────────────────┘
```

## 快速开始

### 安装

```bash
# 克隆并构建
git clone https://github.com/evif/evif.git
cd evif
cargo build --release
cargo install --path crates/evif-cli

# 或使用 Homebrew (macOS)
brew tap evif/evif
brew install evif
```

### 启动服务

```bash
evif-rest --port 8081
```

### 基本使用

```bash
# 文件操作
evif ls /                            # 列出根目录
evif mkdir /mem/demo                 # 创建目录
evif write /mem/hello.txt -c "Hi"   # 写入文件
evif cat /mem/hello.txt              # 读取文件
evif mounts                          # 列出挂载

# 健康检查
evif health
```

### 30 秒 Agent 演示

```bash
# 上下文管理
cat /context/L0/current
cat /context/L1/decisions.md
ls /skills

# 多智能体协同
mkdir /pipes/task-001
echo "review code" > /pipes/task-001/input
```

## 核心组件

| Crate | 代码行数 | 描述 |
|-------|----------|------|
| **evif-core** | ~15K | 核心抽象，Mount Table，Handle 管理 |
| **evif-plugins** | ~30K | 40+ 存储插件 (云存储、数据库、AI 服务) |
| **evif-rest** | ~20K | HTTP/JSON REST API 服务器 |
| **evif-cli** | ~15K | 命令行工具 (60+ 命令) |
| **evif-mcp** | ~25K | MCP (Model Context Protocol) 服务器 (63 工具) |
| **evif-mem** | ~20K | 记忆平台，向量搜索 |
| **evif-client** | ~5K | TypeScript/JS 客户端 SDK |
| **evif-metrics** | ~5K | Prometheus 指标 |

## 插件目录

### Agent 原语

| 插件 | 路径 | 功能 |
|------|------|------|
| `contextfs` | `/context` | L0/L1/L2 分层上下文 |
| `skillfs` | `/skills` | SKILL.md 技能发现 |
| `pipefs` | `/pipes` | 多智能体协同 |
| `queuefs` | `/queue` | 任务队列 |
| `vectorfs` | `/memories` | 向量记忆 |
| `memfs` | `/mem` | 内存文件系统 |

### 云存储 (9 个插件)

| 插件 | 服务 |
|------|------|
| `s3fs` | Amazon S3 |
| `gcsfs` | Google Cloud Storage |
| `azureblobfs` | Azure Blob |
| `aliyunossfs` | 阿里云 OSS |
| `tencentcosfs` | 腾讯 COS |
| `huaweiobsfs` | 华为 OBS |
| `miniofs` | MinIO |
| `webdavfs` | WebDAV |
| `httpfs` | HTTP/HTTPS |

### AI 服务集成 (8 个插件)

| 插件 | 服务 |
|------|------|
| `gmailfs` | Gmail (发送、回复、草稿、标签) |
| `slackfs` | Slack (发布消息、反应) |
| `discordfs` | Discord (发送、嵌入、反应) |
| `githubfs` | GitHub |
| `notionfs` | Notion |
| `telegramfs` | Telegram |
| `teamsfs` | Microsoft Teams |
| `shopifyfs` | Shopify |

### 数据库插件

| 插件 | 类型 |
|------|------|
| `sqlfs` | SQLite |
| `postgresfs` | PostgreSQL |

## MCP 服务器

EVIF MCP 服务器为 AI Agent 提供 **63 个工具**，全部使用真实后端调用实现。

### 工具分类

| 分类 | 工具数 | 描述 |
|------|--------|------|
| **核心 FS** | 13 | ls, cat, write, mkdir, rm, cp, mv, grep, tree, diff, du, hash |
| **记忆** | 5 | memorize, retrieve, memory_search, memory_clear, memory_stats |
| **技能** | 4 | skill, skill_list, skill_info, claude_md_generate |
| **会话** | 7 | session, session_load, session_delete, agent, subagent_status |
| **配置** | 3 | config_get, config_list, config_set |
| **定时** | 3 | cron_schedule, cron_list, cron_remove |
| **可观测性** | 11 | health, health_detailed, metrics_export, log_query, cache_stats |
| **插件** | 4 | plugin_catalog, plugin_info, plugin_load, plugin_unload |
| **事件/队列** | 5 | event_list, event_subscribe, queue_list, queue_stats |
| **句柄/管道** | 4 | open_handle, close_handle, pipe_create, pipe_list |
| **工具** | 4 | search, archive, batch, watch |

### Token 优化

| 工具 | 参数 | 预估节省 |
|------|------|----------|
| `evif_cat` | `max_lines` (默认 100), `mode` (head/tail/snippet/full) | 60-90% |
| `evif_memory_search` | `compact` (默认 true), `limit` (默认 3) | 40-60% |
| 输出过滤 | strip_ansi, truncate_lines, compact_json, max_string_length | 30-50% |

## AI 平台集成

EVIF 支持 **5 个 AI 平台**，通过 `evif connect` 命令：

| 平台 | 配置文件 | 方法 |
|------|----------|------|
| Claude Desktop | `claude_desktop_config.json` | MCP JSON |
| Claude Code | `settings.json` | MCP JSON |
| Cursor | `mcp.json` | MCP JSON |
| Gemini CLI | `settings.json` | MCP JSON |
| OpenAI Codex | `AGENTS.md` | 规则文件 |

```bash
# 列出支持的平台
evif connect --list

# 连接到 Claude Desktop
evif connect claude

# 检查集成状态
evif connect --check

# 断开平台连接
evif connect --disconnect claude
```

## 配置

### 环境变量

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `EVIF_REST_PORT` | 8081 | REST 端口 |
| `EVIF_REST_HOST` | 0.0.0.0 | 绑定地址 |
| `EVIF_REST_AUTH_MODE` | api-key | 认证模式 |
| `EVIF_API_KEY` | - | API 密钥 |
| `EVIF_LOG_DIR` | logs | 日志目录 |
| `EVIF_METRICS_ENABLED` | false | 启用 Prometheus |

### 配置文件

```toml
# evif.toml
[server]
port = 8081
host = "0.0.0.0"

[auth]
mode = "capability"

[[mounts]]
path = "/mem"
plugin = "memfs"

[[mounts]]
path = "/context"
plugin = "contextfs"
```

## 测试

```bash
# 完整工作区测试
cargo test --workspace

# 按 crate 测试
cargo test -p evif-core
cargo test -p evif-rest
cargo test -p evif-mcp
cargo test -p evif-plugins
cargo test -p evif-cli

# E2E 测试
cargo test -p evif-rest --test e2e_rest_api
```

### 测试覆盖

| Crate | 单元测试数 |
|-------|-----------|
| evif-mem | 106 |
| evif-mcp | 76 |
| evif-plugins | 66 |
| evif-rest | 61 |
| evif-client | 50 |
| evif-core | 59 |
| evif-metrics | 43 |
| evif-cli | 46 |
| evif-auth | 31 |

## 项目结构

```
evif/
├── crates/
│   ├── evif-core/         # 核心引擎 (Mount Table, Handle 管理)
│   ├── evif-plugins/       # 40+ 插件实现
│   ├── evif-rest/          # REST API 服务器 (150 端点)
│   ├── evif-cli/           # CLI 工具 (60+ 命令)
│   ├── evif-mcp/           # MCP 服务器 (63 工具)
│   ├── evif-mem/           # 记忆平台，向量搜索
│   ├── evif-client/        # TypeScript/JS 客户端
│   ├── evif-metrics/       # Prometheus 指标
│   ├── evif-auth/          # 认证
│   ├── evif-bench/          # 基准测试套件
│   ├── evif-fuse/          # FUSE 集成
│   ├── evif-macros/        # 过程宏
│   └── example-dynamic-plugin/  # 插件示例
├── tests/                  # 集成测试
├── docs/                   # 文档 (包含 zh/)
├── homebrew-formula/       # Homebrew tap
├── mvp4.0.md              # MVP 4.0 路线图
├── mvp4.1.md              # MVP 4.1 路线图
├── CHANGELOG.md           # 版本历史
└── CLAUDE.md              # Claude Code 模板
```

## 性能指标

| 指标 | 数值 |
|------|------|
| 路径解析 | O(k) via Radix Tree |
| Handle 租约 | 资源管理 |
| 多级缓存 | inode + 目录 |
| 批量操作 | 并发复制/删除 |
| 流式处理 | 大文件支持 |

## MCP 服务器实现

EVIF MCP 服务器已 **完全实现**，所有 Mock 实现已替换为真实后端调用。

### 实现阶段

| 阶段 | 状态 | 描述 |
|------|------|------|
| Phase 0 | ✅ | 核心结构 |
| Phase 1 | ✅ | VFS 操作 (L1) |
| Phase 2 | ✅ | HTTP 桥接 (L2) |
| Phase 3 | ✅ | 真实实现 |
| Phase 4 | ✅ | 高级工具 |
| Phase 5 | ✅ | Mock 移除 |
| Phase 6 | ✅ | 性能优化 |
| Phase 7 | ✅ | 测试 |
| Phase 8 | ✅ | 文档 |
| Phase 9 | ✅ | 清理 |

### 架构分层

1. **L1: VFS Backend** - 通过 `VfsBackend` 进行核心文件操作
2. **L2: HTTP Bridge** - 通过后端 API 进行复杂操作
3. **L3: Fallback** - 后端不可用时优雅降级

### 输出过滤流水线

```toml
# 输出过滤配置
strip_ansi = true
max_lines = 200
compact_json = true    # 移除 null 字段
max_string_length = 10000
```

## 多智能体协同

### PipeFS 核心功能

| 功能 | 描述 |
|------|------|
| `try_claim()` | 原子 claim，防止多 Agent 同时占用 |
| `wait_for_result()` | tokio::Notify 阻塞等待，带超时 |
| 状态机 | pending→running→completed，error/timeout→pending 重试 |
| 通知机制 | per-pipe Notify，写 output 时唤醒 |

### 状态转换

```
pending → running → completed
pending → error → pending (重试)
running → error → pending (重试)
running → timeout → pending (重试)
completed → running (重用)
```

## 许可证

Apache 2.0 或 MIT

---

**文档**: [中文](README-CN.md) | [English](README.md)
**路线图**: [MVP 4.0](mvp4.0.md) | [MVP 4.1](mvp4.1.md)