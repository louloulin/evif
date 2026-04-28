# EVIF Documentation

> Everything Is a Virtual Filesystem — Persistent context, reusable skills, and multi-agent coordination for AI agents.

## Quick Start

```bash
# 1. Install
cargo build --release

# 2. Start server (Docker for AI agents)
./target/release/evif-rest --port 8081 --auth-mode disabled

# 3. Use CLI (like docker run for AI)
evif health
evif mkdir /mem/demo
evif write -c "Hello EVIF" /mem/demo/intro.txt

# 4. Access via SDK
pip install -e crates/evif-python
python -c "from evif import Client; c = Client('http://localhost:8081'); print(c.health())"
```

## Documentation Chapters

### [1. Architecture Overview](00-overview.md)
System architecture, plugin system, mount table, and project statistics.

**Start here** to understand how EVIF works.

### [2. Core Modules](01-core-modules.md)
EVIF Core engine internals — 26 modules covering:
- Plugin trait and lifecycle
- Radix Mount Table (O(k) path routing)
- Handle Manager with lease-based resource management
- Cache Manager, Circuit Breaker, Batch Operations

**Read this** for deep technical understanding.

### [3. Plugin System](02-plugin-system.md)
40+ plugin implementations:
- **Agent Primitives**: ContextFS, SkillFS, PipeFS, QueueFS, VectorFS
- **Storage**: memfs, localfs, S3FS, GCSFS, AzureFS, SQLiteFS, PostgreSQLFS
- **Enhancement**: EncryptedFS, TieredFS, StreamRotateFS

**Includes SKILL.md format specification** for reusable workflows.

### [4. REST API Reference](03-rest-api.md)
106 endpoints across 14 categories:
- File operations, Handle operations, Mount management
- Memory operations, Context operations, Skill operations
- System operations, Monitoring, Encryption, Collaboration

**API Reference** for developers integrating with EVIF.

### [5. SDK Integration](04-sdk-integration.md)
- **Python SDK**: Full async/sync client, Memory/Queue mixins
- **TypeScript SDK**: Type-safe client
- **Go SDK**: Idiomatic Go interface
- **MCP Server**: 13 tools for Claude Code native integration

**Start here** for SDK development.

### [6. Agent Integration](05-agent-integration.md)
Integration guides for AI agent platforms:
- **Claude Code**: MCP Server + preSession hook setup
- **Codex**: Python SDK plugin
- **OpenClaw**: PipeFS for multi-agent coordination

**Includes CLAUDE.md configuration** for agent projects.

### [7. Deployment & Operations](06-deployment.md)
- Local, Docker, Docker Compose, Kubernetes deployment
- Configuration, health checks, monitoring
- Security, backup/recovery, performance tuning

**For DevOps and production deployment**.

### [8. Developer Guide](07-developer-guide.md)
- Build, test, lint commands
- Creating new plugins
- Adding REST endpoints
- Testing strategies
- Performance benchmarking
- Release process

**For contributors and plugin authors**.

## Quick Reference

### CLI Commands

| Command | Description | Example |
|---------|-------------|---------|
| `evif health` | Check server status | `evif health` |
| `evif ls <path>` | List directory | `evif ls /mem` |
| `evif cat <path>` | Read file | `evif cat /context/L0/current` |
| `evif write -c <text> <path>` | Write file | `evif write -c "task" /context/L0/current` |
| `evif mkdir <path>` | Create directory | `evif mkdir /pipes/my-task` |
| `evif rm <path>` | Remove file/dir | `evif rm /mem/test.txt` |
| `evif stat <path>` | File metadata | `evif stat /mem/test.txt` |
| `evif mounts` | List mounts | `evif mounts` |

### REST API Base

```
http://localhost:8081/api/v1
```

### Default Mounts

| Mount | Plugin | Purpose |
|-------|--------|---------|
| `/mem` | memfs | In-memory filesystem |
| `/context` | contextfs | Persistent context (L0/L1/L2) |
| `/skills` | skillfs | Reusable workflows |
| `/pipes` | pipefs | Multi-agent coordination |
| `/queue` | queuefs | Task queues |
| `/data` | localfs | Local disk storage |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EVIF_REST_PORT` | `8081` | HTTP port |
| `EVIF_REST_AUTH_MODE` | `api-key` | Auth mode |
| `EVIF_API_KEY` | - | API key |
| `EVIF_LOG_DIR` | `logs` | Log directory |

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Consumer Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  CLI     │  │   SDK    │  │  MCP     │  │   REST   │    │
│  │  evif    │  │ Python   │  │ Server   │  │   API    │    │
│  └──────────┘  │ TypeScript│  │          │  │  (106)   │    │
│                │ Go       │  │ Claude   │  │          │    │
│                └──────────┘  │ Code     │  │          │    │
│                              └──────────┘  └──────────┘    │
└───────────────────────────────┬───────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────┐
│                      API Layer                              │
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
│                    EVIF Core                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Radix Mount Table (O(k))                  │  │
│  │                                                       │  │
│  │  /mem ──────► memfs                                  │  │
│  │  /context ──► contextfs                              │  │
│  │  /skills ───► skillfs                                │  │
│  │  /pipes ────► pipefs                                 │  │
│  │  /data ─────► localfs                                │  │
│  │  /s3 ───────► s3fs (configurable)                    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │ Handle  │ │  Cache  │ │Circuit  │ │  Batch  │           │
│  │ Manager │ │ Manager │ │ Breaker │ │   Ops   │           │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
└───────────────────────────────┬───────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────┐
│                   Plugin Layer (40+)                       │
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

## Code Examples

### Python SDK

```python
from evif import Client

client = Client("http://localhost:8081")

# File operations
client.mkdir("/mem/demo")
client.write("/mem/demo/data.txt", "Hello!")
print(client.cat("/mem/demo/data.txt"))

# Context for agents
client.write("/context/L0/current", "Implementing auth module")
decisions = client.cat("/context/L1/decisions.md")

# Memory search
client.memory_store("User prefers dark mode", modality="preference")
results = client.memory_search("editor theme")

# Task queue
import json
client.write("/queue/tasks/enqueue", json.dumps({"type": "review"}))
```

### CLI Workflow

```bash
# Agent session start
evif cat /context/L0/current          # What was I doing?
evif cat /context/L1/decisions.md     # What decisions?

# Agent work
evif write -c "JWT implementation" /context/L0/current
evif mkdir /pipes/review-task
evif write -c "Review auth module" /pipes/review-task/input

# Agent session end
evif write -c "Auth complete, testing" /context/L0/current
```

### MCP Server Tools (Claude Code)

```json
// Claude Code gets these tools automatically
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

## Related Documents

- [Getting Started](GETTING_STARTED.md) - Quick start guide
- [CLI Reference](cli-mode.md) - CLI usage
- [MCP Server](mcp-server.md) - MCP Server configuration
- [Metrics](metrics.md) - Prometheus metrics
- [Plugin Development](plugin-development.md) - Plugin authoring
- [FUSE](fuse.md) - FUSE filesystem

## Project Info

- **Repository**: https://github.com/evif/evif
- **Version**: 0.1.0
- **Language**: Rust
- **Lines of Code**: ~45,000
- **Crates**: 10+
- **Plugins**: 40+

### Crate Structure

```
evif/
├── crates/
│   ├── evif-core/         # Core engine (26 modules)
│   ├── evif-plugins/       # Plugin implementations (40+)
│   ├── evif-rest/          # REST API server
│   ├── evif-cli/           # CLI tool
│   ├── evif-python/        # Python SDK
│   ├── evif-sdk-ts/        # TypeScript SDK
│   ├── evif-sdk-go/        # Go SDK
│   └── evif-mcp/           # MCP server
├── tests/                  # Integration tests
├── demos/                  # Demo applications
└── docs/                   # This documentation
```

---

**Questions?** Open an issue at https://github.com/evif/evif/issues
