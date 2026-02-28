// Global Handle Manager - 全局文件句柄管理
//
// 对标 AGFS MountableFS 全局句柄ID管理
// 跨所有插件实例生成唯一句柄ID

use crate::error::{EvifError, EvifResult};
use crate::plugin::FileHandle;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use tracing;
/// 句柄信息
struct HandleInfo {
    /// 句柄ID（全局唯一）
    id: i64,
    /// 挂载点路径
    mount_path: String,
    /// 完整文件路径
    full_path: String,
    /// 本地句柄（由插件管理）
    local_handle: Option<Box<dyn FileHandle>>,
    /// 创建时间
    created_at: std::time::Instant,
    /// 过期时间
    expires_at: std::time::Instant,
}

impl HandleInfo {
    /// 检查句柄是否已过期
    fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at
    }
}

/// 全局句柄管理器
///
/// 对标 AGFS MountableFS 的句柄管理系统
/// 提供跨插件的唯一句柄ID分配和管理
pub struct GlobalHandleManager {
    /// 全局句柄ID计数器（原子操作）
    next_id: Arc<AtomicI64>,

    /// 句柄信息映射表: handle_id -> HandleInfo
    handles: Arc<RwLock<HashMap<i64, HandleInfo>>>,

    /// 默认句柄租约时长
    default_lease: std::time::Duration,

    /// 最大句柄数量
    max_handles: usize,
}

impl GlobalHandleManager {
    /// 创建新的全局句柄管理器
    pub fn new() -> Self {
        Self {
            next_id: Arc::new(AtomicI64::new(1)),
            handles: Arc::new(RwLock::new(HashMap::new())),
            default_lease: std::time::Duration::from_secs(3600), // 1 hour
            max_handles: 10000,
        }
    }

    /// 设置默认租约时长
    pub fn with_lease(mut self, lease: std::time::Duration) -> Self {
        self.default_lease = lease;
        self
    }

    /// 设置最大句柄数量
    pub fn with_max_handles(mut self, max: usize) -> Self {
        self.max_handles = max;
        self
    }

    /// 分配新的全局句柄ID
    ///
    /// # AGFS 对标
    /// ```go
    /// globalID := mfs.globalHandleID.Add(1)
    /// ```
    pub fn allocate_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// 注册句柄
    ///
    /// # 参数
    /// - `id`: 全局句柄ID
    /// - `mount_path`: 挂载点路径
    /// - `full_path`: 完整文件路径
    /// - `local_handle`: 本地句柄（可选）
    ///
    /// # AGFS 对标
    /// ```go
    /// mfs.handleInfos[globalID] = &handleInfo{
    ///     mount: mount,
    ///     localHandle: localHandle,
    /// }
    /// ```
    pub async fn register_handle(
        &self,
        id: i64,
        mount_path: String,
        full_path: String,
        local_handle: Option<Box<dyn FileHandle>>,
    ) -> EvifResult<()> {
        // 检查句柄数量限制
        let handles = self.handles.read().await;
        if handles.len() >= self.max_handles {
            return Err(EvifError::Internal(
                "Maximum handle limit reached".to_string()
            ));
        }
        drop(handles);

        let now = std::time::Instant::now();
        let info = HandleInfo {
            id,
            mount_path,
            full_path,
            local_handle,
            created_at: now,
            expires_at: now + self.default_lease,
        };

        let mut handles = self.handles.write().await;
        handles.insert(id, info);

        Ok(())
    }

    /// 获取句柄信息
    ///
    /// # AGFS 对标
    /// ```go
    /// info, ok := mfs.handleInfos[handleID]
    /// ```
    pub async fn get_handle(&self, id: i64) -> EvifResult<(i64, String, String, std::time::Instant)> {
        let handles = self.handles.read().await;

        handles.get(&id)
            .map(|info| (info.id, info.mount_path.clone(), info.full_path.clone(), info.expires_at))
            .ok_or_else(|| EvifError::NotFound(format!("Handle: {}", id)))
    }
    /// 更新本地句柄
    pub async fn update_local_handle(&self, id: i64, local_handle: Box<dyn FileHandle>) -> EvifResult<()> {
        let mut handles = self.handles.write().await;

        if let Some(info) = handles.get_mut(&id) {
            info.local_handle = Some(local_handle);
            Ok(())
        } else {
            Err(EvifError::NotFound(format!("Handle: {}", id)))
        }
    }

    /// 关闭句柄
    ///
    /// # AGFS 对标
    /// ```go
    /// delete(mfs.handleInfos, handleID)
    /// ```
    pub async fn close_handle(&self, id: i64) -> EvifResult<()> {
        let mut handles = self.handles.write().await;

        handles.remove(&id)
            .ok_or_else(|| EvifError::NotFound(format!("Handle: {}", id)))?;

        Ok(())
    }

    /// 续租句柄
    ///
    /// # 参数
    /// - `id`: 句柄ID
    /// - `lease`: 新的租约时长（None表示使用默认时长）
    ///
    /// # AGFS 对标
    /// ```go
    /// func (h *globalFileHandle) Renew(lease time.Duration) error
    /// ```
    pub async fn renew_handle(&self, id: i64, lease: Option<std::time::Duration>) -> EvifResult<()> {
        let lease = lease.unwrap_or(self.default_lease);

        let mut handles = self.handles.write().await;

        if let Some(info) = handles.get_mut(&id) {
            info.expires_at = std::time::Instant::now() + lease;
            Ok(())
        } else {
            Err(EvifError::NotFound(format!("Handle: {}", id)))
        }
    }

    /// 清理过期句柄
    ///
    /// # AGFS 对标
    /// ```go
    /// // AGFS使用goroutine定期清理
    /// go mfs.cleanupExpiredHandles()
    /// ```
    pub async fn cleanup_expired_handles(&self) -> usize {
        let mut handles = self.handles.write().await;

        let initial_count = handles.len();
        handles.retain(|_, info| !info.is_expired());

        initial_count - handles.len()
    }

    /// 列出所有活动句柄
    pub async fn list_handles(&self) -> Vec<i64> {
        let handles = self.handles.read().await;
        handles.keys().copied().collect()
    }

    /// 获取句柄数量
    pub async fn handle_count(&self) -> usize {
        let handles = self.handles.read().await;
        handles.len()
    }

    /// 启动后台清理任务
    ///
    /// # 参数
    /// - `interval`: 清理间隔
    ///
    /// # AGFS 对标
    /// ```go
    /// go mfs.cleanupExpiredHandles()
    /// ```
    pub fn spawn_cleanup_task(self: Arc<Self>, interval: std::time::Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;
                let cleaned = self.cleanup_expired_handles().await;

                if cleaned > 0 {
                    tracing::debug!("Cleaned up {} expired handles", cleaned);
                }
            }
        })
    }
}

impl Default for GlobalHandleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allocate_id() {
        let manager = GlobalHandleManager::new();

        let id1 = manager.allocate_id();
        let id2 = manager.allocate_id();

        assert_eq!(id2, id1 + 1);
    }

    #[tokio::test]
    async fn test_register_and_get_handle() {
        let manager = GlobalHandleManager::new();
        let id = manager.allocate_id();

        manager.register_handle(
            id,
            "/test".to_string(),
            "/test/file.txt".to_string(),
            None,
        ).await.unwrap();

        let (handle_id, mount_path, full_path, _expires_at) = manager.get_handle(id).await.unwrap();
        assert_eq!(handle_id, id);
        assert_eq!(mount_path, "/test");
        assert_eq!(full_path, "/test/file.txt");
    }

    #[tokio::test]
    async fn test_close_handle() {
        let manager = GlobalHandleManager::new();
        let id = manager.allocate_id();

        manager.register_handle(
            id,
            "/test".to_string(),
            "/test/file.txt".to_string(),
            None,
        ).await.unwrap();

        manager.close_handle(id).await.unwrap();

        assert!(manager.get_handle(id).await.is_err());
    }

    #[tokio::test]
    async fn test_renew_handle() {
        let manager = GlobalHandleManager::new();
        let id = manager.allocate_id();

        manager.register_handle(
            id,
            "/test".to_string(),
            "/test/file.txt".to_string(),
            None,
        ).await.unwrap();

        let new_lease = std::time::Duration::from_secs(7200);
        manager.renew_handle(id, Some(new_lease)).await.unwrap();

        let (_handle_id, _mount_path, _full_path, expires_at) = manager.get_handle(id).await.unwrap();
        assert!(expires_at > std::time::Instant::now());
    }
}
