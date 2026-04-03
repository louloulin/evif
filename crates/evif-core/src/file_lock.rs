// File Lock Manager - Phase 14.2
//
// 文件锁管理器：防止并发写入同一文件的冲突
// 使用 RwLock 实现乐观锁机制

use crate::error::{EvifError, EvifResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 锁信息
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// 文件路径
    pub path: String,
    /// 操作类型: "read" | "write"
    pub operation: String,
    /// 锁持有者的会话ID
    pub session_id: Option<String>,
    /// 获取锁的时间戳
    pub acquired_at: chrono::DateTime<chrono::Utc>,
}

/// 文件锁管理器 (Phase 14.2)
pub struct FileLockManager {
    /// 锁映射表: path -> LockInfo
    locks: Arc<RwLock<HashMap<String, LockInfo>>>,
}

impl FileLockManager {
    /// 创建新的锁管理器
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 尝试获取文件锁
    pub async fn try_lock(
        &self,
        path: &str,
        operation: &str,
        session_id: Option<String>,
    ) -> EvifResult<()> {
        let mut locks = self.locks.write().await;

        if let Some(existing) = locks.get(path) {
            // 如果已有写锁，返回冲突
            if existing.operation == "write" {
                return Err(EvifError::Conflict(format!(
                    "File is locked: {} by session {:?}",
                    path, existing.session_id
                )));
            }
            // 读锁可以共存
            if operation == "read" {
                return Ok(());
            }
        }

        // 获取锁
        let info = LockInfo {
            path: path.to_string(),
            operation: operation.to_string(),
            session_id,
            acquired_at: chrono::Utc::now(),
        };
        locks.insert(path.to_string(), info);
        Ok(())
    }

    /// 释放文件锁
    pub async fn unlock(&self, path: &str) -> EvifResult<()> {
        let mut locks = self.locks.write().await;
        locks.remove(path);
        Ok(())
    }

    /// 获取所有锁
    pub async fn list_locks(&self) -> Vec<LockInfo> {
        let locks = self.locks.read().await;
        locks.values().cloned().collect()
    }

    /// 检查文件是否被锁定
    pub async fn is_locked(&self, path: &str) -> bool {
        let locks = self.locks.read().await;
        locks.contains_key(path)
    }

    /// 获取文件锁信息
    pub async fn get_lock(&self, path: &str) -> Option<LockInfo> {
        let locks = self.locks.read().await;
        locks.get(path).cloned()
    }
}

impl Default for FileLockManager {
    fn default() -> Self {
        Self::new()
    }
}
