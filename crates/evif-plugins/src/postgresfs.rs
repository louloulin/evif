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
        if self.config.connection_string.is_empty() {
            return Ok(false);
        }

        // 尝试连接数据库
        let (client, connection) = tokio_postgres::connect(
            &self.config.connection_string,
            NoTls,
        ).await.map_err(|e| EvifError::Internal(format!("Connection failed: {}", e)))?;

        // spawn the connection handling task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        // 执行一个简单的查询来验证连接
        let _rows = client.query("SELECT 1", &[]).await
            .map_err(|e| EvifError::Internal(format!("Query failed: {}", e)))?;

        Ok(true)
    }

    /// 列出所有数据库
    async fn list_databases(&self) -> EvifResult<Vec<String>> {
        let (client, connection) = tokio_postgres::connect(
            &self.config.connection_string,
            NoTls,
        ).await.map_err(|e| EvifError::Internal(format!("Connection failed: {}", e)))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        let rows = client.query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            &[]
        ).await
            .map_err(|e| EvifError::Internal(format!("Failed to list databases: {}", e)))?;

        Ok(rows.iter().map(|r| r.get("datname")).collect())
    }

    /// 列出所有 schema
    async fn list_schemas(&self, _db: &str) -> EvifResult<Vec<String>> {
        let (client, connection) = tokio_postgres::connect(
            &self.config.connection_string,
            NoTls,
        ).await.map_err(|e| EvifError::Internal(format!("Connection failed: {}", e)))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        let rows = client.query(
            "SELECT schema_name FROM information_schema.schemata WHERE schema_name NOT LIKE 'pg_%' AND schema_name != 'information_schema' ORDER BY schema_name",
            &[]
        ).await
            .map_err(|e| EvifError::Internal(format!("Failed to list schemas: {}", e)))?;

        Ok(rows.iter().map(|r| r.get("schema_name")).collect())
    }

    /// 列出所有表
    async fn list_tables(&self, _db: &str, schema: &str) -> EvifResult<Vec<String>> {
        let (client, connection) = tokio_postgres::connect(
            &self.config.connection_string,
            NoTls,
        ).await.map_err(|e| EvifError::Internal(format!("Connection failed: {}", e)))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        let rows = client.query(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = $1 AND table_type = 'BASE TABLE' ORDER BY table_name",
            &[&schema]
        ).await
            .map_err(|e| EvifError::Internal(format!("Failed to list tables: {}", e)))?;

        Ok(rows.iter().map(|r| r.get("table_name")).collect())
    }

    /// 获取表结构 (使用 information_schema)
    pub async fn get_table_schema(&self, _db: &str, schema: &str, table: &str) -> EvifResult<String> {
        let (client, connection) = tokio_postgres::connect(
            &self.config.connection_string,
            NoTls,
        ).await.map_err(|e| EvifError::Internal(format!("Connection failed: {}", e)))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        let query = "
            SELECT column_name, data_type, is_nullable, column_default
            FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = $2
            ORDER BY ordinal_position
        ";

        let rows = client.query(query, &[&schema, &table]).await
            .map_err(|e| EvifError::Internal(format!("Failed to get schema: {}", e)))?;

        let mut schema_str = format!("-- Schema for {}.{}\nCREATE TABLE {} (\n", schema, table, table);
        let mut first = true;

        for row in rows {
            if !first {
                schema_str.push_str(",\n");
            }
            let col_name: String = row.get("column_name");
            let data_type: String = row.get("data_type");
            let nullable: String = row.get("is_nullable");
            let default: Option<String> = row.get("column_default");

            schema_str.push_str(&format!("  {} {}", col_name, data_type));
            if nullable == "YES" {
                schema_str.push_str(" NULL");
            } else {
                schema_str.push_str(" NOT NULL");
            }
            if let Some(def) = default {
                schema_str.push_str(&format!(" DEFAULT {}", def));
            }
            first = false;
        }

        schema_str.push_str("\n);\n");
        Ok(schema_str)
    }

    /// 获取表记录数
    pub async fn get_table_count(&self, _db: &str, schema: &str, table: &str) -> EvifResult<i64> {
        let (client, connection) = tokio_postgres::connect(
            &self.config.connection_string,
            NoTls,
        ).await.map_err(|e| EvifError::Internal(format!("Connection failed: {}", e)))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        let query = format!(
            "SELECT COUNT(*) as cnt FROM \"{}\".\"{}\"",
            schema.replace("\"", "\"\""),
            table.replace("\"", "\"\"")
        );

        let rows = client.query(&query, &[]).await
            .map_err(|e| EvifError::Internal(format!("Failed to get count: {}", e)))?;

        if let Some(row) = rows.first() {
            let count: i64 = row.get("cnt");
            Ok(count)
        } else {
            Ok(0)
        }
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
                // 列出所有数据库
                let dbs = self.list_databases().await?;
                if dbs.is_empty() {
                    vec![Self::make_file_info("postgres", true)]
                } else {
                    dbs.into_iter().map(|name| Self::make_file_info(&name, true)).collect()
                }
            }
            p if p.starts_with("/databases/") || p.starts_with("databases/") => {
                // 解析路径: /databases/<db>/<path>
                let parts: Vec<&str> = p.trim_start_matches("/databases/").trim_start_matches("databases/").split('/').collect();
                let db = parts.first().unwrap_or(&"postgres");

                if parts.len() == 1 {
                    // /databases/<db> -> 列出 schemas 和 tables
                    vec![
                        Self::make_file_info("schemas", true),
                        Self::make_file_info("tables", true),
                    ]
                } else {
                    let subpath = parts[1..].join("/");
                    if subpath == "schemas" {
                        // 列出所有 schemas
                        let schemas = self.list_schemas(db).await?;
                        if schemas.is_empty() {
                            vec![
                                Self::make_file_info("public", true),
                                Self::make_file_info("pg_catalog", true),
                            ]
                        } else {
                            schemas.into_iter().map(|name| Self::make_file_info(&name, true)).collect()
                        }
                    } else if subpath == "tables" {
                        // 列出 public schema 的表
                        let tables = self.list_tables(db, "public").await?;
                        tables.into_iter().map(|name| Self::make_file_info(&name, true)).collect()
                    } else if subpath.starts_with("schemas/") || subpath.starts_with("tables/") {
                        // /databases/<db>/schemas/<schema> -> 列出该 schema 的表
                        // /databases/<db>/tables/<schema> -> 列出该 schema 的表
                        let schema_name = subpath.trim_start_matches("schemas/")
                            .trim_start_matches("tables/");

                        if schema_name == "public" || schema_name == "pg_catalog" || schema_name == "information_schema" {
                            let tables = self.list_tables(db, schema_name).await?;
                            tables.into_iter().map(|name| Self::make_file_info(&name, true)).collect()
                        } else {
                            vec![]
                        }
                    } else {
                        return Err(EvifError::NotFound(path.to_string()));
                    }
                }
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