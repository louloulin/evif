// EVIF Core - Everything Is a File System
//
// 核心文件系统抽象，完全对标 AGFS
// 使用 Radix Tree 进行插件路由，无图结构依赖

pub mod error;
pub mod plugin;
pub mod mount_table;
pub mod radix_mount_table;
pub mod radix_benchmarks;
pub mod handle_manager;
pub mod memory_handle;
pub mod server;
pub mod cache;
pub mod config;
pub mod config_validation;
pub mod streaming;
pub mod monitoring;
pub mod batch_operations;
pub mod file_monitor;
pub mod acl;
pub mod dynamic_loader;

// WASM plugin support (requires "wasm" feature)
#[cfg(feature = "wasm")]
pub mod extism_plugin;

pub use error::{EvifError, EvifResult};
pub use plugin::{EvifPlugin, WriteFlags, OpenFlags, FileHandle, HandleFS, FileInfo, PluginConfigParam};
pub use mount_table::MountTable;
pub use radix_mount_table::{RadixMountTable, MountTableStats};
pub use handle_manager::GlobalHandleManager;
pub use memory_handle::MemoryFileHandle;
pub use config_validation::{ConfigValidator, ConfigParameter, ConfigParamType};
pub use streaming::{StreamReader, Streamer, MemoryStreamReader, LineReader};
pub use server::EvifServer;
pub use cache::{EvifCache, CacheConfig, MetadataCache, DirectoryCache, Cache, CacheStats};
pub use config::{EvifConfig, ServerConfig, PluginsConfig, CacheConfig as EvifCacheConfig, LoggingConfig};
pub use monitoring::{MetricsCollector, SystemStats, PluginStats, PerformanceMonitor, HealthStatus, PluginHealth};
pub use batch_operations::{
    BatchExecutor, BatchOperations, BatchProgress, BatchResult, BatchError,
    BatchCopyRequest, BatchDeleteRequest, ProgressCallback
};
pub use file_monitor::{
    FileMonitor, FileEvent, FileEventType, EventFilter, EventManager,
    MonitorFactory, MonitorError, SimpleFileMonitor
};
pub use acl::{
    AclManager, AclSupported, AclEntry, AclType, AclPermissions,
    UserContext, AclCheckResult
};
pub use dynamic_loader::{
    DynamicPluginLoader, DynamicPluginLoaderBuilder, PluginInfo,
    EVIF_PLUGIN_ABI_VERSION, PluginPtr
};

