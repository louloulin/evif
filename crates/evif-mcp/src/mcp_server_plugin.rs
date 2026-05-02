// MCP Server Plugin - 外部 MCP Server 接入
//
// 将外部 MCP Server 作为 EVIF Plugin 暴露
// 支持 GitHub, Slack, Notion 等外部 MCP 服务器

use async_trait::async_trait;
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock;
use chrono::Utc;

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
        Self {
            name,
            config,
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
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn write(&self, _path: &str, _data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        Err(EvifError::InvalidPath(
            "MCP Server Plugin is read-only".to_string(),
        ))
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
}