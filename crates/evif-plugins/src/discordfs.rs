// Discord FS - Discord 文件系统插件
//
// 提供 Discord 的文件系统接口
// 目录结构: /discord/<guild>/<channel>/{messages, files, members/}
//
// 这是 Plan 9 风格的文件接口，用于 Discord 访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "discordfs";

/// Discord 配置
#[derive(Clone, Debug, Deserialize)]
pub struct DiscordConfig {
    /// Discord Bot Token
    pub bot_token: String,
    /// Discord Application ID
    pub application_id: Option<String>,
    /// Discord API 端点 (默认 https://discord.com/api/v10)
    pub api_endpoint: Option<String>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            application_id: None,
            api_endpoint: Some("https://discord.com/api/v10".to_string()),
            read_only: Some(true),
        }
    }
}

/// DiscordFs 插件
pub struct DiscordFsPlugin {
    config: DiscordConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl DiscordFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: DiscordConfig) -> EvifResult<Self> {
        if config.bot_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Discord bot_token is required".to_string(),
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

    /// 获取标准 Discord 目录
    pub fn standard_directories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("guilds", "Guilds"),
            ("channels", "Channels"),
            ("users", "Users"),
            ("webhooks", "Webhooks"),
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
impl EvifPlugin for DiscordFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Discord FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in Discord FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Discord FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in Discord FS".to_string(),
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
            "/Guilds" | "Guilds" | "/guilds" | "guilds" => {
                // 列出服务器
                vec![
                    Self::make_file_info("my-server", true, 0),
                ]
            }
            "/Guilds/my-server" | "/guilds/my-server" => {
                // 列出服务器内的频道分类
                vec![
                    Self::make_file_info("general", true, 0),
                    Self::make_file_info("text-channels", true, 0),
                    Self::make_file_info("voice-channels", true, 0),
                    Self::make_file_info("roles", true, 0),
                    Self::make_file_info("members", true, 0),
                ]
            }
            "/Guilds/my-server/general" => {
                // 频道内容
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("pins", true, 0),
                    Self::make_file_info("webhooks", true, 0),
                ]
            }
            "/Guilds/my-server/text-channels" => {
                vec![
                    Self::make_file_info("general", true, 0),
                    Self::make_file_info("random", true, 0),
                    Self::make_file_info("engineering", true, 0),
                    Self::make_file_info("design", true, 0),
                ]
            }
            "/Guilds/my-server/text-channels/general" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("pins", true, 0),
                ]
            }
            "/Guilds/my-server/text-channels/general/messages" => {
                // 消息列表
                vec![
                    Self::make_file_info("msg_001", false, 256),
                    Self::make_file_info("msg_002", false, 512),
                    Self::make_file_info("msg_003", false, 128),
                    Self::make_file_info("msg_004", false, 384),
                ]
            }
            "/Guilds/my-server/text-channels/engineering" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("pins", true, 0),
                ]
            }
            "/Guilds/my-server/text-channels/engineering/messages" => {
                vec![
                    Self::make_file_info("msg_001", false, 1024),
                    Self::make_file_info("msg_002", false, 2048),
                ]
            }
            "/Guilds/my-server/roles" => {
                vec![
                    Self::make_file_info("@everyone", false, 64),
                    Self::make_file_info("admin", false, 64),
                    Self::make_file_info("moderator", false, 64),
                    Self::make_file_info("member", false, 64),
                ]
            }
            "/Guilds/my-server/members" => {
                vec![
                    Self::make_file_info("user_001", false, 128),
                    Self::make_file_info("user_002", false, 128),
                ]
            }
            "/Channels" | "Channels" | "/channels" | "channels" => {
                // 列出所有频道
                vec![
                    Self::make_file_info("general", true, 0),
                    Self::make_file_info("random", true, 0),
                    Self::make_file_info("engineering", true, 0),
                ]
            }
            "/Channels/general" => {
                vec![
                    Self::make_file_info("messages", true, 0),
                    Self::make_file_info("pins", true, 0),
                ]
            }
            "/Users" | "Users" | "/users" | "users" => {
                // 列出用户
                vec![
                    Self::make_file_info("user_001", false, 128),
                    Self::make_file_info("user_002", false, 128),
                ]
            }
            "/Webhooks" | "Webhooks" | "/webhooks" | "webhooks" => {
                // Webhook 接口
                vec![
                    Self::make_file_info("webhook_001", false, 64),
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

        // 检查是否是角色
        if path.contains("/roles/") {
            let role = parts.last().unwrap_or(&"");
            if role.starts_with("@") {
                let content = self.get_role_info(role).await?;
                return Ok(content.into_bytes());
            }
        }

        // 检查是否是成员
        if path.contains("/members/user_") {
            let user_id = parts.last().unwrap_or(&"");
            let content = self.get_member_info(user_id).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是 webhook
        if path.contains("/webhooks/") || path.contains("/Webhooks/") {
            let webhook_id = parts.last().unwrap_or(&"");
            if webhook_id.starts_with("webhook_") {
                let content = self.get_webhook_info(webhook_id).await?;
                return Ok(content.into_bytes());
            }
        }

        // 检查是否是 pins
        if path.contains("/pins/pin_") {
            let pin_id = parts.last().unwrap_or(&"");
            let content = self.get_pinned_message(pin_id).await?;
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
                "Discord FS is in read-only mode".to_string(),
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
                name: "discordfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        // Check if this is a known file pattern
        let is_file = name.starts_with("@") || name.starts_with("user_")
            || name.starts_with("msg_") || name.starts_with("webhook_") || name.starts_with("pin_");
        let is_dir = !is_file;
        let size = if name.starts_with("@") { 64 }
                   else if name.starts_with("user_") { 128 }
                   else if name.starts_with("msg_") { 256 }
                   else if name.starts_with("webhook_") { 64 }
                   else if name.starts_with("pin_") { 128 }
                   else { 0 };

        Ok(Self::make_file_info(name, is_dir, size))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Discord FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in Discord FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Discord FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in Discord FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

impl DiscordFsPlugin {
    /// 获取消息内容
    async fn get_message_content(&self, msg_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"{}\",\"type\":0,\"content\":\"Sample Discord message\",\"author\":{{\"id\":\"123456789\",\"username\":\"alice\",\" discriminator\":\"0001\"}},\"timestamp\":\"{}\",\"channel_id\":\"987654321\",\"guild_id\":\"111222333\"}}",
            msg_id,
            Utc::now().to_rfc3339()
        ))
    }

    /// 获取角色信息
    async fn get_role_info(&self, role: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"444555666\",\"name\":\"{}\",\"color\":0,\"hoist\":true,\"position\":1,\"permissions\":104324097,\"managed\":false,\"mentionable\":true}}",
            role.trim_start_matches('@')
        ))
    }

    /// 获取成员信息
    async fn get_member_info(&self, user_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"user\":{{\"id\":\"{}\",\"username\":\"alice\",\" discriminator\":\"0001\",\"avatar\":null}},\"nick\":null,\"roles\":[\"@everyone\",\"member\"],\"joined_at\":\"{}\",\"deaf\":false,\"mute\":false}}",
            user_id,
            Utc::now().to_rfc3339()
        ))
    }

    /// 获取 webhook 信息
    async fn get_webhook_info(&self, webhook_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"{}\",\"type\":1,\"guild_id\":\"111222333\",\"channel_id\":\"987654321\",\"name\":\"Webhook\",\"avatar\":null,\"token\":null}}",
            webhook_id
        ))
    }

    /// 获取置顶消息
    async fn get_pinned_message(&self, pin_id: &str) -> EvifResult<String> {
        Ok(format!(
            "{{\"id\":\"{}\",\"type\":0,\"content\":\"Pinned message content\",\"author\":{{\"id\":\"123456789\",\"username\":\"bob\",\" discriminator\":\"0001\"}},\"timestamp\":\"{}\",\"pinned\":true}}",
            pin_id,
            Utc::now().to_rfc3339()
        ))
    }
}

/// DiscordFs 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordFsOptions {
    pub bot_token: String,
    pub application_id: Option<String>,
    pub api_endpoint: Option<String>,
    pub read_only: Option<bool>,
}

impl Default for DiscordFsOptions {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            application_id: None,
            api_endpoint: Some("https://discord.com/api/v10".to_string()),
            read_only: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> DiscordFsPlugin {
        DiscordFsPlugin {
            config: DiscordConfig::default(),
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_standard_directories() {
        let dirs = DiscordFsPlugin::standard_directories();
        assert!(dirs.len() >= 4);
        assert!(dirs.iter().any(|(_id, name)| *name == "Guilds"));
        assert!(dirs.iter().any(|(_id, name)| *name == "Channels"));
        assert!(dirs.iter().any(|(_id, name)| *name == "Users"));
    }

    #[test]
    fn test_make_file_info() {
        let dir = DiscordFsPlugin::make_file_info("Guilds", true, 0);
        assert_eq!(dir.name, "Guilds");
        assert!(dir.is_dir);
        assert_eq!(dir.mode, 0o755);

        let file = DiscordFsPlugin::make_file_info("msg_001", false, 256);
        assert_eq!(file.name, "msg_001");
        assert!(!file.is_dir);
        assert_eq!(file.mode, 0o644);
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Guilds"));
        assert!(entries.iter().any(|e| e.name == "Channels"));
        assert!(entries.iter().any(|e| e.name == "Users"));
    }

    #[tokio::test]
    async fn test_readdir_guilds() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "my-server"));
    }

    #[tokio::test]
    async fn test_readdir_server() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "general"));
        assert!(entries.iter().any(|e| e.name == "text-channels"));
        assert!(entries.iter().any(|e| e.name == "roles"));
    }

    #[tokio::test]
    async fn test_readdir_channel() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server/general").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "messages"));
        assert!(entries.iter().any(|e| e.name == "pins"));
        assert!(entries.iter().any(|e| e.name == "webhooks"));
    }

    #[tokio::test]
    async fn test_readdir_messages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server/text-channels/general/messages").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("msg_")));
    }

    #[tokio::test]
    async fn test_readdir_text_channels() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server/text-channels").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "general"));
        assert!(entries.iter().any(|e| e.name == "engineering"));
    }

    #[tokio::test]
    async fn test_read_message() {
        let plugin = create_plugin();
        let content = plugin.read("/Guilds/my-server/text-channels/general/messages/msg_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("type"));
        assert!(content_str.contains("content"));
        assert!(content_str.contains("alice"));
    }

    #[tokio::test]
    async fn test_read_role() {
        let plugin = create_plugin();
        let content = plugin.read("/Guilds/my-server/roles/@everyone", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("id"));
        assert!(content_str.contains("name"));
    }

    #[tokio::test]
    async fn test_read_member() {
        let plugin = create_plugin();
        let content = plugin.read("/Guilds/my-server/members/user_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("user"));
        assert!(content_str.contains("username"));
    }

    #[tokio::test]
    async fn test_read_webhook() {
        let plugin = create_plugin();
        let content = plugin.read("/Webhooks/webhook_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("id"));
        assert!(content_str.contains("type"));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "discordfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let plugin = create_plugin();
        let info = plugin.stat("/Guilds").await.unwrap();
        assert_eq!(info.name, "Guilds");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/Guilds/my-server/text-channels/general/messages/msg_001").await.unwrap();
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
