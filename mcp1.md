# EVIF MCP 集成架构与协议转换

> 创建时间：2026-04-30
> 目标：EVIF 作为 MCP 网关的协议转换设计与实现
> 参考：mem33.md 100 插件计划

---

## 一、核心定位

### 1.1 EVIF 与 MCP 的关系

EVIF 是 MCP 的**文件系统实现**——所有 MCP 能力通过 VFS 接口暴露。

| MCP 概念 | EVIF 实现 | 协议转换 |
|---------|-----------|----------|
| MCP Server | EVIF Plugin | 每个插件 → MCP Server |
| MCP Resources | EVIF File System | `/mounts/*` → Resource URI |
| MCP Tools | EVIF File Operations | `read/write/ls/stat` → Tool Call |
| MCP Prompts | EVIF Skills | `/skills/*.md` → Prompt Template |
| MCP Sampling | EVIF Pipeline | `/queue`, `/pipes` → Sampling Request |

### 1.2 两种集成模式

```
模式 A: EVIF 作为 MCP Client
┌─────────────┐    MCP     ┌─────────────┐    REST    ┌─────────────┐
│  Claude     │ ────────→  │  EVIF MCP   │ ─────────→ │  外部服务    │
│  Desktop    │            │  Client     │            │  (GitHub等)  │
└─────────────┘            └─────────────┘            └─────────────┘

模式 B: EVIF 作为 MCP Server (网关)
┌─────────────┐   VFS      ┌─────────────┐   MCP      ┌─────────────┐
│  Claude     │ ────────→  │  EVIF Core  │ ─────────→ │  MCP Client │
│  Desktop    │            │  (Radix FS) │            │  (其他 Agent) │
└─────────────┘            └─────────────┘            └─────────────┘
```

---

## 二、当前实现分析

### 2.1 evif-mcp 架构（2,427 LOC）

**位置**：`crates/evif-mcp/src/lib.rs`

**核心组件**：

```rust
pub struct EvifMcpServer {
    config: McpServerConfig,           // EVIF URL, Server Name
    client: Client,                    // HTTP Client (reqwest)
    tools: Arc<RwLock<Vec<Tool>>>,     // 26 个 MCP Tools
    prompts: Arc<RwLock<Vec<Prompt>>>, // 3 个 Prompts
    resources: Arc<RwLock<Vec<Resource>>>, // 1 个 Resource
}
```

**协议实现**：
- JSON-RPC 2.0 over stdio
- Protocol Version: 2024-11-05
- 支持 `initialize`, `tools/list`, `resources/list`, `prompts/list`, `ping`, `shutdown`

### 2.2 当前 26 个 Tools

| 类别 | 工具 | VFS 操作映射 |
|------|------|-------------|
| **文件操作** | `evif_ls`, `evif_cat`, `evif_write`, `evif_mkdir`, `evif_rm`, `evif_stat`, `evif_mv`, `evif_cp` | `readdir`, `read`, `write`, `mkdir`, `remove`, `stat`, `rename`, `copy` |
| **插件管理** | `evif_mount`, `evif_unmount`, `evif_mounts` | Mount Table 操作 |
| **Handle** | `evif_open_handle`, `evif_close_handle` | Handle Manager |
| **记忆系统** | `evif_memorize`, `evif_retrieve` | VectorFS |
| **技能系统** | `evif_skill_list`, `evif_skill_info`, `evif_skill_execute` | SkillFS |
| **会话管理** | `evif_session_save`, `evif_session_list` | ContextFS L0/L1 |
| **子 Agent** | `evif_subagent_create`, `evif_subagent_send`, `evif_subagent_list` | PipeFS |
| **工具** | `evif_health`, `evif_grep`, `evif_claude_md_generate` | REST API |

### 2.3 协议转换模式

当前实现使用 **HTTP REST 桥接**：

```rust
// MCP Tool Call → REST API
"evif_ls" → GET /api/v1/fs/list?path={path}
"evif_cat" → GET /api/v1/fs/read?path={path}
"evif_write" → POST /api/v1/fs/write?path={path}
"evif_mkdir" → POST /api/v1/directories
```

**优点**：
- 实现简单，复用现有 REST API
- 支持横向扩展（EVIF Server 集群）

**缺点**：
- 额外网络跳数（stdio → HTTP → VFS）
- 延迟增加（~5-10ms per call）
- JSON-RPC 到 REST 的语义损失

---

## 三、协议转换设计

### 3.1 直接 VFS 模式（推荐）

绕过 REST，直接调用 VFS 操作：

```rust
// 新架构：MCP Tool → VFS Operation
pub async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
    match tool_name {
        "evif_ls" => {
            let path = args["path"].as_str().unwrap();
            let entries = self.vfs.readdir(path).await?;
            Ok(json!({ "entries": entries }))
        }
        "evif_cat" => {
            let path = args["path"].as_str().unwrap();
            let content = self.vfs.read(path).await?;
            Ok(json!({ "content": content }))
        }
        // ... 其他工具
    }
}
```

**优势**：
- 延迟降低 50%+（无 HTTP 开销）
- 语义完整保留
- 支持事务操作

### 3.2 Tool → Plugin 映射

每个 MCP Server 能力映射到 EVIF Plugin：

| MCP Server | EVIF Plugin | 挂载点 |
|------------|-------------|--------|
| GitHub MCP | `githubfs` | `/github` |
| Slack MCP | `slackfs` | `/slack` |
| Notion MCP | `notionfs` | `/notion` |
| PostgreSQL MCP | `postgresfs` | `/postgres` |
| S3 MCP | `s3fs` | `/s3` |

### 3.3 MCP Resource → VFS Path

```
MCP Resource URI                    →  EVIF VFS Path
─────────────────────────────────────────────────────
file:///context/L0/current          →  /context/L0/current
file:///skills/code-review/SKILL.md →  /skills/code-review/SKILL.md
file:///pipes/agent-1/output        →  /pipes/agent-1/output
github://owner/repo/issues          →  /github/owner/repo/issues
```

---

## 四、MCP 能力覆盖矩阵

### 4.1 当前覆盖

| MCP 能力 | EVIF 实现 | 状态 |
|----------|-----------|------|
| tools/call | 26 tools | ✅ |
| tools/list | ✅ | ✅ |
| resources/list | 1 resource | ✅ |
| resources/read | `/files` endpoint | ✅ |
| prompts/list | 3 prompts | ✅ |
| prompts/get | ❌ | ❌ |
| sampling/create | `/queue` | ⚠️ 部分 |
| roots/list | ❌ | ❌ |

### 4.2 缺失能力

| 能力 | 说明 | 实现优先级 |
|------|------|-----------|
| `prompts/get` | 获取 Prompt 详情并渲染 | P1 |
| `roots/list` | 列出工作空间根目录 | P2 |
| `sampling/create` | 完整 LLM 采样支持 | P1 |
| `logging/setLevel` | 日志级别控制 | P3 |

---

## 五、MCP 网关插件设计

### 5.1 McpGateway Plugin

```rust
// crates/evif-plugins/src/mcp_gateway.rs
pub struct McpGatewayPlugin {
    registry: Arc<RwLock<HashMap<String, McpServerConfig>>>,
    session_manager: SessionManager,
    tool_cache: ToolCache,
}

#[async_trait]
impl EvifPlugin for McpGatewayPlugin {
    // 挂载点: /mcp

    async fn readdir(&self, path: &str) -> Result<Vec<DirEntry>> {
        match path {
            "/mcp" => Ok(vec![
                DirEntry::dir("servers"),
                DirEntry::dir("sessions"),
                DirEntry::dir("tools"),
            ]),
            "/mcp/servers" => self.list_registered_servers().await,
            "/mcp/sessions" => self.list_active_sessions().await,
            "/mcp/tools" => self.list_all_tools().await,
            _ => Err(EvifError::NotFound),
        }
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        // /mcp/servers/{name}/config.json → Server Config
        // /mcp/sessions/{id}/status.json → Session Status
        // /mcp/tools/{name}/schema.json → Tool Schema
        unimplemented!()
    }
}
```

### 5.2 McpServer Plugin（外部 MCP Server 接入）

```rust
// 将外部 MCP Server 作为 EVIF Plugin 暴露
pub struct McpServerPlugin {
    server_url: String,
    protocol_version: String,
    tools: Vec<Tool>,
    resources: Vec<Resource>,
    prompts: Vec<Prompt>,
}

#[async_trait]
impl EvifPlugin for McpServerPlugin {
    // 挂载点: /mcp/{server_name}

    async fn readdir(&self, path: &str) -> Result<Vec<DirEntry>> {
        match path {
            "/mcp/github" => Ok(vec![
                DirEntry::dir("repos"),
                DirEntry::dir("issues"),
                DirEntry::dir("prs"),
                DirEntry::dir("actions"),
            ]),
            "/mcp/github/repos" => self.list_repos().await,
            "/mcp/github/issues" => self.list_issues().await,
            _ => Err(EvifError::NotFound),
        }
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        // GitHub 语义映射
        match path {
            "/mcp/github/repos/{owner}/{repo}" => {
                self.github.get_repo(owner, repo).await
            }
            "/mcp/github/issues/{owner}/{repo}/{number}" => {
                self.github.get_issue(owner, repo, number).await
            }
            _ => Err(EvifError::NotFound),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        // 写操作 → MCP Tool Call
        match path {
            "/mcp/github/issues/{owner}/{repo}/new" => {
                self.github.create_issue(data).await
            }
            _ => Err(EvifError::PermissionDenied),
        }
    }
}
```

---

## 六、协议转换实现

### 6.1 JSON-RPC → VFS 适配器

```rust
// crates/evif-mcp/src/vfs_adapter.rs
pub struct VfsAdapter {
    mount_table: Arc<RadixMountTable>,
    auth: Arc<AuthMiddleware>,
}

impl VfsAdapter {
    /// 将 MCP Tool Call 转换为 VFS 操作
    pub fn tool_to_vfs(tool: &Tool, args: Value) -> Result<VfsOperation> {
        let path = extract_path(&tool.name, &args)?;
        let operation = match tool.name.as_str() {
            "evif_ls" => VfsOperation::Readdir(path),
            "evif_cat" | "evif_stat" => VfsOperation::Read(path),
            "evif_write" => VfsOperation::Write(path, extract_content(&args)?),
            "evif_mkdir" => VfsOperation::Mkdir(path, extract_mode(&args)?),
            "evif_rm" => VfsOperation::Remove(path, extract_recursive(&args)?),
            "evif_mv" => VfsOperation::Rename(
                extract_old_path(&args)?,
                extract_new_path(&args)?,
            ),
            _ => return Err(ConversionError::UnknownTool(tool.name.clone())),
        };
        Ok(operation)
    }

    /// 将 MCP Tool 结果转换为 JSON-RPC 响应
    pub fn vfs_result_to_jsonrpc(
        result: VfsResult,
        tool: &Tool,
    ) -> Value {
        match result {
            VfsResult::Entries(entries) => json!({
                "entries": entries.into_iter().map(|e| json!({
                    "name": e.name,
                    "type": e.file_type,
                    "size": e.size,
                })).collect::<Vec<_>>()
            }),
            VfsResult::Content(bytes) => json!({
                "content": String::from_utf8_lossy(&bytes),
            }),
            VfsResult::Stat(stat) => json!({
                "path": stat.path,
                "size": stat.size,
                "modified": stat.modified,
                "created": stat.created,
            }),
        }
    }
}
```

### 6.2 VFS → MCP Protocol 适配器

```rust
impl VfsAdapter {
    /// 将 VFS Path 转换为 MCP Resource URI
    pub fn path_to_resource(path: &str) -> String {
        format!("file://{}", path)
    }

    /// 将 VFS Entry 转换为 MCP Resource
    pub fn entry_to_resource(entry: DirEntry) -> Resource {
        let mime = match entry.file_type {
            FileType::Directory => "inode/directory",
            FileType::File => "text/plain",
            FileType::Symlink => "inode/symlink",
        };
        Resource {
            uri: Self::path_to_resource(&entry.path),
            name: entry.name,
            description: format!("EVIF filesystem: {}", entry.path),
            mime_type: mime.to_string(),
        }
    }

    /// 将 SKILL.md 转换为 MCP Prompt
    pub fn skill_to_prompt(skill: &Skill) -> Prompt {
        Prompt {
            name: skill.name.clone(),
            description: skill.description.clone(),
            arguments: skill
                .parameters
                .iter()
                .map(|p| PromptArgument {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    required: p.required,
                    argument_type: p.r#type.clone(),
                })
                .collect(),
        }
    }
}
```

---

## 七、性能优化

### 7.1 Tool Cache

```rust
pub struct ToolCache {
    cache: Arc< RwLock<LruCache<String, Vec<Tool>>>>,
    ttl: Duration,
}

impl ToolCache {
    pub async fn get_or_fetch(
        &self,
        server: &str,
        fetch: impl Future<Output = Result<Vec<Tool>>>,
    ) -> Result<Vec<Tool>> {
        if let Some(tools) = self.cache.read().await.get(server) {
            return Ok(tools.clone());
        }
        let tools = fetch.await?;
        self.cache.write().await.put(server.to_string(), tools.clone());
        Ok(tools)
    }
}
```

### 7.2 Batch Operations

```rust
// MCP Batch Tool Call → VFS Batch Operation
pub async fn call_batch(&self, calls: Vec<ToolCall>) -> Vec<Result<Value>> {
    // 并行执行独立操作
    let futures = calls.into_iter().map(|call| async {
        self.call_tool(&call.name, call.arguments).await
    });
    futures::future::join_all(futures).await
}
```

---

## 八、安全模型

### 8.1 MCP 认证

```rust
// 每个 MCP Server 需要独立认证
pub struct McpAuth {
    tokens: HashMap<String, McpToken>,
}

impl McpAuth {
    pub async fn authenticate(
        &self,
        server: &str,
        token: &str,
    ) -> Result<McpSession> {
        // 验证 MCP Server Token
        let mcp_token = self.tokens.get(server)
            .ok_or(AuthError::UnknownServer)?;

        if !mcp_token.validate(token) {
            return Err(AuthError::InvalidToken);
        }

        // 创建会话
        Ok(McpSession {
            server: server.to_string(),
            capabilities: mcp_token.capabilities.clone(),
            expires_at: Utc::now() + Duration::hours(1),
        })
    }
}
```

### 8.2 能力边界

```rust
// MCP Tool → EVIF Capability 映射
pub fn tool_capabilities(tool: &Tool) -> Vec<Capability> {
    match tool.name.as_str() {
        "evif_write" => vec![
            Capability::Write,
            Capability::Path(tool.args["path"].clone()),
        ],
        "evif_memorize" => vec![
            Capability::Memory,
            Capability::VectorSearch,
        ],
        _ => vec![Capability::Read],
    }
}
```

---

## 九、测试策略

### 9.1 MCP 协议测试

```rust
#[tokio::test]
async fn test_mcp_tools_list() {
    let server = EvifMcpServer::new(default_config());
    let tools = server.list_tools().await;

    assert!(tools.len() >= 26);
    assert!(tools.iter().any(|t| t.name == "evif_ls"));
    assert!(tools.iter().any(|t| t.name == "evif_memorize"));
}

#[tokio::test]
async fn test_vfs_to_mcp_conversion() {
    let adapter = VfsAdapter::new();
    let tool = Tool {
        name: "evif_ls".to_string(),
        description: "List directory".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        }),
    };

    let args = json!({ "path": "/skills" });
    let op = VfsAdapter::tool_to_vfs(&tool, args).unwrap();

    assert!(matches!(op, VfsOperation::Readdir(_)));
}
```

### 9.2 集成测试

```rust
#[tokio::test]
async fn test_mcp_end_to_end() {
    // 启动 EVIF Server
    let evif = TestEvifServer::new().await;
    let mcp = EvifMcpServer::new(evif.url());

    // MCP 工具调用 → VFS 操作
    let result = mcp.call_tool("evif_ls", json!({ "path": "/" })).await;
    assert!(result.is_ok());
}
```

---

## 十、路线图

### Phase 1: 直接 VFS 集成（P0）

| 任务 | 工作量 | 说明 |
|------|--------|------|
| 重构 `call_tool` 使用 VFS 直接调用 | 4h | 移除 HTTP 桥接 |
| 实现 `prompts/get` | 2h | Prompt 渲染 |
| 实现 `roots/list` | 2h | 工作空间根目录 |
| Tool Cache 优化 | 2h | LRU 缓存 |

### Phase 2: MCP 网关（P1）

| 任务 | 工作量 | 说明 |
|------|--------|------|
| McpGateway Plugin | 8h | 统一 MCP 入口 |
| McpServerPlugin 模板 | 8h | 外部 MCP 接入 |
| 认证集成 | 4h | MCP Token 验证 |

### Phase 3: 100 MCP Server 集成（P2）

根据 mem33.md 计划，逐步实现各服务插件：

| 优先级 | 插件 | 工作量 |
|--------|------|--------|
| P0 | githubfs, slackfs, notionfs | 各 3-4 天 |
| P1 | postgresfs, s3fs, gmailfs | 各 2-3 天 |
| P2 | teamsfs, telegramfs, shopifyfs | 各 2-3 天 |

---

## 十一、配置体系设计

### 11.1 EVIF 现有配置模型

EVIF 使用环境变量 + 配置文件双轨配置：

```bash
# ServerConfig (evif-rest/src/server.rs)
EVIF_REST_HOST=0.0.0.0
EVIF_REST_PORT=8081
EVIF_CORS_ENABLED=true
EVIF_CORS_ORIGINS=https://example.com
EVIF_TLS_CERT_FILE=/path/to/cert.pem
EVIF_TLS_KEY_FILE=/path/to/key.pem

# McpServerConfig (evif-mcp/src/lib.rs)
EVIF_URL=http://localhost:8081
```

### 11.2 MCP 配置扩展

**统一配置位置**：`/etc/evif/mcp.toml` 或 `~/.evif/mcp.toml`

```toml
# MCP Server 全局配置
[mcp]
# 协议版本
protocol_version = "2024-11-05"

# 服务器标识
server_name = "evif-mcp"
version = "1.8.0"

# EVIF 后端连接
[evif]
url = "http://localhost:8081"
timeout_ms = 30000
retry_attempts = 3

# 认证配置
[auth]
type = "bearer"  # bearer | api_key | oauth
token_file = "/etc/evif/mcp-token"

# TLS 配置（可选）
[tls]
enabled = false
cert_file = "/etc/evif/mcp-cert.pem"
key_file = "/etc/evif/mcp-key.pem"
```

### 11.3 MCP Server 注册配置

**配置每个外部 MCP Server 的连接**：

```toml
# MCP Server 注册表
[[servers]]

[[servers.github]]
name = "github"
mount_path = "/mcp/github"
url = "https://api.github.com"
auth_token_env = "GITHUB_TOKEN"
tools = ["repos", "issues", "prs", "actions"]
enabled = true

[[servers.slack]]
name = "slack"
mount_path = "/mcp/slack"
url = "https://slack.com/api"
auth_token_env = "SLACK_BOT_TOKEN"
tools = ["channels", "messages", "files"]
enabled = true

[[servers.notion]]
name = "notion"
mount_path = "/mcp/notion"
url = "https://api.notion.com/v1"
auth_token_env = "NOTION_TOKEN"
tools = ["pages", "databases", "blocks"]
enabled = false  # 需要手动启用
```

### 11.4 VFS 路径映射配置

**MCP Resource URI ↔ VFS Path 映射规则**：

```toml
# 路径映射规则
[mappings]

[mappings.resources]
"file:///context" = "/context"
"file:///skills" = "/skills"
"file:///pipes" = "/pipes"
"file:///memories" = "/memories"
"github://" = "/mcp/github"
"notion://" = "/mcp/notion"

[mappings.tools]
"evif_ls" = { operation = "readdir", path_param = "path" }
"evif_cat" = { operation = "read", path_param = "path" }
"evif_write" = { operation = "write", path_param = "path", content_param = "content" }
"evif_mkdir" = { operation = "mkdir", path_param = "path" }
"evif_memorize" = { operation = "memory_store", backend = "vector" }
"evif_skill_execute" = { operation = "skill_run", path = "/skills/{name}" }

[mappings.prompts]
"file_explorer" = { path = "/skills/file-explorer/SKILL.md" }
"batch_operations" = { path = "/skills/batch-ops/SKILL.md" }
"data_analysis" = { path = "/skills/data-analysis/SKILL.md" }
```

### 11.5 多租户 MCP 配置

**按租户隔离 MCP 配置**：

```toml
# 租户级 MCP 配置
[tenants]

[tenants.acme-corp]
mcp_servers = ["github", "slack", "notion"]
allowed_paths = ["/context/*", "/skills/*", "/mcp/github/acme/*"]
rate_limit = { requests_per_minute = 1000 }

[tenants.startup-inc]
mcp_servers = ["github", "notion"]
allowed_paths = ["/context/*", "/skills/*"]
rate_limit = { requests_per_minute = 500 }
```

### 11.6 配置加载优先级

```
命令行参数 > 环境变量 > 配置文件 > 默认值

优先级顺序（高到低）：
1. CLI args (--evif-url, --server-name)
2. EVIF_URL, EVIF_MCP_SERVER_NAME 环境变量
3. ~/.evif/mcp.toml 或 /etc/evif/mcp.toml
4. McpServerConfig::default() 硬编码默认值
```

### 11.7 配置验证与热重载

```rust
// 配置验证
pub struct McpConfigValidator;

impl McpConfigValidator {
    pub fn validate(config: &McpConfig) -> Result<(), ConfigError> {
        // 1. 检查必需字段
        if config.evif.url.is_empty() {
            return Err(ConfigError::MissingField("evif.url"));
        }

        // 2. 验证 URL 格式
        if !config.evif.url.starts_with("http://")
            && !config.evif.url.starts_with("https://") {
            return Err(ConfigError::InvalidUrl(config.evif.url));
        }

        // 3. 验证服务器注册
        for server in &config.servers {
            if server.mount_path.starts_with("/mcp/") {
                return Err(ConfigError::InvalidMountPath(server.name));
            }
        }

        // 4. 验证认证配置
        for server in &config.servers {
            if server.auth_token_env.is_some() {
                let token = std::env::var(server.auth_token_env.as_ref().unwrap());
                if token.is_err() {
                    return Err(ConfigError::MissingEnvToken(
                        server.auth_token_env.clone().unwrap()
                    ));
                }
            }
        }

        Ok(())
    }
}

// 热重载机制
pub struct McpConfigWatcher {
    watcher: notify::Watcher,
    reload_tx: Arc<Mutex<oneshot::Sender<()>>>,
}

impl McpConfigWatcher {
    pub fn start(&self, config_path: &Path) {
        // 监听配置文件变化
        // 触发配置重载
        // 通知 MCP Server 刷新工具列表
    }
}
```

### 11.8 与 EVIF Config 系统集成

```rust
// 在 EVIF Core 中统一配置管理
pub struct EvifConfig {
    // REST Server 配置
    pub rest: ServerConfig,

    // MCP Server 配置
    pub mcp: McpConfig,

    // 插件配置
    pub plugins: HashMap<String, PluginConfig>,

    // 多租户配置
    pub tenants: HashMap<String, TenantConfig>,
}

impl EvifConfig {
    /// 从多个来源加载配置
    pub fn load() -> Result<Self> {
        let rest = ServerConfig::default();  // 环境变量
        let mcp = McpConfig::load()?;        // 配置文件 + 环境变量
        let plugins = PluginConfig::load_all()?;
        let tenants = TenantConfig::load_all()?;

        Ok(Self { rest, mcp, plugins, tenants })
    }

    /// 导出配置到 YAML
    pub fn export_yaml(&self) -> String {
        // 用于配置备份和迁移
    }
}
```

---

## 十二、关键文件

| 文件 | 说明 |
|------|------|
| `crates/evif-mcp/src/lib.rs` | 当前 MCP Server 实现（2,427 LOC） |
| `crates/evif-rest/src/server.rs` | REST Server 配置（ServerConfig） |
| `crates/evif-plugins/src/contextfs.rs` | Context 层实现 |
| `crates/evif-plugins/src/skillfs.rs` | SkillFS 实现 |
| `crates/evif-core/src/radix_mount_table.rs` | Radix Mount Table |
| `crates/evif-rest/src/routes.rs` | REST API 定义 |

---

## 十三、总结

EVIF 作为 MCP 网关的核心价值：

1. **统一接口**：所有外部服务通过 VFS 访问
2. **协议透明**：MCP ↔ VFS 自动转换
3. **能力复用**：现有插件系统即 MCP 能力
4. **性能优化**：直接 VFS 调用 vs HTTP 桥接
5. **配置集成**：与 EVIF 统一配置体系融合

**推荐实施**：
1. **短期**：重构直接 VFS 集成，降低延迟；定义 McpConfig 结构
2. **中期**：实现 McpGateway Plugin + 配置文件支持
3. **长期**：按 mem33.md 计划，逐步接入 100 个 MCP Server

**配置优先级**：CLI args > 环境变量 > 配置文件 > 默认值
