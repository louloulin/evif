// VectorFS - 向量搜索文件系统插件
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

#[cfg(feature = "rusqlite")]
use rusqlite::{Connection, params};

/// Embedding provider trait for generating text embeddings
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String>;

    /// Generate embeddings for multiple texts
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Name of the provider
    fn name(&self) -> &str;
}

/// OpenAI embedding provider using reqwest
pub struct OpenAIEmbeddingProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIEmbeddingProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            input: String,
        }
        #[derive(Deserialize)]
        struct Response {
            data: Vec<EmbeddingData>,
        }
        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let resp = self.client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&Request {
                model: self.model.clone(),
                input: text.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("Embedding request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Embedding API error {}: {}", status, body));
        }

        let result: Response = resp.json().await
            .map_err(|e| format!("Failed to parse embedding response: {}", e))?;

        result.data.into_iter().next()
            .map(|d| d.embedding)
            .ok_or_else(|| "No embedding in response".to_string())
    }

    fn name(&self) -> &str {
        "openai"
    }
}

/// No-op embedding provider for when no embedding is configured
pub struct NoEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for NoEmbeddingProvider {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
        Err("No embedding provider configured".to_string())
    }

    fn name(&self) -> &str {
        "none"
    }
}

/// 向量搜索配置
#[derive(Debug, Clone)]
pub struct VectorFsConfig {
    /// S3配置
    pub s3_bucket: String,
    pub s3_key_prefix: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,

    /// OpenAI embedding配置
    pub embedding_api_key: Option<String>,
    pub embedding_model: String,
    pub embedding_base_url: Option<String>,

    /// 向量维度 (OpenAI text-embedding-3-small = 1536)
    pub embedding_dim: usize,

    /// 文档分块大小
    pub chunk_size: usize,
    pub chunk_overlap: usize,

    /// 索引worker数量
    pub index_workers: usize,

    /// 持久化存储路径（SQLite）
    /// 设为 None 则仅使用内存存储（重启后丢失）
    /// 设为 Some(path) 则将向量数据持久化到 SQLite 文件
    pub persistence_path: Option<String>,
}

impl Default for VectorFsConfig {
    fn default() -> Self {
        Self {
            s3_bucket: "vectorfs-docs".to_string(),
            s3_key_prefix: Some("vectorfs".to_string()),
            s3_region: Some("us-east-1".to_string()),
            s3_endpoint: None,
            embedding_api_key: None,
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_base_url: None,
            embedding_dim: 1536,
            chunk_size: 512,
            chunk_overlap: 50,
            index_workers: 4,
            persistence_path: None, // 默认仅内存存储
        }
    }
}

/// 索引任务
#[derive(Debug, Clone)]
struct IndexTask {
    namespace: String,
    document_id: String,
    file_name: String,
    data: String,
}

/// 索引文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexingFileInfo {
    file_name: String,
    start_time: DateTime<Utc>,
}

/// 向量文档
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorDocument {
    id: String,
    namespace: String,
    file_name: String,
    chunk_index: usize,
    content: String,
    #[serde(skip)]
    embedding: Option<Vec<f32>>,
    created_at: DateTime<Utc>,
    s3_key: String,
}

/// 命名空间
#[derive(Debug, Clone)]
struct Namespace {
    name: String,
    documents: HashMap<String, VectorDocument>,
    created_at: DateTime<Utc>,
}

/// VectorFS 插件
pub struct VectorFsPlugin {
    config: VectorFsConfig,
    namespaces: Arc<RwLock<HashMap<String, Namespace>>>,
    index_queue: Arc<Mutex<Vec<IndexTask>>>,
    indexing_status: Arc<RwLock<HashMap<String, HashMap<String, IndexingFileInfo>>>>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    #[cfg(feature = "rusqlite")]
    db_path: Option<String>,
}

/// SQLite 持久化辅助方法
#[cfg(feature = "rusqlite")]
impl VectorFsPlugin {
    /// 初始化 SQLite 数据库
    fn init_db(db_path: &str) -> EvifResult<()> {
        let conn = Connection::open(db_path)
            .map_err(|e| EvifError::Storage(format!("Failed to open persistence DB: {}", e)))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS vectorfs_namespaces (
                 name TEXT PRIMARY KEY,
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS vectorfs_documents (
                 id TEXT PRIMARY KEY,
                 namespace TEXT NOT NULL,
                 file_name TEXT NOT NULL,
                 chunk_index INTEGER NOT NULL,
                 content TEXT NOT NULL,
                 embedding BLOB,
                 created_at INTEGER NOT NULL,
                 s3_key TEXT NOT NULL,
                 FOREIGN KEY (namespace) REFERENCES vectorfs_namespaces(name) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_docs_namespace ON vectorfs_documents(namespace);
             CREATE INDEX IF NOT EXISTS idx_docs_filename ON vectorfs_documents(namespace, file_name);"
        ).map_err(|e| EvifError::Storage(format!("Failed to initialize persistence DB: {}", e)))?;

        Ok(())
    }

    /// 从 SQLite 加载所有数据到内存
    fn load_from_db(db_path: &str) -> EvifResult<HashMap<String, Namespace>> {
        let conn = Connection::open(db_path)
            .map_err(|e| EvifError::Storage(format!("Failed to open persistence DB for loading: {}", e)))?;

        let mut namespaces = HashMap::new();

        // 加载命名空间
        let mut ns_stmt = conn.prepare(
            "SELECT name, created_at FROM vectorfs_namespaces"
        ).map_err(|e| EvifError::Storage(format!("Failed to prepare namespace query: {}", e)))?;

        let ns_rows = ns_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
            ))
        }).map_err(|e| EvifError::Storage(format!("Failed to query namespaces: {}", e)))?;

        let mut ns_list = Vec::new();
        for row in ns_rows {
            let (name, created_at_ts) = row
                .map_err(|e| EvifError::Storage(format!("Failed to read namespace row: {}", e)))?;
            ns_list.push((name, created_at_ts));
        }

        // 加载每个命名空间的文档
        for (name, created_at_ts) in ns_list {
            let created_at = DateTime::from_timestamp(created_at_ts, 0).unwrap_or_default();

            let mut doc_stmt = conn.prepare(
                "SELECT id, namespace, file_name, chunk_index, content, embedding, created_at, s3_key
                 FROM vectorfs_documents WHERE namespace = ?1"
            ).map_err(|e| EvifError::Storage(format!("Failed to prepare document query: {}", e)))?;

            let namespace_name = name.clone();
            let doc_rows = doc_stmt.query_map([&name], |row| {
                let embedding_blob: Option<Vec<u8>> = row.get(5)?;
                let embedding = embedding_blob.map(|blob| {
                    // Deserialize Vec<f32> from little-endian bytes
                    let len = blob.len() / 4;
                    let mut vec = Vec::with_capacity(len);
                    for chunk in blob.chunks_exact(4) {
                        let bytes: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                        vec.push(f32::from_le_bytes(bytes));
                    }
                    vec
                });

                Ok(VectorDocument {
                    id: row.get(0)?,
                    namespace: row.get(1)?,
                    file_name: row.get(2)?,
                    chunk_index: row.get(3)?,
                    content: row.get(4)?,
                    embedding,
                    created_at: DateTime::from_timestamp(row.get::<_, i64>(6)?, 0).unwrap_or_default(),
                    s3_key: row.get(7)?,
                })
            }).map_err(|e| EvifError::Storage(format!("Failed to query documents for namespace {}: {}", namespace_name, e)))?;

            let mut documents = HashMap::new();
            for doc_result in doc_rows {
                let doc = doc_result
                    .map_err(|e| EvifError::Storage(format!("Failed to read document: {}", e)))?;
                documents.insert(doc.id.clone(), doc);
            }

            log::info!("Loaded namespace '{}' with {} documents from SQLite", name, documents.len());

            namespaces.insert(name.clone(), Namespace {
                name,
                documents,
                created_at,
            });
        }

        Ok(namespaces)
    }

    /// 持久化命名空间创建
    fn persist_create_namespace(&self, name: &str, created_at: DateTime<Utc>) -> EvifResult<()> {
        if let Some(db_path) = &self.db_path {
            let db_path = db_path.clone();
            let name = name.to_string();
            tokio::task::block_in_place(|| {
                let conn = Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Failed to open DB: {}", e)))?;
                conn.execute(
                    "INSERT OR IGNORE INTO vectorfs_namespaces (name, created_at) VALUES (?1, ?2)",
                    params![name, created_at.timestamp()],
                ).map_err(|e| EvifError::Storage(format!("Failed to persist namespace: {}", e)))?;
                Ok(())
            })
        } else {
            Ok(())
        }
    }

    /// 持久化命名空间删除
    fn persist_delete_namespace(&self, name: &str) -> EvifResult<()> {
        if let Some(db_path) = &self.db_path {
            let db_path = db_path.clone();
            let name = name.to_string();
            tokio::task::block_in_place(|| {
                let conn = Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Failed to open DB: {}", e)))?;
                // CASCADE will delete documents automatically
                conn.execute(
                    "DELETE FROM vectorfs_namespaces WHERE name = ?1",
                    params![name],
                ).map_err(|e| EvifError::Storage(format!("Failed to persist namespace deletion: {}", e)))?;
                Ok(())
            })
        } else {
            Ok(())
        }
    }

    /// 持久化文档写入
    fn persist_write_document(&self, doc: &VectorDocument) -> EvifResult<()> {
        if let Some(db_path) = &self.db_path {
            let db_path = db_path.clone();
            let id = doc.id.clone();
            let namespace = doc.namespace.clone();
            let file_name = doc.file_name.clone();
            let chunk_index = doc.chunk_index;
            let content = doc.content.clone();
            let embedding_blob = doc.embedding.as_ref().map(|emb| {
                // Serialize Vec<f32> to little-endian bytes
                let mut blob = Vec::with_capacity(emb.len() * 4);
                for val in emb {
                    blob.extend_from_slice(&val.to_le_bytes());
                }
                blob
            });
            let created_at = doc.created_at.timestamp();
            let s3_key = doc.s3_key.clone();

            tokio::task::block_in_place(|| {
                let conn = Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Failed to open DB: {}", e)))?;
                conn.execute(
                    "INSERT OR REPLACE INTO vectorfs_documents
                     (id, namespace, file_name, chunk_index, content, embedding, created_at, s3_key)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, namespace, file_name, chunk_index, content, embedding_blob, created_at, s3_key],
                ).map_err(|e| EvifError::Storage(format!("Failed to persist document: {}", e)))?;
                Ok(())
            })
        } else {
            Ok(())
        }
    }
}

impl VectorFsPlugin {
    pub fn new(config: VectorFsConfig) -> Self {
        let embedding_provider: Arc<dyn EmbeddingProvider> = match &config.embedding_api_key {
            Some(api_key) => {
                let mut provider = OpenAIEmbeddingProvider::new(
                    api_key.clone(),
                    config.embedding_model.clone(),
                );
                if let Some(base_url) = &config.embedding_base_url {
                    provider = provider.with_base_url(base_url.clone());
                }
                Arc::new(provider)
            }
            None => Arc::new(NoEmbeddingProvider),
        };

        #[cfg(feature = "rusqlite")]
        let (namespaces, db_path) = {
            if let Some(persistence_path) = &config.persistence_path {
                match Self::init_db(persistence_path) {
                    Ok(()) => {
                        log::info!("VectorFS persistence initialized at: {}", persistence_path);
                    }
                    Err(e) => {
                        log::error!("VectorFS persistence init failed: {}", e);
                    }
                }
                let namespaces = Self::load_from_db(persistence_path).unwrap_or_else(|e| {
                    log::warn!("VectorFS failed to load from SQLite (starting fresh): {}", e);
                    HashMap::new()
                });
                (namespaces, Some(persistence_path.clone()))
            } else {
                (HashMap::new(), None)
            }
        };

        #[cfg(not(feature = "rusqlite"))]
        let namespaces = HashMap::new();

        Self {
            config,
            namespaces: Arc::new(RwLock::new(namespaces)),
            index_queue: Arc::new(Mutex::new(Vec::new())),
            indexing_status: Arc::new(RwLock::new(HashMap::new())),
            embedding_provider,
            #[cfg(feature = "rusqlite")]
            db_path,
        }
    }

    /// 解析路径: /namespace/docs/file.txt -> (namespace, "docs/file.txt")
    fn parse_path(path: &str) -> EvifResult<(String, String)> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Ok(("".to_string(), "".to_string()));
        }

        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.is_empty() {
            return Err(EvifError::InvalidPath("Invalid path".to_string()));
        }

        let namespace = parts[0].to_string();
        let relative_path = if parts.len() == 2 { parts[1].to_string() } else { "".to_string() };

        Ok((namespace, relative_path))
    }

    /// 生成文档ID
    fn generate_document_id(namespace: &str, file_name: &str) -> String {
        format!("{}:{}:{}", namespace, file_name, Uuid::new_v4())
    }

    /// 分块文档
    fn chunk_document(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < chars.len() {
            let end = std::cmp::min(start + chunk_size, chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end == chars.len() {
                break;
            }
            start = end - chunk_overlap;
        }

        chunks
    }

    /// 添加索引任务
    async fn add_index_task(&self, namespace: String, document_id: String, file_name: String, data: String) {
        let task = IndexTask {
            namespace,
            document_id,
            file_name,
            data,
        };

        let mut queue = self.index_queue.lock().await;
        queue.push(task);
    }

    /// 获取索引状态
    async fn get_indexing_status(&self, namespace: &str) -> String {
        let status = self.indexing_status.read().await;
        if let Some(ns_status) = status.get(namespace) {
            serde_json::to_string_pretty(ns_status).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        }
    }

    /// 创建命名空间
    async fn create_namespace(&self, namespace: &str) -> EvifResult<()> {
        if namespace.is_empty() {
            return Err(EvifError::InvalidPath("Invalid namespace name".to_string()));
        }

        let mut namespaces = self.namespaces.write().await;
        if namespaces.contains_key(namespace) {
            return Err(EvifError::Other(format!("Namespace already exists: {}", namespace)));
        }

        let created_at = Utc::now();
        namespaces.insert(namespace.to_string(), Namespace {
            name: namespace.to_string(),
            documents: HashMap::new(),
            created_at,
        });

        // 持久化到 SQLite
        #[cfg(feature = "rusqlite")]
        self.persist_create_namespace(namespace, created_at)?;

        Ok(())
    }

    /// 删除命名空间
    async fn delete_namespace(&self, namespace: &str) -> EvifResult<()> {
        if namespace.is_empty() {
            return Err(EvifError::Other("Cannot remove root directory".to_string()));
        }

        let mut namespaces = self.namespaces.write().await;
        namespaces.remove(namespace)
            .ok_or_else(|| EvifError::NotFound(format!("Namespace not found: {}", namespace)))?;

        // 清理索引状态
        let mut status = self.indexing_status.write().await;
        status.remove(namespace);

        // 持久化删除到 SQLite
        #[cfg(feature = "rusqlite")]
        self.persist_delete_namespace(namespace)?;

        Ok(())
    }

    /// 写入文档
    async fn write_document(&self, namespace: &str, file_name: &str, data: &[u8]) -> EvifResult<String> {
        let text = String::from_utf8(data.to_vec())
            .map_err(|_| EvifError::Other("Invalid UTF-8".to_string()))?;

        let document_id = Self::generate_document_id(namespace, file_name);
        let _chunks = Self::chunk_document(&text, self.config.chunk_size, self.config.chunk_overlap);

        // 确保命名空间存在
        {
            let namespaces = self.namespaces.read().await;
            if !namespaces.contains_key(namespace) {
                return Err(EvifError::NotFound(format!("Namespace not found: {}", namespace)));
            }
        }

        // 尝试生成 embedding
        let embedding = if !text.is_empty() {
            match self.embedding_provider.embed(&text).await {
                Ok(emb) => Some(emb),
                Err(e) => {
                    log::debug!("Embedding generation failed for {}: {}", file_name, e);
                    None
                }
            }
        } else {
            None
        };

        // 创建文档
        let mut namespaces = self.namespaces.write().await;
        if let Some(ns) = namespaces.get_mut(namespace) {
            let doc = VectorDocument {
                id: document_id.clone(),
                namespace: namespace.to_string(),
                file_name: file_name.to_string(),
                chunk_index: 0,
                content: text.clone(),
                embedding,
                created_at: Utc::now(),
                s3_key: format!("{}/{}/{}", namespace, file_name, Uuid::new_v4()),
            };

            // 持久化到 SQLite
            #[cfg(feature = "rusqlite")]
            self.persist_write_document(&doc)?;

            ns.documents.insert(document_id.clone(), doc);
        }

        // 添加到索引队列
        self.add_index_task(
            namespace.to_string(),
            document_id.clone(),
            file_name.to_string(),
            text.clone(),
        ).await;

        Ok(document_id)
    }

    /// 计算余弦相似度
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    /// 搜索文档 (支持向量语义搜索和文本匹配)
    async fn search_documents(&self, namespace: &str, query: &str, limit: usize) -> Vec<VectorDocument> {
        let namespaces = self.namespaces.read().await;
        let ns = match namespaces.get(namespace) {
            Some(ns) => ns,
            None => return Vec::new(),
        };

        // 尝试向量语义搜索
        let query_embedding = self.embedding_provider.embed(query).await.ok();

        let mut scored: Vec<(f32, &VectorDocument)> = ns.documents.values()
            .filter_map(|doc| {
                let score = match (&query_embedding, &doc.embedding) {
                    (Some(q_emb), Some(d_emb)) => Self::cosine_similarity(q_emb, d_emb),
                    _ => {
                        // 退回到文本匹配
                        if doc.content.to_lowercase().contains(&query.to_lowercase()) {
                            0.5
                        } else {
                            0.0
                        }
                    }
                };
                if score > 0.0 {
                    Some((score, doc))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter()
            .take(limit)
            .map(|(_, doc)| doc.clone())
            .collect()
    }
}

#[async_trait]
impl EvifPlugin for VectorFsPlugin {
    fn name(&self) -> &str {
        "VectorFS"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let (namespace, relative_path) = Self::parse_path(path)?;

        // 创建空文件
        if !namespace.is_empty() && !relative_path.is_empty() {
            self.write_document(&namespace, &relative_path, &[]).await?;
        }

        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let (namespace, relative_path) = Self::parse_path(path)?;

        // 只能创建命名空间目录或docs/子目录
        if !relative_path.is_empty() {
            if relative_path.starts_with("docs/") {
                // 创建占位文件使目录可见
                let keep_file = format!("{}/.keep", path);
                self.write_document(&namespace, &keep_file, b"").await?;
                return Ok(());
            }
            return Err(EvifError::Other("Can only create namespace directories or docs/ subdirectories".to_string()));
        }

        if namespace.is_empty() {
            return Err(EvifError::InvalidPath("Invalid namespace name".to_string()));
        }

        self.create_namespace(&namespace).await
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        if path == "/README" || path == "README" {
            let readme = r#"# VectorFS - 向量搜索文件系统

向量搜索插件,支持文档索引和语义搜索。

## 功能特性

- **文档索引**: 自动分块和索引文档
- **语义搜索**: 基于向量相似度的语义搜索
- **命名空间**: 支持多个独立的文档集合
- **S3存储**: 文档存储在S3(或兼容存储)
- **异步索引**: 后台异步处理文档索引

## 路径结构

```
/
├── README                 # 本文档
├── namespace1/           # 命名空间1
│   ├── .indexing         # 索引状态(虚拟文件)
│   └── docs/             # 文档目录
│       ├── file1.txt
│       └── file2.txt
└── namespace2/           # 命名空间2
    └── docs/
        └── file3.txt
```

## 使用示例

创建命名空间:
```
mkdir /vectorfs/mynamespace
```

写入文档:
```
POST /vectorfs/mynamespace/docs/doc1.txt
Content-Type: text/plain

This is a sample document...
```

搜索文档:
```
GET /vectorfs/mynamespace/search?q=sample
```

查看索引状态:
```
GET /vectorfs/mynamespace/.indexing
```

## 配置

- `s3_bucket`: S3存储桶名称
- `s3_key_prefix`: S3键前缀(默认: "vectorfs")
- `embedding_dim`: 向量维度(默认: 1536, OpenAI text-embedding-3-small)
- `chunk_size`: 文档分块大小(默认: 512字符)
- `chunk_overlap`: 分块重叠(默认: 50字符)
- `index_workers`: 索引worker数量(默认: 4)
"#;
            let data = readme.as_bytes();
            let start = offset as usize;
            let end = if size == 0 {
                data.len()
            } else {
                std::cmp::min(start + size as usize, data.len())
            };
            return Ok(data[start..end].to_vec());
        }

        let (namespace, relative_path) = Self::parse_path(path)?;

        // 处理.indexing虚拟文件
        if relative_path == ".indexing" {
            let status = self.get_indexing_status(&namespace).await;
            return Ok(status.into_bytes());
        }

        // 读取文档
        let namespaces = self.namespaces.read().await;
        if let Some(ns) = namespaces.get(&namespace) {
            for doc in ns.documents.values() {
                if doc.file_name == relative_path {
                    let data = doc.content.as_bytes();
                    let start = offset as usize;
                    let end = if size == 0 {
                        data.len()
                    } else {
                        std::cmp::min(start + size as usize, data.len())
                    };
                    return Ok(data[start..end].to_vec());
                }
            }
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        let (namespace, relative_path) = Self::parse_path(path)?;

        if namespace.is_empty() || relative_path.is_empty() {
            return Err(EvifError::InvalidPath("Invalid path".to_string()));
        }

        // 确保命名空间存在
        {
            let namespaces = self.namespaces.read().await;
            if !namespaces.contains_key(&namespace) {
                return Err(EvifError::NotFound(format!("Namespace not found: {}", namespace)));
            }
        }

        self.write_document(&namespace, &relative_path, &data).await?;
        Ok(data.len() as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let (namespace, relative_path) = Self::parse_path(path)?;

        let mut entries = Vec::new();

        // 根目录
        if namespace.is_empty() {
            entries.push(FileInfo {
                name: "README".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o644,
                is_dir: false,
            });

            let namespaces = self.namespaces.read().await;
            for (name, ns) in namespaces.iter() {
                entries.push(FileInfo {
                    name: name.clone(),
                    size: 0,
                    modified: ns.created_at,
                    mode: 0o755,
                    is_dir: true,
                });
            }
            return Ok(entries);
        }

        // 命名空间目录
        if relative_path.is_empty() {
            entries.push(FileInfo {
                name: ".indexing".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o644,
                is_dir: false,
            });

            entries.push(FileInfo {
                name: "docs".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o755,
                is_dir: true,
            });

            return Ok(entries);
        }

        // docs/子目录
        if relative_path == "docs" {
            let namespaces = self.namespaces.read().await;
            if let Some(ns) = namespaces.get(&namespace) {
                let mut file_names = std::collections::HashSet::new();
                for doc in ns.documents.values() {
                    if !file_names.contains(&doc.file_name) {
                        file_names.insert(doc.file_name.clone());
                        entries.push(FileInfo {
                            name: doc.file_name.clone(),
                            size: doc.content.len() as u64,
                            modified: doc.created_at,
                            mode: 0o644,
                            is_dir: false,
                        });
                    }
                }
            }
            return Ok(entries);
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        if path == "/" || path.is_empty() {
            return Ok(FileInfo {
                name: "/".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o755,
                is_dir: true,
            });
        }

        if path == "/README" || path == "README" {
            return Ok(FileInfo {
                name: "README".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o644,
                is_dir: false,
            });
        }

        let (namespace, relative_path) = Self::parse_path(path)?;

        // 命名空间目录
        if relative_path.is_empty() {
            let namespaces = self.namespaces.read().await;
            if let Some(ns) = namespaces.get(&namespace) {
                return Ok(FileInfo {
                    name: namespace.clone(),
                    size: 0,
                    modified: ns.created_at,
                    mode: 0o755,
                    is_dir: true,
                });
            }
            return Err(EvifError::NotFound(path.to_string()));
        }

        // 文档
        let namespaces = self.namespaces.read().await;
        if let Some(ns) = namespaces.get(&namespace) {
            for doc in ns.documents.values() {
                if doc.file_name == relative_path {
                    return Ok(FileInfo {
                        name: doc.file_name.clone(),
                        size: doc.content.len() as u64,
                        modified: doc.created_at,
                        mode: 0o644,
                        is_dir: false,
                    });
                }
            }
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let (namespace, relative_path) = Self::parse_path(path)?;

        // 只允许删除整个命名空间
        if !relative_path.is_empty() {
            return Err(EvifError::Other("Can only remove entire namespace, not subdirectories".to_string()));
        }

        if namespace.is_empty() {
            return Err(EvifError::Other("Cannot remove root directory".to_string()));
        }

        self.delete_namespace(&namespace).await
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vectorfs_basic() {
        let plugin = VectorFsPlugin::new(VectorFsConfig::default());

        // 创建命名空间
        plugin.mkdir("/testns", 0o755).await.unwrap();

        // 写入文档
        let content = b"This is a test document for vector search.".to_vec();
        plugin.write("/testns/docs/doc1.txt", content, 0, WriteFlags::CREATE).await.unwrap();

        // 列出文档
        let entries = plugin.readdir("/testns/docs").await.unwrap();
        assert_eq!(entries.len(), 1);
        // 文件名包含完整路径
        assert!(entries[0].name.contains("doc1.txt"));

        // 删除命名空间
        plugin.remove_all("/testns").await.unwrap();
    }

    #[tokio::test]
    async fn test_vectorfs_multiple_namespaces() {
        let plugin = VectorFsPlugin::new(VectorFsConfig::default());

        // 创建多个命名空间
        plugin.mkdir("/ns1", 0o755).await.unwrap();
        plugin.mkdir("/ns2", 0o755).await.unwrap();

        // 写入不同命名空间的文档
        plugin.write("/ns1/docs/doc1.txt", b"Document 1".to_vec(), 0, WriteFlags::CREATE).await.unwrap();
        plugin.write("/ns2/docs/doc2.txt", b"Document 2".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 列出根目录
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.len() >= 3); // README + ns1 + ns2

        // 清理
        plugin.remove_all("/ns1").await.unwrap();
        plugin.remove_all("/ns2").await.unwrap();
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let c = vec![1.0, 0.0, 0.0];

        // Orthogonal vectors should have 0 similarity
        let sim_ab = VectorFsPlugin::cosine_similarity(&a, &b);
        assert!((sim_ab - 0.0).abs() < 0.001);

        // Identical vectors should have similarity 1.0
        let sim_ac = VectorFsPlugin::cosine_similarity(&a, &c);
        assert!((sim_ac - 1.0).abs() < 0.001);

        // Empty vectors
        let empty: Vec<f32> = vec![];
        assert_eq!(VectorFsPlugin::cosine_similarity(&empty, &a), 0.0);
    }

    #[tokio::test]
    async fn test_no_embedding_provider() {
        let provider = NoEmbeddingProvider;
        assert_eq!(provider.name(), "none");
        let result = provider.embed("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_text_fallback_search() {
        let plugin = VectorFsPlugin::new(VectorFsConfig::default());

        plugin.mkdir("/searchns", 0o755).await.unwrap();
        plugin.write("/searchns/docs/rust.txt", b"Rust is a systems programming language".to_vec(), 0, WriteFlags::CREATE).await.unwrap();
        plugin.write("/searchns/docs/python.txt", b"Python is a scripting language".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        let results = plugin.search_documents("searchns", "rust", 10).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));

        plugin.remove_all("/searchns").await.unwrap();
    }
}
