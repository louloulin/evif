// 文件抽象

use crate::error::{VfsError, VfsResult};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// 文件句柄
///
/// 唯一标识一个打开的文件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHandle(u64);

impl FileHandle {
    /// 创建新的文件句柄
    pub fn new(handle: u64) -> Self {
        FileHandle(handle)
    }

    /// 获取句柄值
    pub fn value(&self) -> u64 {
        self.0
    }

    /// 是否为无效句柄
    pub fn is_invalid(&self) -> bool {
        self.0 == 0
    }
}

impl Default for FileHandle {
    fn default() -> Self {
        FileHandle(0)
    }
}

/// 文件打开标志
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFlags {
    /// 只读
    pub read: bool,

    /// 只写
    pub write: bool,

    /// 读写
    pub rw: bool,

    /// 创建（如果不存在）
    pub create: bool,

    /// 独占创建
    pub exclusive: bool,

    /// 追加
    pub append: bool,

    /// 截断
    pub truncate: bool,

    /// 非阻塞
    pub nonblock: bool,

    /// 同步写入
    pub sync: bool,

    /// 不跟随符号链接
    pub nofollow: bool,
}

impl OpenFlags {
    /// 只读模式
    pub const READ_ONLY: OpenFlags = OpenFlags {
        read: true,
        write: false,
        rw: false,
        create: false,
        exclusive: false,
        append: false,
        truncate: false,
        nonblock: false,
        sync: false,
        nofollow: false,
    };

    /// 只写模式
    pub const WRITE_ONLY: OpenFlags = OpenFlags {
        read: false,
        write: true,
        rw: false,
        create: false,
        exclusive: false,
        append: false,
        truncate: false,
        nonblock: false,
        sync: false,
        nofollow: false,
    };

    /// 读写模式
    pub const READ_WRITE: OpenFlags = OpenFlags {
        read: false,
        write: false,
        rw: true,
        create: false,
        exclusive: false,
        append: false,
        truncate: false,
        nonblock: false,
        sync: false,
        nofollow: false,
    };

    /// 创建模式
    pub const CREATE: OpenFlags = OpenFlags {
        read: false,
        write: true,
        rw: false,
        create: true,
        exclusive: false,
        append: false,
        truncate: false,
        nonblock: false,
        sync: false,
        nofollow: false,
    };

    /// 追加模式
    pub const APPEND: OpenFlags = OpenFlags {
        read: false,
        write: true,
        rw: false,
        create: false,
        exclusive: false,
        append: true,
        truncate: false,
        nonblock: false,
        sync: false,
        nofollow: false,
    };

    /// 截断模式
    pub const TRUNCATE: OpenFlags = OpenFlags {
        read: false,
        write: true,
        rw: false,
        create: false,
        exclusive: false,
        append: false,
        truncate: true,
        nonblock: false,
        sync: false,
        nofollow: false,
    };

    /// 检查是否可读
    pub fn can_read(&self) -> bool {
        self.read || self.rw
    }

    /// 检查是否可写
    pub fn can_write(&self) -> bool {
        self.write || self.rw
    }

    /// 检查是否需要创建
    pub fn should_create(&self) -> bool {
        self.create
    }

    /// 检查是否为独占创建
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }
}

/// 文件模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMode {
    /// 权限位
    pub permissions: u32,

    /// 文件类型
    pub file_type: FileType,
}

impl FileMode {
    /// 默认文件模式 (0644)
    pub const DEFAULT_FILE: FileMode = FileMode {
        permissions: 0o644,
        file_type: FileType::Regular,
    };

    /// 默认目录模式 (0755)
    pub const DEFAULT_DIRECTORY: FileMode = FileMode {
        permissions: 0o755,
        file_type: FileType::Directory,
    };

    /// 创建新的文件模式
    pub const fn new(permissions: u32, file_type: FileType) -> Self {
        FileMode {
            permissions,
            file_type,
        }
    }

    /// 获取完整模式
    pub fn mode(&self) -> u32 {
        self.file_type.to_mode_bits() | (self.permissions & 0o777)
    }

    /// 是否可读
    pub fn is_readable(&self) -> bool {
        self.permissions & 0o444 != 0
    }

    /// 是否可写
    pub fn is_writable(&self) -> bool {
        self.permissions & 0o222 != 0
    }

    /// 是否可执行
    pub fn is_executable(&self) -> bool {
        self.permissions & 0o111 != 0
    }
}

impl Default for FileMode {
    fn default() -> Self {
        FileMode::DEFAULT_FILE
    }
}

/// 文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// FIFO
    Fifo,

    /// Socket
    Socket,
}

impl FileType {
    /// 转换为模式位
    pub const fn to_mode_bits(&self) -> u32 {
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
}

/// 文件描述符
///
/// 表示一个打开的文件的状态
#[derive(Debug)]
pub struct File {
    /// 文件句柄
    handle: FileHandle,

    /// 文件偏移量
    offset: AtomicU64,

    /// 打开标志
    flags: OpenFlags,

    /// 文件大小
    size: u64,
}

impl File {
    /// 创建新的文件描述符
    pub fn new(handle: FileHandle, flags: OpenFlags, size: u64) -> Self {
        File {
            handle,
            offset: AtomicU64::new(0),
            flags,
            size,
        }
    }

    /// 获取文件句柄
    pub fn handle(&self) -> FileHandle {
        self.handle
    }

    /// 获取文件偏移量
    pub fn offset(&self) -> u64 {
        self.offset.load(Ordering::Acquire)
    }

    /// 设置文件偏移量
    pub fn set_offset(&self, offset: u64) {
        self.offset.store(offset, Ordering::Release);
    }

    /// 增加文件偏移量
    pub fn add_offset(&self, delta: u64) -> u64 {
        self.offset.fetch_add(delta, Ordering::AcqRel)
    }

    /// 获取打开标志
    pub fn flags(&self) -> OpenFlags {
        self.flags
    }

    /// 获取文件大小
    pub fn size(&self) -> u64 {
        self.size
    }

    /// 设置文件大小
    pub fn set_size(&mut self, size: u64) {
        self.size = size;
    }

    /// 是否可读
    pub fn can_read(&self) -> bool {
        self.flags.can_read()
    }

    /// 是否可写
    pub fn can_write(&self) -> bool {
        self.flags.can_write()
    }

    /// 是否为追加模式
    pub fn is_append(&self) -> bool {
        self.flags.append
    }

    /// 读取数据（更新偏移量）
    pub fn read(&self, buf: &mut [u8], data: &[u8]) -> VfsResult<usize> {
        if !self.can_read() {
            return Err(VfsError::PermissionDenied("文件未以读模式打开".to_string()));
        }

        let current_offset = self.offset();
        let available = data.len() as u64;

        if current_offset >= available {
            return Ok(0);
        }

        let to_read = std::cmp::min(buf.len() as u64, available - current_offset) as usize;
        let start = current_offset as usize;
        let end = start + to_read;

        buf[..to_read].copy_from_slice(&data[start..end]);
        self.set_offset(current_offset + to_read as u64);

        Ok(to_read)
    }

    /// 写入数据（更新偏移量）
    pub fn write(&self, data: &[u8]) -> VfsResult<u64> {
        if !self.can_write() {
            return Err(VfsError::PermissionDenied("文件未以写模式打开".to_string()));
        }

        let bytes_written = if self.is_append() {
            let _ = self.offset.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(self.size)
            });
            data.len() as u64
        } else {
            let current = self.offset();
            self.add_offset(data.len() as u64);
            data.len() as u64
        };

        Ok(bytes_written)
    }

    /// 定位到指定位置
    pub fn seek(&self, pos: SeekFrom) -> VfsResult<u64> {
        let new_offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset < 0 {
                    let offset_abs = offset.unsigned_abs();
                    if offset_abs > self.size {
                        return Err(VfsError::InvalidOperation("定位位置超出文件范围".to_string()));
                    }
                    self.size - offset_abs
                } else {
                    self.size + offset as u64
                }
            }
            SeekFrom::Current(offset) => {
                let current = self.offset();
                if offset < 0 {
                    let offset_abs = offset.unsigned_abs();
                    if offset_abs > current {
                        return Err(VfsError::InvalidOperation("定位位置超出文件范围".to_string()));
                    }
                    current - offset_abs
                } else {
                    current + offset as u64
                }
            }
        };

        self.set_offset(new_offset);
        Ok(new_offset)
    }
}

/// 定位起始位置
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    /// 从开始位置
    Start(u64),

    /// 从结束位置
    End(i64),

    /// 从当前位置
    Current(i64),
}

/// 文件句柄分配器
#[derive(Debug)]
pub struct FileHandleAllocator {
    next_handle: AtomicU64,
}

impl FileHandleAllocator {
    /// 创建新的分配器
    pub fn new() -> Self {
        FileHandleAllocator {
            next_handle: AtomicU64::new(1),
        }
    }

    /// 分配新的文件句柄
    pub fn alloc(&self) -> FileHandle {
        let handle = self.next_handle.fetch_add(1, Ordering::Relaxed);
        FileHandle(handle)
    }

    /// 重置分配器（仅用于测试）
    #[cfg(test)]
    fn reset(&self) {
        self.next_handle.store(1, Ordering::Relaxed);
    }
}

impl Default for FileHandleAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_handle() {
        let handle = FileHandle::new(42);
        assert_eq!(handle.value(), 42);
        assert!(!handle.is_invalid());

        let invalid = FileHandle::default();
        assert!(invalid.is_invalid());
    }

    #[test]
    fn test_open_flags() {
        assert!(OpenFlags::READ_ONLY.can_read());
        assert!(!OpenFlags::READ_ONLY.can_write());

        assert!(OpenFlags::WRITE_ONLY.can_write());
        assert!(!OpenFlags::WRITE_ONLY.can_read());

        assert!(OpenFlags::READ_WRITE.can_read());
        assert!(OpenFlags::READ_WRITE.can_write());

        assert!(OpenFlags::CREATE.should_create());
        assert!(OpenFlags::APPEND.append);
        assert!(OpenFlags::TRUNCATE.truncate);
    }

    #[test]
    fn test_file_mode() {
        let mode = FileMode::DEFAULT_FILE;
        assert!(mode.is_readable());
        assert!(mode.is_writable());
        assert!(!mode.is_executable());

        let dir_mode = FileMode::DEFAULT_DIRECTORY;
        assert!(dir_mode.file_type.is_directory());
    }

    #[test]
    fn test_file_operations() {
        let allocator = FileHandleAllocator::new();
        let handle = allocator.alloc();

        let mut file = File::new(handle, OpenFlags::READ_WRITE, 1024);

        assert_eq!(file.offset(), 0);
        assert!(file.can_read());
        assert!(file.can_write());

        file.set_offset(100);
        assert_eq!(file.offset(), 100);

        file.add_offset(50);
        assert_eq!(file.offset(), 150);
    }

    #[test]
    fn test_file_seek() {
        let allocator = FileHandleAllocator::new();
        let handle = allocator.alloc();

        let file = File::new(handle, OpenFlags::READ_WRITE, 1024);

        // Seek from start
        let pos = file.seek(SeekFrom::Start(100)).unwrap();
        assert_eq!(pos, 100);
        assert_eq!(file.offset(), 100);

        // Seek from current
        let pos = file.seek(SeekFrom::Current(50)).unwrap();
        assert_eq!(pos, 150);
        assert_eq!(file.offset(), 150);

        // Seek from end
        let pos = file.seek(SeekFrom::End(-50)).unwrap();
        assert_eq!(pos, 974);
        assert_eq!(file.offset(), 974);
    }

    #[test]
    fn test_file_read() {
        let allocator = FileHandleAllocator::new();
        let handle = allocator.alloc();

        let data = b"Hello, World!";
        let file = File::new(handle, OpenFlags::READ_ONLY, data.len() as u64);

        let mut buf = [0u8; 5];
        let n = file.read(&mut buf, data).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"Hello");
        assert_eq!(file.offset(), 5);
    }

    #[test]
    fn test_file_write() {
        let allocator = FileHandleAllocator::new();
        let handle = allocator.alloc();

        let file = File::new(handle, OpenFlags::WRITE_ONLY, 0);

        let data = b"Hello, World!";
        let n = file.write(data).unwrap();
        assert_eq!(n, data.len() as u64);
        assert_eq!(file.offset(), data.len() as u64);
    }

    #[test]
    fn test_file_append() {
        let allocator = FileHandleAllocator::new();
        let handle = allocator.alloc();

        let file = File::new(handle, OpenFlags::APPEND, 1024);

        let data = b"Append data";
        let n = file.write(data).unwrap();
        assert_eq!(n, data.len() as u64);
        // Append mode should move offset to end of file
        assert_eq!(file.offset(), 1024);
    }

    #[test]
    fn test_file_handle_allocator() {
        let allocator = FileHandleAllocator::new();

        let h1 = allocator.alloc();
        let h2 = allocator.alloc();
        let h3 = allocator.alloc();

        assert_eq!(h1.value(), 1);
        assert_eq!(h2.value(), 2);
        assert_eq!(h3.value(), 3);
    }

    #[test]
    fn test_seek_bounds() {
        let allocator = FileHandleAllocator::new();
        let handle = allocator.alloc();

        let file = File::new(handle, OpenFlags::READ_WRITE, 100);

        // Seek beyond file size from start should work
        let pos = file.seek(SeekFrom::Start(200)).unwrap();
        assert_eq!(pos, 200);

        // Seek backward beyond start should fail
        let result = file.seek(SeekFrom::Current(-300));
        assert!(result.is_err());
    }
}
