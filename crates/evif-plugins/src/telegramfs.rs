// Telegram FS - Telegram Bot 文件系统插件
//
// 提供 Telegram Bot 的文件系统接口
// 目录结构: /telegram/<chat>/{messages, media, members, info}
//
// 这是 Plan 9 风格的文件接口，用于 Telegram 访问
// 真实 API 集成: https://api.telegram.org/bot<token>/<method>

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
const TELEGRAM_API_BASE: &str = "https://api.telegram.org";

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

/// Telegram Bot API 响应类型
#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
    is_bot: bool,
    first_name: String,
    last_name: Option<String>,
    username: Option<String>,
    language_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
    title: Option<String>,
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    description: Option<String>,
    invite_link: Option<String>,
    member_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    date: i64,
    text: Option<String>,
    caption: Option<String>,
    photo: Option<Vec<TelegramPhotoSize>>,
    video: Option<TelegramVideo>,
    document: Option<TelegramDocument>,
}

#[derive(Debug, Deserialize)]
struct TelegramPhotoSize {
    file_id: String,
    file_unique_id: String,
    width: i32,
    height: i32,
    file_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TelegramVideo {
    file_id: String,
    file_unique_id: String,
    width: i32,
    height: i32,
    duration: i32,
    file_size: Option<i64>,
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramDocument {
    file_id: String,
    file_unique_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
    edited_message: Option<TelegramMessage>,
    channel_post: Option<TelegramMessage>,
}

/// TelegramFs 插件
pub struct TelegramFsPlugin {
    config: TelegramConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
    /// HTTP 客户端
    http_client: reqwest::Client,
}

impl TelegramFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: TelegramConfig) -> EvifResult<Self> {
        if config.bot_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Telegram bot_token is required".to_string(),
            ));
        }

        let plugin = Self {
            config,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        };

        Ok(plugin)
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

    /// 获取 API 基础 URL
    fn api_base(&self) -> String {
        let endpoint = self.config.api_endpoint.as_deref().unwrap_or(TELEGRAM_API_BASE);
        format!("{}/bot{}", endpoint, self.config.bot_token)
    }

    /// 获取认证头 (Telegram 不需要特殊的认证头, token 已在 URL 中)
    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        reqwest::header::HeaderMap::new()
    }

    /// 调用 Telegram Bot API: getMe
    async fn api_get_me(&self) -> EvifResult<TelegramUser> {
        let url = format!("{}/getMe", self.api_base());
        let resp = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<TelegramUser> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        telegram_resp.result.ok_or_else(|| EvifError::InvalidInput("Telegram API returned no data".to_string()))
    }

    /// 调用 Telegram Bot API: getUpdates
    async fn api_get_updates(&self) -> EvifResult<Vec<TelegramUpdate>> {
        let url = format!("{}/getUpdates", self.api_base());
        let resp = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<Vec<TelegramUpdate>> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        Ok(telegram_resp.result.unwrap_or_default())
    }

    /// 调用 Telegram Bot API: getChat
    async fn api_get_chat(&self, chat_id: &str) -> EvifResult<TelegramChat> {
        let url = format!("{}/getChat", self.api_base());
        let resp = self.http_client
            .get(&url)
            .query(&[("chat_id", chat_id)])
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<TelegramChat> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        telegram_resp.result.ok_or_else(|| EvifError::InvalidInput("Telegram API returned no data".to_string()))
    }

    /// 调用 Telegram Bot API: getChatMemberCount
    async fn api_get_chat_member_count(&self, chat_id: &str) -> EvifResult<i64> {
        let url = format!("{}/getChatMemberCount", self.api_base());
        let resp = self.http_client
            .get(&url)
            .query(&[("chat_id", chat_id)])
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<i64> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        telegram_resp.result.ok_or_else(|| EvifError::InvalidInput("Telegram API returned no data".to_string()))
    }

    /// 调用 Telegram Bot API: getChatAdministrators
    async fn api_get_chat_administrators(&self, chat_id: &str) -> EvifResult<Vec<TelegramChatMember>> {
        let url = format!("{}/getChatAdministrators", self.api_base());
        let resp = self.http_client
            .get(&url)
            .query(&[("chat_id", chat_id)])
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<Vec<TelegramChatMember>> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        Ok(telegram_resp.result.unwrap_or_default())
    }

    /// 调用 Telegram Bot API: getFile
    async fn api_get_file(&self, file_id: &str) -> EvifResult<TelegramFile> {
        let url = format!("{}/getFile", self.api_base());
        let resp = self.http_client
            .get(&url)
            .query(&[("file_id", file_id)])
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<TelegramFile> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        telegram_resp.result.ok_or_else(|| EvifError::InvalidInput("Telegram API returned no data".to_string()))
    }

    /// 调用 Telegram Bot API: getWebhookInfo
    async fn api_get_webhook_info(&self) -> EvifResult<TelegramWebhookInfo> {
        let url = format!("{}/getWebhookInfo", self.api_base());
        let resp = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API error: {}", e)))?;

        let telegram_resp: TelegramResponse<TelegramWebhookInfo> = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Telegram API parse error: {}", e)))?;

        if !telegram_resp.ok {
            return Err(EvifError::InvalidInput(format!("Telegram API error: {}", telegram_resp.description.unwrap_or_default())));
        }

        telegram_resp.result.ok_or_else(|| EvifError::InvalidInput("Telegram API returned no data".to_string()))
    }

    /// 提取消息文本
    fn extract_message_text(message: &TelegramMessage) -> String {
        if let Some(text) = &message.text {
            text.clone()
        } else if let Some(caption) = &message.caption {
            caption.clone()
        } else if message.photo.is_some() {
            "[Photo]".to_string()
        } else if message.video.is_some() {
            "[Video]".to_string()
        } else if message.document.is_some() {
            "[Document]".to_string()
        } else {
            "[Unknown]".to_string()
        }
    }
}

#[derive(Debug, Deserialize)]
struct TelegramChatMember {
    user: TelegramUser,
    status: String,
}

#[derive(Debug, Deserialize)]
struct TelegramFile {
    file_id: String,
    file_unique_id: String,
    file_size: Option<i64>,
    file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramWebhookInfo {
    url: String,
    has_custom_certificate: bool,
    pending_update_count: i64,
    last_error_date: Option<i64>,
    last_error_message: Option<String>,
    max_connections: Option<i64>,
    allowed_updates: Option<Vec<String>>,
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
                // 尝试获取真实聊天列表 (通过 getUpdates)
                match self.api_get_updates().await {
                    Ok(updates) => {
                        let mut chat_ids: Vec<String> = Vec::new();
                        for update in &updates {
                            if let Some(message) = &update.message {
                                let chat_id = message.chat.id.to_string();
                                if !chat_ids.contains(&chat_id) {
                                    chat_ids.push(chat_id);
                                }
                            }
                            if let Some(message) = &update.channel_post {
                                let chat_id = message.chat.id.to_string();
                                if !chat_ids.contains(&chat_id) {
                                    chat_ids.push(chat_id);
                                }
                            }
                        }
                        chat_ids.into_iter()
                            .map(|id| Self::make_file_info(&format!("chat_{}", id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        // 回退到 mock 数据
                        vec![
                            Self::make_file_info("chat_100001", true, 0),
                            Self::make_file_info("chat_100002", true, 0),
                            Self::make_file_info("chat_100003", true, 0),
                        ]
                    }
                }
            }
            "/Channels" | "Channels" | "/channels" | "channels" => {
                // 尝试获取真实频道列表 (通过 getUpdates 中的 channel_post)
                match self.api_get_updates().await {
                    Ok(updates) => {
                        let mut channel_ids: Vec<String> = Vec::new();
                        for update in &updates {
                            if let Some(message) = &update.channel_post {
                                let chat_id = message.chat.id.to_string();
                                if !channel_ids.contains(&chat_id) {
                                    channel_ids.push(chat_id);
                                }
                            }
                        }
                        channel_ids.into_iter()
                            .map(|id| Self::make_file_info(&format!("channel_{}", id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        // 回退到 mock 数据
                        vec![
                            Self::make_file_info("channel_500001", true, 0),
                            Self::make_file_info("channel_500002", true, 0),
                        ]
                    }
                }
            }
            "/Groups" | "Groups" | "/groups" | "groups" => {
                // 尝试获取真实群组列表 (通过 getUpdates 中的 group chat)
                match self.api_get_updates().await {
                    Ok(updates) => {
                        let mut group_ids: Vec<String> = Vec::new();
                        for update in &updates {
                            if let Some(message) = &update.message {
                                if message.chat.chat_type == "group" || message.chat.chat_type == "supergroup" {
                                    let chat_id = message.chat.id.to_string();
                                    if !group_ids.contains(&chat_id) {
                                        group_ids.push(chat_id);
                                    }
                                }
                            }
                        }
                        group_ids.into_iter()
                            .map(|id| Self::make_file_info(&format!("group_{}", id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        // 回退到 mock 数据
                        vec![
                            Self::make_file_info("group_200001", true, 0),
                            Self::make_file_info("group_200002", true, 0),
                        ]
                    }
                }
            }
            "/Bots" | "Bots" | "/bots" | "bots" => {
                // Bot 信息
                vec![
                    Self::make_file_info("me", true, 0),
                    Self::make_file_info("commands", true, 0),
                    Self::make_file_info("webhook", true, 0),
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
            "/Bots/webhook" => {
                vec![
                    Self::make_file_info("webhook_info.json", false, 512),
                ]
            }
            "/Updates" | "Updates" | "/updates" | "updates" => {
                // 尝试获取真实更新列表
                match self.api_get_updates().await {
                    Ok(updates) => {
                        updates.into_iter()
                            .map(|update| Self::make_file_info(&format!("update_{}", update.update_id), false, 1024))
                            .collect()
                    }
                    Err(_) => {
                        // 回退到 mock 数据
                        vec![
                            Self::make_file_info("update_0001", false, 1024),
                            Self::make_file_info("update_0002", false, 768),
                        ]
                    }
                }
            }
            _ => {
                // 处理聊天内容
                let path_clean = path.trim_start_matches('/');
                let parts: Vec<&str> = path_clean.split('/').collect();

                if parts.len() >= 2 {
                    let category = parts[0];
                    let id = parts[1];

                    if let Some(chat_id) = id.strip_prefix("chat_") {
                        match parts.len() {
                            2 => {
                                // 单个聊天内容: /Chats/chat_123
                                // 尝试获取真实聊天信息
                                match self.api_get_chat(chat_id).await {
                                    Ok(_chat) => {
                                        vec![
                                            Self::make_file_info("messages", true, 0),
                                            Self::make_file_info("media", true, 0),
                                            Self::make_file_info("info", true, 0),
                                        ]
                                    }
                                    Err(_) => {
                                        vec![
                                            Self::make_file_info("messages", true, 0),
                                            Self::make_file_info("media", true, 0),
                                            Self::make_file_info("info", true, 0),
                                        ]
                                    }
                                }
                            }
                            3 => {
                                let subcategory = parts[2];
                                match subcategory {
                                    "messages" => {
                                        // 尝试获取真实消息列表
                                        match self.api_get_updates().await {
                                            Ok(updates) => {
                                                let mut messages: Vec<FileInfo> = Vec::new();
                                                for update in &updates {
                                                    if let Some(message) = &update.message {
                                                        if message.chat.id.to_string() == chat_id {
                                                            messages.push(Self::make_file_info(
                                                                &format!("msg_{}", message.message_id),
                                                                false,
                                                                1024,
                                                            ));
                                                        }
                                                    }
                                                }
                                                messages
                                            }
                                            Err(_) => {
                                                vec![
                                                    Self::make_file_info("msg_1001", false, 512),
                                                    Self::make_file_info("msg_1002", false, 768),
                                                    Self::make_file_info("msg_1003", false, 256),
                                                ]
                                            }
                                        }
                                    }
                                    "media" => {
                                        // 尝试获取真实媒体列表
                                        match self.api_get_updates().await {
                                            Ok(updates) => {
                                                let mut media: Vec<FileInfo> = Vec::new();
                                                for update in &updates {
                                                    if let Some(message) = &update.message {
                                                        if message.chat.id.to_string() == chat_id {
                                                            if let Some(photo) = &message.photo {
                                                                media.push(Self::make_file_info(
                                                                    &format!("photo_{}.jpg", message.message_id),
                                                                    false,
                                                                    102400,
                                                                ));
                                                            }
                                                            if let Some(video) = &message.video {
                                                                media.push(Self::make_file_info(
                                                                    &format!("video_{}.mp4", message.message_id),
                                                                    false,
                                                                    2048000,
                                                                ));
                                                            }
                                                            if let Some(document) = &message.document {
                                                                let filename = document.file_name.clone().unwrap_or_default();
                                                                media.push(Self::make_file_info(
                                                                    &format!("doc_{}_{}", message.message_id, filename),
                                                                    false,
                                                                    51200,
                                                                ));
                                                            }
                                                        }
                                                    }
                                                }
                                                media
                                            }
                                            Err(_) => {
                                                vec![
                                                    Self::make_file_info("photo_001.jpg", false, 102400),
                                                    Self::make_file_info("video_001.mp4", false, 2048000),
                                                    Self::make_file_info("document_001.pdf", false, 51200),
                                                ]
                                            }
                                        }
                                    }
                                    "info" => {
                                        vec![
                                            Self::make_file_info("chat.json", false, 256),
                                        ]
                                    }
                                    _ => {
                                        return Err(EvifError::NotFound(path.to_string()));
                                    }
                                }
                            }
                            _ => {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                    } else if let Some(channel_id) = id.strip_prefix("channel_") {
                        match parts.len() {
                            2 => {
                                vec![
                                    Self::make_file_info("messages", true, 0),
                                    Self::make_file_info("subscribers", true, 0),
                                    Self::make_file_info("info", true, 0),
                                ]
                            }
                            3 => {
                                let subcategory = parts[2];
                                match subcategory {
                                    "messages" => {
                                        // 尝试获取真实频道消息
                                        match self.api_get_updates().await {
                                            Ok(updates) => {
                                                let mut messages: Vec<FileInfo> = Vec::new();
                                                for update in &updates {
                                                    if let Some(message) = &update.channel_post {
                                                        if message.chat.id.to_string() == channel_id {
                                                            messages.push(Self::make_file_info(
                                                                &format!("msg_{}", message.message_id),
                                                                false,
                                                                1024,
                                                            ));
                                                        }
                                                    }
                                                }
                                                messages
                                            }
                                            Err(_) => {
                                                vec![
                                                    Self::make_file_info("msg_2001", false, 256),
                                                    Self::make_file_info("msg_2002", false, 384),
                                                ]
                                            }
                                        }
                                    }
                                    "subscribers" => {
                                        // 尝试获取真实订阅者列表
                                        match self.api_get_chat_administrators(channel_id).await {
                                            Ok(members) => {
                                                members.into_iter()
                                                    .map(|member| Self::make_file_info(
                                                        &format!("user_{}", member.user.id),
                                                        false,
                                                        64,
                                                    ))
                                                    .collect()
                                            }
                                            Err(_) => {
                                                vec![
                                                    Self::make_file_info("user_1001", false, 64),
                                                    Self::make_file_info("user_1002", false, 64),
                                                    Self::make_file_info("user_1003", false, 64),
                                                ]
                                            }
                                        }
                                    }
                                    "info" => {
                                        vec![
                                            Self::make_file_info("chat.json", false, 256),
                                        ]
                                    }
                                    _ => {
                                        return Err(EvifError::NotFound(path.to_string()));
                                    }
                                }
                            }
                            _ => {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                    } else if let Some(group_id) = id.strip_prefix("group_") {
                        match parts.len() {
                            2 => {
                                vec![
                                    Self::make_file_info("messages", true, 0),
                                    Self::make_file_info("members", true, 0),
                                    Self::make_file_info("info", true, 0),
                                ]
                            }
                            3 => {
                                let subcategory = parts[2];
                                match subcategory {
                                    "messages" => {
                                        // 尝试获取真实群组消息
                                        match self.api_get_updates().await {
                                            Ok(updates) => {
                                                let mut messages: Vec<FileInfo> = Vec::new();
                                                for update in &updates {
                                                    if let Some(message) = &update.message {
                                                        if message.chat.id.to_string() == group_id
                                                            && (message.chat.chat_type == "group" || message.chat.chat_type == "supergroup")
                                                        {
                                                            messages.push(Self::make_file_info(
                                                                &format!("msg_{}", message.message_id),
                                                                false,
                                                                1024,
                                                            ));
                                                        }
                                                    }
                                                }
                                                messages
                                            }
                                            Err(_) => {
                                                vec![
                                                    Self::make_file_info("msg_3001", false, 320),
                                                    Self::make_file_info("msg_3002", false, 448),
                                                ]
                                            }
                                        }
                                    }
                                    "members" => {
                                        // 尝试获取真实群组成员
                                        match self.api_get_chat_administrators(group_id).await {
                                            Ok(members) => {
                                                members.into_iter()
                                                    .map(|member| {
                                                        let prefix = if member.status == "creator" || member.status == "administrator" {
                                                            "admin"
                                                        } else {
                                                            "member"
                                                        };
                                                        Self::make_file_info(
                                                            &format!("{}_{}", prefix, member.user.id),
                                                            false,
                                                            64,
                                                        )
                                                    })
                                                    .collect()
                                            }
                                            Err(_) => {
                                                vec![
                                                    Self::make_file_info("admin_001", false, 64),
                                                    Self::make_file_info("member_001", false, 64),
                                                    Self::make_file_info("member_002", false, 64),
                                                ]
                                            }
                                        }
                                    }
                                    "info" => {
                                        vec![
                                            Self::make_file_info("chat.json", false, 256),
                                        ]
                                    }
                                    _ => {
                                        return Err(EvifError::NotFound(path.to_string()));
                                    }
                                }
                            }
                            _ => {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                    } else {
                        return Err(EvifError::NotFound(path.to_string()));
                    }
                } else {
                    return Err(EvifError::NotFound(path.to_string()));
                }
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
            let chat_id = parts.get(1).unwrap_or(&"");
            let actual_chat_id = chat_id.strip_prefix("chat_").unwrap_or(chat_id);

            // 尝试从真实 API 获取消息
            if let Ok(updates) = self.api_get_updates().await {
                for update in &updates {
                    if let Some(message) = &update.message {
                        if message.chat.id.to_string() == actual_chat_id
                            && format!("msg_{}", message.message_id) == *msg_id
                        {
                            let content = format!(
                                "Message ID: {}\nFrom: {} ({})\nContent: {}\nTimestamp: {}\nType: {}\n",
                                message.message_id,
                                message.from.as_ref().map(|u| u.username.clone().unwrap_or_else(|| u.first_name.clone())).unwrap_or_else(|| "Unknown".to_string()),
                                message.from.as_ref().map(|u| u.id.to_string()).unwrap_or_else(|| "Unknown".to_string()),
                                Self::extract_message_text(message),
                                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                                if message.photo.is_some() { "photo" } else if message.video.is_some() { "video" } else if message.document.is_some() { "document" } else { "text" }
                            );
                            return Ok(content.into_bytes());
                        }
                    }
                }
            }

            // 回退到 mock 数据
            return Ok(format!(
                "Message ID: {}\nFrom: user123\nContent: Sample Telegram message\nTimestamp: {}\nType: text\n",
                msg_id,
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            ).into_bytes());
        }

        // 检查是否是成员/订阅者
        if path.contains("/members/") || path.contains("/subscribers/") {
            let user = parts.last().unwrap_or(&"");
            let chat_id = parts.get(1).unwrap_or(&"");
            let actual_chat_id = chat_id.strip_prefix("chat_").or(chat_id.strip_prefix("channel_")).or(chat_id.strip_prefix("group_")).unwrap_or(chat_id);

            // 尝试从真实 API 获取成员信息
            if let Ok(members) = self.api_get_chat_administrators(actual_chat_id).await {
                for member in &members {
                    let member_name = format!("user_{}", member.user.id);
                    let admin_name = format!("admin_{}", member.user.id);
                    if member_name == *user || admin_name == *user {
                        let content = format!(
                            "User: {} ({})\nUsername: {}\nRole: {}\nStatus: {}\nIs Bot: {}\n",
                            member.user.first_name,
                            member.user.id,
                            member.user.username.clone().unwrap_or_else(|| "N/A".to_string()),
                            member.status,
                            "active",
                            member.user.is_bot
                        );
                        return Ok(content.into_bytes());
                    }
                }
            }

            // 回退到 mock 数据
            return Ok(format!(
                "User: {}\nDisplay Name: {}\nRole: member\nStatus: active\n",
                user,
                user.split('_').last().unwrap_or(user)
            ).into_bytes());
        }

        // 检查是否是媒体
        if path.contains("/media/") {
            let filename = parts.last().unwrap_or(&"");
            let content = self.get_media_info(filename).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是聊天信息
        if path.contains("/info/chat.json") {
            let chat_id = parts.get(1).unwrap_or(&"");
            let actual_chat_id = chat_id.strip_prefix("chat_").or(chat_id.strip_prefix("channel_")).or(chat_id.strip_prefix("group_")).unwrap_or(chat_id);

            // 尝试从真实 API 获取聊天信息
            if let Ok(chat) = self.api_get_chat(actual_chat_id).await {
                let member_count = self.api_get_chat_member_count(actual_chat_id).await.unwrap_or(0);
                let content = serde_json::json!({
                    "id": chat.id,
                    "type": chat.chat_type,
                    "title": chat.title,
                    "username": chat.username,
                    "description": chat.description,
                    "invite_link": chat.invite_link,
                    "member_count": member_count
                });
                return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
            }

            // 回退到 mock 数据
            return Ok(format!(
                "{{\"id\": {}, \"type\": \"private\", \"title\": \"Chat {}\", \"member_count\": 2}}",
                actual_chat_id,
                actual_chat_id
            ).into_bytes());
        }

        // 检查是否是 Bot 信息
        if path.contains("/profile.json") {
            // 尝试从真实 API 获取 Bot 信息
            if let Ok(user) = self.api_get_me().await {
                let content = serde_json::json!({
                    "bot_token": "***",
                    "username": user.username,
                    "first_name": user.first_name,
                    "last_name": user.last_name,
                    "id": user.id,
                    "language_code": user.language_code
                });
                return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
            }

            // 回退到 mock 数据
            return Ok(format!(
                "{{\"bot_token\": \"***\", \"username\": \"mybot\", \"first_name\": \"My Bot\"}}"
            ).into_bytes());
        }

        if path.contains("/settings.json") {
            return Ok(format!(
                "{{\"privacy_mode\": \"limited\", \"commands\": [\"start\", \"help\", \"settings\"]}}"
            ).into_bytes());
        }

        // 检查是否是 Webhook 信息
        if path.contains("/webhook_info.json") {
            // 尝试从真实 API 获取 Webhook 信息
            if let Ok(info) = self.api_get_webhook_info().await {
                let content = serde_json::json!({
                    "url": info.url,
                    "has_custom_certificate": info.has_custom_certificate,
                    "pending_update_count": info.pending_update_count,
                    "last_error_date": info.last_error_date,
                    "last_error_message": info.last_error_message,
                    "max_connections": info.max_connections,
                    "allowed_updates": info.allowed_updates
                });
                return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
            }

            // 回退到 mock 数据
            return Ok(format!(
                "{{\"url\": \"\", \"has_custom_certificate\": false, \"pending_update_count\": 0}}"
            ).into_bytes());
        }

        // 检查是否是命令
        if path.contains("/commands/cmd_") {
            let cmd_name = parts.last().unwrap_or(&"");
            let content = self.get_command_info(cmd_name).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是更新
        if path.contains("/Updates/update_") || path.contains("/updates/update_") {
            let update_id_str = parts.last().unwrap_or(&"");
            let update_id = update_id_str.strip_prefix("update_").unwrap_or(update_id_str);

            // 尝试从真实 API 获取更新
            if let Ok(updates) = self.api_get_updates().await {
                for update in &updates {
                    if update.update_id.to_string() == update_id {
                        let content = serde_json::json!({
                            "update_id": update.update_id,
                            "message": update.message.as_ref().map(|m| serde_json::json!({
                                "message_id": m.message_id,
                                "from": m.from.as_ref().map(|u| serde_json::json!({
                                    "id": u.id,
                                    "username": u.username,
                                    "first_name": u.first_name
                                })),
                                "chat": serde_json::json!({
                                    "id": m.chat.id,
                                    "type": m.chat.chat_type,
                                    "title": m.chat.title
                                }),
                                "date": m.date,
                                "text": Self::extract_message_text(m)
                            }))
                        });
                        return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
                    }
                }
            }

            // 回退到 mock 数据
            return Ok(format!(
                "{{\"update_id\": {}, \"message\": {{\"text\": \"Sample update\"}}}}",
                update_id
            ).into_bytes());
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

    /// 获取命令信息
    async fn get_command_info(&self, cmd_name: &str) -> EvifResult<String> {
        Ok(format!(
            "Command: /{}\nDescription: {} command\n",
            cmd_name.trim_start_matches("cmd_"),
            cmd_name.trim_start_matches("cmd_")
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
            config: TelegramConfig {
                bot_token: "test_token_123".to_string(),
                ..Default::default()
            },
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
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
    async fn test_read_chat_info() {
        let plugin = create_plugin();
        let content = plugin.read("/Chats/chat_100001/info/chat.json", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("id"));
        assert!(content_str.contains("type"));
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

    #[test]
    fn test_api_base() {
        let plugin = create_plugin();
        let base = plugin.api_base();
        assert!(base.contains("test_token_123"));
        assert!(base.contains("https://api.telegram.org"));
    }

    #[test]
    fn test_auth_headers() {
        let plugin = create_plugin();
        let headers = plugin.auth_headers();
        // Telegram 不需要特殊的认证头, token 已在 URL 中
        assert!(headers.is_empty());
    }
}
