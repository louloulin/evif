// Notion FS - Notion API 文件系统插件
//
// 提供真实 Notion API 集成，通过 VFS 接口暴露
// 目录结构: /notion/<database>/{page}, /notion/search, /notion/blocks/<id>
//
// 这是 Plan 9 风格的文件接口，用于 Notion 知识库访问

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};

const PLUGIN_NAME: &str = "notionfs";
const NOTION_API_BASE: &str = "https://api.notion.com/v1";

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

// ── Notion API 响应类型 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionSearchResult {
    pub results: Vec<NotionPage>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionPage {
    pub id: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub archived: Option<bool>,
    pub properties: Option<serde_json::Value>,
    pub url: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "object")]
    pub object_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionDatabase {
    pub id: String,
    pub title: Vec<NotionRichText>,
    pub created_time: String,
    pub last_edited_time: String,
    pub properties: HashMap<String, NotionProperty>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionRichText {
    pub plain_text: Option<String>,
    #[serde(rename = "type")]
    pub text_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionProperty {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionBlock {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub created_time: Option<String>,
    pub last_edited_time: Option<String>,
    pub has_children: Option<bool>,
    pub paragraph: Option<NotionBlockContent>,
    pub heading_1: Option<NotionBlockContent>,
    pub heading_2: Option<NotionBlockContent>,
    pub heading_3: Option<NotionBlockContent>,
    pub bulleted_list_item: Option<NotionBlockContent>,
    pub numbered_list_item: Option<NotionBlockContent>,
    pub to_do: Option<NotionBlockContent>,
    pub code: Option<NotionBlockContent>,
    pub quote: Option<NotionBlockContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionBlockContent {
    pub rich_text: Vec<NotionRichText>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionBlockChildrenResult {
    pub results: Vec<NotionBlock>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionDatabaseQueryResult {
    pub results: Vec<NotionPage>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionErrorResponse {
    pub message: Option<String>,
    pub status: Option<i32>,
}

// ── NotionFs Plugin ─────────────────────────────────────────────────

/// NotionFs 插件 - 真实 Notion API 集成
pub struct NotionFsPlugin {
    config: NotionConfig,
    /// HTTP 客户端
    client: Client,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 缓存
    cache: Arc<RwLock<HashMap<String, (Vec<u8>, chrono::DateTime<Utc>)>>>,
}

impl NotionFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: NotionConfig) -> EvifResult<Self> {
        if config.api_token.is_empty() {
            return Err(EvifError::InvalidPath(
                "Notion api_token is required".to_string(),
            ));
        }

        let client = Client::builder()
            .user_agent("EVIF-NotionFS/1.0")
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
            format!("Bearer {}", self.config.api_token)
                .parse()
                .unwrap(),
        );
        headers.insert(
            "Notion-Version",
            self.config
                .api_version
                .as_deref()
                .unwrap_or("2022-06-28")
                .parse()
                .unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers
    }

    /// 测试连接 - 调用 Notion /users/me API
    pub async fn test_connection(&self) -> EvifResult<bool> {
        let url = format!("{}/users/me", NOTION_API_BASE);
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

    // ── Notion API 方法 ─────────────────────────────────────────────

    /// 搜索 Notion 内容
    async fn api_search(&self, query: &str) -> EvifResult<NotionSearchResult> {
        let url = format!("{}/search", NOTION_API_BASE);
        let body = serde_json::json!({
            "query": query,
            "page_size": 10
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            let err: NotionErrorResponse = resp.json().await.unwrap_or(NotionErrorResponse {
                message: Some("Unknown error".to_string()),
                status: None,
            });
            return Err(EvifError::NotFound(format!(
                "Notion search failed: {}",
                err.message.unwrap_or_default()
            )));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))
    }

    /// 获取数据库信息
    async fn api_get_database(&self, database_id: &str) -> EvifResult<NotionDatabase> {
        let url = format!("{}/databases/{}", NOTION_API_BASE, database_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!(
                "Database not found: {}",
                database_id
            )));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))
    }

    /// 查询数据库
    async fn api_query_database(
        &self,
        database_id: &str,
        filter: Option<&serde_json::Value>,
    ) -> EvifResult<NotionDatabaseQueryResult> {
        let url = format!("{}/databases/{}/query", NOTION_API_BASE, database_id);
        let mut body = serde_json::json!({ "page_size": 10 });
        if let Some(f) = filter {
            body["filter"] = f.clone();
        }

        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!(
                "Database query failed: {}",
                database_id
            )));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))
    }

    /// 获取页面信息
    async fn api_get_page(&self, page_id: &str) -> EvifResult<NotionPage> {
        let url = format!("{}/pages/{}", NOTION_API_BASE, page_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!(
                "Page not found: {}",
                page_id
            )));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))
    }

    /// 获取块的子块
    async fn api_get_block_children(&self, block_id: &str) -> EvifResult<NotionBlockChildrenResult> {
        let url = format!("{}/blocks/{}/children", NOTION_API_BASE, block_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!(
                "Block children not found: {}",
                block_id
            )));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))
    }

    /// 获取块信息
    async fn api_get_block(&self, block_id: &str) -> EvifResult<NotionBlock> {
        let url = format!("{}/blocks/{}", NOTION_API_BASE, block_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!(
                "Block not found: {}",
                block_id
            )));
        }

        resp.json()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Connection failed: {}", e)))
    }

    /// 提取块内容为纯文本
    fn extract_block_text(block: &NotionBlock) -> String {
        let content = match block.block_type.as_str() {
            "paragraph" => &block.paragraph,
            "heading_1" => &block.heading_1,
            "heading_2" => &block.heading_2,
            "heading_3" => &block.heading_3,
            "bulleted_list_item" => &block.bulleted_list_item,
            "numbered_list_item" => &block.numbered_list_item,
            "to_do" => &block.to_do,
            "code" => &block.code,
            "quote" => &block.quote,
            _ => return String::new(),
        };

        match content {
            Some(c) => c
                .rich_text
                .iter()
                .filter_map(|rt| rt.plain_text.clone())
                .collect::<Vec<_>>()
                .join(""),
            None => String::new(),
        }
    }

    /// 提取页面标题
    fn extract_page_title(page: &NotionPage) -> String {
        if let Some(title) = &page.title {
            return title.clone();
        }
        if let Some(props) = &page.properties {
            // 尝试从 properties 中提取 Name/title
            if let Some(name_prop) = props.get("Name") {
                if let Some(title_arr) = name_prop.get("title") {
                    if let Some(first) = title_arr.as_array().and_then(|a| a.first()) {
                        if let Some(text) = first.get("plain_text").and_then(|t| t.as_str()) {
                            return text.to_string();
                        }
                    }
                }
            }
            if let Some(title_prop) = props.get("title") {
                if let Some(title_arr) = title_prop.get("title") {
                    if let Some(first) = title_arr.as_array().and_then(|a| a.first()) {
                        if let Some(text) = first.get("plain_text").and_then(|t| t.as_str()) {
                            return text.to_string();
                        }
                    }
                }
            }
        }
        page.id.chars().take(8).collect()
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
        // Normalize: remove "notion" prefix if present
        let path = path.strip_prefix("notion/").unwrap_or(path);
        let path = if path == "notion" { "" } else { path };

        match path {
            "" => {
                Ok(vec![
                    Self::make_file_info("search", true, 0),
                    Self::make_file_info("databases", true, 0),
                    Self::make_file_info("pages", true, 0),
                    Self::make_file_info("blocks", true, 0),
                ])
            }
            "search" => {
                Ok(vec![Self::make_file_info("query.md", false, 0)])
            }
            "databases" => {
                Ok(vec![
                    Self::make_file_info("query.md", false, 0),
                    Self::make_file_info("schema.md", false, 0),
                ])
            }
            "pages" => {
                Ok(vec![
                    Self::make_file_info("query.md", false, 0),
                    Self::make_file_info("create.md", false, 0),
                ])
            }
            "blocks" => {
                Ok(vec![
                    Self::make_file_info("query.md", false, 0),
                    Self::make_file_info("append.md", false, 0),
                ])
            }
            p if p.starts_with("databases/") => {
                // List database query results or schema
                let parts: Vec<&str> = p.split('/').collect();
                if parts.len() == 2 {
                    // /notion/databases/{id} - show database contents
                    let db_id = parts[1].trim_end_matches(".json").trim_end_matches(".md");
                    match self.api_query_database(db_id, None).await {
                        Ok(result) => {
                            let mut entries: Vec<FileInfo> = vec![
                                Self::make_file_info("schema.json", false, 0),
                            ];
                            for page in &result.results {
                                let title = Self::extract_page_title(page);
                                entries.push(Self::make_file_info(
                                    &format!("{}.md", title.replace('/', "_")),
                                    false,
                                    100,
                                ));
                            }
                            Ok(entries)
                        }
                        Err(_) => {
                            // Fallback: show database info files
                            Ok(vec![
                                Self::make_file_info("schema.json", false, 200),
                                Self::make_file_info("title.md", false, 100),
                            ])
                        }
                    }
                } else {
                    Ok(vec![
                        Self::make_file_info("title.md", false, 100),
                        Self::make_file_info("properties.json", false, 200),
                        Self::make_file_info("children", true, 0),
                    ])
                }
            }
            p if p.starts_with("pages/") => {
                let parts: Vec<&str> = p.split('/').collect();
                if parts.len() >= 2 {
                    let page_id = parts[1].trim_end_matches(".md").trim_end_matches(".json");
                    // Show page contents
                    Ok(vec![
                        Self::make_file_info("content.md", false, 100),
                        Self::make_file_info("properties.json", false, 200),
                        Self::make_file_info("children", true, 0),
                    ])
                } else {
                    Ok(vec![])
                }
            }
            p if p.starts_with("blocks/") => {
                let parts: Vec<&str> = p.split('/').collect();
                if parts.len() >= 2 {
                    let block_id = parts[1].trim_end_matches(".md").trim_end_matches(".json");
                    Ok(vec![
                        Self::make_file_info("content.md", false, 100),
                        Self::make_file_info("children", true, 0),
                    ])
                } else {
                    Ok(vec![])
                }
            }
            _ => Err(EvifError::NotFound(format!("Path not found: /{}", path))),
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_start_matches('/');
        let path = path.strip_prefix("notion/").unwrap_or(path);
        let path = if path == "notion" { "" } else { path };

        match path {
            "search/query.md" => {
                Ok(b"# Notion Search\n\nQuery: [search term]\n\nResults will appear here.".to_vec())
            }
            "databases/query.md" => {
                Ok(b"# Database Query\n\nDatabase ID: [database-id]\nFilter: [optional filter]\n".to_vec())
            }
            "databases/schema.md" => {
                Ok(b"# Database Schema\n\nProperties:\n- Name: title\n- Status: select\n- Tags: multi_select\n"
                    .to_vec())
            }
            "pages/query.md" => Ok(b"# Page Query\n\nPage ID: [page-id]\n".to_vec()),
            "pages/create.md" => {
                Ok(b"# Create Page\n\nParent: [database-id or page-id]\nProperties:\n- Name: [value]\n".to_vec())
            }
            "blocks/query.md" => Ok(b"# Block Query\n\nBlock ID: [block-id]\n".to_vec()),
            "blocks/append.md" => {
                Ok(b"# Append Block\n\nBlock ID: [block-id]\nChildren: [json array of blocks]\n".to_vec())
            }
            p if p.starts_with("search/") => {
                // Search query: /notion/search/{query}
                let query = p.strip_prefix("search/").unwrap_or("");
                let query = query.trim_end_matches(".md");
                match self.api_search(query).await {
                    Ok(result) => {
                        let mut output = format!("# Search Results: {}\n\n", query);
                        output.push_str(&format!("Found {} results\n\n", result.results.len()));
                        for page in &result.results {
                            let title = Self::extract_page_title(page);
                            output.push_str(&format!("- {} ({})\n", title, page.id));
                        }
                        Ok(output.into_bytes())
                    }
                    Err(e) => {
                        let content = format!("# Search Error\n\nQuery: {}\nError: {}", query, e);
                        Ok(content.into_bytes())
                    }
                }
            }
            p if p.starts_with("databases/") => {
                let rest = p.strip_prefix("databases/").unwrap_or("");
                if rest.ends_with("/schema.json") {
                    // Database schema
                    let db_id = rest.strip_suffix("/schema.json").unwrap_or(rest);
                    match self.api_get_database(db_id).await {
                        Ok(db) => {
                            let schema = serde_json::to_string_pretty(&db.properties)
                                .unwrap_or_else(|_| "Error serializing schema".to_string());
                            Ok(schema.into_bytes())
                        }
                        Err(e) => {
                            let content = format!("{{\"error\": \"{}\"}}", e);
                            Ok(content.into_bytes())
                        }
                    }
                } else if rest.ends_with("/title.md") {
                    let db_id = rest.strip_suffix("/title.md").unwrap_or(rest);
                    match self.api_get_database(db_id).await {
                        Ok(db) => {
                            let title: String = db
                                .title
                                .iter()
                                .filter_map(|rt| rt.plain_text.clone())
                                .collect();
                            Ok(format!("# {}\n", title).into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                } else if rest.ends_with("/properties.json") {
                    let db_id = rest.strip_suffix("/properties.json").unwrap_or(rest);
                    match self.api_get_database(db_id).await {
                        Ok(db) => {
                            let props = serde_json::to_string_pretty(&db.properties)
                                .unwrap_or_else(|_| "{}".to_string());
                            Ok(props.into_bytes())
                        }
                        Err(e) => Ok(format!("{{\"error\": \"{}\"}}", e).into_bytes()),
                    }
                } else {
                    // Database content
                    let db_id = rest.trim_end_matches(".md").trim_end_matches(".json");
                    match self.api_get_database(db_id).await {
                        Ok(db) => {
                            let title: String = db
                                .title
                                .iter()
                                .filter_map(|rt| rt.plain_text.clone())
                                .collect();
                            let content =
                                serde_json::to_string_pretty(&db).unwrap_or_else(|_| "{}".to_string());
                            Ok(format!("# Database: {}\n\n```json\n{}\n```", title, content)
                                .into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                }
            }
            p if p.starts_with("pages/") => {
                let rest = p.strip_prefix("pages/").unwrap_or("");
                if rest.ends_with("/content.md") {
                    let page_id = rest.strip_suffix("/content.md").unwrap_or(rest);
                    // Get page block children for content
                    match self.api_get_block_children(page_id).await {
                        Ok(result) => {
                            let mut content = String::new();
                            for block in &result.results {
                                let text = Self::extract_block_text(block);
                                if !text.is_empty() {
                                    match block.block_type.as_str() {
                                        "heading_1" => content.push_str(&format!("# {}\n\n", text)),
                                        "heading_2" => content.push_str(&format!("## {}\n\n", text)),
                                        "heading_3" => content.push_str(&format!("### {}\n\n", text)),
                                        "code" => content.push_str(&format!("```\n{}\n```\n\n", text)),
                                        "quote" => content.push_str(&format!("> {}\n\n", text)),
                                        _ => content.push_str(&format!("{}\n\n", text)),
                                    }
                                }
                            }
                            if content.is_empty() {
                                content = format!("# Page Content\n\nPage ID: {}\n(No content)", page_id);
                            }
                            Ok(content.into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                } else if rest.ends_with("/properties.json") {
                    let page_id = rest.strip_suffix("/properties.json").unwrap_or(rest);
                    match self.api_get_page(page_id).await {
                        Ok(page) => {
                            let props = serde_json::to_string_pretty(
                                &page.properties.unwrap_or(serde_json::Value::Null),
                            )
                            .unwrap_or_else(|_| "{}".to_string());
                            Ok(props.into_bytes())
                        }
                        Err(e) => Ok(format!("{{\"error\": \"{}\"}}", e).into_bytes()),
                    }
                } else {
                    // Page info
                    let page_id = rest.trim_end_matches(".md").trim_end_matches(".json");
                    match self.api_get_page(page_id).await {
                        Ok(page) => {
                            let title = Self::extract_page_title(&page);
                            let content = serde_json::to_string_pretty(&page)
                                .unwrap_or_else(|_| "{}".to_string());
                            Ok(format!("# Page: {}\n\n```json\n{}\n```", title, content).into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                }
            }
            p if p.starts_with("blocks/") => {
                let rest = p.strip_prefix("blocks/").unwrap_or("");
                if rest.ends_with("/content.md") {
                    let block_id = rest.strip_suffix("/content.md").unwrap_or(rest);
                    match self.api_get_block(block_id).await {
                        Ok(block) => {
                            let text = Self::extract_block_text(&block);
                            Ok(format!(
                                "# Block: {}\n\nType: {}\nContent: {}",
                                block.id, block.block_type, text
                            )
                            .into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                } else if rest.ends_with("/children") {
                    let block_id = rest.strip_suffix("/children").unwrap_or(rest);
                    match self.api_get_block_children(block_id).await {
                        Ok(result) => {
                            let mut content = format!("# Block Children: {}\n\n", block_id);
                            for block in &result.results {
                                let text = Self::extract_block_text(block);
                                content.push_str(&format!(
                                    "- [{}] {} ({})\n",
                                    block.block_type,
                                    if text.is_empty() { "(empty)" } else { &text },
                                    block.id
                                ));
                            }
                            Ok(content.into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                } else {
                    let block_id = rest.trim_end_matches(".md").trim_end_matches(".json");
                    match self.api_get_block(block_id).await {
                        Ok(block) => {
                            let content = serde_json::to_string_pretty(&block)
                                .unwrap_or_else(|_| "{}".to_string());
                            Ok(content.into_bytes())
                        }
                        Err(e) => Ok(format!("Error: {}", e).into_bytes()),
                    }
                }
            }
            _ => Err(EvifError::NotFound(format!("File not found: /{}", path))),
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
        let path = path.strip_prefix("notion/").unwrap_or(path);
        let path = if path == "notion" { "" } else { path };

        let is_dir = match path {
            "" | "notion" => true,
            "search" | "databases" | "pages" | "blocks" => true,
            p if p.starts_with("databases/")
                || p.starts_with("pages/")
                || p.starts_with("blocks/") =>
            {
                !p.ends_with(".md") && !p.ends_with(".json")
            }
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
        let result = plugin
            .write("/test", b"data".to_vec(), 0, WriteFlags::NONE)
            .await;
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
    async fn test_read_schema_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin
            .read("/notion/databases/schema.md", 0, 0)
            .await
            .unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Schema"));
        assert!(text.contains("Properties"));
    }

    #[tokio::test]
    async fn test_read_create_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin
            .read("/notion/pages/create.md", 0, 0)
            .await
            .unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Create"));
        assert!(text.contains("Page"));
    }

    #[tokio::test]
    async fn test_read_append_file() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let content = plugin
            .read("/notion/blocks/append.md", 0, 0)
            .await
            .unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Append"));
        assert!(text.contains("Block"));
    }

    #[tokio::test]
    async fn test_auth_headers() {
        let plugin = NotionFsPlugin::new(make_test_config()).await.unwrap();
        let headers = plugin.auth_headers();
        assert!(headers.contains_key(reqwest::header::AUTHORIZATION));
        assert!(headers.contains_key("Notion-Version"));
    }

    #[tokio::test]
    async fn test_extract_block_text() {
        let block = NotionBlock {
            id: "test".to_string(),
            block_type: "paragraph".to_string(),
            created_time: None,
            last_edited_time: None,
            has_children: None,
            paragraph: Some(NotionBlockContent {
                rich_text: vec![NotionRichText {
                    plain_text: Some("Hello World".to_string()),
                    text_type: Some("text".to_string()),
                }],
            }),
            heading_1: None,
            heading_2: None,
            heading_3: None,
            bulleted_list_item: None,
            numbered_list_item: None,
            to_do: None,
            code: None,
            quote: None,
        };
        assert_eq!(NotionFsPlugin::extract_block_text(&block), "Hello World");
    }

    #[tokio::test]
    async fn test_extract_block_text_empty() {
        let block = NotionBlock {
            id: "test".to_string(),
            block_type: "unsupported".to_string(),
            created_time: None,
            last_edited_time: None,
            has_children: None,
            paragraph: None,
            heading_1: None,
            heading_2: None,
            heading_3: None,
            bulleted_list_item: None,
            numbered_list_item: None,
            to_do: None,
            code: None,
            quote: None,
        };
        assert_eq!(NotionFsPlugin::extract_block_text(&block), "");
    }

    #[tokio::test]
    async fn test_extract_page_title() {
        let page = NotionPage {
            id: "abc123".to_string(),
            created_time: String::new(),
            last_edited_time: String::new(),
            archived: None,
            properties: None,
            url: None,
            title: Some("My Page".to_string()),
            object_type: None,
        };
        assert_eq!(NotionFsPlugin::extract_page_title(&page), "My Page");
    }

    #[tokio::test]
    async fn test_extract_page_title_fallback() {
        let page = NotionPage {
            id: "abc12345678".to_string(),
            created_time: String::new(),
            last_edited_time: String::new(),
            archived: None,
            properties: None,
            url: None,
            title: None,
            object_type: None,
        };
        assert_eq!(NotionFsPlugin::extract_page_title(&page), "abc12345");
    }
}
