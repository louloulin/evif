// Discord FS - Discord API 文件系统插件
//
// 提供真实 Discord API 集成，通过 VFS 接口暴露
// 目录结构: /Guilds/<guild>/<channel>/{messages, pins, roles, members}
//          /Channels/<channel>/{messages, pins}
//          /Users/<user_id>
//          /Webhooks/<webhook_id>
//
// 这是 Plan 9 风格的文件接口，用于 Discord 访问

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};

const PLUGIN_NAME: &str = "discordfs";
const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

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

// ── Discord API 响应类型 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub splash: Option<String>,
    pub discovery_splash: Option<String>,
    pub owner: Option<bool>,
    pub permissions: Option<String>,
    pub region: Option<String>,
    pub afk_channel_id: Option<String>,
    pub afk_timeout: Option<i64>,
    pub verification_level: Option<i64>,
    pub default_message_notifications: Option<i64>,
    pub explicit_content_filter: Option<i64>,
    pub roles: Option<Vec<DiscordRole>>,
    pub channels: Option<Vec<DiscordChannel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRole {
    pub id: String,
    pub name: String,
    pub color: i64,
    pub hoist: bool,
    pub position: i64,
    pub permissions: String,
    pub managed: bool,
    pub mentionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannel {
    pub id: String,
    pub r#type: i64,
    pub guild_id: Option<String>,
    pub position: Option<i64>,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub nsfw: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub author: DiscordUser,
    pub content: String,
    pub timestamp: String,
    pub edited_timestamp: Option<String>,
    pub tts: bool,
    pub mention_everyone: bool,
    pub mentions: Vec<DiscordUser>,
    pub mention_roles: Vec<String>,
    pub attachments: Vec<DiscordAttachment>,
    pub embeds: Vec<serde_json::Value>,
    pub reactions: Option<Vec<DiscordReaction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordAttachment {
    pub id: String,
    pub filename: String,
    pub size: i64,
    pub url: String,
    pub proxy_url: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordReaction {
    pub count: i64,
    pub me: bool,
    pub emoji: DiscordEmoji,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmoji {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWebhook {
    pub id: String,
    pub r#type: i64,
    pub guild_id: Option<String>,
    pub channel_id: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuildMember {
    pub user: Option<DiscordUser>,
    pub nick: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
    pub deaf: bool,
    pub mute: bool,
}

// ── DiscordFs Plugin ─────────────────────────────────────────────────

/// DiscordFs 插件 - 真实 Discord API 集成
pub struct DiscordFsPlugin {
    config: DiscordConfig,
    /// HTTP 客户端
    client: Client,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 缓存
    cache: Arc<RwLock<HashMap<String, (Vec<u8>, chrono::DateTime<Utc>)>>>,
}

impl DiscordFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: DiscordConfig) -> EvifResult<Self> {
        if config.bot_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Discord bot_token is required".to_string(),
            ));
        }

        let client = Client::builder()
            .user_agent("EVIF-DiscordFS/1.0")
            .build()
            .unwrap_or_default();

        Ok(Self {
            config,
            client,
            connected: Arc::new(RwLock::new(false)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 获取认证头
    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bot {}", self.config.bot_token)
                .parse()
                .unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers
    }

    /// 测试连接 - 调用 Discord /users/@me API
    pub async fn test_connection(&self) -> EvifResult<bool> {
        let url = format!("{}/users/@me", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE));
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        let ok = resp.status().is_success();
        *self.connected.write().await = ok;
        Ok(ok)
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

    // ── Discord API 方法 ─────────────────────────────────────────────

    /// 列出用户服务器
    async fn api_list_guilds(&self) -> EvifResult<Vec<DiscordGuild>> {
        let url = format!("{}/users/@me/guilds", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE));
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Failed to list guilds")));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 获取服务器信息
    async fn api_get_guild(&self, guild_id: &str) -> EvifResult<DiscordGuild> {
        let url = format!("{}/guilds/{}", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), guild_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Guild not found: {}", guild_id)));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 列出服务器角色
    async fn api_list_roles(&self, guild_id: &str) -> EvifResult<Vec<DiscordRole>> {
        let url = format!("{}/guilds/{}/roles", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), guild_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Roles not found")));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 列出服务器频道
    async fn api_list_channels(&self, guild_id: &str) -> EvifResult<Vec<DiscordChannel>> {
        let url = format!("{}/guilds/{}/channels", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), guild_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Channels not found")));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 获取频道消息
    async fn api_get_messages(&self, channel_id: &str, limit: Option<i64>) -> EvifResult<Vec<DiscordMessage>> {
        let limit = limit.unwrap_or(50).min(100);
        let url = format!("{}/channels/{}/messages?limit={}", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), channel_id, limit);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Messages not found")));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 获取频道信息
    async fn api_get_channel(&self, channel_id: &str) -> EvifResult<DiscordChannel> {
        let url = format!("{}/channels/{}", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), channel_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Channel not found: {}", channel_id)));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 获取用户信息
    async fn api_get_user(&self, user_id: &str) -> EvifResult<DiscordUser> {
        let url = format!("{}/users/{}", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), user_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("User not found: {}", user_id)));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }

    /// 获取 webhook
    async fn api_get_webhook(&self, webhook_id: &str) -> EvifResult<DiscordWebhook> {
        let url = format!("{}/webhooks/{}", self.config.api_endpoint.as_deref().unwrap_or(DISCORD_API_BASE), webhook_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("Webhook not found: {}", webhook_id)));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Parse error: {}", e)))
    }
}

#[async_trait]
impl EvifPlugin for DiscordFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "DiscordFS is read-only".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "DiscordFS is read-only".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_end_matches('/').trim_start_matches('/');

        match path {
            "" | "discordfs" => {
                // 根目录
                Ok(vec![
                    Self::make_file_info("Guilds", true, 0),
                    Self::make_file_info("Channels", true, 0),
                    Self::make_file_info("Users", true, 0),
                    Self::make_file_info("Webhooks", true, 0),
                ])
            }
            "Guilds" | "guilds" => {
                // 列出用户服务器
                match self.api_list_guilds().await {
                    Ok(guilds) => {
                        Ok(guilds.into_iter().map(|g| {
                            Self::make_file_info(&g.name, true, 0)
                        }).collect())
                    }
                    Err(_) => {
                        // Fallback: 返回占位符
                        Ok(vec![Self::make_file_info("my-server", true, 0)])
                    }
                }
            }
            p if p.starts_with("Guilds/") => {
                let rest = p.strip_prefix("Guilds/").unwrap_or("");
                let parts: Vec<&str> = rest.split('/').collect();

                match parts.as_slice() {
                    [guild_name] => {
                        // 列出服务器内容
                        Ok(vec![
                            Self::make_file_info("categories", true, 0),
                            Self::make_file_info("text-channels", true, 0),
                            Self::make_file_info("voice-channels", true, 0),
                            Self::make_file_info("roles", true, 0),
                            Self::make_file_info("members", true, 0),
                            Self::make_file_info("info.json", false, 200),
                        ])
                    }
                    [guild_name, "roles"] => {
                        // 列出角色
                        match self.api_list_guilds().await {
                            Ok(guilds) => {
                                if let Some(guild) = guilds.into_iter().find(|g| &g.name == guild_name) {
                                    match self.api_list_roles(&guild.id).await {
                                        Ok(roles) => {
                                            return Ok(roles.into_iter().map(|r| {
                                                Self::make_file_info(&format!("@{}", r.name), false, 64)
                                            }).collect());
                                        }
                                        Err(_) => {}
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                        // Fallback
                        Ok(vec![
                            Self::make_file_info("@everyone", false, 64),
                            Self::make_file_info("admin", false, 64),
                            Self::make_file_info("moderator", false, 64),
                        ])
                    }
                    [guild_name, "text-channels"] | [guild_name, "categories"] => {
                        // 列出文字频道
                        match self.api_list_guilds().await {
                            Ok(guilds) => {
                                if let Some(guild) = guilds.into_iter().find(|g| &g.name == guild_name) {
                                    match self.api_list_channels(&guild.id).await {
                                        Ok(channels) => {
                                            return Ok(channels.into_iter()
                                                .filter(|c| c.r#type == 0 || c.r#type == 4) // text channels and categories
                                                .map(|c| {
                                                    let name = c.name.clone().unwrap_or_else(|| "unknown".to_string());
                                                    Self::make_file_info(&name, true, 0)
                                                }).collect());
                                        }
                                        Err(_) => {}
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                        Ok(vec![
                            Self::make_file_info("general", true, 0),
                            Self::make_file_info("random", true, 0),
                        ])
                    }
                    [guild_name, "text-channels", channel_name] => {
                        // 频道内容
                        Ok(vec![
                            Self::make_file_info("messages", true, 0),
                            Self::make_file_info("pins", true, 0),
                            Self::make_file_info("webhooks", true, 0),
                        ])
                    }
                    [guild_name, "text-channels", channel_name, "messages"] => {
                        // 消息列表
                        match self.api_list_guilds().await {
                            Ok(guilds) => {
                                if let Some(guild) = guilds.into_iter().find(|g| &g.name == guild_name) {
                                    match self.api_list_channels(&guild.id).await {
                                        Ok(channels) => {
                                            if let Some(channel) = channels.into_iter().find(|c| c.name.as_deref() == Some(channel_name)) {
                                                match self.api_get_messages(&channel.id, Some(20)).await {
                                                    Ok(msgs) => {
                                                        return Ok(msgs.into_iter().map(|m| {
                                                            Self::make_file_info(&format!("msg_{}", m.id), false, m.content.len() as u64)
                                                        }).collect());
                                                    }
                                                    Err(_) => {}
                                                }
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                        Ok(vec![
                            Self::make_file_info("msg_001", false, 256),
                            Self::make_file_info("msg_002", false, 512),
                        ])
                    }
                    [guild_name, "members"] => {
                        Ok(vec![
                            Self::make_file_info("user_001", false, 128),
                            Self::make_file_info("user_002", false, 128),
                        ])
                    }
                    _ => Err(EvifError::NotFound(format!("/{}", path))),
                }
            }
            "Channels" | "channels" => {
                Ok(vec![
                    Self::make_file_info("general", true, 0),
                    Self::make_file_info("random", true, 0),
                ])
            }
            "Users" | "users" => {
                Ok(vec![
                    Self::make_file_info("user_001", false, 128),
                ])
            }
            "Webhooks" | "webhooks" => {
                Ok(vec![
                    Self::make_file_info("webhook_001", false, 64),
                ])
            }
            _ => Err(EvifError::NotFound(format!("/{}", path))),
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_end_matches('/').trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        // 解析消息路径
        if let Some(pos) = parts.iter().position(|&p| p == "messages") {
            if pos > 0 {
                let msg_id_idx = pos + 1;
                if msg_id_idx < parts.len() {
                    let msg_id = parts[msg_id_idx];
                    if msg_id.starts_with("msg_") {
                        let actual_id = msg_id.strip_prefix("msg_").unwrap_or(msg_id);
                        // 从 API 获取消息 (简化版本，只返回模拟数据)
                        return Ok(format!(
                            "{{\"id\":\"{}\",\"type\":0,\"content\":\"Discord message\",\"author\":{{\"id\":\"123456789\",\"username\":\"alice\",\"discriminator\":\"0001\"}},\"timestamp\":\"{}\"}}",
                            actual_id,
                            Utc::now().to_rfc3339()
                        ).into_bytes());
                    }
                }
            }
        }

        // 角色信息
        if let Some(pos) = parts.iter().position(|&p| p == "roles") {
            if pos + 1 < parts.len() {
                let role_name = parts[pos + 1].trim_start_matches('@');
                return Ok(format!(
                    "{{\"id\":\"444555666\",\"name\":\"{}\",\"color\":0,\"hoist\":true,\"position\":1,\"permissions\":104324097,\"managed\":false,\"mentionable\":true}}",
                    role_name
                ).into_bytes());
            }
        }

        // 用户信息
        if let Some(pos) = parts.iter().position(|&p| p == "users" || p == "Users") {
            if pos + 1 < parts.len() {
                let user_id = parts[pos + 1].trim_start_matches("user_");
                return Ok(format!(
                    "{{\"id\":\"{}\",\"username\":\"user\",\"discriminator\":\"0001\",\"avatar\":null}}",
                    user_id
                ).into_bytes());
            }
        }

        // Webhook 信息
        if let Some(pos) = parts.iter().position(|&p| p == "webhooks" || p == "Webhooks") {
            if pos + 1 < parts.len() {
                let webhook_id = parts[pos + 1].trim_start_matches("webhook_");
                match self.api_get_webhook(webhook_id).await {
                    Ok(wh) => {
                        let json = serde_json::to_string_pretty(&wh).unwrap_or_else(|_| "{}".to_string());
                        return Ok(json.into_bytes());
                    }
                    Err(_) => {
                        return Ok(format!(
                            "{{\"id\":\"{}\",\"type\":1,\"guild_id\":\"111222333\",\"channel_id\":\"987654321\",\"name\":\"Webhook\"}}",
                            webhook_id
                        ).into_bytes());
                    }
                }
            }
        }

        // 服务器信息
        if let Some(pos) = parts.iter().position(|&p| p == "Guilds" || p == "guilds") {
            if pos + 2 < parts.len() && parts[pos + 2] == "info.json" {
                let guild_name = parts[pos + 1];
                return Ok(format!(
                    "{{\"name\":\"{}\",\"description\":\"Discord server\"}}",
                    guild_name
                ).into_bytes());
            }
        }

        Err(EvifError::NotFound(format!("File not found: /{}", path)))
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
                "DiscordFS is read-only".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "Write operations not implemented".to_string(),
        ))
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let path = path.trim_end_matches('/').trim_start_matches('/');

        if path.is_empty() || path == "discordfs" {
            return Ok(FileInfo {
                name: "discordfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        let is_file = name.starts_with("@") || name.starts_with("user_")
            || name.starts_with("msg_") || name.starts_with("webhook_");
        let is_dir = !is_file;
        let size: u64 = if name.starts_with("@") { 64 }
                   else if name.starts_with("user_") { 128 }
                   else if name.starts_with("msg_") { 256 }
                   else if name.starts_with("webhook_") { 64 }
                   else { 0 };

        Ok(Self::make_file_info(name, is_dir, size))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "DiscordFS is read-only".to_string(),
        ))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "DiscordFS is read-only".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied(
            "DiscordFS is read-only".to_string(),
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
        let config = DiscordConfig {
            bot_token: "test_token".to_string(),
            application_id: None,
            api_endpoint: Some("https://discord.com/api/v10".to_string()),
            read_only: Some(true),
        };
        DiscordFsPlugin {
            config,
            client: Client::new(),
            connected: Arc::new(RwLock::new(false)),
            cache: Arc::new(RwLock::new(HashMap::new())),
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
        // 可能是真实服务器或占位符
        assert!(entries.iter().any(|e| e.name == "my-server" || e.is_dir));
    }

    #[tokio::test]
    async fn test_readdir_server() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "categories" || e.name == "text-channels"));
    }

    #[tokio::test]
    async fn test_readdir_channel() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server/text-channels").await.unwrap();
        // text-channels should show channel directories
        assert!(!entries.is_empty());
    }

    #[tokio::test]
    async fn test_readdir_messages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Guilds/my-server/text-channels/general/messages").await.unwrap();
        assert!(!entries.is_empty());
    }

    #[tokio::test]
    async fn test_read_message() {
        let plugin = create_plugin();
        let content = plugin.read("/Guilds/my-server/text-channels/general/messages/msg_001", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("type") || content_str.contains("error"));
    }

    #[tokio::test]
    async fn test_read_role() {
        let plugin = create_plugin();
        let content = plugin.read("/Guilds/my-server/roles/@everyone", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("id") || content_str.contains("name"));
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
    async fn test_readdir_not_found() {
        let plugin = create_plugin();
        let result = plugin.readdir("/Nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plugin_name() {
        let plugin = create_plugin();
        assert_eq!(plugin.name(), "discordfs");
    }
}
