// Service Management Utilities for Testing
// Manages REST server lifecycle for integration tests

use std::net::TcpListener;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use once_cell::sync::Lazy;

/// Server process handle
struct ServerHandle {
    process: Child,
    port: u16,
}

/// Global server process registry
static SERVER_REGISTRY: Lazy<Mutex<Vec<ServerHandle>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Available port finder
pub fn find_available_port() -> Option<u16> {
    (8000..9000).find(|port| TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok())
}

/// Check if port is available
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// Start EVIF REST server
pub fn start_evif_rest(port: u16) -> Result<(), String> {
    if !is_port_available(port) {
        return Err(format!("Port {} is not available", port));
    }

    let child = Command::new("cargo")
        .args(&["run", "-p", "evif-rest", "--"])
        .current_dir(crate::workspace_root())
        .env("EVIF_PORT", &port.to_string())
        .env("EVIF_HOST", "127.0.0.1")
        .spawn()
        .map_err(|e| format!("Failed to spawn server: {}", e))?;

    let mut handle = ServerHandle { process: child, port };

    // Wait for server to become ready
    if !wait_for_health_check(port, 30) {
        // Kill the process if it didn't start
        let _ = handle.process.kill();
        let _ = handle.process.wait();
        return Err(format!("Server failed to start on port {}", port));
    }

    // Register the server
    let mut registry = SERVER_REGISTRY.lock().unwrap();
    registry.push(handle);

    Ok(())
}

/// Stop all running servers
pub fn stop_all_servers() {
    let mut registry = SERVER_REGISTRY.lock().unwrap();
    while let Some(mut handle) = registry.pop() {
        let _ = handle.process.kill();
        let _ = handle.process.wait();
    }
}

/// Stop specific server by port
pub fn stop_server(port: u16) -> bool {
    let mut registry = SERVER_REGISTRY.lock().unwrap();
    let pos = registry.iter().position(|h| h.port == port);

    if let Some(idx) = pos {
        let mut handle = registry.remove(idx);
        let _ = handle.process.kill();
        let _ = handle.process.wait();
        true
    } else {
        false
    }
}

/// Wait for server health check
pub fn wait_for_health_check(port: u16, max_secs: u64) -> bool {
    let start = std::time::Instant::now();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    while start.elapsed().as_secs() < max_secs {
        if let Ok(response) = client.get(format!("http://localhost:{}/health", port)).send() {
            if response.status().is_success() {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    false
}

/// Server health check
pub fn check_server_health(port: u16) -> bool {
    if let Ok(response) = reqwest::blocking::get(format!("http://localhost:{}/health", port)) {
        response.status().is_success()
    } else {
        false
    }
}

/// Get server PID if running
pub fn get_server_pid(port: u16) -> Option<u32> {
    let registry = SERVER_REGISTRY.lock().unwrap();
    registry.iter().find(|h| h.port == port).map(|h| h.process.id())
}

/// Cleanup on test suite completion
impl Drop for ServerHandle {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_available_port() {
        let port = find_available_port();
        assert!(port.is_some());
        assert!(port.unwrap() >= 8000);
    }

    #[test]
    fn test_port_availability() {
        let port = find_available_port().unwrap();
        assert!(is_port_available(port));

        // Bind port temporarily
        let _listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
        assert!(!is_port_available(port));
    }
}
