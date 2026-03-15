// 文件系统抽象 Trait - POSIX 风格接口

use crate::dir::{DirEntry, Directory};
use crate::error::{VfsError, VfsResult};
use crate::file::{FileHandle, FileMode, OpenFlags};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// 文件系统属性
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAttributes {
    /// 文件大小（字节）
    pub size: u64,

    /// 文件类型
    pub file_type: FileType,

    /// 权限模式
    pub mode: u32,

    /// 硬链接数
    pub nlink: u64,

    /// 用户 ID
    pub uid: u32,

    /// 组 ID
    pub gid: u32,

    /// 访问时间
    pub atime: u64,

    /// 修改时间
    pub mtime: u64,

    /// 创建时间
    pub ctime: u64,
}

impl Default for FileAttributes {
    fn default() -> Self {
        FileAttributes {
            size: 0,
            file_type: FileType::Regular,
            mode: 0o644,
            nlink: 1,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }
}

/// 文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// 普通文件
    Regular,

    /// 目录
    Directory,

    /// 符号链接
    Symlink,

    /// 字符设备
    Character,

    /// 块设备
    Block,

    /// FIFO 管道
    Fifo,

    /// Socket
    Socket,
}

impl FileType {
    /// 是否为目录
    pub fn is_directory(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    /// 是否为普通文件
    pub fn is_regular(&self) -> bool {
        matches!(self, FileType::Regular)
    }

    /// 是否为符号链接
    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }

    /// 转换为模式位
    pub fn to_mode_bits(&self) -> u32 {
        match self {
            FileType::Regular => 0o100000,
            FileType::Directory => 0o040000,
            FileType::Symlink => 0o120000,
            FileType::Character => 0o020000,
            FileType::Block => 0o060000,
            FileType::Fifo => 0o010000,
            FileType::Socket => 0o140000,
        }
    }
}

/// 文件系统统计信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSystemStats {
    /// 总空间
    pub total_space: u64,

    /// 可用空间
    pub free_space: u64,

    /// 可用空间（非特权用户）
    pub available_space: u64,

    /// 总文件节点数
    pub total_files: u64,

    /// 可用文件节点数
    pub free_files: u64,

    /// 文件系统 ID
    pub fs_id: u64,

    /// 文件系统标志
    pub flags: u64,

    /// 最大文件名长度
    pub max_name_length: u32,
}

impl Default for FileSystemStats {
    fn default() -> Self {
        FileSystemStats {
            total_space: 0,
            free_space: 0,
            available_space: 0,
            total_files: 0,
            free_files: 0,
            fs_id: 0,
            flags: 0,
            max_name_length: 255,
        }
    }
}

/// 文件系统抽象接口
#[async_trait]
pub trait FileSystem: Send + Sync {
    // ========== 文件操作 ==========

    /// 打开文件
    async fn open(&self, path: &Path, flags: OpenFlags) -> VfsResult<FileHandle>;

    /// 关闭文件
    async fn close(&self, handle: FileHandle) -> VfsResult<()>;

    /// 读取文件
    async fn read(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> VfsResult<usize>;

    /// 写入文件
    async fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize>;

    /// 同步文件到磁盘
    async fn fsync(&self, handle: FileHandle) -> VfsResult<()>;

    /// 获取文件大小
    async fn get_file_size(&self, path: &Path) -> VfsResult<u64>;

    /// 设置文件大小
    async fn set_file_size(&self, path: &Path, size: u64) -> VfsResult<()>;

    /// 创建文件
    async fn create(&self, path: &Path, mode: FileMode) -> VfsResult<FileHandle>;

    /// 删除文件
    async fn unlink(&self, path: &Path) -> VfsResult<()>;

    /// 重命名文件
    async fn rename(&self, old_path: &Path, new_path: &Path) -> VfsResult<()>;

    // ========== 目录操作 ==========

    /// 创建目录
    async fn mkdir(&self, path: &Path, mode: FileMode) -> VfsResult<()>;

    /// 删除目录
    async fn rmdir(&self, path: &Path) -> VfsResult<()>;

    /// 读取目录内容
    async fn readdir(&self, path: &Path) -> VfsResult<Vec<DirEntry>>;

    /// 打开目录
    async fn opendir(&self, path: &Path) -> VfsResult<u64>;

    /// 关闭目录
    async fn closedir(&self, dir_handle: u64) -> VfsResult<()>;

    // ========== 元数据操作 ==========

    /// 获取文件属性
    async fn getattr(&self, path: &Path) -> VfsResult<FileAttributes>;

    /// 设置文件属性
    async fn setattr(&self, path: &Path, attrs: FileAttributes) -> VfsResult<()>;

    /// 更改文件权限
    async fn chmod(&self, path: &Path, mode: u32) -> VfsResult<()>;

    /// 更改文件所有者
    async fn chown(&self, path: &Path, uid: u32, gid: u32) -> VfsResult<()>;

    /// 更改访问和修改时间
    async fn utime(&self, path: &Path, atime: u64, mtime: u64) -> VfsResult<()>;

    // ========== 符号链接操作 ==========

    /// 创建符号链接
    async fn symlink(&self, target: &Path, link_path: &Path) -> VfsResult<()>;

    /// 读取符号链接目标
    async fn readlink(&self, path: &Path) -> VfsResult<PathBuf>;

    // ========== 文件系统操作 ==========

    /// 获取文件系统统计信息
    async fn statfs(&self) -> VfsResult<FileSystemStats>;

    /// 同步文件系统
    async fn sync(&self) -> VfsResult<()>;

    /// 检查路径是否存在
    async fn exists(&self, path: &Path) -> VfsResult<bool>;

    /// 检查是否为文件
    async fn is_file(&self, path: &Path) -> VfsResult<bool>;

    /// 检查是否为目录
    async fn is_directory(&self, path: &Path) -> VfsResult<bool>;

    /// 获取绝对路径
    async fn realpath(&self, path: &Path) -> VfsResult<PathBuf>;

    // ========== 默认实现 ==========

    /// 默认实现：批量读取
    async fn read_full(&self, handle: FileHandle, offset: u64, size: usize) -> VfsResult<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        let total_read = self.read(handle, offset, &mut buffer).await?;
        buffer.truncate(total_read);
        Ok(buffer)
    }

    /// 默认实现：批量写入
    async fn write_full(&self, handle: FileHandle, offset: u64, data: &[u8]) -> VfsResult<usize> {
        let mut total_written = 0;
        let mut current_offset = offset;

        while total_written < data.len() {
            let written = self
                .write(handle, current_offset, &data[total_written..])
                .await?;
            if written == 0 {
                return Err(VfsError::NoSpaceLeft);
            }
            total_written += written;
            current_offset += written as u64;
        }

        Ok(total_written)
    }

    /// 默认实现：创建所有父目录
    async fn mkdir_all(&self, path: &Path) -> VfsResult<()> {
        if self.exists(path).await? {
            return if self.is_directory(path).await? {
                Ok(())
            } else {
                Err(VfsError::NotADirectory(path.display().to_string()))
            };
        }

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                self.mkdir_all(parent).await?;
            }
        }

        self.mkdir(path, FileMode::default()).await
    }

    /// 默认实现：递归删除目录
    async fn remove_dir_all(&self, path: &Path) -> VfsResult<()> {
        if !self.is_directory(path).await? {
            return self.unlink(path).await;
        }

        let entries = self.readdir(path).await?;
        for entry in entries {
            let entry_path = path.join(entry.name);
            if entry.file_type.is_directory() {
                self.remove_dir_all(&entry_path).await?;
            } else {
                self.unlink(&entry_path).await?;
            }
        }

        self.rmdir(path).await
    }
}

/// 文件系统构建器
pub struct FileSystemBuilder {
    root: PathBuf,
    read_only: bool,
    max_name_length: usize,
}

impl Default for FileSystemBuilder {
    fn default() -> Self {
        FileSystemBuilder {
            root: PathBuf::from("/"),
            read_only: false,
            max_name_length: 255,
        }
    }
}

impl FileSystemBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root(mut self, root: impl AsRef<Path>) -> Self {
        self.root = root.as_ref().to_path_buf();
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn max_name_length(mut self, length: usize) -> Self {
        self.max_name_length = length;
        self
    }

    pub fn build(self) -> FileSystemConfig {
        FileSystemConfig {
            root: self.root,
            read_only: self.read_only,
            max_name_length: self.max_name_length,
        }
    }
}

/// 文件系统配置
#[derive(Debug, Clone)]
pub struct FileSystemConfig {
    pub root: PathBuf,
    pub read_only: bool,
    pub max_name_length: usize,
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        FileSystemBuilder::new().build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type() {
        assert!(FileType::Directory.is_directory());
        assert!(FileType::Regular.is_regular());
        assert!(FileType::Symlink.is_symlink());
        assert!(!FileType::Regular.is_directory());
    }

    #[test]
    fn test_file_attributes_default() {
        let attrs = FileAttributes::default();
        assert_eq!(attrs.size, 0);
        assert_eq!(attrs.file_type, FileType::Regular);
        assert_eq!(attrs.mode, 0o644);
    }

    #[test]
    fn test_file_stats_default() {
        let stats = FileSystemStats::default();
        assert_eq!(stats.max_name_length, 255);
    }

    #[test]
    fn test_filesystem_builder() {
        let config = FileSystemBuilder::new()
            .root("/tmp/vfs")
            .read_only(true)
            .max_name_length(512)
            .build();

        assert_eq!(config.root, PathBuf::from("/tmp/vfs"));
        assert!(config.read_only);
        assert_eq!(config.max_name_length, 512);
    }

    #[test]
    fn test_file_type_mode_bits() {
        let dir_mode = FileType::Directory.to_mode_bits();
        assert_eq!(dir_mode, 0o040000);

        let file_mode = FileType::Regular.to_mode_bits();
        assert_eq!(file_mode, 0o100000);
    }
}
