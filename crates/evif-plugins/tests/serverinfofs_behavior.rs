use evif_core::EvifPlugin;
use evif_plugins::ServerInfoFsPlugin;

#[tokio::test]
async fn serverinfofs_rejects_unknown_paths_and_non_root_readdir() {
    let plugin = ServerInfoFsPlugin::new("2.0.0");

    let err = plugin
        .stat("/missing")
        .await
        .expect_err("unknown file should fail");
    assert!(matches!(err, evif_core::EvifError::NotFound(_)));

    let err = plugin
        .readdir("/version")
        .await
        .expect_err("non-root readdir should fail");
    assert!(matches!(err, evif_core::EvifError::InvalidPath(_)));
}

#[tokio::test]
async fn serverinfofs_root_files_are_readable() {
    let plugin = ServerInfoFsPlugin::new("2.0.0");

    let version = plugin.read("/version", 0, 0).await.expect("version");
    assert_eq!(version, b"2.0.0");

    let entries = plugin.readdir("/").await.expect("readdir");
    assert!(entries.iter().any(|entry| entry.name == "info"));
}
