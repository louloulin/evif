// WASM Plugin Backends
//
// Dual-backend WASM plugin support:
// - Extism: Independent plugin framework with multi-language PDK support
// - Wasmtime: Native high-performance runtime with Component Model

#[cfg(feature = "wasmtime-backend")]
pub mod wasmtime_backend;

use crate::error::{EvifError, EvifResult};
use crate::plugin::{EvifPlugin, FileInfo, WriteFlags};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// WASM plugin backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WasmBackendType {
    /// Wasmtime native backend (high performance, Component Model)
    Wasmtime,
    /// Extism independent backend (multi-language PDK, XTP Bindgen)
    Extism,
}

impl Default for WasmBackendType {
    fn default() -> Self {
        WasmBackendType::Extism
    }
}

impl std::fmt::Display for WasmBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmBackendType::Wasmtime => write!(f, "wasmtime"),
            WasmBackendType::Extism => write!(f, "extism"),
        }
    }
}

/// WASM plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginConfig {
    /// WASM file path
    pub wasm_path: String,
    /// Plugin name
    pub name: String,
    /// Mount point
    #[serde(default = "default_mount_point")]
    pub mount_point: String,
    /// Backend type
    #[serde(default)]
    pub backend: WasmBackendType,
    /// Configuration parameters passed to WASM plugin
    #[serde(default)]
    pub config: serde_json::Value,
    /// Memory limit in bytes (default: 64MB)
    #[serde(default = "default_memory_limit")]
    pub memory_limit: u64,
    /// Timeout in milliseconds (default: 5000ms)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_mount_point() -> String {
    "/".to_string()
}

fn default_memory_limit() -> u64 {
    64 * 1024 * 1024 // 64MB
}

fn default_timeout_ms() -> u64 {
    5000 // 5 seconds
}

impl Default for WasmPluginConfig {
    fn default() -> Self {
        Self {
            wasm_path: String::new(),
            name: String::from("wasm_plugin"),
            mount_point: default_mount_point(),
            backend: WasmBackendType::default(),
            config: serde_json::json!({}),
            memory_limit: default_memory_limit(),
            timeout_ms: default_timeout_ms(),
        }
    }
}

/// Unified WASM plugin handle
#[derive(Debug, Clone)]
pub struct WasmPluginHandle {
    /// Plugin ID
    pub id: uuid::Uuid,
    /// Plugin name
    pub name: String,
    /// Backend type
    pub backend: WasmBackendType,
    /// Mount point
    pub mount_point: String,
}

/// Detect backend type from path
///
/// Path conventions:
/// - `plugins/wasmtime/*.wasm` -> Wasmtime backend
/// - `plugins/extism/*.wasm` -> Extism backend
/// - Default -> Extism backend
pub fn detect_backend_from_path(path: &Path) -> WasmBackendType {
    let path_str = path.to_string_lossy().to_lowercase();

    if path_str.contains("/wasmtime/") || path_str.contains("\\wasmtime\\") {
        return WasmBackendType::Wasmtime;
    }
    if path_str.contains("/extism/") || path_str.contains("\\extism\\") {
        return WasmBackendType::Extism;
    }

    // Check file extension hints
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy().to_lowercase();
        if name_str.ends_with(".component.wasm") {
            return WasmBackendType::Wasmtime;
        }
    }

    // Default to Extism
    WasmBackendType::Extism
}

/// Unified WASM plugin manager
///
/// Manages dual backends (Extism and Wasmtime) with automatic detection
pub struct WasmPluginManager {
    /// Loaded plugins indexed by ID
    plugins: RwLock<Vec<(WasmPluginHandle, Arc<dyn EvifPlugin>)>>,
    /// Default backend type
    default_backend: WasmBackendType,
}

impl WasmPluginManager {
    /// Create a new WASM plugin manager
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
            default_backend: WasmBackendType::default(),
        }
    }

    /// Create a new WASM plugin manager with specified default backend
    pub fn with_default_backend(backend: WasmBackendType) -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
            default_backend: backend,
        }
    }

    /// Load a WASM plugin from file
    ///
    /// Automatically detects backend type from path
    #[cfg(feature = "wasm")]
    pub async fn load_plugin(&self, config: WasmPluginConfig) -> EvifResult<WasmPluginHandle> {
        let backend = if matches!(config.backend, WasmBackendType::Extism) {
            let detected = detect_backend_from_path(Path::new(&config.wasm_path));
            if detected != WasmBackendType::Extism {
                detected
            } else {
                config.backend
            }
        } else {
            config.backend
        };

        let handle = WasmPluginHandle {
            id: uuid::Uuid::new_v4(),
            name: config.name.clone(),
            backend,
            mount_point: config.mount_point.clone(),
        };

        let plugin: Arc<dyn EvifPlugin> = match backend {
            #[cfg(feature = "extism-backend")]
            WasmBackendType::Extism => {
                let extism_config = crate::extism_plugin::WasmPluginConfig::new(&config.wasm_path)
                    .with_name(&config.name)
                    .with_mount_point(&config.mount_point)
                    .with_config(config.config.clone())
                    .with_security(
                        crate::extism_plugin::SecurityConfig::new()
                            .with_memory_limit(config.memory_limit)
                            .with_timeout_ms(config.timeout_ms),
                    );
                let extism_plugin = crate::extism_plugin::ExtismPlugin::new(extism_config)?;
                Arc::new(extism_plugin)
            }
            #[cfg(feature = "wasmtime-backend")]
            WasmBackendType::Wasmtime => {
                let wasmtime_plugin = wasmtime_backend::WasmtimePlugin::new(config).await?;
                wasmtime_plugin.load().await?;
                Arc::new(wasmtime_plugin)
            }
            #[allow(unreachable_patterns)]
            _ => {
                return Err(EvifError::Internal(format!(
                    "WASM backend {:?} not enabled. Enable 'extism-backend' or 'wasmtime-backend' feature.",
                    backend
                )));
            }
        };

        let mut plugins = self.plugins.write().await;
        plugins.push((handle.clone(), plugin));

        tracing::info!(
            "Loaded WASM plugin: {} (backend: {:?}, id: {})",
            handle.name,
            handle.backend,
            handle.id
        );

        Ok(handle)
    }

    /// Load a WASM plugin from file (no WASM features)
    #[cfg(not(feature = "wasm"))]
    pub async fn load_plugin(&self, _config: WasmPluginConfig) -> EvifResult<WasmPluginHandle> {
        Err(EvifError::Internal(
            "WASM support not enabled. Enable 'wasm' feature.".to_string(),
        ))
    }

    /// Get plugin by ID
    pub async fn get_plugin(&self, id: &uuid::Uuid) -> Option<Arc<dyn EvifPlugin>> {
        let plugins = self.plugins.read().await;
        plugins
            .iter()
            .find(|(h, _)| &h.id == id)
            .map(|(_, p)| p.clone())
    }

    /// Get plugin by name
    pub async fn get_plugin_by_name(&self, name: &str) -> Option<Arc<dyn EvifPlugin>> {
        let plugins = self.plugins.read().await;
        plugins
            .iter()
            .find(|(h, _)| h.name == name)
            .map(|(_, p)| p.clone())
    }

    /// Unload plugin by ID
    pub async fn unload_plugin(&self, id: &uuid::Uuid) -> EvifResult<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(idx) = plugins.iter().position(|(h, _)| &h.id == id) {
            let (handle, _) = plugins.remove(idx);
            tracing::info!("Unloaded WASM plugin: {} (id: {})", handle.name, handle.id);
            Ok(())
        } else {
            Err(EvifError::NotFound(format!("Plugin not found: {}", id)))
        }
    }

    /// List all loaded plugins
    pub async fn list_plugins(&self) -> Vec<WasmPluginHandle> {
        let plugins = self.plugins.read().await;
        plugins.iter().map(|(h, _)| h.clone()).collect()
    }

    /// Get plugin count
    pub async fn plugin_count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }
}

impl Default for WasmPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Wasmtime native plugin re-export
#[cfg(feature = "wasmtime-backend")]
pub use wasmtime_backend::WasmtimePlugin;

/// Extism plugin re-export
#[cfg(feature = "extism-backend")]
pub use crate::extism_plugin::ExtismPlugin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_backend_from_path() {
        // Wasmtime paths
        assert_eq!(
            detect_backend_from_path(Path::new("plugins/wasmtime/test.wasm")),
            WasmBackendType::Wasmtime
        );
        assert_eq!(
            detect_backend_from_path(Path::new("/app/plugins/wasmtime/memory.wasm")),
            WasmBackendType::Wasmtime
        );
        assert_eq!(
            detect_backend_from_path(Path::new("cache.component.wasm")),
            WasmBackendType::Wasmtime
        );

        // Extism paths
        assert_eq!(
            detect_backend_from_path(Path::new("plugins/extism/test.wasm")),
            WasmBackendType::Extism
        );
        assert_eq!(
            detect_backend_from_path(Path::new("/app/plugins/extism/python.wasm")),
            WasmBackendType::Extism
        );

        // Default (Extism)
        assert_eq!(
            detect_backend_from_path(Path::new("plugins/unknown/test.wasm")),
            WasmBackendType::Extism
        );
    }

    #[test]
    fn test_wasm_plugin_config_default() {
        let config = WasmPluginConfig::default();
        assert_eq!(config.name, "wasm_plugin");
        assert_eq!(config.mount_point, "/");
        assert_eq!(config.backend, WasmBackendType::Extism);
        assert_eq!(config.memory_limit, 64 * 1024 * 1024);
        assert_eq!(config.timeout_ms, 5000);
    }

    #[tokio::test]
    async fn test_wasm_plugin_manager_creation() {
        let manager = WasmPluginManager::new();
        assert_eq!(manager.plugin_count().await, 0);
    }
}
