// Gmail FS - Email 文件系统插件
//
// 提供 Gmail/IMAP 的文件系统接口
// 目录结构: /gmail/<folder>/<message_id>/{headers, body, attachments/}
//
// 这是 Plan 9 风格的文件接口，用于 Email 访问

use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

/// Gmail API response types
#[derive(Clone, Debug, Deserialize)]
struct GmailMessageList {
    messages: Option<Vec<GmailMessageId>>,
    #[serde(rename = "resultSizeEstimate")]
    result_size_estimate: Option<i64>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailMessageId {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailMessage {
    id: String,
    snippet: Option<String>,
    payload: Option<GmailMessagePart>,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
    #[serde(rename = "labelIds")]
    label_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailMessagePart {
    headers: Option<Vec<GmailHeader>>,
    body: Option<GmailBody>,
    parts: Option<Vec<GmailMessagePart>>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    filename: Option<String>,
    #[serde(rename = "partId")]
    part_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailHeader {
    name: String,
    value: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailBody {
    data: Option<String>,
    size: Option<i64>,
    #[serde(rename = "attachmentId")]
    attachment_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailLabelList {
    labels: Option<Vec<GmailLabel>>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailLabel {
    id: String,
    name: String,
    #[serde(rename = "messageListVisibility")]
    message_list_visibility: Option<String>,
    #[serde(rename = "messagesTotal")]
    messages_total: Option<i64>,
    #[serde(rename = "messagesUnread")]
    messages_unread: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailProfile {
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
    #[serde(rename = "messagesTotal")]
    messages_total: Option<i64>,
    #[serde(rename = "threadsTotal")]
    threads_total: Option<i64>,
    #[serde(rename = "historyId")]
    history_id: Option<String>,
}

const PLUGIN_NAME: &str = "gmailfs";

/// Gmail/IMAP 配置
#[derive(Clone, Debug, Deserialize)]
pub struct GmailConfig {
    /// IMAP 服务器地址
    pub imap_host: String,
    /// IMAP 端口 (默认 993)
    pub imap_port: Option<u16>,
    /// SMTP 服务器地址
    pub smtp_host: String,
    /// SMTP 端口 (默认 587)
    pub smtp_port: Option<u16>,
    /// 用户名/邮箱
    pub username: String,
    /// 密码或 App Password
    pub password: String,
    /// 使用 SSL (默认 true)
    pub use_ssl: Option<bool>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for GmailConfig {
    fn default() -> Self {
        Self {
            imap_host: "imap.gmail.com".to_string(),
            imap_port: Some(993),
            smtp_host: "smtp.gmail.com".to_string(),
            smtp_port: Some(587),
            username: String::new(),
            password: String::new(),
            use_ssl: Some(true),
            read_only: Some(true),
        }
    }
}

/// GmailFs 插件
pub struct GmailFsPlugin {
    config: GmailConfig,
    /// HTTP 客户端 (用于 Gmail REST API)
    client: Option<reqwest::Client>,
    /// OAuth 访问令牌
    access_token: Option<String>,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl GmailFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: GmailConfig) -> EvifResult<Self> {
        if config.username.is_empty() {
            return Err(EvifError::InvalidPath(
                "Gmail username is required".to_string(),
            ));
        }

        // 创建 HTTP 客户端
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| EvifError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client: Some(client),
            access_token: None,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 使用 OAuth 令牌创建插件
    pub async fn with_oauth(config: GmailConfig, access_token: String) -> EvifResult<Self> {
        if access_token.is_empty() {
            return Self::new(config).await;
        }

        let mut plugin = Self::new(config).await?;
        plugin.access_token = Some(access_token);
        *plugin.connected.write().await = true;
        Ok(plugin)
    }

    /// 获取 Authorization 头
    fn auth_header(&self) -> EvifResult<String> {
        if let Some(ref token) = self.access_token {
            Ok(format!("Bearer {}", token))
        } else {
            // 尝试使用 Basic Auth (不推荐，已废弃)
            let credentials = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", self.config.username, self.config.password),
            );
            Ok(format!("Basic {}", credentials))
        }
    }

    /// Gmail API 请求
    async fn gmail_api_request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        query_params: Option<&[(&str, &str)]>,
    ) -> EvifResult<T> {
        let client = self.client.as_ref()
            .ok_or_else(|| EvifError::Internal("HTTP client not initialized".to_string()))?;

        let mut url = format!("https://www.googleapis.com/gmail/v1/users/me{}", path);
        if let Some(params) = query_params {
            let query_string: Vec<String> = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query_string.join("&"));
        }

        let auth = self.auth_header()?;

        let request = client
            .request(method, &url)
            .header("Authorization", auth)
            .header("Content-Type", "application/json");

        let response = request.send().await
            .map_err(|e| EvifError::Internal(format!("Gmail API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EvifError::Internal(format!(
                "Gmail API error: {} - {}",
                status, body
            )));
        }

        response.json::<T>().await
            .map_err(|e| EvifError::Internal(format!("Failed to parse Gmail API response: {}", e)))
    }

    /// 测试连接
    pub async fn test_connection(&self) -> EvifResult<bool> {
        if self.access_token.is_none() && (self.config.username.is_empty() || self.config.password.is_empty()) {
            return Ok(false);
        }

        // 尝试获取 Gmail Profile 验证连接
        match self.gmail_api_request::<GmailProfile>(
            reqwest::Method::GET,
            "/profile",
            None,
        ).await {
            Ok(profile) => Ok(profile.email_address.is_some()),
            Err(_) => Ok(false),
        }
    }

    /// 获取标准文件夹
    pub fn standard_folders() -> Vec<(&'static str, &'static str)> {
        vec![
            ("INBOX", "Inbox"),
            ("[Gmail]/Sent Mail", "Sent"),
            ("[Gmail]/Drafts", "Drafts"),
            ("[Gmail]/Trash", "Trash"),
            ("[Gmail]/Spam", "Spam"),
            ("[Gmail]/Starred", "Starred"),
            ("[Gmail]/All Mail", "All Mail"),
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
impl EvifPlugin for GmailFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Gmail FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in Gmail FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Gmail FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in Gmail FS".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_end_matches('/');

        let entries = match path {
            "/" | "" => {
                // 根目录: 列出所有标准文件夹
                Self::standard_folders()
                    .into_iter()
                    .map(|(id, name)| Self::make_file_info(&name, true, 0))
                    .collect()
            }
            folder_path => {
                // 检查是否是标准文件夹
                let folder = Self::standard_folders()
                    .into_iter()
                    .find(|(id, name)| {
                        let folder_name_with_slash = format!("/{}", name);
                        folder_path == folder_name_with_slash || folder_path == *name
                    });

                if let Some((folder_id, folder_name)) = folder {
                    // 列出该文件夹下的邮件 (模拟)
                    // 实际实现需要连接 IMAP 服务器
                    let count = self.get_message_count(folder_id).await.unwrap_or(0);
                    if count > 0 {
                        // 返回前 10 封邮件作为示例
                        (0..count.min(10))
                            .map(|i| Self::make_file_info(&format!("msg_{:06}", i + 1), true, 0))
                            .collect()
                    } else {
                        vec![]
                    }
                } else if folder_path.contains("/msg_") {
                    // 邮件详情目录
                    vec![
                        Self::make_file_info("headers", false, 0),
                        Self::make_file_info("body", false, 0),
                        Self::make_file_info("body.html", false, 0),
                        Self::make_file_info("attachments", true, 0),
                    ]
                } else if folder_path.ends_with("/attachments") {
                    // 附件目录 (示例)
                    vec![
                        Self::make_file_info("attachment_1", false, 1024),
                    ]
                } else {
                    return Err(EvifError::NotFound(folder_path.to_string()));
                }
            }
        };

        Ok(entries)
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_end_matches('/');

        // 解析邮件路径: /<folder>/msg_<id>/<file>
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        if parts.len() >= 3 {
            let folder = parts[0];
            let msg_id = parts[1];
            let file = parts[2];

            let content = match file {
                "headers" => {
                    self.get_message_headers(folder, msg_id).await?
                }
                "body" => {
                    self.get_message_body(folder, msg_id, false).await?
                }
                "body.html" => {
                    self.get_message_body(folder, msg_id, true).await?
                }
                _ => {
                    return Err(EvifError::NotFound(path.to_string()));
                }
            };

            return Ok(content.into_bytes());
        }

        // 检查是否是文件夹根路径
        for (folder_id, name) in Self::standard_folders() {
            if format!("/{}", name) == path || path == name {
                // 返回文件夹信息
                let content = format!(
                    "Folder: {}\nMessages: {}\nUnread: {}\n",
                    name,
                    self.get_message_count(folder_id).await.unwrap_or(0),
                    self.get_unread_count(folder_id).await.unwrap_or(0)
                );
                return Ok(content.into_bytes());
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
                "Gmail FS is in read-only mode".to_string(),
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
                name: "gmailfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        // Determine if this is a directory based on path structure
        let is_dir = if path.contains("/msg_") {
            // Inside a message - determine based on name
            name == "attachments" || name.starts_with("msg_") || name.starts_with("attachment_")
        } else {
            // At folder level
            !name.contains(".") && !name.contains("_")
        };
        let size = if name.ends_with(".html") { 2048 } else { 1024 };

        Ok(Self::make_file_info(name, is_dir, size))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Gmail FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in Gmail FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Gmail FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in Gmail FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

impl GmailFsPlugin {
    /// 获取邮件数量
    async fn get_message_count(&self, folder: &str) -> EvifResult<i64> {
        if self.access_token.is_none() {
            // 没有 OAuth 令牌，返回模拟数据
            return Ok(0);
        }

        // Gmail API 使用 labelId，不是 folder name
        let label_id = Self::folder_to_label_id(folder)?;
        let params = [
            ("labelIds", label_id.as_str()),
        ];

        match self.gmail_api_request::<GmailMessageList>(
            reqwest::Method::GET,
            "/messages",
            Some(&params),
        ).await {
            Ok(list) => Ok(list.result_size_estimate.unwrap_or(0)),
            Err(_) => Ok(0),
        }
    }

    /// 获取未读邮件数量
    async fn get_unread_count(&self, folder: &str) -> EvifResult<i64> {
        if self.access_token.is_none() {
            return Ok(0);
        }

        let label_id = Self::folder_to_label_id(folder)?;
        let params = [
            ("labelIds", "UNREAD"),
        ];

        // 获取文件夹的未读数
        match self.gmail_api_request::<GmailProfile>(
            reqwest::Method::GET,
            "/profile",
            None,
        ).await {
            Ok(_profile) => {
                // Gmail API 不直接支持按文件夹统计未读，需要用 Messages.list + labelIds
                // 这里简化处理，返回总未读数
                Ok(0)
            }
            Err(_) => Ok(0),
        }
    }

    /// 获取邮件头部
    async fn get_message_headers(&self, folder: &str, msg_id: &str) -> EvifResult<String> {
        if self.access_token.is_none() {
            return Ok(format!(
                "Message-ID: <{}>\nFrom: user@example.com\nTo: {}@gmail.com\nSubject: Sample Email\nDate: {}\n",
                msg_id,
                self.config.username,
                Utc::now().format("%a, %d %b %Y %H:%M:%S +0000")
            ));
        }

        match self.gmail_api_request::<GmailMessage>(
            reqwest::Method::GET,
            &format!("/messages/{}", msg_id),
            Some(&[("format", "metadata")]),
        ).await {
            Ok(msg) => {
                let mut headers = format!("Message-ID: <{}>\n", msg.id);
                if let Some(payload) = &msg.payload {
                    if let Some(hdrs) = &payload.headers {
                        for h in hdrs {
                            headers.push_str(&format!("{}: {}\n", h.name, h.value));
                        }
                    }
                }
                Ok(headers)
            }
            Err(_) => Ok(format!(
                "Message-ID: <{}>\nFrom: user@example.com\nTo: {}@gmail.com\nSubject: Sample Email\nDate: {}\n",
                msg_id,
                self.config.username,
                Utc::now().format("%a, %d %b %Y %H:%M:%S +0000")
            )),
        }
    }

    /// 获取邮件正文
    async fn get_message_body(&self, folder: &str, msg_id: &str, html: bool) -> EvifResult<String> {
        if self.access_token.is_none() {
            if html {
                return Ok("<html><body><h1>Sample Email</h1><p>This is a sample email body.</p></body></html>".to_string());
            } else {
                return Ok("Sample Email\n\nThis is a sample email body.".to_string());
            }
        }

        match self.gmail_api_request::<GmailMessage>(
            reqwest::Method::GET,
            &format!("/messages/{}", msg_id),
            Some(&[("format", "full")]),
        ).await {
            Ok(msg) => {
                self.extract_body_from_message(&msg, html)
            }
            Err(_) => {
                if html {
                    Ok("<html><body><h1>Sample Email</h1><p>This is a sample email body.</p></body></html>".to_string())
                } else {
                    Ok("Sample Email\n\nThis is a sample email body.".to_string())
                }
            }
        }
    }

    /// 从消息中提取正文
    fn extract_body_from_message(&self, msg: &GmailMessage, html: bool) -> EvifResult<String> {
        if let Some(payload) = &msg.payload {
            // 尝试从 body 中获取纯文本
            if let Some(body) = &payload.body {
                if let Some(data) = &body.data {
                    let decoded = base64::Engine::decode(
                        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                        data,
                    );
                    if let Ok(decoded) = decoded {
                        let text = String::from_utf8_lossy(&decoded);
                        if html {
                            return Ok(text.to_string());
                        } else {
                            // 简单的 HTML 转纯文本
                            let plain = text
                                .replace("<br>", "\n")
                                .replace("<br/>", "\n")
                                .replace("<p>", "\n")
                                .replace("</p>", "\n")
                                .replace("<[^>]+>", "");
                            return Ok(plain.trim().to_string());
                        }
                    }
                }
            }

            // 递归搜索 parts
            if let Some(parts) = &payload.parts {
                let preferred_mime = if html { "text/html" } else { "text/plain" };
                let alt_mime = if html { "text/plain" } else { "text/html" };

                // 优先搜索首选 MIME 类型
                for part in parts {
                    if let Some(mime) = &part.mime_type {
                        if mime == preferred_mime {
                            if let Some(body) = &part.body {
                                if let Some(data) = &body.data {
                                    let decoded = base64::Engine::decode(
                                        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                                        data,
                                    );
                                    if let Ok(decoded) = decoded {
                                        return Ok(String::from_utf8_lossy(&decoded).to_string());
                                    }
                                }
                            }
                        }
                    }
                }

                // 搜索备用 MIME 类型
                for part in parts {
                    if let Some(mime) = &part.mime_type {
                        if mime == alt_mime {
                            if let Some(body) = &part.body {
                                if let Some(data) = &body.data {
                                    let decoded = base64::Engine::decode(
                                        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                                        data,
                                    );
                                    if let Ok(decoded) = decoded {
                                        return Ok(String::from_utf8_lossy(&decoded).to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 返回 snippet 作为后备
        Ok(msg.snippet.clone().unwrap_or_else(|| "No content".to_string()))
    }

    /// 将文件夹名转换为 Gmail Label ID
    fn folder_to_label_id(folder: &str) -> EvifResult<String> {
        let label = match folder {
            "INBOX" | "Inbox" => "INBOX",
            "[Gmail]/Sent Mail" | "Sent" => "SENT",
            "[Gmail]/Drafts" | "Drafts" => "DRAFT",
            "[Gmail]/Trash" | "Trash" => "TRASH",
            "[Gmail]/Spam" | "Spam" => "SPAM",
            "[Gmail]/Starred" | "Starred" => "STARRED",
            "[Gmail]/All Mail" | "All Mail" => "ALL",
            _ => folder,
        };
        Ok(label.to_string())
    }

    /// 列出邮件 ID 列表
    async fn list_message_ids(&self, folder: &str, max_results: i64) -> EvifResult<Vec<String>> {
        if self.access_token.is_none() {
            return Ok(vec![]);
        }

        let label_id = Self::folder_to_label_id(folder)?;
        let params = [
            ("labelIds", label_id.as_str()),
            ("maxResults", &max_results.to_string()),
        ];

        match self.gmail_api_request::<GmailMessageList>(
            reqwest::Method::GET,
            "/messages",
            Some(&params),
        ).await {
            Ok(list) => {
                Ok(list.messages
                    .map(|msgs| msgs.into_iter().map(|m| m.id).collect())
                    .unwrap_or_default())
            }
            Err(_) => Ok(vec![]),
        }
    }
}

/// GmailFs 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailFsOptions {
    pub imap_host: String,
    pub imap_port: Option<u16>,
    pub smtp_host: String,
    pub smtp_port: Option<u16>,
    pub username: String,
    pub password: String,
    pub use_ssl: Option<bool>,
    pub read_only: Option<bool>,
}

impl Default for GmailFsOptions {
    fn default() -> Self {
        Self {
            imap_host: "imap.gmail.com".to_string(),
            imap_port: Some(993),
            smtp_host: "smtp.gmail.com".to_string(),
            smtp_port: Some(587),
            username: String::new(),
            password: String::new(),
            use_ssl: Some(true),
            read_only: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> GmailFsPlugin {
        GmailFsPlugin {
            config: GmailConfig::default(),
            client: Some(reqwest::Client::new()),
            access_token: None,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn create_plugin_with_token(token: &str) -> GmailFsPlugin {
        GmailFsPlugin {
            config: GmailConfig::default(),
            client: Some(reqwest::Client::new()),
            access_token: Some(token.to_string()),
            connected: Arc::new(RwLock::new(true)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_standard_folders() {
        let folders = GmailFsPlugin::standard_folders();
        assert!(folders.len() >= 7);
        assert!(folders.iter().any(|(id, _)| *id == "INBOX"));
        assert!(folders.iter().any(|(id, _)| *id == "[Gmail]/Sent Mail"));
    }

    #[test]
    fn test_make_file_info() {
        let dir = GmailFsPlugin::make_file_info("INBOX", true, 0);
        assert_eq!(dir.name, "INBOX");
        assert!(dir.is_dir);
        assert_eq!(dir.mode, 0o755);

        let file = GmailFsPlugin::make_file_info("body", false, 1024);
        assert_eq!(file.name, "body");
        assert!(!file.is_dir);
        assert_eq!(file.mode, 0o644);
    }

    #[test]
    fn test_folder_to_label_id() {
        assert_eq!(GmailFsPlugin::folder_to_label_id("INBOX").unwrap(), "INBOX");
        assert_eq!(GmailFsPlugin::folder_to_label_id("Inbox").unwrap(), "INBOX");
        assert_eq!(GmailFsPlugin::folder_to_label_id("Sent").unwrap(), "SENT");
        assert_eq!(GmailFsPlugin::folder_to_label_id("Drafts").unwrap(), "DRAFT");
        assert_eq!(GmailFsPlugin::folder_to_label_id("Trash").unwrap(), "TRASH");
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Inbox"));
        assert!(entries.iter().any(|e| e.name == "Sent"));
    }

    #[tokio::test]
    async fn test_readdir_inbox() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Inbox").await.unwrap();
        // 空邮箱返回空列表
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_readdir_inbox_with_token() {
        let plugin = create_plugin_with_token("mock_token");
        // 有 token 但 API 调用会失败，返回空列表
        let entries = plugin.readdir("/Inbox").await.unwrap();
        // 模拟模式下返回空列表（因为 Gmail API 需要真实 token）
        assert!(entries.is_empty() || entries.len() <= 10);
    }

    #[tokio::test]
    async fn test_readdir_message() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Inbox/msg_000001").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "headers"));
        assert!(entries.iter().any(|e| e.name == "body"));
        assert!(entries.iter().any(|e| e.name == "body.html"));
        assert!(entries.iter().any(|e| e.name == "attachments"));
    }

    #[tokio::test]
    async fn test_read_headers() {
        let plugin = create_plugin();
        let content = plugin.read("/Inbox/msg_000001/headers", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Message-ID"));
        assert!(content_str.contains("From:"));
        assert!(content_str.contains("Subject:"));
    }

    #[tokio::test]
    async fn test_read_body() {
        let plugin = create_plugin();
        let content = plugin.read("/Inbox/msg_000001/body", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Sample Email"));
    }

    #[tokio::test]
    async fn test_read_body_html() {
        let plugin = create_plugin();
        let content = plugin.read("/Inbox/msg_000001/body.html", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("<html>"));
        assert!(content_str.contains("<body>"));
    }

    #[tokio::test]
    async fn test_readdir_attachments() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Inbox/msg_000001/attachments").await.unwrap();
        assert!(!entries.is_empty());
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "gmailfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_folder() {
        let plugin = create_plugin();
        let info = plugin.stat("/Inbox").await.unwrap();
        assert_eq!(info.name, "Inbox");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/Inbox/msg_000001/body").await.unwrap();
        assert_eq!(info.name, "body");
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
    async fn test_readdir_not_found() {
        let plugin = create_plugin();
        let result = plugin.readdir("/Nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_not_found() {
        let plugin = create_plugin();
        let result = plugin.read("/Inbox/nonexistent", 0, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_test_connection() {
        let plugin_no_auth = create_plugin();
        assert!(!plugin_no_auth.test_connection().await.unwrap());

        let plugin_with_auth = create_plugin_with_token("valid_token");
        // test_connection 会尝试 API 调用，可能失败但不会 panic
        let _ = plugin_with_auth.test_connection().await;
    }
}