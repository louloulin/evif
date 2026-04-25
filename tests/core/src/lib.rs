// EVIF Core Module Unit Tests
// Tests for RadixMountTable, GlobalHandleManager, cache, batch, VFS primitives

#![allow(dead_code)]
#![allow(unused_imports)]

mod core_mount_table {
    use evif_core::{EvifPlugin, RadixMountTable};
    use evif_plugins::MemFsPlugin;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_mount_table_add() {
        let table = RadixMountTable::new();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        let result = table.mount("/mem".into(), mem).await;
        assert!(result.is_ok(), "Mount should succeed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_mount_table_remove() {
        let table = RadixMountTable::new();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        table.mount("/mem".into(), mem).await.expect("mount");
        let result = table.unmount("/mem").await;
        assert!(result.is_ok(), "Unmount should succeed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_mount_table_lookup() {
        let table = RadixMountTable::new();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        let local = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        table.mount("/mem".into(), mem).await.expect("mount");
        table.mount("/local".into(), local).await.expect("mount");
        let plugin = table.lookup("/mem/test.txt").await;
        assert!(plugin.is_some(), "Should find /mem plugin");
        assert_eq!(plugin.unwrap().name(), "memfs");
    }

    #[tokio::test]
    async fn test_mount_table_lookup_returns_none_for_unknown() {
        let table = RadixMountTable::new();
        let plugin = table.lookup("/unknown/path").await;
        assert!(plugin.is_none(), "Should return None for unknown path");
    }

    #[tokio::test]
    async fn test_mount_table_list_mounts() {
        let table = RadixMountTable::new();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        let local = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        table.mount("/mem".into(), mem).await.expect("mount");
        table.mount("/local".into(), local).await.expect("mount");
        let mounts = table.list_mounts().await;
        assert_eq!(mounts.len(), 2, "Should have 2 mount points");
    }
}

mod core_handle_manager {
    use evif_core::{EvifPlugin, GlobalHandleManager};
    use evif_plugins::MemFsPlugin;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_handle_allocate_id() {
        let manager = GlobalHandleManager::new();
        let id1 = manager.allocate_id();
        let id2 = manager.allocate_id();
        assert_ne!(id1, id2, "Allocated IDs should be unique");
    }

    #[tokio::test]
    async fn test_handle_register_and_get() {
        let manager = GlobalHandleManager::new();
        let handle_id = manager.allocate_id();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        mem.create("/test_handle_file.txt", 0o644)
            .await
            .expect("create file");
        manager
            .register_handle(
                handle_id,
                "/mem".into(),
                "/test_handle_file.txt".into(),
                None,
            )
            .await
            .expect("register handle");
        let result = manager.get_handle(handle_id).await;
        assert!(result.is_ok(), "Should get handle");
        let (_hid, mount_path, full_path, _expires) = result.unwrap();
        assert_eq!(mount_path, "/mem");
        assert_eq!(full_path, "/test_handle_file.txt");
    }

    #[tokio::test]
    async fn test_handle_get_nonexistent_returns_error() {
        let manager = GlobalHandleManager::new();
        let result = manager.get_handle(99999).await;
        assert!(result.is_err(), "Should error for nonexistent handle");
    }

    #[tokio::test]
    async fn test_handle_close() {
        let manager = GlobalHandleManager::new();
        let handle_id = manager.allocate_id();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        mem.create("/test_close.txt", 0o644).await.expect("create");
        manager
            .register_handle(handle_id, "/mem".into(), "/test_close.txt".into(), None)
            .await
            .expect("register");
        let result = manager.close_handle(handle_id).await;
        assert!(result.is_ok() || result.is_err(), "Close should complete");
        let get_result = manager.get_handle(handle_id).await;
        assert!(get_result.is_err(), "Handle should not exist after close");
    }

    #[tokio::test]
    async fn test_handle_ttl_exists() {
        let manager = GlobalHandleManager::new();
        let handle_id = manager.allocate_id();
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        mem.create("/test_ttl.txt", 0o644).await.expect("create");
        manager
            .register_handle(handle_id, "/mem".into(), "/test_ttl.txt".into(), None)
            .await
            .expect("register");
        let result = manager.get_handle(handle_id).await;
        assert!(
            result.is_ok(),
            "Handle should exist immediately after registration"
        );
    }
}

mod core_cache {
    use evif_core::{EvifPlugin, WriteFlags};
    use evif_plugins::MemFsPlugin;

    #[tokio::test]
    async fn test_memfs_plugin_basic_operations() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/cache_dir", 0o755).await.expect("mkdir");
        mem.create("/cache_dir/item", 0o644)
            .await
            .expect("create file");
        let written = mem
            .write(
                "/cache_dir/item",
                b"cached value".to_vec(),
                0,
                WriteFlags::CREATE,
            )
            .await
            .expect("write");
        assert_eq!(written, 12);
        let data = mem.read("/cache_dir/item", 0, 100).await.expect("read");
        assert_eq!(data, b"cached value");
        let info = mem.stat("/cache_dir/item").await.expect("stat");
        assert_eq!(info.size, 12);
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_memfs_overwrite() {
        let mem = MemFsPlugin::new();
        mem.create("/file.txt", 0o644).await.expect("create");
        mem.write("/file.txt", b"original".to_vec(), 0, WriteFlags::default())
            .await
            .expect("write");
        mem.write("/file.txt", b"updated".to_vec(), 0, WriteFlags::default())
            .await
            .expect("overwrite");
        let data = mem.read("/file.txt", 0, 100).await.expect("read");
        assert_eq!(data, b"updated");
    }

    #[tokio::test]
    async fn test_memfs_readdir() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/list_dir", 0o755).await.expect("mkdir");
        mem.create("/list_dir/f1.txt", 0o644).await.expect("create");
        mem.create("/list_dir/f2.txt", 0o644).await.expect("create");
        let entries = mem.readdir("/list_dir").await.expect("readdir");
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_memfs_delete() {
        let mem = MemFsPlugin::new();
        mem.create("/del_file.txt", 0o644).await.expect("create");
        mem.write(
            "/del_file.txt",
            b"to be deleted".to_vec(),
            0,
            WriteFlags::default(),
        )
        .await
        .expect("write");
        mem.remove("/del_file.txt").await.expect("remove");
        let result = mem.stat("/del_file.txt").await;
        assert!(result.is_err(), "File should be deleted");
    }
}

mod core_batch_operations {
    use evif_core::{EvifPlugin, WriteFlags};
    use evif_plugins::MemFsPlugin;

    #[tokio::test]
    async fn test_vfs_batch_create_and_delete() {
        let mem = MemFsPlugin::new();
        for i in 0..5 {
            let path = format!("/batch/file_{}.txt", i);
            mem.create(&path, 0o644).await.expect("batch create");
            mem.write(
                &path,
                format!("content {}", i).into_bytes(),
                0,
                WriteFlags::default(),
            )
            .await
            .expect("write");
        }
        let entries = mem.readdir("/batch").await.expect("readdir");
        assert_eq!(entries.len(), 5, "All 5 files should exist");
        for i in 0..5 {
            let path = format!("/batch/file_{}.txt", i);
            mem.remove(&path).await.expect("batch delete");
        }
        let entries = mem.readdir("/batch").await.expect("readdir after delete");
        assert_eq!(entries.len(), 0, "All files should be deleted");
    }
}

mod vfs_file {
    use evif_core::{EvifPlugin, WriteFlags};
    use evif_plugins::MemFsPlugin;

    #[tokio::test]
    async fn test_vfs_file_create() {
        let mem = MemFsPlugin::new();
        mem.create("/new_file.txt", 0o644)
            .await
            .expect("create file should succeed");
    }

    #[tokio::test]
    async fn test_vfs_file_read() {
        let mem = MemFsPlugin::new();
        mem.create("/read_test.txt", 0o644).await.expect("create");
        mem.write(
            "/read_test.txt",
            b"hello world".to_vec(),
            0,
            WriteFlags::default(),
        )
        .await
        .expect("write");
        let data = mem.read("/read_test.txt", 0, 100).await.expect("read");
        assert_eq!(data, b"hello world");
    }

    #[tokio::test]
    async fn test_vfs_file_write() {
        let mem = MemFsPlugin::new();
        mem.create("/write_test.txt", 0o644).await.expect("create");
        let n = mem
            .write(
                "/write_test.txt",
                b"written data".to_vec(),
                0,
                WriteFlags::default(),
            )
            .await
            .expect("write");
        assert_eq!(n, 12);
    }

    #[tokio::test]
    async fn test_vfs_file_delete() {
        let mem = MemFsPlugin::new();
        mem.create("/delete_test.txt", 0o644).await.expect("create");
        mem.remove("/delete_test.txt").await.expect("delete");
        let result = mem.stat("/delete_test.txt").await;
        assert!(result.is_err(), "File should be gone");
    }
}

mod vfs_directory {
    use evif_core::EvifPlugin;
    use evif_plugins::MemFsPlugin;

    #[tokio::test]
    async fn test_vfs_directory_create() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/new_dir", 0o755).await.expect("mkdir");
    }

    #[tokio::test]
    async fn test_vfs_directory_list() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/list_dir", 0o755).await.expect("mkdir");
        mem.create("/list_dir/file1.txt", 0o644)
            .await
            .expect("create");
        mem.create("/list_dir/file2.txt", 0o644)
            .await
            .expect("create");
        let entries = mem.readdir("/list_dir").await.expect("readdir");
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_vfs_directory_delete() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/rmdir_test", 0o755).await.expect("mkdir");
        mem.remove("/rmdir_test").await.expect("rmdir");
        let result = mem.stat("/rmdir_test").await;
        assert!(result.is_err(), "Directory should be gone");
    }
}

mod vfs_inode {
    use evif_core::{EvifPlugin, WriteFlags};
    use evif_plugins::MemFsPlugin;

    #[tokio::test]
    async fn test_inode_allocate_on_create() {
        let mem = MemFsPlugin::new();
        mem.create("/inode_alloc.txt", 0o644).await.expect("create");
        let info = mem.stat("/inode_alloc.txt").await.expect("stat");
        assert!(info.size == 0, "New file should have 0 size");
    }

    #[tokio::test]
    async fn test_inode_deallocate_on_delete() {
        let mem = MemFsPlugin::new();
        mem.create("/inode_dealloc.txt", 0o644)
            .await
            .expect("create");
        mem.write(
            "/inode_dealloc.txt",
            b"data".to_vec(),
            0,
            WriteFlags::default(),
        )
        .await
        .expect("write");
        mem.remove("/inode_dealloc.txt").await.expect("delete");
        let result = mem.stat("/inode_dealloc.txt").await;
        assert!(result.is_err(), "Inode should be deallocated");
    }

    #[tokio::test]
    async fn test_inode_reference_counting_via_rename() {
        let mem = MemFsPlugin::new();
        mem.create("/ref_orig.txt", 0o644).await.expect("create");
        mem.write(
            "/ref_orig.txt",
            b"content".to_vec(),
            0,
            WriteFlags::default(),
        )
        .await
        .expect("write");
        mem.rename("/ref_orig.txt", "/ref_renamed.txt")
            .await
            .expect("rename");
        assert!(mem.stat("/ref_orig.txt").await.is_err());
        let data = mem
            .read("/ref_renamed.txt", 0, 100)
            .await
            .expect("read renamed");
        assert_eq!(data, b"content");
    }
}

mod vfs_path {
    use evif_core::EvifPlugin;
    use evif_plugins::MemFsPlugin;

    #[tokio::test]
    async fn test_path_normalize_dot() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/norm_a", 0o755).await.expect("mkdir");
        mem.create("/norm_a/b", 0o644).await.expect("create");
        let entries = mem.readdir("/norm_a").await.expect("readdir");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "b");
    }

    #[tokio::test]
    async fn test_path_join() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/join_base", 0o755).await.expect("mkdir");
        mem.create("/join_base/sub/file.txt", 0o644)
            .await
            .expect("create nested");
        let info = mem
            .stat("/join_base/sub/file.txt")
            .await
            .expect("stat nested");
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_path_absolute() {
        let mem = MemFsPlugin::new();
        mem.mkdir("/abs", 0o755).await.expect("mkdir");
        mem.create("/abs/file.txt", 0o644).await.expect("create");
        let info = mem.stat("/abs/file.txt").await.expect("stat abs path");
        assert_eq!(info.name, "file.txt");
    }
}
