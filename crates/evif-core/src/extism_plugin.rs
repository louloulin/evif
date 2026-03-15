// Extism-based WASM Plugin Support
//
// 实现基于 extism 的 WASM 插件支持
// 充分复用 extism 的 PDK (Plugin Development Kit) 能力

use crate::error::{EvifError, EvifResult};
use crate::plugin::{EvifPlugin, FileHandle, FileInfo, WriteFlags};
use async_trait::async_trait;
use extism::{Manifest, Plugin, Wasm};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use base64::{engine::general_purpose, Engine as _};

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
}

impl Default for WasmPluginConfig {
    fn default() -> Self {
        Self {
            wasm_path: String::new(),
            name: String::from("wasm_plugin"),
            mount_point: String::from("/"),
            config: serde_json::json!({}),
        }
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

        // 创建 Extism Manifest
        let manifest = Manifest::new([Wasm::file(wasm_path)]);

        // 创建 Extism Plugin 实例
        let plugin = Plugin::new(&manifest, [], true)
            .map_err(|e| EvifError::Internal(format!("Failed to create extism plugin: {}", e)))?;

        Ok(Self {
            name: config.name.clone(),
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
            files: Vec<WasmFileInfo>,
            error: Option<String>,
        }

        #[derive(Deserialize)]
        struct WasmFileInfo {
            name: String,
            size: u64,
            mode: u32,
            modified: String, // ISO 8601 字符串
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
            size: u64,
            mode: u32,
            modified: String,
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

    /// 从文件路径加载 WASM 插件
    pub fn load_wasm_plugin_from_file(
        wasm_path: &str,
        name: &str,
        mount_point: &str,
    ) -> EvifResult<ExtismPlugin> {
        let config = WasmPluginConfig {
            wasm_path: wasm_path.to_string(),
            name: name.to_string(),
            mount_point: mount_point.to_string(),
            config: serde_json::json!({}),
        };

        ExtismPlugin::new(config)
    }
}
