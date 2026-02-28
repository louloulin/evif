// EVIF MCP Server - Model Context Protocol 服务器实现
//
// 提供17个工具对等AGFS,支持Claude Desktop和其他MCP客户端

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use reqwest::Client;

/// MCP 服务器配置
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub evif_url: String,
    pub server_name: String,
    pub version: String,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            evif_url: "http://localhost:8081".to_string(),
            server_name: "evif-mcp".to_string(),
            version: "1.8.0".to_string(),
        }
    }
}

/// MCP 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP 资源定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// MCP Prompt 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub argument_type: String,
}

/// EVIF MCP 服务器
pub struct EvifMcpServer {
    config: McpServerConfig,
    client: Client,
    tools: Arc<RwLock<Vec<Tool>>>,
    prompts: Arc<RwLock<Vec<Prompt>>>,
    resources: Arc<RwLock<Vec<Resource>>>,
}

impl EvifMcpServer {
    pub fn new(config: McpServerConfig) -> Arc<Self> {
        let client = Client::new();
        let server = Arc::new(Self {
            config,
            client,
            tools: Arc::new(RwLock::new(Vec::new())),
            prompts: Arc::new(RwLock::new(Vec::new())),
            resources: Arc::new(RwLock::new(Vec::new())),
        });

        // 初始化工具和提示
        let s = server.clone();
        tokio::spawn(async move {
            s.initialize_tools().await;
        });

        let s = server.clone();
        tokio::spawn(async move {
            s.initialize_prompts().await;
        });

        let s = server.clone();
        tokio::spawn(async move {
            s.initialize_resources().await;
        });

        server
    }

    /// 初始化所有工具
    async fn initialize_tools(self: Arc<Self>) {
        let tools = vec![
            // 文件操作工具
            Tool {
                name: "evif_ls".to_string(),
                description: "List files in a directory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to list"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of files to return"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_cat".to_string(),
                description: "Read file contents".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        },
                        "offset": {
                            "type": "number",
                            "description": "Read offset in bytes"
                        },
                        "size": {
                            "type": "number",
                            "description": "Number of bytes to read"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_write".to_string(),
                description: "Write content to a file".to_string(),
                input_schema: json!({
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
                        },
                        "flags": {
                            "type": "number",
                            "description": "Write flags"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
            Tool {
                name: "evif_mkdir".to_string(),
                description: "Create a directory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to create"
                        },
                        "mode": {
                            "type": "number",
                            "description": "Directory permissions (default: 0o755)"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_rm".to_string(),
                description: "Remove a file or directory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to remove"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Recursively remove directories"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_stat".to_string(),
                description: "Get file information".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to stat"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_mv".to_string(),
                description: "Move or rename a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "old_path": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "new_path": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["old_path", "new_path"]
                }),
            },
            Tool {
                name: "evif_cp".to_string(),
                description: "Copy a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "src": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "dst": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["src", "dst"]
                }),
            },
            // 插件管理工具
            Tool {
                name: "evif_mount".to_string(),
                description: "Mount a plugin".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "plugin": {
                            "type": "string",
                            "description": "Plugin name"
                        },
                        "path": {
                            "type": "string",
                            "description": "Mount path"
                        },
                        "config": {
                            "type": "object",
                            "description": "Plugin configuration"
                        }
                    },
                    "required": ["plugin", "path"]
                }),
            },
            Tool {
                name: "evif_unmount".to_string(),
                description: "Unmount a plugin".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Mount path to unmount"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_mounts".to_string(),
                description: "List all mount points".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            // 高级工具
            Tool {
                name: "evif_grep".to_string(),
                description: "Search for text in files".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to search in"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum results"
                        }
                    },
                    "required": ["path", "pattern"]
                }),
            },
            Tool {
                name: "evif_health".to_string(),
                description: "Check server health".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            // HandleFS 工具
            Tool {
                name: "evif_open_handle".to_string(),
                description: "Open a file handle".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to open"
                        },
                        "flags": {
                            "type": "number",
                            "description": "Open flags (1=readonly, 2=writeonly, 3=readwrite)"
                        },
                        "mode": {
                            "type": "number",
                            "description": "File permissions"
                        },
                        "lease": {
                            "type": "number",
                            "description": "Lease duration in seconds"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_close_handle".to_string(),
                description: "Close a file handle".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "handle_id": {
                            "type": "number",
                            "description": "Handle ID to close"
                        }
                    },
                    "required": ["handle_id"]
                }),
            },
        ];

        *self.tools.write().await = tools;
    }

    /// 初始化所有提示
    async fn initialize_prompts(self: Arc<Self>) {
        let prompts = vec![
            Prompt {
                name: "file_explorer".to_string(),
                description: "Explore and interact with the EVIF file system".to_string(),
                arguments: vec![],
            },
            Prompt {
                name: "batch_operations".to_string(),
                description: "Perform batch operations on files".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "operation".to_string(),
                        description: "Operation to perform (copy, move, delete)".to_string(),
                        required: true,
                        argument_type: "string".to_string(),
                    },
                    PromptArgument {
                        name: "files".to_string(),
                        description: "List of files to operate on".to_string(),
                        required: true,
                        argument_type: "array".to_string(),
                    },
                ],
            },
            Prompt {
                name: "data_analysis".to_string(),
                description: "Analyze data in files and generate insights".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "data_path".to_string(),
                        description: "Path to data files".to_string(),
                        required: true,
                        argument_type: "string".to_string(),
                    },
                    PromptArgument {
                        name: "analysis_type".to_string(),
                        description: "Type of analysis to perform".to_string(),
                        required: false,
                        argument_type: "string".to_string(),
                    },
                ],
            },
        ];

        *self.prompts.write().await = prompts;
    }

    /// 初始化所有资源
    async fn initialize_resources(self: Arc<Self>) {
        let resources = vec![
            Resource {
                uri: "file:///".to_string(),
                name: "Root Filesystem".to_string(),
                description: "Access to the entire EVIF filesystem".to_string(),
                mime_type: "inode/directory".to_string(),
            },
        ];

        *self.resources.write().await = resources;
    }

    /// 处理工具调用
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value, String> {
        match tool_name {
            "evif_ls" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;

                let url = format!("{}/api/v1/fs/list?path={}", self.config.evif_url, urlencoding::encode(path));
                let response = self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to list directory: {}", e))?;

                let body: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;
                Ok(body)
            }

            "evif_cat" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;

                let url = format!("{}/api/v1/fs/read?path={}", self.config.evif_url, urlencoding::encode(path));
                let response = self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to read file: {}", e))?;

                let data: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(data)
            }

            "evif_write" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;
                let content = arguments["content"]
                    .as_str()
                    .ok_or("Missing 'content' argument")?;

                let url = format!("{}/api/v1/fs/write?path={}", self.config.evif_url, urlencoding::encode(path));
                let response = self.client
                    .post(&url)
                    .json(&json!({ "content": content }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to write file: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_mkdir" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;

                let url = format!("{}/api/v1/directories", self.config.evif_url);
                let response = self.client
                    .post(&url)
                    .json(&json!({ "path": path }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to create directory: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_rm" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;
                let recursive = arguments["recursive"].as_bool().unwrap_or(false);

                let url = if recursive {
                    format!("{}/api/v1/directories?path={}", self.config.evif_url, urlencoding::encode(path))
                } else {
                    format!("{}/api/v1/files?path={}", self.config.evif_url, urlencoding::encode(path))
                };
                let response = self.client
                    .delete(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to remove: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_stat" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;

                let url = format!("{}/api/v1/stat?path={}", self.config.evif_url, urlencoding::encode(path));
                let response = self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to stat file: {}", e))?;

                let info: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(info)
            }

            "evif_mv" => {
                let old_path = arguments["old_path"]
                    .as_str()
                    .ok_or("Missing 'old_path' argument")?;
                let new_path = arguments["new_path"]
                    .as_str()
                    .ok_or("Missing 'new_path' argument")?;

                let url = format!("{}/api/v1/rename", self.config.evif_url);
                let response = self.client
                    .post(&url)
                    .json(&json!({ "from": old_path, "to": new_path }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to move: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_cp" => {
                let src = arguments["src"]
                    .as_str()
                    .ok_or("Missing 'src' argument")?;
                let dst = arguments["dst"]
                    .as_str()
                    .ok_or("Missing 'dst' argument")?;

                let read_url = format!("{}/api/v1/fs/read?path={}", self.config.evif_url, urlencoding::encode(src));
                let read_response = self.client
                    .get(&read_url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to read source: {}", e))?;

                let read_data: Value = read_response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse read response: {}", e))?;
                let content = read_data["content"].as_str().unwrap_or("");

                let write_url = format!("{}/api/v1/fs/write?path={}", self.config.evif_url, urlencoding::encode(dst));
                let write_response = self.client
                    .post(&write_url)
                    .json(&json!({ "content": content }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to write destination: {}", e))?;

                let result: Value = write_response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse write response: {}", e))?;

                Ok(result)
            }

            "evif_mounts" => {
                let url = format!("{}/api/v1/mounts", self.config.evif_url);
                let response = self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to list mounts: {}", e))?;

                let mounts: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(mounts)
            }

            "evif_health" => {
                let url = format!("{}/health", self.config.evif_url);
                let response = self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to check health: {}", e))?;

                let health: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(health)
            }

            "evif_mount" => {
                let plugin = arguments["plugin"]
                    .as_str()
                    .ok_or("Missing 'plugin' argument")?;
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;
                let config = arguments.get("config").cloned().unwrap_or(json!({}));

                let url = format!("{}/api/v1/mount", self.config.evif_url);
                let response = self.client
                    .post(&url)
                    .json(&json!({
                        "plugin": plugin,
                        "path": path,
                        "config": config
                    }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to mount plugin: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_unmount" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;

                let url = format!("{}/api/v1/unmount", self.config.evif_url);
                let response = self.client
                    .post(&url)
                    .json(&json!({ "path": path }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to unmount: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_grep" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;
                let pattern = arguments["pattern"]
                    .as_str()
                    .ok_or("Missing 'pattern' argument")?;

                let url = format!("{}/api/v1/grep", self.config.evif_url);
                let response = self.client
                    .post(&url)
                    .json(&json!({
                        "path": path,
                        "pattern": pattern,
                        "recursive": true
                    }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to grep: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_open_handle" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;
                let flags = arguments["flags"].as_u64().unwrap_or(1) as i32;
                let mode = arguments["mode"].as_u64().unwrap_or(0o644) as u32;
                let lease = arguments["lease"].as_u64().unwrap_or(300) as u64;

                let url = format!("{}/api/v1/handles/open", self.config.evif_url);
                let response = self.client
                    .post(&url)
                    .json(&json!({
                        "path": path,
                        "flags": flags,
                        "mode": mode,
                        "lease": lease
                    }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to open handle: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_close_handle" => {
                let handle_id = arguments["handle_id"]
                    .as_i64()
                    .ok_or("Missing or invalid 'handle_id' argument")?;

                let url = format!("{}/api/v1/handles/{}/close", self.config.evif_url, handle_id);
                let response = self.client
                    .post(&url)
                    .json(&json!({}))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to close handle: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }

    /// 启动 MCP 服务器 (stdio)
    pub async fn run_stdio(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::{self, BufRead, Write};

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        let mut line = String::new();

        loop {
            line.clear();

            // 读取请求
            let bytes_read = stdin.lock().read_line(&mut line)?;
            if bytes_read == 0 {
                break; // EOF
            }

            // 解析 JSON-RPC 请求
            if let Ok(request) = serde_json::from_str::<Value>(&line) {
                // 处理请求
                let response = self.handle_request(request).await;

                // 返回响应
                let response_json = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    /// 处理 JSON-RPC 请求
    async fn handle_request(&self, request: Value) -> Value {
        // 简化的 JSON-RPC 处理
        if let Some(method) = request.get("method").and_then(|m| m.as_str()) {
            if let Some(params) = request.get("params") {
                match self.call_tool(method, params.clone()).await {
                    Ok(result) => json!({
                        "jsonrpc": "2.0",
                        "result": result,
                        "id": request.get("id")
                    }),
                    Err(error) => json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32000,
                            "message": error
                        },
                        "id": request.get("id")
                    }),
                }
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32602,
                        "message": "Invalid params"
                    },
                    "id": request.get("id")
                })
            }
        } else {
            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Invalid Request"
                },
                "id": null
            })
        }
    }

    /// 获取工具列表
    pub async fn list_tools(&self) -> Vec<Tool> {
        self.tools.read().await.clone()
    }

    /// 获取提示列表
    pub async fn list_prompts(&self) -> Vec<Prompt> {
        self.prompts.read().await.clone()
    }

    /// 获取资源列表
    pub async fn list_resources(&self) -> Vec<Resource> {
        self.resources.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let config = McpServerConfig::default();
        let server = EvifMcpServer::new(config);

        // 等待异步初始化完成
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut tools = server.list_tools().await;
        for _ in 0..20 {
            if tools.len() >= 15 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            tools = server.list_tools().await;
        }
        assert!(!tools.is_empty());
        assert!(tools.len() >= 15, "expected at least 15 tools, got {}", tools.len());

        let prompts = server.list_prompts().await;
        assert_eq!(prompts.len(), 3);
    }
}
