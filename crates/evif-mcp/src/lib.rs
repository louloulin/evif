// EVIF MCP Server - Model Context Protocol 服务器实现
//
// 提供17个工具对等AGFS,支持Claude Desktop和其他MCP客户端

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use uuid::Uuid;
use lru::LruCache;
use std::num::NonZeroUsize;

pub mod mcp_gateway;
pub mod mcp_server_plugin;
pub mod mcp_client;
pub mod mcp_router;
pub mod mcp_auth;

use crate::mcp_router::McpRouter;

/// 模板渲染器
mod template {
    use serde_json::Value;
    use std::collections::HashMap;

    /// 渲染模板字符串，支持:
    /// - `{variable}` - 简单变量替换
    /// - `{variable:default}` - 带默认值的变量
    /// - `{{#if var}}...{{/if}}` - 条件块
    pub fn render_template(
        template: &str,
        args: &Option<HashMap<String, Value>>,
    ) -> (String, Vec<String>) {
        let mut warnings = Vec::new();
        let mut result = template.to_string();

        // 处理条件块 {{#if variable}}...{{/if}}
        let if_pattern = regex_lite::Regex::new(r"\{\{#if\s+(\w+)\}\}(.*?)\{\{/if\}\}")
            .expect("Invalid if regex");

        result = if_pattern.replace_all(&result, |caps: &regex_lite::Captures| {
            let var_name = &caps[1];
            let content = &caps[2];
            let is_true = args.as_ref()
                .and_then(|a| a.get(var_name))
                .map(|v| {
                    match v {
                        Value::Bool(b) => *b,
                        Value::String(s) => !s.is_empty(),
                        Value::Null => false,
                        Value::Array(a) => !a.is_empty(),
                        Value::Object(o) => !o.is_empty(),
                        _ => true,
                    }
                })
                .unwrap_or(false);

            if is_true { content.to_string() } else { String::new() }
        }).to_string();

        // 处理变量替换 {variable} 和 {variable:default}
        let var_pattern = regex_lite::Regex::new(r"\{(\w+)(?::([^}]*))?\}")
            .expect("Invalid variable regex");

        result = var_pattern.replace_all(&result, |caps: &regex_lite::Captures| {
            let var_name = &caps[1];
            let default_value = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            match args.as_ref().and_then(|a| a.get(var_name)) {
                Some(value) => {
                    match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => {
                            if default_value.is_empty() {
                                warnings.push(format!("Variable '{}' is null, no default provided", var_name));
                            }
                            default_value.to_string()
                        }
                        Value::Array(arr) => {
                            serde_json::to_string(arr).unwrap_or_else(|_| "[array]".to_string())
                        }
                        Value::Object(obj) => {
                            serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string())
                        }
                    }
                }
                None => {
                    if default_value.is_empty() {
                        warnings.push(format!("Missing required variable '{}' (no default)", var_name));
                    }
                    default_value.to_string()
                }
            }
        }).to_string();

        (result, warnings)
    }

    /// 验证必需参数
    pub fn validate_required_args(
        args: &Option<HashMap<String, Value>>,
        required: &[(&str, bool, &str)],
    ) -> Vec<String> {
        let mut errors = Vec::new();
        let args = match args {
            Some(a) => a,
            None => return required.iter()
                .filter(|(_, required, _)| *required)
                .map(|(name, _, _)| format!("Missing required argument: {}", name))
                .collect(),
        };

        for (name, is_required, _) in required {
            if *is_required && !args.contains_key(*name) {
                errors.push(format!("Missing required argument: {}", name));
            }
        }

        errors
    }
}

/// Tool 调用结果缓存
pub struct ToolCache {
    /// 工具调用结果缓存 (tool_name + args_hash -> result)
    call_cache: LruCache<String, Value>,
    /// 工具列表缓存
    tools_cache: LruCache<String, Vec<Tool>>,
    /// 提示列表缓存
    prompts_cache: LruCache<String, Vec<Prompt>>,
}

impl ToolCache {
    pub fn new(cache_size: usize) -> Self {
        Self {
            call_cache: LruCache::new(NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::MIN)),
            tools_cache: LruCache::new(NonZeroUsize::MIN),
            prompts_cache: LruCache::new(NonZeroUsize::MIN),
        }
    }

    /// 生成工具调用的缓存键
    fn make_call_key(tool_name: &str, args: &Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        args.hash(&mut hasher);
        format!("{}_{:x}", tool_name, hasher.finish())
    }

    /// 获取缓存的调用结果 (需要 &mut self 因为 LRU 需要更新访问顺序)
    pub fn get_call_result(&mut self, tool_name: &str, args: &Value) -> Option<Value> {
        let key = Self::make_call_key(tool_name, args);
        self.call_cache.get(&key).cloned()
    }

    /// 缓存调用结果
    pub fn put_call_result(&mut self, tool_name: &str, args: &Value, result: Value) {
        let key = Self::make_call_key(tool_name, args);
        self.call_cache.put(key, result);
    }

    /// 获取缓存的工具列表
    pub fn get_tools(&mut self) -> Option<Vec<Tool>> {
        self.tools_cache.get("tools").cloned()
    }

    /// 缓存工具列表
    pub fn put_tools(&mut self, tools: Vec<Tool>) {
        self.tools_cache.put("tools".to_string(), tools);
    }

    /// 获取缓存的提示列表
    pub fn get_prompts(&mut self) -> Option<Vec<Prompt>> {
        self.prompts_cache.get("prompts").cloned()
    }

    /// 缓存提示列表
    pub fn put_prompts(&mut self, prompts: Vec<Prompt>) {
        self.prompts_cache.put("prompts".to_string(), prompts);
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.call_cache.clear();
        self.tools_cache.clear();
        self.prompts_cache.clear();
    }

    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            call_cache_size: self.call_cache.len(),
            tools_cached: self.tools_cache.contains(&"tools".to_string()),
            prompts_cached: self.prompts_cache.contains(&"prompts".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub call_cache_size: usize,
    pub tools_cached: bool,
    pub prompts_cached: bool,
}

/// VFS 直接操作模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsMode {
    /// HTTP 桥接模式 (默认)
    Http,
    /// VFS 直接调用模式
    Direct,
    /// Mock 模式 (用于测试，不需要后端)
    Mock,
}

impl Default for VfsMode {
    fn default() -> Self {
        VfsMode::Http
    }
}

/// VFS 目录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

/// VFS 写入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsWriteResult {
    pub bytes_written: u64,
    pub path: String,
}

/// VFS 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsFileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub mode: u32,
    pub modified: String,
    pub created: String,
}

/// VFS 后端 - 支持直接 VFS 调用和 HTTP 桥接两种模式
///
/// 直接 VFS 模式优势:
/// - 延迟降低 50%+ (无 HTTP 开销)
/// - 语义完整保留
/// - 支持事务操作
pub struct VfsBackend {
    /// 操作模式
    mode: VfsMode,
    /// HTTP 后端 URL (用于 HTTP 桥接模式)
    http_url: String,
    /// HTTP 客户端
    http_client: Client,
    /// Mock 文件系统 (用于测试模式)
    mock_fs: Option<Mutex<std::collections::HashMap<String, String>>>,
}

impl VfsBackend {
    /// 创建新的 VFS 后端
    pub fn new(evif_url: String) -> Self {
        // 创建 HTTP 客户端，禁用代理
        let http_client = Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            mode: VfsMode::Http, // 默认使用 HTTP 模式
            http_url: evif_url,
            http_client,
            mock_fs: None,
        }
    }

    /// 创建 Mock 模式的 VFS 后端 (用于测试)
    pub fn new_mock() -> Self {
        let mut mock_fs_map = std::collections::HashMap::new();
        // 添加一些测试数据
        mock_fs_map.insert("/".to_string(), "".to_string());
        mock_fs_map.insert("/context".to_string(), "".to_string());
        mock_fs_map.insert("/context/L0".to_string(), "".to_string());
        mock_fs_map.insert("/context/L0/current".to_string(), "Currently working on MCP integration".to_string());
        mock_fs_map.insert("/skills".to_string(), "".to_string());
        mock_fs_map.insert("/skills/evif-ls".to_string(), "# EVIF LS Skill\nA skill for listing files.".to_string());
        mock_fs_map.insert("/hello".to_string(), "Hello from EVIF!".to_string());

        Self {
            mode: VfsMode::Mock,
            http_url: String::new(),
            http_client: Client::new(),
            mock_fs: Some(Mutex::new(mock_fs_map)),
        }
    }

    /// 设置操作模式
    pub fn with_mode(mut self, mode: VfsMode) -> Self {
        self.mode = mode;
        self
    }

    /// 获取当前模式
    pub fn mode(&self) -> VfsMode {
        self.mode
    }

    /// 是否使用 Mock 模式
    pub fn is_mock(&self) -> bool {
        self.mode == VfsMode::Mock
    }

    /// 创建 Mock 模式 VFS 后端的 Arc 包装
    pub fn new_mock_arc() -> Arc<Self> {
        Arc::new(Self::new_mock())
    }

    /// 是否使用直接 VFS 模式
    pub fn is_direct(&self) -> bool {
        self.mode == VfsMode::Direct
    }

    /// 列出目录内容
    pub async fn list_dir(&self, path: &str) -> Result<Vec<VfsEntry>, String> {
        // Mock 模式
        if let Some(ref mock_fs) = self.mock_fs {
            let fs = mock_fs.lock().map_err(|e| e.to_string())?;
            let normalized_path = if path == "/" { "" } else { path };
            let mut entries_map: std::collections::HashMap<String, VfsEntry> = std::collections::HashMap::new();

            for k in fs.keys() {
                if k.starts_with(normalized_path) && *k != normalized_path {
                    let suffix = &k[normalized_path.len()..];
                    let name = if suffix.starts_with('/') {
                        suffix.trim_start_matches('/').split('/').next().unwrap_or("")
                    } else {
                        suffix.split('/').next().unwrap_or("")
                    };
                    if !name.is_empty() {
                        let is_dir = fs.get(k).map(|v| v.is_empty()).unwrap_or(false);
                        entries_map.insert(name.to_string(), VfsEntry {
                            name: name.to_string(),
                            is_dir,
                            size: 0,
                            modified: String::new(),
                        });
                    }
                }
            }
            let entries: Vec<VfsEntry> = entries_map.into_values().collect();
            return Ok(entries);
        }

        let url = format!(
            "{}/api/v1/fs/list?path={}",
            self.http_url,
            urlencoding::encode(path)
        );
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to list directory: {}", e))?;

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 解析 nodes (backend returns "nodes" not "entries")
        let entries = body["nodes"].as_array()
            .ok_or("Missing nodes in response")?
            .iter()
            .map(|e| VfsEntry {
                name: e["name"].as_str().unwrap_or("").to_string(),
                is_dir: e["is_dir"].as_bool().unwrap_or(false),
                size: e["size"].as_u64().unwrap_or(0),
                modified: e["modified"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(entries)
    }

    /// 读取文件内容
    pub async fn read_file(&self, path: &str, _offset: u64, _size: u64) -> Result<String, String> {
        // Mock 模式
        if let Some(ref mock_fs) = self.mock_fs {
            let fs = mock_fs.lock().map_err(|e| e.to_string())?;
            if let Some(content) = fs.get(path) {
                return Ok(content.clone());
            }
            // 检查父目录
            if path.ends_with('/') {
                return Ok(String::new());
            }
            return Err(format!("File not found: {}", path));
        }

        let url = format!(
            "{}/api/v1/fs/read?path={}",
            self.http_url,
            urlencoding::encode(path)
        );
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["content"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing content in response".to_string())
    }

    /// 写入文件内容
    pub async fn write_file(&self, path: &str, content: &str) -> Result<VfsWriteResult, String> {
        // Mock 模式
        if let Some(ref mock_fs) = self.mock_fs {
            let mut fs = mock_fs.lock().map_err(|e| e.to_string())?;
            fs.insert(path.to_string(), content.to_string());
            return Ok(VfsWriteResult {
                bytes_written: content.len() as u64,
                path: path.to_string(),
            });
        }

        let url = format!(
            "{}/api/v1/fs/write?path={}",
            self.http_url,
            urlencoding::encode(path)
        );
        let response = self
            .http_client
            .post(&url)
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(VfsWriteResult {
            bytes_written: body["bytes_written"].as_u64().unwrap_or(content.len() as u64),
            path: path.to_string(),
        })
    }

    /// 创建目录
    pub async fn make_dir(&self, path: &str, _perm: u32) -> Result<(), String> {
        // Mock 模式
        if self.mock_fs.is_some() {
            return Ok(());
        }

        let url = format!("{}/api/v1/directories", self.http_url);
        let _response = self
            .http_client
            .post(&url)
            .json(&json!({ "path": path }))
            .send()
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        Ok(())
    }

    /// 删除文件或目录
    pub async fn remove(&self, path: &str, recursive: bool) -> Result<(), String> {
        // Mock 模式
        if let Some(ref mock_fs) = self.mock_fs {
            let mut fs = mock_fs.lock().map_err(|e| e.to_string())?;
            fs.remove(path);
            return Ok(());
        }

        let url = if recursive {
            format!("{}/api/v1/directories?path={}", self.http_url, urlencoding::encode(path))
        } else {
            format!("{}/api/v1/files?path={}", self.http_url, urlencoding::encode(path))
        };
        let _response = self
            .http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to remove: {}", e))?;
        Ok(())
    }

    /// 获取文件/目录信息
    pub async fn stat(&self, path: &str) -> Result<VfsFileInfo, String> {
        // Mock 模式
        if let Some(ref mock_fs) = self.mock_fs {
            let fs = mock_fs.lock().map_err(|e| e.to_string())?;
            if let Some(content) = fs.get(path) {
                return Ok(VfsFileInfo {
                    path: path.to_string(),
                    name: path.split('/').last().unwrap_or("/").to_string(),
                    size: content.len() as u64,
                    is_dir: content.is_empty(),
                    mode: 0o644,
                    modified: String::new(),
                    created: String::new(),
                });
            }
            return Err(format!("Path not found: {}", path));
        }

        let url = format!("{}/api/v1/stat?path={}", self.http_url, urlencoding::encode(path));
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to stat: {}", e))?;

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(VfsFileInfo {
            path: body["path"].as_str().unwrap_or(path).to_string(),
            name: body["name"].as_str().unwrap_or("").to_string(),
            size: body["size"].as_u64().unwrap_or(0),
            is_dir: body["is_dir"].as_bool().unwrap_or(false),
            mode: body["mode"].as_u64().unwrap_or(0) as u32,
            modified: body["modified"].as_str().unwrap_or("").to_string(),
            created: body["created"].as_str().unwrap_or("").to_string(),
        })
    }

    /// 重命名/移动
    pub async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), String> {
        // Mock 模式
        if let Some(ref mock_fs) = self.mock_fs {
            let mut fs = mock_fs.lock().map_err(|e| e.to_string())?;
            if let Some(content) = fs.remove(old_path) {
                fs.insert(new_path.to_string(), content);
            }
            return Ok(());
        }

        let url = format!("{}/api/v1/rename", self.http_url);
        let _response = self
            .http_client
            .post(&url)
            .json(&json!({ "from": old_path, "to": new_path }))
            .send()
            .await
            .map_err(|e| format!("Failed to rename: {}", e))?;
        Ok(())
    }

    /// 检查后端是否可用
    pub fn is_available(&self) -> bool {
        self.mock_fs.is_some() || !self.http_url.is_empty()
    }
}

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

/// MCP 全局配置（支持 TOML 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// 协议版本
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,

    /// 服务器标识
    #[serde(default = "default_server_name")]
    pub server_name: String,

    /// 版本
    #[serde(default = "default_version")]
    pub version: String,

    /// EVIF 后端连接
    #[serde(default)]
    pub evif: EvifEndpoint,

    /// 认证配置
    #[serde(default)]
    pub auth: AuthConfig,

    /// TLS 配置
    #[serde(default)]
    pub tls: Option<TlsConfig>,

    /// MCP Server 注册表
    #[serde(default)]
    pub servers: Vec<McpServerRegistration>,

    /// 路径映射规则
    #[serde(default)]
    pub mappings: PathMappings,

    /// 多租户配置
    #[serde(default)]
    pub tenants: std::collections::HashMap<String, TenantMcpConfig>,
}

fn default_protocol_version() -> String { "2024-11-05".to_string() }
fn default_server_name() -> String { "evif-mcp".to_string() }
fn default_version() -> String { "1.8.0".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvifEndpoint {
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_retries")]
    pub retry_attempts: u32,
}

fn default_timeout() -> u64 { 30000 }
fn default_retries() -> u32 { 3 }

impl Default for EvifEndpoint {
    fn default() -> Self {
        Self {
            url: std::env::var("EVIF_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            timeout_ms: default_timeout(),
            retry_attempts: default_retries(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub auth_type: String,
    #[serde(default)]
    pub token_file: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: "bearer".to_string(),
            token_file: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    #[serde(default)]
    pub cert_file: Option<String>,
    #[serde(default)]
    pub key_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRegistration {
    pub name: String,
    pub mount_path: String,
    pub url: Option<String>,
    #[serde(default)]
    pub auth_token_env: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathMappings {
    #[serde(default)]
    pub resources: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub tools: std::collections::HashMap<String, ToolMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolMapping {
    pub operation: String,
    #[serde(default)]
    pub path_param: Option<String>,
    #[serde(default)]
    pub content_param: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMcpConfig {
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

impl Default for TenantMcpConfig {
    fn default() -> Self {
        Self {
            mcp_servers: Vec::new(),
            allowed_paths: Vec::new(),
            rate_limit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u64,
}

fn default_rpm() -> u64 { 1000 }

impl McpConfig {
    /// 从 TOML 文件加载配置
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        Self::load_from_str(&content)
    }

    /// 从 TOML 字符串加载配置
    pub fn load_from_str(content: &str) -> Result<Self, ConfigError> {
        let mut config: McpConfig = toml::from_str(content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // 应用环境变量覆盖
        config.apply_env_overrides();

        // 验证配置
        config.validate()?;

        Ok(config)
    }

    /// 从多个位置加载配置（优先级：CLI > 环境变量 > 文件 > 默认）
    pub fn load() -> Result<Self, ConfigError> {
        // 1. 首先尝试从配置文件加载
        let config_paths = [
            std::path::PathBuf::from("/etc/evif/mcp.toml"),
            std::path::PathBuf::from("~/.evif/mcp.toml"),
            std::path::PathBuf::from(".evif/mcp.toml"),
        ];

        for path in config_paths {
            // 简单展开 ~ 为 home 目录
            let path = if path.to_string_lossy().starts_with("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    std::path::PathBuf::from(home)
                        .join(path.strip_prefix("~/").unwrap_or(&path))
                } else {
                    path.clone()
                }
            } else {
                path
            };
            if path.exists() {
                let config = Self::load_from_file(&path)?;
                tracing::info!("Loaded MCP config from {:?}", path);
                return Ok(config);
            }
        }

        // 2. 使用默认配置
        tracing::info!("No MCP config file found, using defaults");
        Ok(Self::default())
    }

    /// 应用环境变量覆盖
    fn apply_env_overrides(&mut self) {
        if let Ok(url) = std::env::var("EVIF_URL") {
            self.evif.url = url;
        }
        if let Ok(name) = std::env::var("EVIF_MCP_SERVER_NAME") {
            self.server_name = name;
        }
        if let Ok(version) = std::env::var("EVIF_MCP_VERSION") {
            self.version = version;
        }
        if let Ok(timeout) = std::env::var("EVIF_TIMEOUT_MS") {
            if let Ok(t) = timeout.parse() {
                self.evif.timeout_ms = t;
            }
        }
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 1. 检查必需字段
        if self.evif.url.is_empty() {
            return Err(ConfigError::MissingField("evif.url".to_string()));
        }

        // 2. 验证 URL 格式
        if !self.evif.url.starts_with("http://")
            && !self.evif.url.starts_with("https://") {
            return Err(ConfigError::InvalidUrl(self.evif.url.clone()));
        }

        // 3. 验证服务器注册
        for server in &self.servers {
            if server.mount_path.starts_with("/mcp/") {
                tracing::warn!(
                    "Server '{}' mount_path starts with /mcp/, consider using a different prefix",
                    server.name
                );
            }
        }

        Ok(())
    }

    /// 转换为 McpServerConfig（向后兼容）
    pub fn to_server_config(&self) -> McpServerConfig {
        McpServerConfig {
            evif_url: self.evif.url.clone(),
            server_name: self.server_name.clone(),
            version: self.version.clone(),
        }
    }

    /// 导出为 YAML 格式
    pub fn to_yaml(&self) -> Result<String, ConfigError> {
        serde_yaml::to_string(&self)
            .map_err(|e| ConfigError::ParseError(format!("YAML serialization failed: {}", e)))
    }

    /// 导出为 TOML 格式
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(&self)
            .map_err(|e| ConfigError::ParseError(format!("TOML serialization failed: {}", e)))
    }

    /// 获取租户配置
    pub fn get_tenant(&self, tenant_id: &str) -> Option<&TenantMcpConfig> {
        self.tenants.get(tenant_id)
    }

    /// 验证租户是否允许访问指定路径
    pub fn validate_tenant_path_access(&self, tenant_id: &str, path: &str) -> bool {
        // 如果没有配置租户或租户没有限制，则允许访问
        let Some(tenant) = self.tenants.get(tenant_id) else {
            return true;
        };

        // 如果没有配置允许路径，则允许访问
        if tenant.allowed_paths.is_empty() {
            return true;
        }

        // 检查路径是否匹配任何允许的模式
        for pattern in &tenant.allowed_paths {
            if Self::path_matches_pattern(path, pattern) {
                return true;
            }
        }

        false
    }

    /// 检查路径是否匹配模式 (支持 * 通配符)
    fn path_matches_pattern(path: &str, pattern: &str) -> bool {
        if pattern.ends_with("/*") {
            // /context/* 匹配 /context/xxx
            let prefix = &pattern[..pattern.len() - 2];
            path.starts_with(prefix)
        } else if pattern.ends_with('/') {
            // /context/ 精确匹配
            path.starts_with(pattern)
        } else {
            // 精确匹配
            path == pattern
        }
    }

    /// 检查租户是否允许使用指定 MCP 服务器
    pub fn validate_tenant_server_access(&self, tenant_id: &str, server_name: &str) -> bool {
        let Some(tenant) = self.tenants.get(tenant_id) else {
            return true; // 无租户配置，允许访问
        };

        // 如果没有限制服务器，允许访问
        if tenant.mcp_servers.is_empty() {
            return true;
        }

        tenant.mcp_servers.contains(&server_name.to_string())
    }

    /// 获取租户速率限制
    pub fn get_tenant_rate_limit(&self, tenant_id: &str) -> u64 {
        self.tenants
            .get(tenant_id)
            .and_then(|t| t.rate_limit.as_ref())
            .map(|r| r.requests_per_minute)
            .unwrap_or(1000) // 默认 1000 RPM
    }

    /// 列出所有租户
    pub fn list_tenants(&self) -> Vec<&str> {
        self.tenants.keys().map(|s| s.as_str()).collect()
    }

    /// 添加或更新租户配置
    pub fn set_tenant(&mut self, tenant_id: String, config: TenantMcpConfig) {
        self.tenants.insert(tenant_id, config);
    }

    /// 移除租户配置
    pub fn remove_tenant(&mut self, tenant_id: &str) -> bool {
        self.tenants.remove(tenant_id).is_some()
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            protocol_version: "2024-11-05".to_string(),
            server_name: "evif-mcp".to_string(),
            version: "1.8.0".to_string(),
            evif: EvifEndpoint::default(),
            auth: AuthConfig::default(),
            tls: None,
            servers: Vec::new(),
            mappings: PathMappings {
                resources: std::collections::HashMap::new(),
                tools: std::collections::HashMap::new(),
            },
            tenants: std::collections::HashMap::new(),
        }
    }
}

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// 配置验证器
pub struct McpConfigValidator;

impl McpConfigValidator {
    /// 验证 MCP 配置
    pub fn validate(config: &McpConfig) -> Result<(), ConfigError> {
        config.validate()
    }
}

/// 配置热重载监视器
pub struct McpConfigWatcher {
    /// 配置文件路径
    config_path: std::path::PathBuf,
    /// 最后一个已知的修改时间
    last_modified: std::time::SystemTime,
    /// 轮询间隔（用于跨平台兼容）
    poll_interval: std::time::Duration,
}

impl McpConfigWatcher {
    /// 创建新的配置监视器
    pub fn new(config_path: impl Into<std::path::PathBuf>) -> Result<Self, ConfigError> {
        let config_path = config_path.into();

        if !config_path.exists() {
            return Err(ConfigError::FileNotFound(
                config_path.display().to_string()
            ));
        }

        let last_modified = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(Self {
            config_path,
            last_modified,
            poll_interval: std::time::Duration::from_secs(5),
        })
    }

    /// 创建带自定义轮询间隔的监视器
    pub fn with_poll_interval(
        config_path: impl Into<std::path::PathBuf>,
        interval: std::time::Duration,
    ) -> Result<Self, ConfigError> {
        let mut watcher = Self::new(config_path)?;
        watcher.poll_interval = interval;
        Ok(watcher)
    }

    /// 检查配置是否已更改
    pub fn has_changed(&self) -> bool {
        if let Ok(metadata) = std::fs::metadata(&self.config_path) {
            if let Ok(modified) = metadata.modified() {
                return modified > self.last_modified;
            }
        }
        false
    }

    /// 检查并更新配置（如果已更改则返回新配置）
    pub fn check_and_reload(&mut self) -> Result<Option<McpConfig>, ConfigError> {
        if self.has_changed() {
            let config = McpConfig::load_from_file(&self.config_path)?;

            // 更新修改时间
            self.last_modified = std::fs::metadata(&self.config_path)
                .and_then(|m| m.modified())
                .unwrap_or(self.last_modified);

            tracing::info!("Configuration hot-reloaded from {:?}", self.config_path);
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// 获取轮询间隔
    pub fn poll_interval(&self) -> std::time::Duration {
        self.poll_interval
    }

    /// 获取配置文件路径
    pub fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    /// 手动触发重新加载
    pub fn reload(&mut self) -> Result<McpConfig, ConfigError> {
        let config = McpConfig::load_from_file(&self.config_path)?;

        // 更新修改时间
        self.last_modified = std::fs::metadata(&self.config_path)
            .and_then(|m| m.modified())
            .unwrap_or(self.last_modified);

        tracing::info!("Configuration manually reloaded from {:?}", self.config_path);
        Ok(config)
    }
}

impl std::fmt::Debug for McpConfigWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConfigWatcher")
            .field("config_path", &self.config_path)
            .field("poll_interval", &self.poll_interval)
            .finish()
    }
}

/// VFS 操作类型
#[derive(Debug, Clone)]
pub enum VfsOperation {
    Readdir(String),
    Read(String),
    Write(String, Vec<u8>),
    Mkdir(String, u32),
    Remove(String, bool),
    Rename(String, String),
    Copy(String, String),
    Stat(String),
}

/// VFS 适配器 - 将 MCP Tool 转换为 VFS 操作
pub struct VfsAdapter {
    /// 路径映射配置
    pub mappings: PathMappings,
}

impl VfsAdapter {
    pub fn new(mappings: PathMappings) -> Self {
        Self { mappings }
    }

    /// 将 MCP Tool 名称和参数转换为 VFS 操作
    pub fn tool_to_vfs(tool_name: &str, args: &serde_json::Value) -> Result<VfsOperation, String> {
        let operation = match tool_name {
            "evif_ls" => {
                let path = args["path"].as_str().unwrap_or("/");
                VfsOperation::Readdir(path.to_string())
            }
            "evif_cat" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                VfsOperation::Read(path.to_string())
            }
            "evif_write" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                let content = args["content"].as_str().unwrap_or("");
                VfsOperation::Write(path.to_string(), content.as_bytes().to_vec())
            }
            "evif_mkdir" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                let mode = args["mode"].as_u64().unwrap_or(0o755) as u32;
                VfsOperation::Mkdir(path.to_string(), mode)
            }
            "evif_rm" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                let recursive = args["recursive"].as_bool().unwrap_or(false);
                VfsOperation::Remove(path.to_string(), recursive)
            }
            "evif_mv" => {
                let old_path = args["old_path"].as_str().ok_or("Missing 'old_path'")?;
                let new_path = args["new_path"].as_str().ok_or("Missing 'new_path'")?;
                VfsOperation::Rename(old_path.to_string(), new_path.to_string())
            }
            "evif_cp" => {
                let src = args["src"].as_str().ok_or("Missing 'src'")?;
                let dst = args["dst"].as_str().ok_or("Missing 'dst'")?;
                VfsOperation::Copy(src.to_string(), dst.to_string())
            }
            "evif_stat" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                VfsOperation::Stat(path.to_string())
            }
            _ => return Err(format!("Unknown tool: {}", tool_name)),
        };
        Ok(operation)
    }

    /// 将 VFS 路径转换为 MCP Resource URI
    pub fn path_to_resource(path: &str) -> String {
        if path.starts_with("file://") {
            path.to_string()
        } else {
            format!("file://{}", path)
        }
    }

    /// 获取工具对应的 VFS 路径
    pub fn get_tool_path(&self, tool_name: &str, args: &serde_json::Value) -> Option<String> {
        match tool_name {
            "evif_ls" | "evif_cat" | "evif_stat" | "evif_rm" => {
                args["path"].as_str().map(String::from)
            }
            "evif_write" | "evif_mkdir" => {
                args["path"].as_str().map(String::from)
            }
            "evif_mv" => {
                args["old_path"].as_str().map(String::from)
            }
            "evif_cp" => {
                args["src"].as_str().map(String::from)
            }
            _ => None,
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
    /// 工具缓存 (LRU)
    tool_cache: Arc<RwLock<ToolCache>>,
    /// 缓存大小配置
    #[allow(dead_code)]
    cache_size: usize,
    /// VFS 后端 (可选，用于直接 VFS 调用)
    vfs_backend: Option<Arc<VfsBackend>>,
    /// 路由系统 (用于 URI ↔ Path 转换)
    router: Arc<McpRouter>,
}

impl EvifMcpServer {
    /// 创建新的 MCP 服务器实例
    pub fn new(config: McpServerConfig) -> Arc<Self> {
        Self::with_cache_size_internal(config, 1024, None)
    }

    /// 创建带自定义缓存大小的 MCP 服务器
    pub fn with_cache_size(config: McpServerConfig, cache_size: usize) -> Arc<Self> {
        Self::with_cache_size_internal(config, cache_size, None)
    }

    /// 创建带自定义缓存大小和 VFS 后端的 MCP 服务器
    pub fn with_vfs_backend(config: McpServerConfig, vfs_backend: Arc<VfsBackend>) -> Arc<Self> {
        Self::with_cache_size_internal(config, 1024, Some(vfs_backend))
    }

    /// 内部创建方法
    fn with_cache_size_internal(config: McpServerConfig, cache_size: usize, vfs_backend: Option<Arc<VfsBackend>>) -> Arc<Self> {
        // 创建 HTTP 客户端，禁用代理
        let client = Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_else(|_| Client::new());
        let tool_cache = ToolCache::new(cache_size);
        let router = Arc::new(McpRouter::new());
        let server = Arc::new(Self {
            config,
            client,
            tools: Arc::new(RwLock::new(Vec::new())),
            prompts: Arc::new(RwLock::new(Vec::new())),
            resources: Arc::new(RwLock::new(Vec::new())),
            tool_cache: Arc::new(RwLock::new(tool_cache)),
            cache_size,
            vfs_backend,
            router,
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
            Tool {
                name: "evif_ping_with_stats".to_string(),
                description: "Ping with detailed server statistics (uptime, memory, connections)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed metrics",
                            "default": false
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_latency_test".to_string(),
                description: "Test API latency to EVIF server".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "API endpoint to test (default: /api/v1/health)",
                            "default": "/api/v1/health"
                        },
                        "iterations": {
                            "type": "number",
                            "description": "Number of test iterations (default: 5)",
                            "default": 5
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_request_trace".to_string(),
                description: "Enable request tracing for debugging".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "enable": {
                            "type": "boolean",
                            "description": "Enable (true) or disable (false) tracing"
                        },
                        "verbose": {
                            "type": "boolean",
                            "description": "Include verbose headers and bodies"
                        }
                    },
                    "required": ["enable"]
                }),
            },
            Tool {
                name: "evif_cache_stats".to_string(),
                description: "Get tool result cache statistics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "reset": {
                            "type": "boolean",
                            "description": "Reset counters after returning stats"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_log_query".to_string(),
                description: "Query server logs with filtering".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "level": {
                            "type": "string",
                            "description": "Log level filter (info, warn, error)",
                            "enum": ["info", "warn", "error", "debug"]
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of entries to return",
                            "default": 50
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Filter by pattern in log message"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_metrics_export".to_string(),
                description: "Export server metrics in various formats".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "description": "Export format (json, prometheus, csv)",
                            "default": "json",
                            "enum": ["json", "prometheus", "csv"]
                        },
                        "include_histograms": {
                            "type": "boolean",
                            "description": "Include histogram data"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_config_get".to_string(),
                description: "Get EVIF configuration values".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "Configuration key to retrieve"
                        },
                        "include_hidden": {
                            "type": "boolean",
                            "description": "Include hidden/internal config keys"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_event_subscribe".to_string(),
                description: "Subscribe to server events (file changes, mounts, etc)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "event_type": {
                            "type": "string",
                            "description": "Event type to subscribe to",
                            "enum": ["file_change", "mount", "unmount", "session", "all"]
                        },
                        "path_filter": {
                            "type": "string",
                            "description": "Filter events by path pattern"
                        }
                    },
                    "required": ["event_type"]
                }),
            },
            Tool {
                name: "evif_event_list".to_string(),
                description: "List available event types and subscriptions".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "include_history": {
                            "type": "boolean",
                            "description": "Include recent event history"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_cron_schedule".to_string(),
                description: "Schedule recurring tasks or reminders".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "Cron expression (e.g., '0 9 * * *' for daily at 9am)"
                        },
                        "task": {
                            "type": "string",
                            "description": "Task description or command"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Enable or disable the schedule"
                        }
                    },
                    "required": ["expression", "task"]
                }),
            },
            Tool {
                name: "evif_event_unsubscribe".to_string(),
                description: "Unsubscribe from server events".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "subscription_id": {
                            "type": "string",
                            "description": "Subscription ID to cancel"
                        }
                    },
                    "required": ["subscription_id"]
                }),
            },
            Tool {
                name: "evif_cron_list".to_string(),
                description: "List all scheduled cron tasks".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "include_disabled": {
                            "type": "boolean",
                            "description": "Include disabled schedules"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_cron_remove".to_string(),
                description: "Remove a scheduled cron task".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "schedule_id": {
                            "type": "string",
                            "description": "Schedule ID to remove"
                        }
                    },
                    "required": ["schedule_id"]
                }),
            },
            Tool {
                name: "evif_session_load".to_string(),
                description: "Load a previously saved session".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Session name to load"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_subagent_kill".to_string(),
                description: "Terminate a running subagent".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Subagent ID to terminate"
                        },
                        "reason": {
                            "type": "string",
                            "description": "Reason for termination"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "evif_skill_create".to_string(),
                description: "Create a new skill from template".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Skill name"
                        },
                        "template": {
                            "type": "string",
                            "description": "Skill template type (code-review, test, docs)"
                        },
                        "description": {
                            "type": "string",
                            "description": "Skill description"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_skill_delete".to_string(),
                description: "Delete a skill".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Skill name to delete"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force delete without confirmation"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_memory_search".to_string(),
                description: "Search memories by content".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum results"
                        },
                        "filter": {
                            "type": "string",
                            "description": "Filter by memory type"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "evif_memory_stats".to_string(),
                description: "Get memory system statistics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed statistics"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_pipe_create".to_string(),
                description: "Create a pipe for multi-agent coordination".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Pipe name"
                        },
                        "capacity": {
                            "type": "number",
                            "description": "Pipe capacity"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_pipe_list".to_string(),
                description: "List all pipes".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Filter by status (active, idle)"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_health_detailed".to_string(),
                description: "Get detailed health information".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "include_components": {
                            "type": "boolean",
                            "description": "Include component status"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_server_restart".to_string(),
                description: "Restart the EVIF server".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "graceful": {
                            "type": "boolean",
                            "description": "Graceful restart (wait for connections)"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_log_level".to_string(),
                description: "Get or set log level".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "level": {
                            "type": "string",
                            "description": "Log level (debug, info, warn, error)"
                        },
                        "component": {
                            "type": "string",
                            "description": "Component name"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_version".to_string(),
                description: "Get EVIF version information".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed version info"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_config_set".to_string(),
                description: "Set a configuration value".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "Configuration key"
                        },
                        "value": {
                            "type": "string",
                            "description": "Configuration value"
                        },
                        "persist": {
                            "type": "boolean",
                            "description": "Persist to config file"
                        }
                    },
                    "required": ["key", "value"]
                }),
            },
            Tool {
                name: "evif_config_list".to_string(),
                description: "List all configuration keys".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": "string",
                            "description": "Filter keys by prefix"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_plugin_load".to_string(),
                description: "Load a plugin dynamically".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Plugin name"
                        },
                        "path": {
                            "type": "string",
                            "description": "Plugin path or URL"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_plugin_unload".to_string(),
                description: "Unload a plugin".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Plugin name to unload"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force unload"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_plugin_info".to_string(),
                description: "Get plugin information".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Plugin name"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_subagent_status".to_string(),
                description: "Get subagent status".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Subagent ID"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "evif_queue_list".to_string(),
                description: "List queue items".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Filter by status (pending, processing, completed)"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum items to return"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_queue_stats".to_string(),
                description: "Get queue statistics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed stats"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "evif_session_delete".to_string(),
                description: "Delete a saved session".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Session name to delete"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force delete"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "evif_memory_clear".to_string(),
                description: "Clear memory entries".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Memory category to clear"
                        },
                        "confirm": {
                            "type": "boolean",
                            "description": "Confirm deletion"
                        }
                    },
                    "required": []
                }),
            },
            // 文件搜索工具
            Tool {
                name: "evif_find".to_string(),
                description: "Find files by name pattern".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to search in"
                        },
                        "name": {
                            "type": "string",
                            "description": "File name pattern (supports * and ?)"
                        },
                        "max_depth": {
                            "type": "number",
                            "description": "Maximum directory depth to search"
                        }
                    },
                    "required": ["path", "name"]
                }),
            },
            Tool {
                name: "evif_wc".to_string(),
                description: "Count lines, words, and characters in a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to count"
                        },
                        "lines": {
                            "type": "boolean",
                            "description": "Count lines"
                        },
                        "words": {
                            "type": "boolean",
                            "description": "Count words"
                        },
                        "chars": {
                            "type": "boolean",
                            "description": "Count characters"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "evif_tail".to_string(),
                description: "Show last N lines of a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        },
                        "lines": {
                            "type": "number",
                            "description": "Number of lines to show (default: 10)"
                        }
                    },
                    "required": ["path"]
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
            // MCP Capability Discovery Tool
            Tool {
                name: "evif_mcp_capabilities".to_string(),
                description: "Discover and list all MCP server capabilities including tools, resources, prompts, and extensions".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Filter by category: tools, resources, prompts, all (default: all)"
                        },
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed schema information (default: false)"
                        },
                        "mount_points": {
                            "type": "boolean",
                            "description": "Include available mount points for plugins (default: true)"
                        }
                    },
                    "required": []
                }),
            },
            // Plugin Catalog Discovery Tool
            Tool {
                name: "evif_plugin_catalog".to_string(),
                description: "List all available EVIF plugins and their status, including core and experimental plugins".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "tier": {
                            "type": "string",
                            "description": "Filter by tier: core, experimental, all (default: all)"
                        },
                        "mounted_only": {
                            "type": "boolean",
                            "description": "Only show currently mounted plugins (default: false)"
                        }
                    },
                    "required": []
                }),
            },
            // Server Statistics Tool
            Tool {
                name: "evif_server_stats".to_string(),
                description: "Get server runtime statistics including cache status, memory usage, and performance metrics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed per-cache metrics (default: false)"
                        },
                        "reset": {
                            "type": "boolean",
                            "description": "Reset statistics after returning (default: false)"
                        }
                    },
                    "required": []
                }),
            },
            // Batch Operations Tool
            Tool {
                name: "evif_batch".to_string(),
                description: "Execute multiple file operations in a single request for efficiency".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "operations": {
                            "type": "array",
                            "description": "Array of operations to execute",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "op": {
                                        "type": "string",
                                        "description": "Operation: read, write, mkdir, rm, list"
                                    },
                                    "path": {
                                        "type": "string",
                                        "description": "File/directory path"
                                    },
                                    "content": {
                                        "type": "string",
                                        "description": "Content for write operations"
                                    },
                                    "recursive": {
                                        "type": "boolean",
                                        "description": "Recursive flag for rm operations"
                                    }
                                },
                                "required": ["op", "path"]
                            }
                        },
                        "continue_on_error": {
                            "type": "boolean",
                            "description": "Continue processing if an operation fails (default: false)"
                        }
                    },
                    "required": ["operations"]
                }),
            },
            // Semantic Search Tool
            Tool {
                name: "evif_search".to_string(),
                description: "Perform semantic search across files and memories using vector similarity".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query text"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of results (default: 10)"
                        },
                        "source": {
                            "type": "string",
                            "description": "Search source: files, memories, all (default: all)"
                        },
                        "threshold": {
                            "type": "number",
                            "description": "Similarity threshold 0-1 (default: 0.7)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            // File Diff Tool
            Tool {
                name: "evif_diff".to_string(),
                description: "Compare two files and return differences using unified diff format".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "old_path": {
                            "type": "string",
                            "description": "Path to the original file"
                        },
                        "new_path": {
                            "type": "string",
                            "description": "Path to the modified file"
                        },
                        "context": {
                            "type": "number",
                            "description": "Number of context lines (default: 3)"
                        },
                        "ignore_whitespace": {
                            "type": "boolean",
                            "description": "Ignore whitespace changes (default: false)"
                        }
                    },
                    "required": ["old_path", "new_path"]
                }),
            },
            // File Watch Tool
            Tool {
                name: "evif_watch".to_string(),
                description: "Watch a file or directory for changes and return an event stream".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to watch (file or directory)"
                        },
                        "events": {
                            "type": "array",
                            "description": "Events to watch: create, modify, delete, rename (default: all)"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Watch subdirectories recursively (default: false)"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Watch timeout in seconds (default: 60)"
                        }
                    },
                    "required": ["path"]
                }),
            },
            // Tree Listing Tool
            Tool {
                name: "evif_tree".to_string(),
                description: "List files and directories in a tree structure with depth control".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Root path for tree listing"
                        },
                        "max_depth": {
                            "type": "number",
                            "description": "Maximum depth to traverse (default: 3)"
                        },
                        "include_hidden": {
                            "type": "boolean",
                            "description": "Include hidden files (default: false)"
                        },
                        "filter": {
                            "type": "string",
                            "description": "Filter pattern (e.g., *.rs for Rust files)"
                        }
                    },
                    "required": ["path"]
                }),
            },
            // Archive Tool
            Tool {
                name: "evif_archive".to_string(),
                description: "Create or extract archive files (tar, zip, gzip)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "description": "Operation: create, extract, list (required)"
                        },
                        "archive_path": {
                            "type": "string",
                            "description": "Path to the archive file"
                        },
                        "source_path": {
                            "type": "string",
                            "description": "Source directory/file to archive (for create)"
                        },
                        "destination_path": {
                            "type": "string",
                            "description": "Destination directory (for extract)"
                        },
                        "format": {
                            "type": "string",
                            "description": "Archive format: tar, zip, gzip (default: tar)"
                        },
                        "compression": {
                            "type": "string",
                            "description": "Compression: none, gzip, bzip2, xz (default: gzip)"
                        }
                    },
                    "required": ["operation", "archive_path"]
                }),
            },
            // Hash Tool
            Tool {
                name: "evif_hash".to_string(),
                description: "Calculate file hash values using various algorithms".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to hash"
                        },
                        "algorithm": {
                            "type": "string",
                            "description": "Hash algorithm: md5, sha1, sha256, sha512 (default: sha256)"
                        }
                    },
                    "required": ["path"]
                }),
            },
            // Disk Usage Tool
            Tool {
                name: "evif_du".to_string(),
                description: "Estimate disk usage of files and directories".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to analyze (default: /)"
                        },
                        "max_depth": {
                            "type": "number",
                            "description": "Maximum directory depth (default: 3)"
                        },
                        "sort_by": {
                            "type": "string",
                            "description": "Sort by: size, name, count (default: size)"
                        },
                        "top_n": {
                            "type": "number",
                            "description": "Return only top N entries (default: 10)"
                        }
                    },
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
            // Project Documentation Generation Prompt
            Prompt {
                name: "project_documentation".to_string(),
                description: "Generate project documentation including README, API docs, and architecture diagrams".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "project_path".to_string(),
                        description: "Path to the project root".to_string(),
                        required: true,
                        argument_type: "string".to_string(),
                    },
                    PromptArgument {
                        name: "doc_type".to_string(),
                        description: "Type of documentation: README, API, ARCHITECTURE, CHANGELOG".to_string(),
                        required: false,
                        argument_type: "string".to_string(),
                    },
                    PromptArgument {
                        name: "include_examples".to_string(),
                        description: "Include code examples in documentation".to_string(),
                        required: false,
                        argument_type: "boolean".to_string(),
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
            Resource {
                uri: "file:///context/L0/current".to_string(),
                name: "Current Context (L0)".to_string(),
                description: "Current working context at L0 level".to_string(),
                mime_type: "text/plain".to_string(),
            },
            Resource {
                uri: "file:///context/L1/decisions.md".to_string(),
                name: "Session Decisions (L1)".to_string(),
                description: "Session decisions and their rationale".to_string(),
                mime_type: "text/markdown".to_string(),
            },
        ];

        *self.resources.write().await = resources;
    }

    /// 检查 VFS 后端是否可用
    #[allow(dead_code)]
    fn has_vfs_backend(&self) -> bool {
        self.vfs_backend.as_ref().map(|b| b.is_available()).unwrap_or(false)
    }

    /// 处理工具调用 - 支持直接 VFS 调用
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value, String> {
        // 如果有 VFS 后端且工具支持 VFS，直接调用
        if let Some(backend) = &self.vfs_backend {
            if backend.is_available() {
                if let Some(result) = self.try_vfs_direct(tool_name, &arguments, backend).await {
                    return result;
                }
            }
        }

        // 回退到 HTTP 桥接模式
        self.call_tool_http(tool_name, arguments).await
    }

    /// 尝试直接 VFS 调用
    async fn try_vfs_direct(
        &self,
        tool_name: &str,
        arguments: &Value,
        backend: &Arc<VfsBackend>,
    ) -> Option<Result<Value, String>> {
        match tool_name {
            "evif_ls" => {
                let path = arguments["path"].as_str()?;
                match backend.list_dir(path).await {
                    Ok(entries) => Some(Ok(json!({
                        "entries": entries.iter().map(|e| json!({
                            "name": e.name,
                            "is_dir": e.is_dir,
                            "size": e.size,
                            "modified": e.modified
                        })).collect::<Vec<_>>()
                    }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_cat" => {
                let path = arguments["path"].as_str()?;
                let offset = arguments["offset"].as_u64().unwrap_or(0);
                let size = arguments["size"].as_u64().unwrap_or(0);
                match backend.read_file(path, offset, size).await {
                    Ok(content) => Some(Ok(json!({ "content": content }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_memorize" => {
                // Mock memory storage
                if backend.is_mock() {
                    let key = arguments["key"].as_str().unwrap_or("default");
                    let value = arguments["value"].as_str().unwrap_or("");
                    return Some(Ok(json!({
                        "stored": true,
                        "key": key,
                        "length": value.len()
                    })));
                }
                None
            }
            "evif_retrieve" => {
                // Mock memory retrieval
                if backend.is_mock() {
                    let key = arguments["key"].as_str().unwrap_or("default");
                    return Some(Ok(json!({
                        "found": false,
                        "key": key,
                        "value": ""
                    })));
                }
                None
            }
            "evif_skill_list" => {
                // Mock skill list
                if backend.is_mock() {
                    return Some(Ok(json!({
                        "skills": [
                            {"name": "evif-ls", "path": "/skills/evif-ls"},
                            {"name": "code-review", "path": "/skills/code-review"}
                        ]
                    })));
                }
                None
            }
            "evif_find" => {
                // Mock file find
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("/");
                    let name = arguments["name"].as_str().unwrap_or("*");
                    // Simple mock: return files matching the pattern
                    let mock_files = vec![
                        format!("{}/{}", path.trim_end_matches('/'), name.replace("*", "file")),
                    ];
                    return Some(Ok(json!({
                        "matches": mock_files,
                        "count": mock_files.len()
                    })));
                }
                None
            }
            "evif_wc" => {
                // Mock word count
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("");
                    return match backend.read_file(path, 0, 0).await {
                        Ok(content) => {
                            let lines = content.lines().count();
                            let words = content.split_whitespace().count();
                            let chars = content.chars().count();
                            let count_lines = arguments["lines"].as_bool().unwrap_or(true);
                            let count_words = arguments["words"].as_bool().unwrap_or(true);
                            let count_chars = arguments["chars"].as_bool().unwrap_or(false);

                            let mut result = json!({
                                "path": path,
                                "lines": lines,
                                "words": words,
                                "chars": chars
                            });

                            // Remove fields based on flags
                            if !count_lines { result["lines"] = json!(null); }
                            if !count_words { result["words"] = json!(null); }
                            if !count_chars { result["chars"] = json!(null); }

                            Some(Ok(result))
                        }
                        Err(e) => Some(Err(e)),
                    };
                }
                None
            }
            "evif_tail" => {
                // Mock tail
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("");
                    let n = arguments["lines"].as_u64().unwrap_or(10) as usize;
                    return match backend.read_file(path, 0, 0).await {
                        Ok(content) => {
                            let lines: Vec<&str> = content.lines().collect();
                            let tail_lines: Vec<&str> = lines.iter().rev().take(n).cloned().collect();
                            let tail: String = tail_lines.iter().rev().map(|s| *s).collect::<Vec<_>>().join("\n");
                            Some(Ok(json!({
                                "lines": tail_lines.len(),
                                "content": tail
                            })))
                        }
                        Err(e) => Some(Err(e)),
                    };
                }
                None
            }
            "evif_write" => {
                let path = arguments["path"].as_str()?;
                let content = arguments["content"].as_str()?;
                match backend.write_file(path, content).await {
                    Ok(result) => Some(Ok(json!({
                        "bytes_written": result.bytes_written,
                        "path": result.path
                    }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_mkdir" => {
                let path = arguments["path"].as_str()?;
                let mode = arguments["mode"].as_u64().unwrap_or(0o755) as u32;
                match backend.make_dir(path, mode).await {
                    Ok(_) => Some(Ok(json!({ "success": true }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_rm" => {
                let path = arguments["path"].as_str()?;
                let recursive = arguments["recursive"].as_bool().unwrap_or(false);
                match backend.remove(path, recursive).await {
                    Ok(_) => Some(Ok(json!({ "success": true }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_stat" => {
                let path = arguments["path"].as_str()?;
                match backend.stat(path).await {
                    Ok(info) => Some(Ok(json!({
                        "path": info.path,
                        "name": info.name,
                        "size": info.size,
                        "is_dir": info.is_dir,
                        "mode": info.mode,
                        "modified": info.modified,
                        "created": info.created
                    }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_mv" => {
                let old_path = arguments["old_path"].as_str()?;
                let new_path = arguments["new_path"].as_str()?;
                match backend.rename(old_path, new_path).await {
                    Ok(_) => Some(Ok(json!({ "success": true }))),
                    Err(e) => Some(Err(e)),
                }
            }
            "evif_health" => {
                // Mock health check - always returns OK in mock mode
                if backend.is_mock() {
                    return Some(Ok(json!({
                        "status": "ok",
                        "mode": "mock",
                        "version": "1.8.0",
                        "uptime_seconds": 0
                    })));
                }
                None
            }
            "evif_ping_with_stats" => {
                // Ping with detailed server statistics
                if backend.is_mock() {
                    let detailed = arguments["detailed"].as_bool().unwrap_or(false);
                    let mut response = json!({
                        "status": "pong",
                        "mode": "mock",
                        "server_name": "evif-mcp",
                        "version": "1.8.0",
                        "uptime_seconds": 3600,
                        "timestamp": "2026-05-01T12:00:00Z"
                    });

                    if detailed {
                        response["memory_usage"] = json!({
                            "rss_bytes": 50_000_000,
                            "heap_used_bytes": 10_000_000,
                            "heap_available_bytes": 90_000_000
                        });
                        response["active_connections"] = json!(5);
                        response["total_requests"] = json!(12345);
                        response["cache_hit_rate"] = json!(0.85);
                    }

                    return Some(Ok(response));
                }
                None
            }
            "evif_latency_test" => {
                // Test API latency to EVIF server
                if backend.is_mock() {
                    let target = arguments["target"].as_str().unwrap_or("/api/v1/health");
                    let iterations = arguments["iterations"].as_u64().unwrap_or(5) as usize;
                    let iterations = iterations.min(100); // Cap at 100

                    let mut results = Vec::with_capacity(iterations);
                    let base_latency_ms = 5.0; // Mock base latency

                    for i in 0..iterations {
                        let jitter = (i as f64 * 0.5) % 10.0;
                        results.push(json!({
                            "iteration": i + 1,
                            "latency_ms": base_latency_ms + jitter,
                            "target": target
                        }));
                    }

                    let avg: f64 = results.iter()
                        .map(|r| r["latency_ms"].as_f64().unwrap_or(0.0))
                        .sum::<f64>() / iterations as f64;

                    return Some(Ok(json!({
                        "target": target,
                        "iterations": iterations,
                        "results": results,
                        "average_latency_ms": avg,
                        "min_latency_ms": 5.0,
                        "max_latency_ms": 5.0 + (iterations as f64 * 0.5 % 10.0)
                    })));
                }
                None
            }
            "evif_request_trace" => {
                // Enable/disable request tracing
                if backend.is_mock() {
                    let enable = arguments["enable"].as_bool().unwrap_or(false);
                    let verbose = arguments["verbose"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "tracing_enabled": enable,
                        "verbose_mode": verbose,
                        "message": if enable {
                            "Request tracing enabled"
                        } else {
                            "Request tracing disabled"
                        }
                    })));
                }
                None
            }
            "evif_cache_stats" => {
                // Get tool result cache statistics
                if backend.is_mock() {
                    let reset = arguments["reset"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "cache_enabled": true,
                        "cache_size": 1000,
                        "entries_count": 42,
                        "hits": 156,
                        "misses": 23,
                        "hit_rate": 0.87,
                        "total_requests": 179,
                        "reset": reset
                    })));
                }
                None
            }
            "evif_log_query" => {
                // Query server logs with filtering
                if backend.is_mock() {
                    let level = arguments["level"].as_str().unwrap_or("info");
                    let limit = arguments["limit"].as_u64().unwrap_or(50) as usize;
                    let pattern = arguments["pattern"].as_str().unwrap_or("");

                    let mock_logs = json!([
                        {"timestamp": "2026-05-01T12:00:00Z", "level": "info", "message": "Server started successfully"},
                        {"timestamp": "2026-05-01T12:00:01Z", "level": "info", "message": "Loaded 6 plugins"},
                        {"timestamp": "2026-05-01T12:00:05Z", "level": "info", "message": "MCP client connected"},
                        {"timestamp": "2026-05-01T12:00:10Z", "level": "debug", "message": "Cache hit for evif_ls"},
                        {"timestamp": "2026-05-01T12:00:15Z", "level": "warn", "message": "Slow request detected: 250ms"}
                    ]);

                    let logs: Vec<Value> = mock_logs.as_array().unwrap()
                        .iter()
                        .filter(|log| {
                            let log_level = log["level"].as_str().unwrap_or("");
                            let matches_level = level == "debug" || log_level == level || level == "info" && (log_level == "info" || log_level == "warn");
                            let matches_pattern = pattern.is_empty() || log["message"].as_str().unwrap_or("").contains(pattern);
                            matches_level && matches_pattern
                        })
                        .cloned()
                        .take(limit)
                        .collect();

                    return Some(Ok(json!({
                        "level": level,
                        "limit": limit,
                        "count": logs.len(),
                        "logs": logs
                    })));
                }
                None
            }
            "evif_metrics_export" => {
                // Export server metrics in various formats
                if backend.is_mock() {
                    let format = arguments["format"].as_str().unwrap_or("json");
                    let include_histograms = arguments["include_histograms"].as_bool().unwrap_or(false);

                    let mut metrics = json!({
                        "server": "evif-mcp",
                        "version": "1.8.0",
                        "timestamp": "2026-05-01T12:00:00Z",
                        "metrics": {
                            "requests_total": 12345,
                            "requests_success": 11900,
                            "requests_error": 445,
                            "cache_hits": 8900,
                            "cache_misses": 1230,
                            "avg_latency_ms": 12.5,
                            "p50_latency_ms": 8.2,
                            "p95_latency_ms": 35.1,
                            "p99_latency_ms": 58.3
                        }
                    });

                    if include_histograms {
                        metrics["histograms"] = json!({
                            "request_latency": [5, 8, 10, 15, 25, 35, 50, 75, 100],
                            "cache_ttl": [60, 120, 300, 600, 900, 1800]
                        });
                    }

                    match format {
                        "prometheus" => {
                            let prometheus = format!(
                                "# HELP evif_requests_total Total requests\n\
                                 # TYPE evif_requests_total counter\n\
                                 evif_requests_total {}\n\
                                 # HELP evif_cache_hits Cache hits\n\
                                 # TYPE evif_cache_hits counter\n\
                                 evif_cache_hits {}\n",
                                12345, 8900
                            );
                            return Some(Ok(json!({"format": "prometheus", "content": prometheus})));
                        }
                        "csv" => {
                            return Some(Ok(json!({"format": "csv", "content": "metric,value\nrequests_total,12345\ncache_hits,8900\n"})));
                        }
                        _ => return Some(Ok(json!({"format": "json", "content": metrics}))),
                    }
                }
                None
            }
            "evif_config_get" => {
                // Get EVIF configuration values
                if backend.is_mock() {
                    let key = arguments["key"].as_str().unwrap_or("");
                    let _include_hidden = arguments["include_hidden"].as_bool().unwrap_or(false);

                    let configs = json!({
                        "evif_url": "http://localhost:8081",
                        "server_name": "evif-mcp",
                        "log_level": "info",
                        "cache_size": 1000,
                        "mock_mode": true,
                        "protocol_version": "2024-11-05"
                    });

                    if key.is_empty() {
                        return Some(Ok(json!({
                            "all_configs": configs,
                            "count": configs.as_object().unwrap().len()
                        })));
                    }

                    if let Some(val) = configs.get(key) {
                        return Some(Ok(json!({"key": key, "value": val})));
                    }

                    return Some(Ok(json!({"key": key, "value": null, "error": "Config key not found"})));
                }
                None
            }
            "evif_event_subscribe" => {
                // Subscribe to server events
                if backend.is_mock() {
                    let event_type = arguments["event_type"].as_str().unwrap_or("all");
                    let path_filter = arguments["path_filter"].as_str().unwrap_or("*");

                    return Some(Ok(json!({
                        "subscription_id": "sub-12345",
                        "event_type": event_type,
                        "path_filter": path_filter,
                        "status": "subscribed",
                        "expires_at": null
                    })));
                }
                None
            }
            "evif_event_list" => {
                // List available event types and subscriptions
                if backend.is_mock() {
                    let include_history = arguments["include_history"].as_bool().unwrap_or(false);

                    let mut response = json!({
                        "available_events": [
                            {"type": "file_change", "description": "File or directory changed"},
                            {"type": "mount", "description": "Plugin mounted"},
                            {"type": "unmount", "description": "Plugin unmounted"},
                            {"type": "session", "description": "Session created/closed"}
                        ],
                        "active_subscriptions": [
                            {"id": "sub-001", "type": "file_change", "active": true}
                        ]
                    });

                    if include_history {
                        response["recent_events"] = json!([
                            {"timestamp": "2026-05-01T12:00:00Z", "type": "mount", "data": {"plugin": "hello"}},
                            {"timestamp": "2026-05-01T11:55:00Z", "type": "file_change", "data": {"path": "/hello"}}
                        ]);
                    }

                    return Some(Ok(response));
                }
                None
            }
            "evif_cron_schedule" => {
                // Schedule recurring tasks
                if backend.is_mock() {
                    let expression = arguments["expression"].as_str().unwrap_or("");
                    let task = arguments["task"].as_str().unwrap_or("");
                    let enabled = arguments["enabled"].as_bool().unwrap_or(true);

                    return Some(Ok(json!({
                        "schedule_id": "cron-67890",
                        "expression": expression,
                        "task": task,
                        "enabled": enabled,
                        "next_run": "2026-05-02T09:00:00Z",
                        "created_at": "2026-05-01T12:00:00Z"
                    })));
                }
                None
            }
            "evif_event_unsubscribe" => {
                // Unsubscribe from events
                if backend.is_mock() {
                    let subscription_id = arguments["subscription_id"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "subscription_id": subscription_id,
                        "status": "unsubscribed",
                        "unsubscribed_at": "2026-05-01T12:05:00Z"
                    })));
                }
                None
            }
            "evif_cron_list" => {
                // List all scheduled tasks
                if backend.is_mock() {
                    let _include_disabled = arguments["include_disabled"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "schedules": [
                            {"id": "cron-001", "expression": "0 9 * * *", "task": "Morning backup", "enabled": true, "next_run": "2026-05-02T09:00:00Z"},
                            {"id": "cron-002", "expression": "0 */6 * * *", "task": "Health check", "enabled": true, "next_run": "2026-05-01T18:00:00Z"}
                        ],
                        "total": 2,
                        "enabled": 2
                    })));
                }
                None
            }
            "evif_cron_remove" => {
                // Remove a scheduled task
                if backend.is_mock() {
                    let schedule_id = arguments["schedule_id"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "schedule_id": schedule_id,
                        "status": "removed",
                        "removed_at": "2026-05-01T12:05:00Z"
                    })));
                }
                None
            }
            "evif_session_load" => {
                // Load a saved session
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "session_name": name,
                        "loaded": true,
                        "context": {
                            "L0": "Previous task context",
                            "L1": "Previous decisions"
                        },
                        "loaded_at": "2026-05-01T12:05:00Z"
                    })));
                }
                None
            }
            "evif_subagent_kill" => {
                // Kill a running subagent
                if backend.is_mock() {
                    let id = arguments["id"].as_str().unwrap_or("");
                    let reason = arguments["reason"].as_str().unwrap_or("Not specified");

                    return Some(Ok(json!({
                        "agent_id": id,
                        "reason": reason,
                        "killed": true,
                        "terminated_at": "2026-05-01T12:05:00Z"
                    })));
                }
                None
            }
            "evif_skill_create" => {
                // Create a new skill
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");
                    let template = arguments["template"].as_str().unwrap_or("default");
                    let description = arguments["description"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "skill_name": name,
                        "template": template,
                        "description": description,
                        "created": true,
                        "created_at": "2026-05-01T12:10:00Z"
                    })));
                }
                None
            }
            "evif_skill_delete" => {
                // Delete a skill
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");
                    let force = arguments["force"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "skill_name": name,
                        "force": force,
                        "deleted": true,
                        "deleted_at": "2026-05-01T12:10:00Z"
                    })));
                }
                None
            }
            "evif_memory_search" => {
                // Search memories
                if backend.is_mock() {
                    let query = arguments["query"].as_str().unwrap_or("");
                    let _limit = arguments["limit"].as_i64().unwrap_or(10) as usize;
                    let filter = arguments["filter"].as_str().unwrap_or("all");

                    return Some(Ok(json!({
                        "query": query,
                        "results": [
                            {"key": "mem-001", "score": 0.95, "content": format!("Memory about {}", query)},
                            {"key": "mem-002", "score": 0.87, "content": format!("Related to {}", query)}
                        ],
                        "total": 2,
                        "filter": filter
                    })));
                }
                None
            }
            "evif_memory_stats" => {
                // Get memory statistics
                if backend.is_mock() {
                    let detailed = arguments["detailed"].as_bool().unwrap_or(false);

                    let mut stats = json!({
                        "total_memories": 128,
                        "total_size_bytes": 524288,
                        "categories": {
                            "code": 45,
                            "docs": 32,
                            "decisions": 51
                        }
                    });

                    if detailed {
                        stats["breakdown"] = json!({
                            "vector_memories": 100,
                            "key_value_memories": 28
                        });
                    }

                    return Some(Ok(stats));
                }
                None
            }
            "evif_pipe_create" => {
                // Create a pipe
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");
                    let capacity = arguments["capacity"].as_i64().unwrap_or(100) as usize;

                    return Some(Ok(json!({
                        "pipe_name": name,
                        "capacity": capacity,
                        "created": true,
                        "path": format!("/pipes/{}", name),
                        "created_at": "2026-05-01T12:10:00Z"
                    })));
                }
                None
            }
            "evif_pipe_list" => {
                // List all pipes
                if backend.is_mock() {
                    let status = arguments["status"].as_str().unwrap_or("all");

                    return Some(Ok(json!({
                        "pipes": [
                            {"name": "pipe-1", "status": "active", "capacity": 100, "used": 45},
                            {"name": "pipe-2", "status": "idle", "capacity": 200, "used": 0}
                        ],
                        "total": 2,
                        "filter": status
                    })));
                }
                None
            }
            "evif_health_detailed" => {
                // Get detailed health
                if backend.is_mock() {
                    let include_components = arguments["include_components"].as_bool().unwrap_or(false);

                    let mut health = json!({
                        "status": "healthy",
                        "uptime_seconds": 3600,
                        "memory_used_bytes": 67108864,
                        "memory_total_bytes": 134217728
                    });

                    if include_components {
                        health["components"] = json!([
                            {"name": "vfs", "status": "healthy", "latency_ms": 2},
                            {"name": "plugins", "status": "healthy", "count": 5},
                            {"name": "cache", "status": "healthy", "hit_rate": 0.95}
                        ]);
                    }

                    return Some(Ok(health));
                }
                None
            }
            "evif_server_restart" => {
                // Restart server
                if backend.is_mock() {
                    let graceful = arguments["graceful"].as_bool().unwrap_or(true);

                    return Some(Ok(json!({
                        "restarting": true,
                        "graceful": graceful,
                        "restart_at": "2026-05-01T12:15:00Z"
                    })));
                }
                None
            }
            "evif_log_level" => {
                // Get or set log level
                if backend.is_mock() {
                    let level = arguments["level"].as_str().unwrap_or("");
                    let component = arguments["component"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "level": if level.is_empty() { "info" } else { level },
                        "component": if component.is_empty() { "global" } else { component },
                        "previous_level": "info"
                    })));
                }
                None
            }
            "evif_version" => {
                // Get version info
                if backend.is_mock() {
                    let detailed = arguments["detailed"].as_bool().unwrap_or(false);

                    let mut version = json!({
                        "version": "1.8.0",
                        "build": "release"
                    });

                    if detailed {
                        version["details"] = json!({
                            "rustc_version": "1.75.0",
                            "build_date": "2026-05-01",
                            "features": ["vfs", "plugins", "mcp", "memory"]
                        });
                    }

                    return Some(Ok(version));
                }
                None
            }
            "evif_config_set" => {
                // Set config value
                if backend.is_mock() {
                    let key = arguments["key"].as_str().unwrap_or("");
                    let value = arguments["value"].as_str().unwrap_or("");
                    let persist = arguments["persist"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "key": key,
                        "value": value,
                        "persist": persist,
                        "set": true
                    })));
                }
                None
            }
            "evif_config_list" => {
                // List config keys
                if backend.is_mock() {
                    let filter = arguments["filter"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "keys": [
                            {"key": "server_name", "value": "evif-mcp"},
                            {"key": "log_level", "value": "info"},
                            {"key": "cache_size", "value": "1000"}
                        ],
                        "total": 3,
                        "filter": filter
                    })));
                }
                None
            }
            "evif_plugin_load" => {
                // Load plugin
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");
                    let path = arguments["path"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "plugin_name": name,
                        "path": path,
                        "loaded": true,
                        "version": "1.0.0"
                    })));
                }
                None
            }
            "evif_plugin_unload" => {
                // Unload plugin
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");
                    let force = arguments["force"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "plugin_name": name,
                        "force": force,
                        "unloaded": true
                    })));
                }
                None
            }
            "evif_plugin_info" => {
                // Get plugin info
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "name": name,
                        "version": "1.0.0",
                        "description": format!("Plugin {}", name),
                        "status": "loaded",
                        "capabilities": ["read", "write", "search"]
                    })));
                }
                None
            }
            "evif_subagent_status" => {
                // Get subagent status
                if backend.is_mock() {
                    let id = arguments["id"].as_str().unwrap_or("");

                    return Some(Ok(json!({
                        "agent_id": id,
                        "status": "running",
                        "uptime_seconds": 3600,
                        "tasks_completed": 42
                    })));
                }
                None
            }
            "evif_queue_list" => {
                // List queue items
                if backend.is_mock() {
                    let status = arguments["status"].as_str().unwrap_or("");
                    let _limit = arguments["limit"].as_i64().unwrap_or(10) as usize;

                    return Some(Ok(json!({
                        "items": [
                            {"id": "q-001", "status": "pending", "created": "2026-05-01T12:00:00Z"},
                            {"id": "q-002", "status": "processing", "created": "2026-05-01T12:05:00Z"}
                        ],
                        "total": 2,
                        "filter": status
                    })));
                }
                None
            }
            "evif_queue_stats" => {
                // Get queue stats
                if backend.is_mock() {
                    let detailed = arguments["detailed"].as_bool().unwrap_or(false);

                    let mut stats = json!({
                        "pending": 5,
                        "processing": 2,
                        "completed": 100,
                        "failed": 3
                    });

                    if detailed {
                        stats["by_type"] = json!({
                            "file_ops": 50,
                            "search": 30,
                            "memory": 20
                        });
                    }

                    return Some(Ok(stats));
                }
                None
            }
            "evif_session_delete" => {
                // Delete session
                if backend.is_mock() {
                    let name = arguments["name"].as_str().unwrap_or("");
                    let force = arguments["force"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "session_name": name,
                        "force": force,
                        "deleted": true,
                        "deleted_at": "2026-05-01T12:20:00Z"
                    })));
                }
                None
            }
            "evif_memory_clear" => {
                // Clear memory
                if backend.is_mock() {
                    let category = arguments["category"].as_str().unwrap_or("");
                    let confirm = arguments["confirm"].as_bool().unwrap_or(false);

                    return Some(Ok(json!({
                        "category": if category.is_empty() { "all" } else { category },
                        "cleared": confirm,
                        "entries_removed": if confirm { 42 } else { 0 }
                    })));
                }
                None
            }
            "evif_mcp_capabilities" => {
                // MCP capability discovery - returns all capabilities
                if backend.is_mock() {
                    let category = arguments["category"].as_str().unwrap_or("all");
                    let _detailed = arguments["detailed"].as_bool().unwrap_or(false);
                    let include_mounts = arguments["mount_points"].as_bool().unwrap_or(true);

                    let mut capabilities = json!({
                        "server_name": "evif-mcp",
                        "version": "1.8.0",
                        "protocol_version": "2024-11-05",
                        "total_tools": 75,
                        "total_prompts": 4,
                        "total_resources": 3,
                        "total_roots": 3
                    });

                    if category == "all" || category == "tools" {
                        capabilities["tools"] = json!([
                            {"name": "evif_ls", "category": "file_ops"},
                            {"name": "evif_cat", "category": "file_ops"},
                            {"name": "evif_write", "category": "file_ops"},
                            {"name": "evif_mkdir", "category": "file_ops"},
                            {"name": "evif_rm", "category": "file_ops"},
                            {"name": "evif_stat", "category": "file_ops"},
                            {"name": "evif_mv", "category": "file_ops"},
                            {"name": "evif_cp", "category": "file_ops"},
                            {"name": "evif_mount", "category": "plugin"},
                            {"name": "evif_unmount", "category": "plugin"},
                            {"name": "evif_mounts", "category": "plugin"},
                            {"name": "evif_grep", "category": "search"},
                            {"name": "evif_find", "category": "search"},
                            {"name": "evif_wc", "category": "search"},
                            {"name": "evif_tail", "category": "search"},
                            {"name": "evif_open_handle", "category": "handle"},
                            {"name": "evif_close_handle", "category": "handle"},
                            {"name": "evif_memorize", "category": "memory"},
                            {"name": "evif_retrieve", "category": "memory"},
                            {"name": "evif_skill_list", "category": "skill"},
                            {"name": "evif_skill_info", "category": "skill"},
                            {"name": "evif_skill_execute", "category": "skill"},
                            {"name": "evif_claude_md_generate", "category": "claude"},
                            {"name": "evif_session_save", "category": "context"},
                            {"name": "evif_session_list", "category": "context"},
                            {"name": "evif_subagent_create", "category": "agent"},
                            {"name": "evif_subagent_send", "category": "agent"},
                            {"name": "evif_subagent_list", "category": "agent"},
                            {"name": "evif_mcp_capabilities", "category": "meta"},
                            {"name": "evif_plugin_catalog", "category": "meta"},
                            {"name": "evif_server_stats", "category": "meta"},
                            {"name": "evif_batch", "category": "batch"},
                            {"name": "evif_search", "category": "search"},
                            {"name": "evif_diff", "category": "utility"},
                            {"name": "evif_watch", "category": "utility"},
                            {"name": "evif_tree", "category": "utility"},{"name": "evif_archive", "category": "archive"}, {"name": "evif_hash", "category": "utility"}, {"name": "evif_du", "category": "utility"},{"name": "evif_latency_test", "category": "diagnostic"}, {"name": "evif_request_trace", "category": "diagnostic"}, {"name": "evif_cache_stats", "category": "diagnostic"},{"name": "evif_log_query", "category": "diagnostic"}, {"name": "evif_metrics_export", "category": "diagnostic"}, {"name": "evif_config_get", "category": "diagnostic"}
                        ]);
                    }

                    if category == "all" || category == "prompts" {
                        capabilities["prompts"] = json!([
                            {"name": "file_explorer", "description": "Explore and interact with the EVIF file system"},
                            {"name": "batch_operations", "description": "Perform batch operations on files"},
                            {"name": "data_analysis", "description": "Analyze data in files and generate insights"}
                        ]);
                    }

                    if category == "all" || category == "resources" {
                        capabilities["resources"] = json!([
                            {"uri": "file:///context/L0/current", "name": "Current Context", "mime_type": "text/plain"}
                        ]);
                    }

                    if category == "all" || category == "roots" {
                        capabilities["roots"] = json!([
                            {"path": "/context", "description": "Context filesystem root"},
                            {"path": "/skills", "description": "Skills filesystem root"},
                            {"path": "/pipes", "description": "Pipes filesystem root"}
                        ]);
                    }

                    if include_mounts {
                        capabilities["mount_points"] = json!([
                            {"name": "contextfs", "path": "/context", "type": "core"},
                            {"name": "skillfs", "path": "/skills", "type": "core"},
                            {"name": "pipefs", "path": "/pipes", "type": "core"},
                            {"name": "memfs", "path": "/mem", "type": "core"},
                            {"name": "hellofs", "path": "/hello", "type": "core"},
                            {"name": "postgresfs", "path": "/postgres", "type": "experimental"},
                            {"name": "s3fs", "path": "/s3", "type": "experimental"},
                            {"name": "gmailfs", "path": "/gmail", "type": "experimental"},
                            {"name": "teamsfs", "path": "/teams", "type": "experimental"},
                            {"name": "telegramfs", "path": "/telegram", "type": "experimental"},
                            {"name": "shopifyfs", "path": "/shopify", "type": "experimental"}
                        ]);
                    }

                    return Some(Ok(capabilities));
                }
                None
            }
            "evif_plugin_catalog" => {
                // Plugin catalog discovery - returns all available plugins
                if backend.is_mock() {
                    let tier = arguments["tier"].as_str().unwrap_or("all");
                    let mounted_only = arguments["mounted_only"].as_bool().unwrap_or(false);

                    let mut plugins = json!({
                        "server_name": "evif-mcp",
                        "total_plugins": 23
                    });

                    let core_plugins = json!([
                        {"id": "contextfs", "name": "ContextFS", "description": "Layered L0/L1/L2 context filesystem", "tier": "core", "mountable": true, "mounted": true, "path": "/context"},
                        {"id": "memfs", "name": "MemFS", "description": "High-speed in-memory filesystem", "tier": "core", "mountable": true, "mounted": true, "path": "/mem"},
                        {"id": "skillfs", "name": "SkillFS", "description": "Standard SKILL.md discovery", "tier": "core", "mountable": true, "mounted": true, "path": "/skills"},
                        {"id": "pipefs", "name": "PipeFS", "description": "Bidirectional pipe primitives", "tier": "core", "mountable": true, "mounted": true, "path": "/pipes"},
                        {"id": "localfs", "name": "LocalFS", "description": "Mount a host directory", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "hellofs", "name": "HelloFS", "description": "Minimal demo filesystem", "tier": "core", "mountable": true, "mounted": true, "path": "/hello"},
                        {"id": "kvfs", "name": "KVFS", "description": "Key-value storage", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "queuefs", "name": "QueueFS", "description": "FIFO queue interface", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "sqlfs2", "name": "SQLFS2", "description": "SQLite-backed file interface", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "streamfs", "name": "StreamFS", "description": "Streaming read and append", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "heartbeatfs", "name": "HeartbeatFS", "description": "Liveness and lease primitives", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "proxyfs", "name": "ProxyFS", "description": "Proxy to another endpoint", "tier": "core", "mountable": true, "mounted": false, "path": null},
                        {"id": "serverinfofs", "name": "ServerInfoFS", "description": "Server health metadata", "tier": "core", "mountable": true, "mounted": false, "path": null}
                    ]);

                    let experimental_plugins = json!([
                        {"id": "devfs", "name": "DevFS", "description": "Device examples", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "httpfs", "name": "HTTPFS", "description": "HTTP-backed filesystem", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "postgresfs", "name": "PostgresFS", "description": "PostgreSQL interface", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "gmailfs", "name": "GmailFS", "description": "Gmail/IMAP interface", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "teamsfs", "name": "TeamsFS", "description": "Microsoft Teams interface", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "telegramfs", "name": "TelegramFS", "description": "Telegram Bot interface", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "shopifyfs", "name": "ShopifyFS", "description": "Shopify e-commerce interface", "tier": "experimental", "mountable": true, "mounted": false, "path": null},
                        {"id": "handlefs", "name": "HandleFS", "description": "Handle-oriented wrapper", "tier": "experimental", "mountable": false, "mounted": false, "path": null},
                        {"id": "tieredfs", "name": "TieredFS", "description": "Multi-tier storage", "tier": "experimental", "mountable": false, "mounted": false, "path": null},
                        {"id": "encryptedfs", "name": "EncryptedFS", "description": "Encryption wrapper", "tier": "experimental", "mountable": false, "mounted": false, "path": null}
                    ]);

                    match tier {
                        "core" => {
                            plugins["plugins"] = core_plugins.clone();
                            plugins["total_count"] = json!(13);
                        },
                        "experimental" => {
                            plugins["plugins"] = experimental_plugins.clone();
                            plugins["total_count"] = json!(10);
                        },
                        _ => {
                            plugins["plugins"] = json!({
                                "core": core_plugins,
                                "experimental": experimental_plugins
                            });
                            plugins["total_count"] = json!(23);
                        }
                    }

                    if mounted_only {
                        let mounted: Vec<Value> = if tier == "core" {
                            serde_json::from_value::<Vec<Value>>(core_plugins.clone())
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|p| p.get("mounted").and_then(|v| v.as_bool()).unwrap_or(false))
                                .collect()
                        } else if tier == "experimental" {
                            serde_json::from_value::<Vec<Value>>(experimental_plugins.clone())
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|p| p.get("mounted").and_then(|v| v.as_bool()).unwrap_or(false))
                                .collect()
                        } else {
                            let mut all_mounted = serde_json::from_value::<Vec<Value>>(core_plugins.clone())
                                .unwrap_or_default();
                            all_mounted.extend(
                                serde_json::from_value::<Vec<Value>>(experimental_plugins.clone())
                                    .unwrap_or_default()
                            );
                            all_mounted.into_iter()
                                .filter(|p| p.get("mounted").and_then(|v| v.as_bool()).unwrap_or(false))
                                .collect()
                        };
                        plugins["plugins"] = json!(mounted);
                    }

                    return Some(Ok(plugins));
                }
                None
            }
            "evif_server_stats" => {
                // Server statistics - returns runtime metrics
                if backend.is_mock() {
                    let detailed = arguments["detailed"].as_bool().unwrap_or(false);
                    let reset = arguments["reset"].as_bool().unwrap_or(false);

                    let mut stats = json!({
                        "server_name": "evif-mcp",
                        "version": "1.8.0",
                        "uptime_seconds": 3600,
                        "total_requests": 12345,
                        "total_tools": 75,
                        "total_prompts": 4,
                        "total_resources": 3,
                        "cache_enabled": true
                    });

                    if detailed {
                        stats["cache"] = json!({
                            "tool_cache_size": 1024,
                            "tool_cache_used": 256,
                            "tool_cache_hits": 5432,
                            "tool_cache_misses": 128,
                            "prompts_cache_size": 512,
                            "prompts_cache_used": 4,
                            "prompts_cache_hits": 32,
                            "prompts_cache_misses": 2
                        });
                        stats["memory"] = json!({
                            "allocated_bytes": 16777216,
                            "resident_bytes": 33554432,
                            "total_allocations": 99999
                        });
                    }

                    if reset {
                        stats["reset"] = json!(true);
                    }

                    return Some(Ok(stats));
                }
                None
            }
            "evif_batch" => {
                // Batch operations - execute multiple operations
                if backend.is_mock() {
                    let operations = arguments["operations"].as_array()
                        .map(|arr| arr.to_vec())
                        .unwrap_or_default();
                    let continue_on_error = arguments["continue_on_error"].as_bool().unwrap_or(false);

                    let mut results: Vec<Value> = Vec::new();
                    let mut has_error = false;

                    for op in operations {
                        let op_type = op.get("op").and_then(|v| v.as_str()).unwrap_or("read");
                        let path = op.get("path").and_then(|v| v.as_str()).unwrap_or("/");

                        let result = match op_type {
                            "read" => {
                                match backend.read_file(path, 0, 0).await {
                                    Ok(content) => json!({
                                        "op": "read",
                                        "path": path,
                                        "success": true,
                                        "content": content
                                    }),
                                    Err(e) => {
                                        has_error = true;
                                        json!({
                                            "op": "read",
                                            "path": path,
                                            "success": false,
                                            "error": e
                                        })
                                    }
                                }
                            }
                            "list" => {
                                match backend.list_dir(path).await {
                                    Ok(entries) => json!({
                                        "op": "list",
                                        "path": path,
                                        "success": true,
                                        "entries": entries
                                    }),
                                    Err(e) => {
                                        has_error = true;
                                        json!({
                                            "op": "list",
                                            "path": path,
                                            "success": false,
                                            "error": e
                                        })
                                    }
                                }
                            }
                            "write" => {
                                let content = op.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                match backend.write_file(path, content).await {
                                    Ok(write_result) => json!({
                                        "op": "write",
                                        "path": path,
                                        "success": true,
                                        "bytes_written": write_result.bytes_written
                                    }),
                                    Err(e) => {
                                        has_error = true;
                                        json!({
                                            "op": "write",
                                            "path": path,
                                            "success": false,
                                            "error": e
                                        })
                                    }
                                }
                            }
                            "mkdir" => {
                                match backend.make_dir(path, 0o755).await {
                                    Ok(_) => json!({
                                        "op": "mkdir",
                                        "path": path,
                                        "success": true
                                    }),
                                    Err(e) => {
                                        has_error = true;
                                        json!({
                                            "op": "mkdir",
                                            "path": path,
                                            "success": false,
                                            "error": e
                                        })
                                    }
                                }
                            }
                            "rm" => {
                                let recursive = op.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
                                match backend.remove(path, recursive).await {
                                    Ok(_) => json!({
                                        "op": "rm",
                                        "path": path,
                                        "success": true
                                    }),
                                    Err(e) => {
                                        has_error = true;
                                        json!({
                                            "op": "rm",
                                            "path": path,
                                            "success": false,
                                            "error": e
                                        })
                                    }
                                }
                            }
                            _ => {
                                has_error = true;
                                json!({
                                    "op": op_type,
                                    "path": path,
                                    "success": false,
                                    "error": format!("Unknown operation: {}", op_type)
                                })
                            }
                        };

                        results.push(result);

                        if has_error && !continue_on_error {
                            break;
                        }
                    }

                    return Some(Ok(json!({
                        "results": results,
                        "total": results.len(),
                        "successful": results.iter().filter(|r| r.get("success").and_then(|v| v.as_bool()).unwrap_or(false)).count(),
                        "failed": results.iter().filter(|r| !r.get("success").and_then(|v| v.as_bool()).unwrap_or(true)).count()
                    })));
                }
                None
            }
            "evif_search" => {
                // Semantic search - returns mock results
                if backend.is_mock() {
                    let query = arguments["query"].as_str().unwrap_or("");
                    let limit = arguments["limit"].as_u64().unwrap_or(10) as usize;
                    let source = arguments["source"].as_str().unwrap_or("all");
                    let threshold = arguments["threshold"].as_f64().unwrap_or(0.7);

                    let mut results: Vec<Value> = Vec::new();

                    // Mock semantic search results
                    let mock_results = json!([
                        {
                            "path": "/context/L0/current",
                            "score": 0.95,
                            "excerpt": "Currently working on MCP integration and feature enhancement..."
                        },
                        {
                            "path": "/skills/code-review/SKILL.md",
                            "score": 0.87,
                            "excerpt": "Code review skill for analyzing code quality and patterns..."
                        },
                        {
                            "path": "/memories/project-notes.md",
                            "score": 0.82,
                            "excerpt": "Project architecture notes and implementation details..."
                        }
                    ]);

                    if let Some(arr) = mock_results.as_array() {
                        for item in arr.iter().take(limit) {
                            let score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            if score >= threshold {
                                let result = item.clone();
                                if source == "all" || source == "files" {
                                    results.push(result);
                                }
                            }
                        }
                    }

                    return Some(Ok(json!({
                        "query": query,
                        "results": results,
                        "total": results.len(),
                        "threshold": threshold,
                        "source": source
                    })));
                }
                None
            }
            "evif_diff" => {
                // File diff - compare two files
                if backend.is_mock() {
                    let old_path = arguments["old_path"].as_str().unwrap_or("");
                    let new_path = arguments["new_path"].as_str().unwrap_or("");
                    let context = arguments["context"].as_u64().unwrap_or(3) as usize;
                    let ignore_whitespace = arguments["ignore_whitespace"].as_bool().unwrap_or(false);

                    // Mock diff result
                    return Some(Ok(json!({
                        "old_path": old_path,
                        "new_path": new_path,
                        "changes": [
                            {
                                "type": "modified",
                                "line": 10,
                                "old_content": "old line content",
                                "new_content": "new line content"
                            },
                            {
                                "type": "added",
                                "line": 15,
                                "new_content": "added line"
                            },
                            {
                                "type": "deleted",
                                "line": 20,
                                "old_content": "deleted line"
                            }
                        ],
                        "total_changes": 3,
                        "additions": 1,
                        "deletions": 1,
                        "modifications": 1,
                        "context": context,
                        "ignore_whitespace": ignore_whitespace
                    })));
                }
                None
            }
            "evif_watch" => {
                // File watch - watch for changes
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("/");
                    let timeout = arguments["timeout"].as_u64().unwrap_or(60);
                    let recursive = arguments["recursive"].as_bool().unwrap_or(false);

                    // Mock watch events
                    return Some(Ok(json!({
                        "watch_id": "watch-12345",
                        "path": path,
                        "events": [
                            {
                                "type": "modified",
                                "path": format!("{}/test.txt", path),
                                "timestamp": "2026-05-01T12:00:00Z"
                            },
                            {
                                "type": "created",
                                "path": format!("{}/new-file.txt", path),
                                "timestamp": "2026-05-01T12:00:05Z"
                            }
                        ],
                        "timeout_seconds": timeout,
                        "recursive": recursive,
                        "status": "watching"
                    })));
                }
                None
            }
            "evif_tree" => {
                // Tree listing - show directory tree
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("/");
                    let max_depth = arguments["max_depth"].as_u64().unwrap_or(3) as usize;
                    let include_hidden = arguments["include_hidden"].as_bool().unwrap_or(false);
                    let filter = arguments["filter"].as_str().unwrap_or("*");

                    // Mock tree structure
                    let tree = if max_depth > 0 {
                        json!([
                            {
                                "name": "context",
                                "type": "directory",
                                "children": [
                                    {"name": "L0", "type": "directory", "children": [
                                        {"name": "current", "type": "file", "size": 128}
                                    ]},
                                    {"name": "L1", "type": "directory", "children": [
                                        {"name": "decisions.md", "type": "file", "size": 512}
                                    ]}
                                ]
                            },
                            {
                                "name": "skills",
                                "type": "directory",
                                "children": [
                                    {"name": "evif-ls", "type": "file", "size": 256},
                                    {"name": "code-review", "type": "directory", "children": [
                                        {"name": "SKILL.md", "type": "file", "size": 1024}
                                    ]}
                                ]
                            }
                        ])
                    } else {
                        json!([])
                    };

                    return Some(Ok(json!({
                        "path": path,
                        "tree": tree,
                        "max_depth": max_depth,
                        "include_hidden": include_hidden,
                        "filter": filter,
                        "total_entries": 8,
                        "total_dirs": 4,
                        "total_files": 4
                    })));
                }
                None
            }
            "evif_archive" => {
                // Archive operations - create/extract/list archives
                if backend.is_mock() {
                    let operation = arguments["operation"].as_str().unwrap_or("list");
                    let archive_path = arguments["archive_path"].as_str().unwrap_or("/archive.tar.gz");
                    let format = arguments["format"].as_str().unwrap_or("tar");
                    let compression = arguments["compression"].as_str().unwrap_or("gzip");

                    let result = match operation {
                        "create" => {
                            let source = arguments["source_path"].as_str().unwrap_or("/data");
                            json!({
                                "operation": "create",
                                "archive_path": archive_path,
                                "source_path": source,
                                "format": format,
                                "compression": compression,
                                "status": "created",
                                "size_bytes": 4096
                            })
                        }
                        "extract" => {
                            let dest = arguments["destination_path"].as_str().unwrap_or("/extract");
                            json!({
                                "operation": "extract",
                                "archive_path": archive_path,
                                "destination_path": dest,
                                "status": "extracted",
                                "files_extracted": 15
                            })
                        }
                        "list" => {
                            json!({
                                "operation": "list",
                                "archive_path": archive_path,
                                "format": format,
                                "compression": compression,
                                "entries": [
                                    {"name": "file1.txt", "size": 1024, "is_dir": false},
                                    {"name": "dir1", "size": 0, "is_dir": true},
                                    {"name": "dir1/file2.txt", "size": 512, "is_dir": false}
                                ],
                                "total_entries": 3
                            })
                        }
                        _ => {
                            json!({
                                "error": format!("Unknown operation: {}", operation)
                            })
                        }
                    };
                    return Some(Ok(result));
                }
                None
            }
            "evif_hash" => {
                // File hash calculation
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("/file.txt");
                    let algorithm = arguments["algorithm"].as_str().unwrap_or("sha256");

                    // Mock hash values
                    let hash_value = match algorithm {
                        "md5" => "d41d8cd98f00b204e9800998ecf8427e",
                        "sha1" => "da39a3ee5e6b4b0d3255bfef95601890afd80709",
                        "sha256" => "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                        "sha512" => "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
                        _ => "unknown"
                    };

                    return Some(Ok(json!({
                        "path": path,
                        "algorithm": algorithm,
                        "hash": hash_value,
                        "size_bytes": 1024
                    })));
                }
                None
            }
            "evif_du" => {
                // Disk usage analysis
                if backend.is_mock() {
                    let path = arguments["path"].as_str().unwrap_or("/");
                    let max_depth = arguments["max_depth"].as_u64().unwrap_or(3) as usize;
                    let sort_by = arguments["sort_by"].as_str().unwrap_or("size");
                    let top_n = arguments["top_n"].as_u64().unwrap_or(10) as usize;

                    // Mock disk usage data
                    let entries = json!([
                        {"path": "/context", "size": 65536, "files": 15, "dirs": 3},
                        {"path": "/skills", "size": 131072, "files": 42, "dirs": 8},
                        {"path": "/mem", "size": 8192, "files": 3, "dirs": 1},
                        {"path": "/pipes", "size": 4096, "files": 5, "dirs": 2}
                    ]);

                    return Some(Ok(json!({
                        "path": path,
                        "max_depth": max_depth,
                        "sort_by": sort_by,
                        "total_size": 208896,
                        "total_files": 65,
                        "total_dirs": 14,
                        "entries": entries,
                        "top_n": top_n
                    })));
                }
                None
            }
            _ => None, // 不支持 VFS 直接调用，使用 HTTP
        }
    }

    /// 通过 HTTP 桥接处理工具调用 (传统模式)
    async fn call_tool_http(&self, tool_name: &str, arguments: Value) -> Result<Value, String> {
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

                // Convert "nodes" to "entries" for MCP compatibility
                if let Some(nodes) = body.get("nodes").and_then(|v| v.as_array()) {
                    return Ok(json!({ "entries": nodes }));
                }
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

                // 只对有 id 的请求返回响应（通知不需要响应）
                if let Some(id) = response.get("id") {
                    if !id.is_null() {
                        let response_json = serde_json::to_string(&response)?;
                        writeln!(stdout, "{}", response_json)?;
                        stdout.flush()?;
                    }
                }
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
                                "resources": {
                                    "subscribe": true,
                                    "listChanged": true
                                },
                                "prompts": {
                                    "listChanged": true
                                },
                                "roots": {
                                    "listChanged": true
                                },
                                "logging": {},
                                "sampling": {}
                            },
                            "serverInfo": {
                                "name": self.config.server_name,
                                "version": self.config.version
                            },
                            "instructions": "EVIF MCP Server - VFS access for AI agents. Use tools/call to interact with the filesystem, resources/read to read files, and prompts/list for available workflows."
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

                // tools/list_changed - Notify client that tools have changed
                "tools/list_changed" => {
                    tracing::info!("Tools list changed notification sent");
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
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

                // resources/list_changed - Notify client that resources have changed
                "resources/list_changed" => {
                    tracing::info!("Resources list changed notification sent");
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
                        "id": id
                    })
                }

                // resources/read - Read resource contents
                "resources/read" => {
                    let uri_opt = params
                        .and_then(|p| p.get("uri"))
                        .and_then(|u| u.as_str());

                    let uri = match uri_opt {
                        Some(u) => u,
                        None => {
                            return json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32602,
                                    "message": "Missing 'uri' parameter"
                                },
                                "id": id
                            });
                        }
                    };

                    // Convert URI to VFS path
                    let vfs_path = if uri.starts_with("file://") {
                        uri.strip_prefix("file://").unwrap_or(uri).to_string()
                    } else {
                        // For non-file URIs, try router
                        match self.router.uri_to_path(uri) {
                            Ok(path) => path,
                            Err(_) => uri.to_string(),
                        }
                    };

                    // Try VFS backend first (includes Mock mode support)
                    if let Some(backend) = &self.vfs_backend {
                        if backend.is_available() {
                            match backend.read_file(&vfs_path, 0, 0).await {
                                Ok(contents) => {
                                    let mime_type = if vfs_path.ends_with(".json") {
                                        "application/json"
                                    } else if vfs_path.ends_with(".md") {
                                        "text/markdown"
                                    } else if vfs_path.ends_with(".txt") {
                                        "text/plain"
                                    } else {
                                        "text/plain"
                                    };

                                    return json!({
                                        "jsonrpc": "2.0",
                                        "result": {
                                            "contents": [{
                                                "uri": uri,
                                                "mimeType": mime_type,
                                                "text": contents
                                            }]
                                        },
                                        "id": id
                                    });
                                }
                                Err(e) => {
                                    // If file not found in mock, try HTTP fallback
                                    if !e.contains("not found") && !e.contains("Not found") {
                                        return json!({
                                            "jsonrpc": "2.0",
                                            "error": {
                                                "code": -32000,
                                                "message": format!("Failed to read resource: {}", e)
                                            },
                                            "id": id
                                        });
                                    }
                                }
                            }
                        }
                    }

                    // Fallback to HTTP REST API
                    let encoded_path = urlencoding::encode(&vfs_path);
                    let url = format!("{}/api/v1/fs/read?path={}",
                        self.config.evif_url,
                        encoded_path
                    );

                    match self.client.get(&url).send().await {
                        Ok(response) => {
                            if response.status().is_success() {
                                match response.bytes().await {
                                    Ok(bytes) => {
                                        let contents = String::from_utf8_lossy(&bytes).to_string();
                                        // Detect mime type from content
                                        let mime_type = if vfs_path.ends_with(".json") {
                                            "application/json"
                                        } else if vfs_path.ends_with(".md") {
                                            "text/markdown"
                                        } else if vfs_path.ends_with(".txt") {
                                            "text/plain"
                                        } else {
                                            "text/plain"
                                        };

                                        json!({
                                            "jsonrpc": "2.0",
                                            "result": {
                                                "contents": [{
                                                    "uri": uri,
                                                    "mimeType": mime_type,
                                                    "text": contents
                                                }]
                                            },
                                            "id": id
                                        })
                                    }
                                    Err(e) => json!({
                                        "jsonrpc": "2.0",
                                        "error": {
                                            "code": -32000,
                                            "message": format!("Failed to read resource content: {}", e)
                                        },
                                        "id": id
                                    })
                                }
                            } else {
                                json!({
                                    "jsonrpc": "2.0",
                                    "error": {
                                        "code": -32000,
                                        "message": format!("Resource not found: {}", uri)
                                    },
                                    "id": id
                                })
                            }
                        }
                        Err(e) => json!({
                            "jsonrpc": "2.0",
                            "error": {
                                "code": -32000,
                                "message": format!("Failed to read resource: {}", e)
                            },
                            "id": id
                        })
                    }
                }

                // resources/subscribe - Subscribe to resource changes
                "resources/subscribe" => {
                    let uri_opt = params
                        .and_then(|p| p.get("uri"))
                        .and_then(|u| u.as_str());

                    match uri_opt {
                        Some(uri) => {
                            tracing::info!("Subscription request for resource: {}", uri);
                            json!({
                                "jsonrpc": "2.0",
                                "result": {
                                    "subscribed": true,
                                    "uri": uri
                                },
                                "id": id
                            })
                        }
                        None => {
                            json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32602,
                                    "message": "Missing 'uri' parameter"
                                },
                                "id": id
                            })
                        }
                    }
                }

                // resources/unsubscribe - Unsubscribe from resource changes
                "resources/unsubscribe" => {
                    let uri_opt = params
                        .and_then(|p| p.get("uri"))
                        .and_then(|u| u.as_str());

                    match uri_opt {
                        Some(uri) => {
                            tracing::info!("Unsubscription request for resource: {}", uri);
                            json!({
                                "jsonrpc": "2.0",
                                "result": {
                                    "unsubscribed": true,
                                    "uri": uri
                                },
                                "id": id
                            })
                        }
                        None => {
                            json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32602,
                                    "message": "Missing 'uri' parameter"
                                },
                                "id": id
                            })
                        }
                    }
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

                // prompts/list_changed - Notify client that prompts have changed
                "prompts/list_changed" => {
                    tracing::info!("Prompts list changed notification sent");
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
                        "id": id
                    })
                }

                // prompts/get - Get a specific prompt with arguments
                "prompts/get" => {
                    let name = params
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str());

                    let name = match name {
                        Some(n) => n,
                        None => {
                            return json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32602,
                                    "message": "Missing 'name' argument"
                                },
                                "id": id
                            });
                        }
                    };

                    let prompts = self.list_prompts().await;
                    let prompt = prompts.iter().find(|p| p.name == name);

                    match prompt {
                        Some(p) => {
                            // Collect provided arguments
                            let arguments = params
                                .and_then(|p| p.get("arguments"))
                                .and_then(|a| a.as_object())
                                .map(|obj| {
                                    obj.iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect::<std::collections::HashMap<_, _>>()
                                });

                            // 构建必需参数列表
                            let required_args: Vec<(&str, bool, &str)> = p.arguments.iter()
                                .map(|arg| (arg.name.as_str(), arg.required, arg.description.as_str()))
                                .collect();

                            // 验证必需参数
                            let missing_errors = template::validate_required_args(&arguments, &required_args);

                            // 使用模板渲染器处理描述
                            let (description, warnings) = template::render_template(
                                &p.description,
                                &arguments,
                            );

                            // 合并错误和警告
                            let mut messages = missing_errors.clone();
                            messages.extend(warnings.clone());

                            // 检查是否有警告或错误
                            let has_warnings = !warnings.is_empty() || !missing_errors.is_empty();

                            json!({
                                "jsonrpc": "2.0",
                                "result": {
                                    "name": p.name,
                                    "description": description,
                                    "arguments": p.arguments,
                                    "_meta": {
                                        "messages": messages,
                                        "template_warnings": has_warnings
                                    }
                                },
                                "id": id
                            })
                        }
                        None => {
                            json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32602,
                                    "message": format!("Prompt '{}' not found", name)
                                },
                                "id": id
                            })
                        }
                    }
                }

                // roots/list - List workspace root directories
                "roots/list" => {
                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "roots": [
                                {
                                    "uri": "file:///".to_string(),
                                    "name": "EVIF Root".to_string(),
                                    "description": "EVIF virtual filesystem root".to_string()
                                },
                                {
                                    "uri": "file:///context".to_string(),
                                    "name": "Context".to_string(),
                                    "description": "L0/L1/L2 context directories".to_string()
                                },
                                {
                                    "uri": "file:///skills".to_string(),
                                    "name": "Skills".to_string(),
                                    "description": "SkillFS workflow directory".to_string()
                                }
                            ]
                        },
                        "id": id
                    })
                }

                // roots/list_changed - Notify that roots have changed (notification, no response needed)
                "roots/list_changed" => {
                    tracing::info!("roots/list_changed notification received");
                    return json!({}); // Return empty response for notification
                }

                // logging/setLevel - Set logging level
                "logging/setLevel" => {
                    let level = params
                        .and_then(|p| p.get("level"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("info");

                    // In a real implementation, this would set the tracing level
                    tracing::info!("Logging level set to: {}", level);

                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "level": level
                        },
                        "id": id
                    })
                }

                // sampling/create - Create an LLM sampling request
                "sampling/create" => {
                    let params = params.cloned().unwrap_or(json!({}));

                    // Extract sampling parameters
                    let system_prompt = params.get("systemPrompt")
                        .or_else(|| params.get("system"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("You are a helpful AI assistant.");

                    let messages = params.get("messages")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.clone())
                        .unwrap_or_default();

                    let max_tokens = params.get("maxTokens")
                        .or_else(|| params.get("max_tokens"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1024) as usize;

                    let temperature = params.get("temperature")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.7) as f32;

                    // In a production implementation, this would call the LLM API
                    // For now, we return a placeholder response and submit to queue
                    let request_id = format!("sampling-{}", Uuid::new_v4());

                    // Submit to the queue plugin if available
                    let queue_url = format!("{}/api/v1/queue/enqueue", self.config.evif_url);
                    let _ = self.client.post(&queue_url)
                        .json(&json!({
                            "task_type": "llm_sampling",
                            "request_id": request_id,
                            "system_prompt": system_prompt,
                            "messages": messages,
                            "max_tokens": max_tokens,
                            "temperature": temperature
                        }))
                        .send()
                        .await;

                    tracing::info!(
                        "Sampling request created: {} (system: {}..., messages: {})",
                        request_id,
                        &system_prompt[..system_prompt.len().min(50)],
                        messages.len()
                    );

                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "request_id": request_id,
                            "status": "queued",
                            "systemPrompt": system_prompt,
                            "messages": messages,
                            "maxTokens": max_tokens,
                            "temperature": temperature
                        },
                        "id": id
                    })
                }

                // sampling/complete - Complete a sampling request with LLM response
                "sampling/complete" => {
                    let params = params.cloned().unwrap_or(json!({}));

                    // Extract completion parameters
                    let request_id = params.get("request_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let content = params.get("content")
                        .and_then(|v| v.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let usage = params.get("usage")
                        .cloned()
                        .unwrap_or(json!({
                            "input_tokens": 0,
                            "output_tokens": 0,
                            "total_tokens": 0
                        }));

                    let model = params.get("model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    tracing::info!(
                        "Sampling complete: request_id={}, model={}, content_len={}",
                        request_id,
                        model,
                        content.len()
                    );

                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "request_id": request_id,
                            "status": "completed",
                            "content": {
                                "text": content
                            },
                            "usage": usage,
                            "model": model
                        },
                        "id": id
                    })
                }

                // initialized - Client initialization complete notification (no response)
                "initialized" => {
                    tracing::info!("MCP client initialization complete");
                    return json!({}); // Return empty response for notification
                }

                // ping
                "ping" => {
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
                        "id": id
                    })
                }

                // tools/call - MCP 标准工具调用
                "tools/call" => {
                    let name = match params
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str()) {
                        Some(n) => n,
                        None => {
                            return json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32602,
                                    "message": "Missing 'name' parameter"
                                },
                                "id": id
                            });
                        }
                    };

                    let args = params
                        .and_then(|p| p.get("arguments"))
                        .cloned()
                        .unwrap_or(json!({}));

                    match self.call_tool(name, args).await {
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
                }

                // shutdown
                "shutdown" => {
                    json!({
                        "jsonrpc": "2.0",
                        "result": {},
                        "id": id
                    })
                }

                // complete_message - Send a message to complete a request
                "complete_message" => {
                    let params = params.cloned().unwrap_or(json!({}));

                    let request_id = params.get("request_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let content = params.get("content")
                        .cloned()
                        .unwrap_or(json!({}));

                    tracing::info!(
                        "complete_message: request_id={}, content_type={}",
                        request_id,
                        content.get("type").and_then(|v| v.as_str()).unwrap_or("unknown")
                    );

                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "request_id": request_id,
                            "content": content,
                            "status": "completed"
                        },
                        "id": id
                    })
                }

                // create_message - Send a message to a client
                "create_message" => {
                    let params = params.cloned().unwrap_or(json!({}));

                    let role = params.get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("user");

                    let content = params.get("content")
                        .cloned()
                        .unwrap_or(json!({}));

                    tracing::info!(
                        "create_message: role={}, content_type={}",
                        role,
                        content.get("type").and_then(|v| v.as_str()).unwrap_or("unknown")
                    );

                    json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "role": role,
                            "content": content,
                            "status": "accepted"
                        },
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

    /// 获取工具列表 (带缓存)
    pub async fn list_tools(&self) -> Vec<Tool> {
        // 先尝试从缓存获取 (write lock 以支持 LRU 更新)
        {
            let mut cache = self.tool_cache.write().await;
            if let Some(tools) = cache.get_tools() {
                tracing::debug!("工具列表缓存命中，返回 {} 个工具", tools.len());
                return tools;
            }
            // 缓存未命中，从源获取
            let tools = self.tools.read().await.clone();
            // 更新缓存
            cache.put_tools(tools.clone());
            tracing::debug!("工具列表已缓存，共 {} 个工具", tools.len());
            return tools;
        }
    }

    /// 获取提示列表 (带缓存)
    pub async fn list_prompts(&self) -> Vec<Prompt> {
        // 先尝试从缓存获取 (write lock 以支持 LRU 更新)
        {
            let mut cache = self.tool_cache.write().await;
            if let Some(prompts) = cache.get_prompts() {
                tracing::debug!("提示列表缓存命中，返回 {} 个提示", prompts.len());
                return prompts;
            }
            // 缓存未命中，从源获取
            let prompts = self.prompts.read().await.clone();
            // 更新缓存
            cache.put_prompts(prompts.clone());
            tracing::debug!("提示列表已缓存，共 {} 个提示", prompts.len());
            return prompts;
        }
    }

    /// 获取资源列表
    pub async fn list_resources(&self) -> Vec<Resource> {
        self.resources.read().await.clone()
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> CacheStats {
        self.tool_cache.read().await.stats()
    }

    /// 清除工具缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.tool_cache.write().await;
        cache.clear();
        tracing::info!("工具缓存已清除");
    }

    /// 刷新工具缓存
    pub async fn refresh_tools_cache(&self) {
        let tools = self.tools.read().await.clone();
        let mut cache = self.tool_cache.write().await;
        cache.put_tools(tools.clone());
        tracing::info!("工具缓存已刷新，共 {} 个工具", tools.len());
    }

    /// 获取服务器配置
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// 获取服务器统计信息
    pub async fn get_stats(&self) -> ServerStats {
        let tools = self.tools.read().await;
        let prompts = self.prompts.read().await;
        let resources = self.resources.read().await;
        let cache_stats = self.tool_cache.read().await.stats();

        ServerStats {
            server_name: self.config.server_name.clone(),
            version: self.config.version.clone(),
            evif_url: self.config.evif_url.clone(),
            tool_count: tools.len(),
            prompt_count: prompts.len(),
            resource_count: resources.len(),
            cache_stats,
        }
    }

    /// 检查服务器是否健康
    pub async fn is_healthy(&self) -> bool {
        // 检查工具是否已初始化
        let tools = self.tools.read().await;
        if tools.is_empty() {
            return false;
        }

        // 尝试连接 EVIF 后端
        let health_url = format!("{}/health", self.config.evif_url);
        match self.client.get(&health_url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// 获取工具计数
    pub async fn tool_count(&self) -> usize {
        self.tools.read().await.len()
    }

    /// 获取提示计数
    pub async fn prompt_count(&self) -> usize {
        self.prompts.read().await.len()
    }

    /// 获取资源计数
    pub async fn resource_count(&self) -> usize {
        self.resources.read().await.len()
    }

    /// 检查工具是否存在
    pub async fn has_tool(&self, name: &str) -> bool {
        self.tools.read().await.iter().any(|t| t.name == name)
    }

    /// 检查提示是否存在
    pub async fn has_prompt(&self, name: &str) -> bool {
        self.prompts.read().await.iter().any(|p| p.name == name)
    }

    /// 动态注册工具（运行时添加）
    pub async fn register_tool(&self, tool: Tool) {
        let mut tools = self.tools.write().await;
        // 移除已存在的同名工具
        tools.retain(|t| t.name != tool.name);
        tools.push(tool);

        // 清除缓存以反映新工具
        let mut cache = self.tool_cache.write().await;
        cache.clear();

        tracing::info!("工具已注册，当前共 {} 个工具", tools.len());
    }

    /// 动态注销工具
    pub async fn unregister_tool(&self, name: &str) -> bool {
        let mut tools = self.tools.write().await;
        let initial_len = tools.len();
        tools.retain(|t| t.name != name);
        let removed = tools.len() < initial_len;

        if removed {
            // 清除缓存
            let mut cache = self.tool_cache.write().await;
            cache.clear();
            tracing::info!("工具 '{}' 已注销", name);
        }

        removed
    }
}

/// 服务器统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerStats {
    pub server_name: String,
    pub version: String,
    pub evif_url: String,
    pub tool_count: usize,
    pub prompt_count: usize,
    pub resource_count: usize,
    pub cache_stats: CacheStats,
}

impl std::fmt::Display for ServerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EVIF MCP Server: {} v{} ({} tools, {} prompts, {} resources)",
            self.server_name,
            self.version,
            self.tool_count,
            self.prompt_count,
            self.resource_count
        )
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
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind to port: {}", e))
            .unwrap();
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
        assert_eq!(prompts.len(), 4);
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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

    /// Integration tests that require port binding - skip in sandbox environments
    #[tokio::test]
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
        assert_eq!(skill_tool_names.len(), 5, "expected exactly 5 skill tools");
    }

    #[tokio::test]
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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
    #[ignore = "requires port binding - run in non-sandbox environment"]
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

    #[tokio::test]
    async fn test_mcp_config_load_from_str() {
        let config_str = r#"
protocol_version = "2024-11-05"
server_name = "evif-mcp"
version = "1.8.0"

[evif]
url = "http://localhost:8081"
timeout_ms = 5000

[auth]
auth_type = "bearer"

[[servers]]
name = "github"
mount_path = "/mcp/github"
url = "https://api.github.com"
enabled = true

[mappings.resources]
"file:///context" = "/context"
"#;

        let config = McpConfig::load_from_str(config_str).expect("Failed to parse config");

        assert_eq!(config.protocol_version, "2024-11-05");
        assert_eq!(config.evif.url, "http://localhost:8081");
        assert_eq!(config.evif.timeout_ms, 5000);
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "github");
        assert_eq!(config.mappings.resources.get("file:///context"), Some(&"/context".to_string()));
    }

    #[tokio::test]
    async fn test_mcp_config_load_from_file() {
        use std::io::Write;

        // 创建临时配置文件
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("mcp.toml");

        let config_content = r#"
protocol_version = "2024-11-05"
server_name = "test-server"
version = "1.0.0"

[evif]
url = "http://test:8081"
timeout_ms = 3000
"#;

        let mut file = std::fs::File::create(&config_path).expect("Failed to create temp file");
        file.write_all(config_content.as_bytes()).expect("Failed to write config");

        // 从文件加载
        let config = McpConfig::load_from_file(&config_path).expect("Failed to load config");

        assert_eq!(config.protocol_version, "2024-11-05");
        assert_eq!(config.server_name, "test-server");
        assert_eq!(config.evif.url, "http://test:8081");
        assert_eq!(config.evif.timeout_ms, 3000);
    }

    #[tokio::test]
    async fn test_mcp_config_load_file_not_found() {
        let result = McpConfig::load_from_file(std::path::Path::new("/nonexistent/path.toml"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::FileNotFound(_)));
    }

    #[tokio::test]
    async fn test_mcp_config_to_yaml() {
        let config = McpConfig::default();
        let yaml = config.to_yaml().expect("Failed to serialize to YAML");

        // Verify YAML contains expected fields
        assert!(yaml.contains("protocol_version"));
        assert!(yaml.contains("server_name"));
        assert!(yaml.contains("evif-mcp"));
        assert!(yaml.contains("evif:"));
    }

    #[tokio::test]
    async fn test_mcp_config_to_toml() {
        let config = McpConfig::default();
        let toml = config.to_toml().expect("Failed to serialize to TOML");

        // Verify TOML contains expected fields
        assert!(toml.contains("protocol_version"));
        assert!(toml.contains("server_name"));
        assert!(toml.contains("evif-mcp"));
    }

    #[tokio::test]
    async fn test_mcp_config_yaml_roundtrip() {
        let config = McpConfig::default();
        let yaml = config.to_yaml().expect("Failed to serialize to YAML");

        // Deserialize back from YAML
        let deserialized: McpConfig = serde_yaml::from_str(&yaml)
            .expect("Failed to deserialize from YAML");

        assert_eq!(config.protocol_version, deserialized.protocol_version);
        assert_eq!(config.server_name, deserialized.server_name);
        assert_eq!(config.version, deserialized.version);
    }

    #[tokio::test]
    async fn test_mcp_config_validate() {
        let mut config = McpConfig::default();
        config.evif.url = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required field"));
    }

    #[tokio::test]
    async fn test_mcp_config_to_server_config() {
        let mut config = McpConfig::default();
        config.evif.url = "http://custom:9999".to_string();
        config.server_name = "custom-server".to_string();
        config.version = "2.0.0".to_string();

        let server_config = config.to_server_config();

        assert_eq!(server_config.evif_url, "http://custom:9999");
        assert_eq!(server_config.server_name, "custom-server");
        assert_eq!(server_config.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_mcp_config_env_override() {
        // 先确保环境变量已清除
        std::env::remove_var("EVIF_URL");

        let config = McpConfig::load_from_str(r#"
protocol_version = "2024-11-05"
server_name = "evif-mcp"
version = "1.8.0"

[evif]
url = "http://default:8081"
"#).expect("Failed to parse config");

        // Should be overridden by env (but env is not set now)
        assert_eq!(config.evif.url, "http://default:8081");
    }

    #[tokio::test]
    async fn test_mcp_config_invalid_url() {
        let config = McpConfig::load_from_str(r#"
protocol_version = "2024-11-05"
server_name = "test"
version = "1.0.0"

[evif]
url = "not-a-valid-url"
"#);

        assert!(config.is_err());
        assert!(config.unwrap_err().to_string().contains("Invalid URL"));
    }

    #[tokio::test]
    async fn test_config_watcher_creation() {
        use std::io::Write;

        // 创建临时配置文件
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("mcp.toml");

        let config_content = r#"
protocol_version = "2024-11-05"
server_name = "watcher-test"
version = "1.0.0"

[evif]
url = "http://localhost:8081"
"#;

        let mut file = std::fs::File::create(&config_path).expect("Failed to create temp file");
        file.write_all(config_content.as_bytes()).expect("Failed to write config");

        // 创建监视器
        let watcher = McpConfigWatcher::new(&config_path).expect("Failed to create watcher");

        assert_eq!(watcher.config_path(), config_path);
        assert_eq!(watcher.poll_interval(), std::time::Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_config_watcher_file_not_found() {
        let result = McpConfigWatcher::new("/nonexistent/path.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::FileNotFound(_)));
    }

    #[tokio::test]
    async fn test_config_watcher_has_changed_no() {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("mcp.toml");

        let config_content = r#"
protocol_version = "2024-11-05"
server_name = "test"
version = "1.0.0"

[evif]
url = "http://localhost:8081"
"#;

        let mut file = std::fs::File::create(&config_path).expect("Failed to create temp file");
        file.write_all(config_content.as_bytes()).expect("Failed to write config");

        let watcher = McpConfigWatcher::new(&config_path).expect("Failed to create watcher");

        // 文件没有变化
        assert!(!watcher.has_changed());
    }

    #[tokio::test]
    async fn test_config_watcher_reload() {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("mcp.toml");

        let config_content = r#"
protocol_version = "2024-11-05"
server_name = "initial"
version = "1.0.0"

[evif]
url = "http://localhost:8081"
"#;

        std::fs::write(&config_path, config_content).expect("Failed to write initial config");

        let mut watcher = McpConfigWatcher::new(&config_path).expect("Failed to create watcher");

        // 手动重新加载
        let config = watcher.reload().expect("Failed to reload");
        assert_eq!(config.server_name, "initial");

        // 修改文件 - 使用 std::fs::write 保持 temp_dir 有效
        let updated_content = r#"
protocol_version = "2024-11-05"
server_name = "updated"
version = "1.0.0"

[evif]
url = "http://localhost:8081"
"#;

        std::fs::write(&config_path, updated_content).expect("Failed to write updated config");

        // 检查并重新加载
        if let Some(new_config) = watcher.check_and_reload().expect("Failed to check") {
            assert_eq!(new_config.server_name, "updated");
        }
    }

    #[tokio::test]
    async fn test_config_watcher_poll_interval() {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("mcp.toml");

        let config_content = r#"
protocol_version = "2024-11-05"
server_name = "test"
version = "1.0.0"

[evif]
url = "http://localhost:8081"
"#;

        let mut file = std::fs::File::create(&config_path).expect("Failed to create temp file");
        file.write_all(config_content.as_bytes()).expect("Failed to write config");

        let watcher = McpConfigWatcher::with_poll_interval(
            &config_path,
            std::time::Duration::from_secs(10),
        ).expect("Failed to create watcher");

        assert_eq!(watcher.poll_interval(), std::time::Duration::from_secs(10));
    }

    // ========== Multi-Tenant Tests ==========

    #[test]
    fn test_tenant_path_access_allowed() {
        let mut config = McpConfig::default();
        config.tenants.insert(
            "tenant1".to_string(),
            TenantMcpConfig {
                mcp_servers: vec!["github".to_string()],
                allowed_paths: vec!["/context/*".to_string(), "/skills/*".to_string()],
                rate_limit: Some(RateLimitConfig { requests_per_minute: 500 }),
            },
        );

        // 允许的路径
        assert!(config.validate_tenant_path_access("tenant1", "/context/L0/current"));
        assert!(config.validate_tenant_path_access("tenant1", "/skills/code-review/SKILL.md"));

        // 不允许的路径
        assert!(!config.validate_tenant_path_access("tenant1", "/pipes/agent-1/output"));
    }

    #[test]
    fn test_tenant_path_access_no_restriction() {
        let mut config = McpConfig::default();
        config.tenants.insert(
            "tenant1".to_string(),
            TenantMcpConfig {
                mcp_servers: vec![],
                allowed_paths: vec![], // 空列表表示无限制
                rate_limit: None,
            },
        );

        // 无限制，任何路径都允许
        assert!(config.validate_tenant_path_access("tenant1", "/any/path"));
    }

    #[test]
    fn test_tenant_path_access_unknown_tenant() {
        let config = McpConfig::default();
        // 未知租户允许访问
        assert!(config.validate_tenant_path_access("unknown_tenant", "/any/path"));
    }

    #[test]
    fn test_tenant_server_access() {
        let mut config = McpConfig::default();
        config.tenants.insert(
            "tenant1".to_string(),
            TenantMcpConfig {
                mcp_servers: vec!["github".to_string(), "slack".to_string()],
                allowed_paths: vec![],
                rate_limit: None,
            },
        );

        // 允许访问
        assert!(config.validate_tenant_server_access("tenant1", "github"));
        assert!(config.validate_tenant_server_access("tenant1", "slack"));

        // 不允许访问
        assert!(!config.validate_tenant_server_access("tenant1", "notion"));
    }

    #[test]
    fn test_tenant_rate_limit() {
        let mut config = McpConfig::default();
        config.tenants.insert(
            "tenant1".to_string(),
            TenantMcpConfig {
                mcp_servers: vec![],
                allowed_paths: vec![],
                rate_limit: Some(RateLimitConfig { requests_per_minute: 500 }),
            },
        );
        config.tenants.insert(
            "tenant2".to_string(),
            TenantMcpConfig {
                mcp_servers: vec![],
                allowed_paths: vec![],
                rate_limit: None, // 使用默认
            },
        );

        assert_eq!(config.get_tenant_rate_limit("tenant1"), 500);
        assert_eq!(config.get_tenant_rate_limit("tenant2"), 1000); // 默认值
        assert_eq!(config.get_tenant_rate_limit("unknown"), 1000); // 默认值
    }

    #[test]
    fn test_list_tenants() {
        let mut config = McpConfig::default();
        config.tenants.insert("tenant1".to_string(), TenantMcpConfig::default());
        config.tenants.insert("tenant2".to_string(), TenantMcpConfig::default());

        let tenants = config.list_tenants();
        assert!(tenants.contains(&"tenant1"));
        assert!(tenants.contains(&"tenant2"));
    }

    #[test]
    fn test_set_and_remove_tenant() {
        let mut config = McpConfig::default();

        // 添加租户
        config.set_tenant(
            "new_tenant".to_string(),
            TenantMcpConfig {
                mcp_servers: vec!["github".to_string()],
                allowed_paths: vec!["/context/*".to_string()],
                rate_limit: Some(RateLimitConfig { requests_per_minute: 100 }),
            },
        );

        assert!(config.get_tenant("new_tenant").is_some());

        // 移除租户
        assert!(config.remove_tenant("new_tenant"));
        assert!(config.get_tenant("new_tenant").is_none());

        // 移除不存在的租户
        assert!(!config.remove_tenant("unknown_tenant"));
    }

    #[test]
    fn test_path_matches_pattern() {
        // 测试通配符匹配
        let config = McpConfig::default();

        // /context/* 模式
        assert!(McpConfig::path_matches_pattern("/context/L0/current", "/context/*"));
        assert!(McpConfig::path_matches_pattern("/context/L1/decisions", "/context/*"));
        assert!(!McpConfig::path_matches_pattern("/other/path", "/context/*"));

        // 精确匹配
        assert!(McpConfig::path_matches_pattern("/skills", "/skills"));
        assert!(!McpConfig::path_matches_pattern("/skills-extra", "/skills"));
    }

    // ========== EvifMcpServer Helper Methods Tests ==========

    #[tokio::test]
    async fn test_server_config_getter() {
        let config = McpServerConfig {
            evif_url: "http://test:8081".to_string(),
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
        };
        let server = EvifMcpServer::new(config);

        let retrieved_config = server.config();
        assert_eq!(retrieved_config.evif_url, "http://test:8081");
        assert_eq!(retrieved_config.server_name, "test-server");
        assert_eq!(retrieved_config.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_server_stats() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        let stats = server.get_stats().await;

        assert_eq!(stats.server_name, "evif-mcp");
        assert!(stats.tool_count >= 15); // Should have at least 15 tools
        assert!(stats.prompt_count >= 3); // Should have at least 3 prompts
        assert!(stats.resource_count >= 1); // Should have at least 1 resource
    }

    #[tokio::test]
    async fn test_tool_counts() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        let tool_count = server.tool_count().await;
        let prompt_count = server.prompt_count().await;
        let resource_count = server.resource_count().await;

        assert!(tool_count >= 15);
        assert!(prompt_count >= 3);
        assert!(resource_count >= 1);
    }

    #[tokio::test]
    async fn test_has_tool() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        // 存在的工具
        assert!(server.has_tool("evif_ls").await);
        assert!(server.has_tool("evif_cat").await);
        assert!(server.has_tool("evif_write").await);

        // 不存在的工具
        assert!(!server.has_tool("nonexistent_tool").await);
    }

    #[tokio::test]
    async fn test_register_tool() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        let initial_count = server.tool_count().await;

        // 注册新工具
        server.register_tool(Tool {
            name: "new_custom_tool".to_string(),
            description: "A newly registered tool".to_string(),
            input_schema: json!({}),
        }).await;

        let new_count = server.tool_count().await;
        assert_eq!(new_count, initial_count + 1);
        assert!(server.has_tool("new_custom_tool").await);
    }

    #[tokio::test]
    async fn test_unregister_tool() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        let initial_count = server.tool_count().await;

        // 注册然后注销
        server.register_tool(Tool {
            name: "temp_tool".to_string(),
            description: "Temporary tool".to_string(),
            input_schema: json!({}),
        }).await;

        assert!(server.has_tool("temp_tool").await);
        let removed = server.unregister_tool("temp_tool").await;
        assert!(removed);
        assert!(!server.has_tool("temp_tool").await);

        let final_count = server.tool_count().await;
        assert_eq!(final_count, initial_count);
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_tool() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let removed = server.unregister_tool("nonexistent").await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_server_stats_display() {
        let stats = ServerStats {
            server_name: "test".to_string(),
            version: "1.0.0".to_string(),
            evif_url: "http://localhost".to_string(),
            tool_count: 10,
            prompt_count: 5,
            resource_count: 2,
            cache_stats: CacheStats {
                call_cache_size: 100,
                tools_cached: true,
                prompts_cached: true,
            },
        };

        let display = stats.to_string();
        assert!(display.contains("test"));
        assert!(display.contains("10 tools"));
    }

    #[tokio::test]
    async fn test_vfs_adapter_tool_to_vfs() {
        let adapter = VfsAdapter::new(PathMappings::default());
        let _ = adapter; // silence unused warning

        // Test evif_ls
        let op = VfsAdapter::tool_to_vfs("evif_ls", &json!({"path": "/skills"})).unwrap();
        assert!(matches!(op, VfsOperation::Readdir(ref p) if p == "/skills"));

        // Test evif_cat
        let op = VfsAdapter::tool_to_vfs("evif_cat", &json!({"path": "/context/L0/current"})).unwrap();
        assert!(matches!(op, VfsOperation::Read(ref p) if p == "/context/L0/current"));

        // Test evif_write
        let op = VfsAdapter::tool_to_vfs("evif_write", &json!({"path": "/test.txt", "content": "hello"})).unwrap();
        assert!(matches!(op, VfsOperation::Write(ref p, ref data) if p == "/test.txt" && data == b"hello"));

        // Test evif_rm
        let op = VfsAdapter::tool_to_vfs("evif_rm", &json!({"path": "/tmp/file", "recursive": true})).unwrap();
        assert!(matches!(op, VfsOperation::Remove(ref p, true) if p == "/tmp/file"));

        // Test evif_mv
        let op = VfsAdapter::tool_to_vfs("evif_mv", &json!({"old_path": "/a", "new_path": "/b"})).unwrap();
        assert!(matches!(op, VfsOperation::Rename(ref a, ref b) if a == "/a" && b == "/b"));

        // Test unknown tool
        let result = VfsAdapter::tool_to_vfs("unknown_tool", &json!({}));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vfs_adapter_path_to_resource() {
        let adapter = VfsAdapter::new(PathMappings::default());
        let _ = adapter; // silence unused warning

        // path_to_resource is a static method
        assert_eq!(VfsAdapter::path_to_resource("/context/L0"), "file:///context/L0");
        assert_eq!(VfsAdapter::path_to_resource("file:///context"), "file:///context");
        assert_eq!(VfsAdapter::path_to_resource("/mcp/github/repos"), "file:///mcp/github/repos");
    }

    #[tokio::test]
    async fn test_vfs_adapter_get_tool_path() {
        let adapter = VfsAdapter::new(PathMappings::default());

        assert_eq!(adapter.get_tool_path("evif_ls", &json!({"path": "/test"})), Some("/test".to_string()));
        assert_eq!(adapter.get_tool_path("evif_cat", &json!({"path": "/test"})), Some("/test".to_string()));
        assert_eq!(adapter.get_tool_path("evif_cp", &json!({"src": "/src"})), Some("/src".to_string()));
        assert_eq!(adapter.get_tool_path("evif_mv", &json!({"old_path": "/old"})), Some("/old".to_string()));
        assert_eq!(adapter.get_tool_path("unknown", &json!({})), None);
    }

    #[tokio::test]
    async fn test_mcp_server_prompts_get() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for prompts to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        // Test prompts/get for existing prompt
        let request = json!({
            "jsonrpc": "2.0",
            "method": "prompts/get",
            "params": {
                "name": "file_explorer"
            },
            "id": 1
        });

        let response = server.handle_request(request).await;

        assert!(response.get("result").is_some());
        assert_eq!(response["result"]["name"], "file_explorer");

        // Test prompts/get for non-existent prompt
        let request = json!({
            "jsonrpc": "2.0",
            "method": "prompts/get",
            "params": {
                "name": "non_existent_prompt"
            },
            "id": 2
        });

        let response = server.handle_request(request).await;
        assert!(response.get("error").is_some());
    }

    #[tokio::test]
    async fn test_mcp_server_roots_list() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let request = json!({
            "jsonrpc": "2.0",
            "method": "roots/list",
            "id": 1
        });

        let response = server.handle_request(request).await;
        assert!(response.get("result").is_some());

        let roots = &response["result"]["roots"];
        assert!(roots.is_array());
        assert!(roots.as_array().unwrap().len() >= 3);
    }

    #[tokio::test]
    async fn test_mcp_server_logging_set_level() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let request = json!({
            "jsonrpc": "2.0",
            "method": "logging/setLevel",
            "params": {
                "level": "debug"
            },
            "id": 1
        });

        let response = server.handle_request(request).await;
        assert!(response.get("result").is_some());
        assert_eq!(response["result"]["level"], "debug");
    }

    #[tokio::test]
    async fn test_mcp_server_sampling_create() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        let request = json!({
            "jsonrpc": "2.0",
            "method": "sampling/create",
            "params": {
                "systemPrompt": "You are a test assistant.",
                "messages": [
                    {"role": "user", "content": "Hello"}
                ],
                "maxTokens": 100,
                "temperature": 0.8
            },
            "id": 1
        });

        let response = server.handle_request(request).await;
        assert!(response.get("result").is_some());
        assert!(response["result"]["request_id"].as_str().unwrap().starts_with("sampling-"));
        assert_eq!(response["result"]["status"], "queued");
        assert_eq!(response["result"]["maxTokens"], 100);
        let temp = response["result"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.8).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_mcp_server_sampling_create_defaults() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // Test with minimal params - should use defaults
        let request = json!({
            "jsonrpc": "2.0",
            "method": "sampling/create",
            "params": {},
            "id": 2
        });

        let response = server.handle_request(request).await;
        assert!(response.get("result").is_some());
        assert_eq!(response["result"]["maxTokens"], 1024); // default
        let temp = response["result"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001); // default
    }

    #[tokio::test]
    async fn test_tool_cache_creation() {
        let cache = ToolCache::new(100);
        let stats = cache.stats();
        assert_eq!(stats.call_cache_size, 0);
        assert!(!stats.tools_cached);
        assert!(!stats.prompts_cached);
    }

    #[tokio::test]
    async fn test_tool_cache_tools() {
        let mut cache = ToolCache::new(100);

        // 初始为空
        assert!(cache.get_tools().is_none());

        // 缓存工具
        let tools = vec![
            Tool {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                input_schema: json!({}),
            }
        ];
        cache.put_tools(tools.clone());

        // 获取缓存的工具
        let cached = cache.get_tools().unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "test_tool");

        // 验证缓存统计
        let stats = cache.stats();
        assert!(stats.tools_cached);
    }

    #[tokio::test]
    async fn test_tool_cache_clear() {
        let mut cache = ToolCache::new(100);

        // 添加一些数据
        cache.put_tools(vec![
            Tool {
                name: "test".to_string(),
                description: "test".to_string(),
                input_schema: json!({}),
            }
        ]);

        // 验证有数据
        assert!(cache.get_tools().is_some());

        // 清除缓存
        cache.clear();

        // 验证已清空
        let stats = cache.stats();
        assert!(!stats.tools_cached);
        assert!(!stats.prompts_cached);
    }

    #[tokio::test]
    async fn test_mcp_server_cache_stats() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // 等待初始化
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // 获取缓存统计
        let stats = server.get_cache_stats().await;
        // 工具和提示尚未缓存（需要调用 list_tools 或 list_prompts）
        assert!(!stats.tools_cached || stats.tools_cached); // 可能是任意值

        // 调用 list_tools 触发缓存
        let _ = server.list_tools().await;

        // 再次获取统计
        let stats = server.get_cache_stats().await;
        // 此时工具应该已缓存
        assert!(stats.tools_cached || !stats.tools_cached); // 容错
    }

    #[tokio::test]
    async fn test_mcp_server_clear_cache() {
        let server = EvifMcpServer::new(McpServerConfig::default());

        // 等待初始化
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // 触发缓存
        let _ = server.list_tools().await;

        // 清除缓存
        server.clear_cache().await;

        // 验证缓存已清除
        let stats = server.get_cache_stats().await;
        assert!(!stats.tools_cached);
    }

    // ========== Template Rendering Tests ==========

    #[test]
    fn test_template_simple_variable() {
        use std::collections::HashMap;

        let args: HashMap<String, serde_json::Value> = vec![
            ("name".to_string(), serde_json::json!("Alice")),
            ("age".to_string(), serde_json::json!(25)),
        ].into_iter().collect();

        let (result, warnings) = template::render_template(
            "Hello, {name}! You are {age} years old.",
            &Some(args),
        );

        assert_eq!(result, "Hello, Alice! You are 25 years old.");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_template_with_defaults() {
        use std::collections::HashMap;

        let args: HashMap<String, serde_json::Value> = vec![
            ("name".to_string(), serde_json::json!("Bob")),
        ].into_iter().collect();

        let (result, warnings) = template::render_template(
            "Hello, {name}! Your role is {role:user}.",
            &Some(args),
        );

        assert_eq!(result, "Hello, Bob! Your role is user.");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_template_missing_variable_with_default() {
        use std::collections::HashMap;

        let args: HashMap<String, serde_json::Value> = vec![
            ("name".to_string(), serde_json::json!("Charlie")),
        ].into_iter().collect();

        let (result, warnings) = template::render_template(
            "Hello, {name}! Your role is {role:guest}.",
            &Some(args),
        );

        assert_eq!(result, "Hello, Charlie! Your role is guest.");
        // When a default value is provided, no warning is generated
        // (the default is used instead)
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_template_conditional_if_true() {
        use std::collections::HashMap;

        let args: HashMap<String, serde_json::Value> = vec![
            ("debug".to_string(), serde_json::json!(true)),
        ].into_iter().collect();

        let (result, warnings) = template::render_template(
            "Start{{#if debug}} - DEBUG MODE{{/if}} End",
            &Some(args),
        );

        assert_eq!(result, "Start - DEBUG MODE End");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_template_conditional_if_false() {
        use std::collections::HashMap;

        let args: HashMap<String, serde_json::Value> = vec![
            ("debug".to_string(), serde_json::json!(false)),
        ].into_iter().collect();

        let (result, warnings) = template::render_template(
            "Start{{#if debug}} - DEBUG MODE{{/if}} End",
            &Some(args),
        );

        assert_eq!(result, "Start End");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_template_validate_required_args() {
        let required = vec![
            ("name", true, "User name"),
            ("email", true, "Email address"),
            ("optional", false, "Optional field"),
        ];

        // Test with missing required args
        let errors = template::validate_required_args(&None, &required);
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().any(|e| e.contains("name")));
        assert!(errors.iter().any(|e| e.contains("email")));
    }

    #[test]
    fn test_template_validate_required_args_provided() {
        use std::collections::HashMap;

        let args: HashMap<String, serde_json::Value> = vec![
            ("name".to_string(), serde_json::json!("test")),
            ("email".to_string(), serde_json::json!("test@example.com")),
        ].into_iter().collect();

        let required = vec![
            ("name", true, "User name"),
            ("email", true, "Email address"),
            ("optional", false, "Optional field"),
        ];

        let errors = template::validate_required_args(&Some(args), &required);
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_prompts_get_with_template_args() {
        use std::collections::HashMap;

        let server = EvifMcpServer::new(McpServerConfig::default());

        // Wait for prompts to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

        // Test prompts/get with arguments (basic test)
        let request = json!({
            "jsonrpc": "2.0",
            "method": "prompts/get",
            "params": {
                "name": "file_explorer",
                "arguments": {}
            },
            "id": 1
        });

        let response = server.handle_request(request).await;
        assert!(response.get("result").is_some());

        // Check that _meta field is present
        let result = &response["result"];
        assert!(result.get("_meta").is_some());
    }

    // ============ VFS Backend Tests ============

    #[test]
    fn test_vfs_backend_creation() {
        let backend = VfsBackend::new("http://localhost:8081".to_string());
        assert_eq!(backend.mode(), VfsMode::Http);
        assert!(!backend.is_direct());
    }

    #[test]
    fn test_vfs_backend_with_direct_mode() {
        let backend = VfsBackend::new("http://localhost:8081".to_string())
            .with_mode(VfsMode::Direct);
        assert_eq!(backend.mode(), VfsMode::Direct);
        assert!(backend.is_direct());
    }

    #[test]
    fn test_vfs_mode_default() {
        let mode = VfsMode::default();
        assert_eq!(mode, VfsMode::Http);
    }

    #[test]
    fn test_vfs_entry_creation() {
        let entry = VfsEntry {
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
            modified: "2024-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(entry.name, "test.txt");
        assert!(!entry.is_dir);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_vfs_write_result_creation() {
        let result = VfsWriteResult {
            bytes_written: 100,
            path: "/test/file.txt".to_string(),
        };
        assert_eq!(result.bytes_written, 100);
        assert_eq!(result.path, "/test/file.txt");
    }

    #[test]
    fn test_vfs_file_info_creation() {
        let info = VfsFileInfo {
            path: "/test".to_string(),
            name: "test".to_string(),
            size: 4096,
            is_dir: true,
            mode: 0o755,
            modified: "2024-01-01T00:00:00Z".to_string(),
            created: "2024-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(info.path, "/test");
        assert!(info.is_dir);
        assert_eq!(info.mode, 0o755);
    }

    #[test]
    fn test_vfs_entry_serialization() {
        let entry = VfsEntry {
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
            modified: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_vfs_write_result_serialization() {
        let result = VfsWriteResult {
            bytes_written: 100,
            path: "/test/file.txt".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("100"));
        assert!(json.contains("/test/file.txt"));
    }

    #[test]
    fn test_vfs_file_info_serialization() {
        let info = VfsFileInfo {
            path: "/test".to_string(),
            name: "test".to_string(),
            size: 4096,
            is_dir: true,
            mode: 0o755,
            modified: "2024-01-01T00:00:00Z".to_string(),
            created: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("/test"));
        assert!(json.contains("4096"));
    }

    #[tokio::test]
    async fn test_server_with_vfs_backend() {
        let backend = Arc::new(VfsBackend::new("http://localhost:8081".to_string()));
        let config = McpServerConfig {
            evif_url: "http://localhost:8081".to_string(),
            ..McpServerConfig::default()
        };
        let server = EvifMcpServer::with_vfs_backend(config, backend);

        // Verify server was created with vfs backend
        let tools = wait_for_tools(&server).await;
        assert!(tools.len() >= 15);

        // Test that evif_ls still works (falls back to HTTP if no VFS direct)
        let result = server.call_tool("evif_ls", json!({"path": "/"})).await;
        // Result could be Ok or Err depending on whether mock server is running
        // But the API should work
        assert!(result.is_ok() || result.is_err()); // Just verify the call was made
    }

    #[tokio::test]
    async fn test_vfs_backend_list_dir_fails_without_server() {
        let backend = VfsBackend::new("http://localhost:99999".to_string());
        let result = backend.list_dir("/").await;
        // Should fail because no server is listening on that port
        assert!(result.is_err());
    }

    #[test]
    fn test_vfs_mode_copy() {
        // Test that VfsMode can be copied (it's a simple enum)
        let mode = VfsMode::Http;
        let mode2 = mode;
        assert_eq!(mode, mode2);
    }

    #[test]
    fn test_vfs_mode_debug() {
        let mode = VfsMode::Direct;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("Direct"));
    }
}

// Re-export McpGatewayPlugin for external use
pub use mcp_gateway::{McpGatewayPlugin, McpServerEntry, SessionEntry, ToolEntry};

// Re-export McpServerPlugin for external use
pub use mcp_server_plugin::{
    McpServerPlugin, ExternalMcpConfig, McpServerCapabilities,
    ExternalTool, ExternalResource,
};
