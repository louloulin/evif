// MCP Gateway Plugin - 统一 MCP 入口
//
// 提供 /mcp 虚拟文件系统，用于访问 MCP 服务器、会话和工具
// 实现 EvifPlugin trait，可以挂载到 EVIF 挂载表

use async_trait::async_trait;
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

/// MCP 网关插件
///
/// 挂载点: /mcp
///
/// 目录结构:
/// /mcp/
/// ├── servers/          # 已注册的 MCP 服务器
/// │   ├── evif/        # 内置 EVIF MCP 服务器
/// │   └── {name}/      # 外部 MCP 服务器
/// ├── sessions/        # 活跃会话
/// │   └── {id}/
/// │       ├── status.json
/// │       └── tools.json
/// ├── tools/           # 可用工具列表
/// │   └── {name}/
/// │       └── schema.json
/// └── config.toml      # MCP 全局配置

pub struct McpGatewayPlugin {
    /// 服务器注册表
    servers: Arc<RwLock<Vec<McpServerEntry>>>,
    /// 活跃会话
    sessions: Arc<RwLock<Vec<SessionEntry>>>,
    /// 可用工具
    tools: Arc<RwLock<Vec<ToolEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    pub server_type: String,  // "builtin" | "external"
    pub url: Option<String>,
    pub enabled: bool,
    pub tool_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: String,
    pub server: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    pub name: String,
    pub server: String,
    pub description: String,
}

impl McpGatewayPlugin {
    /// 创建新的 MCP 网关插件
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(Vec::new())),
            sessions: Arc::new(RwLock::new(Vec::new())),
            tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册 MCP 服务器
    pub async fn register_server(&self, entry: McpServerEntry) {
        let mut servers = self.servers.write().await;
        // 避免重复
        servers.retain(|s| s.name != entry.name);
        servers.push(entry);
    }

    /// 注销 MCP 服务器
    pub async fn unregister_server(&self, name: &str) {
        let mut servers = self.servers.write().await;
        servers.retain(|s| s.name != name);
    }

    /// 获取所有注册的服务器
    pub async fn list_servers(&self) -> Vec<McpServerEntry> {
        self.servers.read().await.clone()
    }

    /// 添加会话
    pub async fn add_session(&self, session: SessionEntry) {
        let mut sessions = self.sessions.write().await;
        sessions.push(session);
    }

    /// 获取所有活跃会话
    pub async fn list_sessions(&self) -> Vec<SessionEntry> {
        self.sessions.read().await.clone()
    }

    /// 注册工具
    pub async fn register_tool(&self, entry: ToolEntry) {
        let mut tools = self.tools.write().await;
        tools.retain(|t| t.name != entry.name);
        tools.push(entry);
    }

    /// 获取所有工具
    pub async fn list_tools(&self) -> Vec<ToolEntry> {
        self.tools.read().await.clone()
    }

    /// 注册内置 EVIF MCP 服务器
    pub async fn register_builtin_server(&self, tool_count: usize) {
        let entry = McpServerEntry {
            name: "evif".to_string(),
            server_type: "builtin".to_string(),
            url: None,
            enabled: true,
            tool_count,
        };
        self.register_server(entry).await;
    }
}

impl Default for McpGatewayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for McpGatewayPlugin {
    fn name(&self) -> &str {
        "mcp-gateway"
    }

    fn get_readme(&self) -> String {
        r#"# MCP Gateway Plugin

统一 MCP 入口，提供虚拟文件系统访问 MCP 资源。

## 挂载点

`/mcp`

## 目录结构

```
/mcp/
├── servers/          # MCP 服务器注册表
├── sessions/         # 活跃 MCP 会话
├── tools/            # 可用 MCP 工具
└── config.toml       # MCP 全局配置
```

## 配置

无需要配置。使用内置 EVIF MCP 服务器。

## 示例

- `ls /mcp/servers/` - 列出已注册的 MCP 服务器
- `cat /mcp/servers/evif/status.json` - 查看 EVIF MCP 服务器状态
- `ls /mcp/tools/` - 列出所有可用工具
"#.to_string()
    }

    fn get_config_params(&self) -> Vec<evif_core::PluginConfigParam> {
        vec![
            evif_core::PluginConfigParam {
                name: "auto_register".to_string(),
                param_type: "bool".to_string(),
                required: false,
                default: Some("true".to_string()),
                description: Some("自动注册内置 EVIF MCP 服务器".to_string()),
            },
        ]
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // 只允许创建特定路径
        match path {
            p if p.starts_with("/mcp/sessions/") => Ok(()),
            _ => Err(EvifError::InvalidPath(format!("Cannot create: {}", path))),
        }
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::InvalidPath("Directories are virtual, cannot create".to_string()))
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let servers = self.servers.read().await;
        let sessions = self.sessions.read().await;
        let tools = self.tools.read().await;

        match path {
            "/mcp" | "/mcp/" => {
                // 返回根目录内容
                Ok(b"mcp-gateway root\n".to_vec())
            }
            "/mcp/servers" | "/mcp/servers/" => {
                // 返回服务器列表 JSON
                let json = serde_json::to_string_pretty(&*servers)
                    .map_err(|e| EvifError::InvalidPath(format!("JSON error: {}", e)))?;
                Ok(json.into_bytes())
            }
            p if p.starts_with("/mcp/servers/") && !p.contains('.') => {
                // 返回特定服务器目录列表
                let server_name = p.trim_start_matches("/mcp/servers/");
                if server_name.is_empty() {
                    // 服务器目录根 - 返回所有服务器名
                    let names: Vec<String> = servers.iter().map(|s| s.name.clone()).collect();
                    return Ok(names.join("\n").into_bytes());
                }
                Err(EvifError::NotFound(path.to_string()))
            }
            "/mcp/sessions" | "/mcp/sessions/" => {
                // 返回会话列表 JSON
                let json = serde_json::to_string_pretty(&*sessions)
                    .map_err(|e| EvifError::InvalidPath(format!("JSON error: {}", e)))?;
                Ok(json.into_bytes())
            }
            "/mcp/tools" | "/mcp/tools/" => {
                // 返回工具列表 JSON
                let json = serde_json::to_string_pretty(&*tools)
                    .map_err(|e| EvifError::InvalidPath(format!("JSON error: {}", e)))?;
                Ok(json.into_bytes())
            }
            "/mcp/config.toml" => {
                // 返回默认配置
                let config = r#"# MCP Gateway Configuration
[mcp]
protocol_version = "2024-11-05"
server_name = "evif-mcp-gateway"

[evif]
url = "http://localhost:8081"

[builtin]
auto_register = true
"#;
                Ok(config.as_bytes().to_vec())
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn write(&self, _path: &str, _data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        Err(EvifError::InvalidPath("MCP Gateway is read-only".to_string()))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let servers = self.servers.read().await;
        let sessions = self.sessions.read().await;
        let tools = self.tools.read().await;
        let now = Utc::now();

        match path {
            "/" | "" | "/mcp" => {
                // /mcp 根目录
                Ok(vec![
                    FileInfo {
                        name: "servers".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                    FileInfo {
                        name: "sessions".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                    FileInfo {
                        name: "tools".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                    FileInfo {
                        name: "config.toml".to_string(),
                        size: 256,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    },
                ])
            }
            "/mcp/servers" | "/mcp/servers/" => {
                // 服务器列表
                let mut entries = vec![
                    FileInfo {
                        name: "evif".to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    },
                ];
                // 添加外部服务器
                for server in servers.iter() {
                    if server.server_type == "external" {
                        entries.push(FileInfo {
                            name: server.name.clone(),
                            size: 0,
                            mode: 0o755,
                            modified: now,
                            is_dir: true,
                        });
                    }
                }
                Ok(entries)
            }
            "/mcp/servers/evif" => {
                // EVIF 内置服务器详细信息
                Ok(vec![
                    FileInfo {
                        name: "status.json".to_string(),
                        size: 128,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    },
                    FileInfo {
                        name: "tools.json".to_string(),
                        size: 512,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    },
                ])
            }
            "/mcp/sessions" | "/mcp/sessions/" => {
                // 会话列表
                let mut entries = Vec::new();
                for session in sessions.iter() {
                    entries.push(FileInfo {
                        name: session.id.clone(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    });
                }
                Ok(entries)
            }
            "/mcp/tools" | "/mcp/tools/" => {
                // 工具列表
                let mut entries = Vec::new();
                for tool in tools.iter() {
                    entries.push(FileInfo {
                        name: format!("{}.json", tool.name),
                        size: 256,
                        mode: 0o644,
                        modified: now,
                        is_dir: false,
                    });
                }
                Ok(entries)
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let servers = self.servers.read().await;
        let sessions = self.sessions.read().await;
        let tools = self.tools.read().await;
        let now = Utc::now();

        match path {
            "/mcp" | "/mcp/" => Ok(FileInfo {
                name: "mcp".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            "/mcp/servers" => Ok(FileInfo {
                name: "servers".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            "/mcp/servers/evif" => Ok(FileInfo {
                name: "evif".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            "/mcp/servers/evif/status.json" => Ok(FileInfo {
                name: "status.json".to_string(),
                size: 128,
                mode: 0o644,
                modified: now,
                is_dir: false,
            }),
            "/mcp/servers/evif/tools.json" => Ok(FileInfo {
                name: "tools.json".to_string(),
                size: servers.len() as u64 * 64,
                mode: 0o644,
                modified: now,
                is_dir: false,
            }),
            "/mcp/sessions" => Ok(FileInfo {
                name: "sessions".to_string(),
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            "/mcp/tools" => Ok(FileInfo {
                name: "tools".to_string(),
                size: tools.len() as u64 * 64,
                mode: 0o755,
                modified: now,
                is_dir: true,
            }),
            "/mcp/config.toml" => Ok(FileInfo {
                name: "config.toml".to_string(),
                size: 256,
                mode: 0o644,
                modified: now,
                is_dir: false,
            }),
            p if p.starts_with("/mcp/sessions/") => {
                // 检查是否是已知会话
                let session_id = p.trim_start_matches("/mcp/sessions/");
                if sessions.iter().any(|s| s.id == session_id) {
                    Ok(FileInfo {
                        name: session_id.to_string(),
                        size: 0,
                        mode: 0o755,
                        modified: now,
                        is_dir: true,
                    })
                } else {
                    Err(EvifError::NotFound(path.to_string()))
                }
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("MCP Gateway is read-only".to_string()))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("MCP Gateway is read-only".to_string()))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("MCP Gateway is read-only".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_gateway_root() {
        let plugin = McpGatewayPlugin::new();

        // 注册内置服务器
        plugin.register_builtin_server(26).await;

        // 读取根目录
        let entries = plugin.readdir("/mcp").await.unwrap();
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().any(|e| e.name == "servers"));
        assert!(entries.iter().any(|e| e.name == "sessions"));
        assert!(entries.iter().any(|e| e.name == "tools"));
    }

    #[tokio::test]
    async fn test_mcp_gateway_servers() {
        let plugin = McpGatewayPlugin::new();
        plugin.register_builtin_server(26).await;

        // 列出服务器
        let servers = plugin.list_servers().await;
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "evif");
        assert_eq!(servers[0].server_type, "builtin");
    }

    #[tokio::test]
    async fn test_mcp_gateway_register_external_server() {
        let plugin = McpGatewayPlugin::new();
        plugin.register_builtin_server(26).await;

        // 注册外部服务器
        plugin.register_server(McpServerEntry {
            name: "github".to_string(),
            server_type: "external".to_string(),
            url: Some("https://api.github.com".to_string()),
            enabled: true,
            tool_count: 10,
        }).await;

        let servers = plugin.list_servers().await;
        assert_eq!(servers.len(), 2);
    }

    #[tokio::test]
    async fn test_mcp_gateway_tools() {
        let plugin = McpGatewayPlugin::new();

        // 注册工具
        plugin.register_tool(ToolEntry {
            name: "evif_ls".to_string(),
            server: "evif".to_string(),
            description: "List directory contents".to_string(),
        }).await;

        let tools = plugin.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "evif_ls");
    }

    #[tokio::test]
    async fn test_mcp_gateway_read_servers_json() {
        let plugin = McpGatewayPlugin::new();
        plugin.register_builtin_server(26).await;

        let data = plugin.read("/mcp/servers", 0, 1000).await.unwrap();
        let json_str = String::from_utf8(data).unwrap();

        // 应该能解析为 JSON
        let servers: Vec<McpServerEntry> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(servers.len(), 1);
    }

    #[tokio::test]
    async fn test_mcp_gateway_read_config() {
        let plugin = McpGatewayPlugin::new();

        let data = plugin.read("/mcp/config.toml", 0, 1000).await.unwrap();
        let content = String::from_utf8(data).unwrap();

        assert!(content.contains("[mcp]"));
        assert!(content.contains("protocol_version"));
    }

    #[tokio::test]
    async fn test_mcp_gateway_stat() {
        let plugin = McpGatewayPlugin::new();

        let info = plugin.stat("/mcp").await.unwrap();
        assert_eq!(info.name, "mcp");
        assert!(info.is_dir);

        let info = plugin.stat("/mcp/config.toml").await.unwrap();
        assert_eq!(info.name, "config.toml");
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_mcp_gateway_read_only() {
        let plugin = McpGatewayPlugin::new();

        // 写操作应该失败
        let result = plugin.write("/mcp/test", b"data".to_vec(), 0, WriteFlags::CREATE).await;
        assert!(result.is_err());

        // 删除操作应该失败
        let result = plugin.remove("/mcp/test").await;
        assert!(result.is_err());

        // 重命名操作应该失败
        let result = plugin.rename("/mcp/a", "/mcp/b").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mcp_gateway_session() {
        let plugin = McpGatewayPlugin::new();

        // 添加会话
        plugin.add_session(SessionEntry {
            id: "session-123".to_string(),
            server: "evif".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            active: true,
        }).await;

        // 检查会话存在
        let sessions = plugin.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "session-123");
    }
}