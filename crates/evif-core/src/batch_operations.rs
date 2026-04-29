// Batch Operations Module
//
// 提供批量文件操作优化，支持并行处理和进度跟踪
//
// 功能：
// - �批量复制优化
// - 批量删除优化
// - 并行处理
// - 进度回调支持

use crate::error::{EvifError, EvifResult};
use crate::plugin::EvifPlugin;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// 批量操作进度回调
pub type ProgressCallback = Box<dyn Fn(BatchProgress) + Send + Sync>;

/// 批量操作进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    /// 总任务数
    pub total: usize,

    /// 已完成任务数
    pub completed: usize,

    /// 失败任务数
    pub failed: usize,

    /// 当前进度百分比 (0-100)
    pub percent: f64,

    /// 当前处理的文件
    pub current_file: Option<String>,

    /// 预计剩余时间（毫秒）
    pub estimated_remaining_ms: Option<u64>,
}

impl BatchProgress {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: 0,
            failed: 0,
            percent: 0.0,
            current_file: None,
            estimated_remaining_ms: None,
        }
    }

    pub fn update(&mut self, completed: usize, failed: usize, current: Option<String>) {
        self.completed = completed;
        self.failed = failed;
        self.current_file = current;
        self.percent = if self.total > 0 {
            (completed as f64 / self.total as f64) * 100.0
        } else {
            0.0
        };
    }
}

/// 批量操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// 成功的文件列表
    pub success: Vec<String>,

    /// 失败的文件列表及错误信息
    pub errors: Vec<BatchError>,

    /// 总操作时间（毫秒）
    pub total_time_ms: u64,
}

/// 批量操作错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    /// 文件路径
    pub path: String,

    /// 错误信息
    pub error: String,
}

/// 批量复制操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCopyRequest {
    /// 源路径列表
    pub sources: Vec<String>,

    /// 目标目录
    pub destination: String,

    /// 是否递归复制子目录
    pub recursive: bool,

    /// 是否覆盖已存在文件
    pub overwrite: bool,

    /// 并发度（同时处理的任务数）
    pub concurrency: usize,
}

impl Default for BatchCopyRequest {
    fn default() -> Self {
        Self {
            sources: vec![],
            destination: "/".to_string(),
            recursive: true,
            overwrite: false,
            concurrency: 4, // 默认 4 个并发任务
        }
    }
}

/// 批量删除操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteRequest {
    /// 要删除的路径列表
    pub paths: Vec<String>,

    /// 是否递归删除子目录
    pub recursive: bool,

    /// 并发度
    pub concurrency: usize,
}

impl Default for BatchDeleteRequest {
    fn default() -> Self {
        Self {
            paths: vec![],
            recursive: true,
            concurrency: 4,
        }
    }
}

/// 批量操作执行器
pub struct BatchExecutor {
    plugin: Arc<dyn EvifPlugin>,
    concurrency: usize,
    progress_callback: Option<ProgressCallback>,
}

impl BatchExecutor {
    /// 创建新的批量操作执行器
    pub fn new(plugin: Arc<dyn EvifPlugin>) -> Self {
        Self {
            plugin,
            concurrency: 4,
            progress_callback: None,
        }
    }

    /// 设置并发度
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.clamp(1, 64); // 限制在 1-64 之间
        self
    }

    /// 设置进度回调
    pub fn with_progress(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// 触发进度回调
    fn notify_progress(&self, progress: BatchProgress) {
        if let Some(ref callback) = self.progress_callback {
            callback(progress);
        }
    }

    /// 执行批量复制操作
    pub async fn batch_copy(&self, _request: &BatchCopyRequest) -> EvifResult<BatchResult> {
        let start_time = std::time::Instant::now();
        let total = _request.sources.len();

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut progress = BatchProgress::new(total);
        let mut join_set = JoinSet::new();

        let mut success = Vec::new();
        let mut errors = Vec::new();
        let _success_mutex = tokio::sync::Mutex::new(&mut success);
        let _errors_mutex = tokio::sync::Mutex::new(&mut errors);

        let plugin = self.plugin.clone();

        for source in &_request.sources {
            let semaphore = semaphore.clone();
            let plugin = plugin.clone();
            let dest = _request.destination.clone();
            let recursive = _request.recursive;
            let _overwrite = _request.overwrite;
            let source = source.clone();

            join_set.spawn(async move {
                // Handle semaphore closure gracefully - return error tuple
                if semaphore.acquire().await.is_err() {
                    return Err((source.clone(), "Concurrency limit unavailable".to_string()));
                }

                let target_path = if recursive {
                    // 递归复制：保持目录结构
                    let source_name = source.rsplit('/').next().unwrap_or(&source);
                    format!("{}/{}", dest.trim_end_matches('/'), source_name)
                } else {
                    format!(
                        "{}/{}",
                        dest.trim_end_matches('/'),
                        source.rsplit('/').next().unwrap_or(&source)
                    )
                };

                match plugin.clone().rename(&source, &target_path).await {
                    Ok(_) => Ok((source.clone(), target_path)),
                    Err(_) => {
                        // 如果 rename 失败，尝试读取并写入
                        match plugin.clone().read(&source, 0, 0).await {
                            Ok(data) => {
                                let write_flags = crate::plugin::WriteFlags::NONE;
                                match plugin
                                    .clone()
                                    .write(&target_path, data, -1, write_flags)
                                    .await
                                {
                                    Ok(_) => Ok((source.clone(), target_path)),
                                    Err(e) => Err((source.clone(), e.to_string())),
                                }
                            }
                            Err(e) => Err((source.clone(), e.to_string())),
                        }
                    }
                }
            });
        }

        let mut completed = 0;
        let mut failed = 0;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok((source, target))) => {
                    completed += 1;
                    success.push(target);

                    progress.update(completed, failed, Some(source.clone()));
                    self.notify_progress(progress.clone());
                }
                Ok(Err((path, error))) => {
                    failed += 1;
                    errors.push(BatchError {
                        path: path.clone(),
                        error,
                    });

                    progress.update(completed, failed, Some(path.clone()));
                    self.notify_progress(progress.clone());
                }
                Err(e) => {
                    failed += 1;
                    errors.push(BatchError {
                        path: "unknown".to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(BatchResult {
            success,
            errors,
            total_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// 执行批量删除操作
    pub async fn batch_delete(&self, _request: &BatchDeleteRequest) -> EvifResult<BatchResult> {
        let start_time = std::time::Instant::now();
        let total = _request.paths.len();

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut progress = BatchProgress::new(total);
        let mut join_set = JoinSet::new();

        let mut success = Vec::new();
        let mut errors = Vec::new();

        let plugin = self.plugin.clone();

        for path in &_request.paths {
            let semaphore = semaphore.clone();
            let plugin = plugin.clone();
            let path = path.clone();
            let recursive = _request.recursive;

            join_set.spawn(async move {
                // Handle semaphore closure gracefully - return error tuple
                if semaphore.acquire().await.is_err() {
                    return Err((path.clone(), "Concurrency limit unavailable".to_string()));
                }

                if recursive {
                    plugin.clone().remove_all(&path).await
                } else {
                    plugin.clone().remove(&path).await
                }
                .map(|_| path.clone())
                .map_err(|e| (path.clone(), e.to_string()))
            });
        }

        let mut completed = 0;
        let mut failed = 0;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(path)) => {
                    completed += 1;
                    success.push(path.clone());

                    progress.update(completed, failed, Some(path.clone()));
                    self.notify_progress(progress.clone());
                }
                Ok(Err((path, error))) => {
                    failed += 1;
                    errors.push(BatchError {
                        path: path.clone(),
                        error,
                    });

                    progress.update(completed, failed, Some(path.clone()));
                    self.notify_progress(progress.clone());
                }
                Err(e) => {
                    failed += 1;
                    errors.push(BatchError {
                        path: "unknown".to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(BatchResult {
            success,
            errors,
            total_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

/// 批量操作 trait - 允许插件提供优化的批量操作
#[async_trait]
pub trait BatchOperations: Send + Sync {
    /// 批量复制（插件可以实现自己的优化版本）
    async fn batch_copy_optimized(
        &self,
        _request: BatchCopyRequest,
        _callback: Option<ProgressCallback>,
    ) -> EvifResult<BatchResult> {
        // 默认实现：使用通用批量执行器
        Err(EvifError::NotSupportedGeneric)
    }

    /// 批量删除（插件可以实现自己的优化版本）
    async fn batch_delete_optimized(
        &self,
        _request: BatchDeleteRequest,
        _callback: Option<ProgressCallback>,
    ) -> EvifResult<BatchResult> {
        // 默认实现：使用通用批量执行器
        Err(EvifError::NotSupportedGeneric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::FileInfo;
    use chrono::Utc;

    // Mock plugin for testing
    #[allow(dead_code)]
    struct MockPlugin;

    #[async_trait]
    impl EvifPlugin for MockPlugin {
        fn name(&self) -> &str {
            "mock"
        }

        async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
            Ok(())
        }

        async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
            Ok(())
        }

        async fn read(&self, _path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
            Ok(vec![1, 2, 3])
        }

        async fn write(
            &self,
            _path: &str,
            _data: Vec<u8>,
            _offset: i64,
            _flags: crate::plugin::WriteFlags,
        ) -> EvifResult<u64> {
            Ok(3)
        }

        async fn readdir(&self, _path: &str) -> EvifResult<Vec<FileInfo>> {
            Ok(vec![])
        }

        async fn stat(&self, _path: &str) -> EvifResult<FileInfo> {
            Ok(FileInfo {
                name: "test".to_string(),
                size: 100,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            })
        }

        async fn remove(&self, _path: &str) -> EvifResult<()> {
            Ok(())
        }

        async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
            Ok(())
        }

        async fn remove_all(&self, _path: &str) -> EvifResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_batch_progress() {
        let mut progress = BatchProgress::new(100);
        assert_eq!(progress.total, 100);
        assert_eq!(progress.percent, 0.0);

        progress.update(50, 0, Some("test.txt".to_string()));
        assert_eq!(progress.completed, 50);
        assert_eq!(progress.percent, 50.0);
        assert_eq!(progress.current_file, Some("test.txt".to_string()));
    }

    #[tokio::test]
    async fn test_batch_copy_request_default() {
        let req = BatchCopyRequest::default();
        assert!(req.sources.is_empty());
        assert_eq!(req.destination, "/");
        assert!(req.recursive);
        assert!(!req.overwrite);
        assert_eq!(req.concurrency, 4);
    }

    #[tokio::test]
    async fn test_batch_delete_request_default() {
        let req = BatchDeleteRequest::default();
        assert!(req.paths.is_empty());
        assert!(req.recursive);
        assert_eq!(req.concurrency, 4);
    }
}
