// Phase 14.1: Cross-Filesystem Copy
//
// 跨文件系统复制：支持不同挂载点间的文件复制

use crate::error::{EvifError, EvifResult};
use crate::EvifPlugin;
use std::sync::Arc;

/// 挂载表 trait，用于跨文件系统操作
pub trait MountTableLookup: Send + Sync {
    /// 查找插件并返回相对路径
    #[allow(async_fn_in_trait)]
    async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String);
}

/// 跨文件系统复制管理器 (Phase 14.1)
pub struct CrossFsCopyManager<T: MountTableLookup> {
    /// 挂载表，用于查找源和目标的插件
    mount_table: Arc<T>,
}

impl<T: MountTableLookup> CrossFsCopyManager<T> {
    /// 创建新的跨文件系统复制管理器
    pub fn new(mount_table: Arc<T>) -> Self {
        Self { mount_table }
    }

    /// 从源路径复制到目标路径
    ///
    /// 支持跨文件系统复制，自动识别源和目标所在的插件
    pub async fn copy(&self, src: &str, dst: &str, overwrite: bool) -> EvifResult<u64> {
        // 查找源路径对应的插件
        let (src_plugin_opt, src_rel_path) = self.mount_table.lookup_with_path(src).await;

        let src_plugin = src_plugin_opt
            .ok_or_else(|| EvifError::NotFound(format!("Source not found: {}", src)))?;

        // 查找目标路径对应的插件
        let (dst_plugin_opt, dst_rel_path) = self.mount_table.lookup_with_path(dst).await;

        let dst_plugin = dst_plugin_opt
            .ok_or_else(|| EvifError::NotFound(format!("Destination not found: {}", dst)))?;

        // 检查目标文件是否已存在
        if !overwrite && dst_plugin.stat(&dst_rel_path).await.is_ok() {
            return Err(EvifError::AlreadyExists(format!(
                "Destination already exists: {}",
                dst
            )));
        }

        // 读取源文件内容
        let data = src_plugin.read(&src_rel_path, 0, 0).await?;
        let bytes_copied = data.len() as u64;

        // 如果源和目标在同一个插件，使用 rename（移动）
        if Arc::ptr_eq(&src_plugin, &dst_plugin) {
            // 确保目标目录存在
            let dst_dir = dst_rel_path.rsplit('/').nth(1).unwrap_or("");
            if !dst_dir.is_empty() {
                let _ = dst_plugin.mkdir(dst_dir, 0o755).await;
            }
            dst_plugin.rename(&src_rel_path, &dst_rel_path).await?;
            return Ok(bytes_copied);
        }

        // 跨文件系统：读取源内容，写入目标
        // 确保目标目录存在
        let dst_dir = dst_rel_path.rsplit('/').nth(1).unwrap_or("");
        if !dst_dir.is_empty() {
            let _ = dst_plugin.mkdir(dst_dir, 0o755).await;
        }

        // 创建目标文件
        dst_plugin.create(&dst_rel_path, 0o644).await?;
        // 写入内容
        dst_plugin
            .write(&dst_rel_path, data, 0, crate::WriteFlags::TRUNCATE)
            .await?;

        Ok(bytes_copied)
    }

    /// 递归复制目录
    pub async fn copy_recursive(&self, src: &str, dst: &str) -> EvifResult<u64> {
        let (src_plugin_opt, src_rel_path) = self.mount_table.lookup_with_path(src).await;

        let src_plugin = src_plugin_opt
            .ok_or_else(|| EvifError::NotFound(format!("Source not found: {}", src)))?;

        let (dst_plugin_opt, dst_rel_path) = self.mount_table.lookup_with_path(dst).await;

        let dst_plugin = dst_plugin_opt
            .ok_or_else(|| EvifError::NotFound(format!("Destination not found: {}", dst)))?;

        let mut total_bytes = 0u64;

        // 递归复制目录内容
        self.copy_dir_recursive(
            src_plugin,
            &src_rel_path,
            dst_plugin,
            &dst_rel_path,
            &mut total_bytes,
        )
        .await?;

        Ok(total_bytes)
    }

    /// 递归复制目录的帮助函数 (使用 Box::pin 避免递归编译错误)
    async fn copy_dir_recursive(
        &self,
        src_plugin: Arc<dyn EvifPlugin>,
        src_path: &str,
        dst_plugin: Arc<dyn EvifPlugin>,
        dst_path: &str,
        total_bytes: &mut u64,
    ) -> EvifResult<()> {
        let entries = src_plugin.readdir(src_path).await?;

        for entry in entries {
            let src_child = format!("{}/{}", src_path.trim_end_matches('/'), entry.name);
            let dst_child = format!("{}/{}", dst_path.trim_end_matches('/'), entry.name);

            if entry.is_dir {
                // 创建目标目录
                dst_plugin.mkdir(&dst_child, 0o755).await?;
                // 递归复制子目录
                Box::pin(self.copy_dir_recursive(
                    src_plugin.clone(),
                    &src_child,
                    dst_plugin.clone(),
                    &dst_child,
                    total_bytes,
                ))
                .await?;
            } else {
                // 复制文件
                let data = src_plugin.read(&src_child, 0, 0).await?;
                *total_bytes += data.len() as u64;
                dst_plugin.create(&dst_child, 0o644).await?;
                dst_plugin
                    .write(&dst_child, data, 0, crate::WriteFlags::TRUNCATE)
                    .await?;
            }
        }

        Ok(())
    }
}

// 为 MountTable 实现 MountTableLookup
impl MountTableLookup for crate::MountTable {
    async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String) {
        crate::MountTable::lookup_with_path(self, path).await
    }
}

// 为 RadixMountTable 实现 MountTableLookup
impl MountTableLookup for crate::RadixMountTable {
    async fn lookup_with_path(&self, path: &str) -> (Option<Arc<dyn EvifPlugin>>, String) {
        crate::RadixMountTable::lookup_with_path(self, path).await
    }
}
