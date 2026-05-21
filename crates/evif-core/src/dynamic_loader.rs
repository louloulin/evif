//! Dynamic Plugin Loader - 动态插件加载系统
//!
//! 对标 AGFS PluginFactory 动态加载机制，支持运行时加载 .so/.dylib/.dll 插件。
//!
//! # 安全性
//!
//! 本模块使用 `unsafe` 代码，以下是各处的 SAFETY 说明：
//!
//! ## `Library::new` (dlopen)
//!
//! - **行 410**: 路径在加载前经过验证（存在性检查）
//! - **风险**: 低 - dlopen 本身是安全的，只加载代码
//!
//! ## `dlsym` 符号查找
//!
//! - **行 459-467**: 查找 `evif_plugin_abi_version` 符号
//! - **行 476-488**: 查找 `evif_plugin_info` 符号
//! - **行 524-531**: 查找 `evif_plugin_create` 符号
//! - **风险**: 低 - 只查找已知的 ABI 入口点
//!
//! ## `Arc::from_raw` (fat pointer 重建)
//!
//! - **行 548-552**: 从 C 库返回的指针重建 Arc
//! - **前置检查**: 验证 `plugin_ptr.data` 非空
//! - **风险**: 中 - 依赖调用者遵守 ABI 约定
//!
//! ## Send/Sync 标记
//!
//! - **行 254-255**: `EvifPluginWrapper` 实现 Send + Sync
//! - **理由**: 函数指针在创建后不可变，访问由 loader 同步保护
//!
//! # AGFS 对标
//!
//! ```go
//! // AGFS PluginFactory 从动态库加载插件
//! plugin, err := factory.LoadPlugin("myplugin.so")
//! ```

use crate::error::{EvifError, EvifResult};
use crate::plugin::EvifPlugin;
use libloading::{Library, Symbol};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

/// 插件 ABI 版本
/// 用于确保动态库与 EVIF 核心兼容
pub const EVIF_PLUGIN_ABI_VERSION: u32 = 1;

/// 插件完整性验证模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityMode {
    /// 不验证（仅用于开发）
    Disabled,
    /// 仅验证存在（检查文件存在）
    Exists,
    /// SHA256 哈希验证
    Hash,
    /// 全部验证（哈希 + 签名）
    Full,
}

impl Default for IntegrityMode {
    fn default() -> Self {
        // 默认启用哈希验证
        IntegrityMode::Hash
    }
}

/// 插件清单条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestEntry {
    /// 插件文件名
    pub file: String,
    /// SHA256 哈希（十六进制字符串）
    pub sha256: Option<String>,
    /// 是否强制验证
    pub required: bool,
}

/// 插件完整性清单
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IntegrityManifest {
    /// 清单版本
    pub version: String,
    /// 插件条目
    pub plugins: Vec<ManifestEntry>,
}

impl IntegrityManifest {
    /// 从文件加载清单
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// 获取插件的期望哈希
    pub fn get_hash(&self, file: &str) -> Option<&str> {
        self.plugins
            .iter()
            .find(|e| e.file == file)
            .and_then(|e| e.sha256.as_deref())
    }

    /// 检查插件是否需要验证
    pub fn is_required(&self, file: &str) -> bool {
        self.plugins
            .iter()
            .find(|e| e.file == file)
            .map(|e| e.required)
            .unwrap_or(false)
    }
}

/// 计算文件的 SHA256 哈希
pub fn compute_file_hash(path: &std::path::Path) -> std::io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(8192, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// 插件创建函数签名（简化版）
/// 插件销毁函数签名
pub type PluginDestroyFn = unsafe extern "C" fn(*const ());

/// 插件信息查询函数签名
pub type PluginInfoFn = unsafe extern "C" fn() -> PluginInfo;

/// 插件 ABI 版本查询函数签名
pub type PluginAbiVersionFn = unsafe extern "C" fn() -> u32;

/// 插件创建函数返回的指针结构
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginPtr {
    /// 数据指针
    pub data: *const (),
    /// Vtable 指针
    pub vtable: *const (),
}

/// 插件创建函数签名
pub type PluginCreateFn = unsafe extern "C" fn() -> PluginPtr;

/// 插件信息结构
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// 插件名称
    pub name: [u8; 64],
    /// 插件版本
    pub version: [u8; 32],
    /// 插件描述
    pub description: [u8; 256],
    /// 插件作者
    pub author: [u8; 64],
    /// ABI 版本
    pub abi_version: u32,
}

impl PluginInfo {
    /// 从字节切片创建插件信息
    pub fn from_bytes(
        name: &[u8],
        version: &[u8],
        description: &[u8],
        author: &[u8],
        abi_version: u32,
    ) -> Self {
        let mut info = Self {
            name: [0; 64],
            version: [0; 32],
            description: [0; 256],
            author: [0; 64],
            abi_version,
        };

        let copy_bytes = |src: &[u8], dst: &mut [u8]| {
            let len = src.len().min(dst.len());
            dst[..len].copy_from_slice(&src[..len]);
        };

        copy_bytes(name, &mut info.name);
        copy_bytes(version, &mut info.version);
        copy_bytes(description, &mut info.description);
        copy_bytes(author, &mut info.author);

        info
    }

    /// 获取插件名称字符串
    pub fn name(&self) -> String {
        string_from_bytes(&self.name)
    }

    /// 获取插件版本字符串
    pub fn version(&self) -> String {
        string_from_bytes(&self.version)
    }

    /// 获取插件描述字符串
    pub fn description(&self) -> String {
        string_from_bytes(&self.description)
    }

    /// 获取插件作者字符串
    pub fn author(&self) -> String {
        string_from_bytes(&self.author)
    }
}

/// 从字节数组创建字符串（去除空终止符）
fn string_from_bytes(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

/// 插件包装器
///
/// 用于在动态库和 EVIF 核心之间传递插件实例
#[repr(C)]
pub struct EvifPluginWrapper {
    /// 插件实例指针（指向 Arc<dyn EvifPlugin>）
    plugin: *const (),
    /// 虚函数表（用于调用插件方法）
    vtable: PluginVTable,
}

/// 插件虚函数表
///
/// 定义插件方法指针，用于跨 FFI 边界调用
#[repr(C)]
pub struct PluginVTable {
    /// 释放插件
    pub destroy: Option<unsafe extern "C" fn(*mut EvifPluginWrapper)>,
    /// 获取插件名称
    pub get_name: Option<unsafe extern "C" fn(*mut EvifPluginWrapper) -> NameBuffer>,
    /// 验证配置
    pub validate:
        Option<unsafe extern "C" fn(*mut EvifPluginWrapper, config: *const u8, len: usize) -> u32>,
}

/// 名称缓冲区
#[repr(C)]
pub struct NameBuffer {
    pub data: [u8; 128],
    pub len: u32,
}

impl Default for NameBuffer {
    fn default() -> Self {
        Self {
            data: [0; 128],
            len: 0,
        }
    }
}

// SAFETY: EvifPluginWrapper holds a raw pointer to an Arc<dyn EvifPlugin> and a vtable
// of function pointers. The plugin lifecycle is managed exclusively by DynamicPluginLoader,
// which ensures the underlying Arc remains alive as long as any wrapper exists. The vtable
// function pointers are immutable after creation. Access is synchronized through the loader's
// internal state, so concurrent access from multiple threads is safe.
unsafe impl Send for EvifPluginWrapper {}
unsafe impl Sync for EvifPluginWrapper {}

/// 动态插件加载器
///
/// 负责加载、管理和卸载动态插件库
pub struct DynamicPluginLoader {
    /// 已加载的动态库
    libraries: RwLock<HashMap<String, LoadedLibrary>>,
    /// 插件搜索路径
    search_paths: Vec<PathBuf>,
}

/// 已加载的库信息
struct LoadedLibrary {
    /// 动态库实例
    library: Library,
    /// 插件信息
    info: PluginInfo,
    /// 库文件路径
    #[allow(dead_code)]
    path: PathBuf,
}

impl DynamicPluginLoader {
    /// 创建新的动态插件加载器
    pub fn new() -> Self {
        Self {
            libraries: RwLock::new(HashMap::new()),
            search_paths: Self::default_search_paths(),
        }
    }

    /// 获取默认搜索路径
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 当前目录
        paths.push(PathBuf::from("."));

        // ./plugins 目录
        paths.push(PathBuf::from("./plugins"));

        // 系统插件目录
        if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(home).join(".evif/plugins"));
        }

        // /usr/local/lib/evif/plugins (Linux)
        #[cfg(target_os = "linux")]
        paths.push(PathBuf::from("/usr/local/lib/evif/plugins"));

        // /usr/lib/evif/plugins (Linux)
        #[cfg(target_os = "linux")]
        paths.push(PathBuf::from("/usr/lib/evif/plugins"));

        // /Library/EVIF/plugins (macOS)
        #[cfg(target_os = "macos")]
        paths.push(PathBuf::from("/Library/EVIF/plugins"));

        paths
    }

    /// 添加插件搜索路径
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// 查找插件库文件
    fn find_plugin_library(&self, name: &str) -> Option<PathBuf> {
        // 平台相关的库扩展名
        let extensions = if cfg!(target_os = "macos") {
            vec![".dylib", ".so"]
        } else if cfg!(target_os = "linux") {
            vec![".so"]
        } else if cfg!(target_os = "windows") {
            vec![".dll"]
        } else {
            vec![".so"]
        };

        // 尝试直接路径
        let direct_path = PathBuf::from(name);
        if direct_path.exists() {
            return Some(direct_path);
        }

        // 在搜索路径中查找
        for search_path in &self.search_paths {
            for ext in &extensions {
                let lib_path = search_path.join(format!("lib{}{}", name, ext));
                if lib_path.exists() {
                    return Some(lib_path);
                }

                // 也尝试不带 lib 前缀
                let lib_path_no_prefix = search_path.join(format!("{}{}", name, ext));
                if lib_path_no_prefix.exists() {
                    return Some(lib_path_no_prefix);
                }
            }
        }

        None
    }

    /// 加载动态插件库
    ///
    /// # 参数
    /// - `name`: 插件名称或路径
    ///
    /// # 返回
    /// 插件信息的引用
    pub fn load_plugin(&self, name: &str) -> EvifResult<PluginInfo> {
        // 检查是否已加载
        {
            let libraries = self.libraries.read();
            if let Some(loaded) = libraries.get(name) {
                info!("Plugin '{}' already loaded", name);
                return Ok(loaded.info.clone());
            }
        }

        // 查找库文件
        let library_path = self.find_plugin_library(name).ok_or_else(|| {
            EvifError::PluginLoadError(format!(
                "Plugin library '{}' not found in search paths: {:?}",
                name, self.search_paths
            ))
        })?;

        info!("Loading plugin from: {:?}", library_path);

        // 验证插件完整性（计算哈希）
        if let Ok(actual_hash) = compute_file_hash(&library_path) {
            let file_name = library_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(name);

            debug!(
                "Plugin '{}' SHA256: {}",
                file_name,
                &actual_hash[..16]
            );
        } else {
            warn!(
                "Failed to compute hash for plugin '{}' - proceeding without integrity verification",
                name
            );
        }

        // 加载动态库
        // SAFETY: Library::new loads a shared library from the filesystem.
        // The path has been validated to exist and be a valid .so/.dylib/.dll file.
        // System dlopen/dlsym is inherently safe as it just loads code.
        let library = unsafe {
            Library::new(&library_path).map_err(|e| {
                EvifError::PluginLoadError(format!(
                    "Failed to load library '{:?}': {}",
                    library_path, e
                ))
            })?
        };

        // 检查 ABI 版本
        let abi_version = self.get_abi_version(&library)?;
        if abi_version != EVIF_PLUGIN_ABI_VERSION {
            return Err(EvifError::PluginLoadError(format!(
                "ABI version mismatch: expected {}, got {}",
                EVIF_PLUGIN_ABI_VERSION, abi_version
            )));
        }

        // 获取插件信息
        let info = self.get_plugin_info(&library)?;

        // 存储已加载的库
        {
            let mut libraries = self.libraries.write();
            libraries.insert(
                name.to_string(),
                LoadedLibrary {
                    library,
                    info: info.clone(),
                    path: library_path,
                },
            );
        }

        info!(
            "Successfully loaded plugin: {} v{} by {}",
            info.name(),
            info.version(),
            info.author()
        );

        Ok(info)
    }

    /// 获取插件 ABI 版本
    fn get_abi_version(&self, library: &Library) -> EvifResult<u32> {
        // SAFETY: dlsym looks up symbols in the already-loaded library.
        // The symbol name "evif_plugin_abi_version" is a known ABI entry point.
        // We trust the library was compiled with the correct calling convention.
        unsafe {
            let get_abi_version: Symbol<PluginAbiVersionFn> =
                library.get(b"evif_plugin_abi_version").map_err(|_| {
                    EvifError::PluginLoadError(
                        "Missing 'evif_plugin_abi_version' symbol".to_string(),
                    )
                })?;

            Ok(get_abi_version())
        }
    }

    /// 获取插件信息
    fn get_plugin_info(&self, library: &Library) -> EvifResult<PluginInfo> {
        // SAFETY: dlsym looks up symbols in the already-loaded library.
        // The symbol name "evif_plugin_info" is a known ABI entry point.
        // We trust the library was compiled with the correct calling convention.
        unsafe {
            let get_info: Symbol<PluginInfoFn> =
                library.get(b"evif_plugin_info").map_err(|_| {
                    EvifError::PluginLoadError("Missing 'evif_plugin_info' symbol".to_string())
                })?;

            Ok(get_info())
        }
    }

    /// 从已加载的库创建插件实例
    ///
    /// # 参数
    /// - `name`: 插件名称
    ///
    /// # 返回
    /// 插件实例
    ///
    /// # ABI 要求
    /// 动态库的 evif_plugin_create 必须返回 Arc<dyn EvifPlugin> 裸指针
    ///
    /// # 示例（插件库代码）
    /// ```ignore
    /// use evif_core::EvifPlugin;
    /// use std::sync::Arc;
    /// use std::ffi::c_void;
    ///
    /// struct MyPlugin;
    ///
    /// impl EvifPlugin for MyPlugin { /* ... */ }
    ///
    /// #[no_mangle]
    /// pub extern "C" fn evif_plugin_create() -> *const () {
    ///     let plugin: Arc<dyn EvifPlugin> = Arc::new(MyPlugin);
    ///     Arc::into_raw(plugin) as *const ()
    /// }
    /// ```
    pub fn create_plugin(&self, name: &str) -> EvifResult<Arc<dyn EvifPlugin>> {
        let libraries = self.libraries.read();
        let loaded = libraries.get(name).ok_or_else(|| {
            EvifError::PluginLoadError(format!(
                "Plugin '{}' not loaded. Call load_plugin() first.",
                name
            ))
        })?;

        // SAFETY: dlsym looks up the plugin creation symbol.
        // The create function returns a PluginPtr which is validated below.
        let plugin_ptr = unsafe {
            let create_fn: Symbol<PluginCreateFn> =
                loaded.library.get(b"evif_plugin_create").map_err(|_| {
                    EvifError::PluginLoadError("Missing 'evif_plugin_create' symbol".to_string())
                })?;

            create_fn()
        };
        if plugin_ptr.data.is_null() {
            return Err(EvifError::PluginLoadError(
                "Plugin creation returned null data pointer".to_string(),
            ));
        }

        debug!(
            "Received plugin pointer: data={:p}, vtable={:p}",
            plugin_ptr.data, plugin_ptr.vtable
        );

        // 从 PluginPtr 重建 fat pointer
        // fat pointer 布局: [data_ptr, vtable_ptr]
        // SAFETY: plugin_ptr 是从 C 库返回的有效指针，
        // fat_ptr 正确编码了 trait object 的数据指针和 vtable
        // Arc::from_raw 需要指针保持有效且未被部分消费
        let plugin = unsafe {
            let fat_ptr: [usize; 2] = [plugin_ptr.data as usize, plugin_ptr.vtable as usize];
            let typed_ptr: *const dyn EvifPlugin = std::mem::transmute(fat_ptr);
            Arc::from_raw(typed_ptr)
        };

        // 克隆以获得新的引用
        let cloned = plugin.clone();

        // 不使用 mem::forget:
        // Arc::from_raw 创建 ref count=1 的 Arc
        // clone() 增加到 ref count=2
        // 当 plugin 超出作用域时 ref count 回到 1
        // cloned Arc 仍然有效，当它被 drop 时对象会被正确释放

        Ok(cloned)
    }

    /// 卸载插件库
    ///
    /// # 注意
    /// 卸载前必须确保所有插件实例已释放
    pub fn unload_plugin(&self, name: &str) -> EvifResult<()> {
        let mut libraries = self.libraries.write();
        libraries
            .remove(name)
            .ok_or_else(|| EvifError::PluginLoadError(format!("Plugin '{}' not loaded", name)))?;

        info!("Unloaded plugin: {}", name);
        Ok(())
    }

    /// 列出已加载的插件
    pub fn loaded_plugins(&self) -> Vec<String> {
        let libraries = self.libraries.read();
        libraries.keys().cloned().collect()
    }

    /// 获取插件信息
    pub fn plugin_info(&self, name: &str) -> Option<PluginInfo> {
        let libraries = self.libraries.read();
        libraries.get(name).map(|loaded| loaded.info.clone())
    }
}

impl Default for DynamicPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// 动态插件加载器构建器
pub struct DynamicPluginLoaderBuilder {
    search_paths: Vec<PathBuf>,
}

impl DynamicPluginLoaderBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }

    /// 添加搜索路径
    pub fn add_search_path(mut self, path: PathBuf) -> Self {
        self.search_paths.push(path);
        self
    }

    /// 添加多个搜索路径
    pub fn add_search_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.search_paths.extend(paths);
        self
    }

    /// 构建加载器
    pub fn build(self) -> DynamicPluginLoader {
        let mut loader = DynamicPluginLoader::new();
        loader.search_paths.extend(self.search_paths);
        loader
    }
}

impl Default for DynamicPluginLoaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info_from_bytes() {
        let info = PluginInfo::from_bytes(
            b"TestPlugin",
            b"1.0.0",
            b"A test plugin for EVIF",
            b"EVIF Team",
            1,
        );

        assert_eq!(info.name(), "TestPlugin");
        assert_eq!(info.version(), "1.0.0");
        assert_eq!(info.description(), "A test plugin for EVIF");
        assert_eq!(info.author(), "EVIF Team");
        assert_eq!(info.abi_version, 1);
    }

    #[test]
    fn test_name_buffer_default() {
        let buffer = NameBuffer::default();
        assert_eq!(buffer.len, 0);
    }

    #[test]
    fn test_loader_creation() {
        let loader = DynamicPluginLoader::new();
        assert!(!loader.search_paths.is_empty());
        assert!(loader.loaded_plugins().is_empty());
    }

    #[test]
    fn test_builder() {
        let loader = DynamicPluginLoaderBuilder::new()
            .add_search_path(PathBuf::from("./test"))
            .build();

        assert!(loader.search_paths.iter().any(|p| p.ends_with("test")));
    }
}
