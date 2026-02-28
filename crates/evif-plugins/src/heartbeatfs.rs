// HeartbeatFS - 心跳监控服务插件
//
// 提供服务健康检查和心跳监控功能

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, BinaryHeap};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// 心跳项
#[derive(Clone)]
struct HeartbeatItem {
    name: String,
    last_heartbeat: Instant,
    expire_time: Instant,
    timeout: Duration,
}

/// 过期堆项 (用于 BinaryHeap)
#[derive(Clone)]
struct ExpiryHeapItem {
    expire_time: Instant,
    name: String,
}

// 实现 Ord 使得 BinaryHeap 成为最小堆 (最早过期的在堆顶)
impl PartialEq for ExpiryHeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.expire_time == other.expire_time
    }
}

impl Eq for ExpiryHeapItem {}

impl PartialOrd for ExpiryHeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpiryHeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // 反向比较,使得最小堆成为最小过期时间在堆顶
        other.expire_time.cmp(&self.expire_time)
    }
}

/// HeartbeatFS 配置
#[derive(Clone, Debug)]
pub struct HeartbeatConfig {
    pub default_timeout: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(300), // 5 分钟
        }
    }
}

/// HeartbeatFS 插件
pub struct HeartbeatFsPlugin {
    items: Arc<RwLock<HashMap<String, HeartbeatItem>>>,
    expiry_heap: Arc<RwLock<BinaryHeap<ExpiryHeapItem>>>,
    config: HeartbeatConfig,
}

impl HeartbeatFsPlugin {
    /// 创建新的 HeartbeatFS 插件
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            expiry_heap: Arc::new(RwLock::new(BinaryHeap::new())),
            config: HeartbeatConfig::default(),
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(config: HeartbeatConfig) -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            expiry_heap: Arc::new(RwLock::new(BinaryHeap::new())),
            config,
        }
    }

    /// 清理过期项 (后台任务)
    pub async fn cleanup_expired(&self) {
        let now = Instant::now();

        // 查找所有过期的项
        let expired_names: Vec<String> = {
            let heap = self.expiry_heap.read().await;
            heap.iter()
                .filter(|item| item.expire_time <= now)
                .map(|item| item.name.clone())
                .collect()
        };

        if !expired_names.is_empty() {
            // 从 items 中删除
            let mut items = self.items.write().await;
            let mut heap = self.expiry_heap.write().await;

            for name in &expired_names {
                items.remove(name);
            }

            // 重建堆 (移除过期项)
            *heap = heap.iter()
                .filter(|item| item.expire_time > now)
                .cloned()
                .collect();
        }
    }

    /// 解析路径
    fn parse_path(&self, path: &str) -> EvifResult<(String, Option<String>)> {
        let path = path.trim_start_matches('/');

        if path.is_empty() || path == "README" {
            return Ok((String::new(), None));
        }

        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() == 1 {
            Ok((parts[0].to_string(), None))
        } else if parts.len() == 2 {
            Ok((parts[0].to_string(), Some(parts[1].to_string())))
        } else {
            Err(EvifError::InvalidPath(format!("invalid path: {}", path)))
        }
    }

    /// 更新心跳
    async fn update_heartbeat(&self, name: &str) -> EvifResult<()> {
        let mut items = self.items.write().await;
        let mut heap = self.expiry_heap.write().await;

        if let Some(item) = items.get_mut(name) {
            let now = Instant::now();
            item.last_heartbeat = now;
            item.expire_time = now + item.timeout;

            // 更新堆
            let heap_item = ExpiryHeapItem {
                expire_time: item.expire_time,
                name: name.to_string(),
            };
            heap.push(heap_item);

            Ok(())
        } else {
            Err(EvifError::NotFound(name.to_string()))
        }
    }

    /// 更新超时时间
    async fn update_timeout(&self, name: &str, timeout_secs: u64) -> EvifResult<()> {
        let mut items = self.items.write().await;
        let mut heap = self.expiry_heap.write().await;

        if let Some(item) = items.get_mut(name) {
            let new_timeout = Duration::from_secs(timeout_secs);
            item.timeout = new_timeout;
            item.expire_time = item.last_heartbeat + new_timeout;

            // 更新堆
            let heap_item = ExpiryHeapItem {
                expire_time: item.expire_time,
                name: name.to_string(),
            };
            heap.push(heap_item);

            Ok(())
        } else {
            Err(EvifError::NotFound(name.to_string()))
        }
    }

    /// 获取心跳状态
    async fn get_status(&self, name: &str) -> EvifResult<String> {
        let items = self.items.read().await;

        if let Some(item) = items.get(name) {
            let now = Instant::now();
            let status = if now <= item.expire_time {
                "alive"
            } else {
                "expired"
            };

            let last_heartbeat_dt: DateTime<Utc> = DateTime::from_timestamp(
                (item.last_heartbeat.elapsed().as_secs() as i64),
                0
            ).unwrap_or_else(|| Utc::now());

            let expire_dt: DateTime<Utc> = DateTime::from_timestamp(
                (item.expire_time.elapsed().as_secs() as i64),
                0
            ).unwrap_or_else(|| Utc::now());

            Ok(format!(
                "last_heartbeat_ts: {}\nexpire_ts: {}\ntimeout: {}\nstatus: {}\n",
                last_heartbeat_dt.format("%Y-%m-%dT%H:%M:%SZ"),
                expire_dt.format("%Y-%m-%dT%H:%M:%SZ"),
                item.timeout.as_secs(),
                status
            ))
        } else {
            Err(EvifError::NotFound(name.to_string()))
        }
    }
}

impl Default for HeartbeatFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for HeartbeatFsPlugin {
    fn name(&self) -> &str {
        "heartbeatfs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let (name, file) = self.parse_path(path)?;

        if file.is_some() {
            return Err(EvifError::InvalidPath("use mkdir to create heartbeat items".to_string()));
        }

        if name.is_empty() || name == "README" {
            return Err(EvifError::InvalidPath(format!("invalid heartbeat item name: {}", name)));
        }

        let mut items = self.items.write().await;
        let mut heap = self.expiry_heap.write().await;

        if items.contains_key(&name) {
            return Err(EvifError::InvalidPath(format!("heartbeat item already exists: {}", name)));
        }

        let now = Instant::now();
        let expire_time = now + self.config.default_timeout;

        let item = HeartbeatItem {
            name: name.clone(),
            last_heartbeat: now,
            expire_time,
            timeout: self.config.default_timeout,
        };

        items.insert(name.clone(), item);

        let heap_item = ExpiryHeapItem {
            expire_time,
            name,
        };
        heap.push(heap_item);

        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        self.create(path, 0).await
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let (name, file) = self.parse_path(path)?;

        if name.is_empty() {
            return Err(EvifError::InvalidPath("is a directory".to_string()));
        }

        if name == "README" {
            let readme = r#"HeartbeatFS Plugin - Heartbeat Monitoring Service

This plugin provides a heartbeat monitoring service through a file system interface.

USAGE:
  Create a new heartbeat item:
    mkdir /heartbeatfs/<name>

  Update heartbeat (keepalive):
    touch /heartbeatfs/<name>/keepalive
    echo "ping" > /heartbeatfs/<name>/keepalive

  Update timeout:
    echo "timeout=60" > /heartbeatfs/<name>/ctl

  Check heartbeat status:
    cat /heartbeatfs/<name>/ctl

  Check if heartbeat is alive (stat will fail if expired):
    stat /heartbeatfs/<name>

  List all heartbeat items:
    ls /heartbeatfs

  Remove heartbeat item:
    rm -r /heartbeatfs/<name>

STRUCTURE:
  /<name>/           - Directory for each heartbeat item (auto-deleted when expired)
  /<name>/keepalive  - Touch or write to update heartbeat
  /<name>/ctl        - Read to get status, write to update timeout (timeout=N in seconds)
  /README            - This file

BEHAVIOR:
  - Default timeout: 5 minutes (300 seconds) from last heartbeat
  - Timeout can be customized per item by writing to ctl file
  - Expired items are automatically removed by the system
  - Use stat to check if an item still exists (alive)
"#;
            return Ok(readme.as_bytes().to_vec());
        }

        let file = file.ok_or_else(|| EvifError::InvalidPath("invalid path".to_string()))?;

        match file.as_str() {
            "keepalive" => Ok(Vec::new()),
            "ctl" => {
                let status = self.get_status(&name).await?;
                Ok(status.into_bytes())
            }
            _ => Err(EvifError::InvalidPath(format!("invalid file: {}", file))),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        let (name, file) = self.parse_path(path)?;

        if name.is_empty() || name == "README" {
            return Err(EvifError::InvalidPath("invalid path".to_string()));
        }

        let file = file.ok_or_else(|| EvifError::InvalidPath("invalid path".to_string()))?;

        match file.as_str() {
            "keepalive" => {
                self.update_heartbeat(&name).await?;
                Ok(data.len() as u64)
            }
            "ctl" => {
                // 解析 timeout=N
                let content = String::from_utf8_lossy(&data);
                let content = content.trim();

                if let Some(rest) = content.strip_prefix("timeout=") {
                    let timeout_secs: u64 = rest.parse()
                        .map_err(|_| EvifError::InvalidPath("invalid timeout value".to_string()))?;

                    if timeout_secs == 0 {
                        return Err(EvifError::InvalidPath("timeout must be positive".to_string()));
                    }

                    self.update_timeout(&name, timeout_secs).await?;
                    Ok(data.len() as u64)
                } else {
                    Err(EvifError::InvalidPath("invalid ctl command, use 'timeout=N' (seconds)".to_string()))
                }
            }
            _ => Err(EvifError::InvalidPath("can only write to keepalive or ctl files".to_string())),
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let (name, _) = self.parse_path(path)?;

        if name.is_empty() {
            // 根目录
            let items = self.items.read().await;
            let mut files = Vec::new();

            // 添加 README
            files.push(FileInfo {
                name: "README".to_string(),
                size: 0,
                mode: 0o444,
                modified: Utc::now(),
                is_dir: false,
            });

            // 添加所有心跳项
            for name in items.keys() {
                files.push(FileInfo {
                    name: name.clone(),
                    size: 0,
                    mode: 0o755,
                    modified: Utc::now(),
                    is_dir: true,
                });
            }

            Ok(files)
        } else {
            // 心跳项目录
            Ok(vec![
                FileInfo {
                    name: "keepalive".to_string(),
                    size: 0,
                    mode: 0o644,
                    modified: Utc::now(),
                    is_dir: false,
                },
                FileInfo {
                    name: "ctl".to_string(),
                    size: 0,
                    mode: 0o644,
                    modified: Utc::now(),
                    is_dir: false,
                },
            ])
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let (name, file) = self.parse_path(path)?;

        if name.is_empty() {
            return Ok(FileInfo {
                name: "/".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        if name == "README" {
            return Ok(FileInfo {
                name: "README".to_string(),
                size: 0,
                mode: 0o444,
                modified: Utc::now(),
                is_dir: false,
            });
        }

        let items = self.items.read().await;

        if !items.contains_key(&name) {
            return Err(EvifError::NotFound(name.to_string()));
        }

        if file.is_none() {
            // 目录本身
            Ok(FileInfo {
                name: name.clone(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            })
        } else {
            // 目录中的文件
            let file = file.unwrap();
            Ok(FileInfo {
                name: file,
                size: 0,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            })
        }
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.remove_all(path).await
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("rename not supported in heartbeatfs".to_string()))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let (name, _) = self.parse_path(path)?;

        if name.is_empty() {
            return Err(EvifError::InvalidPath("cannot remove root".to_string()));
        }

        if name == "README" {
            return Err(EvifError::InvalidPath("cannot remove README".to_string()));
        }

        let mut items = self.items.write().await;

        if !items.contains_key(&name) {
            return Err(EvifError::NotFound(name.to_string()));
        }

        items.remove(&name);

        // 清理堆 (重建)
        let mut heap = self.expiry_heap.write().await;
        *heap = heap.iter()
            .filter(|item| item.name != name)
            .cloned()
            .collect();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heartbeatfs_basic() {
        let plugin = HeartbeatFsPlugin::new();

        // 创建心跳项
        plugin.mkdir("/service1", 0o755).await.unwrap();

        // 检查是否存在
        let info = plugin.stat("/service1").await.unwrap();
        assert_eq!(info.name, "service1");
        assert!(info.is_dir);

        // 列出目录
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "service1"));

        // 删除
        plugin.remove_all("/service1").await.unwrap();

        // 验证已删除
        let result = plugin.stat("/service1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_heartbeatfs_keepalive() {
        let plugin = HeartbeatFsPlugin::new();

        plugin.mkdir("/service1", 0o755).await.unwrap();

        // 更新心跳
        plugin.write("/service1/keepalive", b"ping".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 读取状态
        let data = plugin.read("/service1/ctl", 0, 1000).await.unwrap();
        let status = String::from_utf8(data).unwrap();
        assert!(status.contains("status: alive"));
    }

    #[tokio::test]
    async fn test_heartbeatfs_update_timeout() {
        let plugin = HeartbeatFsPlugin::new();

        plugin.mkdir("/service1", 0o755).await.unwrap();

        // 更新超时
        plugin.write("/service1/ctl", b"timeout=60".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 读取状态
        let data = plugin.read("/service1/ctl", 0, 1000).await.unwrap();
        let status = String::from_utf8(data).unwrap();
        assert!(status.contains("timeout: 60"));
    }

    #[tokio::test]
    async fn test_heartbeatfs_readdir() {
        let plugin = HeartbeatFsPlugin::new();

        plugin.mkdir("/service1", 0o755).await.unwrap();
        plugin.mkdir("/service2", 0o755).await.unwrap();

        // 列出根目录
        let entries = plugin.readdir("/").await.unwrap();
        assert_eq!(entries.len(), 3); // README, service1, service2

        // 列出服务目录
        let files = plugin.readdir("/service1").await.unwrap();
        assert_eq!(files.len(), 2); // keepalive, ctl
        assert!(files.iter().any(|f| f.name == "keepalive"));
        assert!(files.iter().any(|f| f.name == "ctl"));
    }

    #[tokio::test]
    async fn test_heartbeatfs_cleanup() {
        let plugin = HeartbeatFsPlugin::with_config(HeartbeatConfig {
            default_timeout: Duration::from_millis(100),
        });

        plugin.mkdir("/service1", 0o755).await.unwrap();

        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 清理过期项
        plugin.cleanup_expired().await;

        // 验证已删除
        let result = plugin.stat("/service1").await;
        assert!(result.is_err());
    }
}
