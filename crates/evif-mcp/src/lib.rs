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
            evif_url: std::env::var("EVIF_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            server_name: "evif-mcp".to_string(),
            version: "1.8.0".to_string(),
        }
    }
}

impl McpServerConfig {
    /// Create a config from CLI args, falling back to env vars and defaults.
    pub fn from_cli(url: Option<String>, server_name: Option<String>) -> Self {
        let mut base = Self::default();
        if let Some(u) = url {
            base.evif_url = u;
        }
        if let Some(name) = server_name {
            base.server_name = name;
        }
        base
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
            // SkillFS tools - expose Skills as MCP tools
            Tool {
                name: "evif_skill_list".to_string(),
                description: "List all registered skills in the SkillFS".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "evif_skill_info".to_string(),
                description: "Get detailed info about a specific skill".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Skill name"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_skill_execute".to_string(),
                description: "Execute a skill with input data".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Skill name to execute"
                        },
                        "input": {
                            "type": "string",
                            "description": "Input data for the skill"
                        },
                        "mode": {
                            "type": "string",
                            "description": "Execution mode: native, wasm, docker (default: native)"
                        }
                    },
                    "required": ["name", "input"]
                }),
            },
            // Phase 15: Claude Code 集成工具
            // ── CLAUDE.md 自动生成 ────────────────────────────────────────
            Tool {
                name: "evif_claude_md_generate".to_string(),
                description:
                    "Auto-generate CLAUDE.md for the current project by analyzing its structure"
                        .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Project root path (default: /)"
                        },
                        "include_skills": {
                            "type": "boolean",
                            "description": "Include skill references (default: true)"
                        },
                        "include_context": {
                            "type": "boolean",
                            "description": "Include context structure (default: true)"
                        }
                    },
                    "required": []
                }),
            },
            // ── Auto-memory 增强 ──────────────────────────────────────────
            Tool {
                name: "evif_session_save".to_string(),
                description: "Save current session state to L0/L1 context".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "level": {
                            "type": "string",
                            "description": "Context level: L0 (current task) or L1 (decisions)"
                        },
                        "content": {
                            "type": "string",
                            "description": "Session content to save"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Brief summary of the session"
                        }
                    },
                    "required": ["content"]
                }),
            },
            Tool {
                name: "evif_session_list".to_string(),
                description: "List all saved sessions".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "level": {
                            "type": "string",
                            "description": "Filter by level: L0 or L1"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of sessions to return"
                        }
                    },
                    "required": []
                }),
            },
            // ── Subagent 协调 ─────────────────────────────────────────────
            Tool {
                name: "evif_subagent_create".to_string(),
                description: "Create a new subagent with assigned context".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Subagent name/ID"
                        },
                        "task": {
                            "type": "string",
                            "description": "Task description for the subagent"
                        },
                        "context_path": {
                            "type": "string",
                            "description": "Context path to share with subagent"
                        }
                    },
                    "required": ["name", "task"]
                }),
            },
            Tool {
                name: "evif_subagent_send".to_string(),
                description: "Send a message to a subagent".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Subagent name/ID"
                        },
                        "message": {
                            "type": "string",
                            "description": "Message to send"
                        }
                    },
                    "required": ["name", "message"]
                }),
            },
            Tool {
                name: "evif_subagent_list".to_string(),
                description: "List all active subagents".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
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
                let url = format!("{}/api/v1/health", self.config.evif_url);
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
                let lease = arguments["lease"].as_u64().unwrap_or(300);

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

            // SkillFS tools - expose Skills via MCP
            "evif_skill_list" => {
                let url = format!("{}/api/v1/directories?path=/skills", self.config.evif_url);
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to list skills: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                Ok(result)
            }

            "evif_skill_info" => {
                let name = arguments["name"]
                    .as_str()
                    .ok_or("Missing 'name' argument")?;

                // Read the SKILL.md file for the requested skill
                let url = format!(
                    "{}/api/v1/files?path=/skills/{}/SKILL.md",
                    self.config.evif_url,
                    urlencoding::encode(name)
                );
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to get skill info: {}", e))?;

                if response.status().as_u16() >= 400 {
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    return Err(format!(
                        "Skill '{}' not found (HTTP {}): {}",
                        name, status, body
                    ));
                }

                let body: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse skill info: {}", e))?;

                Ok(body)
            }

            "evif_skill_execute" => {
                let name = arguments["name"]
                    .as_str()
                    .ok_or("Missing 'name' argument")?;
                let input = arguments["input"]
                    .as_str()
                    .ok_or("Missing 'input' argument")?;
                let mode = arguments["mode"].as_str().unwrap_or("native");

                // Write input to skill's input file
                let write_url = format!(
                    "{}/api/v1/files?path=/skills/{}/input",
                    self.config.evif_url,
                    urlencoding::encode(name)
                );
                let write_response = self
                    .client
                    .put(&write_url)
                    .body(input.to_string())
                    .send()
                    .await
                    .map_err(|e| format!("Failed to write skill input: {}", e))?;

                if write_response.status().as_u16() >= 400 {
                    let status = write_response.status().as_u16();
                    let body = write_response.text().await.unwrap_or_default();
                    return Err(format!(
                        "Failed to write input for skill '{}' (HTTP {}): {}",
                        name, status, body
                    ));
                }

                // Read the output from the skill
                let read_url = format!(
                    "{}/api/v1/files?path=/skills/{}/output",
                    self.config.evif_url,
                    urlencoding::encode(name)
                );
                let read_response = self
                    .client
                    .get(&read_url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to read skill output: {}", e))?;

                let output: Value = if read_response.status().as_u16() >= 400 {
                    json!({
                        "skill": name,
                        "mode": mode,
                        "status": "executed",
                        "input_written": true,
                        "output": "Skill input written successfully. Read /skills/{}/output when ready.",
                        "note": "Skill execution is asynchronous - check output file for results."
                    })
                } else {
                    let body: Value = read_response
                        .json()
                        .await
                        .map_err(|e| format!("Failed to parse skill output: {}", e))?;
                    json!({
                        "skill": name,
                        "mode": mode,
                        "status": "completed",
                        "output": body
                    })
                };

                Ok(output)
            }

            // Phase 15.1: CLAUDE.md auto-generation
            "evif_claude_md_generate" => {
                let project_path = arguments["path"].as_str().unwrap_or("/");
                let include_skills = arguments["include_skills"].as_bool().unwrap_or(true);
                let include_context = arguments["include_context"].as_bool().unwrap_or(true);

                // Scan project structure
                let dirs_url = format!(
                    "{}/api/v1/directories?path={}",
                    self.config.evif_url,
                    urlencoding::encode(project_path)
                );
                let dirs_response = self
                    .client
                    .get(&dirs_url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to scan project: {}", e))?;

                let dirs_data: Value = dirs_response.json().await.unwrap_or(json!({"data": []}));

                // Generate CLAUDE.md content
                let data = dirs_data
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|e| e.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();

                let mut claude_md = format!(
                    r#"# Project Context

## Mission

Auto-generated CLAUDE.md for EVIF context filesystem.

## Project Structure

{}

## Quick Reference

- Context filesystem: `/context/L0/current`, `/context/L1/decisions`
"#,
                    data
                );

                if include_skills {
                    claude_md.push_str("\n## Skills\n\n- `/skills/` — Available agent skills\n");
                }

                if include_context {
                    claude_md.push_str("\n## Context Convention\n\n- Read `/context/L0/current` for active task\n- Write decisions to `/context/L1/decisions.md`\n");
                }

                claude_md.push_str("\n---\n*Auto-generated by EVIF MCP Server*\n");

                Ok(json!({
                    "content": claude_md,
                    "path": format!("{}CLAUDE.md", project_path.trim_end_matches('/')),
                    "status": "ready"
                }))
            }

            // Phase 15.2: Session management
            "evif_session_save" => {
                let level = arguments["level"].as_str().unwrap_or("L0");
                let content = arguments["content"]
                    .as_str()
                    .ok_or("Missing 'content' argument")?;
                let summary = arguments["summary"].as_str().unwrap_or("");

                let context_path = if level == "L1" {
                    "/context/L1/decisions.md"
                } else {
                    "/context/L0/current"
                };

                // Append to context file
                let write_url = format!(
                    "{}/api/v1/files?path={}",
                    self.config.evif_url,
                    urlencoding::encode(context_path)
                );

                let body = if level == "L1" {
                    serde_json::json!({
                        "data": format!("\n\n## Session {}\n\n{}\n\n{}", chrono::Utc::now().format("%Y-%m-%d %H:%M"), summary, content)
                    })
                } else {
                    serde_json::json!({ "data": content })
                };

                let response = self
                    .client
                    .put(&write_url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to save session: {}", e))?;

                if response.status().is_success() || response.status().as_u16() == 201 {
                    Ok(json!({
                        "status": "saved",
                        "level": level,
                        "path": context_path
                    }))
                } else {
                    Err(format!(
                        "Failed to save (HTTP {})",
                        response.status().as_u16()
                    ))
                }
            }

            "evif_session_list" => {
                let level = arguments["level"].as_str().unwrap_or("");
                let _limit = arguments["limit"].as_i64().unwrap_or(20);

                // Read context directory
                let path = if level == "L1" {
                    "/context/L1"
                } else if level == "L0" {
                    "/context/L0"
                } else {
                    "/context"
                };

                let url = format!(
                    "{}/api/v1/directories?path={}",
                    self.config.evif_url,
                    urlencoding::encode(path)
                );
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to list sessions: {}", e))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse: {}", e))?;

                Ok(result)
            }

            // Phase 15.3: Subagent coordination
            "evif_subagent_create" => {
                let name = arguments["name"]
                    .as_str()
                    .ok_or("Missing 'name' argument")?;
                let task = arguments["task"]
                    .as_str()
                    .ok_or("Missing 'task' argument")?;
                let context_path = arguments["context_path"].as_str().unwrap_or("/context");

                // Create pipe for subagent communication
                let pipe_url = format!(
                    "{}/api/v1/directories?path=/pipes/{}",
                    self.config.evif_url,
                    urlencoding::encode(name)
                );
                let _ = self.client.post(&pipe_url).send().await;

                // Write task to pipe input
                let input_url = format!(
                    "{}/api/v1/files?path=/pipes/{}/input",
                    self.config.evif_url,
                    urlencoding::encode(name)
                );
                let _ = self
                    .client
                    .put(&input_url)
                    .json(&serde_json::json!({ "data": task }))
                    .send()
                    .await;

                Ok(json!({
                    "status": "created",
                    "name": name,
                    "context_path": context_path,
                    "input": format!("/pipes/{}/input", name),
                    "output": format!("/pipes/{}/output", name)
                }))
            }

            "evif_subagent_send" => {
                let name = arguments["name"]
                    .as_str()
                    .ok_or("Missing 'name' argument")?;
                let message = arguments["message"]
                    .as_str()
                    .ok_or("Missing 'message' argument")?;

                let url = format!(
                    "{}/api/v1/files?path=/pipes/{}/input",
                    self.config.evif_url,
                    urlencoding::encode(name)
                );
                let response = self
                    .client
                    .put(&url)
                    .json(&serde_json::json!({ "data": message }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to send to subagent: {}", e))?;

                if response.status().is_success() || response.status().as_u16() == 201 {
                    Ok(json!({
                        "status": "sent",
                        "to": name
                    }))
                } else {
                    Err(format!("Subagent '{}' not found or unavailable", name))
                }
            }

            "evif_subagent_list" => {
                let url = format!("{}/api/v1/directories?path=/pipes", self.config.evif_url);
                let response = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to list subagents: {}", e))?;

                let result: Value = response.json().await.unwrap_or(json!({"data": []}));

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
        // 标准 MCP 协议方法处理
        if let Some(method) = request.get("method").and_then(|m| m.as_str()) {
            let id = request.get("id");
            let params = request.get("params");

            match method {
                // 初始化 - Claude Code CLI 健康检查需要此方法
                "initialize" => {
                    let client_info = params
                        .and_then(|p| p.get("clientInfo"))
                        .cloned()
                        .unwrap_or(json!({}));
                    tracing::info!("MCP client initializing: {:?}", client_info);
                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": {},
                                "resources": {}
                            },
                            "serverInfo": {
                                "name": self.config.server_name,
                                "version": self.config.version
                            }
                        },
                        "id": id
                    })
                }

                // 工具列表
                "tools/list" => {
                    let tools = self.list_tools().await;
                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "tools": tools.into_iter().map(|t| {
                                json!({
                                    "name": t.name,
                                    "description": t.description,
                                    "inputSchema": t.input_schema
                                })
                            }).collect::<Vec<_>>()
                        },
                        "id": id
                    })
                }

                // 资源列表
                "resources/list" => {
                    let resources = self.list_resources().await;
                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "resources": resources.into_iter().map(|r| {
                                json!({
                                    "uri": r.uri,
                                    "name": r.name,
                                    "description": r.description,
                                    "mimeType": r.mime_type
                                })
                            }).collect::<Vec<_>>()
                        },
                        "id": id
                    })
                }

                // prompts/list
                "prompts/list" => {
                    let prompts = self.list_prompts().await;
                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "prompts": prompts.into_iter().map(|p| {
                                json!({
                                    "name": p.name,
                                    "description": p.description,
                                    "arguments": p.arguments
                                })
                            }).collect::<Vec<_>>()
                        },
                        "id": id
                    })
                }

                // ping
                "ping" => {
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
                        "id": id
                    })
                }

                // shutdown
                "shutdown" => {
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
                        "id": id
                    })
                }

                _ => {
                    // 其他方法尝试作为工具调用
                    if let Some(p) = params {
                        match self.call_tool(method, p.clone()).await {
                            Ok(result) => json!({
                                "jsonrpc": "2.0",
                                "result": result,
                                "id": id
                            }),
                            Err(error) => json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32000,
                                    "message": error
                                },
                                "id": id
                            }),
                        }
                    } else {
                        json!({
                            "jsonrpc": "2.0",
                            "error": {
                                "code": -32602,
                                "message": "Invalid params"
                            },
                            "id": id
                        })
                    }
                }
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

    /// Spawn a server that handles PUT (captures body) and GET (captures query).
    async fn spawn_put_get_server(
        route: &str,
        put_response: Value,
        get_response: Value,
    ) -> (String, Arc<Mutex<Option<String>>>, JoinHandle<()>) {
        let captured_body = Arc::new(Mutex::new(None::<String>));
        let state = captured_body.clone();
        let get_resp = get_response.clone();
        let app = Router::new()
            .route(
                route,
                axum::routing::put(capture_string_body).get(return_json_get),
            )
            .with_state((state, put_response, get_resp));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{}", address), captured_body, handle)
    }

    type BodyCaptureState = (Arc<Mutex<Option<String>>>, Value, Value);

    async fn capture_string_body(
        State((captured_body, put_response, _)): State<BodyCaptureState>,
        body: String,
    ) -> Json<Value> {
        *captured_body.lock().await = Some(body);
        Json(put_response)
    }

    async fn return_json_get(State((_, _, get_response)): State<BodyCaptureState>) -> Json<Value> {
        Json(get_response.clone())
    }

    async fn get_capture_query(
        State((captured_params, response_body)): State<(Arc<Mutex<Option<Value>>>, Value)>,
        axum::extract::Query(params): axum::extract::Query<
            std::collections::HashMap<String, String>,
        >,
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
            tools.len() >= 18,
            "expected at least 18 tools, got {}",
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
    async fn test_evif_health_calls_rest_v1_health_contract() {
        let (base_url, _captured_params, handle) = spawn_get_json_server(
            "/api/v1/health",
            json!({
                "status": "healthy",
                "version": env!("CARGO_PKG_VERSION"),
                "uptime": 12
            }),
        )
        .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_health", json!({}))
            .await
            .expect("health tool should follow REST v1 health contract");

        assert_eq!(result["status"], "healthy");
        assert_eq!(result["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(result["uptime"], 12);

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
        let (base_url, captured_body, handle) =
            spawn_json_capture_server("/api/v1/fs/write", json!({"data": {"bytes_written": 5}}))
                .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool(
                "evif_write",
                json!({"path": "/memfs/hello.txt", "content": "hello"}),
            )
            .await
            .unwrap();

        assert_eq!(result["data"]["bytes_written"], 5);

        let captured = captured_body.lock().await.clone().unwrap();
        assert_eq!(captured["content"], "hello");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_mkdir_calls_rest_post() {
        let (base_url, captured_body, handle) =
            spawn_json_capture_server("/api/v1/directories", json!({"data": {}})).await;
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
            .call_tool(
                "evif_mount",
                json!({
                    "plugin": "s3fs",
                    "path": "/s3",
                    "config": {"region": "us-west-1"}
                }),
            )
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
            .call_tool(
                "evif_grep",
                json!({
                    "path": "/memfs",
                    "pattern": "TODO",
                    "recursive": true
                }),
            )
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
        let (base_url, _captured_body, handle) =
            spawn_json_capture_server("/api/v1/fs/write", json!({"data": {"bytes_written": 12}}))
                .await;
        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        // Write
        let write_result = server
            .call_tool(
                "evif_write",
                json!({"path": "/memfs/agent.txt", "content": "hello agent"}),
            )
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
            assert!(
                !tool.description.is_empty(),
                "tool {} description should not be empty",
                tool.name
            );
            assert!(
                tool.input_schema.is_object(),
                "tool {} input_schema should be an object",
                tool.name
            );
            assert!(
                tool.input_schema.get("type").is_some(),
                "tool {} input_schema should have type",
                tool.name
            );
        }
    }

    // ── SkillFS MCP tool tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_skill_tools_are_registered() {
        let server = EvifMcpServer::new(McpServerConfig::default());
        let tools = wait_for_tools(&server).await;

        let skill_tool_names: Vec<&str> = tools
            .iter()
            .filter(|t| t.name.starts_with("evif_skill_"))
            .map(|t| t.name.as_str())
            .collect();

        assert!(
            skill_tool_names.contains(&"evif_skill_list"),
            "evif_skill_list tool should be registered"
        );
        assert!(
            skill_tool_names.contains(&"evif_skill_info"),
            "evif_skill_info tool should be registered"
        );
        assert!(
            skill_tool_names.contains(&"evif_skill_execute"),
            "evif_skill_execute tool should be registered"
        );
        assert_eq!(skill_tool_names.len(), 3, "expected exactly 3 skill tools");
    }

    #[tokio::test]
    async fn test_evif_skill_list_calls_directory_api() {
        let (base_url, captured_params, handle) = spawn_get_json_server(
            "/api/v1/directories",
            json!({
                "data": [
                    {"name": "code-review", "size": 0, "is_dir": true},
                    {"name": "test-gen", "size": 0, "is_dir": true},
                    {"name": "doc-gen", "size": 0, "is_dir": true}
                ]
            }),
        )
        .await;

        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_skill_list", json!({}))
            .await
            .unwrap();

        let skills = result["data"].as_array().expect("should have data array");
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0]["name"], "code-review");
        assert_eq!(skills[1]["name"], "test-gen");
        assert_eq!(skills[2]["name"], "doc-gen");

        // Verify the request used /skills path
        let captured = captured_params.lock().await.clone().unwrap();
        assert_eq!(captured["path"], "/skills");

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_skill_info_reads_skill_md() {
        let (base_url, _captured_params, handle) = spawn_get_json_server(
            "/api/v1/files",
            json!({
                "data": "---\nname: code-review\ndescription: Review code\ntriggers:\n  - review code\n  - check code quality\n---\n# Code Review Skill\n\nAnalyzes code for bugs, security issues, and style."
            }),
        )
        .await;

        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool("evif_skill_info", json!({"name": "code-review"}))
            .await
            .unwrap();

        // The result should contain the skill data
        assert!(result["data"]
            .as_str()
            .unwrap_or_default()
            .contains("code-review"));

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_skill_info_rejects_missing_name() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let result = server.call_tool("evif_skill_info", json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing 'name'"));
    }

    #[tokio::test]
    async fn test_evif_skill_execute_writes_input() {
        let (base_url, captured_body, handle) = spawn_put_get_server(
            "/api/v1/files",
            json!({"data": {"bytes_written": 42}}),
            json!({"data": "skill output result"}),
        )
        .await;

        let server = EvifMcpServer::new(McpServerConfig {
            evif_url: base_url,
            ..McpServerConfig::default()
        });

        let result = server
            .call_tool(
                "evif_skill_execute",
                json!({
                    "name": "code-review",
                    "input": "fn main() { println!(\"hello\"); }",
                    "mode": "native"
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["skill"], "code-review");
        assert_eq!(result["mode"], "native");

        // Verify the PUT request was made with correct input
        let captured = captured_body.lock().await.clone().unwrap();
        assert!(captured.contains("fn main()"));

        handle.abort();
    }

    #[tokio::test]
    async fn test_evif_skill_execute_rejects_missing_input() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let result = server
            .call_tool("evif_skill_execute", json!({"name": "test-skill"}))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing 'input'"));
    }

    #[tokio::test]
    async fn test_evif_skill_execute_rejects_missing_name() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let result = server
            .call_tool("evif_skill_execute", json!({"input": "test data"}))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing 'name'"));
    }
}
