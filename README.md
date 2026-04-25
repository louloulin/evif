# EVIF - Everything Is a File

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)

> A context-oriented, extensible virtual file system built in Rust, following the Plan 9 "Everything Is a File" philosophy.

[中文文档](README-CN.md)

## Overview

EVIF is evolving from "Everything Is a File" toward "Context Is a File" for AI agents. The current repository now includes:

- `ContextFS` for layered `L0/L1/L2` working context
- `SkillFS` for standard `SKILL.md` discovery and invocation
- `PipeFS` for lightweight multi-agent coordination
- Traditional EVIF plugin infrastructure for storage, routing, REST, CLI, and FUSE

### Agent Positioning

EVIF gives agents one file-oriented surface for:

- active context in `/context`
- reusable workflows in `/skills`
- task coordination in `/pipes`

This keeps agent interaction compatible with simple primitives such as `ls`, `cat`, `grep`, and `write`.

EVIF is a modular plugin filesystem platform that exposes multiple backends through one mount table, one plugin lifecycle, and one file-oriented surface. The supported product path is centered on mount routing, plugin lifecycle management, file and directory operations, handle management, and access surfaces such as REST, CLI, and FUSE.

### Key Features

- **Plugin Architecture**: 30+ built-in plugins for various storage backends
- **Plugin Kernel**: Mountable plugin filesystem with radix-tree routing and handle management
- **Multiple Access Methods**: REST API, CLI, FUSE mount, and WebSocket
- **Storage Backends**: Memory, Local FS, S3, Azure Blob, GCS, Aliyun OSS, and more
- **Advanced Features**: Batch operations, streaming, encryption, tiering, and monitoring
- **Dynamic Plugin Loading**: Runtime loading of `.so`/`.dylib`/`.dll` plugins
- **WASM Plugin Support**: WebAssembly-based plugin extensions

## Architecture

### Agent-Centered Layers

```
┌─────────────────────────────────────────────────────────────┐
│ Agent Access Layer                                          │
│  /context      /skills      /pipes      REST / CLI / FUSE  │
├─────────────────────────────────────────────────────────────┤
│ EVIF Core                                                   │
│  Mount Table   Plugin Lifecycle   Handles   Metrics         │
├─────────────────────────────────────────────────────────────┤
│ Plugin Layer                                                │
│  ContextFS    SkillFS    PipeFS    Storage / Queue / Vector │
└─────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────┐
│                      Access Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ REST API │  │   CLI    │  │   FUSE   │  │WebSocket │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
├───────┴────────────┴─────────┴─────────┴─────────┴─────────┤
│                      Core Layer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Mount Table  │  │ Plugin System│  │ Handle Mgr   │     │
│  │ (Radix Tree) │  │              │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
├─────────────────────────────────────────────────────────────┤
│                    Storage Layer (Plugins)                  │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │
│  │Memory  │ │ Local  │ │   S3   │ │ Azure  │ │  SQL   │  │
│  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/evif.git
cd evif

# Build all components
cargo build --release

# Install CLI tool
cargo install --path crates/evif-cli
```

### Start the Server

```bash
# Start REST API server (default port 8081)
evif-rest

# Or specify a custom port
evif-rest --port 3000
```

### Basic Usage

```bash
# Check server health
evif health

# List root directory
evif ls /

# Create a file in memory filesystem
evif write /mem/hello.txt --content "Hello, EVIF!"

# Read file contents
evif cat /mem/hello.txt

# Create a directory
evif mkdir /mem/mydir

# Mount a local filesystem
evif mount-plugin local /local --config root=/tmp

# List mounted plugins
evif list-mounts
```

### 30-Second Agent Demo

```bash
cat /context/L0/current
cat /context/L1/decisions.md
ls /skills
cat /skills/code-review/SKILL.md
mkdir /pipes/task-001
echo "review changed handlers" > /pipes/task-001/input
cat /pipes/task-001/status
```

### 3-Minute Claude Code Setup

```bash
cargo build --release -p evif-rest -p evif-fuse
cp CLAUDE.md /path/to/your/project/CLAUDE.md
./target/release/evif-rest --port 8081
./target/release/evif-fuse-mount /tmp/evif --readwrite
cat /tmp/evif/context/L0/current
ls /tmp/evif/skills
```

## Core Components

| Crate | Description |
|-------|-------------|
| **evif-core** | Core abstractions, plugin system, mount table, handle manager |
| **evif-rest** | HTTP/JSON REST API server |
| **evif-cli** | Command-line interface (60+ commands) |
| **evif-client** | Rust client SDK |
| **evif-fuse** | FUSE filesystem integration (Linux/macOS) |
| **evif-auth** | Authentication and authorization layer |
| **evif-macros** | Procedural macros (`#[node]`, `#[builder]`, `#[error_macro]`) |
| **evif-metrics** | Prometheus metrics collection and export |
| **evif-mem** | Optional memory subsystem with timeline and relation queries |

## Available Plugins

### Core Supported Plugins
| Plugin | Description | Default Mount |
|--------|-------------|---------------|
| `memfs` | In-memory filesystem | `/mem` |
| `contextfs` | Layered agent context filesystem | `/context` |
| `skillfs` | Standard `SKILL.md` skill surface | `/skills` |
| `pipefs` | Agent coordination pipes | `/pipes` |
| `localfs` | Local filesystem access | - |
| `hellofs` | Hello world example plugin | `/hello` |
| `serverinfofs` | Server status and metrics | `/serverinfo` |
| `kvfs` | Key-value store interface | `/kv` |
| `queuefs` | Message queue interface | `/queue` |
| `sqlfs2` | SQLite-backed structured data filesystem | `/sqlfs2` |
| `proxyfs` | Proxy to other paths | - |
| `streamfs` | Streaming data interface | - |
| `heartbeatfs` | Health and lease heartbeat interface | - |

### Cloud Storage Plugins
| Plugin | Description | Feature Flag |
|--------|-------------|--------------|
| `s3fs` | Amazon S3 | `s3fs` |
| `azureblobfs` | Azure Blob Storage | `azureblobfs` |
| `gcsfs` | Google Cloud Storage | `gcsfs` |
| `aliyunossfs` | Aliyun OSS | `aliyunossfs` |
| `tencentcosfs` | Tencent COS | `tencentcosfs` |
| `huaweiobsfs` | Huawei OBS | `huaweiobsfs` |
| `miniofs` | MinIO | `miniofs` |

### OpenDAL Plugins (EVIF 2.1)
Based on OpenDAL 0.50.x for unified storage interface. See `evif-plugins/src/opendal.rs` for available services.

### Experimental Plugins
| Plugin | Description | Feature Flag |
|--------|-------------|--------------|
| `httpfs` | HTTP-based filesystem | - |
| `devfs` | Device and pseudo-file examples | - |
| `encryptedfs` | Encrypted filesystem layer | - |
| `tieredfs` | Tiered storage (hot/warm/cold) | - |
| `handlefs` | File handle management | - |
| `gptfs` | GPT/AI model interface | `gptfs` |
| `vectorfs` | Vector database interface | `vectorfs` |
| `streamrotatefs` | Stream rotation | `streamrotatefs` |

## REST API

### Base URL
Default: `http://localhost:8081` (configurable via `EVIF_PORT`)

### File Operations

```bash
# Read file (returns content + base64 data)
GET /api/v1/files?path=/mem/hello.txt&offset=0&size=0

# Write file
PUT /api/v1/files?path=/mem/hello.txt
Body: { "content": "base64-encoded-data", "encoding": "base64" }

# Delete file
DELETE /api/v1/files?path=/mem/hello.txt

# List directory
GET /api/v1/directories?path=/mem

# Create directory
POST /api/v1/directories
Body: { "path": "/mem/newdir", "parents": true }

# File metadata
GET /api/v1/stat?path=/mem/hello.txt
```

### Mount Management

```bash
# List mounts
GET /api/v1/mounts

# Mount plugin
POST /api/v1/mount
Body: { "plugin": "localfs", "path": "/local", "config": {"root": "/tmp"} }

# Unmount
POST /api/v1/unmount
Body: { "path": "/local" }
```

### Batch Operations

```bash
# Batch copy
POST /api/v1/batch/copy
Body: { "sources": ["/mem/a"], "destination": "/mem/dest", "concurrency": 4 }

# Batch delete
POST /api/v1/batch/delete
Body: { "paths": ["/mem/a", "/mem/b"] }

# Check progress
GET /api/v1/batch/progress/:operation_id
```

### WebSocket

```bash
# Connect to WebSocket
ws://localhost:8081/ws
```

For complete API documentation, see [docs/API.md](docs/API.md).

## CLI Commands

EVIF CLI provides 60+ commands:

### File Operations
- `ls`, `cat`, `write`, `mkdir`, `rm`, `mv`, `cp`, `stat`, `touch`, `tree`
- `head`, `tail`, `grep`, `digest`, `wc`, `sort`, `uniq`, `cut`, `tr`, `base`

### Mount Management
- `mount`, `umount`, `list-mounts`, `mount-plugin`, `unmount-plugin`

### Advanced Operations
- `upload`, `download`, `find`, `locate`, `diff`, `du`, `file`
- `ln`, `readlink`, `realpath`, `basename`, `dirname`, `truncate`, `split`
- `rev`, `tac`

### REPL Mode
```bash
evif repl
```

### Environment
- `env`, `export`, `unset`, `pwd`, `cd`, `echo`, `date`, `sleep`, `true`, `false`

## Plugin Development

### Creating a Basic Plugin

```rust
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult};
use async_trait::async_trait;

pub struct MyPlugin {
    // Plugin state
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    async fn create(&self, path: &str, mode: u32) -> EvifResult<()> {
        // Implementation
        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<i32> {
        // Implementation
        Ok(data.len() as i32)
    }

    // Implement other required methods...
}
```

### Creating a Dynamic Plugin

```rust
use evif_core::EvifPlugin;

#[no_mangle]
pub static evif_plugin_abi_version: u32 = 1;

#[no_mangle]
pub extern "C" fn evif_plugin_info() -> *const u8 {
    // Return plugin info JSON
}

#[no_mangle]
pub extern "C" fn evif_plugin_create() -> *mut std::os::raw::c_void {
    // Create and return plugin instance
}
```

For detailed plugin development guide, see [docs/plugin-development.md](docs/plugin-development.md).

## FUSE Integration

Mount EVIF as a userspace filesystem:

```bash
# Mount (read-only by default)
evif mount /mnt/evif

# Mount with write support
evif mount /mnt/evif --write

# Custom cache settings
evif mount /mnt/evif --write --cache-size 5000 --cache-timeout 120

# Unmount
evif umount /mnt/evif
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EVIF_PORT` / `EVIF_REST_PORT` | 8081 | REST API server port |
| `EVIF_HOST` / `EVIF_REST_HOST` | 0.0.0.0 | Server bind address |
| `EVIF_LOG_LEVEL` | info | Logging level |
| `EVIF_CACHE_SIZE` | 10000 | Inode cache size |
| `EVIF_CACHE_TIMEOUT` | 60 | Cache timeout (seconds) |
| `EVIF_CORS_ENABLED` | true | Enable CORS |
| `EVIF_CORS_ORIGINS` | (any) | CORS allowed origins (comma-separated) |
| `EVIF_REST_PRODUCTION_MODE` | false | Enable production mode (strict config checks) |

### CLI Flags

```bash
# REST API server
evif-rest --help
evif-rest --port 3000       # or -p 3000
evif-rest --host 127.0.0.1
evif-rest --production

# MCP server
evif-mcp --help
evif-mcp --url http://localhost:3000
evif-mcp --server-name evif-mcp
```

### Configuration File

EVIF supports configuration via `evif.toml`:

```toml
[server]
port = 8081
host = "0.0.0.0"

[cache]
size = 10000
timeout = 60

[logging]
level = "info"
```

## Project Structure

```
evif/
├── crates/
│   ├── evif-core/        # Core abstractions and plugin system
│   ├── evif-rest/        # REST API server
│   ├── evif-cli/         # CLI tool
│   ├── evif-client/      # Client SDK
│   ├── evif-fuse/        # FUSE integration
│   ├── evif-auth/        # Authentication layer
│   ├── evif-macros/      # Procedural macros
│   ├── evif-metrics/     # Metrics collection
│   ├── evif-mem/         # Optional memory subsystem
│   └── evif-plugins/     # Plugin implementations and catalog
├── docs/                  # Documentation
├── benches/               # Benchmarks
├── tests/                 # Integration tests
├── examples/              # Example code
└── skills/                # Cangjie skills system
```

## Performance

EVIF uses a Radix tree-based mount table for O(k) path lookup, where k is the path length.

### Key Optimizations
- Inode caching for fast attribute lookups
- Directory caching for readdir operations
- Streaming for large file operations
- Concurrent batch operations
- Handle management for efficient file access

## Roadmap

### Completed ✅
- [x] Core plugin system with 30+ plugins
- [x] REST API with full CRUD operations
- [x] CLI tool with 60+ commands
- [x] FUSE integration (Linux/macOS)
- [x] WebSocket support
- [x] Batch operations (copy, delete)
- [x] File monitoring
- [x] ACL support
- [x] Dynamic plugin loading
- [x] Metrics collection (Prometheus)
- [x] WASM plugin support

### In Progress 🚧
- [ ] Two-layer caching system
- [ ] Configurable mount system
- [ ] Vector retrieval optimization
- [ ] Enhanced MCP integration

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p evif-plugins core_supported_plugins
cargo test -p evif-rest

# Run with all features
cargo test --workspace --all-features
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- Inspired by [Plan 9 from Bell Labs](https://9p.io/) and its "Everything Is a File" philosophy
- Inspired by [AGFS](https://github.com/c4pt0r/agfs) - Agent File System by Dongxu Huang (PingCAP co-founder)
- Built with [Rust](https://www.rust-lang.org/)
- Uses [OpenDAL](https://github.com/apache/opendal) for unified storage access

---

**Documentation**: [docs/](docs/) | **API Reference**: [docs/API.md](docs/API.md) | **Getting Started**: [docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)
