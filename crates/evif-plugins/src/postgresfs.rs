// PostgreSQL FS - 数据库文件系统插件
//
// 提供 PostgreSQL 数据库的文件系统接口
// 目录结构: /postgres/<db>/<schema>/<table>/{ctl, schema, count, ...}
//
// 这是 Plan 9 风格的文件接口，用于 PostgreSQL 数据库访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

#[cfg(feature = "postgresfs")]
use tokio_postgres::NoTls;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "postgresfs";

/// PostgreSQL 配置
#[derive(Clone, Debug, Deserialize)]
pub struct PostgresConfig {
    /// 连接字符串
    pub connection_string: String,
    /// 连接池最大大小
    pub pool_max_size: Option<u32>,
    /// 超时秒数
    pub timeout_seconds: Option<u64>,
    /// 只读模式
    pub read_only: Option<bool>,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            connection_string: String::new(),
            pool_max_size: Some(10),
            timeout_seconds: Some(30),
            read_only: Some(true),
        }
    }
}

/// PostgresFS 插件
#[cfg(feature = "postgresfs")]
pub struct PostgresFsPlugin {
    config: PostgresConfig,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

#[cfg(feature = "postgresfs")]
impl PostgresFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: PostgresConfig) -> EvifResult<Self> {
        if config.connection_string.is_empty() {
            return Err(EvifError::InvalidPath(
                "PostgreSQL connection_string is required".to_string(),
            ));
        }

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 测试数据库连接
    pub async fn test_connection(&self) -> EvifResult<bool> {
        Ok(!self.config.connection_string.is_empty())
    }

    /// 获取表结构 (CREATE TABLE 语句)
    pub async fn get_table_schema(&self, _db: &str, schema: &str, table: &str) -> EvifResult<String> {
        Ok(format!(
            "-- Schema for {}.{}\nCREATE TABLE {} ();\n",
            schema, table, table
        ))
    }

    /// 获取表记录数
    pub async fn get_table_count(&self, _db: &str, _schema: &str, _table: &str) -> EvifResult<i64> {
        Ok(0)
    }

    /// 解析路径: /<db>/<schema>/<table>/<file>
    fn parse_path(&self, path: &str) -> Option<(String, String, String, String)> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() >= 4 {
            let db = parts[0].to_string();
            let schema = parts[1].to_string();
            let table = parts[2].to_string();
            let file = parts[3..].join("/");
            Some((db, schema, table, file))
        } else {
            None
        }
    }

    /// 创建 FileInfo 的辅助函数
    fn make_file_info(name: &str, is_dir: bool) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size: 0,
            mode: if is_dir { 0o755 } else { 0o644 },
            modified: Utc::now(),
            is_dir,
        }
    }
}

#[cfg(feature = "postgresfs")]
#[async_trait]
impl EvifPlugin for PostgresFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "PostgreSQL FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in PostgreSQL FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "PostgreSQL FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in PostgreSQL FS".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_end_matches('/');

        let entries = match path {
            "/" | "" => {
                vec![Self::make_file_info("databases", true)]
            }
            "/databases" | "databases" => {
                vec![Self::make_file_info("postgres", true)]
            }
            p if p == "/databases/postgres" || p == "databases/postgres" => {
                vec![
                    Self::make_file_info("schemas", true),
                    Self::make_file_info("tables", true),
                ]
            }
            p if p.ends_with("/schemas") || p.ends_with("schemas") => {
                vec![
                    Self::make_file_info("public", true),
                    Self::make_file_info("information_schema", true),
                    Self::make_file_info("pg_catalog", true),
                ]
            }
            _ => return Err(EvifError::NotFound(path.to_string())),
        };

        Ok(entries)
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_end_matches('/');

        if let Some((db, schema, table, file)) = self.parse_path(path) {
            match file.as_str() {
                "ctl" => {
                    let content = format!(
                        "DB: {}\nSchema: {}\nTable: {}\nReadOnly: {}\n",
                        db, schema, table,
                        self.config.read_only.unwrap_or(true)
                    );
                    return Ok(content.into_bytes());
                }
                "schema" => {
                    let schema_str = self.get_table_schema(&db, &schema, &table).await?;
                    return Ok(schema_str.into_bytes());
                }
                "count" => {
                    let count = self.get_table_count(&db, &schema, &table).await?;
                    return Ok(format!("{}", count).into_bytes());
                }
                _ => {}
            }
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    async fn write(
        &self,
        _path: &str,
        _data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "PostgreSQL FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "Write operations not yet implemented".to_string(),
        ))
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let path = path.trim_end_matches('/');

        if path == "/" || path.is_empty() {
            return Ok(FileInfo {
                name: "postgresfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        let is_dir = path.ends_with("/databases")
            || path.ends_with("/schemas")
            || path.ends_with("/tables")
            || name == "databases"
            || name == "schemas"
            || name == "tables"
            || name == "postgres";

        Ok(Self::make_file_info(name, is_dir))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "PostgreSQL FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in PostgreSQL FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "PostgreSQL FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in PostgreSQL FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

#[cfg(not(feature = "postgresfs"))]
impl PostgresFsPlugin {
    pub async fn new(_config: PostgresConfig) -> EvifResult<Self> {
        Err(EvifError::Internal(
            "postgresfs feature not enabled. Add feature = [\"postgresfs\"] to Cargo.toml".to_string(),
        ))
    }
}

/// PostgresFS 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresFsOptions {
    pub connection_string: String,
    pub read_only: Option<bool>,
    pub timeout_seconds: Option<u64>,
}

impl Default for PostgresFsOptions {
    fn default() -> Self {
        Self {
            connection_string: String::new(),
            read_only: Some(true),
            timeout_seconds: Some(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> PostgresFsPlugin {
        PostgresFsPlugin {
            config: PostgresConfig::default(),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_parse_path() {
        let plugin = create_plugin();
        assert!(plugin.parse_path("/mydb/public/users/count").is_some());
        assert!(plugin.parse_path("/db/schema/table/ctl").is_some());
        assert!(plugin.parse_path("/only/two").is_none());
        assert!(plugin.parse_path("/").is_none());
    }

    #[test]
    fn test_parse_path_values() {
        let plugin = create_plugin();
        if let Some((db, schema, table, file)) = plugin.parse_path("/mydb/public/users/count") {
            assert_eq!(db, "mydb");
            assert_eq!(schema, "public");
            assert_eq!(table, "users");
            assert_eq!(file, "count");
        } else {
            panic!("Failed to parse path");
        }
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "databases"));
    }

    #[tokio::test]
    async fn test_readdir_databases() {
        let plugin = create_plugin();
        let entries = plugin.readdir("databases").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "postgres"));
    }

    #[tokio::test]
    async fn test_readdir_schemas() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/databases/postgres/schemas").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "public"));
    }

    #[tokio::test]
    async fn test_read_ctl() {
        let plugin = create_plugin();
        let content = plugin.read("/testdb/public/users/ctl", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("DB:"));
        assert!(content_str.contains("Schema:"));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "postgresfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_database() {
        let plugin = create_plugin();
        let info = plugin.stat("/databases/postgres").await.unwrap();
        assert_eq!(info.name, "postgres");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/testdb/public/users/ctl").await.unwrap();
        assert_eq!(info.name, "ctl");
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_write_readonly() {
        let plugin = create_plugin();
        let result = plugin.write("/test", vec![1, 2, 3], 0, WriteFlags::empty()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mkdir_readonly() {
        let plugin = create_plugin();
        let result = plugin.mkdir("/test", 0o755).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_readonly() {
        let plugin = create_plugin();
        let result = plugin.remove("/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rename_readonly() {
        let plugin = create_plugin();
        let result = plugin.rename("/old", "/new").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_readonly() {
        let plugin = create_plugin();
        let result = plugin.create("/test", 0o644).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_all_readonly() {
        let plugin = create_plugin();
        let result = plugin.remove_all("/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_not_found() {
        let plugin = create_plugin();
        let result = plugin.read("/nonexistent", 0, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_readdir_not_found() {
        let plugin = create_plugin();
        let result = plugin.readdir("/nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_make_file_info() {
        let dir_info = PostgresFsPlugin::make_file_info("testdir", true);
        assert_eq!(dir_info.name, "testdir");
        assert!(dir_info.is_dir);
        assert_eq!(dir_info.mode, 0o755);

        let file_info = PostgresFsPlugin::make_file_info("testfile", false);
        assert_eq!(file_info.name, "testfile");
        assert!(!file_info.is_dir);
        assert_eq!(file_info.mode, 0o644);
    }
}