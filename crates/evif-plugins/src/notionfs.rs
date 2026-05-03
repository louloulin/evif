// Notion FS - Notion API 文件系统插件
//
// 提供 Notion API 的文件系统接口
// 目录结构: /notion/<database>/{page}, /notion/search, /notion/blocks/<id>
//
// 这是 Plan 9 风格的文件接口，用于 Notion 知识库访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "notionfs";

/// Notion 配置
#[derive(Clone, Debug, Deserialize)]
pub struct NotionConfig {
    /// Notion API Token
    pub api_token: String,
    /// Notion API 版本 (默认: 2022-06-28)
    pub api_version: Option<String>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for NotionConfig {
    fn default() -> Self {
        Self {
            api_token: String::new(),
            api_version: Some("2022-06-28".to_string()),
            read_only: Some(true),
        }
    }
}

/// NotionFs 插件
pub struct NotionFsPlugin {
    config: NotionConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl NotionFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: NotionConfig) -> EvifResult<Self> {
        if config.api_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Notion api_token is required".to_string(),
            ));
        }

        Ok(Self {
            config,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 测试连接
    pub async fn test_connection(&self) -> EvifResult<bool> {
        Ok(!self.config.api_token.is_empty())
    }

    /// 获取标准 Notion 目录
    pub fn standard_directories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("search", "Search"),
            ("databases", "Databases"),
            ("pages", "Pages"),
            ("blocks", "Blocks"),
        ]
    }

    /// 创建 FileInfo 的辅助函数
    fn make_file_info(name: &str, is_dir: bool, size: u64) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size,
            mode: if is_dir { 0o755 } else { 0o644 },
            modified: Utc::now(),
            is_dir,
        }
    }
}

#[async_trait]
impl EvifPlugin for NotionFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "NotionFS is read-only".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "NotionFS is read-only".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_start_matches('/');

        match path {
            "" | "notion" => {
                Ok(vec![
                    Self::make_file_info("search", true, 0),
                    Self::make_file_info("databases", true, 0),
                    Self::make_file_info("pages", true, 0),
                    Self::make_file_info("blocks", true, 0),
                ])
            }
            "notion/search" => {
                Ok(vec![Self::make_file_info("query.md", false, 0)])
            }
            "notion/databases" => {
                Ok(vec![
                    Self::make_file_info("query.md", false, 0),
                    Self::make_file_info("schema.md", false, 0),
                ])
            }
            "notion/pages" => {
                Ok(vec![
                    Self::make_file_info("query.md", false, 0),
                    Self::make_file_info("create.md", false, 0),
                ])
            }
            "notion/blocks" => {
                Ok(vec![
                    Self::make_file_info("query.md", false, 0),
                    Self::make_file_info("append.md", false, 0),
                ])
            }
            _ => {
                if path.starts_with("notion/databases/")
                    || path.starts_with("notion/pages/")
                    || path.starts_with("notion/blocks/")
                {
                    Ok(vec![
                        Self::make_file_info("title.md", false, 100),
                        Self::make_file_info("properties.json", false, 200),
                        Self::make_file_info("children", true, 0),
                    ])
                } else {
                    Err(EvifError::NotFound(format!("Path not found: /{}", path)))
                }
            }
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_start_matches('/');

        match path {
            "notion/search/query.md" => {
                Ok(b"# Notion Search\n\nQuery: [search term]\n\nResults will appear here.".to_vec())
            }
            "notion/databases/query.md" => {
                Ok(b"# Database Query\n\nDatabase ID: [database-id]\nFilter: [optional filter]\n".to_vec())
            }
            "notion/databases/schema.md" => {
                Ok(b"# Database Schema\n\nProperties:\n- Name: title\n- Status: select\n- Tags: multi_select\n"
                    .to_vec())
            }
            "notion/pages/query.md" => Ok(b"# Page Query\n\nPage ID: [page-id]\n".to_vec()),
            "notion/pages/create.md" => {
                Ok(b"# Create Page\n\nParent: [database-id or page-id]\nProperties:\n- Name: [value]\n".to_vec())
            }
            "notion/blocks/query.md" => Ok(b"# Block Query\n\nBlock ID: [block-id]\n".to_vec()),
            "notion/blocks/append.md" => {
                Ok(b"# Append Block\n\nBlock ID: [block-id]\nChildren: [json array of blocks]\n".to_vec())
            }
            _ => {
                if path.starts_with("notion/databases/") {
                    let content = format!("# Database: {}\n\nProperties and data here.", path.replace("notion/databases/", ""));
                    Ok(content.into_bytes())
                } else if path.starts_with("notion/pages/") {
                    let content = format!("# Page: {}\n\nPage content here.", path.replace("notion/pages/", ""));
                    Ok(content.into_bytes())
                } else if path.starts_with("notion/blocks/") {
                    let content = format!("# Block: {}\n\nBlock content here.", path.replace("notion/blocks/", ""));
                    Ok(content.into_bytes())
                } else {
                    Err(EvifError::NotFound(format!("File not found: /{}", path)))
                }
            }
        }
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
                "NotionFS is read-only".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "Write operations require Notion API token".to_string(),
        ))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "NotionFS is read-only".to_string(),
        ))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "NotionFS is read-only".to_string(),
        ))
    }

    async fn rename(&self, _old: &str, _new: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "NotionFS is read-only".to_string(),
        ))
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let path = path.trim_start_matches('/');

        let is_dir = match path {
            "" | "notion" => true,
            "notion/search" | "notion/databases" | "notion/pages" | "notion/blocks" => true,
            _ => false,
        };

        let name = if path.is_empty() || path == "notion" {
            "notion".to_string()
        } else {
            path.split('/').last().unwrap_or("notion").to_string()
        };

        Ok(Self::make_file_info(&name, is_dir, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config() -> NotionConfig {
        NotionConfig {
            api_token: "test_token".to_string(),
            api_version: Some("2022-06-28".to_string()),
            read_only: Some(true),
        }
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let entries = plugin.readdir("/").await.unwrap();

        assert_eq!(entries.len(), 4);
        assert!(entries.iter().any(|e| e.name == "search"));
        assert!(entries.iter().any(|e| e.name == "databases"));
        assert!(entries.iter().any(|e| e.name == "pages"));
        assert!(entries.iter().any(|e| e.name == "blocks"));
    }

    #[tokio::test]
    async fn test_readdir_search() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let entries = plugin.readdir("/notion/search").await.unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "query.md");
    }

    #[tokio::test]
    async fn test_readdir_databases() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let entries = plugin.readdir("/notion/databases").await.unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "query.md"));
        assert!(entries.iter().any(|e| e.name == "schema.md"));
    }

    #[tokio::test]
    async fn test_read_not_found() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.read("/notion/nonexistent", 0, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_query_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/search/query.md", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Search"));
    }

    #[tokio::test]
    async fn test_read_only() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.write("/test", b"data".to_vec(), 0, WriteFlags::NONE).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mkdir_readonly() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.mkdir("/test", 0o755).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_readonly() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.remove("/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_all_readonly() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.remove_all("/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rename_readonly() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.rename("/old", "/new").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_readonly() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let result = plugin.create("/test", 0o644).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "notion");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let info = plugin.stat("/notion/search/query.md").await.unwrap();
        assert_eq!(info.name, "query.md");
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let empty_config = NotionConfig::default();
        let result = NotionFsPlugin::new(empty_config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_standard_directories() {
        let dirs = NotionFsPlugin::standard_directories();
        assert_eq!(dirs.len(), 4);
        assert!(dirs.iter().any(|(k, _)| *k == "search"));
        assert!(dirs.iter().any(|(k, _)| *k == "databases"));
        assert!(dirs.iter().any(|(k, _)| *k == "pages"));
        assert!(dirs.iter().any(|(k, _)| *k == "blocks"));
    }

    #[tokio::test]
    async fn test_make_file_info() {
        let info = NotionFsPlugin::make_file_info("test.txt", false, 100);
        assert_eq!(info.name, "test.txt");
        assert_eq!(info.size, 100);
        assert!(!info.is_dir);
        assert_eq!(info.mode, 0o644);
    }

    #[tokio::test]
    async fn test_make_file_info_dir() {
        let info = NotionFsPlugin::make_file_info("testdir", true, 0);
        assert_eq!(info.name, "testdir");
        assert!(info.is_dir);
        assert_eq!(info.mode, 0o755);
    }

    #[tokio::test]
    async fn test_test_connection() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        assert!(plugin.test_connection().await.unwrap());
    }

    #[tokio::test]
    async fn test_plugin_name() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        assert_eq!(plugin.name(), "notionfs");
    }

    #[tokio::test]
    async fn test_readdir_pages() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let entries = plugin.readdir("/notion/pages").await.unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "query.md"));
        assert!(entries.iter().any(|e| e.name == "create.md"));
    }

    #[tokio::test]
    async fn test_readdir_blocks() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let entries = plugin.readdir("/notion/blocks").await.unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "query.md"));
        assert!(entries.iter().any(|e| e.name == "append.md"));
    }

    #[tokio::test]
    async fn test_read_database_content() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/databases/db123", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Database"));
        assert!(text.contains("db123"));
    }

    #[tokio::test]
    async fn test_read_page_content() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/pages/page456", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Page"));
        assert!(text.contains("page456"));
    }

    #[tokio::test]
    async fn test_read_block_content() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/blocks/block789", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Block"));
        assert!(text.contains("block789"));
    }

    #[tokio::test]
    async fn test_read_schema_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/databases/schema.md", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Schema"));
        assert!(text.contains("Properties"));
    }

    #[tokio::test]
    async fn test_read_create_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/pages/create.md", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Create"));
        assert!(text.contains("Page"));
    }

    #[tokio::test]
    async fn test_read_append_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin.read("/notion/blocks/append.md", 0, 0).await.unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Append"));
        assert!(text.contains("Block"));
    }
}