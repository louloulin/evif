// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Copy-on-Write Snapshot Module
//!
//! Provides filesystem snapshot functionality using copy-on-write semantics.
//! Snapshots capture the state of a filesystem at a point in time,
//! allowing efficient branching and versioning.

use crate::FileInfo;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Unique snapshot ID generator
static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Snapshot entry - represents a file/directory at snapshot time
#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    /// Original path
    pub path: String,
    /// File metadata at snapshot time
    pub file_info: FileInfo,
    /// Content hash (for deduplication)
    pub content_hash: Option<u64>,
    /// Is this a directory
    pub is_dir: bool,
}

/// Snapshot metadata
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    /// Unique snapshot ID
    pub id: u64,
    /// Human-readable snapshot name
    pub name: String,
    /// Creation timestamp (Unix timestamp in seconds)
    pub created_at: u64,
    /// Parent snapshot ID (None for root)
    pub parent_id: Option<u64>,
    /// Number of files in snapshot
    pub file_count: usize,
    /// Total size in bytes
    pub total_size: u64,
    /// Description
    pub description: String,
}

/// Copy-on-Write Snapshot
///
/// A snapshot captures the state of a filesystem at a point in time.
/// Writing to a snapshot creates a copy (CoW), leaving the original unchanged.
pub struct CowSnapshot {
    /// Snapshot metadata (with interior mutability)
    metadata: Arc<parking_lot::RwLock<SnapshotMetadata>>,
    /// File entries (path -> entry)
    entries: Arc<parking_lot::RwLock<HashMap<String, SnapshotEntry>>>,
    /// Base snapshot for deduplication (if derived)
    base: Option<Arc<CowSnapshot>>,
    /// Modified paths in this snapshot (for CoW tracking)
    modified: Arc<parking_lot::RwLock<HashMap<String, SnapshotEntry>>>,
}

impl CowSnapshot {
    /// Create a new root snapshot
    pub fn new(name: String, description: String) -> Self {
        let id = SNAPSHOT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: Arc::new(RwLock::new(SnapshotMetadata {
                id,
                name,
                created_at: now,
                parent_id: None,
                file_count: 0,
                total_size: 0,
                description,
            })),
            entries: Arc::new(RwLock::new(HashMap::new())),
            base: None,
            modified: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a derived snapshot (branch) from a parent
    pub fn branch(&self, name: String, description: String) -> Self {
        let id = SNAPSHOT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let parent_meta = self.metadata.read();
        Self {
            metadata: Arc::new(RwLock::new(SnapshotMetadata {
                id,
                name,
                created_at: now,
                parent_id: Some(parent_meta.id),
                file_count: parent_meta.file_count,
                total_size: parent_meta.total_size,
                description,
            })),
            entries: Arc::clone(&self.entries),
            base: Some(Arc::new(self.clone_inner())),
            modified: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Internal clone for base snapshot
    fn clone_inner(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            entries: Arc::clone(&self.entries),
            base: None,
            modified: Arc::clone(&self.modified),
        }
    }

    /// Add a file to the snapshot
    pub fn add_file(&self, path: String, file_info: FileInfo, content_hash: Option<u64>) {
        let entry = SnapshotEntry {
            path: path.clone(),
            file_info: file_info.clone(),
            content_hash,
            is_dir: file_info.is_dir,
        };

        let mut modified = self.modified.write();
        modified.insert(path, entry);

        drop(modified);
        self.update_metadata();
    }

    /// Read a file from the snapshot (CoW aware)
    pub fn read(&self, path: &str) -> Option<SnapshotEntry> {
        // Check modified entries first
        let modified = self.modified.read();
        if let Some(entry) = modified.get(path) {
            return Some(entry.clone());
        }

        // Fall back to base entries
        let entries = self.entries.read();
        entries.get(path).cloned()
    }

    /// List all files in the snapshot
    pub fn list(&self) -> Vec<SnapshotEntry> {
        let mut result = Vec::new();

        // Include base entries
        let entries = self.entries.read();
        for entry in entries.values() {
            result.push(entry.clone());
        }

        // Include modified entries (overriding base if same path)
        let modified = self.modified.read();
        for (path, entry) in modified.iter() {
            if let Some(base_entry) = entries.get(path) {
                // This path exists in base, check if it was modified
                if base_entry.file_info.modified != entry.file_info.modified {
                    // Modified, replace in result
                    if let Some(existing) = result.iter_mut().find(|e| e.path == *path) {
                        *existing = entry.clone();
                    }
                }
            } else {
                // New file added in this snapshot
                result.push(entry.clone());
            }
        }

        result
    }

    /// Check if a path was modified in this snapshot
    pub fn is_modified(&self, path: &str) -> bool {
        let modified = self.modified.read();
        modified.contains_key(path)
    }

    /// Get the diff between this snapshot and its parent
    pub fn diff(&self) -> SnapshotDiff {
        let mut added = Vec::new();
        let mut modified_paths = Vec::new();
        let mut removed = Vec::new();

        match &self.base {
            Some(base) => {
                // Get all base entries (including parent's modified entries)
                let base_all = base.list();
                let base_paths: std::collections::HashSet<_> = base_all.iter().map(|e| e.path.clone()).collect();
                let base_map: std::collections::HashMap<_, _> = base_all.iter().map(|e| (e.path.clone(), e)).collect();

                let current_modified = self.modified.read();
                let current_entries = self.entries.read();

                // Get all paths in this snapshot
                let all_paths: std::collections::HashSet<_> = current_entries
                    .keys()
                    .chain(current_modified.keys())
                    .cloned()
                    .collect();

                // Find added and modified files
                for (path, entry) in current_modified.iter() {
                    if base_paths.contains(path) {
                        // Check if content actually changed
                        if let Some(base_entry) = base_map.get(path) {
                            if base_entry.file_info.modified != entry.file_info.modified ||
                               base_entry.content_hash != entry.content_hash {
                                modified_paths.push(path.clone());
                            }
                        }
                    } else {
                        added.push(entry.clone());
                    }
                }

                // Find removed files (in base but not in current snapshot)
                for path in &base_paths {
                    if !all_paths.contains(path) {
                        removed.push(path.clone());
                    }
                }
            }
            None => {
                // Root snapshot - all entries are additions
                let modified = self.modified.read();
                for entry in modified.values() {
                    added.push(entry.clone());
                }
            }
        }

        SnapshotDiff {
            added,
            modified: modified_paths,
            removed,
        }
    }

    /// Update metadata based on current entries
    fn update_metadata(&self) {
        let entries = self.list();
        let file_count = entries.len();
        let total_size: u64 = entries.iter().map(|e| e.file_info.size).sum();

        let mut metadata = self.metadata.write();
        metadata.file_count = file_count;
        metadata.total_size = total_size;
    }

    /// Get snapshot metadata
    pub fn metadata(&self) -> SnapshotMetadata {
        self.metadata.read().clone()
    }

    /// Get parent snapshot ID
    pub fn parent_id(&self) -> Option<u64> {
        self.metadata.read().parent_id
    }

    /// Check if this is a root snapshot
    pub fn is_root(&self) -> bool {
        self.metadata.read().parent_id.is_none()
    }
}

impl Clone for CowSnapshot {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            entries: Arc::clone(&self.entries),
            base: self.base.clone(),
            modified: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Snapshot diff result
#[derive(Debug, Clone, Default)]
pub struct SnapshotDiff {
    /// Files added in this snapshot
    pub added: Vec<SnapshotEntry>,
    /// Files modified in this snapshot (paths only)
    pub modified: Vec<String>,
    /// Files removed in this snapshot (paths only)
    pub removed: Vec<String>,
}

/// Snapshot manager - manages multiple snapshots
pub struct SnapshotManager {
    /// All snapshots (id -> snapshot)
    snapshots: RwLock<HashMap<u64, Arc<CowSnapshot>>>,
    /// Snapshot name index (name -> id)
    names: RwLock<HashMap<String, u64>>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
            names: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new root snapshot
    pub fn create_snapshot(&self, name: String, description: String) -> Arc<CowSnapshot> {
        let snapshot = Arc::new(CowSnapshot::new(name.clone(), description));

        // Store by ID
        let id = snapshot.metadata().id;
        self.snapshots.write().insert(id, Arc::clone(&snapshot));

        // Index by name
        self.names.write().insert(name, id);

        snapshot
    }

    /// Branch from an existing snapshot
    pub fn branch(
        &self,
        parent_id: u64,
        name: String,
        description: String,
    ) -> Option<Arc<CowSnapshot>> {
        let snapshots = self.snapshots.read();
        let parent = snapshots.get(&parent_id)?;

        let child = Arc::new(parent.branch(name.clone(), description));

        // Store by ID
        let id = child.metadata.read().id;
        drop(snapshots);
        self.snapshots.write().insert(id, Arc::clone(&child));

        // Index by name
        self.names.write().insert(name, id);

        Some(child)
    }

    /// Get a snapshot by ID
    pub fn get(&self, id: u64) -> Option<Arc<CowSnapshot>> {
        self.snapshots.read().get(&id).cloned()
    }

    /// Get a snapshot by name
    pub fn get_by_name(&self, name: &str) -> Option<Arc<CowSnapshot>> {
        let names = self.names.read();
        let id = names.get(name)?.clone();
        drop(names);
        self.get(id)
    }

    /// List all snapshots
    pub fn list(&self) -> Vec<SnapshotMetadata> {
        let snapshots = self.snapshots.read();
        snapshots.values().map(|s| s.metadata().clone()).collect()
    }

    /// Delete a snapshot (only if it has no children)
    pub fn delete(&self, id: u64) -> bool {
        let snapshots = self.snapshots.read();
        let _snapshot = match snapshots.get(&id) {
            Some(s) => s,
            None => return false,
        };

        // Check for children
        let has_children = snapshots.values().any(|s| s.parent_id() == Some(id));
        if has_children {
            return false;
        }

        // Remove from snapshots
        drop(snapshots);
        let name = self.snapshots.read().get(&id).map(|s| s.metadata().name.clone());
        self.snapshots.write().remove(&id);

        // Remove from name index
        if let Some(name) = name {
            self.names.write().remove(&name);
        }

        true
    }

    /// Merge two snapshots (child into parent)
    /// Returns a new snapshot that combines both parent and child
    pub fn merge(&self, child_id: u64) -> Option<Arc<CowSnapshot>> {
        let snapshots = self.snapshots.read();
        let child = snapshots.get(&child_id)?.clone();
        let parent_id = child.parent_id()?;
        let parent_base = snapshots.get(&parent_id)?.clone();
        drop(snapshots);

        // Create a merged snapshot from parent
        let merged = CowSnapshot::new(
            format!("merged-{}-to-{}", child_id, parent_id),
            format!("Merged child {} into parent {}", child_id, parent_id),
        );

        // Copy parent entries
        let parent_entries = parent_base.entries.read();
        let child_modified = child.modified.read();

        for (path, entry) in parent_entries.iter() {
            // If child modified this path, use child's version
            if let Some(child_entry) = child_modified.get(path) {
                merged.entries.write().insert(path.clone(), child_entry.clone());
            } else {
                merged.entries.write().insert(path.clone(), entry.clone());
            }
        }

        // Add new files from child (not in parent)
        for (path, entry) in child_modified.iter() {
            if !parent_entries.contains_key(path) {
                merged.entries.write().insert(path.clone(), entry.clone());
            }
        }

        // Update metadata
        let entries = merged.entries.read();
        let file_count = entries.len();
        let total_size: u64 = entries.iter().map(|e| e.1.file_info.size).sum();
        drop(entries); // Release borrow before moving merged
        {
            let mut metadata = merged.metadata.write();
            metadata.file_count = file_count;
            metadata.total_size = total_size;
        }

        // Store the merged snapshot
        let merged_arc = Arc::new(merged);
        let meta = merged_arc.metadata();
        self.snapshots.write().insert(meta.id, Arc::clone(&merged_arc));
        self.names.write().insert(meta.name.clone(), meta.id);

        // Delete the child
        self.delete(child_id);

        Some(merged_arc)
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_file_info(name: &str, size: u64) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size,
            mode: 0o644,
            modified: chrono::Utc::now(),
            is_dir: false,
        }
    }

    #[test]
    fn test_create_snapshot() {
        let manager = SnapshotManager::new();
        let snapshot = manager.create_snapshot("test".to_string(), "Test snapshot".to_string());

        assert_eq!(snapshot.metadata().name, "test");
        assert!(snapshot.is_root());
        assert!(snapshot.parent_id().is_none());
    }

    #[test]
    fn test_branch_snapshot() {
        let manager = SnapshotManager::new();
        let parent = manager.create_snapshot("parent".to_string(), "Parent".to_string());

        let child = manager
            .branch(parent.metadata().id, "child".to_string(), "Child".to_string())
            .unwrap();

        assert_eq!(child.parent_id(), Some(parent.metadata().id));
        assert!(!child.is_root());
    }

    #[test]
    fn test_add_file_to_snapshot() {
        let manager = SnapshotManager::new();
        let snapshot = manager.create_snapshot("test".to_string(), "Test".to_string());

        snapshot.add_file(
            "/test.txt".to_string(),
            create_test_file_info("test.txt", 100),
            Some(1234),
        );

        let entry = snapshot.read("/test.txt");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().file_info.size, 100);
    }

    #[test]
    fn test_copy_on_write() {
        let manager = SnapshotManager::new();
        let parent = manager.create_snapshot("parent".to_string(), "Parent".to_string());

        // Add file to parent
        parent.add_file(
            "/test.txt".to_string(),
            create_test_file_info("test.txt", 100),
            Some(1234),
        );

        // Branch and modify in child
        let child = manager
            .branch(parent.metadata().id, "child".to_string(), "Child".to_string())
            .unwrap();

        child.add_file(
            "/test.txt".to_string(),
            create_test_file_info("test.txt", 200),
            Some(5678),
        );

        // Parent should be unchanged
        let parent_entry = parent.read("/test.txt").unwrap();
        assert_eq!(parent_entry.file_info.size, 100);

        // Child should have new value
        let child_entry = child.read("/test.txt").unwrap();
        assert_eq!(child_entry.file_info.size, 200);
    }

    #[test]
    fn test_snapshot_diff() {
        let manager = SnapshotManager::new();
        let parent = manager.create_snapshot("parent".to_string(), "Parent".to_string());

        parent.add_file(
            "/existing.txt".to_string(),
            create_test_file_info("existing.txt", 50),
            Some(1111),
        );

        let child = manager
            .branch(parent.metadata().id, "child".to_string(), "Child".to_string())
            .unwrap();

        child.add_file(
            "/new.txt".to_string(),
            create_test_file_info("new.txt", 75),
            Some(2222),
        );

        child.add_file(
            "/existing.txt".to_string(),
            create_test_file_info("existing.txt", 100),
            Some(3333),
        );

        let diff = child.diff();

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].path, "/new.txt");
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0], "/existing.txt");
    }

    #[test]
    fn test_list_snapshots() {
        let manager = SnapshotManager::new();

        manager.create_snapshot("snap1".to_string(), "Snap 1".to_string());
        manager.create_snapshot("snap2".to_string(), "Snap 2".to_string());

        let snapshots = manager.list();
        assert_eq!(snapshots.len(), 2);
    }

    #[test]
    fn test_delete_snapshot() {
        let manager = SnapshotManager::new();
        let snapshot = manager.create_snapshot("test".to_string(), "Test".to_string());
        let id = snapshot.metadata().id;

        assert!(manager.delete(id));
        assert!(manager.get(id).is_none());
    }

    #[test]
    fn test_cannot_delete_snapshot_with_children() {
        let manager = SnapshotManager::new();
        let parent = manager.create_snapshot("parent".to_string(), "Parent".to_string());

        manager.branch(
            parent.metadata().id,
            "child".to_string(),
            "Child".to_string(),
        );

        // Cannot delete parent with child
        assert!(!manager.delete(parent.metadata().id));

        // Parent still exists
        assert!(manager.get(parent.metadata().id).is_some());
    }
}
