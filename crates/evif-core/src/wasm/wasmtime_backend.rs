// Wasmtime Native WASM Backend
//
// High-performance WASM plugin backend using Wasmtime runtime
// Supports Component Model, WASI Preview 2, Winch baseline compiler,
// and Pooling Allocator for efficient instance reuse.

use crate::error::{EvifError, EvifResult};
use crate::plugin::{EvifPlugin, FileInfo, WriteFlags};
use crate::wasm::{WasmBackendType, WasmPluginConfig};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use wasmtime::*;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiView, WasiCtxBuilder};

use base64::{engine::general_purpose, Engine as _};

/// Wasmtime plugin state container
struct PluginState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for PluginState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

/// Wasmtime engine configuration profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineProfile {
    /// Default profile: Cranelift optimizing compiler, on-demand allocation
    /// Best for general-purpose plugins with complex logic
    Default,
    /// Fast startup profile: Winch baseline compiler, on-demand allocation
    /// Microsecond cold-start, lower peak performance
    /// Ideal for short-lived / frequently instantiated plugins
    FastStartup,
    /// Production pool profile: Cranelift + Pooling Allocator
    /// Optimized for high-concurrency scenarios with many instances
    /// Reuses memory slots, avoids mmap per instantiation
    ProductionPool,
}

impl Default for EngineProfile {
    fn default() -> Self {
        EngineProfile::Default
    }
}

/// Wasmtime native plugin implementation
///
/// Implements EvifPlugin trait using Wasmtime runtime with Component Model.
/// Supports three engine profiles for different use cases.
pub struct WasmtimePlugin {
    /// Plugin name
    name: String,
    /// Wasmtime engine
    engine: Engine,
    /// Engine profile used
    profile: EngineProfile,
    /// Wasmtime store (thread-safe wrapper)
    store: Arc<Mutex<Option<Store<PluginState>>>>,
    /// Plugin instance (thread-safe wrapper)
    instance: Arc<Mutex<Option<wasmtime::component::Instance>>>,
    /// Plugin configuration
    config: WasmPluginConfig,
}

impl WasmtimePlugin {
    /// Create a new Wasmtime plugin instance with default engine profile
    pub async fn new(config: WasmPluginConfig) -> EvifResult<Self> {
        Self::with_profile(config, EngineProfile::Default).await
    }

    /// Create a new Wasmtime plugin instance with specified engine profile
    ///
    /// # Arguments
    /// - `config`: WASM plugin configuration
    /// - `profile`: Engine profile determining compiler and allocation strategy
    pub async fn with_profile(config: WasmPluginConfig, profile: EngineProfile) -> EvifResult<Self> {
        // Check WASM file exists
        let wasm_path = Path::new(&config.wasm_path);
        if !wasm_path.exists() {
            return Err(EvifError::NotFound(format!(
                "WASM file not found: {}",
                config.wasm_path
            )));
        }

        let engine = Self::build_engine(profile)?;

        // Create WASI context with security restrictions
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();

        let state = PluginState {
            wasi,
            table: ResourceTable::new(),
        };

        let store = Store::new(&engine, state);

        tracing::info!(
            "Created Wasmtime engine with profile {:?} for plugin '{}'",
            profile,
            config.name
        );

        Ok(Self {
            name: config.name.clone(),
            engine,
            profile,
            store: Arc::new(Mutex::new(Some(store))),
            instance: Arc::new(Mutex::new(None)),
            config,
        })
    }

    /// Build a Wasmtime engine with the specified profile
    fn build_engine(profile: EngineProfile) -> EvifResult<Engine> {
        let mut engine_config = Config::new();

        match profile {
            EngineProfile::Default => {
                // Cranelift optimizing compiler, on-demand allocation
                engine_config
                    .wasm_component_model(true)
                    .wasm_backtrace_details(WasmBacktraceDetails::Enable)
                    .cranelift_opt_level(OptLevel::Speed);
            }
            EngineProfile::FastStartup => {
                // Winch baseline compiler for microsecond cold-start
                engine_config
                    .wasm_component_model(true)
                    .wasm_backtrace_details(WasmBacktraceDetails::Enable)
                    .strategy(Strategy::Winch)
                    .cranelift_opt_level(OptLevel::None);
            }
            EngineProfile::ProductionPool => {
                // Cranelift + Pooling Allocator for high-concurrency
                engine_config
                    .wasm_component_model(true)
                    .wasm_backtrace_details(WasmBacktraceDetails::Enable)
                    .cranelift_opt_level(OptLevel::Speed)
                    .allocation_strategy(InstanceAllocationStrategy::Pooling(
                        PoolingAllocationConfig::default(),
                    ));
            }
        }

        Engine::new(&engine_config)
            .map_err(|e| EvifError::Internal(format!("Failed to create Wasmtime engine: {}", e)))
    }

    /// Get the engine profile used by this plugin
    pub fn profile(&self) -> EngineProfile {
        self.profile
    }

    /// Load and instantiate the WASM module
    pub async fn load(&self) -> EvifResult<()> {
        let component_bytes = std::fs::read(&self.config.wasm_path).map_err(|e| {
            EvifError::Internal(format!("Failed to read WASM file: {}", e))
        })?;

        let component = wasmtime::component::Component::new(&self.engine, &component_bytes)
            .map_err(|e| EvifError::Internal(format!("Failed to compile component: {}", e)))?;

        let linker = wasmtime::component::Linker::<PluginState>::new(&self.engine);

        let mut store = self.store.lock().await;
        let store = store.as_mut().ok_or_else(|| {
            EvifError::Internal("Store not initialized".to_string())
        })?;

        let instance = linker
            .instantiate(store, &component)
            .map_err(|e| EvifError::Internal(format!("Failed to instantiate component: {}", e)))?;

        let mut instance_guard = self.instance.lock().await;
        *instance_guard = Some(instance);

        tracing::info!(
            "Loaded Wasmtime plugin: {} (profile: {:?})",
            self.name, self.profile
        );
        Ok(())
    }

    /// Load with specified engine profile (convenience method)
    pub async fn load_with_profile(&mut self, profile: EngineProfile) -> EvifResult<()> {
        // Rebuild engine if profile changed
        if self.profile != profile {
            self.engine = Self::build_engine(profile)?;
            self.profile = profile;

            // Reset store and instance with new engine
            {
                let wasi = WasiCtxBuilder::new().inherit_stdio().build();
                let state = PluginState {
                    wasi,
                    table: ResourceTable::new(),
                };
                let store = Store::new(&self.engine, state);
                *self.store.lock().await = Some(store);
            }
            *self.instance.lock().await = None;
        }
        self.load().await
    }

    /// Call a WASM function with JSON input
    async fn call_function<T>(
        &self,
        func_name: &str,
        input: serde_json::Value,
    ) -> EvifResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut store_guard = self.store.lock().await;
        let store = store_guard.as_mut().ok_or_else(|| {
            EvifError::Internal("Store not initialized".to_string())
        })?;

        let instance_guard = self.instance.lock().await;
        let instance = instance_guard.as_ref().ok_or_else(|| {
            EvifError::Internal("Plugin not loaded. Call load() first.".to_string())
        })?;

        // Get exported function
        let func = instance
            .get_typed_func::<(String,), (String,)>(&mut *store, func_name)
            .map_err(|e| {
                EvifError::Internal(format!("Function {} not found: {}", func_name, e))
            })?;

        let input_json = input.to_string();

        // Call function
        let (output_json,) = func
            .call(store, (input_json,))
            .map_err(|e| EvifError::Internal(format!("WASM call failed: {}", e)))?;

        // Parse response
        let result: T = serde_json::from_str(&output_json)
            .map_err(|e| EvifError::Internal(format!("Failed to parse response: {}", e)))?;

        Ok(result)
    }
}

#[async_trait]
impl EvifPlugin for WasmtimePlugin {
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

        let response: Response = self.call_function("evif_create", input).await?;

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

        let response: Response = self.call_function("evif_mkdir", input).await?;

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
            data: String,
            error: Option<String>,
        }

        let response: Response = self.call_function("evif_read", input).await?;

        if let Some(error) = response.error {
            return Err(EvifError::Internal(error));
        }

        // Base64 decode
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

        let response: Response = self.call_function("evif_write", input).await?;

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
            modified: String,
            is_dir: bool,
        }

        let response: Response = self.call_function("evif_readdir", input).await?;

        if let Some(error) = response.error {
            return Err(EvifError::Internal(error));
        }

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

        let response: Response = self.call_function("evif_stat", input).await?;

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

        let response: Response = self.call_function("evif_remove", input).await?;

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

        let response: Response = self.call_function("evif_rename", input).await?;

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

        let response: Response = self.call_function("evif_remove_all", input).await?;

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

/// Helper functions for Wasmtime plugin creation
pub mod helpers {
    use super::*;

    /// Load a Wasmtime plugin from file
    pub async fn load_wasmtime_plugin_from_file(
        wasm_path: &str,
        name: &str,
        mount_point: &str,
    ) -> EvifResult<WasmtimePlugin> {
        let config = WasmPluginConfig {
            wasm_path: wasm_path.to_string(),
            name: name.to_string(),
            mount_point: mount_point.to_string(),
            backend: WasmBackendType::Wasmtime,
            ..Default::default()
        };

        let plugin = WasmtimePlugin::new(config).await?;
        plugin.load().await?;
        Ok(plugin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasmtime_plugin_config() {
        let config = WasmPluginConfig {
            wasm_path: "test.wasm".to_string(),
            name: "test_plugin".to_string(),
            backend: WasmBackendType::Wasmtime,
            ..Default::default()
        };

        assert_eq!(config.name, "test_plugin");
        assert_eq!(config.backend, WasmBackendType::Wasmtime);
    }
}
