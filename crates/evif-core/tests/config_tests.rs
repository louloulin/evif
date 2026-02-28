// EVIF Configuration System Tests

use evif_core::config::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_config_from_toml() {
    let config_content = r#"
[server]
bind_address = "127.0.0.1"
port = 8080
timeout_secs = 30
max_connections = 1000

[plugins]
plugins_dir = "/usr/local/lib/evif/plugins"
plugin_configs = {}
auto_mount = []

[cache]
enabled = true
metadata_ttl_secs = 60
directory_ttl_secs = 30
max_entries = 10000

[logging]
level = "info"
format = "json"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(config_content.as_bytes()).unwrap();

    let config = EvifConfig::from_file(temp_file.path()).unwrap();

    assert_eq!(config.server.bind_address, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.cache.enabled, true);
    assert_eq!(config.cache.metadata_ttl_secs, 60);
    assert_eq!(config.logging.level, "info");
}

#[test]
fn test_config_default() {
    let config = EvifConfig::default();

    assert_eq!(config.server.bind_address, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.cache.enabled, true);
    assert_eq!(config.logging.level, "info");
}

#[test]
fn test_plugin_config() {
    let config_content = r#"
[server]
bind_address = "0.0.0.0"
port = 8080
timeout_secs = 30
max_connections = 1000

[plugins]
plugins_dir = "/usr/local/lib/evif/plugins"
plugin_configs = {}

[[plugins.auto_mount]]
plugin = "memfs"
path = "/mem"

[[plugins.auto_mount]]
plugin = "localfs"
path = "/local"

[cache]
enabled = true
metadata_ttl_secs = 60
directory_ttl_secs = 30
max_entries = 10000

[logging]
level = "info"
format = "json"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(config_content.as_bytes()).unwrap();

    let config = EvifConfig::from_file(temp_file.path()).unwrap();

    assert_eq!(config.plugins.plugins_dir, "/usr/local/lib/evif/plugins");
    assert_eq!(config.plugins.auto_mount.len(), 2);
}

#[test]
fn test_security_config() {
    let config_content = r#"
[server]
bind_address = "0.0.0.0"
port = 8080
timeout_secs = 30
max_connections = 1000

[plugins]
plugins_dir = "/usr/local/lib/evif/plugins"
plugin_configs = {}
auto_mount = []

[cache]
enabled = true
metadata_ttl_secs = 60
directory_ttl_secs = 30
max_entries = 10000

[logging]
level = "info"
format = "json"

[security]
tls_enabled = true
cert_path = "/etc/certs/server.crt"
key_path = "/etc/certs/server.key"
api_keys = ["key1", "key2"]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(config_content.as_bytes()).unwrap();

    let config = EvifConfig::from_file(temp_file.path()).unwrap();

    assert!(config.security.is_some());
    let security = config.security.unwrap();
    assert_eq!(security.tls_enabled, true);
    assert_eq!(security.api_keys.unwrap().len(), 2);
}
