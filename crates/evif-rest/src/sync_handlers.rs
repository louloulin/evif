// Phase 17.3: Incremental Sync Protocol
//
// 提供增量同步功能 - delta sync, version tracking, watch events

use crate::{RestError, RestResult};
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Sync status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncStatus {
    pub synced: bool,
    pub last_version: u64,
    pub pending_changes: usize,
    pub tracked_paths: Vec<String>,
}

/// A delta change entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeltaChange {
    /// Virtual path affected
    pub path: String,
    /// "created" | "modified" | "deleted"
    pub op: String,
    /// Version after change
    pub version: u64,
    /// Change timestamp (ISO 8601)
    pub timestamp: String,
}

/// Request to submit delta changes
#[derive(Debug, Deserialize)]
pub struct DeltaRequest {
    /// Base version to sync from
    pub base_version: u64,
    /// Changes since base_version
    pub changes: Vec<DeltaChange>,
}

/// Response from delta sync
#[derive(Debug, Serialize)]
pub struct DeltaResponse {
    pub synced_version: u64,
    pub accepted: usize,
    pub conflicts: Vec<String>,
}

/// Watch event
#[derive(Clone, Debug, Serialize)]
pub struct WatchEvent {
    pub event_type: String,
    pub path: String,
    pub version: u64,
    pub timestamp: String,
}

/// Sync state manager
#[derive(Clone)]
pub struct SyncState {
    inner: Arc<RwLock<SyncInner>>,
    persistence_path: Arc<Option<PathBuf>>,
}

struct SyncInner {
    version: u64,
    pending_changes: Vec<DeltaChange>,
    tracked_paths: HashMap<String, u64>, // path -> last_synced_version
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncSnapshot {
    version: u64,
    pending_changes: Vec<DeltaChange>,
    tracked_paths: HashMap<String, u64>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SyncInner {
                version: 0,
                pending_changes: Vec::new(),
                tracked_paths: HashMap::new(),
            })),
            persistence_path: Arc::new(None),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        match std::env::var("EVIF_REST_SYNC_STATE_PATH") {
            Ok(path) if !path.trim().is_empty() => Self::persistent(path.trim()),
            _ => Ok(Self::new()),
        }
    }

    pub fn persistent(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let snapshot = if path.exists() {
            Self::load_snapshot(&path)?
        } else {
            let snapshot = Self::default_snapshot();
            Self::save_snapshot(&path, &snapshot)?;
            snapshot
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(SyncInner {
                version: snapshot.version,
                pending_changes: snapshot.pending_changes,
                tracked_paths: snapshot.tracked_paths,
            })),
            persistence_path: Arc::new(Some(path)),
        })
    }

    fn default_snapshot() -> SyncSnapshot {
        SyncSnapshot {
            version: 0,
            pending_changes: Vec::new(),
            tracked_paths: HashMap::new(),
        }
    }

    fn load_snapshot(path: &Path) -> Result<SyncSnapshot, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read sync state '{}': {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse sync state '{}': {}", path.display(), e))
    }

    fn save_snapshot(path: &Path, snapshot: &SyncSnapshot) -> Result<(), String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create sync state parent '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
        let content = serde_json::to_string_pretty(snapshot)
            .map_err(|e| format!("Failed to serialize sync state: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write sync state '{}': {}", path.display(), e))
    }

    fn persist_inner(&self, inner: &SyncInner) -> Result<(), String> {
        if let Some(path) = self.persistence_path.as_ref().as_ref() {
            let snapshot = SyncSnapshot {
                version: inner.version,
                pending_changes: inner.pending_changes.clone(),
                tracked_paths: inner.tracked_paths.clone(),
            };
            Self::save_snapshot(path, &snapshot)?;
        }
        Ok(())
    }

    pub fn get_status(&self) -> SyncStatus {
        let inner = self.inner.read();
        SyncStatus {
            synced: true,
            last_version: inner.version,
            pending_changes: inner.pending_changes.len(),
            tracked_paths: inner.tracked_paths.keys().cloned().collect(),
        }
    }

    pub async fn apply_delta(&self, req: DeltaRequest) -> DeltaResponse {
        let mut inner = self.inner.write();
        let mut conflicts = Vec::new();
        let mut accepted = 0;
        let mut applied_changes = Vec::new();

        for change in &req.changes {
            // Check for conflicts
            if let Some(&current_version) = inner.tracked_paths.get(&change.path) {
                // Path is tracked and current version is higher than base_version
                if current_version > change.version && change.version < req.base_version {
                    conflicts.push(change.path.clone());
                    continue;
                }
            }

            // Apply the change
            match change.op.as_str() {
                "created" | "modified" => {
                    inner.tracked_paths.insert(change.path.clone(), change.version);
                    accepted += 1;
                    applied_changes.push(change.clone());
                }
                "deleted" => {
                    inner.tracked_paths.remove(&change.path);
                    accepted += 1;
                    applied_changes.push(change.clone());
                }
                _ => {
                    conflicts.push(format!("unknown operation: {}", change.op));
                }
            }
        }

        // Update global version
        if accepted > 0 {
            let max_change_version = req
                .changes
                .iter()
                .filter(|_| accepted > 0)
                .map(|c| c.version)
                .max()
                .unwrap_or(0);
            inner.version = inner.version.max(max_change_version);
        }

        inner.pending_changes = applied_changes;
        let persist_error = self.persist_inner(&inner).err();
        if let Some(error) = persist_error {
            conflicts.push(format!("failed to persist sync state: {}", error));
        }

        DeltaResponse {
            synced_version: inner.version,
            accepted,
            conflicts,
        }
    }

    pub fn get_version(&self) -> u64 {
        self.inner.read().version
    }
}

/// Sync handlers
pub struct SyncHandlers;

impl SyncHandlers {
    /// GET /api/v1/sync/status - Get sync status
    pub async fn get_status(
        State(state): State<SyncState>,
    ) -> RestResult<impl IntoResponse> {
        Ok(Json(state.get_status()))
    }

    /// POST /api/v1/sync/delta - Submit delta changes
    pub async fn apply_delta(
        State(state): State<SyncState>,
        Json(req): Json<DeltaRequest>,
    ) -> RestResult<impl IntoResponse> {
        if req.changes.is_empty() {
            return Err(RestError::BadRequest(
                "No changes provided".into(),
            ));
        }

        let response = state.apply_delta(req).await;
        Ok(Json(response))
    }

    /// GET /api/v1/sync/version - Get current version
    pub async fn get_version(
        State(state): State<SyncState>,
    ) -> RestResult<impl IntoResponse> {
        let version = state.get_version();
        Ok(Json(serde_json::json!({ "version": version })))
    }

    /// GET /api/v1/sync/:path/version - Get version for specific path
    pub async fn get_path_version(
        State(state): State<SyncState>,
        path: axum::extract::Path<String>,
    ) -> RestResult<impl IntoResponse> {
        let inner = state.inner.read();
        let version = inner.tracked_paths.get(&path.0).copied().unwrap_or(0);
        Ok(Json(serde_json::json!({
            "path": path.0,
            "version": version
        })))
    }
}
