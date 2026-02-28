// KVFS - 键值存储插件
//
// 对标 AGFS KVFS

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::Utc;

/// 简单的内存键值存储
pub struct KvStore {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> EvifResult<Option<Vec<u8>>> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    pub async fn put(&self, key: String, value: Vec<u8>) -> EvifResult<()> {
        let mut data = self.data.write().await;
        data.insert(key, value);
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> EvifResult<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    pub async fn list_keys(&self) -> EvifResult<Vec<String>> {
        let data = self.data.read().await;
        Ok(data.keys().cloned().collect())
    }
}

pub struct KvfsPlugin {
    store: Arc<KvStore>,
    prefix: String,
}

impl KvfsPlugin {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            store: Arc::new(KvStore::new()),
            prefix: prefix.into(),
        }
    }

    /// 将文件路径转换为存储key
    fn path_to_key(&self, path: &str) -> EvifResult<String> {
        let clean_path = path.trim_start_matches('/');
        if clean_path.is_empty() {
            return Err(EvifError::InvalidPath("Path cannot be empty".to_string()));
        }
        Ok(format!("{}/{}", self.prefix.trim_end_matches('/'), clean_path))
    }

    /// 将文件路径转换为key前缀（用于列出目录）
    fn path_to_prefix(&self, path: &str) -> EvifResult<String> {
        let clean_path = path.trim_start_matches('/');
        let base = self.prefix.trim_end_matches('/');
        if clean_path.is_empty() || clean_path == "/" {
            Ok(format!("{}/", base))
        } else {
            Ok(format!("{}/{}/", base, clean_path.trim_end_matches('/')))
        }
    }

    /// 从key中提取相对路径
    fn key_to_relative_path(&self, key: &str) -> String {
        key.trim_start_matches(&format!("{}/", self.prefix.trim_end_matches('/')))
            .to_string()
    }
}

#[async_trait]
impl EvifPlugin for KvfsPlugin {
    fn name(&self) -> &str {
        "kvfs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let key = self.path_to_key(path)?;
        // 创建空值
        self.store.put(key, vec![]).await?;
        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // KVFS 中目录是虚拟的，不需要实际创建
        // 只需要确保前缀有效
        self.path_to_prefix(path)?;
        Ok(())
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let key = self.path_to_key(path)?;
        self.store.get(&key).await?
            .ok_or_else(|| EvifError::NotFound(key))
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        let key = self.path_to_key(path)?;
        let len = data.len() as u64;
        self.store.put(key, data).await?;
        Ok(len)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let prefix = self.path_to_prefix(path)?;
        let all_keys = self.store.list_keys().await?;

        // 过滤出匹配前缀的key
        let matching_keys: Vec<String> = all_keys.into_iter()
            .filter(|k| k.starts_with(&prefix))
            .collect();

        // 计算需要跳过的路径深度
        let path_depth = path.trim_start_matches('/').trim_end_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .count();

        // 提取唯一的文件/目录名
        let mut entries = HashMap::new();
        for key in matching_keys {
            let relative = self.key_to_relative_path(&key);
            let parts: Vec<&str> = relative.split('/').collect();

            // 跳过路径部分，获取下一级的名称
            if parts.len() > path_depth {
                let name = parts[path_depth].to_string();

                // 检查是否是目录（还有更深的层级）
                let is_dir = parts.len() > path_depth + 1;

                entries.entry(name.clone())
                    .or_insert_with(|| FileInfo {
                        name: name.clone(),
                        size: 0,
                        mode: if is_dir { 0o755 } else { 0o644 },
                        modified: Utc::now(),
                        is_dir,
                    });
            }
        }

        Ok(entries.into_values().collect())
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let key = self.path_to_key(path)?;

        // 尝试获取值
        if let Some(data) = self.store.get(&key).await? {
            Ok(FileInfo {
                name: path.trim_start_matches('/')
                    .split('/')
                    .last()
                    .unwrap_or("unknown")
                    .to_string(),
                size: data.len() as u64,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            })
        } else {
            // 检查是否是目录（存在以此为前缀的key）
            let prefix = format!("{}/", key.trim_end_matches('/'));
            let all_keys = self.store.list_keys().await?;
            let has_children = all_keys.iter().any(|k| k.starts_with(&prefix));

            if has_children {
                Ok(FileInfo {
                    name: path.trim_start_matches('/')
                        .split('/')
                        .last()
                        .unwrap_or("unknown")
                        .to_string(),
                    size: 0,
                    mode: 0o755,
                    modified: Utc::now(),
                    is_dir: true,
                })
            } else {
                Err(EvifError::NotFound(key))
            }
        }
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let key = self.path_to_key(path)?;

        // 检查是否存在
        if self.store.get(&key).await?.is_some() {
            self.store.delete(&key).await?;
            Ok(())
        } else {
            Err(EvifError::NotFound(key))
        }
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let old_key = self.path_to_key(old_path)?;
        let new_key = self.path_to_key(new_path)?;

        // 获取旧值
        let data = self.store.get(&old_key).await?
            .ok_or_else(|| EvifError::NotFound(old_key.clone()))?;

        // 写入新位置
        self.store.put(new_key, data).await?;

        // 删除旧位置
        self.store.delete(&old_key).await?;

        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        // KVFS 中删除所有匹配前缀的键
        let prefix = self.path_to_prefix(path)?;
        let all_keys = self.store.list_keys().await?;

        // 找到所有以该前缀开头的键
        for key in all_keys {
            if key.starts_with(&prefix) {
                self.store.delete(&key).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kvfs_basic() {
        let plugin = KvfsPlugin::new("kvfs");

        let test_data = b"Hello, World!";  // 13 bytes

        // 测试创建和写入（不使用create，直接write）
        plugin.write("/mykey", test_data.to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 测试读取
        let data = plugin.read("/mykey", 0, 100).await.unwrap();
        assert_eq!(data, test_data);

        // 测试 stat
        let info = plugin.stat("/mykey").await.unwrap();
        assert_eq!(data.len(), 13);
        assert_eq!(info.size, 13);
        assert!(!info.is_dir);

        // 测试目录
        plugin.mkdir("/subdir", 0o755).await.unwrap();
        plugin.write("/subdir/key1", b"value1".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "mykey"));
        assert!(entries.iter().any(|e| e.name == "subdir"));

        // 测试子目录
        let subdir_entries = plugin.readdir("/subdir").await.unwrap();
        assert!(subdir_entries.iter().any(|e| e.name == "key1"));

        // 测试重命名
        plugin.rename("/mykey", "/mykey_renamed").await.unwrap();
        let data = plugin.read("/mykey_renamed", 0, 100).await.unwrap();
        assert_eq!(data, b"Hello, World!");

        // 测试删除
        plugin.remove("/mykey_renamed").await.unwrap();
        let result = plugin.stat("/mykey_renamed").await;
        assert!(result.is_err());
    }
}
