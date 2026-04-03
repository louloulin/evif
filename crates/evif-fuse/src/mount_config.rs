// FUSE 挂载配置
//
// 管理文件系统挂载的配置选项

use std::path::PathBuf;

/// FUSE 挂载配置
#[derive(Debug, Clone)]
pub struct FuseMountConfig {
    /// 挂载点路径
    pub mount_point: PathBuf,

    /// 根路径（在 EVIF 中的路径）
    pub root_path: PathBuf,

    /// 允许写操作
    pub allow_write: bool,

    /// 允许其他用户访问
    pub allow_other: bool,

    /// 缓存大小（inode 缓存条目数）
    pub cache_size: usize,

    /// 缓存超时时间（秒）
    pub cache_timeout: u64,
}

impl FuseMountConfig {
    /// 创建默认配置
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self {
            mount_point: PathBuf::from("/mnt/evif"),
            root_path: PathBuf::from("/"),
            allow_write: false,
            allow_other: false,
            cache_size: 10000,
            cache_timeout: 60,
        }
    }

    /// 创建只读挂载配置
    pub fn readonly(mount_point: PathBuf) -> Self {
        Self {
            mount_point,
            root_path: PathBuf::from("/"),
            allow_write: false,
            allow_other: false,
            cache_size: 10000,
            cache_timeout: 60,
        }
    }

    /// 创建读写挂载配置
    pub fn readwrite(mount_point: PathBuf) -> Self {
        Self {
            mount_point,
            root_path: PathBuf::from("/"),
            allow_write: true,
            allow_other: false,
            cache_size: 10000,
            cache_timeout: 60,
        }
    }
}

/// FUSE 挂载选项
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountOptions {
    /// 只读挂载
    ReadOnly,

    /// 读写挂载
    ReadWrite,

    /// 允许其他用户访问
    AllowOther,

    /// 允许其他用户映射
    AllowMap,

    /// 允许其他用户锁定
    AllowLock,

    /// 允许其他用户执行
    AllowExec,

    /// 禁用 FUSE 缓存
    NoCache,

    /// 禁用属性缓存
    NoAttrCache,

    /// 禁用目录缓存
    NoDirCache,

    /// 启用内核缓存
    KernelCache,

    /// 启用异步读取
    AsyncRead,

    /// 启用写入时复制
    WritebackCache,
}

impl MountOptions {
    /// 转换为 FUSE 挂载选项字符串
    pub fn as_fuse_option(&self) -> &'static str {
        match self {
            MountOptions::ReadOnly => "ro",
            MountOptions::ReadWrite => "rw",
            MountOptions::AllowOther => "allow_other",
            MountOptions::AllowMap => "allow_map",
            MountOptions::AllowLock => "allow_lock",
            MountOptions::AllowExec => "allow_exec",
            MountOptions::NoCache => "no_cache",
            MountOptions::NoAttrCache => "no_attr_cache",
            MountOptions::NoDirCache => "no_dir_cache",
            MountOptions::KernelCache => "kernel_cache",
            MountOptions::AsyncRead => "async_read",
            MountOptions::WritebackCache => "writeback_cache",
        }
    }

    /// 从字符串解析挂载选项
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ro" => Some(MountOptions::ReadOnly),
            "rw" => Some(MountOptions::ReadWrite),
            "allow_other" => Some(MountOptions::AllowOther),
            "allow_map" => Some(MountOptions::AllowMap),
            "allow_lock" => Some(MountOptions::AllowLock),
            "allow_exec" => Some(MountOptions::AllowExec),
            "no_cache" => Some(MountOptions::NoCache),
            "no_attr_cache" => Some(MountOptions::NoAttrCache),
            "no_dir_cache" => Some(MountOptions::NoDirCache),
            "kernel_cache" => Some(MountOptions::KernelCache),
            "async_read" => Some(MountOptions::AsyncRead),
            "writeback_cache" => Some(MountOptions::WritebackCache),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_default_config() {
        let config = FuseMountConfig::default();
        assert_eq!(config.mount_point, PathBuf::from("/mnt/evif"));
        assert!(!config.allow_write);
        assert!(!config.allow_other);
        assert_eq!(config.cache_size, 10000);
        assert_eq!(config.cache_timeout, 60);
    }

    #[test]
    fn test_readonly_config() {
        let config = FuseMountConfig::readonly(PathBuf::from("/tmp/evif"));
        assert_eq!(config.mount_point, PathBuf::from("/tmp/evif"));
        assert!(!config.allow_write);
    }

    #[test]
    fn test_readwrite_config() {
        let config = FuseMountConfig::readwrite(PathBuf::from("/tmp/evif"));
        assert_eq!(config.mount_point, PathBuf::from("/tmp/evif"));
        assert!(config.allow_write);
    }

    #[test]
    fn test_mount_options() {
        assert_eq!(MountOptions::ReadOnly.as_fuse_option(), "ro");
        assert_eq!(MountOptions::ReadWrite.as_fuse_option(), "rw");
        assert_eq!(MountOptions::AllowOther.as_fuse_option(), "allow_other");
    }

    #[test]
    fn test_mount_options_parse() {
        assert_eq!(MountOptions::from_str("ro"), Some(MountOptions::ReadOnly));
        assert_eq!(MountOptions::from_str("rw"), Some(MountOptions::ReadWrite));
        assert_eq!(
            MountOptions::from_str("allow_other"),
            Some(MountOptions::AllowOther)
        );
        assert_eq!(MountOptions::from_str("invalid"), None);
    }
}
