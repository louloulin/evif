#[test]
fn sqlfs2_core_support_is_enabled_in_default_test_surface() {
    assert!(
        cfg!(feature = "sqlfs"),
        "core sqlfs2 support must be enabled in the default evif-plugins test surface"
    );
}

#[cfg(feature = "sqlfs")]
mod sqlfs2_behavior {
    use evif_core::EvifPlugin;
    use evif_plugins::{SqlfsConfig, SqlfsPlugin};
    use tempfile::tempdir;

    #[tokio::test(flavor = "multi_thread")]
    async fn sqlfs2_reports_core_name_and_supports_basic_file_operations() {
        let temp_dir = tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("sqlfs2.db");
        let plugin = SqlfsPlugin::new(SqlfsConfig {
            db_path: db_path.to_string_lossy().to_string(),
            ..Default::default()
        })
        .expect("plugin");

        assert_eq!(plugin.name(), "sqlfs2");

        plugin.mkdir("/queries", 0o755).await.expect("mkdir");
        plugin.create("/queries/task.sql", 0o644).await.expect("create");
        plugin
            .write(
                "/queries/task.sql",
                b"select 1;".to_vec(),
                0,
                evif_core::WriteFlags::NONE,
            )
            .await
            .expect("write");

        let data = plugin
            .read("/queries/task.sql", 0, 0)
            .await
            .expect("read");
        assert_eq!(data, b"select 1;");

        let entries = plugin.readdir("/queries").await.expect("readdir");
        assert!(entries.iter().any(|entry| entry.name == "task.sql"));

        plugin
            .rename("/queries/task.sql", "/queries/task-2.sql")
            .await
            .expect("rename");
        let renamed = plugin
            .read("/queries/task-2.sql", 0, 0)
            .await
            .expect("read renamed");
        assert_eq!(renamed, b"select 1;");
    }
}
