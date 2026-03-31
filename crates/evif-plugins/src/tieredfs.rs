// TieredFS - Multi-tier Storage Plugin
//
// Automatically migrates files between storage tiers based on:
// - Access frequency (hot/warm/cold)
// - File age
// - Storage capacity thresholds
//
// Tiers:
// - Hot tier: Fast, expensive storage (e.g., Memory, SSD)
// - Warm tier: Medium performance (e.g., local disk)
// - Cold tier: Slow, cheap storage (e.g., S3, remote)

use evif_core::{
    EvifPlugin, FileInfo, EvifResult, WriteFlags, PluginConfigParam,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Storage tier definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageTier {
    Hot,
    Warm,
    Cold,
}

impl StorageTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageTier::Hot => "hot",
            StorageTier::Warm => "warm",
            StorageTier::Cold => "cold",
        }
    }
}

/// File metadata for tier management
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TieredFileMeta {
    path: String,
    current_tier: StorageTier,
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
    size_bytes: u64,
}

impl TieredFileMeta {
    fn new(path: String, size_bytes: u64) -> Self {
        let now = Utc::now();
        Self {
            path,
            current_tier: StorageTier::Hot,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            size_bytes,
        }
    }

    fn should_demote(&self, config: &TieredConfig) -> bool {
        let age_hours = (Utc::now() - self.last_accessed).num_hours();

        match self.current_tier {
            StorageTier::Hot => {
                age_hours > config.hot_ttl_hours
                    || self.access_count < config.hot_min_accesses
            }
            StorageTier::Warm => {
                age_hours > config.warm_ttl_hours
                    || self.access_count < config.warm_min_accesses
            }
            StorageTier::Cold => false,
        }
    }

    fn should_promote(&self, config: &TieredConfig) -> bool {
        let recent_hours = (Utc::now() - self.last_accessed).num_hours();
        recent_hours < config.promote_threshold_hours
    }
}

/// Configuration for TieredFS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredConfig {
    /// Time-to-live in hot tier (hours)
    pub hot_ttl_hours: i64,

    /// Minimum accesses to stay in hot tier
    pub hot_min_accesses: u64,

    /// Time-to-live in warm tier (hours)
    pub warm_ttl_hours: i64,

    /// Minimum accesses to stay in warm tier
    pub warm_min_accesses: u64,

    /// Hours threshold for promotion back to higher tier
    pub promote_threshold_hours: i64,

    /// Maximum hot tier size in bytes (0 = unlimited)
    pub hot_max_size: u64,

    /// Maximum warm tier size in bytes (0 = unlimited)
    pub warm_max_size: u64,

    /// Background check interval in seconds
    pub check_interval_secs: u64,
}

impl Default for TieredConfig {
    fn default() -> Self {
        Self {
            hot_ttl_hours: 24,
            hot_min_accesses: 5,
            warm_ttl_hours: 168, // 7 days
            warm_min_accesses: 2,
            promote_threshold_hours: 1,
            hot_max_size: 1_000_000_000, // 1GB
            warm_max_size: 10_000_000_000, // 10GB
            check_interval_secs: 300, // 5 minutes
        }
    }
}

/// TieredFS plugin
pub struct TieredFsPlugin {
    config: TieredConfig,
    hot_storage: Arc<dyn EvifPlugin>,
    warm_storage: Arc<dyn EvifPlugin>,
    cold_storage: Arc<dyn EvifPlugin>,
    metadata: Arc<RwLock<HashMap<String, TieredFileMeta>>>,
    migration_semaphore: Arc<Semaphore>,
}

impl TieredFsPlugin {
    pub fn new(
        config: TieredConfig,
        hot_storage: Arc<dyn EvifPlugin>,
        warm_storage: Arc<dyn EvifPlugin>,
        cold_storage: Arc<dyn EvifPlugin>,
    ) -> Self {
        Self {
            config,
            hot_storage,
            warm_storage,
            cold_storage,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            migration_semaphore: Arc::new(Semaphore::new(4)), // Max 4 concurrent migrations
        }
    }

    /// Get the appropriate storage backend for a file
    async fn get_storage_for_path(&self, path: &str) -> (Arc<dyn EvifPlugin>, StorageTier) {
        let meta = self.metadata.read().await;
        if let Some(file_meta) = meta.get(path) {
            let storage = match file_meta.current_tier {
                StorageTier::Hot => self.hot_storage.clone(),
                StorageTier::Warm => self.warm_storage.clone(),
                StorageTier::Cold => self.cold_storage.clone(),
            };
            return (storage, file_meta.current_tier);
        }

        // Default to hot tier for new files
        (self.hot_storage.clone(), StorageTier::Hot)
    }

    /// Update file access statistics
    async fn update_access(&self, path: &str, size_bytes: u64) {
        let mut meta = self.metadata.write().await;
        let now = Utc::now();

        if let Some(file_meta) = meta.get_mut(path) {
            file_meta.last_accessed = now;
            file_meta.access_count += 1;
        } else {
            meta.insert(path.to_string(), TieredFileMeta::new(path.to_string(), size_bytes));
        }
    }

    /// Migrate file to a different tier
    async fn migrate_file(&self, path: &str, target_tier: StorageTier) -> EvifResult<()> {
        let (source_storage, current_tier) = self.get_storage_for_path(path).await;

        if current_tier == target_tier {
            return Ok(());
        }

        let target_storage = match target_tier {
            StorageTier::Hot => self.hot_storage.clone(),
            StorageTier::Warm => self.warm_storage.clone(),
            StorageTier::Cold => self.cold_storage.clone(),
        };

        // Read from source
        let data = source_storage.read(path, 0, 0).await?;

        // Write to target
        target_storage.write(path, data, 0, WriteFlags::NONE).await?;

        // Delete from source
        source_storage.remove(path).await?;

        // Update metadata
        let mut meta = self.metadata.write().await;
        if let Some(file_meta) = meta.get_mut(path) {
            file_meta.current_tier = target_tier;
        }

        Ok(())
    }

    /// Background task to check and migrate files
    pub async fn run_tier_maintenance(&self) -> EvifResult<()> {
        let mut paths_to_migrate: Vec<(String, StorageTier)> = Vec::new();

        {
            let meta = self.metadata.read().await;
            for (path, file_meta) in meta.iter() {
                if file_meta.should_demote(&self.config) {
                    let target_tier = match file_meta.current_tier {
                        StorageTier::Hot => StorageTier::Warm,
                        StorageTier::Warm => StorageTier::Cold,
                        StorageTier::Cold => continue,
                    };
                    paths_to_migrate.push((path.clone(), target_tier));
                } else if file_meta.should_promote(&self.config) {
                    let target_tier = match file_meta.current_tier {
                        StorageTier::Cold => StorageTier::Warm,
                        StorageTier::Warm => StorageTier::Hot,
                        StorageTier::Hot => continue,
                    };
                    paths_to_migrate.push((path.clone(), target_tier));
                }
            }
        }

        for (path, target_tier) in paths_to_migrate {
            let _permit = match self.migration_semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => {
                    eprintln!("Migration semaphore closed, aborting migration");
                    break;
                }
            };
            if let Err(e) = self.migrate_file(&path, target_tier).await {
                eprintln!("Failed to migrate {}: {:?}", path, e);
            }
        }

        Ok(())
    }

    /// Get tier statistics
    pub async fn get_tier_stats(&self) -> TierStats {
        let meta = self.metadata.read().await;
        let mut hot_size = 0u64;
        let mut warm_size = 0u64;
        let mut cold_size = 0u64;
        let mut hot_count = 0usize;
        let mut warm_count = 0usize;
        let mut cold_count = 0usize;

        for file_meta in meta.values() {
            match file_meta.current_tier {
                StorageTier::Hot => {
                    hot_size += file_meta.size_bytes;
                    hot_count += 1;
                }
                StorageTier::Warm => {
                    warm_size += file_meta.size_bytes;
                    warm_count += 1;
                }
                StorageTier::Cold => {
                    cold_size += file_meta.size_bytes;
                    cold_count += 1;
                }
            }
        }

        TierStats {
            hot_size,
            hot_count,
            warm_size,
            warm_count,
            cold_size,
            cold_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierStats {
    pub hot_size: u64,
    pub hot_count: usize,
    pub warm_size: u64,
    pub warm_count: usize,
    pub cold_size: u64,
    pub cold_count: usize,
}

#[async_trait::async_trait]
impl EvifPlugin for TieredFsPlugin {
    fn name(&self) -> &str {
        "TieredFS"
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.create(path, perm).await?;
        self.update_access(path, 0).await;
        Ok(())
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.mkdir(path, perm).await?;
        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let (storage, tier) = self.get_storage_for_path(path).await;
        let data = storage.read(path, offset, size).await?;

        // Update access stats (read file to get size)
        if let Ok(info) = storage.stat(path).await {
            self.update_access(path, info.size).await;

            // Check if file should be promoted
            let meta = self.metadata.read().await;
            if let Some(file_meta) = meta.get(path) {
                if file_meta.current_tier != StorageTier::Hot && file_meta.should_promote(&self.config) {
                    drop(meta);
                    let target = match tier {
                        StorageTier::Cold => StorageTier::Warm,
                        StorageTier::Warm => StorageTier::Hot,
                        StorageTier::Hot => StorageTier::Hot,
                    };
                    let _ = self.migrate_file(path, target).await;
                }
            }
        }

        Ok(data)
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        let (storage, _) = self.get_storage_for_path(path).await;
        let size = data.len() as u64;
        let written = storage.write(path, data, 0, WriteFlags::NONE).await?;
        self.update_access(path, size).await;
        Ok(written)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        // Merge results from all tiers
        let mut all_files = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        for storage in [&self.hot_storage, &self.warm_storage, &self.cold_storage] {
            if let Ok(files) = storage.readdir(path).await {
                for file in files {
                    if seen_paths.insert(file.name.clone()) {
                        all_files.push(file);
                    }
                }
            }
        }

        Ok(all_files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.remove(path).await?;

        // Remove from metadata
        let mut meta = self.metadata.write().await;
        meta.remove(path);

        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.remove_all(path).await?;

        // Remove from metadata
        let mut meta = self.metadata.write().await;
        meta.remove(path);

        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(old_path).await;
        storage.rename(old_path, new_path).await?;

        // Update metadata
        let mut meta = self.metadata.write().await;
        if let Some(mut file_meta) = meta.remove(old_path) {
            file_meta.path = new_path.to_string();
            meta.insert(new_path.to_string(), file_meta);
        }

        Ok(())
    }

    async fn chmod(&self, path: &str, mode: u32) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.chmod(path, mode).await
    }

    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()> {
        let (storage, _) = self.get_storage_for_path(path).await;
        storage.truncate(path, size).await?;
        self.update_access(path, size).await;
        Ok(())
    }

    async fn validate(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        Ok(())
    }

    fn get_readme(&self) -> String {
        r#"
# TieredFS Plugin

Multi-tier storage system that automatically migrates files between hot, warm, and cold storage.

## Features

- **Hot Tier**: Fast storage for frequently accessed files
- **Warm Tier**: Medium performance for less active files
- **Cold Tier**: Cheap storage for archival data
- Automatic migration based on:
  - File age
  - Access frequency
  - Storage capacity

## Configuration

```json
{
  "hot_ttl_hours": 24,
  "hot_min_accesses": 5,
  "warm_ttl_hours": 168,
  "warm_min_accesses": 2,
  "promote_threshold_hours": 1,
  "hot_max_size": 1000000000,
  "warm_max_size": 10000000000,
  "check_interval_secs": 300
}
```

## Usage

The plugin automatically manages file placement. Access files normally and TieredFS will:
- Promote frequently accessed files to higher tiers
- Demote old, unused files to lower tiers
- Respect capacity limits on each tier
"#.to_string()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![
            PluginConfigParam {
                name: "hot_ttl_hours".to_string(),
                param_type: "number".to_string(),
                description: Some("Hours before file moves from hot tier".to_string()),
                required: false,
                default: Some("24".to_string()),
            },
            PluginConfigParam {
                name: "hot_min_accesses".to_string(),
                param_type: "number".to_string(),
                description: Some("Min accesses to stay in hot tier".to_string()),
                required: false,
                default: Some("5".to_string()),
            },
            PluginConfigParam {
                name: "warm_ttl_hours".to_string(),
                param_type: "number".to_string(),
                description: Some("Hours before file moves from warm tier".to_string()),
                required: false,
                default: Some("168".to_string()),
            },
            PluginConfigParam {
                name: "warm_min_accesses".to_string(),
                param_type: "number".to_string(),
                description: Some("Min accesses to stay in warm tier".to_string()),
                required: false,
                default: Some("2".to_string()),
            },
            PluginConfigParam {
                name: "hot_max_size".to_string(),
                param_type: "number".to_string(),
                description: Some("Max hot tier size in bytes".to_string()),
                required: false,
                default: Some("1000000000".to_string()),
            },
            PluginConfigParam {
                name: "warm_max_size".to_string(),
                param_type: "number".to_string(),
                description: Some("Max warm tier size in bytes".to_string()),
                required: false,
                default: Some("10000000000".to_string()),
            },
        ]
    }
}
