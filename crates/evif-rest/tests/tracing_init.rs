use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn rest_binary_emits_startup_logs_when_started() {
    let binary = env!("CARGO_BIN_EXE_evif-rest");

    let mut command = Command::new(binary);
    command
        .env("RUST_LOG", "info")
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

    thread::sleep(Duration::from_millis(800));

    let _ = child.kill();
    let output = child
        .wait_with_output()
        .expect("should collect binary output");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Loading plugins"),
        "startup logs should be emitted once tracing_subscriber is initialized; stdout was: {stdout}",
    );
}
