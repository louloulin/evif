// Unit tests for EncryptedFS plugin
//
// Tests transparent encryption with AES-256-GCM

use evif_plugins::{EncryptedFsPlugin, EncryptedConfig, MemFsPlugin};
use evif_core::EvifPlugin;
use std::sync::Arc;
use tokio;

#[tokio::test]
async fn test_encrypted_config_default() {
    let config = EncryptedConfig::default();
    assert_eq!(config.argon2_memory_kb, 65536);
    assert_eq!(config.argon2_iterations, 3);
    assert_eq!(config.argon2_parallelism, 4);
}

#[tokio::test]
async fn test_encryptedfs_readme() {
    let backend = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    let config = EncryptedConfig {
        master_password: String::from("test_password"),
        ..Default::default()
    };

    let plugin = Arc::new(EncryptedFsPlugin::new(backend, config).unwrap());

    let readme = plugin.get_readme();
    assert!(readme.contains("AES-256-GCM"));
    assert!(readme.contains("Argon2id"));
    assert!(readme.contains("Transparent encryption"));
}

#[tokio::test]
async fn test_encryptedfs_config_params() {
    let backend = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    let config = EncryptedConfig {
        master_password: String::from("test_password"),
        ..Default::default()
    };

    let plugin = Arc::new(EncryptedFsPlugin::new(backend, config).unwrap());

    let params = plugin.get_config_params();

    // Should have at least 4 config params
    assert!(params.len() >= 4);

    // Check for required master_password param
    let password_param = params.iter().find(|p| p.name == "master_password");
    assert!(password_param.is_some());
    assert!(password_param.unwrap().required);
}
