// EVIF Core - Everything Is a File System
//
// 核心文件系统抽象，完全对标 AGFS
// 使用 Radix Tree 进行插件路由，无图结构依赖

pub mod acl;
pub mod batch_operations;
pub mod cache;
pub mod circuit_breaker;
pub mod config;
pub mod config_validation;
pub mod cross_fs_copy;
pub mod dynamic_loader;
pub mod error;
pub mod file_lock;
pub mod file_monitor;
pub mod handle_manager;
pub mod memory_handle;
pub mod monitoring;
pub mod mount_table;
pub mod plugin;
pub mod plugin_registry;
pub mod radix_benchmarks;
pub mod radix_mount_table;
pub mod server;
pub mod snapshot;
pub mod streaming;

// WASM plugin support (requires "wasm" feature - enables both backends)
#[cfg(feature = "wasm")]
pub mod wasm;

// Extism WASM plugin support (requires "extism-backend" feature)
#[cfg(feature = "extism-backend")]
pub mod extism_plugin;

// Extism WASM plugin pool (requires "extism-backend" feature)
#[cfg(feature = "extism-backend")]
pub mod plugin_pool;

pub use acl::{
    AclCheckResult, AclEntry, AclManager, AclPermissions, AclSupported, AclType, UserContext,
};
pub use batch_operations::{
    BatchCopyRequest, BatchDeleteRequest, BatchError, BatchExecutor, BatchOperations,
    BatchProgress, BatchResult, ProgressCallback,
};
pub use cache::{Cache, CacheConfig, CacheStats, DirectoryCache, EvifCache, MetadataCache};
pub use circuit_breaker::{
    all_circuit_breakers, get_circuit_breaker, CircuitBreaker, CircuitBreakerConfig,
    CircuitBreakerError, CircuitBreakerSnapshot, CircuitState,
};
pub use config::{
    CacheConfig as EvifCacheConfig, EvifConfig, LoggingConfig, PluginsConfig, ServerConfig,
};
pub use config_validation::{ConfigParamType, ConfigParameter, ConfigValidator};
pub use dynamic_loader::{
    DynamicPluginLoader, DynamicPluginLoaderBuilder, PluginInfo, PluginPtr, EVIF_PLUGIN_ABI_VERSION,
};
pub use error::{EvifError, EvifResult};
pub use file_monitor::{
    EventFilter, EventManager, FileEvent, FileEventType, FileMonitor, MonitorError, MonitorFactory,
    SimpleFileMonitor,
};
pub use handle_manager::GlobalHandleManager;
pub use memory_handle::MemoryFileHandle;
pub use monitoring::{
    HealthStatus, MetricsCollector, PerformanceMonitor, PluginHealth, PluginStats, SystemStats,
};
pub use mount_table::MountTable;
pub use plugin::{
    validate_and_initialize_plugin, EvifPlugin, FileHandle, FileInfo, HandleFS, OpenFlags,
    PluginConfigParam, WriteFlags,
};
pub use plugin_registry::{PluginRegistry, PluginRegistryRef, PluginState, RegisteredPlugin};
pub use radix_mount_table::{MountTableStats, RadixMountTable};
pub use server::EvifServer;
pub use snapshot::{CowSnapshot, SnapshotDiff, SnapshotEntry, SnapshotManager, SnapshotMetadata};
pub use streaming::{LineReader, MemoryStreamReader, StreamReader, Streamer};

// WASM plugin exports
#[cfg(feature = "wasm")]
pub use wasm::{
    detect_backend_from_path, WasmBackendType, WasmPluginConfig, WasmPluginHandle,
    WasmPluginManager,
};

// Extism plugin exports
#[cfg(feature = "extism-backend")]
pub use extism_plugin::{ExtismPlugin, SecurityConfig as WasmSecurityConfig};

// Plugin pool exports
#[cfg(feature = "extism-backend")]
pub use plugin_pool::{
    PoolConfig, PoolError, PoolStats, PooledPlugin, PluginPool, PluginPoolManager, PoolResult,
};
