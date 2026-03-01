# 第十章：高级主题

本章介绍在生产环境中优化和扩展 EVIF 的高级特性和技术。

## 目录

- [性能优化](#性能优化)
- [高级插件特性](#高级插件特性)
- [图查询系统](#图查询系统)
- [MCP 集成](#mcp-集成)
- [故障排除](#故障排除)
- [最佳实践](#最佳实践)

---

## 性能优化

### 缓存策略

EVIF 实现了多层缓存来优化性能。

#### 目录缓存

目录缓存（`DirCache`）通过以下机制提高 `readdir` 操作性能：

- **基于 TTL 的过期**：缓存条目在可配置的生存时间后过期
- **LRU 淘汰**：当缓存已满时，自动淘汰最近最少使用的条目
- **自动失效**：目录修改时缓存条目自动失效

```rust
use evif_fuse::dir_cache::DirCache;

// 创建 TTL 为 60 秒的目录缓存
let cache = DirCache::new(60);

// 缓存目录条目
cache.put("/data/".to_string(), entries);

// 获取缓存的条目
if let Some(entries) = cache.get("/data/") {
    // 缓存命中 - 使用条目
}

// 修改时失效
cache.invalidate("/data/");
```

**配置选项**：
- `ttl_seconds`：缓存条目的生存时间（默认：60 秒）
- `max_entries`：最大缓存目录数（默认：10,000）

#### 客户端缓存

客户端缓存（`ClientCache`）为图操作提供节点缓存：

```rust
use evif_client::cache::ClientCache;
use evif_graph::{NodeId, Node, NodeType};

// 创建指定容量的缓存
let cache = ClientCache::new(1000);

// 缓存节点
let id = NodeId::new_v4();
let node = Node::new(NodeType::File, "document.txt");
cache.put(id, node).await;

// 从缓存检索
if let Some(node) = cache.get(&id).await {
    // 使用缓存的节点
}
```

### 批量操作

批量操作显著减少往返开销：

```rust
// 批量读取文件
let paths = vec!["/file1.txt", "/file2.txt", "/file3.txt"];
let contents: Vec<_> = futures::stream::iter(paths)
    .map(|path| async move {
        client.read_file(path).await
    })
    .buffer_unordered(10) // 并发处理 10 个
    .collect()
    .await;
```

### 并发访问模式

EVIF 为并发访问而设计：

- **读写锁**：多个读取者可以同时访问数据
- **异步/等待**：所有操作都是非阻塞的
- **连接池**：HTTP 客户端复用连接

**示例**：并发目录列表

```rust
use tokio::task::JoinSet;

let mut tasks = JoinSet::new();

for path in paths {
    tasks.spawn(async move {
        client.list_directory(path).await
    });
}

while let Some(result) = tasks.join_next().await {
    // 处理每个结果
}
```

### 指标和监控

EVIF 通过 `evif-metrics` crate 提供全面的指标：

#### 支持的指标类型

- **Counter**（计数器）：单调递增值（请求数、错误数）
- **Gauge**（仪表）：时刻值（内存使用、活动连接数）
- **Histogram**（直方图）：分布（请求延迟、文件大小）

#### Prometheus 集成

```rust
use evif_metrics::{PrometheusMetricsRegistry, MetricType};

let registry = PrometheusMetricsRegistry::new();

// 注册计数器
registry.register_counter(
    "evif_requests_total",
    "请求总数",
    &["operation", "status"]
);

// 记录值
registry.increment_counter(
    "evif_requests_total",
    &["read", "success"],
    1.0
);

// 导出指标给 Prometheus
let metrics = registry.export();
```

**关键监控指标**：

- `evif_requests_total`：请求总数
- `evif_request_duration_seconds`：请求延迟直方图
- `evif_cache_hits_total`：缓存命中率
- `evif_active_handles`：当前打开的文件句柄
- `evif_memory_usage_bytes`：内存消耗

---

## 高级插件特性

### HandleFS 特性

`HandleFS` trait 为有状态文件访问提供文件句柄操作：

```rust
use evif_core::HandleFS;
use evif_protocol::OpenFlags;

pub struct MyPlugin {
    // 内部状态
}

impl HandleFS for MyPlugin {
    fn open(&mut self, path: &str, flags: OpenFlags, mode: u32) -> Result<u64, Error> {
        // 打开文件并返回句柄 ID
        Ok(handle_id)
    }

    fn close(&mut self, handle_id: u64) -> Result<(), Error> {
        // 关闭文件句柄
        Ok(())
    }

    fn read(&mut self, handle_id: u64, offset: u64, size: u32) -> Result<Vec<u8>, Error> {
        // 从文件句柄读取
        Ok(data)
    }

    fn write(&mut self, handle_id: u64, offset: u64, data: Vec<u8>) -> Result<u32, Error> {
        // 写入文件句柄
        Ok(bytes_written)
    }
}
```

**句柄生命周期**：
1. **打开**：客户端调用 `open()` → 服务器返回句柄 ID
2. **使用**：客户端使用句柄 ID 执行读/写操作
3. **租约续期**：句柄在租约持续时间后自动过期
4. **关闭**：客户端显式关闭或句柄过期

### Streamer 特性

`Streamer` trait 支持高效流式传输大文件：

```rust
use evif_protocol::stream::{StreamHandle, StreamChunk, StreamConfig};

impl Streamer for MyPlugin {
    fn stream_read(
        &mut self,
        path: &str,
        offset: u64,
        size: u64
    ) -> Result<StreamHandle, Error> {
        // 创建流句柄
        let handle = StreamHandle::new();

        // 启动后台流式传输任务
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

**流式传输优势**：
- **内存效率**：无需将整个文件加载到内存即可处理大文件
- **渐进式交付**：在完整文件传输之前开始处理数据
- **恢复能力**：从最后一个块继续中断的传输

### WASM 插件支持

EVIF 通过 Extism PDK 支持 WebAssembly 插件，实现：

- **跨语言插件**：使用 Rust、Go、JavaScript、Python 等编写插件
- **沙箱执行**：WASM 插件在隔离环境中运行
- **可移植性**：WASM 插件与平台无关

#### 创建 WASM 插件

**1. 添加依赖** (`Cargo.toml`)：

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
extism-pdk = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**2. 实现插件** (`lib.rs`)：

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

    // 插件逻辑
    let content = format!("Content of {}", input.path);

    Ok(content)
}

#[plugin_fn]
pub fn write_file(input: String) -> FnResult<String> {
    let input: Input = serde_json::from_str(&input)?;

    // 写入逻辑
    Ok("Success".to_string())
}
```

**3. 构建插件**：

```bash
cargo build --release --target wasm32-unknown-unknown
```

**4. 挂载 WASM 插件**：

```bash
evif mount my-wasm-plugin /mnt/wasm \
  --config '{"wasm_path": "/path/to/plugin.wasm"}'
```

### 插件配置和验证

插件支持结构化配置和验证：

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
        // 验证配置
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

## 图查询系统

### 概述

EVIF 包含图查询引擎（`evif-graph`），用于建模和查询文件系统实体之间的关系。

### 图架构

**组件**：
- **节点**：表示文件、目录或元数据
- **边**：表示关系（父子、引用、依赖）
- **索引**：加速常见查询

```rust
use evif_graph::{Graph, Node, NodeId, NodeType, Edge, EdgeType};

// 创建图
let mut graph = Graph::new();

// 添加节点
let file_id = graph.add_node(Node::new(NodeType::File, "document.txt"));
let dir_id = graph.add_node(Node::new(NodeType::Directory, "/data"));

// 添加关系
graph.add_edge(Edge::new(
    EdgeType::Parent,
    dir_id,
    file_id
));
```

### 查询语言

EVIF 提供流式查询构建器：

```rust
use evif_graph::{QueryBuilder, QueryExecutor};

// 查找目录中的所有文件
let results = QueryBuilder::new()
    .node_type(NodeType::File)
    .has_parent(dir_id)
    .execute(&graph)?;

// 查找所有后代
let descendants = QueryBuilder::new()
    .starting_from(dir_id)
    .traverse(EdgeType::Parent, TraversalDirection::Outgoing)
    .execute(&graph)?;

// 查找具有特定属性的节点
let files = QueryBuilder::new()
    .has_attribute("extension", "txt")
    .execute(&graph)?;
```

### 当前实现状态

图系统处于**实验阶段**，正在积极开发中：

- ✅ 核心图数据结构
- ✅ 节点和边的 CRUD 操作
- ✅ 基本查询构建器
- ⚠️ 高级遍历算法（开发中）
- ⚠️ 持久化存储（计划中）
- ❌ 查询优化（未实现）

**建议**：将图系统用于原型和非关键应用程序。对于生产使用，请考虑成熟的替代方案，如 Neo4j 或支持递归查询的 SQLite。

---

## MCP 集成

### 概述

EVIF 提供模型上下文协议（MCP）服务器用于 AI 助手集成，通过标准化工具公开文件系统操作。

### MCP 服务器架构

`evif-mcp` crate 实现 MCP 规范：

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

### 可用工具

EVIF MCP 服务器提供 17 个工具：

#### 文件操作

| 工具 | 描述 |
|------|------|
| `evif_ls` | 列出目录中的文件 |
| `evif_cat` | 读取文件内容 |
| `evif_write` | 写入内容到文件 |
| `evif_mkdir` | 创建目录 |
| `evif_rm` | 删除文件或目录 |
| `evif_stat` | 获取文件信息 |
| `evif_mv` | 移动或重命名文件 |
| `evif_cp` | 复制文件 |
| `evif_grep` | 在文件中搜索文本 |

#### 插件管理

| 工具 | 描述 |
|------|------|
| `evif_mount` | 挂载插件 |
| `evif_unmount` | 卸载插件 |
| `evif_mounts` | 列出所有挂载点 |

#### 句柄操作

| 工具 | 描述 |
|------|------|
| `evif_open_handle` | 打开文件句柄 |
| `evif_close_handle` | 关闭文件句柄 |

#### 系统操作

| 工具 | 描述 |
|------|------|
| `evif_health` | 检查服务器健康状态 |

### 与 Claude Desktop 集成

**1. 配置 Claude Desktop** (`~claude_desktop_config.json`)：

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

**2. 重启 Claude Desktop**

**3. 在对话中使用 EVIF 工具**：

```
用户：列出 /data 目录中的文件
Claude：[调用 evif_ls 工具]
Claude：在 /data 中找到 3 个文件：
        - document.txt (1024 字节)
        - image.png (2048 字节)
        - script.sh (512 字节)
```

### 工具定义

每个工具包含用于验证的 JSON Schema：

```json
{
  "name": "evif_write",
  "description": "写入内容到文件",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "要写入的文件路径"
      },
      "content": {
        "type": "string",
        "description": "要写入的内容"
      },
      "offset": {
        "type": "number",
        "description": "写入偏移量（-1 表示追加）"
      }
    },
    "required": ["path", "content"]
  }
}
```

---

## 故障排除

### 常见问题和解决方案

#### 问题：挂载点不可见

**症状**：`evif mounts` 不显示最近挂载的插件

**诊断**：
```bash
# 检查挂载注册表
evif mounts --verbose

# 检查插件日志
journalctl -u evif-rest -f
```

**解决方案**：
1. 验证插件配置：`evif validate-plugin <plugin>`
2. 检查插件依赖：`ldd /path/to/plugin.so`
3. 重启 EVIF 服务器：`systemctl restart evif-rest`

#### 问题：高内存使用

**症状**：EVIF 进程消耗 >1GB 内存

**诊断**：
```bash
# 检查内存使用
evif stats

# 检查缓存指标
curl http://localhost:8081/metrics | grep cache
```

**解决方案**：
1. 减少配置中的缓存大小
2. 启用缓存 TTL：`DirCache::new(30)`  # 30 秒
3. 监控指标：设置 Prometheus 告警

#### 问题：目录列表缓慢

**症状**：`ls` 操作耗时 >5 秒

**诊断**：
```bash
# 检查是否启用缓存
curl http://localhost:8081/api/v1/config | grep cache

# 测量单个操作
time evif ls /large/directory
```

**解决方案**：
1. 启用目录缓存
2. 为静态目录增加缓存 TTL
3. 使用批量操作进行多个列表

#### 问题：文件句柄泄漏

**症状**："打开的文件过多" 错误

**诊断**：
```bash
# 检查打开的句柄
evif handles --status open

# 检查系统限制
ulimit -n
```

**解决方案**：
1. 确保句柄已关闭：始终调用 `close_handle()`
2. 减少句柄租约持续时间
3. 增加系统限制：`ulimit -n 4096`

### 调试技巧

#### 启用调试日志

```bash
# 设置日志级别
RUST_LOG=evif=debug evif-rest

# 启用特定模块
RUST_LOG=evif_fuse::dir_cache=trace evif-fuse
```

#### 捕获堆栈跟踪

```bash
# 在 panic 时启用回溯
RUST_BACKTRACE=1 evif-rest

# 完整回溯
RUST_BACKTRACE=full evif-rest
```

#### 性能分析

```rust
// 使用 flamegraph 进行性能分析
use flame::*;

fn main() {
    flame::start("main_operation");

    // 您的代码

    flame::end("main_operation");
    flame::dump_svg(&mut File::create("flamegraph.svg").unwrap()).unwrap();
}
```

```bash
# 生成火焰图
cargo install flamegraph
cargo flamegraph --bin evif-rest
```

---

## 最佳实践

### 安全注意事项

#### 1. 输入验证

始终验证和清理用户输入：

```rust
pub fn safe_path(path: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path);

    // 防止路径遍历
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(Error::InvalidPath("检测到路径遍历"));
    }

    // 规范化路径
    Ok(path.canonicalize()?)
}
```

#### 2. 最小权限原则

以最小权限运行 EVIF：

```bash
# 创建专用用户
sudo useradd -r -s /bin/false evif

# 以降权运行
sudo -u evif evif-rest
```

#### 3. 认证和授权

为多用户部署使用认证系统：

```rust
use evif_auth::{AuthMiddleware, Role};

let auth = AuthMiddleware::new()
    .with_role(Role::Admin, "/admin/*")
    .with_role(Role::User, "/data/*");
```

### 错误处理模式

#### 1. 结构化错误

定义错误类型以进行清晰的错误处理：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvifError {
    #[error("文件未找到：{0}")]
    NotFound(String),

    #[error("权限拒绝：{0}")]
    PermissionDenied(String),

    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
}
```

#### 2. 优雅降级

为非关键故障提供回退行为：

```rust
pub fn read_with_cache(&self, path: &str) -> Result<String, Error> {
    // 首先尝试缓存
    if let Some(cached) = self.cache.get(path) {
        return Ok(cached);
    }

    // 回退到磁盘
    let content = self.read_from_disk(path)?;

    // 更新缓存（如果失败不要失败）
    let _ = self.cache.put(path, content.clone());

    Ok(content)
}
```

### 生产部署技巧

#### 1. 配置管理

使用特定于环境的配置：

```bash
# 开发环境
export EVIF_CONFIG=dev.toml
evif-rest

# 生产环境
export EVIF_CONFIG=prod.toml
evif-rest
```

#### 2. 健康检查

实现适当的健康检查：

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

#### 3. 优雅关闭

正确处理关闭信号：

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

#### 4. 监控设置

设置全面的监控：

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'evif'
    static_configs:
      - targets: ['localhost:8081']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

**告警规则**：

```yaml
# alerts.yml
groups:
  - name: evif_alerts
    rules:
      - alert: HighErrorRate
        expr: rate(evif_errors_total[5m]) > 10
        annotations:
          summary: "检测到高错误率"

      - alert: HighMemoryUsage
        expr: evif_memory_usage_bytes > 1073741824
        annotations:
          summary: "内存使用 > 1GB"
```

### 性能调优清单

- [ ] 启用具有适当 TTL 的目录缓存
- [ ] 根据可用内存配置客户端缓存大小
- [ ] 使用批量操作进行批量文件操作
- [ ] 为网络传输启用压缩
- [ ] 调整连接池大小以处理并发请求
- [ ] 使用火焰图分析和优化热路径
- [ ] 设置指标收集和仪表板
- [ ] 为错误率和延迟配置告警
- [ ] 使用 WASM 插件实现跨平台功能
- [ ] 为缓存故障实现优雅降级

---

## 下一步

恭喜！您已完成 EVIF 文档。欲了解更多信息：

- **API 参考**：见[第七章 - API 参考](chapter-7-api-reference.md)
- **部署指南**：见[第九章 - 部署](chapter-9-deployment.md)
- **源代码**：https://github.com/evif/evif
- **问题**：在 https://github.com/evif/evif/issues 报告错误

如需问题和社区支持：
- **Discord**：加入我们的 Discord 服务器
- **讨论**：GitHub Discussions
- **邮件**：support@evif.dev
