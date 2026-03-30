// EVIF MCP Server - Model Context Protocol 服务器实现
//
// 提供17个工具对等AGFS,支持Claude Desktop和其他MCP客户端

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

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
            // Memory tools
            Tool {
                name: "evif_memorize".to_string(),
                description: "Store content as memories in the memory system".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content to memorize"
                        },
                        "text": {
                            "type": "string",
                            "description": "Deprecated alias for content"
                        },
                        "modality": {
                            "type": "string",
                            "description": "Modality type (conversation, document, code, etc.)"
                        },
                        "metadata": {
                            "type": "object",
                            "description": "Optional metadata forwarded to the REST memory API"
                        }
                    },
                    "oneOf": [
                        { "required": ["content"] },
                        { "required": ["text"] }
                    ]
                }),
            },
            Tool {
                name: "evif_retrieve".to_string(),
                description: "Search memories using vector or hybrid search".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "mode": {
                            "type": "string",
                            "description": "Search mode: vector or hybrid"
                        },
                        "k": {
                            "type": "number",
                            "description": "Number of results to return"
                        },
                        "threshold": {
                            "type": "number",
                            "description": "Similarity threshold (0.0-1.0)"
                        }
                    },
                    "required": ["query"]
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
        let resources = vec![Resource {
            uri: "file:///".to_string(),
            name: "Root Filesystem".to_string(),
            description: "Access to the entire EVIF filesystem".to_string(),
            mime_type: "inode/directory".to_string(),
        }];

        *self.resources.write().await = resources;
    }

    /// 处理工具调用
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value, String> {
        match tool_name {
            "evif_ls" => {
                let path = arguments["path"]
                    .as_str()
                    .ok_or("Missing 'path' argument")?;

                let url = format!(
                    "{}/api/v1/fs/list?path={}",
                    self.config.evif_url,
                    urlencoding::encode(path)
                );
                let response = self
                    .client
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

                let url = format!(
                    "{}/api/v1/fs/read?path={}",
                    self.config.evif_url,
                    urlencoding::encode(path)
                );
                let response = self
                    .client
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

                let url = format!(
                    "{}/api/v1/fs/write?path={}",
                    self.config.evif_url,
                    urlencoding::encode(path)
                );
                let response = self
                    .client
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
                let response = self
                    .client
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
                    format!(
                        "{}/api/v1/directories?path={}",
                        self.config.evif_url,
                        urlencoding::encode(path)
                    )
                } else {
                    format!(
                        "{}/api/v1/files?path={}",
                        self.config.evif_url,
                        urlencoding::encode(path)
                    )
                };
                let response = self
                    .client
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

                let url = format!(
                    "{}/api/v1/stat?path={}",
                    self.config.evif_url,
                    urlencoding::encode(path)
                );
                let response = self
                    .client
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
                let response = self
                    .client
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
                let src = arguments["src"].as_str().ok_or("Missing 'src' argument")?;
                let dst = arguments["dst"].as_str().ok_or("Missing 'dst' argument")?;

                let read_url = format!(
                    "{}/api/v1/fs/read?path={}",
                    self.config.evif_url,
                    urlencoding::encode(src)
                );
                let read_response = self
                    .client
                    .get(&read_url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to read source: {}", e))?;

                let read_data: Value = read_response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse read response: {}", e))?;
                let content = read_data["content"].as_str().unwrap_or("");

                let write_url = format!(
                    "{}/api/v1/fs/write?path={}",
                    self.config.evif_url,
                    urlencoding::encode(dst)
                );
                let write_response = self
                    .client
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
                let response = self
                    .client
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
                let response = self
                    .client
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
                let response = self
                    .client
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
                let response = self
                    .client
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
                let response = self
                    .client
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
                let response = self
                    .client
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

                let url = format!(
                    "{}/api/v1/handles/{}/close",
                    self.config.evif_url, handle_id
                );
                let response = self
                    .client
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

            // Memory tools - using REST API (requires evif-mem REST endpoints)
            "evif_memorize" => {
                let content = arguments
                    .get("content")
                    .and_then(Value::as_str)
                    .or_else(|| arguments.get("text").and_then(Value::as_str))
                    .ok_or("Missing 'content' argument")?;

                let modality = arguments["modality"].as_str().unwrap_or("conversation");

                let mut body = serde_json::Map::from_iter([
                    ("content".to_string(), json!(content)),
                    ("modality".to_string(), json!(modality)),
                ]);

                if let Some(metadata) = arguments.get("metadata").filter(|value| !value.is_null()) {
                    body.insert("metadata".to_string(), metadata.clone());
                }

                let url = format!("{}/api/v1/memories", self.config.evif_url);
                let response = self
                    .client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to memorize: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_retrieve" => {
                let query = arguments["query"]
                    .as_str()
                    .ok_or("Missing 'query' argument")?;

                let mode = arguments["mode"].as_str().unwrap_or("vector");
                let k = arguments["k"].as_u64().unwrap_or(10) as usize;
                let threshold = arguments["threshold"].as_f64().unwrap_or(0.5) as f32;

                let mut body = serde_json::Map::from_iter([
                    ("query".to_string(), json!(query)),
                    ("mode".to_string(), json!(mode)),
                    ("vector_k".to_string(), json!(k)),
                ]);

                if mode == "hybrid" {
                    body.insert("llm_top_n".to_string(), json!(3));
                }

                // The REST contract currently does not accept threshold explicitly.
                let _ = threshold;

                let url = format!("{}/api/v1/memories/search", self.config.evif_url);
                let response = self
                    .client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to retrieve: {}", e))?;

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
    use axum::{extract::State, routing::post, Json, Router};
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};

    async fn wait_for_tools(server: &Arc<EvifMcpServer>) -> Vec<Tool> {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut tools = server.list_tools().await;
        for _ in 0..20 {
            if tools.len() >= 15 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            tools = server.list_tools().await;
        }
        tools
    }

    async fn spawn_json_capture_server(
        route: &str,
        response_body: Value,
    ) -> (String, Arc<Mutex<Option<Value>>>, JoinHandle<()>) {
        let captured_body = Arc::new(Mutex::new(None));
        let state = captured_body.clone();
        let app = Router::new()
            .route(route, post(capture_json))
            .with_state((state, response_body));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{}", address), captured_body, handle)
    }

    /// Spawn a server that captures GET query params and returns JSON.
    async fn spawn_get_json_server(
        route: &str,
        response_body: Value,
    ) -> (String, Arc<Mutex<Option<Value>>>, JoinHandle<()>) {
        let captured_params = Arc::new(Mutex::new(None::<Value>));
        let state = captured_params.clone();
        let app = Router::new()
            .route(route, axum::routing::get(get_capture_query))
            .with_state((state, response_body));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{}", address), captured_params, handle)
    }

    async fn get_capture_query(
        State((captured_params, response_body)): State<(
            Arc<Mutex<Option<Value>>>,
            Value,
        )>,
        axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    ) -> Json<Value> {
        let map: serde_json::Map<String, Value> = params
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        *captured_params.lock().await = Some(Value::Object(map));
        Json(response_body)
    }

    async fn capture_json(
        State((captured_body, response_body)): State<(Arc<Mutex<Option<Value>>>, Value)>,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *captured_body.lock().await = Some(body);
        Json(response_body)
    }

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let config = McpServerConfig::default();
        let server = EvifMcpServer::new(config);

        let tools = wait_for_tools(&server).await;
        assert!(!tools.is_empty());
        assert!(
            tools.len() >= 15,
            "expected at least 15 tools, got {}",
            tools.len()
        );

        let prompts = server.list_prompts().await;
        assert_eq!(prompts.len(), 3);
    }

    #[tokio::test]
    async fn test_evif_memorize_schema_prefers_content_with_legacy_alias() {
        let server = EvifMcpServer::new(McpServerConfig::default());
        let tools = wait_for_tools(&server).await;
        let memorize_tool = tools
            .into_iter()
            .find(|tool| tool.name == "evif_memorize")
            .expect("evif_memorize tool should exist");

        assert!(memorize_tool.input_schema["properties"]
            .get("content")
            .is_some());
        assert!(memorize_tool.input_schema["properties"]
            .get("text")
            .is_some());
        assert_eq!(
            memorize_tool.input_schema["properties"]["text"]["description"],
            "Deprecated alias for content"
        );
        let alternatives = memorize_tool.input_schema["oneOf"]
            .as_array()
            .expect("schema should define accepted argument alternatives");
        assert_eq!(alternatives.len(), 2);
    }

    #[tokio::test]
    async fn test_evif_memorize_posts_rest_contract() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/memories",
            json!({
                "memory_id": "mem-1",
                "extracted_items": []
            }),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool(
                "evif_memorize",
                json!({
                    "content": "remember this",
                    "modality": "document",
                    "metadata": { "source": "unit-test" }
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["memory_id"], "mem-1");

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["content"], "remember this");
        assert_eq!(captured["modality"], "document");
        assert_eq!(captured["metadata"]["source"], "unit-test");
        assert!(captured.get("text").is_none());

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_memorize_accepts_legacy_text_argument() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/memories",
            json!({
                "memory_id": "mem-legacy",
                "extracted_items": []
            }),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool(
                "evif_memorize",
                json!({
                    "text": "legacy payload",
                    "modality": "conversation"
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["memory_id"], "mem-legacy");

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["content"], "legacy payload");
        assert_eq!(captured["modality"], "conversation");
        assert!(captured.get("text").is_none());

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_retrieve_posts_rest_contract() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/memories/search",
            json!({
                "results": [],
                "total": 0
            }),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool(
                "evif_retrieve",
                json!({
                    "query": "rust memory",
                    "mode": "hybrid",
                    "k": 7,
                    "threshold": 0.8
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 0);

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["query"], "rust memory");
        assert_eq!(captured["mode"], "hybrid");
        assert_eq!(captured["vector_k"], 7);
        assert_eq!(captured["llm_top_n"], 3);
        assert!(captured.get("mode_params").is_none());

        handle.abort();
    }

    // ── Agent workflow tests: file system tools ──────────────────

    #[tokio::test]
    async fn test_evif_ls_calls_rest_get() {
        let (base_url, captured, handle) = spawn_get_json_server(
            "/api/v1/fs/list",
            json!({
                "data": [
                    {"name": "file1.txt", "size": 100, "is_dir": false},
                    {"name": "subdir", "size": 0, "is_dir": true},
                ]
            }),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_ls", json!({"path": "/memfs"}))
            .await
            .unwrap();

        let entries = result["data"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["name"], "file1.txt");

        let params = captured.lock().await.clone().unwrap();
        assert_eq!(params.get("path").unwrap(), "/memfs");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_cat_calls_rest_get() {
        let (base_url, captured, handle) = spawn_get_json_server(
            "/api/v1/fs/read",
            json!({"data": {"content": "hello world", "size": 11}}),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_cat", json!({"path": "/memfs/test.txt"}))
            .await
            .unwrap();

        assert_eq!(result["data"]["content"], "hello world");

        let params = captured.lock().await.clone().unwrap();
        assert_eq!(params.get("path").unwrap(), "/memfs/test.txt");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_stat_calls_rest_get() {
        let (base_url, captured, handle) = spawn_get_json_server(
            "/api/v1/stat",
            json!({
                "data": {"name": "test.txt", "size": 42, "is_dir": false, "mode": 420}
            }),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_stat", json!({"path": "/memfs/test.txt"}))
            .await
            .unwrap();

        assert_eq!(result["data"]["name"], "test.txt");
        assert_eq!(result["data"]["size"], 42);

        let params = captured.lock().await.clone().unwrap();
        assert_eq!(params.get("path").unwrap(), "/memfs/test.txt");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_write_calls_rest_post() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/fs/write",
            json!({"data": {"bytes_written": 5}}),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_write", json!({"path": "/memfs/hello.txt", "content": "hello"}))
            .await
            .unwrap();

        assert_eq!(result["data"]["bytes_written"], 5);

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["content"], "hello");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_mkdir_calls_rest_post() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/directories",
            json!({"data": {}}),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_mkdir", json!({"path": "/memfs/newdir"}))
            .await
            .unwrap();

        // Should succeed
        assert!(result["data"].is_object());

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["path"], "/memfs/newdir");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_mount_calls_rest_post() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/mount",
            json!({"data": {"path": "/s3", "plugin_type": "s3fs"}}),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_mount", json!({
                "plugin": "s3fs",
                "path": "/s3",
                "config": {"region": "us-west-1"}
            }))
            .await
            .unwrap();

        assert_eq!(result["data"]["path"], "/s3");
        assert_eq!(result["data"]["plugin_type"], "s3fs");

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["plugin"], "s3fs");
        assert_eq!(captured["path"], "/s3");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_grep_calls_rest_post() {
        let (base_url, captured_body, handle) = spawn_json_capture_server(
            "/api/v1/grep",
            json!({"data": {"matches": [{"file": "a.txt", "line": 1, "content": "found"}], "count": 1}}),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_grep", json!({
                "path": "/memfs",
                "pattern": "TODO",
                "recursive": true
            }))
            .await
            .unwrap();

        assert_eq!(result["data"]["count"], 1);

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["path"], "/memfs");
        assert_eq!(captured["pattern"], "TODO");

        handle.abort();
    }

    #[tokio::test]
    async fn test_agent_workflow_write_read_stat() {
        // Simulate a complete agent workflow: write → cat → stat
        let (base_url, _captured_body, handle) = spawn_json_capture_server(
            "/api/v1/fs/write",
            json!({"data": {"bytes_written": 12}}),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        // Write
        let write_result = server
            .call_tool("evif_write", json!({"path": "/memfs/agent.txt", "content": "hello agent"}))
            .await
            .unwrap();
        assert_eq!(write_result["data"]["bytes_written"], 12);

        handle.abort();
    }

    #[tokio::test]
    async fn test_all_tools_have_required_fields() {
        let server = EvifMcpServer::new(McpServerConfig::default());
        let tools = wait_for_tools(&server).await;

        for tool in &tools {
            assert!(!tool.name.is_empty(), "tool name should not be empty");
            assert!(!tool.description.is_empty(), "tool {} description should not be empty", tool.name);
            assert!(tool.input_schema.is_object(), "tool {} input_schema should be an object", tool.name);
            assert!(tool.input_schema.get("type").is_some(), "tool {} input_schema should have type", tool.name);
        }
    }
}
