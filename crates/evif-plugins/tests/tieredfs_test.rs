// Unit tests for TieredFS plugin
//
// Tests multi-tier storage with automatic migration logic

use evif_core::EvifPlugin;
use evif_plugins::{MemFsPlugin, StorageTier, TieredConfig, TieredFsPlugin};
use std::sync::Arc;

#[tokio::test]
async fn test_tiered_config_default() {
    let config = TieredConfig::default();
    ();
    assert_eq!(config.hot_ttl_hours, 24);
    assert_eq!(config.hot_min_accesses, 5);
}

#[tokio::test]
async fn test_storage_tier_as_str() {
    assert_eq!(StorageTier::Hot.as_str(), "hot");
    assert_eq!(StorageTier::Warm.as_str(), "warm");
    assert_eq!(StorageTier::Cold.as_str(), "cold");
}

#[tokio::test]
async fn test_tieredfs_creation() {
    let hot = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    let warm = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    let cold = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;

    let config = TieredConfig::default();
    let _plugin = TieredFsPlugin::new(config, hot, warm, cold);
}

#[tokio::test]
async fn test_tieredfs_readme() {
    let hot = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    let warm = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    let cold = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;

    let plugin = TieredFsPlugin::new(TieredConfig::default(), hot, warm, cold);

    let readme = plugin.get_readme();
    assert!(readme.contains("TieredFS Plugin"));
    assert!(readme.contains("Multi-tier storage"));
}
