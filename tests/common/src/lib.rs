// EVIF Test Helpers Library
// Common utilities for CLI and API testing

pub mod services;

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::time::Duration;

/// Global server process handle
static SERVER_PROCESS: OnceLock<std::sync::Mutex<Option<std::process::Child>>> = OnceLock::new();

/// Get the workspace root directory
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Run an EVIF CLI command and return the output
pub fn run_evif_cli(args: &[&str]) -> std::io::Result<Output> {
    Command::new("cargo")
        .args(&["run", "-p", "evif-cli", "--bin", "evif", "--"])
        .args(args)
        .current_dir(workspace_root())
        .output()
}

/// Run an EVIF CLI command with timeout
pub fn run_evif_cli_timeout(args: &[&str], timeout_secs: u64) -> std::io::Result<Output> {
    let output = Command::new("cargo")
        .args(&["run", "-p", "evif-cli", "--bin", "evif", "--"])
        .args(args)
        .current_dir(workspace_root())
        .output()?;

    Ok(output)
}

/// Parse CLI output into lines
pub fn parse_output(output: &Output) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|s| s.to_string()).collect()
}

/// Check if CLI command succeeded
pub fn cli_success(output: &Output) -> bool {
    output.status.success()
}

/// Get CLI exit code
pub fn cli_exit_code(output: &Output) -> Option<i32> {
    output.status.code()
}

/// Create a temporary directory for testing
pub fn create_temp_dir(prefix: &str) -> std::io::Result<tempfile::TempDir> {
    Ok(tempfile::Builder::new().prefix(prefix).tempdir()?)
}

/// Cleanup function for test environment
pub fn cleanup_test_files(paths: Vec<PathBuf>) {
    for path in paths {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else if path.is_file() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Build the EVIF CLI if not already built
pub fn build_cli() -> std::io::Result<Output> {
    Command::new("cargo")
        .args(&["build", "-p", "evif-cli"])
        .current_dir(workspace_root())
        .output()
}

/// Get the CLI binary path
pub fn cli_bin_path() -> PathBuf {
    workspace_root().join("target/debug/evif")
}

/// Start the REST server for testing
pub fn start_test_server(port: u16) -> Result<(), String> {
    // Check if port is available
    let port_available = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok();
    if !port_available {
        return Err(format!("Port {} is not available", port));
    }

    // Spawn the server
    let child = Command::new("cargo")
        .args(&["run", "-p", "evif-rest"])
        .current_dir(workspace_root())
        .env("EVIF_PORT", &port.to_string())
        .spawn()
        .map_err(|e| format!("Failed to spawn server: {}", e))?;

    // Store the process handle
    let _ = SERVER_PROCESS.get_or_init(|| std::sync::Mutex::new(Some(child)));

    // Wait for server to be ready
    let max_attempts = 30;
    for attempt in 0..max_attempts {
        if let Ok(response) = reqwest::blocking::get(format!("http://localhost:{}/health", port)) {
            if response.status().is_success() {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err("Server failed to start within timeout".to_string())
}

/// Stop the test server
pub fn stop_test_server() {
    if let Some(mutex) = SERVER_PROCESS.get() {
        if let Ok(mut guard) = mutex.lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.kill();
                let _ = child.wait();
            }
            *guard = None;
        }
    }
}

/// Make HTTP request to REST API
pub fn api_get(port: u16, path: &str) -> Result<String, String> {
    let url = format!("http://localhost:{}{}", port, path);
    reqwest::blocking::get(&url)
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())
}

/// Make POST request to REST API
pub fn api_post(port: u16, path: &str, body: &str) -> Result<String, String> {
    let url = format!("http://localhost:{}{}", port, path);
    let client = reqwest::blocking::Client::new();
    client
        .post(&url)
        .body(body.to_string())
        .header("Content-Type", "application/json")
        .send()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())
}

/// Make DELETE request to REST API
pub fn api_delete(port: u16, path: &str) -> Result<String, String> {
    let url = format!("http://localhost:{}{}", port, path);
    let client = reqwest::blocking::Client::new();
    client
        .delete(&url)
        .send()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())
}

/// Wait for server to be healthy
pub fn wait_for_server(port: u16, max_secs: u64) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < max_secs {
        if std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_err() {
            // Port is in use, server might be running
            if let Ok(response) =
                reqwest::blocking::get(format!("http://localhost:{}/health", port))
            {
                if response.status().is_success() {
                    return true;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

/// Check if server is running on port
pub fn is_server_running(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
}

/// Get test server port (default or from environment)
pub fn get_test_port() -> u16 {
    std::env::var("EVIF_TEST_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081)
}

/// Test data helpers
pub mod test_data {
    /// Sample text content for file tests
    pub const SAMPLE_TEXT: &str = "Hello, EVIF World!\nThis is a test file.\nLine 3\n";

    /// Sample binary content
    pub const SAMPLE_BINARY: &[u8] = &[0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];

    /// JSON test data
    pub const SAMPLE_JSON: &str = r#"{"name": "test", "value": 42}"#;

    /// Large text content for performance tests
    pub fn large_text(size: usize) -> String {
        "x".repeat(size)
    }
}

/// Assert helpers
#[macro_export]
macro_rules! assert_cli_success {
    ($output:expr) => {
        assert!(
            $output.status.success(),
            "CLI command failed with exit code: {:?}\nstdout: {}\nstderr: {}",
            $output.status.code(),
            String::from_utf8_lossy(&$output.stdout),
            String::from_utf8_lossy(&$output.stderr)
        )
    };
}

#[macro_export]
macro_rules! assert_cli_failure {
    ($output:expr) => {
        assert!(
            !$output.status.success(),
            "CLI command should have failed but succeeded"
        )
    };
}

#[macro_export]
macro_rules! assert_output_contains {
    ($output:expr, $text:expr) => {
        let stdout = String::from_utf8_lossy(&$output.stdout);
        assert!(
            stdout.contains($text),
            "Output should contain '{}', but got: {}",
            $text,
            stdout
        )
    };
}

#[macro_export]
macro_rules! assert_api_success {
    ($result:expr) => {
        assert!($result.is_ok(), "API request failed: {:?}", $result.err())
    };
}

#[macro_export]
macro_rules! assert_api_contains {
    ($result:expr, $text:expr) => {
        let content = $result.unwrap();
        assert!(
            content.contains($text),
            "API response should contain '{}', but got: {}",
            $text,
            content
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_root() {
        let root = workspace_root();
        assert!(root.exists());
        assert!(root.join("Cargo.toml").exists());
    }

    #[test]
    fn test_test_data() {
        assert!(!test_data::SAMPLE_TEXT.is_empty());
        assert_eq!(test_data::SAMPLE_BINARY.len(), 6);
    }
}
