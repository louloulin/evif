// VFS 错误类型定义

use std::fmt;

/// VFS 操作结果类型
pub type VfsResult<T> = std::result::Result<T, VfsError>;

/// VFS 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsError {
    /// 路径未找到
    PathNotFound(String),

    /// 文件未找到
    FileNotFound(String),

    /// 目录未找到
    DirectoryNotFound(String),

    /// 文件已存在
    FileExists(String),

    /// 目录已存在
    DirectoryExists(String),

    /// 不是目录
    NotADirectory(String),

    /// 不是文件
    NotAFile(String),

    /// 权限被拒绝
    PermissionDenied(String),

    /// 无效的路径
    InvalidPath(String),

    /// 路径过长
    PathTooLong,

    /// 名字过长
    NameTooLong,

    /// 无效的文件句柄
    InvalidFileHandle(u64),

    /// 文件句柄已关闭
    FileClosed,

    /// 无效的操作
    InvalidOperation(String),

    /// 目录非空
    DirectoryNotEmpty(String),

    /// 符号链接循环
    SymbolicLinkLoop(String),

    /// 只读文件系统
    ReadOnlyFileSystem,

    /// 磁盘空间不足
    NoSpaceLeft,

    /// 配额超出
    QuotaExceeded,

    /// I/O 错误
    IoError(String),

    /// 认证错误
    AuthError(String),

    /// 超时
    Timeout,

    /// 连接中断
    ConnectionLost,

    /// 内部错误
    InternalError(String),

    /// 不支持的操作
    Unsupported(String),
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::PathNotFound(path) => write!(f, "路径未找到: {}", path),
            VfsError::FileNotFound(file) => write!(f, "文件未找到: {}", file),
            VfsError::DirectoryNotFound(dir) => write!(f, "目录未找到: {}", dir),
            VfsError::FileExists(file) => write!(f, "文件已存在: {}", file),
            VfsError::DirectoryExists(dir) => write!(f, "目录已存在: {}", dir),
            VfsError::NotADirectory(path) => write!(f, "不是目录: {}", path),
            VfsError::NotAFile(path) => write!(f, "不是文件: {}", path),
            VfsError::PermissionDenied(msg) => write!(f, "权限被拒绝: {}", msg),
            VfsError::InvalidPath(path) => write!(f, "无效的路径: {}", path),
            VfsError::PathTooLong => write!(f, "路径过长"),
            VfsError::NameTooLong => write!(f, "名字过长"),
            VfsError::InvalidFileHandle(handle) => write!(f, "无效的文件句柄: {}", handle),
            VfsError::FileClosed => write!(f, "文件句柄已关闭"),
            VfsError::InvalidOperation(op) => write!(f, "无效的操作: {}", op),
            VfsError::DirectoryNotEmpty(dir) => write!(f, "目录非空: {}", dir),
            VfsError::SymbolicLinkLoop(path) => write!(f, "符号链接循环: {}", path),
            VfsError::ReadOnlyFileSystem => write!(f, "只读文件系统"),
            VfsError::NoSpaceLeft => write!(f, "磁盘空间不足"),
            VfsError::QuotaExceeded => write!(f, "配额超出"),
            VfsError::IoError(msg) => write!(f, "I/O 错误: {}", msg),
            VfsError::AuthError(msg) => write!(f, "认证错误: {}", msg),
            VfsError::Timeout => write!(f, "操作超时"),
            VfsError::ConnectionLost => write!(f, "连接中断"),
            VfsError::InternalError(msg) => write!(f, "内部错误: {}", msg),
            VfsError::Unsupported(msg) => write!(f, "不支持的操作: {}", msg),
        }
    }
}

impl std::error::Error for VfsError {}

// 从标准错误类型转换
impl From<std::io::Error> for VfsError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => VfsError::FileNotFound(err.to_string()),
            std::io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(err.to_string()),
            std::io::ErrorKind::AlreadyExists => VfsError::FileExists(err.to_string()),
            std::io::ErrorKind::InvalidInput => VfsError::InvalidPath(err.to_string()),
            std::io::ErrorKind::NotFound => VfsError::PathNotFound(err.to_string()),
            _ => VfsError::IoError(err.to_string()),
        }
    }
}

impl From<evif_graph::GraphError> for VfsError {
    fn from(err: evif_graph::GraphError) -> Self {
        VfsError::InternalError(format!("图错误: {}", err))
    }
}

impl From<evif_auth::AuthError> for VfsError {
    fn from(err: evif_auth::AuthError) -> Self {
        VfsError::AuthError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = VfsError::FileNotFound("test.txt".to_string());
        assert_eq!(err.to_string(), "文件未找到: test.txt");

        let err = VfsError::PermissionDenied("访问被拒绝".to_string());
        assert_eq!(err.to_string(), "权限被拒绝: 访问被拒绝");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let vfs_err: VfsError = io_err.into();
        matches!(vfs_err, VfsError::FileNotFound(_));
    }

    #[test]
    fn test_invalid_handle_error() {
        let err = VfsError::InvalidFileHandle(12345);
        assert!(err.to_string().contains("12345"));
    }

    #[test]
    fn test_readonly_filesystem() {
        let err = VfsError::ReadOnlyFileSystem;
        assert_eq!(err.to_string(), "只读文件系统");
    }
}
