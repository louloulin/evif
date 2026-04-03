// SQLFS - Database-backed File System Plugin
//
// Provides persistent file system storage using SQLite backend
// with LRU caching for efficient directory listings.

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};

#[cfg(feature = "sqlfs")]
use rusqlite::{Connection, params};

/// Maximum file size: 5MB (matches AGFS)
pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

/// Cache entry for directory listings
#[derive(Clone)]
struct CacheEntry {
    files: Vec<FileInfo>,
    mod_time: Instant,
}

/// LRU Cache for directory listings
struct ListDirCache {
    cache: HashMap<String, CacheEntry>,
    lru_list: VecDeque<String>,
    max_size: usize,
    ttl: Duration,
    enabled: bool,
    hits: u64,
    misses: u64,
}

impl ListDirCache {
    fn new(max_size: usize, ttl: Duration, enabled: bool) -> Self {
        Self {
            cache: HashMap::new(),
            lru_list: VecDeque::new(),
            max_size: max_size.max(1),
            ttl,
            enabled,
            hits: 0,
            misses: 0,
        }
    }

    fn get(&mut self, path: &str) -> Option<Vec<FileInfo>> {
        if !self.enabled {
            return None;
        }

        if let Some(entry) = self.cache.get(path) {
            if entry.mod_time.elapsed() < self.ttl {
                // Cache hit - move to front of LRU list
                self.hits += 1;
                self.lru_list.retain(|p| p != path);
                self.lru_list.push_front(path.to_string());
                return Some(entry.files.clone());
            } else {
                // Expired
                self.cache.remove(path);
                self.lru_list.retain(|p| p != path);
            }
        }

        self.misses += 1;
        None
    }

    fn put(&mut self, path: String, files: Vec<FileInfo>) {
        if !self.enabled {
            return;
        }

        // Remove existing entry if present
        self.cache.remove(&path);
        self.lru_list.retain(|p| p != &path);

        // Evict oldest entry if cache is full
        if self.lru_list.len() >= self.max_size {
            if let Some(oldest) = self.lru_list.pop_back() {
                self.cache.remove(&oldest);
            }
        }

        // Add new entry
        self.cache.insert(path.clone(), CacheEntry {
            files,
            mod_time: Instant::now(),
        });
        self.lru_list.push_front(path);
    }

    fn invalidate(&mut self, path: &str) {
        if !self.enabled {
            return;
        }
        self.cache.remove(path);
        self.lru_list.retain(|p| p != path);
    }

    fn invalidate_prefix(&mut self, prefix: &str) {
        if !self.enabled {
            return;
        }
        let to_remove: Vec<String> = self.cache.keys()
            .filter(|k| *k == prefix || is_descendant(k, prefix))
            .cloned()
            .collect();

        for path in to_remove {
            self.cache.remove(&path);
            self.lru_list.retain(|p| p != &path);
        }
    }

    fn invalidate_parent(&mut self, path: &str) {
        let parent = get_parent_path(path);
        self.invalidate(&parent);
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.lru_list.clear();
    }
}

/// Check if path is a descendant of parent
fn is_descendant(path: &str, parent: &str) -> bool {
    if path == parent {
        return false;
    }
    if parent == "/" {
        return path != "/";
    }
    if path.len() <= parent.len() {
        return false;
    }
    &path[..parent.len()] == parent && path.as_bytes()[parent.len()] == b'/'
}

/// Get parent directory path
fn get_parent_path(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }

    if let Some(last_slash) = path.rfind('/') {
        if last_slash == 0 {
            return "/".to_string();
        }
        return path[..last_slash].to_string();
    }

    "/".to_string()
}

/// Normalize path to start with / and have no trailing slash (except root)
fn normalize_path(path: &str) -> String {
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        return "/".to_string();
    }

    format!("/{}", path)
}

/// SQLFS configuration
#[derive(Clone, Debug)]
pub struct SqlfsConfig {
    pub db_path: String,
    pub cache_enabled: bool,
    pub cache_max_size: usize,
    pub cache_ttl_seconds: u64,
}

impl Default for SqlfsConfig {
    fn default() -> Self {
        Self {
            db_path: "sqlfs.db".to_string(),
            cache_enabled: true,
            cache_max_size: 1000,
            cache_ttl_seconds: 5,
        }
    }
}

/// SQLFS Plugin (using tokio::task::spawn_blocking for database operations)
#[cfg(feature = "sqlfs")]
pub struct SqlfsPlugin {
    db_path: String,
    cache: Arc<RwLock<ListDirCache>>,
    config: SqlfsConfig,
}

#[cfg(feature = "sqlfs")]
impl SqlfsPlugin {
    /// Create new SQLFS plugin
    pub fn new(config: SqlfsConfig) -> EvifResult<Self> {
        let db_path = config.db_path.clone();

        // Initialize database in blocking thread
        let db_path_clone = db_path.clone();
        tokio::task::block_in_place(|| {
            let conn = Connection::open(&db_path_clone)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Create schema
            conn.execute(
                "CREATE TABLE IF NOT EXISTS files (
                    path TEXT PRIMARY KEY,
                    is_dir INTEGER NOT NULL,
                    mode INTEGER NOT NULL,
                    size INTEGER NOT NULL,
                    mod_time INTEGER NOT NULL,
                    data BLOB
                )",
                [],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to create table: {}", e)))?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS saved_queries (
                    name TEXT PRIMARY KEY,
                    query TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    modified_at INTEGER NOT NULL
                )",
                [],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to create saved_queries table: {}", e)))?;

            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_parent ON files(path)",
                [],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to create index: {}", e)))?;

            // Optimize SQLite (ignore PRAGMA errors as they may not return rows)
            let _ = conn.execute("PRAGMA journal_mode=WAL", []);
            let _ = conn.execute("PRAGMA synchronous=NORMAL", []);
            let _ = conn.execute("PRAGMA cache_size=-64000", []);

            // Ensure root exists
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files WHERE path = '/'",
                [],
                |row| row.get(0),
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            if count == 0 {
                let now = Utc::now().timestamp();
                conn.execute(
                    "INSERT INTO files (path, is_dir, mode, size, mod_time, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    ["/", "1", "755", "0", &now.to_string(), ""],
                ).map_err(|e| EvifError::InvalidPath(format!("failed to create root: {}", e)))?;
            }

            Ok::<(), EvifError>(())
        })?;

        Ok(Self {
            db_path,
            cache: Arc::new(RwLock::new(ListDirCache::new(
                config.cache_max_size,
                Duration::from_secs(config.cache_ttl_seconds),
                config.cache_enabled,
            ))),
            config,
        })
    }

    /// Execute a SELECT SQL query and return results as JSON
    /// The query runs against the internal files table and any other tables in the DB.
    /// Results are returned as a JSON array of objects.
    pub async fn execute_query(&self, query: &str) -> EvifResult<String> {
        let db_path = self.db_path.clone();
        let query = query.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::Storage(format!("failed to open database: {}", e)))?;

            // Only allow SELECT statements (basic safety check)
            let trimmed = query.trim().to_uppercase();
            if !trimmed.starts_with("SELECT") {
                return Err(EvifError::InvalidPath(
                    "Only SELECT queries are allowed".to_string()
                ));
            }

            let mut stmt = conn.prepare(&query)
                .map_err(|e| EvifError::Storage(format!("Failed to prepare query: {}", e)))?;

            let column_count = stmt.column_count();
            let column_names: Vec<String> = (0..column_count)
                .map(|i| stmt.column_name(i).unwrap_or("unknown").to_string())
                .collect();

            let mut rows_result = stmt.query([])
                .map_err(|e| EvifError::Storage(format!("Failed to execute query: {}", e)))?;

            let mut json_rows: Vec<serde_json::Value> = Vec::new();
            while let Some(row) = rows_result.next()
                .map_err(|e| EvifError::Storage(format!("Failed to fetch row: {}", e)))?
            {
                let mut obj = serde_json::Map::new();
                for (i, col_name) in column_names.iter().enumerate() {
                    let value: serde_json::Value = match row.get_ref(i) {
                        Ok(value_ref) => {
                            match value_ref {
                                rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                                rusqlite::types::ValueRef::Integer(n) => serde_json::json!(n),
                                rusqlite::types::ValueRef::Real(f) => serde_json::json!(f),
                                rusqlite::types::ValueRef::Text(s) => {
                                    serde_json::json!(String::from_utf8_lossy(s).to_string())
                                }
                                rusqlite::types::ValueRef::Blob(b) => {
                                    serde_json::json!(format!("<blob {} bytes>", b.len()))
                                }
                            }
                        }
                        Err(_) => serde_json::Value::Null,
                    };
                    obj.insert(col_name.clone(), value);
                }
                json_rows.push(serde_json::Value::Object(obj));
            }

            let result = serde_json::to_string_pretty(&json_rows)
                .map_err(|e| EvifError::Storage(format!("Failed to serialize results: {}", e)))?;

            Ok(result)
        }).await
        .map_err(|e| EvifError::Storage(format!("spawn_blocking error: {}", e)))?
    }

    /// List all saved queries
    pub async fn list_saved_queries(&self) -> EvifResult<Vec<(String, String, DateTime<Utc>)>> {
        let db_path = self.db_path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::Storage(format!("failed to open database: {}", e)))?;

            let mut stmt = conn.prepare(
                "SELECT name, query, modified_at FROM saved_queries ORDER BY name"
            ).map_err(|e| EvifError::Storage(format!("Failed to prepare: {}", e)))?;

            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            }).map_err(|e| EvifError::Storage(format!("Failed to query: {}", e)))?;

            let mut queries = Vec::new();
            for row in rows {
                let (name, query, modified_at) = row
                    .map_err(|e| EvifError::Storage(format!("Failed to read row: {}", e)))?;
                queries.push((
                    name,
                    query,
                    DateTime::from_timestamp(modified_at, 0).unwrap_or_default(),
                ));
            }

            Ok(queries)
        }).await
        .map_err(|e| EvifError::Storage(format!("spawn_blocking error: {}", e)))?
    }

    /// Save a query for later reuse
    pub async fn save_query(&self, name: &str, query: &str) -> EvifResult<()> {
        let db_path = self.db_path.clone();
        let name = name.to_string();
        let query = query.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::Storage(format!("failed to open database: {}", e)))?;

            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT OR REPLACE INTO saved_queries (name, query, created_at, modified_at) VALUES (?1, ?2, ?3, ?4)",
                params![name, query, now, now],
            ).map_err(|e| EvifError::Storage(format!("Failed to save query: {}", e)))?;

            Ok(())
        }).await
        .map_err(|e| EvifError::Storage(format!("spawn_blocking error: {}", e)))?
    }

    /// Delete a saved query
    pub async fn delete_query(&self, name: &str) -> EvifResult<()> {
        let db_path = self.db_path.clone();
        let name = name.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::Storage(format!("failed to open database: {}", e)))?;

            conn.execute(
                "DELETE FROM saved_queries WHERE name = ?1",
                params![name],
            ).map_err(|e| EvifError::Storage(format!("Failed to delete query: {}", e)))?;

            Ok(())
        }).await
        .map_err(|e| EvifError::Storage(format!("spawn_blocking error: {}", e)))?
    }
}

#[cfg(feature = "sqlfs")]
#[async_trait]
impl EvifPlugin for SqlfsPlugin {
    fn name(&self) -> &str {
        "sqlfs2"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let path = normalize_path(path);
        let db_path = self.db_path.clone();
        let path_for_cache = path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Check if parent exists
            let parent = get_parent_path(&path);
            let parent_for_error = parent.clone();
            if parent != "/" {
                let is_dir: i64 = conn.query_row(
                    "SELECT is_dir FROM files WHERE path = ?1",
                    [&parent],
                    |row| row.get(0),
                ).map_err(|e| {
                    if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                        EvifError::NotFound(parent_for_error)
                    } else {
                        EvifError::InvalidPath(format!("query failed: {}", e))
                    }
                })?;

                if is_dir == 0 {
                    return Err(EvifError::InvalidPath(format!("parent is not a directory: {}", parent)));
                }
            }

            // Check if file already exists
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                [&path],
                |row| row.get(0),
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            if count > 0 {
                return Err(EvifError::InvalidPath(format!("file already exists: {}", path)));
            }

            // Create empty file
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO files (path, is_dir, mode, size, mod_time, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [&path, "0", "644", "0", &now.to_string(), ""],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to create file: {}", e)))?;

            Ok::<(), EvifError>(())
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Invalidate parent cache
        self.cache.write().await.invalidate_parent(&path_for_cache);

        Ok(())
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        let path = normalize_path(path);
        let db_path = self.db_path.clone();
        let path_for_cache = path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Check if parent exists
            let parent = get_parent_path(&path);
            let parent_for_error = parent.clone();
            if parent != "/" {
                let is_dir: i64 = conn.query_row(
                    "SELECT is_dir FROM files WHERE path = ?1",
                    [&parent],
                    |row| row.get(0),
                ).map_err(|e| {
                    if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                        EvifError::NotFound(parent_for_error)
                    } else {
                        EvifError::InvalidPath(format!("query failed: {}", e))
                    }
                })?;

                if is_dir == 0 {
                    return Err(EvifError::InvalidPath(format!("parent is not a directory: {}", parent)));
                }
            }

            // Check if directory already exists
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                [&path],
                |row| row.get(0),
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            if count > 0 {
                return Err(EvifError::InvalidPath(format!("directory already exists: {}", path)));
            }

            // Create directory
            let mode = if perm == 0 { 0o755 } else { perm };
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO files (path, is_dir, mode, size, mod_time, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [&path, "1", &mode.to_string(), "0", &now.to_string(), ""],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to create directory: {}", e)))?;

            Ok::<(), EvifError>(())
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Invalidate parent cache
        self.cache.write().await.invalidate_parent(&path_for_cache);

        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let path = normalize_path(path);
        let db_path = self.db_path.clone();
        let path_for_error = path.clone();

        let data = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            let (is_dir, data): (i64, Vec<u8>) = conn.query_row(
                "SELECT is_dir, data FROM files WHERE path = ?1",
                [&path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    EvifError::NotFound(path_for_error.clone())
                } else {
                    EvifError::InvalidPath(format!("query failed: {}", e))
                }
            })?;

            if is_dir == 1 {
                return Err(EvifError::InvalidPath(format!("path is a directory: {}", path_for_error)));
            }

            // Apply offset and size
            let data_len = data.len() as u64;
            let offset = offset.min(data_len) as usize;

            if offset >= data.len() {
                return Ok(Vec::new());
            }

            let end = if size > 0 {
                (offset as u64 + size).min(data_len) as usize
            } else {
                data.len()
            };

            Ok::<Vec<u8>, EvifError>(data[offset..end].to_vec())
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        Ok(data)
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        let path = normalize_path(path);

        // Check file size limit
        if data.len() > MAX_FILE_SIZE {
            return Err(EvifError::InvalidPath(format!(
                "file size exceeds maximum limit of {}MB (got {} bytes)",
                MAX_FILE_SIZE / (1024 * 1024),
                data.len()
            )));
        }

        let db_path = self.db_path.clone();
        let path_clone = path.clone();

        let (data_len, is_new_file) = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Check if file exists
            let (exists, is_dir): (i64, i64) = conn.query_row(
                "SELECT COUNT(*), COALESCE(MAX(is_dir), 0) FROM files WHERE path = ?1",
                [&path_clone],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            if exists > 0 && is_dir == 1 {
                return Err(EvifError::InvalidPath(format!("path is a directory: {}", path_clone)));
            }

            let data_len = data.len() as i64;
            let now = Utc::now().timestamp();

            if exists == 0 {
                // Create new file
                let parent = get_parent_path(&path_clone);
                let parent_for_error = parent.clone();
                if parent != "/" {
                    let parent_is_dir: i64 = conn.query_row(
                        "SELECT is_dir FROM files WHERE path = ?1",
                        [&parent],
                        |row| row.get(0),
                    ).map_err(|e| {
                        if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                            EvifError::NotFound(parent_for_error)
                        } else {
                            EvifError::InvalidPath(format!("query failed: {}", e))
                        }
                    })?;

                    if parent_is_dir == 0 {
                        return Err(EvifError::InvalidPath(format!("parent is not a directory: {}", parent)));
                    }
                }

                conn.execute(
                    "INSERT INTO files (path, is_dir, mode, size, mod_time, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    [&path_clone, "0", "644", &data_len.to_string(), &now.to_string()],
                ).map_err(|e| EvifError::InvalidPath(format!("failed to create file: {}", e)))?;

                // Then set the BLOB data
                conn.execute(
                    "UPDATE files SET data = ?1 WHERE path = ?2",
                    rusqlite::params![data.as_slice(), path_clone.as_str()],
                ).map_err(|e| EvifError::InvalidPath(format!("failed to set file data: {}", e)))?;

                Ok::<(i64, bool), EvifError>((data_len, true))
            } else {
                // Update existing file
                conn.execute(
                    "UPDATE files SET data = ?1, size = ?2, mod_time = ?3 WHERE path = ?4",
                    rusqlite::params![data.as_slice(), data_len.to_string(), now.to_string(), path_clone.as_str()],
                ).map_err(|e| EvifError::InvalidPath(format!("failed to update file: {}", e)))?;

                Ok((data_len, false))
            }
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Invalidate parent cache on new file creation
        if is_new_file {
            self.cache.write().await.invalidate_parent(&path);
        }

        Ok(data_len as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = normalize_path(path);
        let path_for_cache = path.clone();

        // Try cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(files) = cache.get(&path) {
                return Ok(files);
            }
        }

        let db_path = self.db_path.clone();
        let path_clone = path.clone();

        let files = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Check if directory exists
            let is_dir: i64 = conn.query_row(
                "SELECT is_dir FROM files WHERE path = ?1",
                [&path_clone],
                |row| row.get(0),
            ).map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    EvifError::NotFound(path_clone.clone())
                } else {
                    EvifError::InvalidPath(format!("query failed: {}", e))
                }
            })?;

            if is_dir == 0 {
                return Err(EvifError::InvalidPath(format!("path is not a directory: {}", path_clone)));
            }

            // Query children
            let pattern = if path_clone == "/" {
                "/".to_string()
            } else {
                format!("{}/", path_clone)
            };

            let pattern_like = format!("{}%", pattern);
            let pattern_deep = format!("{}%/%", pattern);

            let mut stmt = conn.prepare(
                "SELECT path, is_dir, mode, size, mod_time FROM files WHERE path LIKE ?1 AND path != ?2 AND path NOT LIKE ?3"
            ).map_err(|e| EvifError::InvalidPath(format!("prepare failed: {}", e)))?;

            let rows = stmt.query_map(
                [&pattern_like, &path, &pattern_deep],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, u32>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            let mut files = Vec::new();
            for row in rows {
                let (full_path, is_dir, mode, size, mod_time) = row
                    .map_err(|e| EvifError::InvalidPath(format!("row failed: {}", e)))?;

                let name = if full_path == "/" {
                    "/".to_string()
                } else {
                    full_path.split('/').next_back().unwrap_or(&full_path).to_string()
                };

                files.push(FileInfo {
                    name,
                    size: size as u64,
                    mode,
                    modified: DateTime::from_timestamp(mod_time, 0).unwrap_or_default(),
                    is_dir: is_dir == 1,
                });
            }

            Ok::<Vec<FileInfo>, EvifError>(files)
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Cache the result
        self.cache.write().await.put(path_for_cache, files.clone());

        Ok(files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let path = normalize_path(path);
        let db_path = self.db_path.clone();
        let path_clone = path.clone();

        let info = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            let (is_dir, mode, size, mod_time): (i64, u32, i64, i64) = conn.query_row(
                "SELECT is_dir, mode, size, mod_time FROM files WHERE path = ?1",
                [&path_clone],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    EvifError::NotFound(path_clone.clone())
                } else {
                    EvifError::InvalidPath(format!("query failed: {}", e))
                }
            })?;

            let name = if path_clone == "/" {
                "/".to_string()
            } else {
                path.split('/').next_back().unwrap_or(&path).to_string()
            };

            let info = FileInfo {
                name,
                size: size as u64,
                mode,
                modified: DateTime::from_timestamp(mod_time, 0).unwrap_or_default(),
                is_dir: is_dir == 1,
            };

            Ok::<FileInfo, EvifError>(info)
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        Ok(info)
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let path = normalize_path(path);

        if path == "/" {
            return Err(EvifError::InvalidPath("cannot remove root directory".to_string()));
        }

        let db_path = self.db_path.clone();
        let path_clone = path.clone();
        let path_for_error = path.clone();
        let path_for_cache = path.clone();

        let is_dir = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Check if file exists and is not a directory
            let is_dir: i64 = conn.query_row(
                "SELECT is_dir FROM files WHERE path = ?1",
                [&path_clone],
                |row| row.get(0),
            ).map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    EvifError::NotFound(path_for_error)
                } else {
                    EvifError::InvalidPath(format!("query failed: {}", e))
                }
            })?;

            if is_dir == 1 {
                // Check if directory is empty
                let pattern = format!("{}/%", path);
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM files WHERE path LIKE ?1 AND path != ?2",
                    [&pattern, &path],
                    |row| row.get(0),
                ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

                if count > 0 {
                    return Err(EvifError::InvalidPath(format!("directory not empty: {}", path)));
                }
            }

            // Delete file
            conn.execute(
                "DELETE FROM files WHERE path = ?1",
                [&path],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to delete: {}", e)))?;

            Ok::<i64, EvifError>(is_dir)
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Invalidate cache
        let mut cache = self.cache.write().await;
        cache.invalidate_parent(&path_for_cache);
        if is_dir == 1 {
            cache.invalidate(&path_for_cache);
        }

        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let old_path = normalize_path(old_path);
        let new_path = normalize_path(new_path);

        if old_path == "/" || new_path == "/" {
            return Err(EvifError::InvalidPath("cannot rename root directory".to_string()));
        }

        let db_path = self.db_path.clone();
        let old_path_clone = old_path.clone();
        let new_path_clone = new_path.clone();
        let old_path_for_children = old_path.clone();
        let old_path_for_error = old_path.clone();
        let new_path_for_error = new_path.clone();
        let old_path_for_cache = old_path.clone();
        let new_path_for_cache = new_path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Check if old path exists
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                [&old_path_clone],
                |row| row.get(0),
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            if count == 0 {
                return Err(EvifError::NotFound(old_path_for_error));
            }

            // Check if new path already exists
            let new_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                [&new_path_clone],
                |row| row.get(0),
            ).map_err(|e| EvifError::InvalidPath(format!("query failed: {}", e)))?;

            if new_exists > 0 {
                return Err(EvifError::InvalidPath(format!("file already exists: {}", new_path_for_error)));
            }

            // Rename file/directory
            conn.execute(
                "UPDATE files SET path = ?1 WHERE path = ?2",
                [&new_path, &old_path],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to rename: {}", e)))?;

            // If it's a directory, rename all children
            let pattern = format!("{}/%", old_path_for_children);
            conn.execute(
                "UPDATE files SET path = ?1 || SUBSTR(path, ?2) WHERE path LIKE ?3",
                [&new_path, &(old_path.len() + 1).to_string(), &pattern],
            ).map_err(|e| EvifError::InvalidPath(format!("failed to rename children: {}", e)))?;

            Ok::<(), EvifError>(())
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Invalidate cache
        let mut cache = self.cache.write().await;
        cache.invalidate_parent(&old_path_for_cache);
        cache.invalidate_parent(&new_path_for_cache);
        cache.invalidate(&old_path_for_cache);
        cache.invalidate_prefix(&old_path_for_cache);

        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let path = normalize_path(path);
        let db_path = self.db_path.clone();
        let path_clone = path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)
                .map_err(|e| EvifError::InvalidPath(format!("failed to open database: {}", e)))?;

            // Use batched deletion (1000 at a time)
            const BATCH_SIZE: i64 = 1000;

            if path_clone == "/" {
                // Delete all children but not root itself
                loop {
                    let result = conn.execute(
                        "DELETE FROM files WHERE rowid IN (SELECT rowid FROM files WHERE path != '/' LIMIT ?1)",
                        [BATCH_SIZE],
                    ).map_err(|e| EvifError::InvalidPath(format!("delete failed: {}", e)))?;

                    if result == 0 {
                        break;
                    }
                }
            } else {
                // Delete file and all children in batches
                let pattern = format!("{}/%", path_clone);
                loop {
                    let result = conn.execute(
                        "DELETE FROM files WHERE rowid IN (SELECT rowid FROM files WHERE (path = ?1 OR path LIKE ?2) LIMIT ?3)",
                        [&path_clone as &str, &pattern as &str, &(BATCH_SIZE as usize).to_string()],
                    ).map_err(|e| EvifError::InvalidPath(format!("delete failed: {}", e)))?;

                    if result == 0 {
                        break;
                    }
                }
            }

            Ok::<(), EvifError>(())
        }).await
        .map_err(|e| EvifError::InvalidPath(format!("spawn_blocking error: {}", e)))??;

        // Invalidate cache
        if path == "/" {
            self.cache.write().await.clear();
        } else {
            let mut cache = self.cache.write().await;
            cache.invalidate_parent(&path);
            cache.invalidate_prefix(&path);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_basic() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create directory
        plugin.mkdir("/test", 0o755).await.unwrap();

        // Check if exists
        let info = plugin.stat("/test").await.unwrap();
        assert_eq!(info.name, "test");
        assert!(info.is_dir);

        // List directory
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "test"));

        // Remove
        plugin.remove_all("/test").await.unwrap();

        // Verify removed
        let result = plugin.stat("/test").await;
        assert!(result.is_err());
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_file_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create file
        plugin.create("/test.txt", 0o644).await.unwrap();

        // Write data
        let data = b"Hello, World!".to_vec();
        plugin.write("/test.txt", data.clone(), 0, WriteFlags::CREATE).await.unwrap();

        // Read data
        let read_data = plugin.read("/test.txt", 0, 0).await.unwrap();
        assert_eq!(read_data, data);

        // Stat
        let info = plugin.stat("/test.txt").await.unwrap();
        assert_eq!(info.size, data.len() as u64);
        assert!(!info.is_dir);
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_readdir() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create directory structure
        plugin.mkdir("/dir1", 0o755).await.unwrap();
        plugin.mkdir("/dir2", 0o755).await.unwrap();
        plugin.create("/dir1/file1.txt", 0o644).await.unwrap();
        plugin.create("/dir1/file2.txt", 0o644).await.unwrap();

        // List root
        let entries = plugin.readdir("/").await.unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "dir1"));
        assert!(entries.iter().any(|e| e.name == "dir2"));

        // List dir1
        let files = plugin.readdir("/dir1").await.unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.name == "file1.txt"));
        assert!(files.iter().any(|f| f.name == "file2.txt"));
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_rename() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create file
        plugin.create("/old.txt", 0o644).await.unwrap();
        plugin.write("/old.txt", b"data".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // Rename
        plugin.rename("/old.txt", "/new.txt").await.unwrap();

        // Check old doesn't exist
        let result = plugin.stat("/old.txt").await;
        assert!(result.is_err());

        // Check new exists
        let info = plugin.stat("/new.txt").await.unwrap();
        assert_eq!(info.name, "new.txt");
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_remove_all() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create directory structure
        plugin.mkdir("/dir", 0o755).await.unwrap();
        plugin.create("/dir/file1.txt", 0o644).await.unwrap();
        plugin.create("/dir/file2.txt", 0o644).await.unwrap();
        plugin.mkdir("/dir/subdir", 0o755).await.unwrap();
        plugin.create("/dir/subdir/file3.txt", 0o644).await.unwrap();

        // Remove all
        plugin.remove_all("/dir").await.unwrap();

        // Verify all removed
        let result = plugin.stat("/dir").await;
        assert!(result.is_err());
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_execute_query() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("query_test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create files to query
        plugin.mkdir("/data", 0o755).await.unwrap();
        plugin.create("/data/file1.txt", 0o644).await.unwrap();
        plugin.write("/data/file1.txt", b"Hello World".to_vec(), 0, WriteFlags::CREATE).await.unwrap();
        plugin.create("/data/file2.txt", 0o644).await.unwrap();
        plugin.write("/data/file2.txt", b"Hello Rust".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // Execute a SELECT query against the files table
        let result = plugin.execute_query(
            "SELECT path, is_dir, size FROM files WHERE path LIKE '/data/%' AND is_dir = 0 ORDER BY path"
        ).await.unwrap();

        // Verify JSON results
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert!(arr[0]["path"].as_str().unwrap().contains("file1.txt"));
        assert!(arr[1]["path"].as_str().unwrap().contains("file2.txt"));
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_saved_queries() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("saved_query_test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // Create files
        plugin.mkdir("/test", 0o755).await.unwrap();
        plugin.create("/test/a.txt", 0o644).await.unwrap();
        plugin.create("/test/b.txt", 0o644).await.unwrap();

        // Save a query
        plugin.save_query("list_test_files", "SELECT path, size FROM files WHERE path LIKE '/test/%' AND is_dir = 0").await.unwrap();

        // List saved queries
        let queries = plugin.list_saved_queries().await.unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].0, "list_test_files");
        assert!(queries[0].1.contains("SELECT"));

        // Execute the saved query's SQL
        let result = plugin.execute_query(&queries[0].1).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);

        // Delete the saved query
        plugin.delete_query("list_test_files").await.unwrap();
        let queries = plugin.list_saved_queries().await.unwrap();
        assert!(queries.is_empty());
    }

    #[cfg(feature = "sqlfs")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlfs_query_only_select_allowed() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("query_safety_test.db");
        let config = SqlfsConfig {
            db_path: db_path.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let plugin = SqlfsPlugin::new(config).unwrap();

        // INSERT should be rejected
        let result = plugin.execute_query("INSERT INTO files VALUES ('/hack', 0, 644, 0, 0, '')").await;
        assert!(result.is_err());

        // DELETE should be rejected
        let result = plugin.execute_query("DELETE FROM files WHERE path = '/'").await;
        assert!(result.is_err());

        // SELECT should work
        let result = plugin.execute_query("SELECT COUNT(*) as cnt FROM files").await;
        assert!(result.is_ok());
    }
}
