// EVIF VFS - Virtual Filesystem Abstraction Layer
// 基于 POSIX 的虚拟文件系统抽象层

pub mod dentry;
pub mod dir;
pub mod error;
pub mod file;
pub mod filesystem;
pub mod inode;
pub mod path;
pub mod vfs;
pub mod vnode;

// Re-export main types
pub use dentry::{DEntry, DEntryCache};
pub use dir::{DirEntry, Directory};
pub use error::{VfsError, VfsResult};
pub use file::{File, FileHandle, FileMode, OpenFlags};
pub use filesystem::{FileAttributes, FileSystem, FileSystemStats, FileType};
pub use inode::{INode, INodeCache};
pub use path::PathResolver;
pub use vfs::{Vfs, VfsConfig};
pub use vnode::{VNode, VNodeData};

/// VFS 版本信息
pub const VFS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// VFS 核心特性
pub struct VfsFeatures;

impl VfsFeatures {
    /// 支持的操作
    pub fn supported_operations() -> &'static [&'static str] {
        &[
            "open", "close", "read", "write", "seek", "create", "unlink", "rename", "mkdir",
            "rmdir", "readdir", "getattr", "setattr", "symlink", "readlink", "chmod", "chown",
            "stat", "fstat", "lstat",
        ]
    }

    /// 支持的文件类型
    pub fn supported_types() -> &'static [&'static str] {
        &[
            "regular",
            "directory",
            "symlink",
            "character",
            "block",
            "fifo",
            "socket",
        ]
    }
}
