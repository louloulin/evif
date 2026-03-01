# Chapter 1: Introduction

## What is EVIF?

**EVIF** (Extensible Virtual File System) is a powerful, extensible virtual filesystem framework written in Rust. It implements the Unix philosophy of "everything is a file", unifying various storage backends, data sources, and services into a consistent filesystem interface.

### Core Philosophy

EVIF's core design philosophy is built on **plugin system** and **unified mount table**:

- **Plugin Architecture**: Each filesystem backend is implemented as an independent plugin
- **Path Abstraction**: Uses Radix tree for efficient longest-prefix matching routing
- **POSIX Compatible**: Provides traditional filesystem-like operations (create, read, write, delete, etc.)
- **Multi-Protocol Support**: Supports REST API, WebSocket, FUSE, gRPC, and more

### Relationship with AGFS

EVIF is similar in design and functionality to AGFS (Another Graph File System), providing:

- **EvifPlugin trait**: Equivalent to AGFS's ServicePlugin/FileSystem
- **RadixMountTable**: Equivalent to AGFS's mount table mechanism
- **Multiple Backend Support**: Memory, local filesystem, cloud storage (S3, Azure, GCS, etc.), databases, and more

## Key Features

### 1. Plugin System

EVIF's core is its powerful plugin system, including 20+ built-in plugins:

**Basic Plugins**:
- `memfs`: In-memory filesystem, ideal for testing and temporary data
- `localfs`: Local filesystem mounting
- `hellofs`: Example plugin

**Cloud Storage Plugins**:
- `s3fs`: AWS S3 integration
- `azureblobfs`: Azure Blob Storage
- `gcsfs`: Google Cloud Storage
- `aliyunossfs`: Alibaba Cloud Object Storage
- `miniofs`: MinIO object storage

**Database Plugins**:
- `sqlfs`: SQL database backend
- `kvfs`: Key-value storage backend

**Special Purpose Plugins**:
- `queuefs`: Queue filesystem
- `streamfs`: Streaming data access
- `httpfs`: HTTP file access
- `proxyfs`: Proxy filesystem

Each plugin implements the unified `EvifPlugin` trait, providing a consistent file operation interface.

### 2. Flexible Mounting Mechanism

Uses Radix tree mount table with support for:

- **Path Mapping**: Mount different plugins to different paths
- **Longest Prefix Matching**: Intelligent routing of file operations to the correct plugin
- **Dynamic Mounting**: Runtime mount and unmount of plugins
- **Nested Mounting**: Support for multi-level path nesting

Example mount structure:
```
/           → (root)
  /mem      → memfs (in-memory filesystem)
  /local    → localfs (local filesystem)
  /s3       → s3fs (AWS S3)
  /local/home/user/docs → localfs (nested mount)
```

### 3. Multiple Access Interfaces

EVIF provides multiple access methods for different use cases:

**REST API**:
- HTTP/HTTPS interface
- JSON request/response
- Complete file operation support
- Port: default 8081

**WebSocket**:
- Real-time terminal interface
- Interactive command support
- Ideal for web application integration

**FUSE**:
- Mount EVIF filesystem as a local directory
- Compatible with standard filesystem tools (ls, cp, cat, etc.)
- Support for caching and performance optimization

**CLI**:
- Command-line tool `evif`
- Supports both REPL and batch modes
- 61+ built-in commands

**gRPC**:
- High-performance RPC interface (in development)
- Support for streaming

### 4. Extensible Architecture

**Dynamic Plugin Loading**:
- Runtime loading of .so/.dylib/.dll plugins
- Standard ABI interface
- Plugin information queries

**Authentication and Security**:
- Capability-based access control
- Multiple authentication mechanisms
- Audit logging

**Monitoring and Metrics**:
- Prometheus metrics integration
- Traffic monitoring
- Operation statistics
- Performance analysis

### 5. High Performance Features

- **Async I/O**: Tokio-based async runtime
- **Concurrency Safe**: Thread-safe with Arc and Mutex
- **Caching Mechanism**: Metadata and directory caching
- **Batch Operations**: Support for batch file operations

### 6. Graph Model Support (Planned)

EVIF includes graph data structures and algorithms layer, with future support for:

- Representing files and directories as graph nodes
- Complex graph queries and traversals
- Relationship analysis and graph construction

## Architecture Overview

### Core Components

EVIF consists of multiple functional modules:

```
evif-core          Core abstractions and infrastructure
  ├── plugin.rs          EvifPlugin trait definition
  ├── radix_mount_table  Radix tree mount table
  ├── server.rs          Server abstraction
  └── handle_manager.rs  Handle management

evif-plugins       Plugin implementation collection
  ├── memfs             In-memory filesystem
  ├── localfs           Local filesystem
  ├── s3fs              S3 filesystem
  └── ...               Other 20+ plugins

evif-rest          REST API server
  ├── handlers          File operation handlers
  ├── compat_fs         Compatibility layer API
  └── ws_handlers       WebSocket handlers

evif-fuse          FUSE integration
evif-client        HTTP client library
evif-cli           Command-line tool
evif-auth          Authentication and authorization
evif-storage        Storage abstraction layer
evif-graph          Graph data structures
evif-runtime        Runtime configuration
evif-metrics        Metrics collection
evif-mcp            MCP server integration
```

### Data Flow

```
Client Request
    ↓
evif-rest (HTTP/WebSocket) / evif-fuse (FUSE) / evif-cli (CLI)
    ↓
RadixMountTable (Path Resolution)
    ↓
EvifPlugin (Plugin Execution)
    ↓
Actual Storage (Memory/Disk/Cloud Storage/Database)
```

### Technology Stack

**Language and Runtime**:
- Rust 1.70+
- Tokio async runtime
- async-trait async traits

**Major Dependencies**:
- serde: Serialization/deserialization
- tokio: Async I/O
- petgraph: Graph algorithms
- dashmap: Concurrent hashmap
- fuser: FUSE bindings
- reqwest: HTTP client

## Use Cases

### 1. Unified Cloud Storage Access

Unify multiple cloud storage services (S3, Azure, GCS) into a local filesystem interface:

```bash
# Mount S3
mount s3fs /my-bucket --bucket my-bucket --region us-east-1

# Mount Azure
mount azureblobfs /azure --account myaccount --container data

# Unified access
ls /my-bucket/documents
cp /my-bucket/file.txt /azure/backup/
```

### 2. Data Processing Pipeline

Build data processing pipelines using queue and stream plugins:

```bash
# Create processing queues
queuefs create /input-queue
queuefs create /output-queue

# Stream processing
streamfs read /input-queue | process | streamfs write /output-queue
```

### 3. Development and Testing

Use in-memory filesystem for rapid testing:

```bash
# Mount in-memory filesystem
mount memfs /test

# Create test data
create /test/config.json '{"key":"value"}'

# Run tests
run_tests.sh

# Cleanup
umount /test  # Data automatically cleaned
```

### 4. Multi-Tenant Filesystem

Mount independent storage backends for different users or teams:

```bash
# Team A uses S3
mount s3fs /team-a --bucket team-a-bucket

# Team B uses local storage
mount localfs /team-b --root /data/team-b

# Team C uses Azure
mount azureblobfs /team-c --account team-c --container files
```

### 5. Backup and Synchronization

Implement cross-storage backup and sync through unified interface:

```bash
# Local to S3
copy /local/documents/* /s3/backup/

# S3 to Azure
copy /s3/data/* /azure/archive/

# Incremental sync
sync /source/ /destination/
```

### 6. FUSE Local Mount

Mount cloud storage as local directory:

```bash
# Start FUSE mount
evif-fuse-mount /mnt/cloud --plugin s3fs --bucket my-bucket

# Use standard tools
ls /mnt/cloud
cp local_file.txt /mnt/cloud/
```

## Project Status

### Currently Available Features

✅ **Core Features**:
- Complete plugin system
- Radix tree mount table
- REST API service
- WebSocket terminal
- FUSE integration
- CLI tool

✅ **Available Plugins**:
- memfs, localfs, hellofs
- s3fs, azureblobfs, gcsfs, aliyunossfs, miniofs
- sqlfs, kvfs
- queuefs, streamfs, httpfs, proxyfs
- And 10+ other special-purpose plugins

✅ **Monitoring and Metrics**:
- Prometheus metrics endpoints
- Traffic monitoring
- Operation statistics

### Features in Development

⚠️ **Graph API**: Graph query functionality is currently a placeholder implementation

⚠️ **Dynamic Mounting**: REST API mount endpoints return "not yet supported"

### Future Plans

🔮 **Enhanced Features**:
- Configuration file support
- Dynamic plugin loading improvements
- Graph query functionality implementation
- gRPC service activation
- Web UI feature expansion

## Why Choose EVIF?

### Compared to Traditional Filesystems

**Advantages**:
- Unified interface to multiple storage types
- No application code changes needed
- Flexible plugin extension mechanism
- Cloud-native design

### Compared to Other Virtual Filesystems

**Unique Features**:
- Rust-implemented memory safety guarantees
- Native asynchronous high performance
- Built-in multiple cloud storage plugins
- Graph model support (planned)
- Multi-protocol access

### Suitable Scenarios

EVIF is particularly suitable for:

- **Cloud Applications**: Need unified access to multiple cloud storages
- **Microservices**: Need flexible storage abstraction
- **Data Processing**: Need to build data pipelines
- **Development Testing**: Need rapid test environment setup
- **Multi-Tenant**: Need isolated storage spaces

## Community and Resources

### Documentation

- 📖 [Getting Started](chapter-2-getting-started.md)
- 🏗️ [Architecture](chapter-3-architecture.md)
- 🔌 [Plugin Development](chapter-5-plugin-development.md)
- 📡 [FUSE Integration](chapter-6-fuse-integration.md)
- 📘 [API Reference](chapter-7-api-reference.md)

### Project Resources

- 📦 Repository: https://github.com/evif/evif
- 🐛 Issue Tracker: https://github.com/evif/evif/issues
- 💬 Discussions: https://github.com/evif/evif/discussions
- 📄 License: MIT OR Apache-2.0

### Related Projects

- **AGFS**: Another Graph File System - Inspiration for EVIF
- **FUSE**: Filesystem in Userspace
- **Tokio**: Rust async runtime

## Next Steps

Ready to get started with EVIF? Check out [Chapter 2: Getting Started](chapter-2-getting-started.md) for installation and basic usage.

Interested in plugin development? Jump to [Chapter 5: Plugin Development](chapter-5-plugin-development.md) to learn how to create custom plugins.

Want to understand the architecture deeply? Read [Chapter 3: Architecture](chapter-3-architecture.md) to learn about system design and component interactions.
