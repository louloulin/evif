// Extism-based WASM Plugin Support
//
// 实现基于 extism 的 WASM 插件支持
// 充分复用 extism 的 PDK (Plugin Development Kit) 能力

use crate::error::{EvifError, EvifResult};
use crate::plugin::{EvifPlugin, FileInfo, WriteFlags};
use async_trait::async_trait;
use extism::{Manifest, Plugin, Wasm};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use base64::{engine::general_purpose, Engine as _};

/// 安全配置
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// 内存限制（字节）
    pub memory_limit: u64,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 允许访问的主机列表
    pub allowed_hosts: Vec<String>,
    /// 允许访问的路径列表
    pub allowed_paths: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            memory_limit: 64 * 1024 * 1024, // 64MB
            timeout_ms: 5000,               // 5 seconds
            allowed_hosts: vec![],
            allowed_paths: vec![],
        }
    }
}

impl SecurityConfig {
    /// 创建新的安全配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置内存限制
    pub fn with_memory_limit(mut self, limit: u64) -> Self {
        self.memory_limit = limit;
        self
    }

    /// 设置超时时间
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// 设置允许的主机列表
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_hosts = hosts;
        self
    }

    /// 设置允许的路径列表
    pub fn with_allowed_paths(mut self, paths: Vec<String>) -> Self {
        self.allowed_paths = paths;
        self
    }
}

/// WASM 插件配置
#[derive(Debug, Clone)]
pub struct WasmPluginConfig {
    /// WASM 文件路径
    pub wasm_path: String,
    /// 插件名称
    pub name: String,
    /// 挂载点
    pub mount_point: String,
    /// 配置参数（传递给 WASM 插件）
    pub config: serde_json::Value,
    /// 安全配置
    pub security: SecurityConfig,
}

impl Default for WasmPluginConfig {
    fn default() -> Self {
        Self {
            wasm_path: String::new(),
            name: String::from("wasm_plugin"),
            mount_point: String::from("/"),
            config: serde_json::json!({}),
            security: SecurityConfig::default(),
        }
    }
}

impl WasmPluginConfig {
    /// 创建新的 WASM 插件配置
    pub fn new(wasm_path: impl Into<String>) -> Self {
        Self {
            wasm_path: wasm_path.into(),
            ..Default::default()
        }
    }

    /// 设置插件名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 设置挂载点
    pub fn with_mount_point(mut self, mount_point: impl Into<String>) -> Self {
        self.mount_point = mount_point.into();
        self
    }

    /// 设置配置参数
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    /// 设置安全配置
    pub fn with_security(mut self, security: SecurityConfig) -> Self {
        self.security = security;
        self
    }
}

/// Extism WASM 插件包装器
///
/// 实现 EvifPlugin trait，桥接 extism 运行时和 EVIF 核心
pub struct ExtismPlugin {
    name: String,
    plugin: Arc<Mutex<Plugin>>,
    config: WasmPluginConfig,
}

impl ExtismPlugin {
    /// 创建新的 Extism WASM 插件实例
    ///
    /// # 参数
    /// - `config`: WASM 插件配置
    ///
    /// # 返回
    /// 插件实例
    pub fn new(config: WasmPluginConfig) -> EvifResult<Self> {
        // 检查 WASM 文件是否存在
        let wasm_path = Path::new(&config.wasm_path);
        if !wasm_path.exists() {
            return Err(EvifError::NotFound(format!(
                "WASM file not found: {}",
                config.wasm_path
            )));
        }

        // 创建带安全配置的 Extism Manifest
        let manifest = Manifest::new([Wasm::file(wasm_path)])
            .with_memory_max(config.security.memory_limit as u32)
            .with_timeout(Duration::from_millis(config.security.timeout_ms));

        // 配置允许的主机列表（HTTP 访问控制）
        let manifest = if !config.security.allowed_hosts.is_empty() {
            manifest.with_allowed_hosts(
                config
                    .security
                    .allowed_hosts
                    .iter()
                    .map(|s| s.clone()),
            )
        } else {
            manifest
        };

        // 创建 Extism Plugin 实例（带 AOT 编译）
        let plugin = Plugin::new(&manifest, [], true)
            .map_err(|e| EvifError::Internal(format!("Failed to create extism plugin: {}", e)))?;

        let name = config.name.clone();
        tracing::info!(
            "Created Extism plugin '{}' with security config: memory={}MB, timeout={}ms, hosts={:?}",
            name,
            config.security.memory_limit / 1024 / 1024,
            config.security.timeout_ms,
            config.security.allowed_hosts
        );

        Ok(Self {
            name,
            plugin: Arc::new(Mutex::new(plugin)),
            config,
        })
    }

    /// 调用 WASM 插件的导出函数
    ///
    /// # 参数
    /// - `func_name`: 导出函数名称
    /// - `input`: 输入参数（JSON 序列化）
    ///
    /// # 返回
    /// 插件返回的数据
    async fn call_wasm_function(
        &self,
        func_name: &str,
        input: serde_json::Value,
    ) -> EvifResult<Vec<u8>> {
        let mut plugin = self.plugin.lock().await;
        let input_json = input.to_string();

        // 调用 extism 插件函数
        let output: &[u8] = plugin
            .call(func_name, &input_json)
            .map_err(|e| EvifError::Internal(format!("WASM function call failed: {}", e)))?;

        Ok(output.to_vec())
    }

    /// 调用 WASM 插件并反序列化返回值
    async fn call_and_parse<T>(&self, func_name: &str, input: serde_json::Value) -> EvifResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let output = self.call_wasm_function(func_name, input).await?;
        let result = serde_json::from_slice(&output)
            .map_err(|e| EvifError::Internal(format!("Failed to parse WASM output: {}", e)))?;
        Ok(result)
    }

    /// 获取插件导出的函数列表
    pub fn get_exports(&self) -> Vec<String> {
        // Extism 不直接提供函数列表，需要通过模块信息获取
        // 返回已知的 EVIF 标准函数
        vec![
            "evif_create".to_string(),
            "evif_mkdir".to_string(),
            "evif_read".to_string(),
            "evif_write".to_string(),
            "evif_readdir".to_string(),
            "evif_stat".to_string(),
            "evif_remove".to_string(),
            "evif_rename".to_string(),
            "evif_remove_all".to_string(),
        ]
    }
}

#[async_trait]
impl EvifPlugin for ExtismPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        let input = serde_json::json!({
            "path": path,
            "perm": perm,
            "config": self.config.config
        });

        #[derive(Deserialize)]
        struct Response {
            success: bool,
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_create", input).await?;

        if !response.success {
            return Err(EvifError::Internal(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        let input = serde_json::json!({
            "path": path,
            "perm": perm
        });

        #[derive(Deserialize)]
        struct Response {
            success: bool,
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_mkdir", input).await?;

        if !response.success {
            return Err(EvifError::Internal(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let input = serde_json::json!({
            "path": path,
            "offset": offset,
            "size": size
        });

        #[derive(Deserialize)]
        struct Response {
            #[serde(default)]
            data: String, // Base64 编码的数据
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_read", input).await?;

        if let Some(error) = response.error {
            return Err(EvifError::Internal(error));
        }

        // Base64 解码
        let data = general_purpose::STANDARD
            .decode(&response.data)
            .map_err(|e| EvifError::Internal(format!("Base64 decode failed: {}", e)))?;

        Ok(data)
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        offset: i64,
        flags: WriteFlags,
    ) -> EvifResult<u64> {
        // Base64 编码数据
        let data_base64 = general_purpose::STANDARD.encode(&data);

        let input = serde_json::json!({
            "path": path,
            "data": data_base64,
            "offset": offset,
            "flags": flags.bits()
        });

        #[derive(Deserialize)]
        struct Response {
            bytes_written: u64,
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_write", input).await?;

        if let Some(error) = response.error {
            return Err(EvifError::Internal(error));
        }

        Ok(response.bytes_written)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let input = serde_json::json!({
            "path": path
        });

        #[derive(Deserialize)]
        struct Response {
            #[serde(default)]
            files: Vec<WasmFileInfo>,
            error: Option<String>,
        }

        #[derive(Deserialize)]
        struct WasmFileInfo {
            name: String,
            #[serde(default)]
            size: u64,
            #[serde(default)]
            mode: u32,
            #[serde(default)]
            modified: String,
            #[serde(default)]
            is_dir: bool,
        }

        let response: Response = self.call_and_parse("evif_readdir", input).await?;

        if let Some(error) = response.error {
            return Err(EvifError::Internal(error));
        }

        // 转换为 EVIF FileInfo
        let files = response
            .files
            .into_iter()
            .map(|f| FileInfo {
                name: f.name,
                size: f.size,
                mode: f.mode,
                modified: chrono::DateTime::parse_from_rfc3339(&f.modified)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                is_dir: f.is_dir,
            })
            .collect();

        Ok(files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let input = serde_json::json!({
            "path": path
        });

        #[derive(Deserialize)]
        struct Response {
            file: WasmFileInfo,
            error: Option<String>,
        }

        #[derive(Deserialize)]
        struct WasmFileInfo {
            name: String,
            #[serde(default)]
            size: u64,
            #[serde(default)]
            mode: u32,
            #[serde(default)]
            modified: String,
            #[serde(default)]
            is_dir: bool,
        }

        let response: Response = self.call_and_parse("evif_stat", input).await?;

        if let Some(error) = response.error {
            return Err(EvifError::Internal(error));
        }

        Ok(FileInfo {
            name: response.file.name,
            size: response.file.size,
            mode: response.file.mode,
            modified: chrono::DateTime::parse_from_rfc3339(&response.file.modified)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            is_dir: response.file.is_dir,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let input = serde_json::json!({
            "path": path
        });

        #[derive(Deserialize)]
        struct Response {
            success: bool,
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_remove", input).await?;

        if !response.success {
            return Err(EvifError::Internal(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let input = serde_json::json!({
            "old_path": old_path,
            "new_path": new_path
        });

        #[derive(Deserialize)]
        struct Response {
            success: bool,
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_rename", input).await?;

        if !response.success {
            return Err(EvifError::Internal(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let input = serde_json::json!({
            "path": path
        });

        #[derive(Deserialize)]
        struct Response {
            success: bool,
            error: Option<String>,
        }

        let response: Response = self.call_and_parse("evif_remove_all", input).await?;

        if !response.success {
            return Err(EvifError::Internal(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }
}

/// 导出辅助函数
pub mod helpers {
    use super::*;

    /// 从文件路径加载 WASM 插件（使用默认安全配置）
    pub fn load_wasm_plugin_from_file(
        wasm_path: &str,
        name: &str,
        mount_point: &str,
    ) -> EvifResult<ExtismPlugin> {
        let config = WasmPluginConfig::new(wasm_path)
            .with_name(name)
            .with_mount_point(mount_point);

        ExtismPlugin::new(config)
    }

    /// 从文件路径加载 WASM 插件（使用自定义安全配置）
    pub fn load_wasm_plugin_with_security(
        wasm_path: &str,
        name: &str,
        mount_point: &str,
        security: SecurityConfig,
    ) -> EvifResult<ExtismPlugin> {
        let config = WasmPluginConfig::new(wasm_path)
            .with_name(name)
            .with_mount_point(mount_point)
            .with_security(security);

        ExtismPlugin::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.memory_limit, 64 * 1024 * 1024);
        assert_eq!(config.timeout_ms, 5000);
        assert!(config.allowed_hosts.is_empty());
        assert!(config.allowed_paths.is_empty());
    }

    #[test]
    fn test_security_config_builder() {
        let config = SecurityConfig::new()
            .with_memory_limit(128 * 1024 * 1024)
            .with_timeout_ms(10000)
            .with_allowed_hosts(vec!["api.example.com".to_string()])
            .with_allowed_paths(vec!["/data".to_string()]);

        assert_eq!(config.memory_limit, 128 * 1024 * 1024);
        assert_eq!(config.timeout_ms, 10000);
        assert_eq!(config.allowed_hosts.len(), 1);
        assert_eq!(config.allowed_paths.len(), 1);
    }

    #[test]
    fn test_wasm_plugin_config_default() {
        let config = WasmPluginConfig::default();
        assert_eq!(config.name, "wasm_plugin");
        assert_eq!(config.mount_point, "/");
        assert_eq!(config.security.memory_limit, 64 * 1024 * 1024);
    }

    #[test]
    fn test_wasm_plugin_config_builder() {
        let config = WasmPluginConfig::new("/path/to/plugin.wasm")
            .with_name("test_plugin")
            .with_mount_point("/mnt/test")
            .with_config(serde_json::json!({"key": "value"}));

        assert_eq!(config.wasm_path, "/path/to/plugin.wasm");
        assert_eq!(config.name, "test_plugin");
        assert_eq!(config.mount_point, "/mnt/test");
        assert_eq!(config.config["key"], "value");
    }

    #[test]
    fn test_extism_plugin_not_found() {
        let config = WasmPluginConfig::new("/nonexistent/path.wasm");
        let result = ExtismPlugin::new(config);
        assert!(result.is_err());
    }
}
