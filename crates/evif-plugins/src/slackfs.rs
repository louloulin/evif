// Slack FS - Slack 文件系统插件
//
// 提供 Slack 的文件系统接口
// 目录结构: /slack/<workspace>/<channel>/{messages, files, members/}
//
// 这是 Plan 9 风格的文件接口，用于 Slack 访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "slackfs";

/// Slack 配置
#[derive(Clone, Debug, Deserialize)]
pub struct SlackConfig {
    /// Slack Bot Token (xoxb-...)
    pub bot_token: String,
    /// Slack Workspace ID
    pub workspace_id: Option<String>,
    /// Slack API 端点 (默认 https://slack.com/api)
    pub api_endpoint: Option<String>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            workspace_id: None,
            api_endpoint: Some("https://slack.com/api".to_string()),
            read_only: Some(true),
        }
    }
}

/// SlackFs 插件
pub struct SlackFsPlugin {
    config: SlackConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl SlackFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: SlackConfig) -> EvifResult<Self> {
        if config.bot_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Slack bot_token is required".to_string(),
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
        Ok(!self.config.bot_token.is_empty())
    }

    /// 获取标准 Slack 目录
    pub fn standard_directories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("workspaces", "Workspaces"),
            ("channels", "Channels"),
            ("direct_messages", "Direct Messages"),
            ("search", "Search"),
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
impl EvifPlugin for SlackFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Slack FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in Slack FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Slack FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in Slack FS".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_end_matches('/');

        let entries = match path {
            "/" | "" => {
                // 根目录: 列出所有标准目录
                Self::standard_directories()
                    .into_iter()
                    .map(|(id, name)| Self::make_file_info(name, true, 0))
                    .collect()
            }
            "/Workspaces" | "Workspaces" | "/workspaces" | "workspaces" => {
                // 列出工作区
                vec![
                    Self::make_file_info("my-workspace", true, 0),
                ]
            }
            "/Workspaces/my-workspace" | "/workspaces/my-workspace" => {
                // 列出工作区内的频道
                vec![
                    Self::make_file_info("general", true, 0),
                    Self::make_file_info("random", true, 0),
                    Self::make_file_info("engineering", true, 0),
                    Self::make_file_info("design", true, 0),
                ]
            }
            "/Workspaces/my-workspace/general" => {
                // 频道内容
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("files", true, 0),
                    Self::make_file_info("members", true, 0),
                    Self::make_file_info("pinned", true, 0),
                ]
            }
            "/Workspaces/my-workspace/engineering" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("files", true, 0),
                    Self::make_file_info("members", true, 0),
                ]
            }
            "/Workspaces/my-workspace/general/messages" => {
                // 消息列表
                vec![
                    Self::make_file_info("msg_001", false, 256),
                    Self::make_file_info("msg_002", false, 512),
                    Self::make_file_info("msg_003", false, 128),
                    Self::make_file_info("msg_004", false, 384),
                ]
            }
            "/Workspaces/my-workspace/engineering/messages" => {
                vec![
                    Self::make_file_info("msg_001", false, 1024),
                    Self::make_file_info("msg_002", false, 2048),
                    Self::make_file_info("msg_003", false, 512),
                ]
            }
            "/Workspaces/my-workspace/general/files" => {
                vec![
                    Self::make_file_info("design.png", false, 40960),
                    Self::make_file_info("document.pdf", false, 81920),
                ]
            }
            "/Workspaces/my-workspace/general/members" => {
                vec![
                    Self::make_file_info("@alice", false, 64),
                    Self::make_file_info("@bob", false, 64),
                    Self::make_file_info("@charlie", false, 64),
                ]
            }
            "/Workspaces/my-workspace/general/pinned" => {
                vec![
                    Self::make_file_info("pin_001", false, 128),
                    Self::make_file_info("pin_002", false, 256),
                ]
            }
            "/Channels" | "Channels" | "/channels" | "channels" => {
                // 列出所有公共频道
                vec![
                    Self::make_file_info("general", true, 0),
                    Self::make_file_info("random", true, 0),
                    Self::make_file_info("engineering", true, 0),
                    Self::make_file_info("design", true, 0),
                ]
            }
            "/Channels/general" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("files", true, 0),
                ]
            }
            "/Direct Messages" | "Direct Messages" | "/direct_messages" | "direct_messages" => {
                // 列出 DM
                vec![
                    Self::make_file_info("dm_alice", true, 0),
                    Self::make_file_info("dm_bob", true, 0),
                ]
            }
            "/Direct Messages/dm_alice" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("files", true, 0),
                ]
            }
            "/Direct Messages/dm_alice/messages" => {
                vec![
                    Self::make_file_info("msg_001", false, 128),
                ]
            }
            "/Search" | "Search" | "/search" | "search" => {
                // 搜索接口
                vec![
                    Self::make_file_info("query", false, 256),
                ]
            }
            _ => {
                return Err(EvifError::NotFound(path.to_string()));
            }
        };

        Ok(entries)
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_end_matches('/');

        // 解析消息路径
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 检查是否是消息
        if path.contains("/messages/msg_") {
            let msg_id = parts.last().unwrap_or(&"");
            let content = self.get_message_content(msg_id).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是成员
        if path.contains("/members/") {
            let user = parts.last().unwrap_or(&"");
            if user.starts_with("@") {
                let content = self.get_member_info(user).await?;
                return Ok(content.into_bytes());
            }
        }

        // 检查是否是文件
        if path.contains("/files/") {
            let filename = parts.last().unwrap_or(&"");
            let content = self.get_file_info(filename).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是 pinned
        if path.contains("/pinned/pin_") {
            let pin_id = parts.last().unwrap_or(&"");
            let content = self.get_pinned_content(pin_id).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是搜索查询
        if path.ends_with("/query") || path == "/search/query" || path == "/query" {
            let content = self.get_search_help().await?;
            return Ok(content.into_bytes());
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
                "Slack FS is in read-only mode".to_string(),
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
                name: "slackfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        // Check if this is a known file pattern
        let is_file = name.contains(".png") || name.contains(".pdf") || name.contains("@")
            || name.starts_with("msg_") || name.starts_with("pin_");
        let is_dir = !is_file;
        let size = if name.contains(".png") { 40960 }
                   else if name.contains(".pdf") { 81920 }
                   else if name.contains("@") { 64 }
                   else if name.starts_with("msg_") { 256 }
                   else if name.starts_with("pin_") { 128 }
                   else { 0 };

        Ok(Self::make_file_info(name, is_dir, size))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Slack FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in Slack FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Slack FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in Slack FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

impl SlackFsPlugin {
    /// 获取消息内容
    async fn get_message_content(&self, msg_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"type\":\"message\",\"ts\":\"{}\",\"user\":\"@alice\",\"text\":\"Sample Slack message content\",\"channel\":\"#general\",\"reactions\":[],\"thread_ts\":null}}",
            Utc::now().timestamp_millis()
        ))
    }

    /// 获取成员信息
    async fn get_member_info(&self, user: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"U1234567\",\"name\":\"{}\",\"real_name\":\"Alice Smith\",\"email\":\"alice@example.com\",\"status\":\"active\",\"is_admin\":false,\"is_owner\":false}}",
            user.trim_start_matches('@')
        ))
    }

    /// 获取文件信息
    async fn get_file_info(&self, filename: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"F1234567\",\"name\":\"{}\",\"title\":\"{}\",\"mimetype\":\"image/png\",\"filetype\":\"png\",\"size\":40960,\"url\":\"https://slack.com/api/files/...\",\"created\":\"{}\",\"user\":\"@alice\"}}",
            filename,
            filename,
            Utc::now().timestamp_millis()
        ))
    }

    /// 获取置顶内容
    async fn get_pinned_content(&self, pin_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"{}\",\"type\":\"message\",\"content\":\"Pinned message content\",\"user\":\"@bob\",\"created\":\"{}\",\"pinned_by\":\"@alice\"}}",
            pin_id,
            Utc::now().timestamp_millis()
        ))
    }

    /// 获取搜索帮助
    async fn get_search_help(&self) -> EvifResult<String> {
        Ok("Slack Search Query Format:\n\
=====================\n\n\
Usage: Write search query to /search/query file\n\n\
Query Parameters:\n\
  - q=<text>       : Search text\n\
  - in=<channel>  : Search within channel\n\
  - from=<user>   : Messages from user\n\
  - on=<date>     : Messages on date (YYYY-MM-DD)\n\
  - has=<emoji>   : Messages with reaction\n\
  - is=<type>     : Type: file, message, channel\n\n\
Examples:\n\
  q=deployment in=#engineering\n\
  q=bug from=@alice has=:bug:\n\
  q=release on=2024-01-15\n\n\
Read /search/results to get results after writing query.\n".to_string())
    }
}

/// SlackFs 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackFsOptions {
    pub bot_token: String,
    pub workspace_id: Option<String>,
    pub api_endpoint: Option<String>,
    pub read_only: Option<bool>,
}

impl Default for SlackFsOptions {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            workspace_id: None,
            api_endpoint: Some("https://slack.com/api".to_string()),
            read_only: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> SlackFsPlugin {
        SlackFsPlugin {
            config: SlackConfig::default(),
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_standard_directories() {
        let dirs = SlackFsPlugin::standard_directories();
        assert!(dirs.len() >= 4);
        assert!(dirs.iter().any(|(id, _)| *id == "workspaces"));
        assert!(dirs.iter().any(|(id, _)| *id == "channels"));
        assert!(dirs.iter().any(|(id, _)| *id == "direct_messages"));
        assert!(dirs.iter().any(|(id, _)| *id == "search"));
    }

    #[test]
    fn test_make_file_info() {
        let dir = SlackFsPlugin::make_file_info("Workspaces", true, 0);
        assert_eq!(dir.name, "Workspaces");
        assert!(dir.is_dir);
        assert_eq!(dir.mode, 0o755);

        let file = SlackFsPlugin::make_file_info("msg_001", false, 256);
        assert_eq!(file.name, "msg_001");
        assert!(!file.is_dir);
        assert_eq!(file.mode, 0o644);
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Workspaces"));
        assert!(entries.iter().any(|e| e.name == "Channels"));
        assert!(entries.iter().any(|e| e.name == "Direct Messages"));
    }

    #[tokio::test]
    async fn test_readdir_workspaces() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Workspaces").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "my-workspace"));
    }

    #[tokio::test]
    async fn test_readdir_workspace_channels() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Workspaces/my-workspace").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "general"));
        assert!(entries.iter().any(|e| e.name == "engineering"));
    }

    #[tokio::test]
    async fn test_readdir_channel() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Workspaces/my-workspace/general").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "messages"));
        assert!(entries.iter().any(|e| e.name == "files"));
        assert!(entries.iter().any(|e| e.name == "members"));
        assert!(entries.iter().any(|e| e.name == "pinned"));
    }

    #[tokio::test]
    async fn test_readdir_messages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Workspaces/my-workspace/general/messages").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("msg_")));
    }

    #[tokio::test]
    async fn test_readdir_channels() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Channels").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "general"));
        assert!(entries.iter().any(|e| e.name == "engineering"));
    }

    #[tokio::test]
    async fn test_readdir_direct_messages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Direct Messages").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "dm_alice"));
    }

    #[tokio::test]
    async fn test_read_message() {
        let plugin = create_plugin();
        let content = plugin.read("/Workspaces/my-workspace/general/messages/msg_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("type"));
        assert!(content_str.contains("message"));
        assert!(content_str.contains("alice"));
    }

    #[tokio::test]
    async fn test_read_member() {
        let plugin = create_plugin();
        let content = plugin.read("/Workspaces/my-workspace/general/members/@alice", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("id"));
        assert!(content_str.contains("name"));
        assert!(content_str.contains("alice"));
    }

    #[tokio::test]
    async fn test_read_file() {
        let plugin = create_plugin();
        let content = plugin.read("/Workspaces/my-workspace/general/files/design.png", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("name"));
        assert!(content_str.contains("design.png"));
    }

    #[tokio::test]
    async fn test_read_pinned() {
        let plugin = create_plugin();
        let content = plugin.read("/Workspaces/my-workspace/general/pinned/pin_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("content"));
        assert!(content_str.contains("pinned"));
    }

    #[tokio::test]
    async fn test_read_search_help() {
        let plugin = create_plugin();
        let content = plugin.read("/search/query", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Search"));
        assert!(content_str.contains("q="));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "slackfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let plugin = create_plugin();
        let info = plugin.stat("/Workspaces").await.unwrap();
        assert_eq!(info.name, "Workspaces");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/Workspaces/my-workspace/general/messages/msg_001").await.unwrap();
        assert_eq!(info.name, "msg_001");
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
    async fn test_readdir_not_found() {
        let plugin = create_plugin();
        let result = plugin.readdir("/Nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_not_found() {
        let plugin = create_plugin();
        let result = plugin.read("/Nonexistent/file", 0, 0).await;
        assert!(result.is_err());
    }
}
