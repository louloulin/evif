// Telegram FS - Telegram Bot 文件系统插件
//
// 提供 Telegram Bot 的文件系统接口
// 目录结构: /telegram/<chat>/{messages, media, members, info}
//
// 这是 Plan 9 风格的文件接口，用于 Telegram 访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "telegramfs";

/// Telegram 配置
#[derive(Clone, Debug, Deserialize)]
pub struct TelegramConfig {
    /// Telegram Bot Token
    pub bot_token: String,
    /// Bot API 端点 (默认 https://api.telegram.org)
    pub api_endpoint: Option<String>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            api_endpoint: Some("https://api.telegram.org".to_string()),
            read_only: Some(true),
        }
    }
}

/// TelegramFs 插件
pub struct TelegramFsPlugin {
    config: TelegramConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl TelegramFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: TelegramConfig) -> EvifResult<Self> {
        if config.bot_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Telegram bot_token is required".to_string(),
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

    /// 获取标准 Telegram 目录
    pub fn standard_directories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("chats", "Chats"),
            ("channels", "Channels"),
            ("groups", "Groups"),
            ("bots", "Bots"),
            ("updates", "Updates"),
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
impl EvifPlugin for TelegramFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Telegram FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in Telegram FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Telegram FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in Telegram FS".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_end_matches('/');

        let entries = match path {
            "/" | "" => {
                // 根目录: 列出所有标准目录
                Self::standard_directories()
                    .into_iter()
                    .map(|(_id, name)| Self::make_file_info(name, true, 0))
                    .collect()
            }
            "/Chats" | "Chats" | "/chats" | "chats" => {
                // 列出私人聊天
                vec![
                    Self::make_file_info("chat_100001", true, 0),
                    Self::make_file_info("chat_100002", true, 0),
                    Self::make_file_info("chat_100003", true, 0),
                ]
            }
            "/Chats/chat_100001" => {
                // 单个聊天内容
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("media", true, 0),
                    Self::make_file_info("info", true, 0),
                ]
            }
            "/Chats/chat_100001/messages" => {
                // 消息列表
                vec![
                    Self::make_file_info("msg_1001", false, 512),
                    Self::make_file_info("msg_1002", false, 768),
                    Self::make_file_info("msg_1003", false, 256),
                ]
            }
            "/Chats/chat_100001/media" => {
                // 媒体列表
                vec![
                    Self::make_file_info("photo_001.jpg", false, 102400),
                    Self::make_file_info("video_001.mp4", false, 2048000),
                    Self::make_file_info("document_001.pdf", false, 51200),
                ]
            }
            "/Chats/chat_100001/info" => {
                // 聊天信息
                vec![
                    Self::make_file_info("chat.json", false, 256),
                ]
            }
            "/Channels" | "Channels" | "/channels" | "channels" => {
                // 列出频道
                vec![
                    Self::make_file_info("channel_500001", true, 0),
                    Self::make_file_info("channel_500002", true, 0),
                ]
            }
            "/Channels/channel_500001" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("subscribers", true, 0),
                    Self::make_file_info("info", true, 0),
                ]
            }
            "/Channels/channel_500001/messages" => {
                vec![
                    Self::make_file_info("msg_2001", false, 256),
                    Self::make_file_info("msg_2002", false, 384),
                ]
            }
            "/Channels/channel_500001/subscribers" => {
                vec![
                    Self::make_file_info("user_1001", false, 64),
                    Self::make_file_info("user_1002", false, 64),
                    Self::make_file_info("user_1003", false, 64),
                ]
            }
            "/Groups" | "Groups" | "/groups" | "groups" => {
                // 列出群组
                vec![
                    Self::make_file_info("group_200001", true, 0),
                    Self::make_file_info("group_200002", true, 0),
                ]
            }
            "/Groups/group_200001" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("members", true, 0),
                    Self::make_file_info("info", true, 0),
                ]
            }
            "/Groups/group_200001/messages" => {
                vec![
                    Self::make_file_info("msg_3001", false, 320),
                    Self::make_file_info("msg_3002", false, 448),
                ]
            }
            "/Groups/group_200001/members" => {
                vec![
                    Self::make_file_info("admin_001", false, 64),
                    Self::make_file_info("member_001", false, 64),
                    Self::make_file_info("member_002", false, 64),
                ]
            }
            "/Bots" | "Bots" | "/bots" | "bots" => {
                // Bot 信息
                vec![
                    Self::make_file_info("me", true, 0),
                    Self::make_file_info("commands", true, 0),
                ]
            }
            "/Bots/me" => {
                vec![
                    Self::make_file_info("profile.json", false, 512),
                    Self::make_file_info("settings.json", false, 256),
                ]
            }
            "/Bots/commands" => {
                vec![
                    Self::make_file_info("cmd_start", false, 64),
                    Self::make_file_info("cmd_help", false, 64),
                    Self::make_file_info("cmd_settings", false, 64),
                ]
            }
            "/Updates" | "Updates" | "/updates" | "updates" => {
                // 更新列表
                vec![
                    Self::make_file_info("update_0001", false, 1024),
                    Self::make_file_info("update_0002", false, 768),
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
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 检查是否是消息
        if path.contains("/messages/msg_") {
            let msg_id = parts.last().unwrap_or(&"");
            let content = self.get_message_content(msg_id).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是成员
        if path.contains("/members/") || path.contains("/subscribers/") {
            let user = parts.last().unwrap_or(&"");
            let content = self.get_member_info(user).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是媒体
        if path.contains("/media/") {
            let filename = parts.last().unwrap_or(&"");
            let content = self.get_media_info(filename).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是聊天信息
        if path.contains("/info/chat.json") {
            let content = self.get_chat_info(&path).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是 Bot 信息
        if path.contains("/profile.json") || path.contains("/settings.json") {
            let content = self.get_bot_info(&path).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是命令
        if path.contains("/commands/cmd_") {
            let cmd_name = parts.last().unwrap_or(&"");
            let content = self.get_command_info(cmd_name).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是更新
        if path.contains("/Updates/update_") {
            let update_id = parts.last().unwrap_or(&"");
            let content = self.get_update_info(update_id).await?;
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
                "Telegram FS is in read-only mode".to_string(),
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
                name: "telegramfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        // Check if this is a known file pattern
        let is_file = name.contains(".jpg") || name.contains(".mp4") || name.contains(".pdf")
            || name.contains(".json") || name.starts_with("msg_") || name.starts_with("update_")
            || name.starts_with("cmd_") || name.starts_with("user_") || name.starts_with("admin_");
        let is_dir = !is_file;

        Ok(Self::make_file_info(name, is_dir, 0))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Telegram FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in Telegram FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Telegram FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in Telegram FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

impl TelegramFsPlugin {
    /// 获取消息内容
    async fn get_message_content(&self, msg_id: &str) -> EvifResult<String> {
        Ok(format!(
            "Message ID: {}\nFrom: user123\nContent: Sample Telegram message\nTimestamp: {}\nType: text\n",
            msg_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }

    /// 获取成员信息
    async fn get_member_info(&self, user: &str) -> EvifResult<String> {
        Ok(format!(
            "User: {}\nDisplay Name: {}\nRole: member\nStatus: active\n",
            user,
            user.split('_').last().unwrap_or(user)
        ))
    }

    /// 获取媒体信息
    async fn get_media_info(&self, filename: &str) -> EvifResult<String> {
        let (size, mime) = if filename.contains(".jpg") {
            (102400, "image/jpeg")
        } else if filename.contains(".mp4") {
            (2048000, "video/mp4")
        } else if filename.contains(".pdf") {
            (51200, "application/pdf")
        } else {
            (0, "application/octet-stream")
        };
        Ok(format!(
            "File: {}\nSize: {} bytes\nMIME: {}\nModified: {}\n",
            filename,
            size,
            mime,
            Utc::now().format("%Y-%m-%d")
        ))
    }

    /// 获取聊天信息
    async fn get_chat_info(&self, path: &str) -> EvifResult<String> {
        let chat_id = path.split('/').nth(2).unwrap_or("unknown");
        Ok(format!(
            "{{\"id\": {}, \"type\": \"private\", \"title\": \"Chat {}\", \"member_count\": 2}}",
            chat_id.trim_start_matches("chat_"),
            chat_id
        ))
    }

    /// 获取 Bot 信息
    async fn get_bot_info(&self, path: &str) -> EvifResult<String> {
        if path.contains("profile.json") {
            Ok(format!(
                "{{\"bot_token\": \"***\", \"username\": \"mybot\", \"first_name\": \"My Bot\"}}"
            ))
        } else {
            Ok(format!(
                "{{\"privacy_mode\": \"limited\", \"commands\": [\"start\", \"help\", \"settings\"]}}"
            ))
        }
    }

    /// 获取命令信息
    async fn get_command_info(&self, cmd_name: &str) -> EvifResult<String> {
        Ok(format!(
            "Command: /{}\nDescription: {} command\n",
            cmd_name.trim_start_matches("cmd_"),
            cmd_name.trim_start_matches("cmd_")
        ))
    }

    /// 获取更新信息
    async fn get_update_info(&self, update_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"update_id\": {}, \"message\": {{\"text\": \"Sample update\"}}}}",
            update_id.trim_start_matches("update_")
        ))
    }
}

/// TelegramFs 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramFsOptions {
    pub bot_token: String,
    pub api_endpoint: Option<String>,
    pub read_only: Option<bool>,
}

impl Default for TelegramFsOptions {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            api_endpoint: Some("https://api.telegram.org".to_string()),
            read_only: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> TelegramFsPlugin {
        TelegramFsPlugin {
            config: TelegramConfig::default(),
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_standard_directories() {
        let dirs = TelegramFsPlugin::standard_directories();
        assert!(dirs.len() >= 5);
        assert!(dirs.iter().any(|(id, _)| *id == "chats"));
        assert!(dirs.iter().any(|(id, _)| *id == "channels"));
    }

    #[test]
    fn test_make_file_info() {
        let dir = TelegramFsPlugin::make_file_info("Chats", true, 0);
        assert_eq!(dir.name, "Chats");
        assert!(dir.is_dir);
        assert_eq!(dir.mode, 0o755);

        let file = TelegramFsPlugin::make_file_info("photo.jpg", false, 102400);
        assert_eq!(file.name, "photo.jpg");
        assert!(!file.is_dir);
        assert_eq!(file.mode, 0o644);
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Chats"));
        assert!(entries.iter().any(|e| e.name == "Channels"));
    }

    #[tokio::test]
    async fn test_readdir_chats() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Chats").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("chat_")));
    }

    #[tokio::test]
    async fn test_readdir_chat_content() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Chats/chat_100001").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "messages"));
        assert!(entries.iter().any(|e| e.name == "media"));
    }

    #[tokio::test]
    async fn test_readdir_messages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Chats/chat_100001/messages").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("msg_")));
    }

    #[tokio::test]
    async fn test_readdir_media() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Chats/chat_100001/media").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.contains(".jpg") || e.name.contains(".mp4")));
    }

    #[tokio::test]
    async fn test_readdir_channels() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Channels").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("channel_")));
    }

    #[tokio::test]
    async fn test_readdir_groups() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Groups").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("group_")));
    }

    #[tokio::test]
    async fn test_readdir_bots() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Bots").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "me" || e.name == "commands"));
    }

    #[tokio::test]
    async fn test_readdir_updates() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Updates").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("update_")));
    }

    #[tokio::test]
    async fn test_read_message() {
        let plugin = create_plugin();
        let content = plugin.read("/Chats/chat_100001/messages/msg_1001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Message ID"));
        assert!(content_str.contains("msg_1001"));
    }

    #[tokio::test]
    async fn test_read_media() {
        let plugin = create_plugin();
        let content = plugin.read("/Chats/chat_100001/media/photo_001.jpg", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("File:"));
        assert!(content_str.contains("photo_001.jpg"));
    }

    #[tokio::test]
    async fn test_read_bot_info() {
        let plugin = create_plugin();
        let content = plugin.read("/Bots/me/profile.json", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("bot_token"));
        assert!(content_str.contains("username"));
    }

    #[tokio::test]
    async fn test_read_command() {
        let plugin = create_plugin();
        let content = plugin.read("/Bots/commands/cmd_start", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Command:"));
        assert!(content_str.contains("/start"));
    }

    #[tokio::test]
    async fn test_read_update() {
        let plugin = create_plugin();
        let content = plugin.read("/Updates/update_0001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("update_id"));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "telegramfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let plugin = create_plugin();
        let info = plugin.stat("/Chats").await.unwrap();
        assert_eq!(info.name, "Chats");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/Chats/chat_100001/messages/msg_1001").await.unwrap();
        assert_eq!(info.name, "msg_1001");
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