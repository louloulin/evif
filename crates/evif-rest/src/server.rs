// REST API 服务器
//
// Phase 7.2: 支持从配置文件或环境变量 EVIF_CONFIG / EVIF_MOUNTS 读取挂载列表
// Phase 7.3: 支持动态 .so 插件加载（对标 AGFS PluginFactory）

use crate::{RestResult, RestError, create_routes, LoggingMiddleware};
use axum::middleware;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use evif_core::{RadixMountTable, EvifPlugin, DynamicPluginLoader};
use evif_plugins::{MemFsPlugin, LocalFsPlugin, HelloFsPlugin};
use std::sync::Arc;
use std::path::Path;

/// 全局动态插件加载器（单例）
static DYNAMIC_LOADER: OnceLock<Arc<DynamicPluginLoader>> = OnceLock::new();
use std::sync::OnceLock;

/// 单条挂载配置（与 evif.json / EVIF_MOUNTS 一致）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MountConfigEntry {
    pub path: String,
    pub plugin: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// 从配置创建插件实例（与 handlers::mount 逻辑一致）
/// 支持动态加载 .so 插件（对标 AGFS PluginFactory）
fn create_plugin_from_config(plugin: &str, config: Option<&serde_json::Value>) -> Arc<dyn EvifPlugin> {
    let name = plugin.to_lowercase();
    match name.as_str() {
        "mem" | "memfs" => Arc::new(MemFsPlugin::new()),
        "hello" | "hellofs" => Arc::new(HelloFsPlugin::new()),
        "local" | "localfs" => {
            let root = config
                .and_then(|c| c.get("root"))
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp/evif-local")
                .to_string();
            Arc::new(LocalFsPlugin::new(&root))
        }
        _ => {
            // 尝试从动态库加载
            info!("Attempting to load plugin '{}' from dynamic library", plugin);

            // 初始化全局动态加载器
            let loader = DYNAMIC_LOADER.get_or_init(|| {
                info!("Initializing dynamic plugin loader");
                Arc::new(DynamicPluginLoader::new())
            });

            // 尝试加载插件库
            match loader.load_plugin(&name) {
                Ok(info) => {
                    info!("Loaded dynamic plugin: {} v{}", info.name(), info.version());
                    match loader.create_plugin(&name) {
                        Ok(plugin) => {
                            info!("Successfully created dynamic plugin instance: {}", plugin.name());
                            return plugin;
                        }
                        Err(e) => {
                            warn!("Failed to create dynamic plugin instance: {}, falling back to MemFS", e);
                            Arc::new(MemFsPlugin::new())
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load dynamic plugin '{}': {}, falling back to MemFS", plugin, e);
                    Arc::new(MemFsPlugin::new())
                }
            }
        }
    }
}

/// 加载挂载配置：优先 EVIF_CONFIG 文件，其次 EVIF_MOUNTS 环境变量（JSON/YAML/TOML 数组），否则返回默认列表
/// 支持的文件格式：JSON (.json), YAML (.yaml/.yml), TOML (.toml)
fn load_mount_config() -> Vec<MountConfigEntry> {
    // 1. 环境变量 EVIF_CONFIG 指定配置文件路径（支持 JSON/YAML/TOML）
    if let Ok(path) = std::env::var("EVIF_CONFIG") {
        if let Ok(entries) = load_config_file(&path) {
            info!("Loaded {} mounts from EVIF_CONFIG={}", entries.len(), path);
            return entries;
        }
    }

    // 2. 环境变量 EVIF_MOUNTS 为 JSON/YAML/TOML 数组字符串
    if let Ok(json_str) = std::env::var("EVIF_MOUNTS") {
        if let Ok(entries) = parse_mounts_from_string(&json_str) {
            info!("Loaded {} mounts from EVIF_MOUNTS", entries.len());
            return entries;
        }
    }

    // 3. 当前目录下的配置文件（按优先级：evif.json > evif.yaml > evif.yml > evif.toml）
    for name in ["evif.json", "evif.yaml", "evif.yml", "evif.toml", "./evif.json", "./evif.yaml", "./evif.yml", "./evif.toml"] {
        if Path::new(name).exists() {
            if let Ok(entries) = load_config_file(name) {
                info!("Loaded {} mounts from {}", entries.len(), name);
                return entries;
            }
        }
    }

    // 4. 默认挂载（与原先写死行为一致）
    info!("Using default mount config (no EVIF_CONFIG/EVIF_MOUNTS/evif config files)");
    vec![
        MountConfigEntry { path: "/mem".to_string(), plugin: "mem".to_string(), config: None },
        MountConfigEntry { path: "/hello".to_string(), plugin: "hello".to_string(), config: None },
        MountConfigEntry {
            path: "/local".to_string(),
            plugin: "local".to_string(),
            config: Some(serde_json::json!({ "root": "/tmp/evif-local" })),
        },
    ]
}

/// 从文件加载配置（自动检测格式：JSON/YAML/TOML）
fn load_config_file(path: &str) -> Result<Vec<MountConfigEntry>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    parse_mounts_from_string(&content)
}

/// 从字符串解析挂载配置（支持 JSON/YAML/TOML）
fn parse_mounts_from_string(content: &str) -> Result<Vec<MountConfigEntry>, String> {
    // 尝试 JSON
    if let Ok(entries) = serde_json::from_str::<Vec<MountConfigEntry>>(content) {
        return Ok(entries);
    }

    // 尝试 JSON（对象形式：{ mounts: [...] }）
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(arr) = parsed.get("mounts").and_then(|m| m.as_array()) {
            let entries: Result<Vec<MountConfigEntry>, _> = arr.iter()
                .map(|v| serde_json::from_value(v.clone()))
                .collect();
            if let Ok(entries) = entries {
                return Ok(entries);
            }
        }
    }

    // 尝试 YAML
    if let Ok(entries) = serde_yaml::from_str::<Vec<MountConfigEntry>>(content) {
        return Ok(entries);
    }

    // 尝试 YAML（对象形式：{ mounts: [...] }）
    if let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        if let Some(arr) = parsed.get("mounts") {
            if let Some(arr) = arr.as_sequence() {
                let entries: Result<Vec<MountConfigEntry>, _> = arr.iter()
                    .map(|v| serde_yaml::from_value(v.clone()))
                    .collect();
                if let Ok(entries) = entries {
                    return Ok(entries);
                }
            }
        }
    }

    // 尝试 TOML
    if let Ok(entries) = toml::from_str::<Vec<MountConfigEntry>>(content) {
        return Ok(entries);
    }

    // 尝试 TOML（表形式：[mounts]）
    if let Ok(parsed) = toml::from_str::<toml::Value>(content) {
        if let Some(arr) = parsed.get("mounts") {
            if let Some(arr) = arr.as_array() {
                let entries: Result<Vec<MountConfigEntry>, _> = arr.iter()
                    .map(|v| serde::Deserialize::deserialize(v.clone()))
                    .collect();
                if let Ok(entries) = entries {
                    return Ok(entries);
                }
            }
        }
    }

    Err("Failed to parse config: unsupported format (tried JSON, YAML, TOML)".to_string())
}

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 绑定地址
    pub bind_addr: String,

    /// 端口
    pub port: u16,

    /// 启用 CORS
    pub enable_cors: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0".to_string(),
            port: 8081,
            enable_cors: true,
        }
    }
}

/// EVIF REST 服务器
pub struct EvifServer {
    config: ServerConfig,
}

impl EvifServer {
    /// 创建新服务器
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    /// 启动服务器（Phase 7.2: 从配置或默认挂载加载）
    pub async fn run(self) -> RestResult<()> {
        let mount_table = Arc::new(RadixMountTable::new());
        let mounts = load_mount_config();

        info!("Loading plugins ({} mount(s))...", mounts.len());
        for entry in mounts {
            let plugin = create_plugin_from_config(&entry.plugin, entry.config.as_ref());
            let path = entry.path.clone();
            mount_table.mount(path.clone(), plugin).await
                .map_err(|e| RestError::Internal(format!("Failed to mount {} at {}: {}", entry.plugin, path, e)))?;
            info!("✓ Mounted {} at {}", entry.plugin, path);
        }
        info!("All plugins loaded successfully");

        let app = create_routes(mount_table).layer(middleware::from_fn(LoggingMiddleware));

        let addr = format!("{}:{}", self.config.bind_addr, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("EVIF REST API listening on http://{}", addr);

        axum::serve(listener, app.layer(TraceLayer::new_for_http())).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0");
        assert_eq!(config.port, 8081);
        assert!(config.enable_cors);
    }

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = EvifServer::new(config);
        assert_eq!(server.config.bind_addr, "0.0.0.0");
    }

    #[test]
    fn test_parse_mounts_json_array() {
        let json = r#"[
            {"path": "/test1", "plugin": "mem", "config": null},
            {"path": "/test2", "plugin": "local", "config": {"root": "/tmp/test"}}
        ]"#;
        let entries = parse_mounts_from_string(json).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/test1");
        assert_eq!(entries[0].plugin, "mem");
        assert_eq!(entries[1].path, "/test2");
        assert_eq!(entries[1].plugin, "local");
    }

    #[test]
    fn test_parse_mounts_json_object() {
        let json = r#"{
            "mounts": [
                {"path": "/test", "plugin": "mem"}
            ]
        }"#;
        let entries = parse_mounts_from_string(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/test");
    }

    #[test]
    fn test_parse_mounts_yaml_array() {
        let yaml = r#"
- path: /test1
  plugin: mem
- path: /test2
  plugin: local
  config:
    root: /tmp/test
"#;
        let entries = parse_mounts_from_string(yaml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/test1");
        assert_eq!(entries[1].path, "/test2");
    }

    #[test]
    fn test_parse_mounts_yaml_object() {
        let yaml = r#"
mounts:
  - path: /test
    plugin: mem
"#;
        let entries = parse_mounts_from_string(yaml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/test");
    }

    #[test]
    fn test_parse_mounts_toml_array() {
        let toml = r#"
[[mounts]]
path = "/test1"
plugin = "mem"

[[mounts]]
path = "/test2"
plugin = "local"
config = { root = "/tmp/test" }
"#;
        let entries = parse_mounts_from_string(toml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/test1");
        assert_eq!(entries[1].path, "/test2");
    }

    #[test]
    fn test_parse_mounts_invalid_format() {
        let invalid = "not a valid config";
        let result = parse_mounts_from_string(invalid);
        assert!(result.is_err());
    }
}
