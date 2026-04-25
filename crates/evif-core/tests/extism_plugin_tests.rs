// Extism WASM Plugin Integration Tests
//
// Tests the Extism backend plugin loading and invocation
// Only runs with `extism-backend` feature

#![cfg(feature = "extism-backend")]

use evif_core::extism_plugin::{ExtismPlugin, SecurityConfig, WasmPluginConfig};
use evif_core::EvifPlugin;

/// Get the workspace root directory
fn workspace_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| {
            let parent = std::path::Path::new(&d).parent().unwrap().parent().unwrap();
            parent.to_string_lossy().to_string()
        })
        .unwrap_or_else(|_| ".".to_string())
}

#[test]
fn test_extism_plugin_not_found() {
    let config = WasmPluginConfig::new("/nonexistent/plugin.wasm");
    let result = ExtismPlugin::new(config);
    assert!(result.is_err());
    match result {
        Err(evif_core::EvifError::NotFound(msg)) => {
            assert!(msg.contains("WASM file not found"));
        }
        other => panic!("Expected NotFound error, got: {:?}", other.err()),
    }
}

#[test]
fn test_security_config_default() {
    let config = SecurityConfig::default();
    assert_eq!(config.memory_limit, 64 * 1024 * 1024);
    assert_eq!(config.timeout_ms, 5000);
    assert!(config.allowed_hosts.is_empty());
    assert!(config.allowed_paths.is_empty());
}

#[test]
fn test_security_config_builder() {
    let config = SecurityConfig::new()
        .with_memory_limit(128 * 1024 * 1024)
        .with_timeout_ms(10000)
        .with_allowed_hosts(vec!["api.example.com".to_string()])
        .with_allowed_paths(vec!["/data".to_string()]);

    assert_eq!(config.memory_limit, 128 * 1024 * 1024);
    assert_eq!(config.timeout_ms, 10000);
    assert_eq!(config.allowed_hosts, vec!["api.example.com"]);
    assert_eq!(config.allowed_paths, vec!["/data"]);
}

#[test]
fn test_wasm_plugin_config_default() {
    let config = WasmPluginConfig::default();
    assert_eq!(config.name, "wasm_plugin");
    assert_eq!(config.mount_point, "/");
    assert_eq!(config.security.memory_limit, 64 * 1024 * 1024);
    assert_eq!(config.security.timeout_ms, 5000);
}

#[test]
fn test_wasm_plugin_config_builder() {
    let config = WasmPluginConfig::new("/path/to/plugin.wasm")
        .with_name("test_plugin")
        .with_mount_point("/mnt/test")
        .with_config(serde_json::json!({"key": "value"}));

    assert_eq!(config.wasm_path, "/path/to/plugin.wasm");
    assert_eq!(config.name, "test_plugin");
    assert_eq!(config.mount_point, "/mnt/test");
    assert_eq!(config.config["key"], "value");
}

#[test]
fn test_wasm_plugin_config_with_security() {
    let security = SecurityConfig::new()
        .with_memory_limit(32 * 1024 * 1024)
        .with_timeout_ms(3000)
        .with_allowed_hosts(vec!["localhost".to_string()]);

    let config = WasmPluginConfig::new("/path/to/plugin.wasm")
        .with_name("secure_plugin")
        .with_security(security);

    assert_eq!(config.security.memory_limit, 32 * 1024 * 1024);
    assert_eq!(config.security.timeout_ms, 3000);
    assert_eq!(config.security.allowed_hosts, vec!["localhost"]);
}

#[test]
fn test_extism_plugin_load_greet() {
    let wasm_path = format!("{}/test_scripts/wasm/greet.wasm", workspace_root());

    // Skip test if WASM file doesn't exist
    if !std::path::Path::new(&wasm_path).exists() {
        eprintln!("Skipping test: WASM file not found at {}", wasm_path);
        return;
    }

    let config = WasmPluginConfig::new(&wasm_path)
        .with_name("greet_plugin")
        .with_mount_point("/greet");

    let result = ExtismPlugin::new(config);
    assert!(
        result.is_ok(),
        "Failed to load Extism plugin: {:?}",
        result.err()
    );

    let plugin = result.unwrap();
    // name() comes from EvifPlugin trait
    assert_eq!(plugin.name(), "greet_plugin");
}

#[tokio::test]
async fn test_extism_plugin_call_greet() {
    let wasm_path = format!("{}/test_scripts/wasm/greet.wasm", workspace_root());

    if !std::path::Path::new(&wasm_path).exists() {
        eprintln!("Skipping test: WASM file not found at {}", wasm_path);
        return;
    }

    let config = WasmPluginConfig::new(&wasm_path)
        .with_name("greet_plugin")
        .with_security(
            SecurityConfig::new()
                .with_memory_limit(16 * 1024 * 1024)
                .with_timeout_ms(5000),
        );

    let plugin = ExtismPlugin::new(config).expect("Failed to load plugin");

    // Test the EvifPlugin trait implementation
    assert_eq!(plugin.name(), "greet_plugin");

    // Test get_exports returns the expected functions
    let exports = plugin.get_exports();
    assert!(exports.contains(&"evif_create".to_string()));
    assert!(exports.contains(&"evif_read".to_string()));
    assert!(exports.contains(&"evif_write".to_string()));
}

#[tokio::test]
async fn test_extism_plugin_with_tight_security() {
    let wasm_path = format!("{}/test_scripts/wasm/greet.wasm", workspace_root());

    if !std::path::Path::new(&wasm_path).exists() {
        eprintln!("Skipping test: WASM file not found");
        return;
    }

    // Very tight security: 1MB memory, 1s timeout
    let config = WasmPluginConfig::new(&wasm_path)
        .with_name("tight_security")
        .with_security(
            SecurityConfig::new()
                .with_memory_limit(1024 * 1024)
                .with_timeout_ms(1000),
        );

    let result = ExtismPlugin::new(config);
    assert!(
        result.is_ok(),
        "Should load with tight security: {:?}",
        result.err()
    );
}
