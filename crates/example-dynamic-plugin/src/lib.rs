// Example Dynamic Plugin for EVIF
// This plugin demonstrates how to create a dynamically loadable plugin

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::EvifPlugin;
use evif_core::{EvifError, EvifResult, FileInfo, OpenFlags, WriteFlags};
use std::collections::HashMap;

pub struct ExampleDynamicPlugin {
    name: String,
    data: tokio::sync::RwLock<HashMap<String, Vec<u8>>>,
}

impl ExampleDynamicPlugin {
    pub fn new() -> Self {
        Self {
            name: "example-dynamic".to_string(),
            data: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for ExampleDynamicPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for ExampleDynamicPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let mut data_store = self.data.write().await;
        data_store.insert(path.to_string(), Vec::new());
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Ok(())
    }

    async fn read(&self, path: &str, _offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let data_store = self.data.read().await;
        if let Some(content) = data_store.get(path) {
            if size == 0 {
                Ok(content.clone())
            } else {
                let end = (size as usize).min(content.len());
                Ok(content[..end].to_vec())
            }
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        let mut data_store = self.data.write().await;
        data_store.insert(path.to_string(), data);
        Ok(data_store.get(path).map(|d| d.len() as u64).unwrap_or(0))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let data_store = self.data.read().await;
        let mut files = Vec::new();
        let path_prefix = if path == "/" || path.is_empty() {
            String::new()
        } else {
            format!("{}/", path.trim_start_matches('/'))
        };
        for (key, value) in data_store.iter() {
            if key.starts_with(&path_prefix) {
                let remaining = &key[path_prefix.len()..];
                let name = if remaining.contains('/') {
                    remaining[..remaining.find('/').unwrap()].to_string()
                } else {
                    remaining.to_string()
                };
                files.push(FileInfo {
                    name,
                    size: value.len() as u64,
                    mode: 0o644,
                    modified: DateTime::default(),
                    is_dir: false,
                });
            }
        }
        Ok(files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let data_store = self.data.read().await;
        if let Some(content) = data_store.get(path) {
            Ok(FileInfo {
                name: path.rsplit('/').next().unwrap_or(path).to_string(),
                size: content.len() as u64,
                mode: 0o644,
                modified: DateTime::default(),
                is_dir: false,
            })
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let mut data_store = self.data.write().await;
        data_store
            .remove(path)
            .ok_or_else(|| EvifError::NotFound(path.to_string()))?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> EvifResult<()> {
        let mut data_store = self.data.write().await;
        if let Some(content) = data_store.remove(from) {
            data_store.insert(to.to_string(), content);
            Ok(())
        } else {
            Err(EvifError::NotFound(from.to_string()))
        }
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }

    async fn symlink(&self, _from: &str, _to: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn readlink(&self, _path: &str) -> EvifResult<String> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn chmod(&self, _path: &str, _mode: u32) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()> {
        let mut data_store = self.data.write().await;
        if let Some(content) = data_store.get_mut(path) {
            content.resize(size.min(1024 * 1024) as usize, 0);
            Ok(())
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }

    fn get_readme(&self) -> String {
        "Example Dynamic Plugin for EVIF\n\nA dynamically loadable plugin demonstrating the EVIF plugin API.".to_string()
    }

    fn get_config_params(&self) -> Vec<evif_core::PluginConfigParam> {
        vec![]
    }
}

// ABI exports
use evif_core::{PluginInfo, PluginPtr, EVIF_PLUGIN_ABI_VERSION};
use std::sync::Arc;

#[no_mangle]
pub extern "C" fn evif_plugin_abi_version() -> u32 {
    EVIF_PLUGIN_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn evif_plugin_info() -> PluginInfo {
    let name = b"ExampleDynamicPlugin";
    let version = b"0.1.0";
    let description = b"A dynamically loadable example plugin for EVIF";
    let author = b"EVIF Team";

    let mut info = PluginInfo {
        name: [0; 64],
        version: [0; 32],
        description: [0; 256],
        author: [0; 64],
        abi_version: EVIF_PLUGIN_ABI_VERSION,
    };

    let name_len = name.len().min(63);
    info.name[..name_len].copy_from_slice(name);

    let version_len = version.len().min(31);
    info.version[..version_len].copy_from_slice(version);

    let desc_len = description.len().min(255);
    info.description[..desc_len].copy_from_slice(description);

    let author_len = author.len().min(63);
    info.author[..author_len].copy_from_slice(author);

    info
}

#[no_mangle]
pub extern "C" fn evif_plugin_create() -> PluginPtr {
    let plugin: Arc<dyn EvifPlugin> = Arc::new(ExampleDynamicPlugin::new());
    let fat_ptr = Arc::into_raw(plugin) as *const dyn EvifPlugin;

    // 将 fat pointer 拆解为 data 和 vtable 指针
    // fat pointer 布局: [data_ptr, vtable_ptr]
    let fat_ptr_usize: [usize; 2] = unsafe { std::mem::transmute(fat_ptr) };

    PluginPtr {
        data: fat_ptr_usize[0] as *const (),
        vtable: fat_ptr_usize[1] as *const (),
    }
}
