// FUSE 集成测试
//
// 测试 EVIF FUSE 文件系统的核心功能
//
// 注意：这些测试需要 FUSE 环境（Linux/macOS）
// 在没有 FUSE 环境的系统上会跳过

#[cfg(test)]
mod fuse_integration_tests {
    use evif_fuse::{
        DirCache, DirEntry, EvifFuseFuse, FuseMountBuilder, FuseMountConfig, InodeManager,
        MountOptions, RadixMountTable, ROOT_INODE,
    };
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    /// 创建测试用挂载表
    async fn create_test_mount_table() -> Arc<RadixMountTable> {
        let mount_table = Arc::new(RadixMountTable::new());

        // 这里可以添加测试插件
        // 例如 memfs, localfs 等

        mount_table
    }

    #[test]
    fn test_fuse_filesystem_creation() {
        let rt = Runtime::new().unwrap();
        let mount_table = rt.block_on(async { create_test_mount_table().await });

        let config = FuseMountConfig {
            mount_point: PathBuf::from("/tmp/evif-test"),
            root_path: PathBuf::from("/"),
            allow_write: false,
            allow_other: false,
            cache_size: 1000,
            cache_timeout: 60,
        };

        let result = EvifFuseFuse::new(mount_table, PathBuf::from("/"), &config);

        assert!(result.is_ok());
        let fs = result.unwrap();
        assert!(!fs.allow_write);
    }

    #[test]
    fn test_fuse_readwrite_mount() {
        let rt = Runtime::new().unwrap();
        let mount_table = rt.block_on(async { create_test_mount_table().await });

        let config = FuseMountConfig {
            mount_point: PathBuf::from("/tmp/evif-test"),
            root_path: PathBuf::from("/"),
            allow_write: true,
            allow_other: false,
            cache_size: 1000,
            cache_timeout: 60,
        };

        let result = EvifFuseFuse::new(mount_table, PathBuf::from("/"), &config);

        assert!(result.is_ok());
        let fs = result.unwrap();
        assert!(fs.allow_write);
    }

    #[test]
    fn test_inode_manager() {
        let manager = InodeManager::new(100);

        // 测试根目录
        assert_eq!(manager.get_inode("/"), Some(ROOT_INODE));
        assert_eq!(manager.get_path(ROOT_INODE), Some("/".to_string()));

        // 测试创建 inode
        let inode1 = manager.get_or_create("/test.txt");
        let inode2 = manager.get_or_create("/dir/");

        assert!(inode1 > ROOT_INODE);
        assert!(inode2 > ROOT_INODE);
        assert_ne!(inode1, inode2);

        // 测试双向映射
        assert_eq!(manager.get_inode("/test.txt"), Some(inode1));
        assert_eq!(manager.get_path(inode1), Some("/test.txt".to_string()));
    }

    #[test]
    fn test_inode_recycle() {
        let manager = InodeManager::new(100);

        let inode = manager.get_or_create("/to_delete.txt");
        assert_eq!(manager.get_path(inode), Some("/to_delete.txt".to_string()));

        manager.recycle(inode);
        assert_eq!(manager.get_path(inode), None);
        assert_eq!(manager.get_inode("/to_delete.txt"), None);
    }

    #[test]
    fn test_dir_cache() {
        let cache = DirCache::new(10);

        // 测试缓存命中
        cache.put(
            "/dir/".to_string(),
            vec![
                DirEntry::new(10, "file1.txt".to_string(), false),
                DirEntry::new(11, "file2.txt".to_string(), false),
            ],
        );

        let entries = cache.get("/dir/");
        assert!(entries.is_some());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "file1.txt");
    }

    #[test]
    fn test_dir_cache_invalidate() {
        let cache = DirCache::new(10);

        cache.put(
            "/dir/".to_string(),
            vec![DirEntry::new(10, "file.txt".to_string(), false)],
        );

        cache.invalidate("/dir/");

        let entries = cache.get("/dir/");
        assert!(entries.is_none());
    }

    #[test]
    fn test_dir_cache_stats() {
        let cache = DirCache::new(10);

        cache.put(
            "/dir/".to_string(),
            vec![DirEntry::new(10, "file.txt".to_string(), false)],
        );

        let (current, max, ttl) = cache.stats();
        assert_eq!(current, 1);
        assert_eq!(max, 10000);
        assert_eq!(ttl, 10);
    }

    #[test]
    fn test_mount_config() {
        let config = FuseMountConfig::default();
        assert_eq!(config.mount_point, PathBuf::from("/mnt/evif"));
        assert!(!config.allow_write);
        assert!(!config.allow_other);

        let readonly_config = FuseMountConfig::readonly(PathBuf::from("/tmp/evif"));
        assert_eq!(readonly_config.mount_point, PathBuf::from("/tmp/evif"));
        assert!(!readonly_config.allow_write);

        let readwrite_config = FuseMountConfig::readwrite(PathBuf::from("/tmp/evif"));
        assert_eq!(readwrite_config.mount_point, PathBuf::from("/tmp/evif"));
        assert!(readwrite_config.allow_write);
    }

    #[test]
    fn test_mount_builder() {
        let config = FuseMountBuilder::new()
            .mount_point(Path::new("/tmp/evif-test"))
            .allow_write(true)
            .cache_size(5000)
            .cache_timeout(120)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.mount_point, PathBuf::from("/tmp/evif-test"));
        assert!(config.allow_write);
        assert_eq!(config.cache_size, 5000);
        assert_eq!(config.cache_timeout, 120);
    }

    #[test]
    fn test_mount_options() {
        assert_eq!(MountOptions::ReadOnly.as_fuse_option(), "ro");
        assert_eq!(MountOptions::ReadWrite.as_fuse_option(), "rw");
        assert_eq!(MountOptions::AllowOther.as_fuse_option(), "allow_other");

        assert_eq!(MountOptions::from_str("ro"), Some(MountOptions::ReadOnly));
        assert_eq!(MountOptions::from_str("rw"), Some(MountOptions::ReadWrite));
        assert_eq!(
            MountOptions::from_str("allow_other"),
            Some(MountOptions::AllowOther)
        );
        assert_eq!(MountOptions::from_str("invalid"), None);
    }
}

#[cfg(test)]
mod fuse_functional_tests {
    use evif_fuse::{EvifFuseFuse, FuseMountConfig, RadixMountTable};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    /// 测试路径解析
    #[test]
    fn test_path_resolution() {
        let rt = Runtime::new().unwrap();
        let mount_table = Arc::new(RadixMountTable::new());

        let config = FuseMountConfig {
            mount_point: PathBuf::from("/tmp/evif-test"),
            root_path: PathBuf::from("/"),
            allow_write: false,
            allow_other: false,
            cache_size: 1000,
            cache_timeout: 60,
        };

        let fs = EvifFuseFuse::new(mount_table, PathBuf::from("/"), &config).unwrap();

        // 测试根路径解析
        let result = fs.resolve_path(1, Path::new("/"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/");

        // 测试相对路径解析
        let result = fs.resolve_path(1, Path::new("/test/file.txt"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/test/file.txt");
    }

    /// 测试文件句柄管理
    #[test]
    fn test_file_handle_management() {
        let rt = Runtime::new().unwrap();
        let mount_table = Arc::new(RadixMountTable::new());

        let config = FuseMountConfig::default();
        let fs = EvifFuseFuse::new(mount_table, PathBuf::from("/"), &config).unwrap();

        // 测试句柄分配
        let ino = 10;
        let handle1 = fs.allocate_handle(ino);
        let handle2 = fs.allocate_handle(ino);

        assert_eq!(handle1, ino);
        assert_eq!(handle2, ino);

        // 测试句柄查询
        assert_eq!(fs.get_handle_inode(handle1), Some(ino));
        assert_eq!(fs.get_handle_inode(handle2), Some(ino));

        // 测试句柄释放
        fs.deallocate_handle(ino);
        assert_eq!(fs.get_handle_inode(handle1), None);
    }

    /// 测试统计信息
    #[test]
    fn test_stats() {
        let rt = Runtime::new().unwrap();
        let mount_table = Arc::new(RadixMountTable::new());

        let config = FuseMountConfig::default();
        let fs = EvifFuseFuse::new(mount_table, PathBuf::from("/"), &config).unwrap();

        let (blocks, bfree, files, ffree) = fs.get_stats();
        assert!(blocks > 0);
        assert!(bfree > 0);
        assert!(files > 0);
        assert!(ffree > 0);
    }
}
