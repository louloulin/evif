// WASM Plugin Loader Handlers
//
// 运行时动态加载 WASM 插件的 REST API 端点

use crate::{handlers::AppState, RestError, RestResult};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

/// WASM 插件加载请求
#[derive(Debug, Deserialize)]
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
pub struct LoadWasmPluginRequest {
    /// WASM 文件路径
    pub wasm_path: String,
    /// 插件名称
    pub name: String,
    /// 挂载点
    pub mount: String,
    /// 配置参数（可选）
    #[serde(default)]
    pub config: serde_json::Value,
}

/// WASM 插件加载响应
#[derive(Debug, Serialize)]
pub struct LoadWasmPluginResponse {
    /// 是否成功
    pub success: bool,
    /// 插件名称
    pub plugin_name: String,
    /// 挂载点
    pub mount_point: String,
    /// 消息
    pub message: String,
}

/// 卸载插件请求
#[derive(Debug, Deserialize)]
pub struct UnloadPluginRequest {
    /// 挂载点
    pub mount_point: String,
}

/// 卸载插件响应
#[derive(Debug, Serialize)]
pub struct UnloadPluginResponse {
    /// 是否成功
    pub success: bool,
    /// 消息
    pub message: String,
}

/// 列出已加载的插件响应
#[derive(Debug, Serialize)]
pub struct ListPluginsResponse {
    /// 插件列表
    pub plugins: Vec<PluginInfo>,
    /// 总数
    pub total: usize,
}

/// 插件信息
#[derive(Debug, Serialize)]
pub struct PluginInfo {
    /// 插件名称
    pub name: String,
    /// 挂载点
    pub mount_point: String,
    /// 插件类型
    pub plugin_type: String,
    /// 是否支持热重载
    pub hot_reloadable: bool,
}

/// 热重载请求 (Phase 16.1: WASM 插件热重载)
#[derive(Debug, Deserialize)]
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
pub struct ReloadWasmPluginRequest {
    /// 挂载点
    pub mount_point: String,
}

/// 热重载响应
#[derive(Debug, Serialize)]
pub struct ReloadWasmPluginResponse {
    /// 是否成功
    pub success: bool,
    /// 插件名称
    pub plugin_name: String,
    /// 挂载点
    pub mount_point: String,
    /// 消息
    pub message: String,
}

/// WASM 插件处理器
pub struct WasmPluginHandlers;

impl WasmPluginHandlers {
    /// 加载 WASM 插件
    ///
    /// # 功能
    /// - 从指定路径加载 WASM 文件
    /// - 创建 ExtismPlugin 实例
    /// - 挂载到 RadixMountTable
    ///
    /// # REST API
    /// POST /api/v1/plugins/wasm/load
    pub async fn load_wasm_plugin(
        State(_state): State<AppState>,
        Json(_req): Json<LoadWasmPluginRequest>,
    ) -> RestResult<Json<LoadWasmPluginResponse>> {
        #[cfg(feature = "wasm")]
        {
            use evif_core::extism_plugin::{ExtismPlugin, WasmPluginConfig};
            use std::path::Path;

            // 验证挂载点格式
            if !req.mount.starts_with('/') {
                return Err(RestError::BadRequest(
                    "Mount point must start with /".to_string(),
                ));
            }

            // 检查 WASM 文件是否存在
            let wasm_path = Path::new(&req.wasm_path);
            if !wasm_path.exists() {
                return Err(RestError::NotFound(format!(
                    "WASM file not found: {}",
                    req.wasm_path
                )));
            }

            // 创建 WASM 插件配置
            let config = WasmPluginConfig {
                wasm_path: req.wasm_path.clone(),
                name: req.name.clone(),
                mount_point: req.mount.clone(),
                config: req.config.clone(),
            };

            // 创建 ExtismPlugin 实例
            let plugin = ExtismPlugin::new(config)
                .map_err(|e| RestError::Internal(format!("Failed to create WASM plugin: {}", e)))?;

            // 挂载插件
            state
                .mount_table
                .mount(req.mount.clone(), Arc::new(plugin))
                .await
                .map_err(|e| RestError::Internal(format!("Failed to mount plugin: {}", e)))?;

            Ok(Json(LoadWasmPluginResponse {
                success: true,
                plugin_name: req.name.clone(),
                mount_point: req.mount.clone(),
                message: format!(
                    "WASM plugin '{}' loaded successfully at '{}'",
                    req.name, req.mount
                ),
            }))
        }

        #[cfg(not(feature = "wasm"))]
        {
            Err(RestError::BadRequest(
                "WASM support is not enabled. Build with --features wasm".to_string(),
            ))
        }
    }

    /// 卸载插件
    ///
    /// # 功能
    /// - 从挂载表卸载插件
    ///
    /// # REST API
    /// POST /api/v1/plugins/unload
    pub async fn unload_plugin(
        State(state): State<AppState>,
        Json(req): Json<UnloadPluginRequest>,
    ) -> RestResult<Json<UnloadPluginResponse>> {
        state
            .mount_table
            .unmount(&req.mount_point)
            .await
            .map_err(|e| RestError::Internal(format!("Failed to unload plugin: {}", e)))?;

        Ok(Json(UnloadPluginResponse {
            success: true,
            message: format!("Plugin unloaded from '{}'", req.mount_point),
        }))
    }

    /// 列出所有已加载的插件
    ///
    /// # 功能
    /// - 返回所有挂载点信息
    ///
    /// # REST API
    /// GET /api/v1/plugins/list
    pub async fn list_plugins(
        State(state): State<AppState>,
    ) -> RestResult<Json<ListPluginsResponse>> {
        // 获取所有挂载点
        let mount_points = state.mount_table.list_mounts().await;

        // 转换为插件信息
        let plugins: Vec<PluginInfo> = mount_points
            .into_iter()
            .map(|path| {
                // 判断插件类型（基于路径简单判断）
                let is_wasm = path.contains("wasm") || path.contains("WASM");
                let plugin_type = if is_wasm {
                    "wasm".to_string()
                } else if path.contains("memory") || path.starts_with("/mem") {
                    "memory".to_string()
                } else if path.contains("s3") {
                    "s3".to_string()
                } else if path.contains("http") {
                    "http".to_string()
                } else {
                    "native".to_string()
                };

                PluginInfo {
                    name: format!("plugin_{}", path.replace("/", "_")),
                    mount_point: path,
                    plugin_type,
                    hot_reloadable: is_wasm,
                }
            })
            .collect();

        Ok(Json(ListPluginsResponse {
            total: plugins.len(),
            plugins,
        }))
    }

    /// 热重载 WASM 插件 (Phase 16.1)
    ///
    /// # 功能
    /// - 卸载现有插件
    /// - 重新读取 WASM 文件
    /// - 重新挂载插件
    ///
    /// # REST API
    /// POST /api/v1/plugins/wasm/reload
    pub async fn reload_wasm_plugin(
        State(_state): State<AppState>,
        Json(_req): Json<ReloadWasmPluginRequest>,
    ) -> RestResult<Json<ReloadWasmPluginResponse>> {
        #[cfg(feature = "wasm")]
        {
            use evif_core::extism_plugin::{ExtismPlugin, WasmPluginConfig};

            // 1. 获取现有插件的配置信息（从挂载点查找）
            let wasm_path = format!("{}.wasm", req.mount_point.trim_end_matches('/'));
            let plugin_name = req
                .mount_point
                .trim_start_matches('/')
                .split('/')
                .last()
                .unwrap_or("wasm_plugin")
                .to_string();

            // 2. 卸载现有插件
            state
                .mount_table
                .unmount(&req.mount_point)
                .await
                .map_err(|e| {
                    RestError::Internal(format!("Failed to unmount plugin for reload: {}", e))
                })?;

            // 3. 创建新插件实例
            let config = WasmPluginConfig {
                wasm_path: wasm_path.clone(),
                name: plugin_name.clone(),
                mount_point: req.mount_point.clone(),
                config: serde_json::Value::Null,
            };

            let plugin = ExtismPlugin::new(config)
                .map_err(|e| RestError::Internal(format!("Failed to reload WASM plugin: {}", e)))?;

            // 4. 重新挂载
            state
                .mount_table
                .mount(req.mount_point.clone(), Arc::new(plugin))
                .await
                .map_err(|e| RestError::Internal(format!("Failed to remount plugin: {}", e)))?;

            Ok(Json(ReloadWasmPluginResponse {
                success: true,
                plugin_name,
                mount_point: req.mount_point,
                message: format!("Plugin hot-reloaded from '{}'", wasm_path),
            }))
        }

        #[cfg(not(feature = "wasm"))]
        {
            Err(RestError::BadRequest(
                "WASM support is not enabled. Build with --features wasm".to_string(),
            ))
        }
    }
}
