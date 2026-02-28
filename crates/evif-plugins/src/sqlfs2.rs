// EVIF SQLFS2 Plugin - Plan 9 Style SQL Interface
//
// Directory structure: /sqlfs2/<dbName>/<tableName>/{ctl, schema, count, <sid>/...
//
// Based on AGFS SQLFS2 implementation adapted for EVIF architecture

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;

use evif_core::{
    EvifError, EvifResult, FileInfo, FileSystem,
    OpenFlags,
};

const PLUGIN_NAME: &str = "sqlfs2";

// ============================================================================
// Session Management
// ============================================================================

/// Session represents a Plan 9 style session for SQL operations
#[derive(Debug, Clone)]
struct Session {
    id: i64,
    db_name: String,
    table_name: String,
    result: Vec<u8>,
    last_error: String,
    last_access: std::time::Instant,
}

/// SessionManager manages all active SQL sessions
#[derive(Debug)]
struct SessionManager {
    sessions: RwLock<HashMap<String, Session>>,
    next_id: std::sync::atomic::AtomicI64,
    timeout: Option<std::time::Duration>,
}

impl SessionManager {
    fn new(timeout: Option<std::time::Duration>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            next_id: std::sync::atomic::AtomicI64::new(1),
            timeout,
        }
    }

    async fn create_session(
        &self,
        db_name: String,
        table_name: String,
    ) -> EvifResult<Session> {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let session = Session {
            id,
            db_name: db_name.clone(),
            table_name: table_name.clone(),
            result: Vec::new(),
            last_error: String::new(),
            last_access: std::time::Instant::now(),
        };

        let key = format!("{}/{}/{}", db_name, table_name, id);
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(key, session);
        }

        Ok(session)
    }

    async fn get_session(&self, db_name: &str, table_name: &str, sid: &str) -> Option<Session> {
        let key = format!("{}/{}/{}", db_name, table_name, sid);
        let sessions = self.sessions.read().await;
        sessions.get(&key).cloned()
    }

    async fn update_session_result(&self, db_name: &str, table_name: &str, sid: &str, result:
 Vec<u8>) {
        let key = format!("{}/{}/{}", db_name, table_name, sid);
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&key) {
            session.result = result;
            session.last_access = std::time::Instant::now();
        }
    }

    async fn update_session_error(&self, db_name: &str, table_name: &str, sid: &str, error: String) {
        let key = format!("{}/{}/{}", db_name, table_name, sid);
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&key) {
            session.last_error = error;
            session.last_access = std::time::Instant::now();
        }
    }

    async fn close_session(&self, db_name: &str, table_name: &str, sid: &str) -> EvifResult<()> {
        let key = format!("{}/{}/{}", db_name, table_name, sid);
        let mut sessions = self.sessions.write().await;
        sessions.remove(&key)
            .ok_or_else(|| EvifError::NotFound(format!("session not found: {}", sid)))?;
        Ok(())
    }

    async fn list_sessions(&self, db_name: &str, table_name: &str) -> Vec<String> {
        let prefix = format!("{}/{}/", db_name, table_name);
        let sessions = self.sessions.read().await;

        sessions.keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| k.trim_start_matches(&prefix).to_string())
            .collect()
    }
}

// ============================================================================
// SQL Backend Trait
// ============================================================================

/// Backend trait for different SQL databases
#[async_trait]
trait SqlBackend: Send + Sync {
    async fn list_databases(&self) -> EvifResult<Vec<String>>;
    async fn list_tables(&self, db_name: &str) -> EvifResult<Vec<String>>;
    async fn get_table_schema(&self, db_name: &str, table_name: &str) -> EvifResult<String>;
    async fn execute_query(&self, db_name: &str, sql: &str) -> EvifResult<QueryResult>;
    async fn execute_statement(&self, db_name: &str, sql: &str) -> EvifResult<StatementResult>;
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryResult {
    rows: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatementResult {
    rows_affected: u64,
    last_insert_id: i64,
}

// ============================================================================
// SQLite Backend Implementation
// ============================================================================

struct SQLiteBackend {
    db_path: String,
}

#[async_trait]
impl SqlBackend for SQLiteBackend {
    async fn list_databases(&self) -> EvifResult<Vec<String>> {
        // SQLite doesn't have multiple databases in same file
        // Return main file as "database"
        Ok(vec!["main".to_string()])
    }

    async fn list_tables(&self, _db_name: &str) -> EvifResult<Vec<String>> {
        // Use rusqlite2 (rusqlite package)
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| EvifError::Internal(format!("Failed to open database: {}", e)))?;

        let mut stmt = {
            use rusqlite::{params, Statement};
            conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .map_err(|e| EvifError::Internal(format!("Failed to prepare statement: {}", e)))?
        };

        let mut tables = Vec::new();
        while let rusqlite::State::Row = stmt.next().map_err(|e| EvifError::Internal(format!("Query error: {}", e)))? {
            use rusqlite::Row;
            let tables_ref = &mut tables;
            let name = stmt.read::<String>(0)
                .map_err(|e| EvifError::Internal(format!("Failed to read: {}", e)))?;
            tables_ref.push(name);
        }

        Ok(tables)
    }

    async fn get_table_schema(&self, _db_name: &str, table_name: &str) -> EvifResult<String> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| EvifError::Internal(format!("Failed to open database: {}", e)))?;

        let sql = format!("SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'", table_name);
        let mut stmt = conn.prepare(&sql)
            .map_err(|e| EvifError::Internal(format!("!Failed to prepare statement: {}", e)))?;

        match stmt.next().map_err(|e| EvifError::Internal(format!("Query error: {}", e)))? {
            rusqlite::State::Row => {
                let schema = stmt.read::<String>(0)
                    .map_err(|e| EvifError::Internal(format!("Failed to read: {}", e)))?;
                Ok(schema)
            }
            _ => Ok(format!("-- Table {} not found", table_name)),
        }
    }

    async fn execute_query(&self, _db_name: &str, sql: &str) -> EvifResult<QueryResult> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| EvifError::Internal(format!("Failed to open database: {}", e)))?;

        let mut stmt = conn.prepare(sql)
            .map_err(|e| EvifError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let mut rows = Vec::new();

        // Get column names
        let column_count = stmt.column_count();
        let mut column_names = Vec::new();
        for i in 0..column_count {
            let name = stmt.column_name(i).unwrap_or_else(|_| format!("col{}", i));
            column_names.push(name);
        }

        while let rusqlite::State::rRow = stmt.next().map_err(|e| EvifError::Internal(format!("Query error: {}", e)))? {
            let mut row = HashMap::new();
            for (i, col_name) in column_names.iter().enumerate() {
                let value: serde_json::Value = match stmt.read::<Option<String>>(i) {
                    Ok(Some(val)) => serde_json::json!(val),
                    Ok(None) => serde_json::Value::Null,
                    Err(_) => serde_json::Value::Null,
                };
                row.insert(col_name.clone(), value);
            }
            rows.push(row);
        }

        Ok(QueryResult { rows })
    }

    async fn execute_statement(&self, _db_name: &str, sql: &str) -> EvifResult<StatementResult> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| EvifError::Internal(format!("Failed to open database: {}", e)))?;

        let rows_affected = conn.execute(sql, rusqlite::params![])
            .map_err(|e| EvifError::Internal(format!("Execution error: {}", e)))?;

        // SQLite doesn't provide rows_affected or last_insert_id directly
        Ok(StatementResult {
            rows_affected: rows_affected as u64,
            last_insert_id: conn.last_insert_rowid(),
        })
    }
}

// ============================================================================
// SQLFS2 Plugin
// ============================================================================

pub struct SqlFS2Plugin {
    backend: Box<dyn SqlBackend>,
    session_manager: SessionManager,
}

impl SqlFS2Plugin {
    pub fn new(backend: Box<dyn SqlBackend>) -> Self {
        Self {
            backend,
            session_manager: SessionManager::new(None),
        }
    }

    /// Parse path into components (dbName, tableName, sid, operation)
    fn parse_path(&self, path: &str) -> EvifResult<(String, String, String, String)> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        match parts.len() {
            0 => Ok((String::new(), String::new(), String::new(), String::new())),
            1 => {
                // Could be /ctl, /<sid>, or /dbName
                let part = parts[0];
                if part == "ctl" {
                    Ok((String::new(), String::new(), String::new(), "ctl".to_string()))
                } else if part.chars().all(|c| c.is_numeric()) {
                    Ok((String::new(), String::new(), part.to_string(), String::new()))
                } else {
                    Ok((part.to_string(), String::new(), String::new(), String::new()))
                }
            }
            2 => {
                // Could be /dbName/ctl, /dbName/<sid>, or /dbName/tableName
                let (db_name, part2) = (parts[0], parts[1]);

                if part2 == "ctl" {
                    Ok((db_name.to_string(), String::new(), String::new(), "ctl".to_string()))
                } else if part2 == "schema" {
                    Ok((db_name.to_string(), String::new(), String::new(), "schema".to_string()))
                } else if part2 == "count" {
                    Ok((db_name.to_string(), String::new(), String::new(), "count".to_string()))
                } else if part2.chars().all(|c| c.is_numeric()) {
                    Ok((db_name.to_string(), String::new(), part2.to_string(), String::new()))
                } else {
                    Ok((db_name.to_string(), part2.to_string(), String::new(), String::new()))
                }
            }
            3 => {
                // /dbName/tableName/ctl, /dbName/tableName/schema, /dbName/tableName/count, or /dbName/tableName/<sid>
                let (db_name, table_name, part3) = (parts[0], parts[1], parts[2]);

                if part3 == "ctl" || part3 == "schema" || part3 == "count" {
                    Ok((db_name.to_string(), table_name.to_string(), String::new(), part3.to_string()))
                } else if part3.chars().all(|c| c.is_numeric()) {
                    Ok((db_name.to_string(), table_name.to_string(), part3.to_string(), String::new()))
                } else {
                    Err(EvifError::NotFound(format!("Invalid path: {}", path)))
                }
            }
            4 => {
                // /dbName/tableName/<sid>/operation
                let (db_name, table_name, sid, operation) = (parts[0], parts[1], parts[2], parts[3]);
                Ok((db_name.to_string(), table_name.to_string(), sid.to_string(), operation.to_string()))
            }
            _ => Err(EvifError::NotFound(format!("Invalid path: {}", path))),
        }
    }

    fn is_session_id(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_numeric())
    }
}

#[async_trait]
impl FileSystem for SqlFS2Plugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let (db_name, table_name, sid, operation) = self.parse_path(path)?;

        // Table-level operations
        if !table_name.is_empty() && sid.is_empty() {
            match operation.as_str() {
                "ctl" => {
                    // Create new session and return session ID
                    let session = self.session_manager.create_session(
                        db_name.clone(),
                        table_name.clone(),
                    ).await?;
                    Ok(format!("{}\n", session.id).into_bytes())
                }
                "schema" => {
                    let schema = self.backend.get_table_schema(&db_name, &table_name).await?;
                    Ok(format!("{}\n", schema).into_bytes())
                }
                "count" => {
                    let sql = format!("SELECT COUNT(*) FROM {}.{}", db_name, table_name);
                    let result = self.backend.execute_query(&db_name, &sql).await?;
                    let count = result.rows.len();
                    Ok(format!("{}\n", count).into_bytes())
                }
                _ => Err(EvifError::InvalidArgument(format!("Unknown operation: {}", operation))),
            }
        }
        // Session-level operations
        else if !sid.is_empty() {
            let session = self.session_manager.get_session(&db_name, &table_name, &sid)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Session not found: {}", sid)))?;

            match operation.as_str() {
                "result" => Ok(session.result.clone()),
                "error" => {
                    if session.last_error.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(format!("{}\n", session.last_error).into_bytes())
                    }
                }
                _ => Err(EvifError::InvalidArgument(format!("{} is write-only", operation))),
            }
        }
        // Root/database-level
        else {
            Err(EvifError::InvalidArgument(format!("Cannot read directory: {}", path)))
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64) -> EvifResult<u64> {
        let (db_name, table_name, sid, operation) = self.parse_path(path)?;

        if sid.is_empty() {
            return Err(EvifError::InvalidArgument("Cannot write to non-session path".to_string()));
        }

        match operation.as_str() {
            "query" => {
                let sql = String::from_utf8_lossy(&data);

                let upper_sql = sql.trim().to_uppercase();
                let is_select = upper_sql.starts_with("SELECT")
                    || upper_sql.starts_with("SHOW")
                    || upper_sql.starts_with("DESCRIBE")
                    || upper_sql.starts_with("EXPLAIN");

                if is_select {
                    match self.backend.execute_query(&db_name, &sql).await {
                        Ok(result) => {
                            let json = serde_json::to_string_pretty(&result)
                                .map_err(|e| EvifError::Internal(format!("JSON error: {}", e)))?;
                            // Update session result
                            self.session_manager.update_session_result(&db_name, &table_name, &sid, json.into_bytes()).await;
                            Ok(data.len() as u64)
                        }
                        Err(e) => {
                            // Update session error
                            self.session_manager.update_session_error(&db_name, &table_name, &sid, e.to_string()).await;
                            Ok(data.len() as u64)
                        }
                    }
                } else {
                    match self.backend.execute_statement(&db_name, &sql).await {
                        Ok(result) => {
                            let json = serde_json::to_string_pretty(&result)
                                .map_err(|e| EvifError::Internal(format!("JSON error: {}", e)))?;
                            // Update session result
                            self.session_manager.update_session_result(&db_name, &table_name, &sid, json.into_bytes()).await;
                            Ok(data.len() as u64)
                        }
                        Err(e) => {
                            // Update session error
                            self.session_manager.update_session_error(&db_name, &table_name, &sid, e.to_string()).await;
                            Ok(data.len() as u64)
                        }
                    }
                }
            }
            "ctl" => {
                let cmd = String::from_utf8_lossy(&data);
                if cmd.trim() == "close" {
                    self.session_manager.close_session(&db_name, &table_name, &sid).await?;
                    Ok(data.len() as u64)
                } else {
                    Err(EvifError::InvalidArgument(format!("Unknown command: {}", cmd)))
                }
            }
            _ => Err(EvifError::InvalidArgument(format!("{} is read-only", operation))),
        }
    }

    async fn list(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let (db_name, table_name, sid, operation) = self.parse_path(path)?;

        let mut files = Vec::new();

        // Root directory
        if db_name.is_empty() && table_name.is_empty() && sid.is_empty() && operation.is_empty() {
            files.push(FileInfo {
                name: "ctl".to_string(),
                path: format!("{}/ctl", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "ctl".to_string(),
            });

            // List databases
            let dbs = self.backend.list_databases().await?;
            for db in dbs {
                files.push(FileInfo {
                    name: db.clone(),
                    path: format!("{}/{}", path.trim_end_matches('/'), db),
                    size: 0,
                    modified: chrono::Utc::now().timestamp(),
                    is_dir: true,
                    file_type: "database".to_string(),
                });
            }
        }
        // Table directory
        else if !table_name.is_empty() && sid.is_empty() && operation.is_empty() {
            files.push(FileInfo {
                name: "ctl".to_string(),
                path: format!("{}/ctl", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "ctl".to_string(),
            });

            files.push(FileInfo {
                name: "schema".to_string(),
                path: format!("{}/schema", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "schema".to_string(),
            });

            files.push(FileInfo {
                name: "count".to_string(),
                path: format!("{}/count", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "count".to_string(),
            });

            // List sessions
            let sessions = self.session_manager.list_sessions(&db_name, &table_name).await;
            for session_id in sessions {
                files.push(FileInfo {
                    name: session_id.clone(),
                    path: format!("{}/{}", path.trim_end_matches('/'), session_id),
                    size: 0,
                    modified: chrono::Utc::now().timestamp(),
                    is_dir: true,
                    file_type: "session".to_string(),
                });
            }
        }
        // Session directory
        else if !sid.is_empty() && operation.is_empty() {
            files.push(FileInfo {
                name: "ctl".to_string(),
                path: format!("{}/ctl", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "session-ctl".to_string(),
            });

            files.push(FileInfo {
                name: "query".to_string(),
                path: format!("{}/query", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "query".to_string(),
            });

            files.push(FileInfo {
                name: "result".to_string(),
                path: format!("{}/result", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "result".to_string(),
            });

            files.push(FileInfo {
                name: "error".to_string(),
                path: format!("{}/error", path.trim_end_matches('/')),
                size: 0,
                modified: chrono::Utc::now().timestamp(),
                is_dir: false,
                file_type: "error".to_string(),
            });
        } else {
            return Err(EvifError::NotFound(format!("Not a directory: {}", path)));
        }

        Ok(files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let (db_name, table_name, sid, operation) = self.parse_path(path)?;

        let now = chrono::Utc::now().timestamp();

        // Directory cases
        if operation.is_empty() {
            Ok(FileInfo {
                name: path.split('/').last().unwrap_or("").to_string(),
                path: path.to_string(),
                size: 0,
                modified: now,
                is_dir: true,
                file_type: if table_name.is_empty() { "database" } else { "session" }.to_string(),
            })
        } else {
            Ok(FileInfo {
                name: operation.clone(),
                path: path.to_string(),
                size: 0,
                modified: now,
                is_dir: false,
                file_type: operation.clone(),
            })
        }
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let (db_name, table_name, sid, operation) = self.parse_path(path)?;

        if !operation.is_empty() {
            return Err(EvifError::InvalidArgument("Cannot remove file, only directories".to_string()));
        }

        // Remove session
        if !sid.is_empty() {
            self.session_manager.close_session(&db_name, &table_name, &sid).await?;
        } else {
            return Err(EvifError::InvalidArgument("Cannot remove database or table".to_string()));
        }

        Ok(())
    }

    async fn mkdir(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupported("Mkdir not supported in SQLFS2".to_string()))
    }

    async fn rmdir(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupported("Rmdir not supported in SQLFS2".to_string()))
    }
}

// ============================================================================
// Plugin Registration
// ============================================================================

/// Create a new SQLFS2 plugin instance with SQLite backend
pub fn create_sqlfs2_plugin(db_path: String) -> Box<dyn FileSystem> {
    let backend = Box::new(SQLiteBackend { db_path }) as Box<dyn SqlBackend>;
    Box::new(SqlFS2Plugin::new(backend))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_path() {
        let backend = Box::new(SQLiteBackend {
            db_path: ":memory:".to_string()
        }) as Box<dyn SqlBackend>;
        let plugin = SqlFS2Plugin::new(backend);

        // Test root
        let (db, table, sid, op) = plugin.parse_path("/").unwrap();
        assert_eq!(db, "");
        assert_eq!(table, "");
        assert_eq!(sid, "");
        assert_eq!(op, "");

        // Test table level
        let (db, table, sid, op) = plugin.parse_path("/mydb/users/ctl").unwrap();
        assert_eq!(db, "mydb");
        assert_eq!(table, "users");
        assert_eq!(sid, "");
        assert_eq!(op, "ctl");

        // Test session level
        let (db, table, sid, op) = plugin.parse_path("/mydb/users/123/query").unwrap();
        assert_eq!(db, "mydb");
        assert_eq!(table, "users");
        assert_eq!(sid, "123");
        assert_eq!(op, "query");
    }
}
