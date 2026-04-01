// REST API 服务器
//
// Phase 7.2: 支持从配置文件或环境变量 EVIF_CONFIG / EVIF_MOUNTS 读取挂载列表
// Phase 7.3: 支持动态 .so 插件加载（对标 AGFS PluginFactory）

use crate::{
    create_memory_state_from_env, validate_memory_for_production, LoggingMiddleware,
    RestAuthState, RestError, RestResult,
};
use axum::middleware;
use evif_core::{
    validate_and_initialize_plugin, DynamicPluginLoader, EvifError, EvifPlugin, RadixMountTable,
};
use evif_plugins::{
    normalize_plugin_id, ContextFsPlugin, DevFsPlugin, HeartbeatFsPlugin, HelloFsPlugin,
    HttpFsPlugin, KvfsPlugin, LocalFsPlugin, MemFsPlugin, PipeFsPlugin, ProxyFsPlugin,
    QueueFsPlugin, ServerInfoFsPlugin, SkillFsPlugin, SqlfsConfig, SqlfsPlugin, StreamFsPlugin,
};
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

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
    /// 实例名称（可选，用于多实例区分，默认使用插件名）
    #[serde(default)]
    pub instance_name: Option<String>,
}

/// 从配置创建插件实例（与 handlers::mount 逻辑一致）
/// 支持动态加载 .so 插件（对标 AGFS PluginFactory）
pub(crate) async fn create_plugin_from_config(
    plugin: &str,
    config: Option<&serde_json::Value>,
) -> Result<Arc<dyn EvifPlugin>, EvifError> {
    let normalized = normalize_plugin_id(plugin);
    let plugin_instance: Arc<dyn EvifPlugin> = if let Some(plugin_instance) =
        create_builtin_plugin_from_config(&normalized, config)?
    {
        plugin_instance
    } else {
        info!(
            "Attempting to load plugin '{}' from dynamic library",
            plugin
        );

        let loader = DYNAMIC_LOADER.get_or_init(|| {
            info!("Initializing dynamic plugin loader");
            Arc::new(DynamicPluginLoader::new())
        });

        match loader.load_plugin(&normalized) {
            Ok(info) => {
                info!("Loaded dynamic plugin: {} v{}", info.name(), info.version());
                match loader.create_plugin(&normalized) {
                    Ok(plugin) => {
                        info!(
                            "Successfully created dynamic plugin instance: {}",
                            plugin.name()
                        );
                        plugin
                    }
                    Err(e) => {
                        return Err(EvifError::PluginLoadError(format!(
                            "Failed to create dynamic plugin instance '{}': {}",
                            plugin, e
                        )));
                    }
                }
            }
            Err(e) => {
                return Err(EvifError::PluginLoadError(format!(
                    "Failed to load dynamic plugin '{}': {}",
                    plugin, e
                )));
            }
        }
    };

    validate_and_initialize_plugin(plugin_instance.as_ref(), config).await?;
    Ok(plugin_instance)
}

pub(crate) fn create_builtin_plugin_from_config(
    plugin: &str,
    config: Option<&serde_json::Value>,
) -> Result<Option<Arc<dyn EvifPlugin>>, EvifError> {
    let normalized = normalize_plugin_id(plugin);
    let plugin_instance: Arc<dyn EvifPlugin> = match normalized.as_str() {
        "contextfs" => Arc::new(ContextFsPlugin::new()),
        "skillfs" => Arc::new(SkillFsPlugin::new()),
        "pipefs" => Arc::new(PipeFsPlugin::new()),
        "memfs" => Arc::new(MemFsPlugin::new()),
        "hellofs" => Arc::new(HelloFsPlugin::new()),
        "localfs" => {
            let root = config
                .and_then(|c| c.get("root"))
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp/evif-local");
            Arc::new(LocalFsPlugin::new(root))
        }
        "kvfs" => {
            let prefix = config
                .and_then(|c| c.get("prefix"))
                .and_then(|v| v.as_str())
                .unwrap_or("evif");
            Arc::new(KvfsPlugin::new(prefix))
        }
        "queuefs" => Arc::new(QueueFsPlugin::new()),
        "sqlfs2" => {
            let mut sql_config = SqlfsConfig::default();
            sql_config.db_path = config
                .and_then(|c| c.get("db_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp/evif-sqlfs2.db")
                .to_string();
            sql_config.cache_enabled = config
                .and_then(|c| c.get("cache_enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(sql_config.cache_enabled);
            sql_config.cache_max_size = config
                .and_then(|c| c.get("cache_max_size"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(sql_config.cache_max_size);
            sql_config.cache_ttl_seconds = config
                .and_then(|c| c.get("cache_ttl_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(sql_config.cache_ttl_seconds);
            Arc::new(SqlfsPlugin::new(sql_config)?)
        }
        "streamfs" => Arc::new(StreamFsPlugin::new()),
        "heartbeatfs" => Arc::new(HeartbeatFsPlugin::new()),
        "proxyfs" => {
            let base_url = config
                .and_then(|c| c.get("base_url"))
                .and_then(|v| v.as_str())
                .unwrap_or("http://127.0.0.1:8081/api/v1");
            Arc::new(ProxyFsPlugin::new(base_url))
        }
        "serverinfofs" => {
            let version = config
                .and_then(|c| c.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or(env!("CARGO_PKG_VERSION"));
            Arc::new(ServerInfoFsPlugin::new(version))
        }
        "devfs" => Arc::new(DevFsPlugin::new()),
        "httpfs" => {
            let base_url = config
                .and_then(|c| c.get("base_url"))
                .and_then(|v| v.as_str())
                .unwrap_or("https://example.invalid");
            let timeout_seconds = config
                .and_then(|c| c.get("timeout_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(30);
            Arc::new(HttpFsPlugin::new(base_url, timeout_seconds))
        }
        _ => return Ok(None),
    };

    Ok(Some(plugin_instance))
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
    for name in [
        "evif.json",
        "evif.yaml",
        "evif.yml",
        "evif.toml",
        "./evif.json",
        "./evif.yaml",
        "./evif.yml",
        "./evif.toml",
    ] {
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
        MountConfigEntry {
            path: "/mem".to_string(),
            plugin: "mem".to_string(),
            config: None,
            instance_name: None,
        },
        MountConfigEntry {
            path: "/hello".to_string(),
            plugin: "hello".to_string(),
            config: None,
            instance_name: None,
        },
        MountConfigEntry {
            path: "/local".to_string(),
            plugin: "local".to_string(),
            config: Some(serde_json::json!({ "root": "/tmp/evif-local" })),
            instance_name: None,
        },
    ]
}

/// 从文件加载配置（自动检测格式：JSON/YAML/TOML）
fn load_config_file(path: &str) -> Result<Vec<MountConfigEntry>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;

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
            let entries: Result<Vec<MountConfigEntry>, _> = arr
                .iter()
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
                let entries: Result<Vec<MountConfigEntry>, _> = arr
                    .iter()
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
                let entries: Result<Vec<MountConfigEntry>, _> = arr
                    .iter()
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

    /// CORS allowed origins (empty = allow all when enable_cors is true)
    pub cors_origins: Vec<String>,

    /// Production mode: strict config checks
    pub production_mode: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let production_mode = std::env::var("EVIF_REST_PRODUCTION_MODE")
            .map(|v| v.trim().to_ascii_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        Self {
            bind_addr: "0.0.0.0".to_string(),
            port: 8081,
            enable_cors: std::env::var("EVIF_CORS_ENABLED")
                .map(|v| v.trim().to_ascii_lowercase() != "false" && v != "0")
                .unwrap_or(true),
            cors_origins: std::env::var("EVIF_CORS_ORIGINS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default(),
            production_mode,
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
        let memory_config = crate::memory_handlers::MemoryBackendConfig::from_env()
            .map_err(|e| RestError::Internal(e))?;

        // Validate memory backend for production mode
        if let Err(e) = validate_memory_for_production(&memory_config) {
            return Err(RestError::Internal(e));
        }

        let memory_state = create_memory_state_from_env().map_err(RestError::Internal)?;

        info!("Loading plugins ({} mount(s))...", mounts.len());
        for entry in mounts {
            let plugin = create_plugin_from_config(&entry.plugin, entry.config.as_ref())
                .await
                .map_err(|e| {
                    RestError::Internal(format!(
                        "Failed to prepare plugin '{}' for '{}': {}",
                        entry.plugin, entry.path, e
                    ))
                })?;
            let path = entry.path.clone();
            let plugin_name = entry.plugin.clone();
            let instance_name = entry.instance_name.clone().unwrap_or_else(|| plugin_name.clone());
            mount_table
                .mount_with_metadata(path.clone(), plugin, plugin_name.clone(), instance_name)
                .await
                .map_err(|e| {
                    RestError::Internal(format!(
                        "Failed to mount {} at {}: {}",
                        entry.plugin, path, e
                    ))
                })?;
            info!("✓ Mounted {} at {}", entry.plugin, path);
        }
        info!("All plugins loaded successfully");
        info!(
            "Configured REST memory backend: {}",
            memory_state.backend_name()
        );

        let mut app = crate::routes::create_routes_with_auth_and_memory_state(
            mount_table,
            Arc::new(RestAuthState::from_env()),
            memory_state,
        )
        .layer(middleware::from_fn(crate::middleware::PathValidationMiddleware))
        .layer(middleware::from_fn(LoggingMiddleware));

        // Apply CORS configuration
        if self.config.enable_cors {
            use tower_http::cors::{CorsLayer, Any};
            let cors = if self.config.cors_origins.is_empty() {
                // No specific origins configured: allow all
                if self.config.production_mode {
                    warn!("CORS enabled in production mode with no origin restrictions - consider setting EVIF_CORS_ORIGINS");
                }
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::PATCH])
                    .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION, axum::http::HeaderName::from_static("x-api-key"), axum::http::HeaderName::from_static("x-evif-api-key")])
                    .max_age(std::time::Duration::from_secs(3600))
            } else {
                // Specific origins configured
                let origins: Vec<_> = self.config.cors_origins.iter()
                    .filter_map(|o| o.parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::PATCH])
                    .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION, axum::http::HeaderName::from_static("x-api-key"), axum::http::HeaderName::from_static("x-evif-api-key")])
                    .max_age(std::time::Duration::from_secs(3600))
            };
            app = app.layer(cors);
            info!("CORS enabled (origins: {})", if self.config.cors_origins.is_empty() { "any".to_string() } else { self.config.cors_origins.join(", ") });
        } else {
            info!("CORS disabled");
        }

        let addr = format!("{}:{}", self.config.bind_addr, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("EVIF REST API listening on http://{}", addr);

        let graceful_timeout = std::env::var("EVIF_SHUTDOWN_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30u64);

        // Build the graceful shutdown signal
        let shutdown_signal = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
            info!("Received SIGINT/Ctrl+C, shutting down gracefully...");
        };

        // Run with graceful shutdown
        axum::serve(listener, app.layer(TraceLayer::new_for_http()))
            .with_graceful_shutdown(shutdown_signal)
            .await?;

        info!("EVIF REST API shutdown complete");

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

    #[tokio::test]
    async fn test_create_plugin_from_config_unknown_plugin_returns_error() {
        let result = create_plugin_from_config("definitely-missing-plugin", None).await;
        assert!(result.is_err());
        let err = result.err().expect("unknown plugin should return error");
        assert!(err.to_string().contains("definitely-missing-plugin"));
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
