// Dynamic Plugin Loader - 动态插件加载系统
//
// 对标 AGFS PluginFactory 动态加载机制
// 支持运行时加载 .so/.dylib/.dll 插件
//
// # AGFS 对标
// ```go
// // AGFS PluginFactory 从动态库加载插件
// plugin, err := factory.LoadPlugin("myplugin.so")
// ```

use crate::error::{EvifError, EvifResult};
use crate::plugin::EvifPlugin;
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};

/// 插件 ABI 版本
/// 用于确保动态库与 EVIF 核心兼容
pub const EVIF_PLUGIN_ABI_VERSION: u32 = 1;

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
            let libraries = self.libraries.read().unwrap();
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

        // 加载动态库
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
            let mut libraries = self.libraries.write().unwrap();
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
    /// ```no_run
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
        let libraries = self.libraries.read().unwrap();
        let loaded = libraries.get(name).ok_or_else(|| {
            EvifError::PluginLoadError(format!(
                "Plugin '{}' not loaded. Call load_plugin() first.",
                name
            ))
        })?;

        unsafe {
            let create_fn: Symbol<PluginCreateFn> =
                loaded.library.get(b"evif_plugin_create").map_err(|_| {
                    EvifError::PluginLoadError("Missing 'evif_plugin_create' symbol".to_string())
                })?;

            let plugin_ptr = create_fn();
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
            let fat_ptr: [usize; 2] = [plugin_ptr.data as usize, plugin_ptr.vtable as usize];
            let typed_ptr: *const dyn EvifPlugin = std::mem::transmute(fat_ptr);

            // 将裸指针转换回 Arc<dyn EvifPlugin>
            let plugin = Arc::from_raw(typed_ptr);

            // 克隆以获得新的引用
            let cloned = plugin.clone();

            // 防止原始 Arc 被释放（泄漏指针）
            // 注意：这会导致内存泄漏，除非有相应的清理机制
            std::mem::forget(plugin);

            Ok(cloned)
        }
    }

    /// 卸载插件库
    ///
    /// # 注意
    /// 卸载前必须确保所有插件实例已释放
    pub fn unload_plugin(&self, name: &str) -> EvifResult<()> {
        let mut libraries = self.libraries.write().unwrap();
        libraries
            .remove(name)
            .ok_or_else(|| EvifError::PluginLoadError(format!("Plugin '{}' not loaded", name)))?;

        info!("Unloaded plugin: {}", name);
        Ok(())
    }

    /// 列出已加载的插件
    pub fn loaded_plugins(&self) -> Vec<String> {
        let libraries = self.libraries.read().unwrap();
        libraries.keys().cloned().collect()
    }

    /// 获取插件信息
    pub fn plugin_info(&self, name: &str) -> Option<PluginInfo> {
        let libraries = self.libraries.read().unwrap();
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
        assert!(loader.search_paths.len() > 0);
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
