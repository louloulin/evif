// Phase 17.3: Incremental Sync Protocol
//
// 提供增量同步功能 - delta sync, version tracking, watch events

use crate::{RestError, RestResult};
use axum::{extract::State, response::IntoResponse, Json};
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

/// Request to resolve sync conflicts
#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    /// List of conflict resolutions
    pub resolutions: Vec<SingleConflictResolution>,
}

/// Single conflict resolution
#[derive(Debug, Deserialize)]
pub struct SingleConflictResolution {
    /// Path with the conflict
    pub path: String,
    /// Strategy: "accept_local" | "accept_remote" | "last_write_wins"
    pub strategy: String,
    /// Remote version (for "accept_remote" and "last_write_wins")
    #[serde(default)]
    pub remote_version: Option<u64>,
}

/// A recorded sync conflict entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictRecord {
    /// Path that had the conflict
    pub path: String,
    /// Local version at time of conflict
    pub local_version: u64,
    /// Remote version that conflicted
    pub remote_version: u64,
    /// Base version the client was syncing from
    pub base_version: u64,
    /// When the conflict was recorded (ISO 8601)
    pub timestamp: String,
}

/// Response listing conflict history
#[derive(Debug, Serialize)]
pub struct ConflictHistoryResponse {
    pub conflicts: Vec<ConflictRecord>,
    pub total: usize,
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
    /// Conflict history (capped at MAX_CONFLICT_HISTORY records)
    conflict_history: Vec<ConflictRecord>,
}

const MAX_CONFLICT_HISTORY: usize = 1000;

#[derive(Debug, Serialize, Deserialize)]
struct SyncSnapshot {
    version: u64,
    pending_changes: Vec<DeltaChange>,
    tracked_paths: HashMap<String, u64>,
    #[serde(default)]
    conflict_history: Vec<ConflictRecord>,
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
                conflict_history: Vec::new(),
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
                conflict_history: snapshot.conflict_history,
            })),
            persistence_path: Arc::new(Some(path)),
        })
    }

    fn default_snapshot() -> SyncSnapshot {
        SyncSnapshot {
            version: 0,
            pending_changes: Vec::new(),
            tracked_paths: HashMap::new(),
            conflict_history: Vec::new(),
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
                conflict_history: inner.conflict_history.clone(),
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
                // Path is tracked and incoming version is behind current version
                if current_version > change.version {
                    // Record the conflict in history (capped at MAX_CONFLICT_HISTORY)
                    if inner.conflict_history.len() >= MAX_CONFLICT_HISTORY {
                        inner.conflict_history.remove(0);
                    }
                    inner.conflict_history.push(ConflictRecord {
                        path: change.path.clone(),
                        local_version: current_version,
                        remote_version: change.version,
                        base_version: req.base_version,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    conflicts.push(change.path.clone());
                    continue;
                }
            }

            // Apply the change
            match change.op.as_str() {
                "created" | "modified" => {
                    inner
                        .tracked_paths
                        .insert(change.path.clone(), change.version);
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

    /// Get conflict history (newest first)
    pub fn get_conflicts(&self) -> ConflictHistoryResponse {
        let inner = self.inner.read();
        let total = inner.conflict_history.len();
        let mut conflicts: Vec<ConflictRecord> = inner.conflict_history.clone();
        conflicts.reverse(); // Newest first
        ConflictHistoryResponse { conflicts, total }
    }

    /// Resolve sync conflicts with given strategy.
    /// Returns the number of resolved paths.
    pub fn resolve_conflicts(&self, path: &str, strategy: &str, remote_version: Option<u64>) {
        let mut inner = self.inner.write();
        match strategy {
            "accept_local" => {
                // Keep current version, do nothing
            }
            "accept_remote" | "last_write_wins" => {
                let new_ver = remote_version.unwrap_or(inner.version.saturating_add(1));
                inner.tracked_paths.insert(path.to_string(), new_ver);
            }
            _ => {}
        }
    }

    /// Get tracked version for a specific path
    pub fn get_tracked_path_version(&self, path: &str) -> u64 {
        self.inner
            .read()
            .tracked_paths
            .get(path)
            .copied()
            .unwrap_or(0)
    }
}

/// Sync handlers
pub struct SyncHandlers;

impl SyncHandlers {
    /// GET /api/v1/sync/status - Get sync status
    pub async fn get_status(State(state): State<SyncState>) -> RestResult<impl IntoResponse> {
        Ok(Json(state.get_status()))
    }

    /// POST /api/v1/sync/delta - Submit delta changes
    pub async fn apply_delta(
        State(state): State<SyncState>,
        Json(req): Json<DeltaRequest>,
    ) -> RestResult<impl IntoResponse> {
        if req.changes.is_empty() {
            return Err(RestError::BadRequest("No changes provided".into()));
        }

        let response = state.apply_delta(req).await;
        Ok(Json(response))
    }

    /// GET /api/v1/sync/version - Get current version
    pub async fn get_version(State(state): State<SyncState>) -> RestResult<impl IntoResponse> {
        let version = state.get_version();
        Ok(Json(serde_json::json!({ "version": version })))
    }

    /// GET /api/v1/sync/:path/version - Get version for specific path
    pub async fn get_path_version(
        State(state): State<SyncState>,
        path: axum::extract::Path<String>,
    ) -> RestResult<impl IntoResponse> {
        let version = state.get_tracked_path_version(&path.0);
        Ok(Json(serde_json::json!({
            "path": path.0,
            "version": version
        })))
    }

    /// POST /api/v1/sync/resolve - Resolve sync conflicts
    pub async fn resolve(
        State(state): State<SyncState>,
        Json(req): Json<ResolveConflictRequest>,
    ) -> RestResult<impl IntoResponse> {
        if req.resolutions.is_empty() {
            return Err(RestError::BadRequest(
                "No conflict resolutions provided".into(),
            ));
        }

        for resolution in &req.resolutions {
            if resolution.path.is_empty() {
                return Err(RestError::BadRequest(
                    "Conflict resolution path cannot be empty".into(),
                ));
            }
            let valid_strategies = ["accept_local", "accept_remote", "last_write_wins"];
            if !valid_strategies.contains(&resolution.strategy.as_str()) {
                return Err(RestError::BadRequest(format!(
                    "Invalid strategy '{}'. Must be one of: accept_local, accept_remote, last_write_wins",
                    resolution.strategy
                )));
            }
            state.resolve_conflicts(
                &resolution.path,
                &resolution.strategy,
                resolution.remote_version,
            );
        }

        Ok(Json(serde_json::json!({
            "resolved": req.resolutions.len(),
            "status": state.get_status()
        })))
    }

    /// GET /api/v1/sync/conflicts - Get conflict history
    pub async fn get_conflicts(State(state): State<SyncState>) -> RestResult<impl IntoResponse> {
        Ok(Json(state.get_conflicts()))
    }
}
