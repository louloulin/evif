use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::ContextFsPlugin;

#[tokio::test]
async fn contextfs_exposes_seeded_layers_and_root_metadata() {
    let plugin = ContextFsPlugin::new();

    let root = plugin.readdir("/").await.expect("root listing");
    let names: Vec<String> = root.into_iter().map(|entry| entry.name).collect();
    assert!(names.iter().any(|name| name == "L0"));
    assert!(names.iter().any(|name| name == "L1"));
    assert!(names.iter().any(|name| name == "L2"));
    assert!(names.iter().any(|name| name == "README"));
    assert!(names.iter().any(|name| name == ".meta"));

    let readme = plugin.read("/README", 0, 0).await.expect("readme");
    let readme_str = String::from_utf8(readme).expect("utf8");
    assert!(readme_str.contains("ContextFS"));

    let current = plugin.read("/L0/current", 0, 0).await.expect("current");
    let current_str = String::from_utf8(current).expect("utf8");
    assert!(current_str.contains("即时上下文"));
}

#[tokio::test]
async fn contextfs_persists_writes_to_seeded_context_files() {
    let plugin = ContextFsPlugin::new();

    plugin
        .write(
            "/L1/decisions.md",
            b"- use ContextFS for layered agent context\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write decisions");

    let decisions = plugin
        .read("/L1/decisions.md", 0, 0)
        .await
        .expect("read decisions");
    let decisions_str = String::from_utf8(decisions).expect("utf8");
    assert!(decisions_str.contains("ContextFS"));

    plugin
        .mkdir("/L1/scratch/run-001", 0o755)
        .await
        .expect("mkdir scratch child");
    plugin
        .create("/L1/scratch/run-001/note.md", 0o644)
        .await
        .expect("create scratch note");
    plugin
        .write(
            "/L1/scratch/run-001/note.md",
            b"temporary reasoning".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write scratch note");

    let scratch = plugin
        .read("/L1/scratch/run-001/note.md", 0, 0)
        .await
        .expect("read scratch note");
    assert_eq!(scratch, b"temporary reasoning");
}

#[tokio::test]
async fn contextfs_meta_reflects_auto_compression_capabilities() {
    let plugin = ContextFsPlugin::new();

    let meta_raw = plugin.read("/.meta", 0, 0).await.expect("read .meta");
    let meta: serde_json::Value =
        serde_json::from_slice(&meta_raw).expect(".meta is valid JSON");

    assert_eq!(meta["version"], 2);
    assert_eq!(meta["compression"], "auto-summary");

    let caps = meta["capabilities"].as_array().expect("capabilities is array");
    let cap_strs: Vec<&str> = caps.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        cap_strs.contains(&"auto-compression"),
        "capabilities should include auto-compression"
    );
    assert!(
        cap_strs.contains(&"recent-ops-tracking"),
        "capabilities should include recent-ops-tracking"
    );
    assert!(
        cap_strs.contains(&"summary-companion-files"),
        "capabilities should include summary-companion-files"
    );

    // Verify the threshold is present.
    assert_eq!(
        meta["compression_threshold_bytes"], 4096,
    );
}

#[tokio::test]
async fn contextfs_auto_compression_creates_summary_for_large_l2_file() {
    // Use a small threshold so the test does not need megabytes of data.
    let plugin = ContextFsPlugin::new().with_max_file_size(64);

    // Create and write a file that exceeds 64 bytes.
    plugin
        .create("/L2/bigdoc.md", 0o644)
        .await
        .expect("create bigdoc");

    let content = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
    assert!(
        content.len() > 64,
        "test content must exceed the 64-byte threshold"
    );

    plugin
        .write("/L2/bigdoc.md", content.as_bytes().to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write bigdoc");

    // The companion .summary file should have been created automatically.
    let summary_raw = plugin
        .read("/L2/bigdoc.md.summary", 0, 0)
        .await
        .expect("read summary companion");
    let summary = String::from_utf8(summary_raw).expect("summary is utf8");

    // Summary should contain the first lines and a truncation marker.
    assert!(summary.contains("line 1"), "summary keeps early content");
    assert!(
        summary.contains("[truncated"),
        "summary must include truncation marker, got: {}",
        summary,
    );
    assert!(
        summary.contains(&format!("of {} bytes]", content.len())),
        "summary mentions original size"
    );
}

#[tokio::test]
async fn contextfs_no_summary_for_small_l2_file() {
    let plugin = ContextFsPlugin::new().with_max_file_size(4096);

    plugin
        .create("/L2/small.md", 0o644)
        .await
        .expect("create small");

    plugin
        .write("/L2/small.md", b"tiny content".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write small");

    // No companion .summary should be persisted for a file below the threshold.
    // (Note: `read` on a .summary path generates one on-the-fly, so we use
    // `stat` to verify the file was never physically created.)
    assert!(
        plugin.stat("/L2/small.md.summary").await.is_err(),
        "no .summary should be persisted for small L2 files"
    );
}

#[tokio::test]
async fn contextfs_recent_ops_tracks_l0_and_l1_writes() {
    let plugin = ContextFsPlugin::new();

    // Write to L0.
    plugin
        .write(
            "/L0/current",
            b"new task state".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write L0");

    // Write to L1.
    plugin
        .write(
            "/L1/decisions.md",
            b"decision A".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write L1");

    let ops_raw = plugin
        .read("/L0/recent_ops", 0, 0)
        .await
        .expect("read recent_ops");
    let ops: serde_json::Value =
        serde_json::from_slice(&ops_raw).expect("recent_ops is valid JSON");

    let arr = ops.as_array().expect("recent_ops is an array");
    assert!(
        arr.len() >= 2,
        "should have at least 2 tracked operations, got {}",
        arr.len()
    );

    let paths: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get("path").and_then(|p| p.as_str()))
        .collect();
    assert!(
        paths.contains(&"/L0/current"),
        "recent_ops should contain /L0/current, got {:?}",
        paths
    );
    assert!(
        paths.contains(&"/L1/decisions.md"),
        "recent_ops should contain /L1/decisions.md, got {:?}",
        paths
    );
}

#[tokio::test]
async fn contextfs_recent_ops_does_not_track_l2_writes() {
    let plugin = ContextFsPlugin::new();

    plugin
        .create("/L2/testdoc.md", 0o644)
        .await
        .expect("create testdoc");

    plugin
        .write(
            "/L2/testdoc.md",
            b"some L2 content".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write L2");

    let ops_raw = plugin
        .read("/L0/recent_ops", 0, 0)
        .await
        .expect("read recent_ops");
    let ops: serde_json::Value =
        serde_json::from_slice(&ops_raw).expect("recent_ops is valid JSON");

    let arr = ops.as_array().expect("recent_ops is an array");
    let paths: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get("path").and_then(|p| p.as_str()))
        .collect();

    assert!(
        !paths.contains(&"/L2/testdoc.md"),
        "L2 writes should NOT be tracked in recent_ops, got {:?}",
        paths
    );
}

#[tokio::test]
async fn contextfs_recent_ops_respects_max_limit() {
    // Use a very small max to test the sliding window.
    let plugin = ContextFsPlugin::new().with_max_recent_ops(3);

    // Seed initialization writes to L0/recent_ops and L1/session_id already,
    // but we reset by writing directly. We'll do 5 writes and verify only 3
    // remain.

    // First, reset recent_ops.
    plugin
        .write("/L0/recent_ops", b"[]\n".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("reset recent_ops");

    // Perform 5 L0 writes.
    for i in 0..5u8 {
        let path = format!("/L0/file{}", i);
        plugin.create(&path, 0o644).await.expect("create file");
        plugin
            .write(&path, vec![i], 0, WriteFlags::TRUNCATE)
            .await
            .expect("write file");
    }

    let ops_raw = plugin
        .read("/L0/recent_ops", 0, 0)
        .await
        .expect("read recent_ops");
    let ops: serde_json::Value =
        serde_json::from_slice(&ops_raw).expect("recent_ops is valid JSON");

    let arr = ops.as_array().expect("recent_ops is an array");
    assert_eq!(
        arr.len(),
        3,
        "recent_ops should be capped at max_recent_ops (3), got {}",
        arr.len()
    );

    // The last 3 entries should be for file2, file3, file4.
    let paths: Vec<&str> = arr
        .iter()
        .filter_map(|v| v.get("path").and_then(|p| p.as_str()))
        .collect();
    assert_eq!(
        paths,
        vec!["/L0/file2", "/L0/file3", "/L0/file4"],
        "should keep the most recent entries"
    );
}

#[tokio::test]
async fn contextfs_read_summary_fallback_generates_on_demand() {
    let plugin = ContextFsPlugin::new().with_max_file_size(4096);

    // Create an L2 file below the auto-compression threshold so no .summary
    // is written automatically.
    plugin
        .create("/L2/ondemand.md", 0o644)
        .await
        .expect("create ondemand");

    plugin
        .write(
            "/L2/ondemand.md",
            b"line A\nline B\nline C\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write ondemand");

    // The `.summary` companion should NOT exist as a real file.
    assert!(
        plugin.stat("/L2/ondemand.md.summary").await.is_err(),
        "no .summary file should have been created"
    );

    // But reading it via the `read` method should generate it on the fly.
    let summary_raw = plugin
        .read("/L2/ondemand.md.summary", 0, 0)
        .await
        .expect("read on-demand summary");
    let summary = String::from_utf8(summary_raw).expect("summary is utf8");
    assert!(
        summary.contains("line A"),
        "on-demand summary should contain file content"
    );
    assert!(
        summary.contains("[truncated"),
        "on-demand summary should include truncation marker"
    );
}

#[tokio::test]
async fn contextfs_config_params_reflect_new_fields() {
    let plugin = ContextFsPlugin::new();
    let params = plugin.get_config_params();

    let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
    assert!(
        names.contains(&"max_file_size"),
        "config params should include max_file_size"
    );
    assert!(
        names.contains(&"max_recent_ops"),
        "config params should include max_recent_ops"
    );
}

// ---------------------------------------------------------------------------
// Token estimation, session lifecycle, and budget status tests
// ---------------------------------------------------------------------------

use evif_plugins::{BudgetLevel};

#[tokio::test]
async fn contextfs_estimate_tokens_counts_l0_l1_l2() {
    let plugin = ContextFsPlugin::new();

    // Write known content to L0.
    let l0_content = "A".repeat(400); // 400 bytes => 100 tokens
    plugin
        .create("/L0/test_tokens", 0o644)
        .await
        .expect("create L0 test file");
    plugin
        .write(
            "/L0/test_tokens",
            l0_content.as_bytes().to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write L0 test file");

    // Write known content to L1.
    let l1_content = "B".repeat(800); // 800 bytes => 200 tokens
    plugin
        .create("/L1/test_l1", 0o644)
        .await
        .expect("create L1 test file");
    plugin
        .write(
            "/L1/test_l1",
            l1_content.as_bytes().to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write L1 test file");

    // Write known content to L2.
    let l2_content = "C".repeat(1200); // 1200 bytes => 300 tokens
    plugin
        .create("/L2/test_l2", 0o644)
        .await
        .expect("create L2 test file");
    plugin
        .write(
            "/L2/test_l2",
            l2_content.as_bytes().to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write L2 test file");

    let budget = plugin.estimate_tokens().await.expect("estimate tokens");

    // Seed files contribute additional tokens, so we verify that our
    // written content is accounted for (>= check).
    assert!(
        budget.l0_tokens >= 100,
        "L0 tokens should be at least 100, got {}",
        budget.l0_tokens,
    );
    assert!(
        budget.l1_tokens >= 200,
        "L1 tokens should be at least 200, got {}",
        budget.l1_tokens,
    );
    assert!(
        budget.l2_tokens >= 300,
        "L2 tokens should be at least 300, got {}",
        budget.l2_tokens,
    );
    assert_eq!(
        budget.total_tokens,
        budget.l0_tokens + budget.l1_tokens + budget.l2_tokens,
        "total should equal the sum of per-layer tokens",
    );
    assert_eq!(
        budget.budget_limit, 200_000,
        "default budget limit should be 200k",
    );
}

#[tokio::test]
async fn contextfs_session_lifecycle_start_and_end() {
    let plugin = ContextFsPlugin::new();

    // Start a session.
    plugin
        .start_session("test-lifecycle-001")
        .await
        .expect("start session");

    // Verify session_id was updated in L1.
    let session_data = plugin
        .read("/L1/session_id", 0, 0)
        .await
        .expect("read session_id");
    let session_text = String::from_utf8(session_data).expect("utf8");
    assert!(
        session_text.contains("test-lifecycle-001"),
        "session_id should contain the new session id, got: {}",
        session_text,
    );

    // Verify L0/current was updated with session info.
    let current_data = plugin
        .read("/L0/current", 0, 0)
        .await
        .expect("read current");
    let current_text = String::from_utf8(current_data).expect("utf8");
    assert!(
        current_text.contains("test-lifecycle-001"),
        "L0/current should mention session id, got: {}",
        current_text,
    );
    assert!(
        current_text.contains("active"),
        "L0/current should show active status, got: {}",
        current_text,
    );

    // Write a decision to L1 before ending the session.
    plugin
        .write(
            "/L1/decisions.md",
            b"# Session Decisions\n\n- important lifecycle decision\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write decisions");

    // End the session.
    plugin.end_session().await.expect("end session");

    // Verify L1 decisions were archived to L2/history.
    let history_entries = plugin
        .readdir("/L2/history")
        .await
        .expect("read L2/history");
    let history_names: Vec<String> = history_entries.into_iter().map(|e| e.name).collect();

    // Should find a file starting with "decisions_" and one starting with "session_id_".
    let has_decisions_archive = history_names
        .iter()
        .any(|n| n.starts_with("decisions_") && n.ends_with(".md"));
    assert!(
        has_decisions_archive,
        "L2/history should contain an archived decisions file, got: {:?}",
        history_names,
    );

    // Verify the archived content contains the decision.
    let archive_name = history_names
        .iter()
        .find(|n| n.starts_with("decisions_") && n.ends_with(".md"))
        .expect("find decisions archive");
    let archive_path = format!("/L2/history/{}", archive_name);
    let archive_data = plugin
        .read(&archive_path, 0, 0)
        .await
        .expect("read archived decisions");
    let archive_text = String::from_utf8(archive_data).expect("utf8");
    assert!(
        archive_text.contains("important lifecycle decision"),
        "archived decisions should contain the written decision, got: {}",
        archive_text,
    );

    // Verify L0 was reset to idle.
    let current_after = plugin
        .read("/L0/current", 0, 0)
        .await
        .expect("read current after end");
    let current_after_text = String::from_utf8(current_after).expect("utf8");
    assert!(
        current_after_text.contains("idle"),
        "L0/current should show idle status after session end, got: {}",
        current_after_text,
    );
}

#[tokio::test]
async fn contextfs_budget_status_tracks_usage() {
    // Use a very small budget so we can observe level transitions.
    let plugin = ContextFsPlugin::new().with_budget_limit(1000);

    // Initial check: with only seed files, usage should be low (Ok level).
    let status_initial = plugin.check_budget().await.expect("check budget initial");
    assert_eq!(
        status_initial.budget_limit, 1000,
        "budget limit should be 1000",
    );
    // Seed files might push usage above zero, but it should be Ok for small seeds.
    // We just verify the level is computed.
    assert!(
        status_initial.usage_percent >= 0.0,
        "usage_percent should be non-negative, got {}",
        status_initial.usage_percent,
    );

    // Write enough data to push past the budget (4000 bytes = 1000 tokens,
    // which is 100% of a 1000-token budget).
    let big_content = "X".repeat(5000); // 5000 bytes => 1250 tokens
    plugin
        .create("/L0/bigfile", 0o644)
        .await
        .expect("create bigfile");
    plugin
        .write(
            "/L0/bigfile",
            big_content.as_bytes().to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write bigfile");

    // The budget_status file should have been updated automatically after the write.
    let budget_file = plugin
        .read("/L0/budget_status", 0, 0)
        .await
        .expect("read budget_status");
    let budget_json: serde_json::Value =
        serde_json::from_slice(&budget_file).expect("budget_status is valid JSON");

    // With 1000-token budget and >1000 tokens used, usage should exceed 80% => Critical.
    assert!(
        budget_json["level"].as_str() == Some("critical") || budget_json["level"].as_str() == Some("warning"),
        "budget level should be warning or critical after heavy usage, got: {:?}",
        budget_json["level"],
    );

    // Also verify via direct check_budget call.
    let status_heavy = plugin.check_budget().await.expect("check budget heavy");
    assert!(
        matches!(status_heavy.level, BudgetLevel::Critical | BudgetLevel::Warning),
        "level should be Critical or Warning with heavy usage, got {:?} ({}%)",
        status_heavy.level,
        status_heavy.usage_percent,
    );

    // Verify used_tokens is greater than zero.
    assert!(
        status_heavy.used_tokens > 0,
        "used_tokens should be positive, got {}",
        status_heavy.used_tokens,
    );
}

// ---------------------------------------------------------------------------
// SQLite persistence tests (require the `sqlfs` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlfs")]
mod sqlite_persistence_tests {
    use evif_core::{EvifPlugin, WriteFlags};
    use evif_plugins::ContextFsPlugin;
    use tempfile::TempDir;

    #[tokio::test]
    async fn contextfs_sqlite_persistence_saves_and_restores_l0_files() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("context.db");
        let db_str = db_path.to_str().expect("valid utf8 path");

        let plugin = ContextFsPlugin::new_with_persistence(db_str);

        // Write to L0.
        plugin
            .write(
                "/L0/current",
                b"persisted task state".to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write L0 current");

        // Verify we can read it back from the in-memory fs.
        let data = plugin
            .read("/L0/current", 0, 0)
            .await
            .expect("read L0 current");
        assert_eq!(data, b"persisted task state");

        // Create a new instance with the same DB path to verify persistence.
        let plugin2 = ContextFsPlugin::new_with_persistence(db_str);
        let restored = plugin2
            .read("/L0/current", 0, 0)
            .await
            .expect("read restored L0 current");
        assert_eq!(
            restored, b"persisted task state",
            "L0 data should survive across instances via SQLite"
        );
    }

    #[tokio::test]
    async fn contextfs_sqlite_persistence_restores_on_new_instance() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("context2.db");
        let db_str = db_path.to_str().expect("valid utf8 path");

        // Instance 1: write L0 and L1 data.
        {
            let plugin = ContextFsPlugin::new_with_persistence(db_str);
            plugin
                .write(
                    "/L0/current",
                    b"cross-session state".to_vec(),
                    0,
                    WriteFlags::TRUNCATE,
                )
                .await
                .expect("write L0");

            plugin
                .write(
                    "/L1/decisions.md",
                    b"decision: use sqlite persistence".to_vec(),
                    0,
                    WriteFlags::TRUNCATE,
                )
                .await
                .expect("write L1 decisions");
        }

        // Instance 2: create a new plugin pointing at the same database.
        let plugin2 = ContextFsPlugin::new_with_persistence(db_str);

        let l0_data = plugin2
            .read("/L0/current", 0, 0)
            .await
            .expect("read restored L0");
        assert_eq!(
            l0_data, b"cross-session state",
            "L0 current should be restored from SQLite"
        );

        let l1_data = plugin2
            .read("/L1/decisions.md", 0, 0)
            .await
            .expect("read restored L1");
        assert!(
            l1_data.starts_with(b"decision: use sqlite persistence"),
            "L1 decisions should be restored from SQLite, got: {:?}",
            String::from_utf8_lossy(&l1_data),
        );
    }

    #[tokio::test]
    async fn contextfs_meta_shows_sqlite_persistence() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("context3.db");
        let db_str = db_path.to_str().expect("valid utf8 path");

        let plugin = ContextFsPlugin::new_with_persistence(db_str);

        let meta_raw = plugin.read("/.meta", 0, 0).await.expect("read .meta");
        let meta: serde_json::Value =
            serde_json::from_slice(&meta_raw).expect(".meta is valid JSON");

        assert_eq!(
            meta["persistence"], "sqlite",
            ".meta should report persistence as sqlite"
        );

        // Verify config params include persistence_path.
        let params = plugin.get_config_params();
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"persistence_path"),
            "config params should include persistence_path when persistence is active"
        );
    }
}

/// Phase 12.1: LLM Summarization Tests
/// 对标 OpenViking L0CO: 自动生成 .abstract 摘要文件

#[tokio::test]
async fn contextfs_llm_abstract_generated_for_large_l2_file() {
    // Without OPENAI_API_KEY, falls back to truncation-based abstract.
    // With API key, would generate LLM summary (tested separately).
    let plugin = ContextFsPlugin::new().with_max_file_size(64);

    // Create a large L2 file that triggers auto-compression.
    plugin
        .create("/L2/large_doc.md", 0o644)
        .await
        .expect("create large doc");

    let content = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
    assert!(content.len() > 64, "test content must exceed threshold");

    plugin
        .write("/L2/large_doc.md", content.as_bytes().to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write large doc");

    // Give the background LLM task a moment to complete.
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // .abstract companion should exist (LLM or fallback).
    let abstract_raw = plugin
        .read("/L2/large_doc.md.abstract", 0, 0)
        .await
        .expect("read .abstract companion");
    let abstract_text = String::from_utf8(abstract_raw).expect("abstract is utf8");

    // Fallback: should contain first few lines.
    assert!(
        abstract_text.contains("line 1") || abstract_text.contains("摘要"),
        ".abstract should contain content or fallback marker, got: {}",
        abstract_text
    );
    assert!(
        abstract_text.len() < content.len(),
        ".abstract should be shorter than original (got {} vs {})",
        abstract_text.len(),
        content.len()
    );
}

#[tokio::test]
async fn contextfs_llm_abstract_not_generated_for_small_file() {
    // Small files should NOT generate .abstract.
    let plugin = ContextFsPlugin::new().with_max_file_size(4096);

    plugin
        .create("/L2/small_llm.md", 0o644)
        .await
        .expect("create small file");

    plugin
        .write("/L2/small_llm.md", b"tiny".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write small file");

    // Small file: .abstract should NOT be created.
    let err = plugin.stat("/L2/small_llm.md.abstract").await;
    assert!(
        err.is_err(),
        "small file should NOT generate .abstract, but stat succeeded"
    );
}
