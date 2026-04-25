use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::HeartbeatFsPlugin;

#[tokio::test]
async fn heartbeatfs_rejects_missing_item_directory_and_exposes_root_readme() {
    let plugin = HeartbeatFsPlugin::new();

    let err = plugin
        .readdir("/missing")
        .await
        .expect_err("missing item should fail");
    assert!(matches!(err, evif_core::EvifError::NotFound(_)));

    let readme = plugin.read("/README", 0, 0).await.expect("readme");
    let readme_str = String::from_utf8(readme).expect("utf8");
    assert!(readme_str.contains("HeartbeatFS"));

    let info = plugin.stat("/README").await.expect("stat readme");
    assert_eq!(info.name, "README");
    assert!(!info.is_dir);
}

#[tokio::test]
async fn heartbeatfs_ctl_updates_timeout_and_reports_status() {
    let plugin = HeartbeatFsPlugin::new();
    plugin.mkdir("/agent-a", 0o755).await.expect("mkdir");

    plugin
        .write("/agent-a/ctl", b"timeout=60".to_vec(), 0, WriteFlags::NONE)
        .await
        .expect("write ctl");

    let ctl = plugin.read("/agent-a/ctl", 0, 0).await.expect("read ctl");
    let ctl_str = String::from_utf8(ctl).expect("utf8");
    assert!(ctl_str.contains("timeout: 60"));
    assert!(ctl_str.contains("status: alive"));
}
