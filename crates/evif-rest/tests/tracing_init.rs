use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn rest_binary_emits_startup_logs_when_started() {
    let binary = env!("CARGO_BIN_EXE_evif-rest");

    let mut child = Command::new(binary)
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("evif-rest binary should start");

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
