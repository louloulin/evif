// EVIF Plugin Management REST API Handlers
//
// 插件管理 HTTP 接口
// 对标 AGFS Plugin Handlers

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;

use evif_core::{MountTable, EvifPlugin, DynamicPluginLoader, PluginInfo as EvifPluginInfo, PluginRegistry, PluginState as RegistryPluginState};
use std::sync::OnceLock;
use crate::handlers::AppState;

/// 应用状态
#[derive(Clone)]
pub struct PluginState {
    pub mount_table: Arc<MountTable>,
    pub dynamic_loader: Arc<DynamicPluginLoader>,
    pub plugin_registry: Arc<PluginRegistry>,
}

/// 插件信息
#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub mount_path: String,
    pub plugin_type: String,
}

/// 插件配置参数
#[derive(Debug, Serialize)]
pub struct ConfigParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: String,
}

/// 插件配置 Schema
#[derive(Debug, Serialize)]
pub struct PluginConfigSchema {
    pub name: String,
    pub description: String,
    pub version: String,
    pub parameters: Vec<ConfigParameter>,
}

/// 加载插件请求
#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub path: String,
    pub config: Option<HashMap<String, serde_json::Value>>,
}

/// 卸载插件请求
#[derive(Debug, Deserialize)]
pub struct UnloadPluginRequest {
    pub path: String,
}

/// EVIF 插件管理 API 处理器
pub struct PluginHandlers;

impl PluginHandlers {
    /// 列出所有可用插件
    /// GET /api/v1/plugins
    pub async fn list_plugins(
        State(state): State<PluginState>,
    ) -> Result<Json<Vec<PluginInfo>>, PluginError> {
        let mount_paths = state.mount_table.list_mounts().await;

        let mut plugins = Vec::new();
        for path in mount_paths {
            // 获取插件信息
            let plugin_name = match state.mount_table.lookup(&path).await {
                Some(plugin) => plugin.name().to_string(),
                None => "unknown".to_string(),
            };

            plugins.push(PluginInfo {
                name: plugin_name,
                mount_path: path,
                plugin_type: std::any::type_name::<&dyn EvifPlugin>().to_string(),
            });
        }

        Ok(Json(plugins))
    }

    /// 列出所有挂载点
    /// GET /api/v1/plugins/mounts
    pub async fn list_mounts(
        State(state): State<PluginState>,
    ) -> Result<Json<serde_json::Value>, PluginError> {
        let mount_paths = state.mount_table.list_mounts().await;

        let mut mount_list = Vec::new();
        for path in mount_paths {
            let plugin_name = match state.mount_table.lookup(&path).await {
                Some(plugin) => plugin.name().to_string(),
                None => "unknown".to_string(),
            };

            mount_list.push(serde_json::json!({
                "path": path,
                "plugin": plugin_name,
            }));
        }

        Ok(Json(serde_json::json!({
            "mounts": mount_list,
            "count": mount_list.len(),
        })))
    }

    /// 挂载插件
    /// POST /api/v1/plugins/mount?plugin=<name>&path=<path>
    pub async fn mount_plugin(
        State(state): State<PluginState>,
        Query(params): Query<std::collections::HashMap<String, String>>,
    ) -> Result<Json<serde_json::Value>, PluginError> {
        let plugin_name = params
            .get("plugin")
            .ok_or_else(|| PluginError::BadRequest("Missing plugin parameter".to_string()))?
            .clone();

        let mount_path = params
            .get("path")
            .ok_or_else(|| PluginError::BadRequest("Missing path parameter".to_string()))?
            .clone();

        let config: Option<serde_json::Value> = params
            .get("config")
            .and_then(|c| serde_json::from_str(c).ok());

        // 这里需要插件工厂来创建插件实例
        // 暂时返回错误提示需要实现
        Err(PluginError::Internal(
            "Plugin mounting not fully implemented - requires plugin factory".to_string()
        ))
    }

    /// 卸载插件
    /// DELETE /api/v1/plugins/mounts?path=<path>
    pub async fn unmount_plugin(
        State(state): State<PluginState>,
        Query(params): Query<std::collections::HashMap<String, String>>,
    ) -> Result<Json<serde_json::Value>, PluginError> {
        let path = params
            .get("path")
            .ok_or_else(|| PluginError::BadRequest("Missing path parameter".to_string()))?
            .clone();

        state.mount_table
            .unmount(&path)
            .await
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({
            "message": "Plugin unmounted",
            "path": path,
        })))
    }

    /// GET /api/v1/plugins/{name}/config
    /// NOTE: Returns plugin configuration schema. Schema definitions are maintained for supported plugins.
    pub async fn get_plugin_config(
        State(_state): State<PluginState>,
        Path(name): Path<String>,
    ) -> Result<Json<PluginConfigSchema>, PluginError> {
        match name.as_str() {
            "localfs" => Ok(Json(PluginConfigSchema {
                name: "localfs".to_string(),
                description: "Local filesystem plugin".to_string(),
                version: "1.0.0".to_string(),
                parameters: vec![
                    ConfigParameter {
                        name: "root".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        default: None,
                        description: "Root directory path".to_string(),
                    },
                ],
            })),
            "s3fs" => Ok(Json(PluginConfigSchema {
                name: "s3fs".to_string(),
                description: "AWS S3 compatible storage plugin".to_string(),
                version: "1.0.0".to_string(),
                parameters: vec![
                    ConfigParameter {
                        name: "bucket".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        default: None,
                        description: "S3 bucket name".to_string(),
                    },
                    ConfigParameter {
                        name: "region".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        default: Some("us-east-1".to_string()),
                        description: "AWS region".to_string(),
                    },
                    ConfigParameter {
                        name: "endpoint".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        default: None,
                        description: "Custom S3 endpoint URL".to_string(),
                    },
                ],
            })),
            "memoryfs" => Ok(Json(PluginConfigSchema {
                name: "memoryfs".to_string(),
                description: "In-memory storage plugin".to_string(),
                version: "1.0.0".to_string(),
                parameters: vec![
                    ConfigParameter {
                        name: "max_size_mb".to_string(),
                        param_type: "number".to_string(),
                        required: false,
                        default: Some("100".to_string()),
                        description: "Maximum memory size in MB".to_string(),
                    },
                ],
            })),
            "httpfs" => Ok(Json(PluginConfigSchema {
                name: "httpfs".to_string(),
                description: "HTTP/HTTPS file access plugin".to_string(),
                version: "1.0.0".to_string(),
                parameters: vec![
                    ConfigParameter {
                        name: "base_url".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        default: None,
                        description: "Base base URL for HTTP requests".to_string(),
                    },
                    ConfigParameter {
                        name: "timeout_secs".to_string(),
                        param_type: "number".to_string(),
                        required: false,
                        default: Some("30".to_string()),
                        description: "Request timeout in seconds".to_string(),
                    },
                ],
            })),
            "queuefs" => Ok(Json(PluginConfigSchema {
                name: "queuefs".to_string(),
                description: "Message queue filesystem plugin".to_string(),
                version: "1.0.0".to_string(),
                parameters: vec![
                    ConfigParameter {
                        name: "max_messages".to_string(),
                        param_type: "number".to_string(),
                        required: false,
                        default: Some("1000".to_string()),
                        description: "Maximum number of messages in queue".to_string(),
                    },
                ],
            })),
            _ => Err(PluginError::NotFound(format!("Plugin not found: {}", name))),
        }
    }

    /// 加载外部插件（支持WASM和动态库）
    /// POST /api/v1/plugins/load
    pub async fn load_plugin(
        State(state): State<PluginState>,
        Json(req): Json<LoadPluginRequest>,
    ) -> Result<Json<serde_json::Value>, PluginError> {
        // 支持动态插件加载
        if req.plugin_type == "dynamic" {
            // 使用动态插件加载器
            let library_name = req.path.clone();
            let mount_path = format!("/{}", library_name);

            // 先加载动态库
            let plugin_info = state.dynamic_loader.load_plugin(&library_name)
                .map_err(|e| PluginError::Internal(format!("Failed to load dynamic library: {}", e)))?;

            // 创建插件实例
            let plugin = state.dynamic_loader.create_plugin(&library_name)
                .map_err(|e| PluginError::Internal(format!("Failed to create plugin instance: {}", e)))?;

            // 挂载插件
            let config = req.config.as_ref().map(|c| serde_json::Value::Object(c.clone().into_iter().collect()));
            plugin.validate(config.as_ref()).await
                .map_err(|e| PluginError::Internal(format!("Plugin validation failed: {}", e)))?;

            state.mount_table.mount(mount_path.clone(), plugin).await
                .map_err(|e| PluginError::Internal(format!("Failed to mount plugin: {}", e)))?;

            return Ok(Json(serde_json::json!({
                "message": format!("Dynamic plugin '{}' loaded and mounted at '{}'", plugin_info.name(), mount_path),
                "plugin_type": "dynamic",
                "name": plugin_info.name(),
                "version": plugin_info.version(),
                "author": plugin_info.author(),
                "description": plugin_info.description(),
                "mount_path": mount_path,
                "path": req.path,
            })));
        }

        // 支持WASM插件加载（使用extism）
        #[cfg(feature = "wasm")]
        {
            if req.plugin_type == "wasm" {
                // 使用WASM插件加载器
                use crate::wasm_handlers::{LoadWasmPluginRequest, WasmPluginHandlers};
                use crate::handlers::AppState;

                let wasm_req = LoadWasmPluginRequest {
                    wasm_path: req.path.clone(),
                    name: req.path.split('/').last().unwrap_or("wasm_plugin").to_string(),
                    mount: format!("/{}", req.path.split('/').last().unwrap_or("wasm")),
                    config: req.config.clone().unwrap_or(serde_json::json!({})),
                };

                // 创建AppState
                let app_state = AppState {
                    mount_table: state.mount_table.clone(),
                };

                // 调用WASM加载器
                match WasmPluginHandlers::load_wasm_plugin(
                    axum::extract::State(app_state),
                    Json(wasm_req),
                ).await {
                    Ok(response) => {
                        return Ok(Json(serde_json::json!({
                            "message": response.message,
                            "plugin_type": "wasm",
                            "path": req.path,
                        })))
                    }
                    Err(e) => {
                        return Err(PluginError::Internal(format!("WASM plugin loading failed: {}", e)));
                    }
                }
            }
        }

        // 其他类型的插件暂不支持
        Err(PluginError::Internal(
            format!("Plugin type '{}' not supported. Currently 'dynamic' and 'wasm' types are supported.", req.plugin_type)
        ))
    }

    /// 卸载外部插件
    /// DELETE /api/v1/plugins/unload
    pub async fn unload_plugin(
        State(state): State<PluginState>,
        Json(req): Json<UnloadPluginRequest>,
    ) -> Result<Json<serde_json::Value>, PluginError> {
        // 直接卸载挂载点
        state.mount_table
            .unmount(&req.path)
            .await
            .map_err(|e| PluginError::Internal(format!("Failed to unload plugin: {}", e)))?;

        Ok(Json(serde_json::json!({
            "message": "Plugin unloaded successfully",
            "path": req.path,
        })))
    }
}

// ==================== 错误类型 ====================

#[derive(Debug)]
pub enum PluginError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl axum::response::IntoResponse for PluginError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            PluginError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            PluginError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            PluginError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": status.to_string(),
            "message": message,
        }));

        (status, body).into_response()
    }
}
