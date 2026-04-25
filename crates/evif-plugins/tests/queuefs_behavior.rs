use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::QueueFsPlugin;

#[tokio::test]
async fn queuefs_rejects_missing_queue_directory_and_exposes_root_readme() {
    let plugin = QueueFsPlugin::new();

    let err = plugin
        .readdir("/missing")
        .await
        .expect_err("missing queue should fail");
    assert!(matches!(err, evif_core::EvifError::NotFound(_)));

    let readme = plugin.read("/README", 0, 0).await.expect("readme");
    let readme_str = String::from_utf8(readme).expect("utf8");
    assert!(readme_str.contains("QueueFS"));

    let info = plugin.stat("/README").await.expect("stat readme");
    assert_eq!(info.name, "README");
    assert!(!info.is_dir);
}

#[tokio::test]
async fn queuefs_queue_controls_work_after_mounting() {
    let plugin = QueueFsPlugin::new();
    plugin.mkdir("/orders", 0o755).await.expect("mkdir");

    plugin
        .write("/orders/enqueue", b"job-1".to_vec(), 0, WriteFlags::NONE)
        .await
        .expect("enqueue");

    let peek = plugin.read("/orders/peek", 0, 0).await.expect("peek");
    let peek_str = String::from_utf8(peek).expect("utf8");
    assert!(peek_str.contains("job-1"));

    let size = plugin.read("/orders/size", 0, 0).await.expect("size");
    assert_eq!(size, b"1");
}
