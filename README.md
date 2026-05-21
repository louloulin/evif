# EVIF - Everything Is a File

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)
[![Tests](https://img.shields.io/badge/Tests-600+-green.svg)](#testing)
[![Crate](https://img.shields.io/badge/Crates-13-orange.svg)](#core-components)

> Agent infrastructure for context, skills, memory, coordination, and universal system connectivity. EVIF turns heterogeneous systems into a unified file interface so AI Agents can work with less integration code, less repeated context, and lower token cost.

**Documentation**: [English](docs/README.md) | [中文](README-CN.md)

## Overview

EVIF evolved from "Everything Is a File" to an Agent-native connectivity layer:

- **ContextFS** - Layered `L0/L1/L2` working context
- **SkillFS** - Standard `SKILL.md` skill discovery and invocation
- **PipeFS** - Lightweight multi-agent coordination
- **MemoryFS/VectorFS** - Long-term memory and semantic retrieval
- **Plugin + Mount Model** - Connect cloud storage, databases, SaaS apps, and local systems through one interface

EVIF's product goal is simple: let Agents operate across many systems with one model — files, directories, skills, memories, queues, and pipes — instead of bespoke API glue for every integration.

### Agent Positioning

EVIF provides unified file interface for Agents:

- `/context` - Active context (L0: current task, L1: session decisions, L2: project knowledge)
- `/skills` - Reusable workflows (SKILL.md format)
- `/pipes` - Task coordination (multi-agent communication)
- `/memories` - Vector memory with semantic search
- `/queue` - Durable task queues for asynchronous work

### Why EVIF for AI Agents

| Need | EVIF Capability | Agent Benefit |
|------|-----------------|---------------|
| Persistent context | `/context/L0`, `/context/L1`, `/context/L2` | Less repeated prompt setup |
| Reusable procedures | `/skills/*.SKILL.md` | Workflows become versioned assets |
| Multi-agent work | `/pipes`, `/queue` | Explicit coordination instead of prompt forwarding |
| Long-term memory | `/memories`, vector retrieval | Retrieve relevant knowledge without loading everything |
| System connectivity | plugin + mount adapters | One interface for many external systems |
| Lower token cost | compact reads, output filtering, bounded search | Smaller model inputs and cheaper tool loops |

### Key Features

| Feature | Description |
|---------|-------------|
| **13 Core Crates** | 120K lines of Rust, organized architecture |
| **40+ Plugins** | Cloud storage, databases, AI services |
| **63 MCP Tools** | Full tool ecosystem for AI agents |
| **150 REST Endpoints** | Comprehensive HTTP API |
| **600+ Tests** | High test coverage |
| **Multi-Agent Coordination** | PipeFS with wait_for_result, atomic claim |
| **Cost-Optimized Retrieval** | max_lines, compact search, output filtering, caching |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Access Layer                             │
│  CLI (60+ cmds) │ REST API (150 endpoints) │ MCP Server │ FUSE   │
├─────────────────────────────────────────────────────────────────┤
│                         Core Layer                               │
│  Mount Table (Radix Tree) │ Plugin Lifecycle │ Handle Management │
├─────────────────────────────────────────────────────────────────┤
│                         Plugin Layer                             │
│  ContextFS │ SkillFS │ PipeFS │ QueueFS │ VectorFS │ Storage    │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/evif/evif.git
cd evif
cargo build --release
cargo install --path crates/evif-cli

# Or use Homebrew (macOS)
brew tap evif/evif
brew install evif
```

### Start Server

```bash
evif-rest --port 8081
```

### Basic Usage

```bash
# File operations
evif ls /                            # List root
evif mkdir /mem/demo                 # Create directory
evif write /mem/hello.txt -c "Hi"   # Write file
evif cat /mem/hello.txt              # Read file
evif mounts                          # List mounts

# Health check
evif health
```

### 30-Second Agent Demo

```bash
# Context management
cat /context/L0/current
cat /context/L1/decisions.md
ls /skills

# Multi-agent coordination
mkdir /pipes/task-001
echo "review code" > /pipes/task-001/input
```

## Core Components

| Crate | Lines | Description |
|-------|-------|-------------|
| **evif-core** | ~15K | Core abstractions, Mount Table, Handle management |
| **evif-plugins** | ~30K | 40+ storage plugins (cloud, database, AI services) |
| **evif-rest** | ~20K | HTTP/JSON REST API server |
| **evif-cli** | ~15K | Command-line tool (60+ commands) |
| **evif-mcp** | ~25K | MCP (Model Context Protocol) server (63 tools) |
| **evif-mem** | ~20K | Memory platform, vector search |
| **evif-client** | ~5K | TypeScript/JS client SDK |
| **evif-metrics** | ~5K | Prometheus metrics |

## Plugin Directory

### Agent Primitives

| Plugin | Path | Function |
|--------|------|----------|
| `contextfs` | `/context` | L0/L1/L2 layered context |
| `skillfs` | `/skills` | SKILL.md skill discovery |
| `pipefs` | `/pipes` | Multi-agent coordination |
| `queuefs` | `/queue` | Task queue |
| `vectorfs` | `/memories` | Vector memory |
| `memfs` | `/mem` | In-memory filesystem |

### Cloud Storage (9 plugins)

| Plugin | Service |
|--------|---------|
| `s3fs` | Amazon S3 |
| `gcsfs` | Google Cloud Storage |
| `azureblobfs` | Azure Blob |
| `aliyunossfs` | Aliyun OSS |
| `tencentcosfs` | Tencent COS |
| `huaweiobsfs` | Huawei OBS |
| `miniofs` | MinIO |
| `webdavfs` | WebDAV |
| `httpfs` | HTTP/HTTPS |

### AI Service Integrations (8 plugins)

| Plugin | Service |
|--------|---------|
| `gmailfs` | Gmail (send, reply, drafts, labels) |
| `slackfs` | Slack (post messages, reactions) |
| `discordfs` | Discord (send, embed, reactions) |
| `githubfs` | GitHub |
| `notionfs` | Notion |
| `telegramfs` | Telegram |
| `teamsfs` | Microsoft Teams |
| `shopifyfs` | Shopify |

### Database Plugins

| Plugin | Type |
|--------|------|
| `sqlfs` | SQLite |
| `postgresfs` | PostgreSQL |

### Connectivity Model

EVIF does not expose integrations as unrelated one-off tools. Each integration is an adapter behind the same mount and file interface:

| Layer | Responsibility |
|-------|----------------|
| Mount | Maps a path such as `/mem`, `/context`, or `/github` to a backend |
| Plugin | Encapsulates auth, API calls, storage semantics, retries, and errors |
| Access Layer | Exposes the same capability through CLI, REST, MCP, FUSE, or SDKs |
| Agent Layer | Consumes files, skills, memory, queues, and pipes with a stable mental model |

See [Connector Capability Matrix](docs/connector-capability-matrix.md) for the current connector map.

## MCP Server

The EVIF MCP Server provides **63 tools** for AI agents, fully implemented with real backend calls.

### Tool Categories

| Category | Tools | Description |
|----------|-------|-------------|
| **Core FS** | 13 | ls, cat, write, mkdir, rm, cp, mv, grep, tree, diff, du, hash |
| **Memory** | 5 | memorize, retrieve, memory_search, memory_clear, memory_stats |
| **Skills** | 4 | skill, skill_list, skill_info, claude_md_generate |
| **Session** | 7 | session, session_load, session_delete, agent, subagent_status |
| **Config** | 3 | config_get, config_list, config_set |
| **Cron** | 3 | cron_schedule, cron_list, cron_remove |
| **Observability** | 11 | health, health_detailed, metrics_export, log_query, cache_stats |
| **Plugins** | 4 | plugin_catalog, plugin_info, plugin_load, plugin_unload |
| **Events/Queues** | 5 | event_list, event_subscribe, queue_list, queue_stats |
| **Handles/Pipes** | 4 | open_handle, close_handle, pipe_create, pipe_list |
| **Utilities** | 4 | search, archive, batch, watch |

### Token Optimization

| Tool | Parameters | Estimated Savings |
|------|------------|-------------------|
| `evif_cat` | `max_lines` (default 100), `mode` (head/tail/snippet/full) | 60-90% |
| `evif_memory_search` | `compact` (default true), `limit` (default 3) | 40-60% |
| Output Filter | strip_ansi, truncate_lines, compact_json, max_string_length | 30-50% |

EVIF reduces Agent cost across six areas: model tokens, repeated context setup, integration code, tool output size, multi-agent coordination, and long-term maintenance. See [Cost Optimization Matrix](docs/cost-optimization-matrix.md) for the optimization model.

## AI Platform Integration

EVIF supports **5 AI platforms** via `evif connect`:

| Platform | Config File | Method |
|----------|-------------|--------|
| Claude Desktop | `claude_desktop_config.json` | MCP JSON |
| Claude Code | `settings.json` | MCP JSON |
| Cursor | `mcp.json` | MCP JSON |
| Gemini CLI | `settings.json` | MCP JSON |
| OpenAI Codex | `AGENTS.md` | Rules file |

```bash
# List supported platforms
evif connect --list

# Connect to Claude Desktop
evif connect claude

# Check integration status
evif connect --check

# Disconnect from platform
evif connect --disconnect claude
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

## Testing

```bash
# Full workspace test
cargo test --workspace

# Per-crate testing
cargo test -p evif-core
cargo test -p evif-rest
cargo test -p evif-mcp
cargo test -p evif-plugins
cargo test -p evif-cli

# E2E tests
cargo test -p evif-rest --test e2e_rest_api
```

### Test Coverage

| Crate | Unit Tests |
|-------|------------|
| evif-mem | 106 |
| evif-mcp | 76 |
| evif-plugins | 66 |
| evif-rest | 61 |
| evif-client | 50 |
| evif-core | 59 |
| evif-metrics | 43 |
| evif-cli | 46 |
| evif-auth | 31 |

## Project Structure

```
evif/
├── crates/
│   ├── evif-core/         # Core engine (Mount Table, Handle management)
│   ├── evif-plugins/       # 40+ plugin implementations
│   ├── evif-rest/          # REST API server (150 endpoints)
│   ├── evif-cli/           # CLI tool (60+ commands)
│   ├── evif-mcp/           # MCP server (63 tools)
│   ├── evif-mem/           # Memory platform, vector search
│   ├── evif-client/        # TypeScript/JS client
│   ├── evif-metrics/       # Prometheus metrics
│   ├── evif-auth/          # Authentication
│   ├── evif-bench/          # Benchmark suite
│   ├── evif-fuse/          # FUSE integration
│   ├── evif-macros/        # Procedural macros
│   └── example-dynamic-plugin/  # Plugin example
├── tests/                  # Integration tests
├── docs/                   # Documentation (includes zh/)
├── homebrew-formula/       # Homebrew tap
├── mvp4.0.md              # MVP 4.0 roadmap
├── mvp4.1.md              # MVP 4.1 roadmap
├── CHANGELOG.md           # Version history
└── CLAUDE.md              # Claude Code template
```

## Performance

| Metric | Value |
|--------|-------|
| Path Resolution | O(k) via Radix Tree |
| Handle Leases | Resource management |
| Multi-level Cache | inode + directory |
| Batch Operations | Concurrent copy/delete |
| Streaming | Large file support |

## MCP Server Implementation

EVIF MCP Server has been **fully implemented** with all mock implementations replaced with real backend calls.

### Implementation Phases

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 0 | ✅ | Core structure |
| Phase 1 | ✅ | VFS operations (L1) |
| Phase 2 | ✅ | HTTP bridge (L2) |
| Phase 3 | ✅ | Real implementations |
| Phase 4 | ✅ | Advanced tools |
| Phase 5 | ✅ | Mock removal |
| Phase 6 | ✅ | Performance optimization |
| Phase 7 | ✅ | Testing |
| Phase 8 | ✅ | Documentation |
| Phase 9 | ✅ | Cleanup |

### Architecture Layers

1. **L1: VFS Backend** - Core file operations via `VfsBackend`
2. **L2: HTTP Bridge** - Complex operations via backend API
3. **L3: Fallback** - Graceful degradation when backend unavailable

### Output Filtering Pipeline

```toml
# Output filter configuration
strip_ansi = true
max_lines = 200
compact_json = true    # Remove null fields
max_string_length = 10000
```

## License

Apache 2.0 or MIT

---

**Docs**: [English](docs/README.md) | [中文](README-CN.md)
**Roadmap**: [MVP 6.0](mvp6.0.md) | [MVP 4.0](mvp4.0.md) | [MVP 4.1](mvp4.1.md)
