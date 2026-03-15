// Memory File Handle - 内存文件句柄实现
//
// 用于提供基础的文件句柄功能
// 对标 AGFS 的内存文件句柄实现

use crate::error::{EvifError, EvifResult};
use crate::plugin::{FileHandle, FileInfo, OpenFlags};
use async_trait::async_trait;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

/// 内存文件句柄
///
/// 在内存中维护文件状态，支持有状态的读写操作
pub struct MemoryFileHandle {
    /// 句柄ID
    id: i64,
    /// 文件路径
    path: String,
    /// 打开标志
    flags: OpenFlags,
    /// 文件内容（使用Cursor支持seek）
    data: Arc<Mutex<Cursor<Vec<u8>>>>,
    /// 创建时间
    created_at: std::time::Instant,
}

impl MemoryFileHandle {
    /// 创建新的内存文件句柄
    pub fn new(id: i64, path: String, flags: OpenFlags, initial_data: Vec<u8>) -> Self {
        let cursor = if flags.contains(OpenFlags::TRUNCATE) {
            Cursor::new(Vec::new())
        } else {
            Cursor::new(initial_data)
        };

        Self {
            id,
            path,
            flags,
            data: Arc::new(Mutex::new(cursor)),
            created_at: std::time::Instant::now(),
        }
    }

    /// 获取当前文件大小
    pub fn len(&self) -> EvifResult<usize> {
        let data = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        Ok(data.get_ref().len())
    }

    /// 检查文件是否为空
    pub fn is_empty(&self) -> EvifResult<bool> {
        Ok(self.len()? == 0)
    }
}

#[async_trait]
impl FileHandle for MemoryFileHandle {
    fn id(&self) -> i64 {
        self.id
    }

    fn path(&self) -> &str {
        &self.path
    }

    async fn read(&mut self, buf: &mut [u8]) -> EvifResult<usize> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        let n = data
            .read(buf)
            .map_err(|e| EvifError::Internal(format!("Read error: {}", e)))?;

        Ok(n)
    }

    async fn read_at(&self, buf: &mut [u8], offset: u64) -> EvifResult<usize> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        data.seek(SeekFrom::Start(offset))
            .map_err(|e| EvifError::Internal(format!("Seek error: {}", e)))?;

        let n = data
            .read(buf)
            .map_err(|e| EvifError::Internal(format!("Read error: {}", e)))?;

        Ok(n)
    }

    async fn write(&mut self, data: &[u8]) -> EvifResult<usize> {
        let mut cursor = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        // 如果是追加模式，先移动到末尾
        if self.flags.contains(OpenFlags::APPEND) {
            cursor
                .seek(SeekFrom::End(0))
                .map_err(|e| EvifError::Internal(format!("Seek error: {}", e)))?;
        }

        let n = cursor
            .write(data)
            .map_err(|e| EvifError::Internal(format!("Write error: {}", e)))?;

        Ok(n)
    }

    async fn write_at(&self, data: &[u8], offset: u64) -> EvifResult<usize> {
        let mut cursor = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        cursor
            .seek(SeekFrom::Start(offset))
            .map_err(|e| EvifError::Internal(format!("Seek error: {}", e)))?;

        let n = cursor
            .write(data)
            .map_err(|e| EvifError::Internal(format!("Write error: {}", e)))?;

        Ok(n)
    }

    async fn seek(&mut self, offset: i64, whence: u8) -> EvifResult<i64> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        let pos = match whence {
            0 => SeekFrom::Start(offset as u64), // SEEK_SET
            1 => SeekFrom::Current(offset),      // SEEK_CUR
            2 => SeekFrom::End(offset),          // SEEK_END
            _ => return Err(EvifError::Internal(format!("Invalid whence: {}", whence))),
        };

        let new_pos = data
            .seek(pos)
            .map_err(|e| EvifError::Internal(format!("Seek error: {}", e)))?;

        Ok(new_pos as i64)
    }

    async fn sync(&self) -> EvifResult<()> {
        // 内存文件无需同步
        Ok(())
    }

    async fn close(&mut self) -> EvifResult<()> {
        // 释放资源
        Ok(())
    }

    async fn stat(&self) -> EvifResult<FileInfo> {
        let data = self
            .data
            .lock()
            .map_err(|e| EvifError::Internal(format!("Lock error: {}", e)))?;

        Ok(FileInfo {
            name: self.path.split('/').last().unwrap_or("").to_string(),
            size: data.get_ref().len() as u64,
            mode: 0o644,
            modified: chrono::Utc::now(),
            is_dir: false,
        })
    }

    fn flags(&self) -> OpenFlags {
        self.flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_handle_read_write() {
        let mut handle = MemoryFileHandle::new(
            1,
            "/test/file.txt".to_string(),
            OpenFlags::READ_WRITE | OpenFlags::CREATE,
            Vec::new(),
        );

        // 写入数据
        let data = b"Hello, World!";
        let n = handle.write(data).await.unwrap();
        assert_eq!(n, data.len());

        // 移动到开始
        handle.seek(0, 0).await.unwrap();

        // 读取数据
        let mut buf = vec![0u8; data.len()];
        let n = handle.read(&mut buf).await.unwrap();
        assert_eq!(n, data.len());
        assert_eq!(&buf, data);
    }

    #[tokio::test]
    async fn test_memory_handle_seek() {
        let mut handle = MemoryFileHandle::new(
            1,
            "/test/file.txt".to_string(),
            OpenFlags::READ_WRITE | OpenFlags::CREATE,
            Vec::new(),
        );

        // 写入数据
        handle.write(b"Hello, World!").await.unwrap();

        // SEEK_SET
        let pos = handle.seek(0, 0).await.unwrap();
        assert_eq!(pos, 0);

        // SEEK_CUR
        let pos = handle.seek(5, 1).await.unwrap();
        assert_eq!(pos, 5);

        // SEEK_END
        let pos = handle.seek(0, 2).await.unwrap();
        assert_eq!(pos, 13);
    }

    #[tokio::test]
    async fn test_memory_handle_write_at() {
        let handle = MemoryFileHandle::new(
            1,
            "/test/file.txt".to_string(),
            OpenFlags::READ_WRITE | OpenFlags::CREATE,
            b"Hello, World!".to_vec(),
        );

        // 在偏移量7处写入
        handle.write_at(b"Rust", 7).await.unwrap();

        // 读取验证
        let mut buf = vec![0u8; 13];
        handle.read_at(&mut buf, 0).await.unwrap();

        assert_eq!(&buf[..7], b"Hello, ");
        assert_eq!(&buf[7..11], b"Rust");
    }

    #[tokio::test]
    async fn test_memory_handle_append() {
        let mut handle = MemoryFileHandle::new(
            1,
            "/test/file.txt".to_string(),
            OpenFlags::WRITE_ONLY | OpenFlags::CREATE | OpenFlags::APPEND,
            b"Hello".to_vec(),
        );

        // 追加数据
        handle.write(b", World!").await.unwrap();

        // 读取全部
        let info = handle.stat().await.unwrap();
        assert_eq!(info.size, 13);

        let mut buf = vec![0u8; 13];
        handle.read_at(&mut buf, 0).await.unwrap();
        assert_eq!(&buf, b"Hello, World!");
    }
}
