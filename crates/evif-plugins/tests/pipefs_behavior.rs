use std::sync::Arc;
use std::time::Duration;

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::PipeFsPlugin;
use evif_plugins::queuefs::MemoryQueueBackend;

#[tokio::test]
async fn pipefs_creates_pipes_and_tracks_status_transitions() {
    let plugin = PipeFsPlugin::new();

    plugin.mkdir("/task-001", 0o755).await.expect("mkdir pipe");

    let pending = plugin.read("/task-001/status", 0, 0).await.expect("pending");
    assert_eq!(pending, b"pending");

    plugin
        .write("/task-001/input", b"build report".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write input");
    let running = plugin.read("/task-001/status", 0, 0).await.expect("running");
    assert_eq!(running, b"running");

    plugin
        .write(
            "/task-001/output",
            b"report complete".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write output");
    let completed = plugin
        .read("/task-001/status", 0, 0)
        .await
        .expect("completed");
    assert_eq!(completed, b"completed");
}

#[tokio::test]
async fn pipefs_broadcasts_messages_to_subscribers() {
    let plugin = PipeFsPlugin::new();

    plugin
        .mkdir("/broadcast/subscribers/agent-a", 0o755)
        .await
        .expect("mkdir subscriber");
    plugin
        .write(
            "/broadcast/input",
            b"team sync".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write broadcast");

    let echoed = plugin
        .read("/broadcast/subscribers/agent-a/output", 0, 0)
        .await
        .expect("read subscriber output");
    assert_eq!(echoed, b"team sync");
}

#[tokio::test]
async fn pipefs_cleans_up_expired_pipes_after_timeout() {
    let plugin = PipeFsPlugin::new();

    plugin.mkdir("/task-expire", 0o755).await.expect("mkdir expiring");
    plugin
        .write("/task-expire/timeout", b"1".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("set timeout");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let err = plugin.read("/task-expire/status", 0, 0).await.expect_err("expired");
    assert!(matches!(err, evif_core::EvifError::NotFound(_)));
}

#[tokio::test]
async fn pipefs_with_backend_persists_messages_across_instances() {
    let backend = Arc::new(MemoryQueueBackend::new());

    // --- Instance 1: create pipe and write messages ---
    let plugin1 = PipeFsPlugin::new_with_backend(backend.clone());
    plugin1.mkdir("/persist-task", 0o755).await.expect("mkdir");

    plugin1
        .write("/persist-task/input", b"hello from instance 1".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write input");

    plugin1
        .write("/persist-task/output", b"response from instance 1".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write output");

    // Drop instance 1 entirely -- its in-memory HashMap is gone
    drop(plugin1);

    // --- Instance 2: same backend, fresh in-memory state ---
    let plugin2 = PipeFsPlugin::new_with_backend(backend);

    // The backend queues still exist, so ensure_pipe should succeed and
    // input/output should be readable from the backend.
    let input = plugin2
        .read("/persist-task/input", 0, 0)
        .await
        .expect("read persisted input");
    assert_eq!(input, b"hello from instance 1");

    let output = plugin2
        .read("/persist-task/output", 0, 0)
        .await
        .expect("read persisted output");
    assert_eq!(output, b"response from instance 1");
}
