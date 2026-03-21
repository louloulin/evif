// EVIF Core - Everything Is a File System
//
// 核心文件系统抽象，完全对标 AGFS
// 使用 Radix Tree 进行插件路由，无图结构依赖

pub mod acl;
pub mod batch_operations;
pub mod cache;
pub mod config;
pub mod config_validation;
pub mod dynamic_loader;
pub mod error;
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
pub mod streaming;

// WASM plugin support (requires "wasm" feature)
#[cfg(feature = "wasm")]
pub mod extism_plugin;

pub use acl::{
    AclCheckResult, AclEntry, AclManager, AclPermissions, AclSupported, AclType, UserContext,
};
pub use batch_operations::{
    BatchCopyRequest, BatchDeleteRequest, BatchError, BatchExecutor, BatchOperations,
    BatchProgress, BatchResult, ProgressCallback,
};
pub use cache::{Cache, CacheConfig, CacheStats, DirectoryCache, EvifCache, MetadataCache};
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
pub use streaming::{LineReader, MemoryStreamReader, StreamReader, Streamer};
