// HandleFS - 文件句柄管理插件
//
// 支持有状态的文件操作,用于大文件分块传输和会话管理

use evif_core::{EvifError, EvifResult, EvifPlugin, FileInfo, WriteFlags};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// 文件句柄标志
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ_ONLY = 1 << 0;
        const WRITE_ONLY = 1 << 1;
        const READ_WRITE = 1 << 2;
        const CREATE = 1 << 3;
        const EXCLUSIVE = 1 << 4;
        const TRUNCATE = 1 << 5;
        const APPEND = 1 << 6;
        const NONBLOCK = 1 << 7;
    }
}

impl Default for OpenFlags {
    fn default() -> Self {
        OpenFlags::READ_ONLY
    }
}

/// 文件句柄
#[derive(Clone)]
pub struct FileHandle {
    pub id: i64,
    pub path: String,
    pub flags: OpenFlags,
    pub mode: u32,
    pub offset: i64,
    pub created_at: Instant,
    pub expires_at: Instant,
    pub client_info: Option<String>,
}

impl FileHandle {
    /// 检查句柄是否已过期
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// 检查句柄是否可读
    pub fn can_read(&self) -> bool {
        self.flags.contains(OpenFlags::READ_ONLY) || self.flags.contains(OpenFlags::READ_WRITE)
    }

    /// 检查句柄是否可写
    pub fn can_write(&self) -> bool {
        self.flags.contains(OpenFlags::WRITE_ONLY) || self.flags.contains(OpenFlags::READ_WRITE)
    }
}

/// HandleFS 配置
#[derive(Debug, Clone)]
pub struct HandleFsConfig {
    pub default_lease: Duration,
    pub max_handles: usize,
    pub cleanup_interval: Duration,
}

impl Default for HandleFsConfig {
    fn default() -> Self {
        Self {
            default_lease: Duration::from_secs(3600), // 1 hour
            max_handles: 10000,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// HandleFS 插件
pub struct HandleFsPlugin {
    base_fs: Arc<dyn EvifPlugin>,
    handles: Arc<RwLock<HashMap<i64, FileHandle>>>,
    next_handle_id: Arc<AtomicI64>,
    config: HandleFsConfig,
}

impl HandleFsPlugin {
    pub fn new(base_fs: Arc<dyn EvifPlugin>, config: HandleFsConfig) -> Self {
        Self {
            base_fs,
            handles: Arc::new(RwLock::new(HashMap::new())),
            next_handle_id: Arc::new(AtomicI64::new(1)),
            config,
        }
    }

    /// 打开文件句柄
    pub async fn open_handle(
        &self,
        path: &str,
        flags: OpenFlags,
        mode: u32,
        lease: Duration,
    ) -> EvifResult<FileHandle> {
        // 检查句柄数量限制
        {
            let handles = self.handles.read().await;
            if handles.len() >= self.config.max_handles {
                return Err(EvifError::Other("Too many open handles".to_string()));
            }
        }

        // 如果需要创建文件
        if flags.contains(OpenFlags::CREATE) {
            // 检查文件是否存在
            let exists = self.base_fs.stat(path).await.is_ok();

            if !exists || flags.contains(OpenFlags::TRUNCATE) {
                self.base_fs.create(path, mode).await?;
            } else if flags.contains(OpenFlags::EXCLUSIVE) {
                return Err(EvifError::AlreadyExists(path.to_string()));
            }
        }

        // 生成句柄ID
        let handle_id = self.next_handle_id.fetch_add(1, Ordering::SeqCst);
        let now = Instant::now();
        let expires_at = now + lease;

        let handle = FileHandle {
            id: handle_id,
            path: path.to_string(),
            flags,
            mode,
            offset: 0,
            created_at: now,
            expires_at,
            client_info: None,
        };

        // 存储句柄
        let mut handles = self.handles.write().await;
        handles.insert(handle_id, handle.clone());

        Ok(handle)
    }

    /// 从句柄读取
    pub async fn read_handle(
        &self,
        handle_id: i64,
        offset: i64,
        size: u64,
    ) -> EvifResult<Vec<u8>> {
        let handle = {
            let handles = self.handles.read().await;
            handles.get(&handle_id).cloned()
        }
        .ok_or(EvifError::HandleNotFound(handle_id))?;

        // 检查是否过期
        if handle.is_expired() {
            return Err(EvifError::LeaseExpired(handle_id));
        }

        // 检查读取权限
        if !handle.can_read() {
            return Err(EvifError::PermissionDenied(
                "Handle not opened for reading".to_string(),
            ));
        }

        // 从底层文件系统读取
        let data = self.base_fs.read(&handle.path, offset as u64, size).await?;

        Ok(data)
    }

    /// 向句柄写入
    pub async fn write_handle(
        &self,
        handle_id: i64,
        data: Vec<u8>,
        offset: i64,
    ) -> EvifResult<u64> {
        let handle = {
            let handles = self.handles.read().await;
            handles.get(&handle_id).cloned()
        }
        .ok_or(EvifError::HandleNotFound(handle_id))?;

        // 检查是否过期
        if handle.is_expired() {
            return Err(EvifError::LeaseExpired(handle_id));
        }

        // 检查写入权限
        if !handle.can_write() {
            return Err(EvifError::PermissionDenied(
                "Handle not opened for writing".to_string(),
            ));
        }

        // 计算写入偏移量
        let write_offset = if handle.flags.contains(OpenFlags::APPEND) {
            -1 // 追加模式
        } else {
            offset
        };

        // 写入到底层文件系统
        let flags = WriteFlags::empty();
        let bytes_written = self
            .base_fs
            .write(&handle.path, data, write_offset, flags)
            .await?;

        Ok(bytes_written)
    }

    /// 关闭句柄
    pub async fn close_handle(&self, handle_id: i64) -> EvifResult<()> {
        let mut handles = self.handles.write().await;
        handles
            .remove(&handle_id)
            .ok_or(EvifError::HandleNotFound(handle_id))?;

        Ok(())
    }

    /// 刷新句柄
    pub async fn flush_handle(&self, handle_id: i64) -> EvifResult<()> {
        let handles = self.handles.read().await;
        handles
            .get(&handle_id)
            .ok_or(EvifError::HandleNotFound(handle_id))?;

        // 对于大多数文件系统,flush是no-op
        // 但可以在这里实现sync操作

        Ok(())
    }

    /// 获取句柄信息
    pub async fn get_handle(&self, handle_id: i64) -> EvifResult<FileHandle> {
        let handles = self.handles.read().await;
        handles
            .get(&handle_id)
            .cloned()
            .ok_or(EvifError::HandleNotFound(handle_id))
    }

    /// 列出所有句柄
    pub async fn list_handles(&self) -> Vec<FileHandle> {
        let handles = self.handles.read().await;
        handles.values().cloned().collect()
    }

    /// 清理过期句柄
    pub async fn cleanup_expired_handles(&self) -> usize {
        let mut handles = self.handles.write().await;
        let before = handles.len();

        handles.retain(|_, handle| !handle.is_expired());

        before - handles.len()
    }

    /// 延长句柄租约
    pub async fn renew_lease(&self, handle_id: i64, additional: Duration) -> EvifResult<()> {
        let mut handles = self.handles.write().await;
        let handle = handles
            .get_mut(&handle_id)
            .ok_or(EvifError::HandleNotFound(handle_id))?;

        if handle.is_expired() {
            return Err(EvifError::LeaseExpired(handle_id));
        }

        handle.expires_at = Instant::now() + additional;
        Ok(())
    }
}

#[async_trait]
impl EvifPlugin for HandleFsPlugin {
    fn name(&self) -> &str {
        "handlefs"
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.base_fs.create(path, perm).await
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.base_fs.mkdir(path, perm).await
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        self.base_fs.read(path, offset, size).await
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        offset: i64,
        flags: WriteFlags,
    ) -> EvifResult<u64> {
        self.base_fs.write(path, data, offset, flags).await
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        self.base_fs.readdir(path).await
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.base_fs.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.base_fs.remove(path).await
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        self.base_fs.rename(old_path, new_path).await
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.base_fs.remove_all(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalFsPlugin;

    #[tokio::test]
    async fn test_open_handle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let handle_fs = HandleFsPlugin::new(local_fs, HandleFsConfig::default());

        let handle = handle_fs
            .open_handle(
                "/test.txt",
                OpenFlags::READ_WRITE | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        assert!(handle.id > 0);
        assert_eq!(handle.path, "/test.txt");
        assert!(handle.can_read());
        assert!(handle.can_write());
    }

    #[tokio::test]
    async fn test_read_write_handle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let handle_fs = HandleFsPlugin::new(local_fs, HandleFsConfig::default());

        // 创建并写入文件
        let handle = handle_fs
            .open_handle(
                "/test.txt",
                OpenFlags::READ_WRITE | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        let data = b"Hello, World!".to_vec();
        let written = handle_fs
            .write_handle(handle.id, data.clone(), 0)
            .await
            .unwrap();
        assert_eq!(written, 13);

        // 读取文件
        let read_data = handle_fs.read_handle(handle.id, 0, 13).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_handle_expiration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let config = HandleFsConfig {
            default_lease: Duration::from_millis(100),
            ..Default::default()
        };
        let handle_fs = HandleFsPlugin::new(local_fs, config);

        let handle = handle_fs
            .open_handle(
                "/test.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_millis(100),
            )
            .await
            .unwrap();

        // 等待句柄过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 尝试读取应该失败
        let result = handle_fs.read_handle(handle.id, 0, 100).await;
        assert!(matches!(result, Err(EvifError::LeaseExpired(_))));
    }

    #[tokio::test]
    async fn test_close_handle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let handle_fs = HandleFsPlugin::new(local_fs, HandleFsConfig::default());

        let handle = handle_fs
            .open_handle(
                "/test.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        // 关闭句柄
        handle_fs.close_handle(handle.id).await.unwrap();

        // 再次关闭应该失败
        let result = handle_fs.close_handle(handle.id).await;
        assert!(matches!(result, Err(EvifError::HandleNotFound(_))));
    }

    #[tokio::test]
    async fn test_list_handles() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let handle_fs = HandleFsPlugin::new(local_fs, HandleFsConfig::default());

        // 打开多个句柄
        let _ = handle_fs
            .open_handle(
                "/test1.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        let _ = handle_fs
            .open_handle(
                "/test2.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        let handles = handle_fs.list_handles().await;
        assert_eq!(handles.len(), 2);
    }

    #[tokio::test]
    async fn test_cleanup_expired_handles() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let config = HandleFsConfig {
            default_lease: Duration::from_millis(100),
            ..Default::default()
        };
        let handle_fs = HandleFsPlugin::new(local_fs, config);

        // 创建一个快速过期的句柄
        let _ = handle_fs
            .open_handle(
                "/test1.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_millis(100),
            )
            .await
            .unwrap();

        // 创建一个长时间存活的句柄
        let _ = handle_fs
            .open_handle(
                "/test2.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        // 等待第一个句柄过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 清理过期句柄
        let cleaned = handle_fs.cleanup_expired_handles().await;
        assert_eq!(cleaned, 1);

        // 验证只剩一个句柄
        let handles = handle_fs.list_handles().await;
        assert_eq!(handles.len(), 1);
    }

    #[tokio::test]
    async fn test_renew_lease() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_fs = Arc::new(LocalFsPlugin::new(temp_dir.path()));
        let handle_fs = HandleFsPlugin::new(local_fs, HandleFsConfig::default());

        let handle = handle_fs
            .open_handle(
                "/test.txt",
                OpenFlags::READ_ONLY | OpenFlags::CREATE,
                0o644,
                Duration::from_secs(1),
            )
            .await
            .unwrap();

        // 延长租约
        handle_fs
            .renew_lease(handle.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 获取句柄信息验证
        let handle_info = handle_fs.get_handle(handle.id).await.unwrap();
        assert!(handle_info.expires_at > (handle.created_at + Duration::from_secs(5)));
    }
}
