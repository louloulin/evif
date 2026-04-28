# EVIF Architecture Overview

## 1. Project Vision

**EVIF (Everything Is a Virtual Filesystem)** is a Rust-based, plugin-driven virtual filesystem that provides a unified file-based interface for AI agents and applications. The core philosophy is that every capability—storage, context, skills, memory, and coordination—can be accessed through standard filesystem operations (`ls`, `cat`, `write`, `mkdir`, `rm`).

## 2. Core Design Principles

| Principle | Description |
|-----------|-------------|
| **Everything is a File** | All resources are accessed via file operations |
| **Plugin-based Architecture** | 40+ plugins extend functionality |
| **Async-first** | Built on tokio async runtime |
| **Zero-copy where possible** | Streaming and handles for efficiency |
| **Resilient** | Circuit breakers, retries, batch operations |

## 3. System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Consumer Layer                                  │
│                                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐│
│  │ Claude   │  │  Codex   │  │OpenClaw  │  │ Python   │  │   CLI    ││
│  │  Code    │  │          │  │          │  │   SDK    │  │          ││
│  │ (MCP)   │  │          │  │          │  │          │  │          ││
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘│
└────────┼────────────┼────────────┼────────────┼────────────┼────────┘
         │            │            │            │            │
         ▼            ▼            ▼            ▼            ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         API Layer (106 endpoints)                        │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     REST API (Axum)                              │   │
│  │  /health /files /stat /grep /mounts /plugins /memories /pipes │   │
│  │  /context /skills /tenants /encryption /sync /graphql /ws     │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     MCP Server (@evif/mcp-server)                │   │
│  │  context_get, context_set, skill_run, memory_search, pipe_send │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     SDKs (Python / TypeScript / Go)              │   │
│  │  SyncEvifClient, EvifClient, REST + WebSocket support           │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         EVIF Core Engine                                 │
│                                                                          │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌───────────┐ │
│  │ Radix Mount  │  │  Plugin       │  │   Handle     │  │  Cache    │ │
│  │    Table     │  │   Registry    │  │   Manager    │  │  Manager  │ │
│  │  O(k) path   │  │  40+ plugins  │  │  Lease-based│  │ LRU/Meta  │ │
│  └───────────────┘  └───────────────┘  └───────────────┘  └───────────┘ │
│                                                                          │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌───────────┐ │
│  │  Auth (ACL)   │  │  Encryption   │  │  Monitoring   │  │   Batch   │ │
│  │ Capability-   │  │  AES-GCM     │  │ Prometheus   │  │  Parallel │ │
│  │   based      │  │ Transparent  │  │   Metrics    │  │  Copy/Del │ │
│  └───────────────┘  └───────────────┘  └───────────────┘  └───────────┘ │
│                                                                          │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌───────────┐ │
│  │ Circuit       │  │ File Monitor  │  │ Dynamic       │  │  WASM     │ │
│  │  Breaker     │  │   (notify)    │  │   Loader      │  │  Runtime  │ │
│  │  Resilience  │  │   Watch       │  │  .so/.dylib  │  │ Extism +  │ │
│  └───────────────┘  └───────────────┘  └───────────────┘  │ Wasmtime  │ │
│                                                               └───────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           Plugin Layer (40+ Built-in)                    │
│                                                                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │   Agent Primitives    │  │   Storage Backends  │  │    Enhancement Capabilities    │   │
│  ├─────────────────┤  ├─────────────────┤  ├─────────────────────────┤   │
│  │ ContextFS      │  │ LocalFS        │  │ EncryptedFS (AES-GCM)    │   │
│  │ SkillFS        │  │ MemFS         │  │ TieredFS (hot/warm/cold)│   │
│  │ PipeFS        │  │ S3FS          │  │ StreamRotateFS           │   │
│  │ QueueFS       │  │ GCSFS         │  │ BatchFS                  │   │
│  │ VectorFS      │  │ AzureBlobFS  │  │ MetricsFS                │   │
│  │                │  │ AliyunOSSFS  │  │ FileLockFS              │   │
│  │                │  │ TencentCOSFS │  │                         │   │
│  │                │  │ HuaweiOBSFS  │  │                         │   │
│  │                │  │ MinIOFS      │  │                         │   │
│  │                │  │ SQLFS (MySQL │  │                         │   │
│  │                │  │   PostgreSQL │  │                         │   │
│  │                │  │   SQLite)    │  │                         │   │
│  │                │  │ SFTPFS       │  │                         │   │
│  │                │  │ FTPFS        │  │                         │   │
│  │                │  │ WebDAVFS     │  │                         │   │
│  │                │  │ HttpFS       │  │                         │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘   │
│                                                                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │    Special Types        │  │   Protocol Bridges   │  │       System Utilities       │   │
│  ├─────────────────┤  ├─────────────────┤  ├─────────────────────────┤   │
│  │ HelloFS        │  │ ProxyFS        │  │ DevFS                   │   │
│  │ HeartbeatFS    │  │ OpenDAL       │  │ ServerInfoFS            │   │
│  │ GPTFS (LLM)    │  │ S3FS-OpenDAL  │  │ HandleFS                │   │
│  │ ContextManager  │  │               │  │                         │   │
│  │ SkillRuntime   │  │               │  │                         │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 4. Core Modules

### 4.1 Radix Mount Table

**Purpose**: O(k) path routing where k is path depth

```rust
// Example: /context/L0/current → contextfs plugin
mount_table.mount("/context", ContextFsPlugin::new())
mount_table.mount("/mem", MemFsPlugin::new())
mount_table.mount("/skills", SkillFsPlugin::new())
mount_table.mount("/pipes", PipeFsPlugin::new())
```

**Key Operations**:
- `mount(path, plugin)` - Register plugin at path
- `unmount(path)` - Remove plugin registration
- `resolve(path)` - Find plugin for path in O(k)
- `list()` - Get all mount points

### 4.2 Plugin Trait

```rust
#[async_trait]
pub trait EvifPlugin: Send + Sync {
    async fn create(&self, path: &Path, options: CreateOptions) -> Result<FileHandle>;
    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> Result<Bytes>;
    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> Result<u64>;
    async fn readdir(&self, path: &Path, offset: u64) -> Result<Vec<DirEntry>>;
    async fn stat(&self, path: &Path) -> Result<Metadata>;
    async fn remove(&self, path: &Path) -> Result<()>;
    async fn rename(&self, old: &Path, new: &Path) -> Result<()>;
    async fn remove_all(&self, path: &Path) -> Result<u32>;
}
```

### 4.3 Handle Manager

**Purpose**: Manage file handles with lease-based resource management

```rust
pub struct GlobalHandleManager {
    counter: AtomicU64,
    handles: DashMap<u64, Arc<HandleState>>,
    ttl_cleanup: Interval,
}
```

**Features**:
- Atomic handle ID allocation
- TTL-based lease cleanup
- Reference counting
- Read/write/seek state tracking

### 4.4 Cache Manager

**Purpose**: Multi-level caching for metadata and directory listings

```rust
pub enum EvifCache {
    Metadata(MetadataCache),
    Directory(DirectoryCache),
    Content(Arc<dyn KvStore>),
}
```

**Strategies**:
- LRU eviction for metadata
- TTL-based expiration
- Write-through for consistency
- Optional content caching

### 4.5 Circuit Breaker

**Purpose**: Prevent cascade failures in distributed operations

```rust
pub enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, reject requests
    HalfOpen,   // Test if recovered
}
```

## 5. Agent Primitives

### 5.1 ContextFS

Three-layer persistent context for AI agents:

```
/context/
├── L0/                    # Immediate context
│   ├── current           # Current task description
│   ├── recent_ops        # Recent operations log
│   └── budget_status     # Token budget tracking
├── L1/                    # Session context
│   ├── decisions.md       # Session decisions with rationale
│   └── scratch/          # Temporary workspace
└── L2/                    # Project knowledge
    ├── architecture.md    # Architecture documentation
    ├── patterns.md       # Code patterns
    ├── runbooks/         # Operational runbooks
    └── history/          # Historical sessions
```

### 5.2 SkillFS

Reusable workflow skills with SKILL.md standard:

```yaml
---
name: code-review
description: "Review code for bugs and security issues"
triggers:
  - "review code"
  - "code review"
---
# Code Review Workflow

1. Read target files
2. Identify potential issues
3. Generate concise report
```

**Execution Flow**:
1. Write input → `/skills/{name}/input`
2. Skill auto-executes
3. Read output → `/skills/{name}/output`

### 5.3 PipeFS

Multi-agent coordination via state machine:

```
/pipes/{name}/
├── input      # Task input
├── output     # Task result
├── status     # pending → running → complete/error
├── assignee   # Worker agent ID
├── timeout    # TTL in seconds
└── priority   # Task priority
```

### 5.4 QueueFS

FIFO task queue:

```
/queue/{name}/
├── enqueue    # Add task (atomic)
├── dequeue    # Get + remove task (atomic)
├── size       # Current queue size
└── peek       # View without removing
```

### 5.5 VectorFS

Semantic memory with embeddings:

```rust
pub struct VectorStore {
    dimension: usize,
    storage: Arc<dyn VectorStorage>,
    embedder: Arc<dyn Embedder>,
}
```

## 6. Storage Plugins

### 6.1 Cloud Storage

| Plugin | Provider | Features |
|--------|----------|----------|
| S3FS | AWS S3 | Multipart, presigned URLs |
| GCSFS | Google Cloud | OAuth, resumable upload |
| AzureBlobFS | Azure | SAS tokens, append blobs |
| AliyunOSSFS | Alibaba | VPC, CDN |
| TencentCOSFS | Tencent | COS API v5 |
| HuaweiOBSFS | Huawei | OBS SDK v3 |
| MinIOFS | MinIO | S3-compatible |

### 6.2 Database Storage

| Plugin | Backend | Features |
|--------|---------|----------|
| SQLFS | MySQL | Full SQL ops |
| SQLFS | PostgreSQL | JSONB, arrays |
| SQLFS2 | SQLite | Lightweight |
| MongoFS | MongoDB | Document store |

### 6.3 Special Storage

| Plugin | Purpose | Use Case |
|--------|---------|----------|
| EncryptedFS | AES-GCM encryption | Sensitive data |
| TieredFS | Hot/warm/cold | Cost optimization |
| StreamRotateFS | Log rotation | Large files |
| BatchFS | Bulk operations | ETL pipelines |

## 7. API Layer

### 7.1 REST Endpoints (106 total)

| Category | Count | Key Endpoints |
|----------|-------|---------------|
| File Operations | 16 | /files, /directories, /stat, /grep, /digest, /touch, /rename, /copy |
| Handle Operations | 10 | /handles/open, /read, /write, /seek, /sync, /close, /renew |
| Plugin Management | 10 | /mounts, /plugins, /reload, /config, /status |
| Memory | 8 | /memories CRUD, /search, /categories, /query |
| Collaboration | 11 | /shares, /permissions, /comments, /activities |
| Tenant | 6 | /tenants CRUD, /quota |
| Encryption | 5 | /encrypt, /decrypt, /rotate, /versions |
| Sync | 5 | /delta, /version, /conflict, /resolve |
| GraphQL | 2 | /graphql, /playground |
| WebSocket | 1 | /ws (realtime) |
| WASM Plugins | 4 | /wasm/load, /reload, /unload, /list |
| Monitoring | 5 | /metrics, /stats/system, /reset |
| Health | 4 | /health, /readiness, /ping, /cloud |
| Context | 3 | /context/L0, /context/L1, /context/search |
| Batch | 3 | /batch/copy, /batch/delete, /batch/stats |
| Misc | 3 | /capabilities, /config, /capabilities |

### 7.2 Authentication

| Mode | Description |
|------|-------------|
| Disabled | No auth (dev mode) |
| API Key | Static key in header |
| Capability-based | Fine-grained permissions |

### 7.3 Rate Limiting

- Per-client rate limits
- Endpoint-specific limits
- Burst allowance
- Circuit breaker integration

## 8. SDKs

### 8.1 Python SDK

```python
from evif import Client

# Sync client (recommended)
client = Client("http://localhost:8081")
health = client.health()
files = client.ls("/mem")

# Async client
async with Client() as client:
    await client.write("/mem/data.txt", "content")
    content = await client.cat("/mem/data.txt")
```

### 8.2 TypeScript SDK

```typescript
import { EvifClient } from '@evif/sdk';

const client = new EvifClient({ baseUrl: 'http://localhost:8081' });
const files = await client.ls('/mem');
await client.write('/mem/data.txt', 'content');
```

### 8.3 Go SDK

```go
import "github.com/evif/evif-go"

client := evif.NewClient("http://localhost:8081")
files, err := client.Ls("/mem")
err = client.Write("/mem/data.txt", "content")
```

### 8.4 MCP Tools

```json
{
  "tools": [
    { "name": "evif_context_get", "args": ["layer"] },
    { "name": "evif_context_set", "args": ["layer", "content"] },
    { "name": "evif_skill_run", "args": ["name", "input"] },
    { "name": "evif_memory_search", "args": ["query"] },
    { "name": "evif_memory_store", "args": ["content", "modality"] },
    { "name": "evif_pipe_create", "args": ["name"] },
    { "name": "evif_pipe_send", "args": ["name", "data"] },
    { "name": "evif_health", "args": [] }
  ]
}
```

## 9. CLI Commands

```bash
# File operations
evif ls <path>           # List directory
evif cat <path>          # Read file
evif write -c <content> <path>  # Write file
evif mkdir <path>        # Create directory
evif rm <path>           # Remove file/directory

# Advanced operations
evif grep <pattern> [path]  # Search files
evif cp <src> <dst>         # Copy file
evif mv <src> <dst>         # Move/rename

# System
evif health              # Server health
evif mount               # List mounts
evif plugin             # Plugin management
evif config             # Configuration

# Agent primitives
evif context            # Context management
evif skill list        # List skills
evif skill run <name>   # Run skill
evif pipe status <name> # Pipe status

# Docker-like commands (Phase 1)
evif start              # Start server
evif stop               # Stop server
evif ps                 # List running
```

## 10. Statistics

| Metric | Value |
|--------|-------|
| Total Rust LOC | ~45,000+ |
| Core modules | 26 |
| Plugin implementations | 40+ |
| REST endpoints | 106 |
| CLI commands | 60+ |
| Test files | 50+ |
| SDKs | 3 (Python, TypeScript, Go) |
| Documentation files | 30+ |

## 11. Dependencies

### Core
- `tokio` - Async runtime
- `axum` - HTTP framework
- `async-trait` - Async trait support
- `serde` - Serialization

### Storage
- `opendal` - Unified storage SDK
- `rusqlite` - SQLite
- `sqlx` - SQL databases
- `rusoto` - AWS SDK

### Utilities
- `tracing` - Structured logging
- `anyhow` - Error handling
- `uuid` - ID generation
- `chrono` - Time handling

## 12. Project Structure

```
evif/
├── Cargo.toml              # Workspace
├── README.md               # Project readme
├── CLAUDE.md              # Claude Code instructions
├── mem*.md                # Planning docs (20+)
│
├── crates/
│   ├── evif-core/         # Core engine (26 modules)
│   ├── evif-plugins/      # 40+ plugins
│   ├── evif-rest/         # REST API server
│   ├── evif-cli/          # CLI tool
│   ├── evif-client/        # Client library
│   ├── evif-auth/         # Authentication
│   ├── evif-metrics/      # Prometheus metrics
│   ├── evif-mcp/          # MCP server
│   ├── evif-mem/          # Memory platform
│   ├── evif-fuse/         # FUSE integration
│   ├── evif-bench/        # Benchmarks
│   ├── evif-macros/       # Procedural macros
│   ├── evif-sdk-ts/       # TypeScript SDK
│   └── evif-sdk-go/       # Go SDK
│
├── tests/                 # Integration tests
│   ├── e2e/              # End-to-end
│   ├── cli/              # CLI tests
│   ├── api/              # API tests
│   ├── core/             # Core tests
│   └── common/           # Shared utilities
│
├── examples/              # Usage examples
│   ├── src/              # Example code
│   ├── 03_auth_*.rs     # Auth examples
│   ├── 06_metrics_*.rs   # Metrics examples
│   └── wasm-plugin/      # WASM plugin example
│
├── demos/                 # Demo applications
│   └── agent_workflow/   # Agent coordination demo
│
├── docs/                  # Documentation
│   ├── en/               # English docs
│   ├── zh/               # Chinese docs
│   ├── API.md            # API reference
│   ├── GETTING_STARTED.md
│   ├── mcp-server.md
│   ├── metrics.md
│   ├── plugin-development.md
│   ├── fuse.md
│   ├── cli-mode.md
│   ├── production-*.md
│   └── grafana/
│
└── .github/
    └── workflows/        # CI/CD pipelines
```

## 13. Related Documents

- [Plugin Development Guide](plugin-development.md)
- [REST API Reference](API.md)
- [MCP Server Setup](mcp-server.md)
- [CLI Reference](cli-mode.md)
- [Getting Started](GETTING_STARTED.md)
- [Production Deployment](production-deployment.md)
