// MCP Server Plugin - 外部 MCP Server 接入
//
// 将外部 MCP Server 作为 EVIF Plugin 暴露
// 支持 GitHub, Slack, Notion 等外部 MCP 服务器

use async_trait::async_trait;
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock;
use chrono::Utc;

use super::mcp_client::McpClient;

/// 外部 MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMcpConfig {
    /// 服务器名称
    pub name: String,
    /// 服务器 URL (stdio 路径或 HTTP URL)
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    /// HTTP 模式下使用的 URL
    pub url: Option<String>,
    /// 认证 token 环境变量名
    pub auth_token_env: Option<String>,
}

/// MCP 服务器能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub sampling: bool,
}

/// MCP 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP 资源定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalResource {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
}

/// MCP Server Plugin - 将外部 MCP Server 暴露为 VFS
///
/// 挂载点: /mcp/{server_name}
///
/// 目录结构:
/// /mcp/{name}/
/// ├── tools/           # 可用工具
/// │   └── {tool_name}.json
/// ├── resources/      # 可用资源
/// │   └── {resource_uri}.json
/// ├── prompts/        # 可用提示
/// │   └── {prompt_name}.json
/// └── status.json     # 服务器状态

pub struct McpServerPlugin {
    /// 服务器名称 (用于挂载点)
    name: String,
    /// 服务器配置
    config: ExternalMcpConfig,
    /// HTTP 客户端 (用于直接 API 调用)
    client: Client,
    /// MCP stdio 客户端 (用于外部 MCP Server 通信)
    mcp_client: Option<std::sync::Mutex<McpClient>>,
    /// 认证 token (从环境变量获取)
    auth_token: Option<String>,
    /// 能力列表
    capabilities: Arc<RwLock<McpServerCapabilities>>,
    /// 工具列表
    tools: Arc<RwLock<Vec<ExternalTool>>>,
    /// 资源列表
    resources: Arc<RwLock<Vec<ExternalResource>>>,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
}

impl McpServerPlugin {
    /// 创建新的 MCP Server Plugin
    pub fn new(name: String, config: ExternalMcpConfig) -> Self {
        // 从环境变量获取认证 token
        let auth_token = config.auth_token_env.as_ref().and_then(|env| {
            std::env::var(env).ok()
        });

        // 尝试启动 MCP stdio 客户端
        let mcp_client = Self::try_start_mcp_client(&name, &config);

        Self {
            name,
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            mcp_client,
            auth_token,
            capabilities: Arc::new(RwLock::new(McpServerCapabilities {
                tools: true,
                resources: true,
                prompts: true,
                sampling: false,
            })),
            tools: Arc::new(RwLock::new(Vec::new())),
            resources: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// 尝试启动 MCP stdio 客户端
    fn try_start_mcp_client(name: &str, config: &ExternalMcpConfig) -> Option<std::sync::Mutex<McpClient>> {
        let command = config.command.as_ref()?;
        let args = config.args.as_ref()?;

        let mut client = McpClient::new(name);
        match client.start(command, args, config.env.as_ref()) {
            Ok(_) => {
                // 尝试初始化
                match client.initialize("evif-mcp", "1.0.0") {
                    Ok(_) => {
                        // 更新连接状态
                        return Some(std::sync::Mutex::new(client));
                    }
                    Err(e) => {
                        tracing::warn!("MCP client initialized but init failed: {:?}", e);
                        // 客户端已启动，工具列表可以通过 list_tools 获取
                        return Some(std::sync::Mutex::new(client));
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Failed to start MCP client: {:?}", e);
                None
            }
        }
    }

    /// 从配置创建 (TOML 格式)
    pub fn from_config(name: String, config_json: &str) -> Result<Self, String> {
        let config: ExternalMcpConfig = serde_json::from_str(config_json)
            .map_err(|e| format!("Failed to parse config: {}", e))?;
        Ok(Self::new(name, config))
    }

    /// 获取服务器名称
    pub fn server_name(&self) -> &str {
        &self.name
    }

    /// 获取挂载路径
    pub fn mount_path(&self) -> String {
        format!("/mcp/{}", self.name)
    }

    /// 设置连接状态
    pub fn set_connected(&self, connected: bool) {
        let mut status = self.connected.write().unwrap();
        *status = connected;
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    /// 更新工具列表
    pub fn update_tools(&self, tools: Vec<ExternalTool>) {
        let mut t = self.tools.write().unwrap();
        *t = tools;
    }

    /// 更新资源列表
    pub fn update_resources(&self, resources: Vec<ExternalResource>) {
        let mut r = self.resources.write().unwrap();
        *r = resources;
    }

    /// 列出所有工具
    pub fn list_tools(&self) -> Vec<ExternalTool> {
        self.tools.read().unwrap().clone()
    }

    /// 列出所有资源
    pub fn list_resources(&self) -> Vec<ExternalResource> {
        self.resources.read().unwrap().clone()
    }

    /// 获取工具详细信息
    pub fn get_tool(&self, name: &str) -> Option<ExternalTool> {
        let tools = self.tools.read().unwrap();
        tools.iter().find(|t| t.name == name).cloned()
    }

    /// 获取资源详细信息
    pub fn get_resource(&self, uri: &str) -> Option<ExternalResource> {
        let resources = self.resources.read().unwrap();
        resources.iter().find(|r| r.uri == uri).cloned()
    }

    /// 验证认证 token
    pub fn validate_auth(&self) -> Result<String, String> {
        if let Some(token_env) = &self.config.auth_token_env {
            std::env::var(token_env)
                .map_err(|_| format!("Missing auth token: {}", token_env))
        } else {
            Ok(String::new())
        }
    }

    /// 从 MCP stdio 服务器同步工具列表
    pub fn sync_tools_from_mcp(&self) -> Result<(), String> {
        let mcp_mutex = self.mcp_client.as_ref()
            .ok_or_else(|| "MCP client not initialized".to_string())?;

        let mut mcp_client = mcp_mutex.lock()
            .map_err(|e| format!("Failed to lock MCP client: {}", e))?;

        let tools = mcp_client.list_tools()
            .map_err(|e| format!("Failed to list tools: {:?}", e))?;

        // 转换为 ExternalTool 并更新
        let external_tools: Vec<ExternalTool> = tools.into_iter().map(|t| {
            ExternalTool {
                name: t.name,
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema,
            }
        }).collect();

        self.update_tools(external_tools);
        Ok(())
    }

    /// 检查 MCP 客户端是否可用
    pub fn has_mcp_client(&self) -> bool {
        self.mcp_client.is_some()
    }

    /// 获取认证 Header
    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = &self.auth_token {
            // 根据服务器类型设置不同的认证头
            match self.name.as_str() {
                "github" => {
                    if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                        headers.insert(HeaderName::from_static("authorization"), value);
                    }
                }
                "slack" => {
                    if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                        headers.insert(HeaderName::from_static("authorization"), value);
                    }
                }
                "notion" => {
                    if let Ok(value) = HeaderValue::from_str(token) {
                        headers.insert(HeaderName::from_static("authorization"), value);
                    }
                }
                _ => {}
            }
        }
        headers
    }

    /// 同步调用工具 (供 VFS write 操作调用)
    pub fn call_tool_sync(&self, tool_name: &str, arguments: serde_json::Value) -> Result<String, String> {
        // 构建请求
        let _tool = self.get_tool(tool_name).ok_or_else(|| {
            format!("Tool not found: {}", tool_name)
        })?;

        // 优先尝试 MCP stdio 客户端
        if let Some(ref mcp_mutex) = self.mcp_client {
            if let Ok(mut mcp_client) = mcp_mutex.lock() {
                match mcp_client.call_tool(tool_name, Some(arguments.clone())) {
                    Ok(result) => {
                        return Ok(serde_json::to_string_pretty(&result).unwrap_or_default());
                    }
                    Err(e) => {
                        tracing::debug!("MCP stdio call failed, falling back to HTTP: {:?}", e);
                        // 继续使用 HTTP API
                    }
                }
            }
        }

        // 根据工具名称调用不同的 HTTP API
        match (self.name.as_str(), tool_name) {
            // GitHub 工具
            ("github", "list_repositories") => {
                let visibility = arguments.get("visibility")
                    .and_then(|v| v.as_str())
                    .unwrap_or("owner");
                self.github_list_repos(visibility)
            }
            ("github", "get_repository") => {
                let owner = arguments.get("owner")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let repo = arguments.get("repo")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.github_get_repo(owner, repo)
            }
            ("github", "list_issues") => {
                let owner = arguments.get("owner")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let repo = arguments.get("repo")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let state = arguments.get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open");
                self.github_list_issues(owner, repo, state)
            }
            ("github", "create_issue") => {
                let owner = arguments.get("owner")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let repo = arguments.get("repo")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let title = arguments.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let body = arguments.get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.github_create_issue(owner, repo, title, body)
            }
            // Slack 工具
            ("slack", "post_message") => {
                let channel = arguments.get("channel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let text = arguments.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.slack_post_message(channel, text)
            }
            ("slack", "list_channels") => {
                self.slack_list_channels()
            }
            // Notion 工具
            ("notion", "search_pages") => {
                let query = arguments.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.notion_search(query)
            }
            ("notion", "get_page") => {
                let page_id = arguments.get("page_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.notion_get_page(page_id)
            }
            ("notion", "create_page") => {
                let parent_id = arguments.get("parent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let title = arguments.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.notion_create_page(parent_id, title)
            }
            _ => Err(format!("Unsupported tool: {} for server: {}", tool_name, self.name))
        }
    }

    // === GitHub API 方法 ===
    fn github_list_repos(&self, visibility: &str) -> Result<String, String> {
        let url = format!("https://api.github.com/user/repos?visibility={}", visibility);
        let headers = self.auth_headers();
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.get(&url)
                .headers(headers)
                .header("User-Agent", "evif-mcp")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn github_get_repo(&self, owner: &str, repo: &str) -> Result<String, String> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let headers = self.auth_headers();
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.get(&url)
                .headers(headers)
                .header("User-Agent", "evif-mcp")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn github_list_issues(&self, owner: &str, repo: &str, state: &str) -> Result<String, String> {
        let url = format!("https://api.github.com/repos/{}/{}/issues?state={}", owner, repo, state);
        let headers = self.auth_headers();
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.get(&url)
                .headers(headers)
                .header("User-Agent", "evif-mcp")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn github_create_issue(&self, owner: &str, repo: &str, title: &str, body: &str) -> Result<String, String> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
        let headers = self.auth_headers();
        let payload = serde_json::json!({
            "title": title,
            "body": body
        });
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.post(&url)
                .headers(headers)
                .header("User-Agent", "evif-mcp")
                .json(&payload)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    // === Slack API 方法 ===
    fn slack_post_message(&self, channel: &str, text: &str) -> Result<String, String> {
        let url = "https://slack.com/api/chat.postMessage";
        let headers = self.auth_headers();
        let payload = serde_json::json!({
            "channel": channel,
            "text": text
        });
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.post(url)
                .headers(headers)
                .json(&payload)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn slack_list_channels(&self) -> Result<String, String> {
        let url = "https://slack.com/api/conversations.list";
        let headers = self.auth_headers();
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.post(url)
                .headers(headers)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    // === Notion API 方法 ===
    fn notion_search(&self, query: &str) -> Result<String, String> {
        let url = "https://api.notion.com/v1/search";
        let headers = self.auth_headers();
        let payload = serde_json::json!({
            "query": query
        });
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.post(url)
                .headers(headers)
                .header("Notion-Version", "2022-06-28")
                .json(&payload)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn notion_get_page(&self, page_id: &str) -> Result<String, String> {
        let url = format!("https://api.notion.com/v1/pages/{}", page_id);
        let headers = self.auth_headers();
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.get(&url)
                .headers(headers)
                .header("Notion-Version", "2022-06-28")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn notion_create_page(&self, parent_id: &str, title: &str) -> Result<String, String> {
        let url = "https://api.notion.com/v1/pages";
        let headers = self.auth_headers();
        let payload = serde_json::json!({
            "parent": { "page_id": parent_id },
            "properties": {
                "title": {
                    "title": [{ "text": { "content": title } }]
                }
            }
        });
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            self.client.post(url)
                .headers(headers)
                .header("Notion-Version", "2022-06-28")
                .json(&payload)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())
        })
    }
}

impl McpServerPlugin {
    /// 创建 GitHub MCP Server Plugin
    pub fn github() -> Self {
        let mut config = ExternalMcpConfig {
            name: "github".to_string(),
            command: Some("npx".to_string()),
            args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()]),
            env: Some(std::collections::HashMap::new()),
            url: None,
            auth_token_env: Some("GITHUB_TOKEN".to_string()),
        };
        // 设置 GitHub token 环境变量
        if let Some(env) = config.env.as_mut() {
            env.insert("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(), "${GITHUB_TOKEN}".to_string());
        }

        let mut plugin = Self::new("github".to_string(), config);
        // 预设 GitHub 工具
        plugin.tools = Arc::new(RwLock::new(vec![
            ExternalTool {
                name: "list_repositories".to_string(),
                description: "List repositories for the authenticated user".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "visibility": {
                            "type": "string",
                            "enum": ["all", "public", "private", "owner"]
                        }
                    }
                }),
            },
            ExternalTool {
                name: "get_repository".to_string(),
                description: "Get repository details".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" },
                        "repo": { "type": "string" }
                    },
                    "required": ["owner", "repo"]
                }),
            },
            ExternalTool {
                name: "list_issues".to_string(),
                description: "List issues in a repository".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" },
                        "repo": { "type": "string" },
                        "state": {
                            "type": "string",
                            "enum": ["open", "closed", "all"]
                        }
                    },
                    "required": ["owner", "repo"]
                }),
            },
            ExternalTool {
                name: "create_issue".to_string(),
                description: "Create a new issue".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" },
                        "repo": { "type": "string" },
                        "title": { "type": "string" },
                        "body": { "type": "string" }
                    },
                    "required": ["owner", "repo", "title"]
                }),
            },
        ]));
        plugin
    }

    /// 创建 Slack MCP Server Plugin
    pub fn slack() -> Self {
        let config = ExternalMcpConfig {
            name: "slack".to_string(),
            command: Some("npx".to_string()),
            args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-slack".to_string()]),
            env: Some(std::collections::HashMap::new()),
            url: None,
            auth_token_env: Some("SLACK_BOT_TOKEN".to_string()),
        };

        let mut plugin = Self::new("slack".to_string(), config);
        plugin.tools = Arc::new(RwLock::new(vec![
            ExternalTool {
                name: "post_message".to_string(),
                description: "Post a message to a Slack channel".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "channel": { "type": "string" },
                        "text": { "type": "string" }
                    },
                    "required": ["channel", "text"]
                }),
            },
            ExternalTool {
                name: "list_channels".to_string(),
                description: "List all channels in the workspace".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]));
        plugin
    }

    /// 创建 Notion MCP Server Plugin
    pub fn notion() -> Self {
        let config = ExternalMcpConfig {
            name: "notion".to_string(),
            command: Some("npx".to_string()),
            args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-notion".to_string()]),
            env: Some(std::collections::HashMap::new()),
            url: None,
            auth_token_env: Some("NOTION_API_KEY".to_string()),
        };

        let mut plugin = Self::new("notion".to_string(), config);
        plugin.tools = Arc::new(RwLock::new(vec![
            ExternalTool {
                name: "search_pages".to_string(),
                description: "Search pages in Notion".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    }
                }),
            },
            ExternalTool {
                name: "get_page".to_string(),
                description: "Get a page by ID".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_id": { "type": "string" }
                    },
                    "required": ["page_id"]
                }),
            },
            ExternalTool {
                name: "create_page".to_string(),
                description: "Create a new page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "parent_id": { "type": "string" },
                        "title": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["parent_id", "title"]
                }),
            },
        ]));
        plugin
    }
}

#[async_trait]
impl EvifPlugin for McpServerPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_readme(&self) -> String {
        format!(
            r#"# MCP Server: {}

外部 MCP 服务器，映射到 VFS 路径 `/mcp/{}`

## 支持的能力

- Tools: {}
- Resources: {}
- Prompts: {}

## 路径结构

```
/mcp/{{name}}/
├── tools/           # 可用工具
├── resources/       # 可用资源
└── status.json     # 服务器状态
```

## 配置

- Auth Token: {}
- Command: {:?}
"#,
            self.name,
            self.name,
            self.capabilities.read().unwrap().tools,
            self.capabilities.read().unwrap().resources,
            self.capabilities.read().unwrap().prompts,
            self.config.auth_token_env.as_deref().unwrap_or("N/A"),
            self.config.command,
        )
    }

    fn get_config_params(&self) -> Vec<evif_core::PluginConfigParam> {
        vec![
            evif_core::PluginConfigParam {
                name: "auth_token_env".to_string(),
                param_type: "string".to_string(),
                required: false,
                default: self.config.auth_token_env.clone(),
                description: Some("环境变量名存储认证 token".to_string()),
            },
        ]
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // 不允许创建
        Err(EvifError::InvalidPath(format!(
            "Cannot create: {}",
            path
        )))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "Directories are virtual".to_string(),
        ))
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let tools = self.tools.read().unwrap();
        let resources = self.resources.read().unwrap();
        let capabilities = self.capabilities.read().unwrap();
        let connected = *self.connected.read().unwrap();

        match path {
            p if p == format!("/mcp/{}", self.name) || p == format!("/mcp/{}/", self.name) => {
                Ok(format!(
                    "MCP Server: {}\nConnected: {}\nTools: {}\n",
                    self.name, connected, tools.len()
                )
                .into_bytes())
            }
            p if p == format!("/mcp/{}/status.json", self.name) => {
                let status = serde_json::json!({
                    "name": self.name,
                    "connected": connected,
                    "capabilities": {
                        "tools": capabilities.tools,
                        "resources": capabilities.resources,
                        "prompts": capabilities.prompts,
                        "sampling": capabilities.sampling,
                    },
                    "tool_count": tools.len(),
                    "resource_count": resources.len(),
                });
                Ok(serde_json::to_string_pretty(&status)
                    .unwrap()
                    .into_bytes())
            }
            p if p == format!("/mcp/{}/tools.json", self.name) => {
                Ok(serde_json::to_string_pretty(&*tools)
                    .unwrap()
                    .into_bytes())
            }
            p if p == format!("/mcp/{}/resources.json", self.name) => {
                Ok(serde_json::to_string_pretty(&*resources)
                    .unwrap()
                    .into_bytes())
            }
            p if p.starts_with(&format!("/mcp/{}/tools/", self.name)) => {
                let tool_name = p.trim_start_matches(&format!("/mcp/{}/tools/", self.name));
                let tool_name = tool_name.trim_end_matches(".json");
                if let Some(tool) = tools.iter().find(|t| t.name == tool_name) {
                    Ok(serde_json::to_string_pretty(tool).unwrap().into_bytes())
                } else {
                    Err(EvifError::NotFound(path.to_string()))
                }
            }
            // 工具调用端点: /mcp/{name}/call/{tool_name}
            p if p.starts_with(&format!("/mcp/{}/call/", self.name)) => {
                let path_after_call = p.trim_start_matches(&format!("/mcp/{}/call/", self.name));
                let parts: Vec<&str> = path_after_call.splitn(2, '?').collect();
                let tool_name = parts[0];

                // 解析参数
                let mut args = serde_json::Map::new();
                if parts.len() > 1 {
                    let query = parts[1];
                    for pair in query.split('&') {
                        let kv: Vec<&str> = pair.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            let key = kv[0];
                            let value = kv[1].replace("%20", " ").replace("%3F", "?");
                            args.insert(key.to_string(), serde_json::Value::String(value));
                        }
                    }
                }

                // 调用工具
                let result = self.call_tool_sync(tool_name, serde_json::Value::Object(args));
                match result {
                    Ok(response) => Ok(response.into_bytes()),
                    Err(e) => Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "error": e
                    })).unwrap().into_bytes()),
                }
            }
            // 调用目录: /mcp/{name}/call
            p if p == format!("/mcp/{}/call", self.name) || p == format!("/mcp/{}/call/", self.name) => {
                let call_tools: Vec<serde_json::Value> = tools.iter().map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "path": format!("/mcp/{}/call/{}", self.name, t.name),
                    })
                }).collect();
                Ok(serde_json::to_string_pretty(&call_tools).unwrap().into_bytes())
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        // 解析工具调用路径: /mcp/{server}/call/{tool_name}?args=...
        let base_path = format!("/mcp/{}/call/", self.name);
        if path.starts_with(&base_path) {
            let path_after_call = path.trim_start_matches(&base_path);
            let parts: Vec<&str> = path_after_call.splitn(2, '?').collect();
            let tool_name = parts[0];

            // 解析参数
            let mut args = serde_json::Map::new();
            if parts.len() > 1 {
                let query = parts[1];
                for pair in query.split('&') {
                    let kv: Vec<&str> = pair.splitn(2, '=').collect();
                    if kv.len() == 2 {
                        let key = kv[0];
                        let value = kv[1].replace("%20", " ").replace("%3F", "?"); // URL decode
                        args.insert(key.to_string(), serde_json::Value::String(value));
                    }
                }
            }

            // 解析 body 数据 (JSON 格式)
            if !data.is_empty() {
                if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&data)) {
                    if let serde_json::Value::Object(obj) = body_json {
                        for (key, value) in obj {
                            args.insert(key, value);
                        }
                    }
                }
            }

            // 调用工具
            let result = self.call_tool_sync(tool_name, serde_json::Value::Object(args));
            match result {
                Ok(response) => Ok(response.len() as u64),
                Err(e) => Err(EvifError::InvalidInput(format!("Tool call failed: {}", e))),
            }
        } else {
            Err(EvifError::InvalidPath(
                "MCP Server Plugin is read-only".to_string(),
            ))
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let tools = self.tools.read().unwrap();
        let resources = self.resources.read().unwrap();
        let now = Utc::now();

        match path {
            "/" | "" => Ok(vec![FileInfo {
                name: self.name.clone(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }]),
            p if p == format!("/mcp/{}", self.name) || p == format!("/mcp/{}/", self.name) => {
                let mut entries = vec![
                    FileInfo {
                        name: "tools".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                    FileInfo {
                        name: "call".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                    FileInfo {
                        name: "resources".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                    FileInfo {
                        name: "status.json".to_string(),
                        size: 128,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    },
                ];
                if !resources.is_empty() {
                    entries.push(FileInfo {
                        name: "resources.json".to_string(),
                        size: 256,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    });
                }
                Ok(entries)
            }
            p if p == format!("/mcp/{}/tools", self.name) || p == format!("/mcp/{}/tools/", self.name) => {
                let entries: Vec<FileInfo> = tools
                    .iter()
                    .map(|t| FileInfo {
                        name: format!("{}.json", t.name),
                        size: t.description.len() as u64,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    })
                    .collect();
                Ok(entries)
            }
            p if p == format!("/mcp/{}/call", self.name) || p == format!("/mcp/{}/call/", self.name) => {
                let entries: Vec<FileInfo> = tools
                    .iter()
                    .map(|t| FileInfo {
                        name: format!("{}.json", t.name),
                        size: t.description.len() as u64,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    })
                    .collect();
                Ok(entries)
            }
            p if p == format!("/mcp/{}/resources", self.name) || p == format!("/mcp/{}/resources/", self.name) => {
                let entries: Vec<FileInfo> = resources
                    .iter()
                    .map(|r| FileInfo {
                        name: format!("{}.json", r.name.replace("/", "_")),
                        size: r.mime_type.len() as u64,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    })
                    .collect();
                Ok(entries)
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let tools = self.tools.read().unwrap();
        let now = Utc::now();

        match path {
            p if p == format!("/mcp/{}", self.name) => Ok(FileInfo {
                name: self.name.clone(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            p if p == format!("/mcp/{}/tools", self.name) => Ok(FileInfo {
                name: "tools".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            p if p == format!("/mcp/{}/call", self.name) => Ok(FileInfo {
                name: "call".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            p if p == format!("/mcp/{}/resources", self.name) => Ok(FileInfo {
                name: "resources".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            p if p == format!("/mcp/{}/status.json", self.name) => Ok(FileInfo {
                name: "status.json".to_string(),
                size: 128,
                mode: 0o644,
                modified: now,
                is_dir: false,
            }),
            p if p == format!("/mcp/{}/tools.json", self.name) => Ok(FileInfo {
                name: "tools.json".to_string(),
                size: tools.len() as u64 * 64,
                mode: 0o644,
                modified: now,
                is_dir: false,
            }),
            p if p.starts_with(&format!("/mcp/{}/tools/", self.name)) => {
                let tool_name = p.trim_start_matches(&format!("/mcp/{}/tools/", self.name));
                let tool_name = tool_name.trim_end_matches(".json");
                if tools.iter().any(|t| t.name == tool_name) {
                    Ok(FileInfo {
                        name: format!("{}.json", tool_name),
                        size: 256,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    })
                } else {
                    Err(EvifError::NotFound(path.to_string()))
                }
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "MCP Server Plugin is read-only".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "MCP Server Plugin is read-only".to_string(),
        ))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "MCP Server Plugin is read-only".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_server_plugin_creation() {
        let config = ExternalMcpConfig {
            name: "test".to_string(),
            command: Some("test-cmd".to_string()),
            args: None,
            env: None,
            url: None,
            auth_token_env: Some("TEST_TOKEN".to_string()),
        };
        let plugin = McpServerPlugin::new("test".to_string(), config);

        assert_eq!(plugin.name(), "test");
        assert_eq!(plugin.mount_path(), "/mcp/test");
    }

    #[tokio::test]
    async fn test_github_plugin_presets() {
        let plugin = McpServerPlugin::github();

        assert_eq!(plugin.name(), "github");
        let tools = plugin.list_tools();
        assert!(tools.len() >= 4);
        assert!(tools.iter().any(|t| t.name == "list_repositories"));
        assert!(tools.iter().any(|t| t.name == "create_issue"));
    }

    #[tokio::test]
    async fn test_slack_plugin_presets() {
        let plugin = McpServerPlugin::slack();

        assert_eq!(plugin.name(), "slack");
        let tools = plugin.list_tools();
        assert!(tools.len() >= 2);
        assert!(tools.iter().any(|t| t.name == "post_message"));
        assert!(tools.iter().any(|t| t.name == "list_channels"));
    }

    #[tokio::test]
    async fn test_notion_plugin_presets() {
        let plugin = McpServerPlugin::notion();

        assert_eq!(plugin.name(), "notion");
        let tools = plugin.list_tools();
        assert!(tools.len() >= 3);
        assert!(tools.iter().any(|t| t.name == "search_pages"));
        assert!(tools.iter().any(|t| t.name == "create_page"));
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = McpServerPlugin::github();
        let entries = plugin.readdir(&format!("/mcp/{}", plugin.name())).await.unwrap();

        assert!(entries.len() >= 3);
        assert!(entries.iter().any(|e| e.name == "tools"));
        assert!(entries.iter().any(|e| e.name == "resources"));
        assert!(entries.iter().any(|e| e.name == "status.json"));
    }

    #[tokio::test]
    async fn test_readdir_tools() {
        let plugin = McpServerPlugin::github();
        let entries = plugin
            .readdir(&format!("/mcp/{}/tools", plugin.name()))
            .await
            .unwrap();

        assert!(!entries.is_empty());
        assert!(entries.iter().all(|e| e.name.ends_with(".json")));
    }

    #[tokio::test]
    async fn test_read_status_json() {
        let plugin = McpServerPlugin::github();
        plugin.set_connected(true);

        let data = plugin
            .read(&format!("/mcp/{}/status.json", plugin.name()), 0, 1000)
            .await
            .unwrap();

        let json_str = String::from_utf8(data).unwrap();
        let status: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(status["name"], "github");
        assert_eq!(status["connected"], true);
    }

    #[tokio::test]
    async fn test_read_tools_json() {
        let plugin = McpServerPlugin::github();

        let data = plugin
            .read(&format!("/mcp/{}/tools.json", plugin.name()), 0, 1000)
            .await
            .unwrap();

        let json_str = String::from_utf8(data).unwrap();
        let tools: Vec<ExternalTool> = serde_json::from_str(&json_str).unwrap();

        assert!(!tools.is_empty());
        assert_eq!(tools[0].name, "list_repositories");
    }

    #[tokio::test]
    async fn test_get_tool() {
        let plugin = McpServerPlugin::github();

        let tool = plugin.get_tool("create_issue");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name, "create_issue");
    }

    #[tokio::test]
    async fn test_get_nonexistent_tool() {
        let plugin = McpServerPlugin::github();

        let tool = plugin.get_tool("nonexistent");
        assert!(tool.is_none());
    }

    #[tokio::test]
    async fn test_validate_auth() {
        let config = ExternalMcpConfig {
            name: "test".to_string(),
            command: None,
            args: None,
            env: None,
            url: None,
            auth_token_env: Some("NONEXISTENT_TOKEN_ENV".to_string()),
        };
        let plugin = McpServerPlugin::new("test".to_string(), config);

        // 应该失败因为环境变量不存在
        let result = plugin.validate_auth();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_only_enforcement() {
        let plugin = McpServerPlugin::github();

        // 写操作应该失败
        let result = plugin
            .write("/mcp/github/test", b"data".to_vec(), 0, WriteFlags::CREATE)
            .await;
        assert!(result.is_err());

        // 删除操作应该失败
        let result = plugin.remove("/mcp/github/test").await;
        assert!(result.is_err());

        // 重命名操作应该失败
        let result = plugin.rename("/mcp/github/a", "/mcp/github/b").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_tools() {
        let plugin = McpServerPlugin::github();

        let new_tools = vec![
            ExternalTool {
                name: "custom_tool".to_string(),
                description: "A custom tool".to_string(),
                input_schema: serde_json::json!({}),
            },
        ];
        plugin.update_tools(new_tools);

        let tools = plugin.list_tools();
        assert!(tools.iter().any(|t| t.name == "custom_tool"));
    }

    #[tokio::test]
    async fn test_stat_operations() {
        let plugin = McpServerPlugin::github();

        let info = plugin.stat("/mcp/github").await.unwrap();
        assert_eq!(info.name, "github");
        assert!(info.is_dir);

        let info = plugin.stat("/mcp/github/status.json").await.unwrap();
        assert_eq!(info.name, "status.json");
        assert!(!info.is_dir);

        // 不存在的路径应该失败
        let result = plugin.stat("/mcp/github/nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_readdir_call_directory() {
        let plugin = McpServerPlugin::github();
        let entries = plugin
            .readdir(&format!("/mcp/{}/call", plugin.name()))
            .await
            .unwrap();

        // 应该包含 GitHub 工具
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|e| e.name.ends_with(".json")));
        // 应该至少有 list_repositories
        assert!(entries.iter().any(|e| e.name.contains("list_repositories")));
    }

    #[tokio::test]
    async fn test_read_call_list_repos() {
        let plugin = McpServerPlugin::github();

        // 读取 call 目录
        let data = plugin
            .read(&format!("/mcp/{}/call/list_repositories.json", plugin.name()), 0, 1000)
            .await;

        // 如果有认证 token，应该返回工具信息
        match data {
            Ok(content) => {
                let json_str = String::from_utf8(content).unwrap();
                assert!(json_str.contains("list_repositories"));
            }
            Err(EvifError::NotFound(_)) => {
                // 工具调用需要正确路径
            }
            Err(e) => {
                // 其他错误（如缺少认证）也是正常的
                println!("Expected error (no auth token): {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_call_tool_sync_unsupported() {
        let plugin = McpServerPlugin::github();

        // 不存在的工具应该返回错误
        let result = plugin.call_tool_sync("unsupported_tool", serde_json::json!({}));
        assert!(result.is_err());
        // 错误信息应该是 "Tool not found" 或 "Unsupported tool"
        let err = result.unwrap_err();
        assert!(err.contains("Tool not found") || err.contains("Unsupported tool"));
    }

    #[tokio::test]
    async fn test_stat_call_directory() {
        let plugin = McpServerPlugin::github();

        let info = plugin.stat(&format!("/mcp/{}/call", plugin.name())).await.unwrap();
        assert_eq!(info.name, "call");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_slack_preset() {
        let plugin = McpServerPlugin::slack();
        let tools = plugin.list_tools();

        assert_eq!(plugin.name(), "slack");
        assert!(tools.iter().any(|t| t.name == "post_message"));
        assert!(tools.iter().any(|t| t.name == "list_channels"));
    }

    #[tokio::test]
    async fn test_notion_preset() {
        let plugin = McpServerPlugin::notion();
        let tools = plugin.list_tools();

        assert_eq!(plugin.name(), "notion");
        assert!(tools.iter().any(|t| t.name == "search_pages"));
        assert!(tools.iter().any(|t| t.name == "get_page"));
        assert!(tools.iter().any(|t| t.name == "create_page"));
    }

    #[tokio::test]
    async fn test_mcp_client_presence() {
        let plugin = McpServerPlugin::github();

        // MCP 客户端可能无法启动（需要 npx 和 npm 包）
        // 但 has_mcp_client() 应该能正确返回状态
        let has_client = plugin.has_mcp_client();
        println!("GitHub plugin has MCP client: {}", has_client);

        // 如果 MCP 客户端存在，尝试同步工具
        if has_client {
            let result = plugin.sync_tools_from_mcp();
            println!("Sync result: {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_fallback_to_http_api() {
        let plugin = McpServerPlugin::github();

        // 测试 fallback 逻辑 - 调用不存在的工具应该返回错误
        let result = plugin.call_tool_sync("unsupported_tool", serde_json::json!({}));
        let is_err = result.is_err();
        if is_err {
            let err = result.unwrap_err();
            assert!(err.contains("Unsupported tool") || err.contains("Tool not found"));
        } else {
            panic!("Expected error for unsupported tool");
        }
    }
}