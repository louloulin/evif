# Chapter 10: Advanced Topics

This chapter covers advanced features and techniques for optimizing and extending EVIF in production environments.

## Table of Contents

- [Performance Optimization](#performance-optimization)
- [Advanced Plugin Features](#advanced-plugin-features)
- [Graph Query System](#graph-query-system)
- [MCP Integration](#mcp-integration)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

---

## Performance Optimization

### Caching Strategies

EVIF implements multiple layers of caching to optimize performance.

#### Directory Cache

The directory cache (`DirCache`) improves `readdir` operation performance through:

- **TTL-based Expiration**: Cache entries expire after a configurable time-to-live
- **LRU Eviction**: Least recently used entries are automatically evicted when cache is full
- **Automatic Invalidation**: Cache entries are invalidated on directory modifications

```rust
use evif_fuse::dir_cache::DirCache;

// Create directory cache with 60-second TTL
let cache = DirCache::new(60);

// Cache directory entries
cache.put("/data/".to_string(), entries);

// Retrieve cached entries
if let Some(entries) = cache.get("/data/") {
    // Cache hit - use entries
}

// Invalidate on modification
cache.invalidate("/data/");
```

**Configuration**:
- `ttl_seconds`: Time-to-live for cache entries (default: 60 seconds)
- `max_entries`: Maximum number of cached directories (default: 10,000)

#### Client Cache

The client cache (`ClientCache`) provides node caching for graph operations:

```rust
use evif_client::cache::ClientCache;
use evif_graph::{NodeId, Node, NodeType};

// Create cache with capacity
let cache = ClientCache::new(1000);

// Cache nodes
let id = NodeId::new_v4();
let node = Node::new(NodeType::File, "document.txt");
cache.put(id, node).await;

// Retrieve from cache
if let Some(node) = cache.get(&id).await {
    // Use cached node
}
```

### Batch Operations

Batch operations significantly reduce round-trip overhead:

```rust
// Batch file reads
let paths = vec!["/file1.txt", "/file2.txt", "/file3.txt"];
let contents: Vec<_> = futures::stream::iter(paths)
    .map(|path| async move {
        client.read_file(path).await
    })
    .buffer_unordered(10) // Process 10 concurrently
    .collect()
    .await;
```

### Concurrent Access Patterns

EVIF is designed for concurrent access:

- **Read-Write Locks**: Multiple readers can access data simultaneously
- **Async/Await**: All operations are non-blocking
- **Connection Pooling**: HTTP clients reuse connections

**Example**: Concurrent directory listing

```rust
use tokio::task::JoinSet;

let mut tasks = JoinSet::new();

for path in paths {
    tasks.spawn(async move {
        client.list_directory(path).await
    });
}

while let Some(result) = tasks.join_next().await {
    // Handle each result
}
```

### Metrics and Monitoring

EVIF provides comprehensive metrics through the `evif-metrics` crate:

#### Supported Metric Types

- **Counter**: Monotonically increasing values (request counts, errors)
- **Gauge**: Point-in-time values (memory usage, active connections)
- **Histogram**: Distributions (request latencies, file sizes)

#### Prometheus Integration

```rust
use evif_metrics::{PrometheusMetricsRegistry, MetricType};

let registry = PrometheusMetricsRegistry::new();

// Register a counter
registry.register_counter(
    "evif_requests_total",
    "Total number of requests",
    &["operation", "status"]
);

// Record a value
registry.increment_counter(
    "evif_requests_total",
    &["read", "success"],
    1.0
);

// Export metrics for Prometheus
let metrics = registry.export();
```

**Key Metrics to Monitor**:

- `evif_requests_total`: Total request count
- `evif_request_duration_seconds`: Request latency histogram
- `evif_cache_hits_total`: Cache hit/miss ratio
- `evif_active_handles`: Currently open file handles
- `evif_memory_usage_bytes`: Memory consumption

---

## Advanced Plugin Features

### HandleFS Trait

The `HandleFS` trait provides file handle operations for stateful file access:

```rust
use evif_core::HandleFS;
use evif_protocol::OpenFlags;

pub struct MyPlugin {
    // Internal state
}

impl HandleFS for MyPlugin {
    fn open(&mut self, path: &str, flags: OpenFlags, mode: u32) -> Result<u64, Error> {
        // Open file and return handle ID
        Ok(handle_id)
    }

    fn close(&mut self, handle_id: u64) -> Result<(), Error> {
        // Close file handle
        Ok(())
    }

    fn read(&mut self, handle_id: u64, offset: u64, size: u32) -> Result<Vec<u8>, Error> {
        // Read from file handle
        Ok(data)
    }

    fn write(&mut self, handle_id: u64, offset: u64, data: Vec<u8>) -> Result<u32, Error> {
        // Write to file handle
        Ok(bytes_written)
    }
}
```

**Handle Lifecycle**:
1. **Open**: Client calls `open()` → Server returns handle ID
2. **Use**: Client performs read/write operations using handle ID
3. **Lease Renewal**: Handle automatically expires after lease duration
4. **Close**: Client explicitly closes or handle expires

### Streamer Trait

The `Streamer` trait supports streaming large files efficiently:

```rust
use evif_protocol::stream::{StreamHandle, StreamChunk, StreamConfig};

impl Streamer for MyPlugin {
    fn stream_read(
        &mut self,
        path: &str,
        offset: u64,
        size: u64
    ) -> Result<StreamHandle, Error> {
        // Create stream handle
        let handle = StreamHandle::new();

        // Start background streaming task
        tokio::spawn(async move {
            let mut sequence = 0;
            loop {
                let chunk = read_next_chunk(offset, size).await?;
                if chunk.is_empty() {
                    break;
                }

                send_chunk(StreamChunk::new(handle, sequence, chunk)).await?;
                sequence += 1;
            }

            Ok::<(), Error>(())
        });

        Ok(handle)
    }
}
```

**Streaming Benefits**:
- **Memory Efficiency**: Process large files without loading entirely into memory
- **Progressive Delivery**: Start processing data before entire file is transferred
- **Resume Capability**: Continue interrupted transfers from last chunk

### WASM Plugin Support

EVIF supports WebAssembly plugins through the Extism PDK, enabling:

- **Cross-Language Plugins**: Write plugins in Rust, Go, JavaScript, Python, etc.
- **Sandboxed Execution**: WASM plugins run in isolated environments
- **Portability**: WASM plugins are platform-independent

#### Creating a WASM Plugin

**1. Add Dependencies** (`Cargo.toml`):

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
extism-pdk = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**2. Implement Plugin** (`lib.rs`):

```rust
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Input {
    path: String,
}

#[plugin_fn]
pub fn read_file(input: String) -> FnResult<String> {
    let input: Input = serde_json::from_str(&input)?;

    // Plugin logic here
    let content = format!("Content of {}", input.path);

    Ok(content)
}

#[plugin_fn]
pub fn write_file(input: String) -> FnResult<String> {
    let input: Input = serde_json::from_str(&input)?;

    // Write logic here
    Ok("Success".to_string())
}
```

**3. Build Plugin**:

```bash
cargo build --release --target wasm32-unknown-unknown
```

**4. Mount WASM Plugin**:

```bash
evif mount my-wasm-plugin /mnt/wasm \
  --config '{"wasm_path": "/path/to/plugin.wasm"}'
```

### Plugin Configuration and Validation

Plugins support structured configuration with validation:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPluginConfig {
    pub api_key: String,
    pub cache_size: usize,
    pub timeout: u64,
}

impl Default for MyPluginConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            cache_size: 1000,
            timeout: 30,
        }
    }
}

impl MyPlugin {
    pub fn with_config(config: MyPluginConfig) -> Result<Self, Error> {
        // Validate configuration
        if config.api_key.is_empty() {
            return Err(Error::InvalidConfig("api_key is required"));
        }

        if config.cache_size == 0 {
            return Err(Error::InvalidConfig("cache_size must be > 0"));
        }

        Ok(Self { config })
    }
}
```

---

## Graph Query System

### Overview

EVIF includes a graph query engine (`evif-graph`) for modeling and querying relationships between filesystem entities.

### Graph Architecture

**Components**:
- **Nodes**: Represent files, directories, or metadata
- **Edges**: Represent relationships (parent-child, references, dependencies)
- **Index**: Accelerated lookups for common queries

```rust
use evif_graph::{Graph, Node, NodeId, NodeType, Edge, EdgeType};

// Create a graph
let mut graph = Graph::new();

// Add nodes
let file_id = graph.add_node(Node::new(NodeType::File, "document.txt"));
let dir_id = graph.add_node(Node::new(NodeType::Directory, "/data"));

// Add relationships
graph.add_edge(Edge::new(
    EdgeType::Parent,
    dir_id,
    file_id
));
```

### Query Language

EVIF provides a fluent query builder:

```rust
use evif_graph::{QueryBuilder, QueryExecutor};

// Find all files in a directory
let results = QueryBuilder::new()
    .node_type(NodeType::File)
    .has_parent(dir_id)
    .execute(&graph)?;

// Find all descendants
let descendants = QueryBuilder::new()
    .starting_from(dir_id)
    .traverse(EdgeType::Parent, TraversalDirection::Outgoing)
    .execute(&graph)?;

// Find nodes with specific attributes
let files = QueryBuilder::new()
    .has_attribute("extension", "txt")
    .execute(&graph)?;
```

### Current Implementation Status

The graph system is **experimental** and under active development:

- ✅ Core graph data structures
- ✅ Node and edge CRUD operations
- ✅ Basic query builder
- ⚠️ Advanced traversal algorithms (in progress)
- ⚠️ Persistent storage (planned)
- ❌ Query optimization (not implemented)

**Recommendation**: Use the graph system for prototyping and non-critical applications. For production use, consider mature alternatives like Neo4j or SQLite with recursive queries.

---

## MCP Integration

### Overview

EVIF provides a Model Context Protocol (MCP) server for AI assistant integration, exposing filesystem operations through standardized tools.

### MCP Server Architecture

The `evif-mcp` crate implements the MCP specification:

```rust
use evif_mcp::{EvifMcpServer, McpServerConfig};

let config = McpServerConfig {
    evif_url: "http://localhost:8081".to_string(),
    server_name: "evif-mcp".to_string(),
    version: "1.8.0".to_string(),
};

let server = EvifMcpServer::new(config);
server.run_stdio().await?;
```

### Available Tools

EVIF MCP server provides 17 tools:

#### File Operations

| Tool | Description |
|------|-------------|
| `evif_ls` | List files in a directory |
| `evif_cat` | Read file contents |
| `evif_write` | Write content to a file |
| `evif_mkdir` | Create a directory |
| `evif_rm` | Remove a file or directory |
| `evif_stat` | Get file information |
| `evif_mv` | Move or rename a file |
| `evif_cp` | Copy a file |
| `evif_grep` | Search for text in files |

#### Plugin Management

| Tool | Description |
|------|-------------|
| `evif_mount` | Mount a plugin |
| `evif_unmount` | Unmount a plugin |
| `evif_mounts` | List all mount points |

#### Handle Operations

| Tool | Description |
|------|-------------|
| `evif_open_handle` | Open a file handle |
| `evif_close_handle` | Close a file handle |

#### System Operations

| Tool | Description |
|------|-------------|
| `evif_health` | Check server health |

### Integration with Claude Desktop

**1. Configure Claude Desktop** (`~claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "evif": {
      "command": "evif-mcp",
      "args": ["--url", "http://localhost:8081"]
    }
  }
}
```

**2. Restart Claude Desktop**

**3. Use EVIF tools in conversations**:

```
User: List files in /data directory
Claude: [calls evif_ls tool]
Claude: Found 3 files in /data:
        - document.txt (1024 bytes)
        - image.png (2048 bytes)
        - script.sh (512 bytes)
```

### Tool Definitions

Each tool includes JSON Schema for validation:

```json
{
  "name": "evif_write",
  "description": "Write content to a file",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path to write"
      },
      "content": {
        "type": "string",
        "description": "Content to write"
      },
      "offset": {
        "type": "number",
        "description": "Write offset (-1 for append)"
      }
    },
    "required": ["path", "content"]
  }
}
```

---

## Troubleshooting

### Common Issues and Solutions

#### Issue: Mount Points Not Visible

**Symptoms**: `evif mounts` doesn't show recently mounted plugins

**Diagnosis**:
```bash
# Check mount registry
evif mounts --verbose

# Check plugin logs
journalctl -u evif-rest -f
```

**Solutions**:
1. Verify plugin configuration: `evif validate-plugin <plugin>`
2. Check plugin dependencies: `ldd /path/to/plugin.so`
3. Restart EVIF server: `systemctl restart evif-rest`

#### Issue: High Memory Usage

**Symptoms**: EVIF process consuming >1GB memory

**Diagnosis**:
```bash
# Check memory usage
evif stats

# Inspect cache metrics
curl http://localhost:8081/metrics | grep cache
```

**Solutions**:
1. Reduce cache sizes in configuration
2. Enable cache TTL: `DirCache::new(30)`  # 30 seconds
3. Monitor metrics: Set up Prometheus alerts

#### Issue: Slow Directory Listings

**Symptoms**: `ls` operations take >5 seconds

**Diagnosis**:
```bash
# Check if cache is enabled
curl http://localhost:8081/api/v1/config | grep cache

# Measure individual operations
time evif ls /large/directory
```

**Solutions**:
1. Enable directory caching
2. Increase cache TTL for static directories
3. Use batch operations for multiple listings

#### Issue: File Handle Leaks

**Symptoms**: "Too many open files" error

**Diagnosis**:
```bash
# Check open handles
evif handles --status open

# Check system limits
ulimit -n
```

**Solutions**:
1. Ensure handles are closed: Always call `close_handle()`
2. Reduce handle lease duration
3. Increase system limits: `ulimit -n 4096`

### Debugging Techniques

#### Enable Debug Logging

```bash
# Set log level
RUST_LOG=evif=debug evif-rest

# Enable specific module
RUST_LOG=evif_fuse::dir_cache=trace evif-fuse
```

#### Capture Stack Traces

```bash
# Enable backtraces on panic
RUST_BACKTRACE=1 evif-rest

# Full backtrace
RUST_BACKTRACE=full evif-rest
```

#### Profile Performance

```rust
// Use flamegraph for profiling
use flame::*;

fn main() {
    flame::start("main_operation");

    // Your code here

    flame::end("main_operation");
    flame::dump_svg(&mut File::create("flamegraph.svg").unwrap()).unwrap();
}
```

```bash
# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bin evif-rest
```

---

## Best Practices

### Security Considerations

#### 1. Input Validation

Always validate and sanitize user inputs:

```rust
pub fn safe_path(path: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path);

    // Prevent path traversal
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(Error::InvalidPath("Path traversal detected"));
    }

    // Normalize path
    Ok(path.canonicalize()?)
}
```

#### 2. Principle of Least Privilege

Run EVIF with minimal permissions:

```bash
# Create dedicated user
sudo useradd -r -s /bin/false evif

# Run with dropped privileges
sudo -u evif evif-rest
```

#### 3. Authentication and Authorization

Use the auth system for multi-user deployments:

```rust
use evif_auth::{AuthMiddleware, Role};

let auth = AuthMiddleware::new()
    .with_role(Role::Admin, "/admin/*")
    .with_role(Role::User, "/data/*");
```

### Error Handling Patterns

#### 1. Structured Errors

Define error types for clear error handling:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvifError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

#### 2. Graceful Degradation

Provide fallback behavior for non-critical failures:

```rust
pub fn read_with_cache(&self, path: &str) -> Result<String, Error> {
    // Try cache first
    if let Some(cached) = self.cache.get(path) {
        return Ok(cached);
    }

    // Fall back to disk
    let content = self.read_from_disk(path)?;

    // Update cache (don't fail if this fails)
    let _ = self.cache.put(path, content.clone());

    Ok(content)
}
```

### Production Deployment Tips

#### 1. Configuration Management

Use environment-specific configs:

```bash
# Development
export EVIF_CONFIG=dev.toml
evif-rest

# Production
export EVIF_CONFIG=prod.toml
evif-rest
```

#### 2. Health Checks

Implement proper health checks:

```rust
// GET /health
pub async fn health_check() -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        uptime: get_uptime(),
        metrics: get_current_metrics(),
    })
}
```

#### 3. Graceful Shutdown

Handle shutdown signals properly:

```rust
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```

#### 4. Monitoring Setup

Set up comprehensive monitoring:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'evif'
    static_configs:
      - targets: ['localhost:8081']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

**Alert Rules**:

```yaml
# alerts.yml
groups:
  - name: evif_alerts
    rules:
      - alert: HighErrorRate
        expr: rate(evif_errors_total[5m]) > 10
        annotations:
          summary: "High error rate detected"

      - alert: HighMemoryUsage
        expr: evif_memory_usage_bytes > 1073741824
        annotations:
          summary: "Memory usage > 1GB"
```

### Performance Tuning Checklist

- [ ] Enable directory caching with appropriate TTL
- [ ] Configure client cache size based on available memory
- [ ] Use batch operations for bulk file operations
- [ ] Enable compression for network transfers
- [ ] Tune connection pool size for concurrent requests
- [ ] Profile and optimize hot paths with flamegraphs
- [ ] Set up metrics collection and dashboards
- [ ] Configure alerts for error rates and latency
- [ ] Use WASM plugins for cross-platform functionality
- [ ] Implement graceful degradation for cache failures

---

## Next Steps

Congratulations! You've completed the EVIF documentation. For more information:

- **API Reference**: See [Chapter 7 - API Reference](chapter-7-api-reference.md)
- **Deployment Guide**: See [Chapter 9 - Deployment](chapter-9-deployment.md)
- **Source Code**: https://github.com/evif/evif
- **Issues**: Report bugs at https://github.com/evif/evif/issues

For questions and community support:
- **Discord**: Join our Discord server
- **Discussions**: GitHub Discussions
- **Email**: support@evif.dev
