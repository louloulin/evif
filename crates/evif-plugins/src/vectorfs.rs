// VectorFS - 向量搜索文件系统插件 (简化版本)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// 向量搜索配置
#[derive(Debug, Clone)]
pub struct VectorFsConfig {
    /// S3配置
    pub s3_bucket: String,
    pub s3_key_prefix: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,

    /// 向量维度 (OpenAI text-embedding-3-small = 1536)
    pub embedding_dim: usize,

    /// 文档分块大小
    pub chunk_size: usize,
    pub chunk_overlap: usize,

    /// 索引worker数量
    pub index_workers: usize,
}

impl Default for VectorFsConfig {
    fn default() -> Self {
        Self {
            s3_bucket: "vectorfs-docs".to_string(),
            s3_key_prefix: Some("vectorfs".to_string()),
            s3_region: Some("us-east-1".to_string()),
            s3_endpoint: None,
            embedding_dim: 1536,
            chunk_size: 512,
            chunk_overlap: 50,
            index_workers: 4,
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
    embedding: Option<Vec<f32>>, // 简化:不实际存储向量
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

    // S3客户端引用 (可选,用于实际存储)
    // s3_client: Option<Arc<S3Client>>,
}

impl VectorFsPlugin {
    pub fn new(config: VectorFsConfig) -> Self {
        Self {
            config,
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            index_queue: Arc::new(Mutex::new(Vec::new())),
            indexing_status: Arc::new(RwLock::new(HashMap::new())),
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

        namespaces.insert(namespace.to_string(), Namespace {
            name: namespace.to_string(),
            documents: HashMap::new(),
            created_at: Utc::now(),
        });

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

        Ok(())
    }

    /// 写入文档
    async fn write_document(&self, namespace: &str, file_name: &str, data: &[u8]) -> EvifResult<String> {
        let text = String::from_utf8(data.to_vec())
            .map_err(|_| EvifError::Other("Invalid UTF-8".to_string()))?;

        let document_id = Self::generate_document_id(namespace, file_name);
        let chunks = Self::chunk_document(&text, self.config.chunk_size, self.config.chunk_overlap);

        // 模拟创建向量文档
        let namespaces = self.namespaces.read().await;
        let namespace_obj = namespaces.get(namespace)
            .ok_or_else(|| EvifError::NotFound(format!("Namespace not found: {}", namespace)))?;

        // 注意: 这里需要drop读锁,因为后面需要获取写锁
        drop(namespaces);

        // 创建文档
        let mut namespaces = self.namespaces.write().await;
        if let Some(ns) = namespaces.get_mut(namespace) {
            let doc = VectorDocument {
                id: document_id.clone(),
                namespace: namespace.to_string(),
                file_name: file_name.to_string(),
                chunk_index: 0,
                content: text.clone(),
                embedding: None, // 简化版本不实际生成向量
                created_at: Utc::now(),
                s3_key: format!("{}/{}/{}", namespace, file_name, Uuid::new_v4()),
            };
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

    /// 搜索文档 (简化版:基于文本匹配)
    async fn search_documents(&self, namespace: &str, query: &str, limit: usize) -> Vec<VectorDocument> {
        let namespaces = self.namespaces.read().await;
        if let Some(ns) = namespaces.get(namespace) {
            let query_lower = query.to_lowercase();
            ns.documents.values()
                .filter(|doc| doc.content.to_lowercase().contains(&query_lower))
                .take(limit)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
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
        Err(EvifError::NotSupported)
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
        Err(EvifError::NotSupported)
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
}
