# EVIF - Everything Is a File

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)

> A powerful, extensible graph-based virtual file system built in Rust, following the Plan 9 "Everything Is a File" philosophy.

[中文文档](README-CN.md)

## Overview

EVIF is a modular virtual file system that provides a unified interface to various storage backends through a plugin architecture. It combines the power of graph data structures with traditional file system semantics, enabling advanced querying capabilities while maintaining POSIX compatibility.

### Key Features

- **Plugin Architecture**: 30+ built-in plugins for various storage backends
- **Graph Engine**: Advanced graph data structures with indexing and query capabilities
- **Multiple Access Methods**: REST API, CLI, FUSE mount, and WebSocket
- **Storage Backends**: Memory, Local FS, S3, Azure Blob, GCS, Aliyun OSS, and more
- **Advanced Features**: Batch operations, streaming, encryption, tiering, and monitoring
- **Dynamic Plugin Loading**: Runtime loading of `.so`/`.dylib`/`.dll` plugins
- **WASM Plugin Support**: WebAssembly-based plugin extensions

## Architecture

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

## Core Components

| Crate | Description |
|-------|-------------|
| **evif-core** | Core abstractions, plugin system, mount table, handle manager |
| **evif-graph** | Graph data structures, nodes, edges, indexing, query execution |
| **evif-storage** | Pluggable storage backends (Memory, Sled, RocksDB, S3) |
| **evif-vfs** | Virtual filesystem abstraction layer |
| **evif-protocol** | Wire protocol definitions and serialization |
| **evif-rest** | HTTP/JSON REST API server |
| **evif-cli** | Command-line interface (60+ commands) |
| **evif-client** | Rust client SDK |
| **evif-fuse** | FUSE filesystem integration (Linux/macOS) |
| **evif-grpc** | gRPC service (currently disabled) |
| **evif-auth** | Authentication and authorization layer |
| **evif-macros** | Procedural macros (`#[node]`, `#[builder]`, `#[error_macro]`) |
| **evif-metrics** | Prometheus metrics collection and export |

## Available Plugins

### Core Plugins
| Plugin | Description | Default Mount |
|--------|-------------|---------------|
| `memfs` | In-memory filesystem | `/mem` |
| `localfs` | Local filesystem access | - |
| `hellofs` | Hello world example plugin | `/hello` |
| `devfs` | Device and system information | `/dev` |
| `serverinfofs` | Server status and metrics | `/serverinfo` |
| `kvfs` | Key-value store interface | `/kv` |
| `queuefs` | Message queue interface | `/queue` |
| `proxyfs` | Proxy to other paths | - |
| `streamfs` | Streaming data interface | - |

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

### Special Purpose Plugins
| Plugin | Description | Feature Flag |
|--------|-------------|--------------|
| `httpfs` | HTTP-based filesystem | - |
| `encryptedfs` | Encrypted filesystem layer | - |
| `tieredfs` | Tiered storage (hot/warm/cold) | - |
| `handlefs` | File handle management | - |
| `heartbeatfs` | Health monitoring | - |
| `sqlfs` | SQL database interface | `sqlfs` |
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
| `EVIF_PORT` | 8081 | REST API server port |
| `EVIF_HOST` | 0.0.0.0 | Server bind address |
| `EVIF_LOG_LEVEL` | info | Logging level |
| `EVIF_CACHE_SIZE` | 10000 | Inode cache size |
| `EVIF_CACHE_TIMEOUT` | 60 | Cache timeout (seconds) |

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
│   ├── evif-graph/       # Graph engine
│   ├── evif-storage/     # Storage backends
│   ├── evif-vfs/         # VFS abstraction
│   ├── evif-protocol/    # Wire protocol
│   ├── evif-rest/        # REST API server
│   ├── evif-cli/         # CLI tool
│   ├── evif-client/      # Client SDK
│   ├── evif-fuse/        # FUSE integration
│   ├── evif-grpc/        # gRPC service (disabled)
│   ├── evif-auth/        # Authentication layer
│   ├── evif-macros/      # Procedural macros
│   ├── evif-metrics/     # Metrics collection
│   └── evif-plugins/     # Plugin implementations (30+)
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
- [ ] Temporal knowledge graph
- [ ] Vector retrieval optimization
- [ ] Enhanced MCP integration

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p evif-graph
cargo test -p evif-storage
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
