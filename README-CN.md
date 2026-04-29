# EVIF - 万物皆为虚拟文件系统

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)

> 基于 Rust 构建的上下文导向虚拟文件系统，遵循 Plan 9 "一切皆文件"理念，为 AI Agent 提供持久化上下文、可复用技能和多智能体协同。

**文档**: [中文](README-CN.md) | [English](README.md)

## 项目简介

EVIF 从 "Everything Is a File" 演进为 AI Agent 的 "Context Is a File"：

- **ContextFS** - 分层 `L0/L1/L2` 工作上下文
- **SkillFS** - 标准 `SKILL.md` 技能发现和调用
- **PipeFS** - 轻量级多 Agent 协同
- **传统 EVIF 插件基础设施**

### 核心定位

EVIF 为 Agent 提供统一的文件接口：

- `/context` - 活跃上下文
- `/skills` - 可复用工作流
- `/pipes` - 任务协同

### 关键特性

- **插件架构**: 30+ 内置存储插件
- **Radix Tree 路由**: O(k) 路径解析
- **多访问方式**: REST API, CLI, FUSE, WebSocket
- **云存储**: S3, Azure, GCS, OSS, COS, OBS
- **AI Agent 原语**: ContextFS, SkillFS, PipeFS, QueueFS, VectorFS
- **WASM 插件**: Extism 多语言插件支持

## 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                      访问层                                  │
│  CLI (60+ 命令) │ REST API (106 端点) │ FUSE │ WebSocket    │
├─────────────────────────────────────────────────────────────┤
│                      核心层                                  │
│  Mount Table (Radix Tree) │ Plugin Lifecycle │ Handles       │
├─────────────────────────────────────────────────────────────┤
│                      插件层                                  │
│  ContextFS │ SkillFS │ PipeFS │ QueueFS │ VectorFS │ Storage│
└─────────────────────────────────────────────────────────────┘
```

## 快速开始

### 安装

```bash
git clone https://github.com/evif/evif.git
cd evif
cargo build --release
cargo install --path crates/evif-cli
```

### 启动服务

```bash
evif-rest --port 8081
```

### 基本使用

```bash
evif health                          # 健康检查
evif ls /                            # 列出根目录
evif mkdir /mem/demo                 # 创建目录
evif write /mem/hello.txt -c "Hi"   # 写入文件
evif cat /mem/hello.txt              # 读取文件
evif mounts                          # 列出挂载
```

### 30 秒 Agent 演示

```bash
cat /context/L0/current
cat /context/L1/decisions.md
ls /skills
mkdir /pipes/task-001
echo "review code" > /pipes/task-001/input
```

### Python SDK

```bash
pip install -e crates/evif-python
```

```python
from evif import Client

client = Client("http://localhost:8081")
client.mkdir("/mem/demo")
client.write("/mem/demo/data.txt", "Hello!")
print(client.cat("/mem/demo/data.txt"))

# Agent 上下文
client.write("/context/L0/current", "Implementing auth")
client.memory_store("JWT token usage", modality="knowledge")

# 任务队列
import json
client.write("/queue/tasks/enqueue", json.dumps({"type": "review"}))
```

## 核心组件

| 组件包 | 说明 |
|--------|------|
| **evif-core** | 核心抽象，插件系统，Mount Table，Handle 管理 |
| **evif-rest** | HTTP/JSON REST API 服务器 |
| **evif-cli** | 命令行工具 (60+ 命令) |
| **evif-plugins** | 30+ 存储插件 |
| **evif-mem** | 记忆平台，向量搜索 |
| **evif-mcp** | MCP (Model Context Protocol) 服务器 |
| **evif-fuse** | FUSE 文件系统集成 |
| **evif-auth** | 认证和授权 |
| **evif-metrics** | Prometheus 指标 |

## 插件目录

### Agent 原语

| 插件 | 路径 | 功能 |
|------|------|------|
| `memfs` | `/mem` | 内存文件系统 |
| `contextfs` | `/context` | 分层上下文 (L0/L1/L2) |
| `skillfs` | `/skills` | SKILL.md 技能 |
| `pipefs` | `/pipes` | 多 Agent 协同 |
| `queuefs` | `/queue` | 任务队列 |
| `vectorfs` | `/memories` | 向量内存 |

### 云存储

| 插件 | 服务 |
|------|------|
| `s3fs` | Amazon S3 |
| `gcsfs` | Google Cloud Storage |
| `azureblobfs` | Azure Blob |
| `aliyunossfs` | 阿里云 OSS |
| `tencentcosfs` | 腾讯 COS |
| `huaweiobsfs` | 华为 OBS |
| `miniofs` | MinIO |

### 数据库

| 插件 | 类型 |
|------|------|
| `sqlfs2` | SQLite |
| `postgresqlfs` | PostgreSQL |

## SDK

### Python

```bash
pip install -e crates/evif-python
```

### TypeScript

```bash
npm install @evif/sdk
```

### Go

```bash
go get github.com/evif/evif-go
```

### MCP Server (Claude Code)

```bash
claude mcp add @evif/mcp-server
```

## 配置

### 环境变量

| 变量 | 默认值 | 说明 |
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

## 项目结构

```
evif/
├── crates/
│   ├── evif-core/         # 核心引擎 (26 模块)
│   ├── evif-plugins/      # 40+ 插件实现
│   ├── evif-rest/         # REST API 服务器
│   ├── evif-cli/          # CLI 工具
│   ├── evif-python/       # Python SDK
│   ├── evif-sdk-ts/        # TypeScript SDK
│   ├── evif-sdk-go/        # Go SDK
│   ├── evif-mcp/          # MCP 服务器
│   ├── evif-mem/          # 记忆平台
│   ├── evif-fuse/         # FUSE 集成
│   └── evif-auth/         # 认证
├── tests/                  # 测试
├── demos/                  # 演示
├── examples/               # 示例
└── docs/                   # 文档 (含 zh/)
```

## 性能指标

- **O(k) 路径解析**: Radix Tree Mount Table
- **Handle 租约**: 资源管理
- **多级缓存**: inode + 目录
- **批量操作**: 并发复制/删除
- **流式处理**: 大文件

## 测试

```bash
cargo test --workspace
cargo test -p evif-core
cargo test -p evif-rest
```

## 许可证

Apache 2.0 或 MIT

---

**文档**: [中文](README-CN.md) | [English](README.md)
