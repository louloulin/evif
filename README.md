# EVIF - Everything Is a File

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)

> Context-oriented virtual filesystem built with Rust, following Plan 9's "Everything Is a File" philosophy. Provides persistent context, reusable skills, and multi-agent coordination for AI Agents.

**Documentation**: [English](docs/README.md) | [中文](README-CN.md)

## Documentation

### Core Documentation

| Chapter | English | Description |
|---------|---------|-------------|
| Architecture Overview | [docs/00-overview.md](docs/00-overview.md) | System architecture, plugin system, mount table |
| Core Modules | [docs/01-core-modules.md](docs/01-core-modules.md) | 26 modules covering lifecycle, routing, caching |
| Plugin System | [docs/02-plugin-system.md](docs/02-plugin-system.md) | 40+ plugin implementations |
| REST API | [docs/03-rest-api.md](docs/03-rest-api.md) | 106 endpoints, 14 categories |
| SDK Integration | [docs/04-sdk-integration.md](docs/04-sdk-integration.md) | Python, TypeScript, Go SDKs |
| Agent Integration | [docs/05-agent-integration.md](docs/05-agent-integration.md) | Claude Code, Codex integration |
| Deployment | [docs/06-deployment.md](docs/06-deployment.md) | Docker, Kubernetes, production |
| Developer Guide | [docs/07-developer-guide.md](docs/07-developer-guide.md) | Contributing, testing, debugging |

### Supplementary Documents

| Document | Purpose |
|----------|---------|
| [docs/GETTING_STARTED.md](docs/GETTING_STARTED.md) | Quick start guide |
| [docs/cli-mode.md](docs/cli-mode.md) | CLI usage |
| [docs/mcp-server.md](docs/mcp-server.md) | MCP Server setup |
| [docs/metrics.md](docs/metrics.md) | Monitoring metrics |
| [docs/plugin-development.md](docs/plugin-development.md) | Plugin development |
| [docs/fuse.md](docs/fuse.md) | FUSE usage |
| [docs/claude-code-workflow.md](docs/claude-code-workflow.md) | Claude Code integration |
| [docs/codex-workflow.md](docs/codex-workflow.md) | Codex integration |
| [docs/slo.md](docs/slo.md) | SLO definitions |
| [docs/production-env-vars.md](docs/production-env-vars.md) | Environment variables |

### Chinese Documentation

| Document | Status |
|----------|--------|
| [docs/zh/00-overview.md](docs/zh/00-overview.md) | Available |
| [README-CN.md](README-CN.md) | Project overview in Chinese |

## Overview

EVIF evolved from "Everything Is a File" to "Context Is a File" for AI Agents:

- **ContextFS** - Layered `L0/L1/L2` working context
- **SkillFS** - Standard `SKILL.md` skill discovery and invocation
- **PipeFS** - Lightweight multi-agent coordination
- **Traditional EVIF plugin infrastructure**

### Agent Positioning

EVIF provides unified file interface for Agents:

- `/context` - Active context
- `/skills` - Reusable workflows
- `/pipes` - Task coordination

### Key Features

- **Plugin Architecture**: 30+ built-in storage plugins
- **Radix Tree Routing**: O(k) path resolution
- **Multiple Access Methods**: REST API, CLI, FUSE, WebSocket
- **Cloud Storage**: S3, Azure, GCS, OSS, COS, OBS
- **AI Agent Primitives**: ContextFS, SkillFS, PipeFS, QueueFS, VectorFS
- **WASM Plugins**: Extism multi-language support

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Access Layer                            │
│  CLI (60+ commands) │ REST API (106 endpoints) │ FUSE │ WS │
├─────────────────────────────────────────────────────────────┤
│                      Core Layer                             │
│  Mount Table (Radix Tree) │ Plugin Lifecycle │ Handles      │
├─────────────────────────────────────────────────────────────┤
│                      Plugin Layer                           │
│  ContextFS │ SkillFS │ PipeFS │ QueueFS │ VectorFS │ Storage│
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Installation

```bash
git clone https://github.com/evif/evif.git
cd evif
cargo build --release
cargo install --path crates/evif-cli
```

### Start Server

```bash
evif-rest --port 8081
```

### Basic Usage

```bash
evif health                          # Health check
evif ls /                            # List root
evif mkdir /mem/demo                 # Create directory
evif write /mem/hello.txt -c "Hi"   # Write file
evif cat /mem/hello.txt              # Read file
evif mounts                          # List mounts
```

### 30-Second Agent Demo

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

# Agent context
client.write("/context/L0/current", "Implementing auth")
client.memory_store("JWT token usage", modality="knowledge")

# Task queue
import json
client.write("/queue/tasks/enqueue", json.dumps({"type": "review"}))
```

## Core Components

| Crate | Description |
|-------|-------------|
| **evif-core** | Core abstractions, plugin system, Mount Table, Handle management |
| **evif-rest** | HTTP/JSON REST API server |
| **evif-cli** | Command-line tool (60+ commands) |
| **evif-plugins** | 30+ storage plugins |
| **evif-mem** | Memory platform, vector search |
| **evif-mcp** | MCP (Model Context Protocol) server |
| **evif-fuse** | FUSE filesystem integration |
| **evif-auth** | Authentication and authorization |
| **evif-metrics** | Prometheus metrics |

## Plugin Directory

### Agent Primitives

| Plugin | Path | Function |
|--------|------|----------|
| `memfs` | `/mem` | In-memory filesystem |
| `contextfs` | `/context` | Layered context (L0/L1/L2) |
| `skillfs` | `/skills` | SKILL.md skills |
| `pipefs` | `/pipes` | Multi-agent coordination |
| `queuefs` | `/queue` | Task queue |
| `vectorfs` | `/memories` | Vector memory |

### Cloud Storage

| Plugin | Service |
|--------|---------|
| `s3fs` | Amazon S3 |
| `gcsfs` | Google Cloud Storage |
| `azureblobfs` | Azure Blob |
| `aliyunossfs` | Aliyun OSS |
| `tencentcosfs` | Tencent COS |
| `huaweiobsfs` | Huawei OBS |
| `miniofs` | MinIO |

### Database

| Plugin | Type |
|--------|------|
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

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EVIF_REST_PORT` | 8081 | REST port |
| `EVIF_REST_HOST` | 0.0.0.0 | Bind address |
| `EVIF_REST_AUTH_MODE` | api-key | Authentication mode |
| `EVIF_API_KEY` | - | API key |
| `EVIF_LOG_DIR` | logs | Log directory |
| `EVIF_METRICS_ENABLED` | false | Enable Prometheus |

### Config File

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

## Project Structure

```
evif/
├── crates/
│   ├── evif-core/         # Core engine (26 modules)
│   ├── evif-plugins/      # 40+ plugin implementations
│   ├── evif-rest/         # REST API server
│   ├── evif-cli/          # CLI tool
│   ├── evif-python/       # Python SDK
│   ├── evif-sdk-ts/        # TypeScript SDK
│   ├── evif-sdk-go/        # Go SDK
│   ├── evif-mcp/          # MCP server
│   ├── evif-mem/          # Memory platform
│   ├── evif-fuse/         # FUSE integration
│   └── evif-auth/         # Authentication
├── tests/                  # Tests
├── demos/                  # Demos
├── examples/               # Examples
└── docs/                   # Documentation (includes zh/)
```

## Performance

- **O(k) Path Resolution**: Radix Tree Mount Table
- **Handle Leases**: Resource management
- **Multi-level Cache**: inode + directory
- **Batch Operations**: Concurrent copy/delete
- **Streaming**: Large files

## Testing

```bash
cargo test --workspace
cargo test -p evif-core
cargo test -p evif-rest
```

## License

Apache 2.0 or MIT

---

**Docs**: [English](docs/README.md) | [中文](README-CN.md)
