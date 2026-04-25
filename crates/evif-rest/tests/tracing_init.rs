use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn rest_binary_emits_startup_logs_when_started() {
    let binary = env!("CARGO_BIN_EXE_evif-rest");
    let log_dir = tempfile::tempdir().expect("tempdir");

    let mut command = Command::new(binary);
    command
        .env("RUST_LOG", "info")
        .env("EVIF_LOG_DIR", log_dir.path())
        .env("EVIF_REST_PORT", "0")
        .env_remove("EVIF_REST_PRODUCTION_MODE")
        .env_remove("EVIF_REST_MEMORY_BACKEND")
        .env_remove("EVIF_REST_MEMORY_SQLITE_PATH")
        .env_remove("EVIF_REST_TENANT_STATE_PATH")
        .env_remove("EVIF_REST_SYNC_STATE_PATH")
        .env_remove("EVIF_REST_ENCRYPTION_STATE_PATH")
        .env_remove("EVIF_REST_WRITE_API_KEYS")
        .env_remove("EVIF_REST_ADMIN_API_KEYS")
        .env_remove("EVIF_REST_AUTH_AUDIT_LOG")
        .env_remove("EVIF_REST_AUTH_MODE")
        .env_remove("EVIF_CONFIG")
        .env_remove("EVIF_MOUNTS")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("evif-rest binary should start");

    let mut combined = String::new();
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(150));
        combined.clear();
        let log_files = std::fs::read_dir(log_dir.path()).expect("log dir");
        for entry in log_files {
            let entry = entry.expect("dir entry");
            if entry.file_type().expect("file type").is_file() {
                combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
        if combined.contains("Loading plugins") {
            break;
        }
    }

    let _ = child.kill();
    let output = child
        .wait_with_output()
        .expect("should collect binary output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let file_logs = combined.clone();
    let combined = format!("{}{}{}", stdout, stderr, file_logs);

    assert!(
        combined.contains("Loading plugins"),
        "startup logs should be emitted once tracing_subscriber is initialized; stdout was: {stdout}; stderr was: {stderr}; file logs were: {file_logs}",
    );
}
