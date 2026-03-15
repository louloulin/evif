// Plugin Registry - 插件注册表
//
// 插件生命周期管理和状态追踪
// 支持运行时插件状态查询、热重载、依赖管理

use crate::dynamic_loader::PluginInfo;
use crate::error::{EvifError, EvifResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

/// 插件生命周期状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginState {
    /// 插件正在加载
    Loading,
    /// 插件已加载但未激活
    Loaded,
    /// 插件已激活并可用
    Active,
    /// 插件已挂起（临时不可用）
    Inactive,
    /// 插件正在卸载
    Unloading,
    /// 插件加载/卸载失败
    Error(String),
}

impl std::fmt::Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginState::Loading => write!(f, "loading"),
            PluginState::Loaded => write!(f, "loaded"),
            PluginState::Active => write!(f, "active"),
            PluginState::Inactive => write!(f, "inactive"),
            PluginState::Unloading => write!(f, "unloading"),
            PluginState::Error(msg) => write!(f, "error: {}", msg),
        }
    }
}

/// 插件注册信息
#[derive(Debug, Clone)]
pub struct RegisteredPlugin {
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件作者
    pub author: String,
    /// 插件描述
    pub description: String,
    /// 当前状态
    pub state: PluginState,
    /// 挂载路径（如果已挂载）
    pub mount_path: Option<String>,
    /// 库文件路径
    pub library_path: String,
    /// 加载时间
    pub loaded_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// 加载/激活失败次数
    pub failure_count: u32,
}

impl RegisteredPlugin {
    /// 创建新的插件注册信息
    pub fn new(name: String, info: PluginInfo, library_path: String) -> Self {
        let now = Utc::now();
        Self {
            name,
            version: info.version(),
            author: info.author(),
            description: info.description(),
            state: PluginState::Loaded,
            mount_path: None,
            library_path,
            loaded_at: now,
            last_active_at: now,
            failure_count: 0,
        }
    }

    /// 激活插件
    pub fn activate(&mut self, mount_path: String) {
        self.state = PluginState::Active;
        self.mount_path = Some(mount_path);
        self.last_active_at = Utc::now();
    }

    /// 停用插件
    pub fn deactivate(&mut self) {
        self.state = PluginState::Inactive;
        self.mount_path = None;
    }

    /// 记录失败
    pub fn record_failure(&mut self, error: String) {
        self.failure_count += 1;
        self.state = PluginState::Error(error);
    }

    /// 重置失败计数
    pub fn reset_failures(&mut self) {
        self.failure_count = 0;
    }
}

/// 插件注册表
///
/// 负责追踪所有已加载插件的状态和元信息
pub struct PluginRegistry {
    /// 已注册的插件
    plugins: RwLock<HashMap<String, RegisteredPlugin>>,
    /// 插件搜索路径
    search_paths: RwLock<Vec<String>>,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            search_paths: RwLock::new(Vec::new()),
        }
    }

    /// 注册新插件
    pub fn register(&self, name: String, info: PluginInfo, library_path: String) {
        let mut plugins = self.plugins.write().unwrap();
        let plugin = RegisteredPlugin::new(name.clone(), info, library_path);
        info!("Registering plugin: {} (state: {:?})", name, plugin.state);
        plugins.insert(name, plugin);
    }

    /// 卸载插件
    pub fn unregister(&self, name: &str) -> EvifResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        if plugins.remove(name).is_some() {
            info!("Unregistered plugin: {}", name);
            Ok(())
        } else {
            warn!("Plugin not found for unregister: {}", name);
            Err(EvifError::NotFound(format!("Plugin not found: {}", name)))
        }
    }

    /// 获取插件信息
    pub fn get(&self, name: &str) -> Option<RegisteredPlugin> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(name).cloned()
    }

    /// 获取所有插件
    pub fn list_all(&self) -> Vec<RegisteredPlugin> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().cloned().collect()
    }

    /// 获取指定状态的插件
    pub fn list_by_state(&self, state: &PluginState) -> Vec<RegisteredPlugin> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .values()
            .filter(|p| &p.state == state)
            .cloned()
            .collect()
    }

    /// 获取活跃插件
    pub fn list_active(&self) -> Vec<RegisteredPlugin> {
        self.list_by_state(&PluginState::Active)
    }

    /// 激活插件
    pub fn activate(&self, name: &str, mount_path: String) -> EvifResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        if let Some(plugin) = plugins.get_mut(name) {
            plugin.activate(mount_path);
            info!("Activated plugin: {}", name);
            Ok(())
        } else {
            Err(EvifError::NotFound(format!("Plugin not found: {}", name)))
        }
    }

    /// 停用插件
    pub fn deactivate(&self, name: &str) -> EvifResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        if let Some(plugin) = plugins.get_mut(name) {
            plugin.deactivate();
            info!("Deactivated plugin: {}", name);
            Ok(())
        } else {
            Err(EvifError::NotFound(format!("Plugin not found: {}", name)))
        }
    }

    /// 记录插件失败
    pub fn record_failure(&self, name: &str, error: String) -> EvifResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        if let Some(plugin) = plugins.get_mut(name) {
            plugin.record_failure(error.clone());
            warn!("Plugin {} failed: {}", name, error);
            Ok(())
        } else {
            Err(EvifError::NotFound(format!("Plugin not found: {}", name)))
        }
    }

    /// 检查插件是否存在
    pub fn exists(&self, name: &str) -> bool {
        let plugins = self.plugins.read().unwrap();
        plugins.contains_key(name)
    }

    /// 检查插件是否活跃
    pub fn is_active(&self, name: &str) -> bool {
        let plugins = self.plugins.read().unwrap();
        matches!(plugins.get(name), Some(p) if p.state == PluginState::Active)
    }

    /// 获取活跃插件数量
    pub fn active_count(&self) -> usize {
        let plugins = self.plugins.read().unwrap();
        plugins
            .values()
            .filter(|p| p.state == PluginState::Active)
            .count()
    }

    /// 获取总插件数量
    pub fn total_count(&self) -> usize {
        let plugins = self.plugins.read().unwrap();
        plugins.len()
    }

    /// 添加搜索路径
    pub fn add_search_path(&self, path: String) {
        let mut paths = self.search_paths.write().unwrap();
        paths.push(path);
    }

    /// 获取搜索路径
    pub fn search_paths(&self) -> Vec<String> {
        let paths = self.search_paths.read().unwrap();
        paths.clone()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 插件注册表 Arc 包装器，便于共享
pub type PluginRegistryRef = Arc<PluginRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.total_count(), 0);
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_plugin_state_display() {
        assert_eq!(PluginState::Loading.to_string(), "loading");
        assert_eq!(PluginState::Active.to_string(), "active");
        assert_eq!(
            PluginState::Error("test".to_string()).to_string(),
            "error: test"
        );
    }
}
