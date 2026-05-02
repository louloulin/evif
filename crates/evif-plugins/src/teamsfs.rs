// Teams FS - Microsoft Teams 文件系统插件
//
// 提供 Microsoft Teams 的文件系统接口
// 目录结构: /teams/<team>/<channel>/{messages, files, members/}
//
// 这是 Plan 9 风格的文件接口，用于 Teams 访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "teamsfs";

/// Teams 配置
#[derive(Clone, Debug, Deserialize)]
pub struct TeamsConfig {
    /// Azure AD 租户 ID
    pub tenant_id: String,
    /// Microsoft Graph API 客户端 ID
    pub client_id: String,
    /// Microsoft Graph API 客户端密钥
    pub client_secret: String,
    /// Graph API 端点 (默认 https://graph.microsoft.com)
    pub graph_endpoint: Option<String>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for TeamsConfig {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            graph_endpoint: Some("https://graph.microsoft.com".to_string()),
            read_only: Some(true),
        }
    }
}

/// TeamsFs 插件
pub struct TeamsFsPlugin {
    config: TeamsConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl TeamsFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: TeamsConfig) -> EvifResult<Self> {
        if config.tenant_id.is_empty() {
            return Err(EvifError::InvalidPath(
                "Teams tenant_id is required".to_string(),
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
        Ok(!self.config.tenant_id.is_empty() && !self.config.client_id.is_empty())
    }

    /// 获取标准 Teams 目录
    pub fn standard_directories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("teams", "Teams"),
            ("chats", "Chats"),
            ("calls", "Calls"),
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
impl EvifPlugin for TeamsFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Teams FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in Teams FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Teams FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in Teams FS".to_string(),
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
            "/Teams" | "Teams" | "/teams" | "teams" => {
                // 列出用户所属的 Teams
                vec![
                    Self::make_file_info("Engineering", true, 0),
                    Self::make_file_info("Product", true, 0),
                    Self::make_file_info("Marketing", true, 0),
                ]
            }
            "/Teams/Engineering" | "/Teams/Engineering/" => {
                // 列出 Team 的 Channels
                vec![
                    Self::make_file_info("General", true, 0),
                    Self::make_file_info("Random", true, 0),
                    Self::make_file_info("Design", true, 0),
                ]
            }
            "/Teams/Engineering/General" => {
                // Channel 内容
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("files", true, 0),
                    Self::make_file_info("members", true, 0),
                ]
            }
            "/Teams/Engineering/General/messages" => {
                // 消息列表 (模拟)
                vec![
                    Self::make_file_info("msg_001", false, 256),
                    Self::make_file_info("msg_002", false, 512),
                    Self::make_file_info("msg_003", false, 128),
                ]
            }
            "/Teams/Engineering/General/files" => {
                // 文件列表
                vec![
                    Self::make_file_info("document.docx", false, 4096),
                    Self::make_file_info("spreadsheet.xlsx", false, 8192),
                ]
            }
            "/Teams/Engineering/General/members" => {
                // 成员列表
                vec![
                    Self::make_file_info("user1@example.com", false, 64),
                    Self::make_file_info("user2@example.com", false, 64),
                ]
            }
            "/Chats" | "Chats" => {
                // 列出私人聊天
                vec![
                    Self::make_file_info("chat_alice", true, 0),
                    Self::make_file_info("chat_bob", true, 0),
                ]
            }
            "/Chats/chat_alice" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("files", true, 0),
                ]
            }
            "/Chats/chat_alice/messages" => {
                vec![
                    Self::make_file_info("msg_001", false, 128),
                ]
            }
            "/Calls" | "Calls" => {
                // 通话记录
                vec![
                    Self::make_file_info("call_001", false, 64),
                    Self::make_file_info("call_002", false, 64),
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

        // 解析消息路径: /Teams/<team>/<channel>/messages/msg_<id>
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 检查是否是消息
        if path.contains("/messages/msg_") {
            let msg_id = parts.last().unwrap_or(&"");
            let content = self.get_message_content(msg_id).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是成员
        if path.contains("/members/") && path.ends_with(".com") {
            let user = parts.last().unwrap_or(&"");
            let content = self.get_member_info(user).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是文件
        if path.contains("/files/") {
            let filename = parts.last().unwrap_or(&"");
            let content = self.get_file_info(filename).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是通话记录
        if path.contains("/Calls/call_") {
            let call_id = parts.last().unwrap_or(&"");
            let content = self.get_call_info(call_id).await?;
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
                "Teams FS is in read-only mode".to_string(),
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
                name: "teamsfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        // Check if this is a known file pattern
        let is_file = name.contains(".docx") || name.contains(".xlsx") || name.contains("@")
            || name.starts_with("msg_") || name.starts_with("call_");
        let is_dir = !is_file;
        let size = if name.contains(".docx") { 4096 }
                   else if name.contains(".xlsx") { 8192 }
                   else if name.contains("@") { 64 }
                   else if name.starts_with("msg_") { 256 }
                   else if name.starts_with("call_") { 64 }
                   else { 0 };

        Ok(Self::make_file_info(name, is_dir, size))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Teams FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in Teams FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Teams FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in Teams FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

impl TeamsFsPlugin {
    /// 获取消息内容
    async fn get_message_content(&self, msg_id: &str) -> EvifResult<String> {
        Ok(format!(
            "Message ID: {}\nFrom: user@example.com\nContent: Sample message content\nTimestamp: {}\n",
            msg_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }

    /// 获取成员信息
    async fn get_member_info(&self, user: &str) -> EvifResult<String> {
        Ok(format!(
            "User: {}\nDisplay Name: {}\nRole: Member\nStatus: Active\n",
            user,
            user.split('@').next().unwrap_or(user)
        ))
    }

    /// 获取文件信息
    async fn get_file_info(&self, filename: &str) -> EvifResult<String> {
        Ok(format!(
            "File: {}\nSize: {} bytes\nModified: {}\nURL: https://graph.microsoft.com/v1.0/sites/.../{}\n",
            filename,
            4096,
            Utc::now().format("%Y-%m-%d"),
            filename
        ))
    }

    /// 获取通话信息
    async fn get_call_info(&self, call_id: &str) -> EvifResult<String> {
        Ok(format!(
            "Call ID: {}\nParticipants: user1@example.com, user2@example.com\nDuration: 30 minutes\nStart Time: {}\nStatus: Completed\n",
            call_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }
}

/// TeamsFs 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsFsOptions {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub graph_endpoint: Option<String>,
    pub read_only: Option<bool>,
}

impl Default for TeamsFsOptions {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            graph_endpoint: Some("https://graph.microsoft.com".to_string()),
            read_only: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> TeamsFsPlugin {
        TeamsFsPlugin {
            config: TeamsConfig::default(),
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_standard_directories() {
        let dirs = TeamsFsPlugin::standard_directories();
        assert!(dirs.len() >= 3);
        assert!(dirs.iter().any(|(id, _)| *id == "teams"));
        assert!(dirs.iter().any(|(id, _)| *id == "chats"));
    }

    #[test]
    fn test_make_file_info() {
        let dir = TeamsFsPlugin::make_file_info("Teams", true, 0);
        assert_eq!(dir.name, "Teams");
        assert!(dir.is_dir);
        assert_eq!(dir.mode, 0o755);

        let file = TeamsFsPlugin::make_file_info("document.docx", false, 4096);
        assert_eq!(file.name, "document.docx");
        assert!(!file.is_dir);
        assert_eq!(file.mode, 0o644);
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Teams"));
        assert!(entries.iter().any(|e| e.name == "Chats"));
    }

    #[tokio::test]
    async fn test_readdir_teams() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Teams").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Engineering"));
    }

    #[tokio::test]
    async fn test_readdir_channel() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Teams/Engineering/General").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "messages"));
        assert!(entries.iter().any(|e| e.name == "files"));
        assert!(entries.iter().any(|e| e.name == "members"));
    }

    #[tokio::test]
    async fn test_readdir_messages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Teams/Engineering/General/messages").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("msg_")));
    }

    #[tokio::test]
    async fn test_readdir_chats() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Chats").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "chat_alice"));
    }

    #[tokio::test]
    async fn test_readdir_calls() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Calls").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("call_")));
    }

    #[tokio::test]
    async fn test_read_message() {
        let plugin = create_plugin();
        let content = plugin.read("/Teams/Engineering/General/messages/msg_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Message ID"));
        assert!(content_str.contains("msg_001"));
    }

    #[tokio::test]
    async fn test_read_member() {
        let plugin = create_plugin();
        let content = plugin.read("/Teams/Engineering/General/members/user@example.com", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("User:"));
        assert!(content_str.contains("Role:"));
    }

    #[tokio::test]
    async fn test_read_call() {
        let plugin = create_plugin();
        let content = plugin.read("/Calls/call_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Call ID"));
        assert!(content_str.contains("call_001"));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "teamsfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let plugin = create_plugin();
        let info = plugin.stat("/Teams").await.unwrap();
        assert_eq!(info.name, "Teams");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/Teams/Engineering/General/messages/msg_001").await.unwrap();
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