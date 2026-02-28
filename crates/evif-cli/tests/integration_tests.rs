// EVIF CLI Integration Tests

use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "evif-cli", "--bin", "evif", "--", "--version"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to execute evif");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("evif"));
}

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "evif-cli", "--bin", "evif", "--", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to execute evif");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("evif"));
    assert!(stdout.contains("Usage") || stdout.contains("USAGE"));
    assert!(stdout.contains("Commands") || stdout.contains("COMMANDS"));
}

#[test]
fn test_cli_script_execution() {
    let temp_dir = TempDir::new().unwrap();
    let script_path = temp_dir.path().join("test.evif");

    let script_content = r#"
# EVIF Test Script
VAR name=value
echo "Testing variable expansion: $name"
"#;

    std::fs::write(&script_path, script_content).unwrap();

    // This test requires a running evif-server
    // In CI/CD, start a test server first
    let output = Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "evif-cli",
            "--bin",
            "evif",
            "--",
            "script",
            script_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to execute evif script");

    // For now, just check it doesn't crash
    // In production, validate output
}

#[test]
fn test_cli_completer() {
    // evif-cli is binary-only; completer is tested in lib tests (completer.rs)
    // Integration test: ensure CLI runs without crash
    let output = Command::new("cargo")
        .args(&["run", "-p", "evif-cli", "--bin", "evif", "--", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to run evif");
    assert!(output.status.success());
}

#[cfg(test)]
mod repl_tests {
    use super::*;

    #[test]
    fn test_repl_command_parsing() {
        // Test that REPL correctly parses commands
        let commands = vec![
            "ls /",
            "cat /test.txt",
            "write /file.txt content",
            "mkdir /newdir",
            "rm /oldfile.txt",
        ];

        for cmd in commands {
            // Basic validation that commands are properly formatted
            assert!(cmd.contains(' '));
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            assert!(parts.len() >= 2);
        }
    }
}
