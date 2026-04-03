// EVIF配置系统 - 完整的配置文件支持

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// EVIF服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvifConfig {
    /// 服务器配置
    pub server: ServerConfig,

    /// 插件配置
    pub plugins: PluginsConfig,

    /// 缓存配置
    pub cache: CacheConfig,

    /// 日志配置
    pub logging: LoggingConfig,

    /// 安全配置
    pub security: Option<SecurityConfig>,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 监听地址
    pub bind_address: String,

    /// 监听端口
    pub port: u16,

    /// 请求超时(秒)
    pub timeout_secs: u64,

    /// 最大连接数
    pub max_connections: usize,

    /// Worker线程数
    pub worker_threads: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            timeout_secs: 30,
            max_connections: 1000,
            worker_threads: None,
        }
    }
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    /// 插件目录
    pub plugins_dir: String,

    /// 插件配置映射
    pub plugin_configs: HashMap<String, serde_json::Value>,

    /// 自动挂载配置
    pub auto_mount: Vec<MountConfig>,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            plugins_dir: "/usr/local/lib/evif/plugins".to_string(),
            plugin_configs: HashMap::new(),
            auto_mount: Vec::new(),
        }
    }
}

/// 挂载配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    /// 插件名称
    pub plugin: String,

    /// 挂载路径
    pub path: String,

    /// 插件配置
    pub config: Option<serde_json::Value>,
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 是否启用缓存
    pub enabled: bool,

    /// 元数据缓存TTL(秒)
    pub metadata_ttl_secs: u64,

    /// 目录缓存TTL(秒)
    pub directory_ttl_secs: u64,

    /// 最大缓存条目数
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metadata_ttl_secs: 60,
            directory_ttl_secs: 30,
            max_entries: 10000,
        }
    }
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别: trace, debug, info, warn, error
    pub level: String,

    /// 日志格式: json, pretty, compact
    pub format: String,

    /// 是否输出到文件
    pub file: Option<LoggingFileConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file: None,
        }
    }
}

/// 日志文件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingFileConfig {
    /// 日志文件路径
    pub path: String,

    /// 日志轮转
    pub rotation: Option<LogRotationConfig>,
}

/// 日志轮转配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// 最大文件大小(MB)
    pub max_size_mb: usize,

    /// 保留的旧文件数量
    pub max_files: usize,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 是否启用TLS
    pub tls_enabled: bool,

    /// TLS证书路径
    pub cert_path: Option<String>,

    /// TLS私钥路径
    pub key_path: Option<String>,

    /// API密钥(可选)
    pub api_keys: Option<Vec<String>>,

    /// CORS配置
    pub cors: Option<CorsConfig>,
}

/// CORS配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// 允许的来源
    pub allowed_origins: Vec<String>,

    /// 允许的方法
    pub allowed_methods: Vec<String>,

    /// 允许的头部
    pub allowed_headers: Vec<String>,
}

impl EvifConfig {
    /// 从TOML文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: EvifConfig =
            toml::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse TOML: {}", e))?;
        Ok(config)
    }

    /// 从JSON文件加载配置
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: EvifConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
        Ok(config)
    }

    /// 从YAML文件加载配置
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: EvifConfig = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;
        Ok(config)
    }

    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server: ServerConfig {
                bind_address: std::env::var("EVIF_BIND_ADDRESS")
                    .unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("EVIF_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid EVIF_PORT"))?,
                timeout_secs: std::env::var("EVIF_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid EVIF_TIMEOUT"))?,
                max_connections: std::env::var("EVIF_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid EVIF_MAX_CONNECTIONS"))?,
                worker_threads: std::env::var("EVIF_WORKER_THREADS")
                    .ok()
                    .and_then(|s| s.parse().ok()),
            },
            plugins: PluginsConfig::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig {
                level: std::env::var("EVIF_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: std::env::var("EVIF_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string()),
                file: None,
            },
            security: None,
        })
    }

    /// 加载配置(自动检测格式)
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "toml" => Self::from_file(path),
            "json" => Self::from_json_file(path),
            "yaml" | "yml" => Self::from_yaml_file(path),
            _ => Err(anyhow::anyhow!("Unsupported config format: {}", extension)),
        }
    }

    /// 保存配置到TOML文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize TOML: {}", e))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 保存配置到JSON文件
    pub fn save_to_json_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 获取默认配置文件路径
    pub fn default_config_path() -> PathBuf {
        // 按优先级查找配置文件
        let candidates = vec![
            PathBuf::from("./evif.toml"),
            PathBuf::from("./config.toml"),
            PathBuf::from("/etc/evif/config.toml"),
            dirs::home_dir()
                .map(|p| p.join(".config/evif/config.toml"))
                .unwrap_or_else(|| PathBuf::from(".evif.toml")),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return candidate;
            }
        }

        // 如果都不存在，返回默认路径
        PathBuf::from("./evif.toml")
    }

    /// 加载或创建默认配置
    pub fn load_or_default() -> Result<Self> {
        let config_path = Self::default_config_path();
        if config_path.exists() {
            Self::load(&config_path)
        } else {
            // 创建默认配置
            let config = Self::default();
            // 确保目录存在
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            config.save_to_file(&config_path)?;
            println!("Created default config at: {}", config_path.display());
            Ok(config)
        }
    }
}

/// 辅助函数：获取用户主目录
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EvifConfig::default();
        assert_eq!(config.server.port, 8080);
        assert!(config.cache.enabled);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("EVIF_PORT", "9090");
        std::env::set_var("EVIF_LOG_LEVEL", "debug");

        let config = EvifConfig::from_env().unwrap();
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.logging.level, "debug");

        std::env::remove_var("EVIF_PORT");
        std::env::remove_var("EVIF_LOG_LEVEL");
    }

    #[test]
    fn test_serialize_toml() {
        let config = EvifConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("port = 8080"));
    }

    #[test]
    fn test_serialize_json() {
        let config = EvifConfig::default();
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        assert!(json_str.contains("8080"));
    }
}
