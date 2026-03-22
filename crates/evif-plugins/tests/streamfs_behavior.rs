use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::StreamFsPlugin;

#[tokio::test]
async fn streamfs_root_can_be_statted_and_lists_created_streams() {
    let plugin = StreamFsPlugin::new();

    let root = plugin.stat("/").await.expect("root stat");
    assert!(root.is_dir);

    plugin.create("/events", 0o644).await.expect("create");
    plugin
        .write("/events", b"payload".to_vec(), 0, WriteFlags::APPEND)
        .await
        .expect("write");

    let entries = plugin.readdir("/").await.expect("readdir");
    assert!(entries.iter().any(|entry| entry.name == "events"));
}
