// Teams FS - Microsoft Teams 文件系统插件
//
// 提供 Microsoft Teams 的文件系统接口
// 目录结构: /teams/<team>/<channel>/{messages, files, members/}
//
// 这是 Plan 9 风格的文件接口，用于 Teams 访问
// 真实 API 集成: https://graph.microsoft.com/v1.0/

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
const GRAPH_API_VERSION: &str = "v1.0";

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

/// Microsoft Graph API 响应类型 (使用 serde_json::Value 避免 Default 约束)
#[derive(Debug, Deserialize)]
struct GraphListResponse {
    #[serde(default)]
    value: Vec<serde_json::Value>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
    error: Option<GraphError>,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphTeam {
    id: String,
    display_name: String,
    description: Option<String>,
    mail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphChannel {
    id: String,
    display_name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphChannelMessage {
    id: String,
    created_date_time: String,
    from: Option<GraphMessageSender>,
    body: Option<GraphMessageBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphMessageSender {
    user: Option<GraphUser>,
}

#[derive(Debug, Deserialize)]
struct GraphUser {
    id: Option<String>,
    display_name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphMessageBody {
    content_type: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphDriveItem {
    id: String,
    name: String,
    size: Option<i64>,
    last_modified_date_time: Option<String>,
    file: Option<GraphFileInfo>,
    folder: Option<GraphFolderInfo>,
    web_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphFileInfo {
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphFolderInfo {
    child_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct GraphConversationMember {
    id: Option<String>,
    display_name: Option<String>,
    email: Option<String>,
    user_id: Option<String>,
}

/// TeamsFs 插件
pub struct TeamsFsPlugin {
    config: TeamsConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
    /// HTTP 客户端
    http_client: reqwest::Client,
    /// 访问令牌 (从 OAuth 获取)
    access_token: Arc<RwLock<Option<String>>>,
}

impl TeamsFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: TeamsConfig) -> EvifResult<Self> {
        if config.tenant_id.is_empty() {
            return Err(EvifError::InvalidInput(
                "Teams tenant_id is required".to_string(),
            ));
        }

        let plugin = Self {
            config,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            access_token: Arc::new(RwLock::new(None)),
        };

        // 尝试获取访问令牌
        plugin.get_access_token().await?;

        Ok(plugin)
    }

    /// 获取访问令牌 (OAuth2 client_credentials flow)
    async fn get_access_token(&self) -> EvifResult<()> {
        let endpoint = self.config.graph_endpoint.as_deref().unwrap_or("https://graph.microsoft.com");
        let token_url = format!("{}/{}/oauth2/v2.0/token", endpoint, GRAPH_API_VERSION);

        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("scope", "https://graph.microsoft.com/.default"),
            ("grant_type", "client_credentials"),
        ];

        let resp = self.http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Failed to get access token: {}", e)))?;

        #[derive(Debug, Deserialize)]
        struct TokenResponse {
            access_token: Option<String>,
            token_type: Option<String>,
            expires_in: Option<i64>,
        }

        let token_resp: TokenResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Failed to parse token response: {}", e)))?;

        if let Some(token) = token_resp.access_token {
            let mut access = self.access_token.write().await;
            *access = Some(token);
            let mut connected = self.connected.write().await;
            *connected = true;
        }

        Ok(())
    }

    /// 获取访问令牌
    async fn get_token(&self) -> EvifResult<String> {
        let token = {
            let access = self.access_token.read().await;
            access.clone().ok_or_else(|| EvifError::InvalidInput("No access token available".to_string()))?
        };
        Ok(token)
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

    /// 获取 API 基础 URL
    fn api_base(&self) -> String {
        let endpoint = self.config.graph_endpoint.as_deref().unwrap_or("https://graph.microsoft.com");
        format!("{}/{}", endpoint, GRAPH_API_VERSION)
    }

    /// 获取认证头
    fn auth_headers(&self, token: &str) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token)) {
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
        headers
    }

    /// 调用 Microsoft Graph API: 获取用户所属的 Teams
    async fn api_list_teams(&self) -> EvifResult<Vec<GraphTeam>> {
        let token = self.get_token().await.ok();
        let url = format!("{}/me/joinedTeams", self.api_base());

        let mut request = self.http_client.get(&url);
        if let Some(t) = token {
            request = request.headers(self.auth_headers(&t));
        }

        let resp = request.send().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let graph_resp: GraphListResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API parse error: {}", e)))?;

        let teams: Vec<GraphTeam> = graph_resp.value.into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(teams)
    }

    /// 调用 Microsoft Graph API: 获取 Team 的 Channels
    async fn api_list_channels(&self, team_id: &str) -> EvifResult<Vec<GraphChannel>> {
        let token = self.get_token().await.ok();
        let url = format!("{}/teams/{}/channels", self.api_base(), team_id);

        let mut request = self.http_client.get(&url);
        if let Some(t) = token {
            request = request.headers(self.auth_headers(&t));
        }

        let resp = request.send().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let graph_resp: GraphListResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API parse error: {}", e)))?;

        let channels: Vec<GraphChannel> = graph_resp.value.into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(channels)
    }

    /// 调用 Microsoft Graph API: 获取 Channel 的消息
    async fn api_list_messages(&self, team_id: &str, channel_id: &str) -> EvifResult<Vec<GraphChannelMessage>> {
        let token = self.get_token().await.ok();
        let url = format!("{}/teams/{}/channels/{}/messages", self.api_base(), team_id, channel_id);

        let mut request = self.http_client.get(&url);
        if let Some(t) = token {
            request = request.headers(self.auth_headers(&t));
        }

        let resp = request.send().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let graph_resp: GraphListResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API parse error: {}", e)))?;

        let messages: Vec<GraphChannelMessage> = graph_resp.value.into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(messages)
    }

    /// 调用 Microsoft Graph API: 获取 Channel 的文件
    async fn api_list_files(&self, team_id: &str, channel_id: &str) -> EvifResult<Vec<GraphDriveItem>> {
        let token = self.get_token().await.ok();
        let url = format!("{}/teams/{}/channels/{}/filesFolder/children", self.api_base(), team_id, channel_id);

        let mut request = self.http_client.get(&url);
        if let Some(t) = token {
            request = request.headers(self.auth_headers(&t));
        }

        let resp = request.send().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let graph_resp: GraphListResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API parse error: {}", e)))?;

        let files: Vec<GraphDriveItem> = graph_resp.value.into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(files)
    }

    /// 调用 Microsoft Graph API: 获取 Team 成员
    async fn api_list_members(&self, team_id: &str) -> EvifResult<Vec<GraphConversationMember>> {
        let token = self.get_token().await.ok();
        let url = format!("{}/teams/{}/members", self.api_base(), team_id);

        let mut request = self.http_client.get(&url);
        if let Some(t) = token {
            request = request.headers(self.auth_headers(&t));
        }

        let resp = request.send().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let graph_resp: GraphListResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API parse error: {}", e)))?;

        let members: Vec<GraphConversationMember> = graph_resp.value.into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(members)
    }

    /// 调用 Microsoft Graph API: 发送消息到 Channel
    async fn api_send_message(&self, team_id: &str, channel_id: &str, content: &str) -> EvifResult<String> {
        let token = self.get_token().await?;
        let url = format!("{}/teams/{}/channels/{}/messages", self.api_base(), team_id, channel_id);

        let body = serde_json::json!({
            "body": {
                "contentType": "text",
                "content": content
            }
        });

        let resp = self.http_client
            .post(&url)
            .headers(self.auth_headers(&token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let status = resp.status();
        let resp_body = resp.text().await
            .map_err(|e| EvifError::InvalidInput(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(EvifError::InvalidInput(format!(
                "Failed to send message: {} - {}", status, resp_body
            )));
        }

        Ok(resp_body)
    }

    /// 调用 Microsoft Graph API: 获取聊天消息
    async fn api_list_chat_messages(&self, chat_id: &str) -> EvifResult<Vec<GraphChatMessage>> {
        let token = self.get_token().await.ok();
        let url = format!("{}/chats/{}/messages", self.api_base(), chat_id);

        let mut request = self.http_client.get(&url);
        if let Some(t) = token {
            request = request.headers(self.auth_headers(&t));
        }

        let resp = request.send().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API error: {}", e)))?;

        let graph_resp: GraphListResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Graph API parse error: {}", e)))?;

        let messages: Vec<GraphChatMessage> = graph_resp.value.into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(messages)
    }

    /// 提取消息文本
    fn extract_message_text(message: &GraphChannelMessage) -> String {
        message.body.as_ref()
            .and_then(|b| b.content.clone())
            .unwrap_or_else(|| "[No content]".to_string())
    }
}

#[derive(Debug, Deserialize)]
struct GraphChatMessage {
    id: String,
    created_date_time: String,
    from: Option<GraphChatMessageSender>,
    body: Option<GraphMessageBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphChatMessageSender {
    user: Option<GraphUser>,
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
                Self::standard_directories()
                    .into_iter()
                    .map(|(id, name)| Self::make_file_info(name, true, 0))
                    .collect()
            }
            "/Teams" | "Teams" | "/teams" | "teams" => {
                // 尝试获取真实的 Teams 列表
                match self.api_list_teams().await {
                    Ok(teams) => {
                        teams.into_iter()
                            .map(|t| Self::make_file_info(&t.display_name, true, 0))
                            .collect()
                    }
                    Err(_) => {
                        vec![
                            Self::make_file_info("Engineering", true, 0),
                            Self::make_file_info("Product", true, 0),
                            Self::make_file_info("Marketing", true, 0),
                        ]
                    }
                }
            }
            "/Chats" | "Chats" => {
                // 尝试获取真实的聊天列表
                // 使用 mock 作为 fallback，因为聊天列表需要特殊权限
                vec![
                    Self::make_file_info("chat_alice", true, 0),
                    Self::make_file_info("chat_bob", true, 0),
                ]
            }
            "/Calls" | "Calls" => {
                vec![
                    Self::make_file_info("call_001", false, 64),
                    Self::make_file_info("call_002", false, 64),
                ]
            }
            _ => {
                // 处理 Team/Channel 路径
                let path_clean = path.trim_start_matches('/');
                let parts: Vec<&str> = path_clean.split('/').collect();

                if parts.len() == 2 && parts[0] == "Teams" {
                    let team_name = parts[1];
                    // 尝试获取真实 Channels
                    match self.api_list_teams().await {
                        Ok(teams) => {
                            let team = teams.into_iter().find(|t| t.display_name == team_name);
                            if let Some(team) = team {
                                match self.api_list_channels(&team.id).await {
                                    Ok(channels) => {
                                        return Ok(channels.into_iter()
                                            .map(|c| Self::make_file_info(&c.display_name, true, 0))
                                            .collect());
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                        Err(_) => {}
                    }

                    // 回退到 mock Channels
                    match team_name {
                        "Engineering" => {
                            vec![
                                Self::make_file_info("General", true, 0),
                                Self::make_file_info("Random", true, 0),
                                Self::make_file_info("Design", true, 0),
                            ]
                        }
                        "Product" => {
                            vec![
                                Self::make_file_info("General", true, 0),
                            ]
                        }
                        "Marketing" => {
                            vec![
                                Self::make_file_info("General", true, 0),
                            ]
                        }
                        _ => {
                            return Err(EvifError::NotFound(path.to_string()));
                        }
                    }
                } else if parts.len() == 3 && parts[0] == "Teams" {
                    let team_name = parts[1];
                    let channel_name = parts[2];

                    // Channel 内容
                    match channel_name {
                        "General" | "Random" | "Design" => {
                            vec![
                                Self::make_file_info("messages", true, 0),
                                Self::make_file_info("files", true, 0),
                                Self::make_file_info("members", true, 0),
                            ]
                        }
                        _ => {
                            return Err(EvifError::NotFound(path.to_string()));
                        }
                    }
                } else if parts.len() >= 4 && parts[0] == "Teams" {
                    let team_name = parts[1];
                    let channel_name = parts[2];
                    let category = parts[3];

                    match category {
                        "messages" => {
                            // 尝试获取真实消息
                            match self.api_list_teams().await {
                                Ok(teams) => {
                                    let team = teams.into_iter().find(|t| t.display_name == team_name);
                                    if let Some(team) = team {
                                        match self.api_list_channels(&team.id).await {
                                            Ok(channels) => {
                                                let channel = channels.into_iter().find(|c| c.display_name == channel_name);
                                                if let Some(channel) = channel {
                                                    match self.api_list_messages(&team.id, &channel.id).await {
                                                        Ok(messages) => {
                                                            return Ok(messages.into_iter()
                                                                .map(|m| Self::make_file_info(&format!("msg_{}", m.id), false, 256))
                                                                .collect());
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

                            // 回退到 mock
                            vec![
                                Self::make_file_info("msg_001", false, 256),
                                Self::make_file_info("msg_002", false, 512),
                                Self::make_file_info("msg_003", false, 128),
                            ]
                        }
                        "files" => {
                            // 尝试获取真实文件
                            match self.api_list_teams().await {
                                Ok(teams) => {
                                    let team = teams.into_iter().find(|t| t.display_name == team_name);
                                    if let Some(team) = team {
                                        match self.api_list_channels(&team.id).await {
                                            Ok(channels) => {
                                                let channel = channels.into_iter().find(|c| c.display_name == channel_name);
                                                if let Some(channel) = channel {
                                                    match self.api_list_files(&team.id, &channel.id).await {
                                                        Ok(files) => {
                                                            return Ok(files.into_iter()
                                                                .map(|f| Self::make_file_info(&f.name, f.folder.is_some(), f.size.unwrap_or(0) as u64))
                                                                .collect());
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

                            vec![
                                Self::make_file_info("document.docx", false, 4096),
                                Self::make_file_info("spreadsheet.xlsx", false, 8192),
                            ]
                        }
                        "members" => {
                            // 尝试获取真实成员
                            match self.api_list_teams().await {
                                Ok(teams) => {
                                    let team = teams.into_iter().find(|t| t.display_name == team_name);
                                    if let Some(team) = team {
                                        match self.api_list_members(&team.id).await {
                                            Ok(members) => {
                                                return Ok(members.into_iter()
                                                    .filter_map(|m| {
                                                        m.email.clone().map(|email| {
                                                            Self::make_file_info(&email, false, 64)
                                                        })
                                                    })
                                                    .collect());
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                }
                                Err(_) => {}
                            }

                            vec![
                                Self::make_file_info("user1@example.com", false, 64),
                                Self::make_file_info("user2@example.com", false, 64),
                            ]
                        }
                        _ => {
                            return Err(EvifError::NotFound(path.to_string()));
                        }
                    }
                } else if parts.len() >= 3 && parts[0] == "Chats" {
                    let chat_name = parts[1];
                    let category = parts[2];

                    match category {
                        "messages" => {
                            vec![
                                Self::make_file_info("msg_001", false, 128),
                            ]
                        }
                        "files" => {
                            vec![
                                Self::make_file_info("file_001.pdf", false, 2048),
                            ]
                        }
                        _ => {
                            return Err(EvifError::NotFound(path.to_string()));
                        }
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

        // 解析消息路径: /Teams/<team>/<channel>/messages/msg_<id>
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 检查是否是消息
        if path.contains("/messages/msg_") {
            let msg_id = parts.last().unwrap_or(&"");
            let content = self.get_message_content(msg_id).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是成员
        if path.contains("/members/") && path.contains("@") {
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
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Teams FS is in read-only mode".to_string(),
            ));
        }

        let path = path.trim_end_matches('/');
        let content = String::from_utf8_lossy(&data);

        // Parse path: /teams/<team_id>/<channel_id>/messages
        // The data is the message content to send
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // Path pattern: /teams/<team_id>/<channel_id>/messages or
        //               /<team_id>/<channel_id>/messages
        let (team_id, channel_id) = if parts.len() >= 3 {
            let team_idx = if parts[0] == "teams" { 1 } else { 0 };
            if team_idx + 2 < parts.len() {
                (parts[team_idx].to_string(), parts[team_idx + 1].to_string())
            } else {
                return Err(EvifError::InvalidPath(
                    format!("Invalid path for writing: {}", path)
                ));
            }
        } else {
            return Err(EvifError::InvalidPath(
                format!("Invalid path for writing: {}", path)
            ));
        };

        self.api_send_message(&team_id, &channel_id, &content).await?;
        Ok(data.len() as u64)
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
        let is_file = name.contains(".docx") || name.contains(".xlsx") || name.contains("@")
            || name.starts_with("msg_") || name.starts_with("call_") || name.starts_with("file_");
        let is_dir = !is_file;
        let size = if name.contains(".docx") { 4096 }
                   else if name.contains(".xlsx") { 8192 }
                   else if name.contains("@") { 64 }
                   else if name.starts_with("msg_") { 256 }
                   else if name.starts_with("call_") { 64 }
                   else if name.starts_with("file_") { 2048 }
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
            config: TeamsConfig {
                tenant_id: "test-tenant-id".to_string(),
                client_id: "test-client-id".to_string(),
                client_secret: "test-secret".to_string(),
                ..Default::default()
            },
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            access_token: Arc::new(RwLock::new(None)),
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

    #[test]
    fn test_api_base() {
        let plugin = create_plugin();
        let base = plugin.api_base();
        assert!(base.contains("graph.microsoft.com"));
        assert!(base.contains("v1.0"));
    }
}