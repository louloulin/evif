// REST API 服务器
//
// Phase 7.2: 支持从配置文件或环境变量 EVIF_CONFIG / EVIF_MOUNTS 读取挂载列表
// Phase 7.3: 支持动态 .so 插件加载（对标 AGFS PluginFactory）

use crate::{
    create_memory_state_from_env, validate_memory_for_production, RestAuthState, RestError,
    RestResult,
};
use crate::{
    middleware::{
        concurrency_limit_middleware, panic_catcher, timeout_middleware, LoggingMiddleware,
    },
    routes::mark_server_ready,
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
    let plugin_instance: Arc<dyn EvifPlugin> =
        if let Some(plugin_instance) = create_builtin_plugin_from_config(&normalized, config)? {
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
        // Phase 8-9: Context Engine 默认挂载
        MountConfigEntry {
            path: "/context".to_string(),
            plugin: "contextfs".to_string(),
            config: None,
            instance_name: None,
        },
        // Phase 9: SkillFS 默认挂载
        MountConfigEntry {
            path: "/skills".to_string(),
            plugin: "skillfs".to_string(),
            config: None,
            instance_name: None,
        },
        // Phase 9: PipeFS 默认挂载
        MountConfigEntry {
            path: "/pipes".to_string(),
            plugin: "pipefs".to_string(),
            config: None,
            instance_name: None,
        },
    ]
}

fn validate_runtime_state_for_production(production_mode: bool) -> Result<(), String> {
    if !production_mode {
        return Ok(());
    }

    let required_vars = [
        (
            "EVIF_REST_TENANT_STATE_PATH",
            "tenant state must persist across restarts in production",
        ),
        (
            "EVIF_REST_SYNC_STATE_PATH",
            "sync state must persist across restarts in production",
        ),
        (
            "EVIF_REST_ENCRYPTION_STATE_PATH",
            "encryption state must persist across restarts in production",
        ),
    ];

    let missing: Vec<String> = required_vars
        .iter()
        .filter_map(|(name, reason)| match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => None,
            _ => Some(format!("{} ({})", name, reason)),
        })
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "EVIF_REST_PRODUCTION_MODE requires persistent runtime state paths: {}",
            missing.join(", ")
        ))
    }
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

/// N0: TLS configuration for HTTPS server
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to TLS certificate file (PEM format)
    pub cert_path: String,
    /// Path to TLS private key file (PEM format)
    pub key_path: String,
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

    /// TLS configuration (when EVIF_TLS_CERT_FILE + EVIF_TLS_KEY_FILE are set)
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let production_mode = std::env::var("EVIF_REST_PRODUCTION_MODE")
            .map(|v| v.trim().eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        // N0: TLS — read from env vars if both cert and key are provided
        let tls = Self::tls_from_env();

        Self {
            // CLI args take precedence; fall back to EVIF_REST_PORT / EVIF_REST_HOST
            bind_addr: std::env::var("EVIF_REST_HOST")
                .or_else(|_| std::env::var("EVIF_HOST"))
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("EVIF_REST_PORT")
                .or_else(|_| std::env::var("EVIF_PORT"))
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8081),
            enable_cors: std::env::var("EVIF_CORS_ENABLED")
                .map(|v| !v.trim().eq_ignore_ascii_case("false") && v != "0")
                .unwrap_or(true),
            cors_origins: std::env::var("EVIF_CORS_ORIGINS")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            production_mode,
            tls,
        }
    }
}

impl ServerConfig {
    /// N0: Load TLS configuration from environment variables.
    /// Requires both EVIF_TLS_CERT_FILE and EVIF_TLS_KEY_FILE.
    fn tls_from_env() -> Option<TlsConfig> {
        let cert_path = std::env::var("EVIF_TLS_CERT_FILE").ok()?;
        let key_path = std::env::var("EVIF_TLS_KEY_FILE").ok()?;
        if cert_path.is_empty() || key_path.is_empty() {
            return None;
        }
        Some(TlsConfig {
            cert_path,
            key_path,
        })
    }

    /// Returns true if TLS is configured and HTTPS server should be used.
    pub fn is_tls_enabled(&self) -> bool {
        self.tls.is_some()
    }

    /// N0: Build a rustls ServerConfig from this ServerConfig's TLS settings.
    /// Panics if TLS is not configured — check is_tls_enabled() first.
    pub fn build_rustls_config(&self) -> std::io::Result<rustls::ServerConfig> {
        use rustls_pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};

        let tls = self
            .tls
            .as_ref()
            .expect("TLS not configured — call is_tls_enabled() first");

        let cert_bytes = std::fs::read(&tls.cert_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("TLS cert not found at '{}': {}", tls.cert_path, e),
            )
        })?;
        let key_bytes = std::fs::read(&tls.key_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("TLS key not found at '{}': {}", tls.key_path, e),
            )
        })?;

        let cert = CertificateDer::from(cert_bytes);
        let key_pem = PrivateKeyDer::<'static>::from_pem_slice(&key_bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid TLS private key (must be PEM): {}", e),
            )
        })?;

        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key_pem)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("TLS certificate/key mismatch: {}", e),
                )
            })?;

        // Advertise HTTP/1.1 via ALPN
        config.alpn_protocols = vec![b"http/1.1".to_vec()];

        Ok(config)
    }

    /// Create a config from CLI args, falling back to env vars and defaults.
    /// CLI args take highest precedence, then env vars, then hardcoded defaults.
    pub fn from_cli(
        host: Option<String>,
        port: Option<u16>,
        production: bool,
        tls_cert: Option<String>,
        tls_key: Option<String>,
    ) -> Self {
        let mut base = Self::default();
        if let Some(h) = host {
            base.bind_addr = h;
        }
        if let Some(p) = port {
            base.port = p;
        }
        if production {
            base.production_mode = true;
        }
        if let (Some(cert), Some(key)) = (tls_cert, tls_key) {
            if !cert.is_empty() && !key.is_empty() {
                base.tls = Some(TlsConfig {
                    cert_path: cert,
                    key_path: key,
                });
            }
        }
        base
    }

    /// Returns the TLS/HTTPS port. Defaults to 8443, or EVIF_TLS_PORT env var.
    pub fn tls_port(&self) -> u16 {
        std::env::var("EVIF_TLS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8443)
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
        let memory_config =
            crate::memory_handlers::MemoryBackendConfig::from_env().map_err(RestError::Internal)?;

        // Validate memory backend for production mode
        if let Err(e) = validate_memory_for_production(&memory_config) {
            return Err(RestError::Internal(e));
        }
        if let Err(e) = validate_runtime_state_for_production(self.config.production_mode) {
            return Err(RestError::Internal(e));
        }

        let memory_state = create_memory_state_from_env()
            .await
            .map_err(RestError::Internal)?;

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
            let instance_name = entry
                .instance_name
                .clone()
                .unwrap_or_else(|| plugin_name.clone());
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
            memory_state.backend_description()
        );

        // N5 + N8 + N10: Middleware chain — outermost (panic catcher) to innermost (logging)
        let mut app = crate::routes::create_routes_with_auth_and_memory_state(
            mount_table,
            Arc::new(RestAuthState::from_env()),
            memory_state,
        )
        // N8: Panic catcher — outermost, catches panics from all inner handlers
        .layer(middleware::from_fn(panic_catcher))
        // N5: Concurrency limit — max 256 concurrent requests
        .layer(middleware::from_fn(concurrency_limit_middleware))
        // N8: Request timeout — 30s per request
        .layer(middleware::from_fn(timeout_middleware))
        // N10: Structured request logging
        .layer(middleware::from_fn(LoggingMiddleware))
        // TraceLayer for HTTP observability
        .layer(TraceLayer::new_for_http());

        // Apply CORS configuration
        if self.config.enable_cors {
            use tower_http::cors::{Any, CorsLayer};
            let cors = if self.config.cors_origins.is_empty() {
                // No specific origins configured: allow all
                if self.config.production_mode {
                    warn!("CORS enabled in production mode with no origin restrictions - consider setting EVIF_CORS_ORIGINS");
                }
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::PUT,
                        axum::http::Method::DELETE,
                        axum::http::Method::PATCH,
                    ])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                        axum::http::HeaderName::from_static("x-api-key"),
                        axum::http::HeaderName::from_static("x-evif-api-key"),
                        axum::http::HeaderName::from_static("x-request-id"),
                        axum::http::HeaderName::from_static("x-correlation-id"),
                    ])
                    .expose_headers([
                        axum::http::HeaderName::from_static("x-request-id"),
                        axum::http::HeaderName::from_static("x-correlation-id"),
                    ])
                    .max_age(std::time::Duration::from_secs(3600))
            } else {
                // Specific origins configured
                let origins: Vec<_> = self
                    .config
                    .cors_origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::PUT,
                        axum::http::Method::DELETE,
                        axum::http::Method::PATCH,
                    ])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                        axum::http::HeaderName::from_static("x-api-key"),
                        axum::http::HeaderName::from_static("x-evif-api-key"),
                        axum::http::HeaderName::from_static("x-request-id"),
                        axum::http::HeaderName::from_static("x-correlation-id"),
                    ])
                    .expose_headers([
                        axum::http::HeaderName::from_static("x-request-id"),
                        axum::http::HeaderName::from_static("x-correlation-id"),
                    ])
                    .max_age(std::time::Duration::from_secs(3600))
            };
            app = app.layer(cors);
            info!(
                "CORS enabled (origins: {})",
                if self.config.cors_origins.is_empty() {
                    "any".to_string()
                } else {
                    self.config.cors_origins.join(", ")
                }
            );
        } else {
            info!("CORS disabled");
        }

        let addr = format!("{}:{}", self.config.bind_addr, self.config.port);

        // N9: Build graceful shutdown signal — handles SIGTERM (K8s/Docker/systemd) and SIGINT
        // Use a broadcast channel so both HTTP and HTTPS can share the same shutdown signal
        let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
        let shutdown_signal = async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
                let mut sigint =
                    signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
                tokio::select! {
                    _ = sigterm.recv() => {
                        info!("Received SIGTERM, shutting down gracefully...");
                    }
                    _ = sigint.recv() => {
                        info!("Received SIGINT (Ctrl+C), shutting down gracefully...");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
                info!("Received Ctrl+C, shutting down gracefully...");
            }
        };

        // Spawn shutdown signal receiver — sends () to shutdown_tx on SIGTERM/SIGINT
        let shutdown_sender = shutdown_tx.clone();
        tokio::spawn(async move {
            shutdown_signal.await;
            let _ = shutdown_sender.send(());
        });

        if self.config.is_tls_enabled() {
            // N0: TLS mode — serve HTTPS alongside HTTP
            use hyper_util::rt::TokioExecutor;
            use hyper_util::server::conn::auto::Builder as ConnBuilder;
            use hyper_util::service::TowerToHyperService;
            use tokio_rustls::TlsAcceptor;

            let tls_server_config = self
                .config
                .build_rustls_config()
                .map_err(|e| RestError::Internal(format!("TLS config error: {}", e)))?;
            let tls_config_arc = Arc::new(tls_server_config);

            let tls_port = self.config.tls_port();
            let tls_addr = format!("{}:{}", self.config.bind_addr, tls_port);
            let tls_listener = TcpListener::bind(&tls_addr).await?;

            // Bind HTTP separately
            let http_listener = TcpListener::bind(&addr).await?;
            info!(
                "EVIF REST API listening on http://{} and https://{} (TLS enabled)",
                addr, tls_addr
            );

            // Spawn TLS server task
            let tls_app = app.clone();
            let tls_shutdown_sender = shutdown_tx.clone();
            tokio::spawn(async move {
                tracing::info!("HTTPS server task started on :{}", tls_port);
                let mut shutdown_rx = tls_shutdown_sender.subscribe();
                loop {
                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            tracing::info!("HTTPS server: shutdown signal received");
                            break;
                        }
                        result = tls_listener.accept() => {
                            match result {
                                Ok((stream, _remote_addr)) => {
                                    let acceptor = TlsAcceptor::from(Arc::clone(&tls_config_arc));
                                    let app = tls_app.clone();
                                    tokio::spawn(async move {
                                        match acceptor.accept(stream).await {
                                            Ok(tls_stream) => {
                                                use hyper_util::rt::TokioIo;
                                                let io = TokioIo::new(tls_stream);
                                                let hyper_service = TowerToHyperService::new(app);
                                                let mut builder = ConnBuilder::new(TokioExecutor::new());
                                                let http1_builder = builder.http1();
                                                let conn = http1_builder
                                                    .serve_connection(io, hyper_service);
                                                if let Err(e) = conn.await {
                                                    tracing::error!(error = %e, "TLS connection error");
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!(error = %e, "TLS accept error");
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "TLS listener error");
                                }
                            }
                        }
                    }
                }
            });

            // Run HTTP server with graceful shutdown
            let (http_shutdown_tx, http_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            tokio::spawn(async move {
                let _ = shutdown_tx.subscribe().recv().await;
                let _ = http_shutdown_tx.send(());
            });
            axum::serve(http_listener, app)
                .with_graceful_shutdown(async move {
                    http_shutdown_rx.await.ok();
                })
                .await?;
        } else {
            info!("EVIF REST API listening on http://{}", addr);

            // N9: Run with graceful shutdown — subscribe to broadcast and forward to oneshot
            let (http_shutdown_tx, http_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let mut broadcast_receiver = shutdown_tx.subscribe();
            tokio::spawn(async move {
                let _ = broadcast_receiver.recv().await;
                let _ = http_shutdown_tx.send(());
            });

            let listener = TcpListener::bind(&addr).await?;
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    http_shutdown_rx.await.ok();
                })
                .await?;
        }

        // N12: Config hot-reload — watch config file for changes and log reload signal.
        // Full hot-reload requires Router rebuild; this provides the file-watching foundation.
        // Enable by setting EVIF_CONFIG_FILE=/path/to/evif.toml
        let config_file_path = std::env::var("EVIF_CONFIG_FILE").ok();
        if let Some(ref path) = config_file_path {
            let path_for_spawn = path.clone();
            tokio::spawn(async move {
                use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
                use std::path::PathBuf;

                let watched_path = PathBuf::from(path_for_spawn.as_str());
                if !watched_path.exists() {
                    tracing::warn!(path = %watched_path.display(), "EVIF_CONFIG_FILE does not exist, hot-reload disabled");
                    return;
                }

                // Clone before the closure so we can still use the original in watcher.watch()
                let watched_for_closure = watched_path.clone();

                let mut watcher = match RecommendedWatcher::new(
                    move |res: Result<notify::Event, notify::Error>| {
                        if let Ok(event) = res {
                            if event.kind.is_modify() {
                                tracing::info!(
                                    path = %watched_for_closure.display(),
                                    "Config file changed — EVIF_REST_RELOAD=1 triggers graceful restart"
                                );
                            }
                        }
                    },
                    NotifyConfig::default(),
                ) {
                    Ok(w) => w,
                    Err(e) => {
                        tracing::warn!("Failed to create config watcher: {}", e);
                        return;
                    }
                };

                if let Err(e) = watcher.watch(&watched_path, RecursiveMode::NonRecursive) {
                    tracing::warn!("Failed to watch config file: {}", e);
                    return;
                }

                tracing::info!(path = %watched_path.display(), "Config file hot-reload watcher started");
                // Watcher runs until dropped — keep alive by sleeping
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                }
            });
            info!(
                "N12: Config hot-reload enabled via EVIF_CONFIG_FILE={}",
                path
            );
        }

        // N9: Mark server as ready — all routes registered, listener bound, ready to accept traffic
        mark_server_ready();
        info!("Server marked as ready (GET /api/v1/ready → 200)");

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

    #[test]
    fn test_validate_runtime_state_for_production_env() {
        std::env::remove_var("EVIF_REST_TENANT_STATE_PATH");
        std::env::remove_var("EVIF_REST_SYNC_STATE_PATH");
        std::env::remove_var("EVIF_REST_ENCRYPTION_STATE_PATH");

        let result = validate_runtime_state_for_production(false);
        assert!(
            result.is_ok(),
            "non-production mode should allow in-memory runtime state"
        );

        let result = validate_runtime_state_for_production(true);
        assert!(
            result.is_err(),
            "production mode should require persistent runtime state"
        );
        let error = result.expect_err("missing state paths should fail");
        assert!(error.contains("EVIF_REST_TENANT_STATE_PATH"));
        assert!(error.contains("EVIF_REST_SYNC_STATE_PATH"));
        assert!(error.contains("EVIF_REST_ENCRYPTION_STATE_PATH"));

        std::env::set_var("EVIF_REST_TENANT_STATE_PATH", "/tmp/evif-tenant-state.json");
        std::env::set_var("EVIF_REST_SYNC_STATE_PATH", "/tmp/evif-sync-state.json");
        std::env::set_var(
            "EVIF_REST_ENCRYPTION_STATE_PATH",
            "/tmp/evif-encryption-state.json",
        );

        let result = validate_runtime_state_for_production(true);
        assert!(
            result.is_ok(),
            "production mode should accept persistent runtime state paths"
        );

        std::env::remove_var("EVIF_REST_TENANT_STATE_PATH");
        std::env::remove_var("EVIF_REST_SYNC_STATE_PATH");
        std::env::remove_var("EVIF_REST_ENCRYPTION_STATE_PATH");
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

    #[test]
    fn test_default_mounts_include_contextfs_skillfs_pipefs() {
        // 清除环境变量和配置文件，确保使用默认挂载
        std::env::remove_var("EVIF_CONFIG");
        std::env::remove_var("EVIF_MOUNTS");

        // 测试默认挂载包含 contextfs, skillfs, pipefs
        let mounts = load_mount_config();
        let paths: Vec<&str> = mounts.iter().map(|m| m.path.as_str()).collect();

        assert!(
            paths.contains(&"/context"),
            "Default mounts should include /context"
        );
        assert!(
            paths.contains(&"/skills"),
            "Default mounts should include /skills"
        );
        assert!(
            paths.contains(&"/pipes"),
            "Default mounts should include /pipes"
        );

        // 验证插件名称
        let context_mount = mounts.iter().find(|m| m.path == "/context").unwrap();
        assert_eq!(context_mount.plugin, "contextfs");

        let skills_mount = mounts.iter().find(|m| m.path == "/skills").unwrap();
        assert_eq!(skills_mount.plugin, "skillfs");

        let pipes_mount = mounts.iter().find(|m| m.path == "/pipes").unwrap();
        assert_eq!(pipes_mount.plugin, "pipefs");
    }
}
