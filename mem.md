# Mem 记忆平台设计计划

## 一、项目概述

### 1.1 目标
基于 EVIF 虚拟文件系统，构建一个类似于 memU 的记忆平台。该平台将利用 EVIF 的"一切皆文件"哲学，将记忆抽象为文件系统操作，实现高效的记忆存储、检索和管理。

### 1.2 核心理念
**记忆即文件系统，文件系统即记忆**

| 文件系统概念 | 记忆平台对应 |
|-------------|-------------|
| 📁 文件夹 | 🏷️ 类别（自动组织的主题） |
| 📄 文件 | 🧠 记忆项（提取的事实、偏好、技能） |
| 🔗 符号链接 | 🔄 交叉引用（关联的记忆链接） |
| 📂 挂载点 | 📥 资源（对话、文档、图像） |

### 1.3 与 memU 的核心差异

| 特性 | memU (Python) | Mem (Rust on EVIF) |
|------|--------------|-------------------|
| 语言 | Python 3.13+ | Rust 1.70+ |
| 基础设施 | 独立框架 | 基于 EVIF 文件系统 |
| 存储后端 | PostgreSQL/SQLite/内存 | RocksDB/Sled/SQLite/内存 |
| 向量支持 | pgvector | sqlite-vec/自定义索引 |
| 图引擎 | 无内置 | EVIF Graph |
| 访问方式 | Python API | REST/CLI/FUSE/MCP |
| 性能 | 中等 | 高（Rust + 异步） |
| 主动记忆 | 支持 | 支持（计划中） |

---

## 二、架构分析

### 2.1 memU 架构深度分析

#### 2.1.1 核心数据模型 (`memu/database/models.py`)

```python
# 原始资源 - 存储对话、文档、图像等原始数据
class Resource(BaseRecord):
    url: str                    # 资源URL或路径
    modality: str               # conversation/document/image/video/audio
    local_path: str             # 本地存储路径
    caption: str | None         # 资源描述
    embedding: list[float]      # 向量嵌入（用于相似性检索）

# 记忆项 - 从资源中提取的结构化记忆
class MemoryItem(BaseRecord):
    resource_id: str | None     # 关联的资源ID
    memory_type: str            # profile/event/knowledge/behavior/skill/tool
    summary: str                # 记忆摘要
    embedding: list[float]      # 向量嵌入
    happened_at: datetime       # 事件发生时间
    extra: dict                 # 扩展字段：
                                # - content_hash: 内容哈希（去重）
                                # - reinforcement_count: 强化次数
                                # - ref_id: 引用ID
                                # - tool_calls: 工具调用记录

# 记忆类别 - 自动组织的主题分类
class MemoryCategory(BaseRecord):
    name: str                   # 类别名称
    description: str            # 类别描述
    embedding: list[float]      # 类别嵌入向量
    summary: str                # 类别摘要（动态更新）

# 类别-项目关联
class CategoryItem(BaseRecord):
    item_id: str
    category_id: str
```

**关键设计点：**
- **三层架构**：Resource（原始数据）→ Item（提取的记忆）→ Category（自动分类）
- **内容哈希去重**：使用 SHA256 哈希防止重复记忆
- **强化学习机制**：重复出现的记忆会增加 reinforcement_count
- **引用追踪**：支持记忆项之间的交叉引用 [ref:xxx]

#### 2.1.2 记忆化管道 (`memu/app/memorize.py`)

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Memorize Pipeline                                │
├─────────────────────────────────────────────────────────────────────┤
│  1. ingest_resource     → 获取资源，加载原始文本                      │
│  2. preprocess_multimodal → 多模态预处理（对话分段、图像描述等）      │
│  3. extract_items       → LLM提取结构化记忆项                        │
│  4. dedupe_merge        → 去重和合并                                 │
│  5. categorize_items    → 向量化 + 自动分类                          │
│  6. persist_index       → 持久化 + 更新类别摘要                      │
│  7. build_response      → 构建返回结果                               │
└─────────────────────────────────────────────────────────────────────┘
```

**核心步骤详解：**

1. **资源摄取 (ingest_resource)**
   - 支持多种模态：conversation, document, image, video, audio
   - 下载或读取资源到本地路径

2. **多模态预处理 (preprocess_multimodal)**
   - conversation: 对话分段，提取关键片段
   - image/video: 使用 Vision API 生成描述
   - audio: 语音转文字（STT）
   - document: 文档压缩和摘要

3. **记忆提取 (extract_items)**
   - 使用 LLM 从预处理内容中提取结构化记忆
   - 六种记忆类型：profile, event, knowledge, behavior, skill, tool
   - 每种类型有专门的提取 prompt

4. **自动分类 (categorize_items)**
   - 生成记忆项的嵌入向量
   - 匹配到预定义的类别
   - 创建 CategoryItem 关联

5. **类别摘要更新 (persist_index)**
   - 使用 LLM 生成/更新类别摘要
   - 支持引用追踪 [ref:xxx]

#### 2.1.3 检索管道 (`memu/app/retrieve.py`)

**双模式检索：**

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Retrieve Pipeline                               │
├─────────────────────────────────────────────────────────────────────┤
│  RAG 模式（快速）                                                     │
│  1. embed_queries    → 查询向量化                                    │
│  2. vector_search    → 向量相似性搜索                                │
│  3. rank_results     → 结果排序                                      │
│                                                                      │
│  LLM 模式（深度）                                                     │
│  1. understand_intent → LLM理解查询意图                              │
│  2. generate_strategy → 生成搜索策略                                 │
│  3. multi_round_search → 多轮检索                                    │
│  4. synthesize_result → 结果综合                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 EVIF 架构深度分析

#### 2.2.1 插件系统核心 (`evif-core/src/plugin.rs`)

```rust
#[async_trait]
pub trait EvifPlugin: Send + Sync {
    // 基础信息
    fn name(&self) -> &str;
    fn get_readme(&self) -> String { String::new() }
    fn get_config_params(&self) -> Vec<PluginConfigParam> { vec![] }

    // 文件操作
    async fn create(&self, path: &str, perm: u32) -> EvifResult<()>;
    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>;
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;
    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<u64>;
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;
    async fn remove(&self, path: &str) -> EvifResult<()>;
    async fn remove_all(&self, path: &str) -> EvifResult<()>;
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;

    // 符号链接（用于交叉引用）
    async fn symlink(&self, target: &str, link: &str) -> EvifResult<()>;
    async fn readlink(&self, path: &str) -> EvifResult<String>;

    // 扩展
    async fn validate(&self, config: Option<&Value>) -> EvifResult<()>;
    fn as_streamer(&self) -> Option<&dyn Streamer>;
    fn as_handle_fs(&self) -> Option<&dyn HandleFS>;
}
```

**关键特性：**
- **Radix Tree 路由**：O(k) 路径查找，k 为路径长度
- **并发安全**：DashMap + RwLock 实现无锁读操作
- **流式传输**：支持大文件分块读写
- **动态加载**：运行时加载 .so/.dylib/.dll 插件

#### 2.2.2 图引擎 (`evif-graph/`)

```rust
// 核心图结构
pub struct Graph {
    nodes: DashMap<NodeId, Node>,
    edges: DashMap<EdgeId, Edge>,
    adjacency_out: DashMap<NodeId, Vec<EdgeId>>,
    adjacency_in: DashMap<NodeId, Vec<EdgeId>>,
}

// 节点定义
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub node_type: NodeType,    // File/Directory/Custom
    pub attributes: HashMap<String, Attribute>,
    pub content: Option<ContentHandle>,
}

// 边定义
pub struct Edge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub edge_type: EdgeType,    // Parent/Child/Custom
    pub weight: f64,
}

// 图引擎
pub struct GraphEngine {
    graph: Arc<Graph>,
}

impl GraphEngine {
    pub fn detect_cycle(&self) -> bool;
    pub fn bfs(&self, start: NodeId) -> Result<Vec<NodeId>>;
    pub fn dfs(&self, start: NodeId) -> Result<Vec<NodeId>>;
}
```

**记忆图应用：**
- 节点：MemoryItem, MemoryCategory, Resource
- 边：BELONGS_TO, REFERENCES, SIMILAR_TO
- 查询：邻居遍历、路径查找、子图提取

#### 2.2.3 VectorFS 插件参考 (`evif-plugins/src/vectorfs.rs`)

```rust
pub struct VectorFsPlugin {
    config: VectorFsConfig,
    namespaces: Arc<RwLock<HashMap<String, Namespace>>>,
    index_queue: Arc<Mutex<Vec<IndexTask>>>,
}

impl VectorFsPlugin {
    // 路径格式: /namespace/docs/file.txt
    fn parse_path(path: &str) -> EvifResult<(String, String)>;

    // 文档分块
    fn chunk_document(text: &str, chunk_size: usize, overlap: usize) -> Vec<String>;

    // 写入文档 → 自动索引
    async fn write_document(&self, ns: &str, name: &str, data: &[u8]) -> EvifResult<String>;

    // 语义搜索（简化版：文本匹配）
    async fn search_documents(&self, ns: &str, query: &str, limit: usize) -> Vec<VectorDocument>;
}
```

**可复用设计模式：**
- 命名空间隔离
- 异步索引队列
- 虚拟文件（.indexing 状态文件）
- 文档分块策略

---

## 三、Mem 插件设计

### 3.1 系统架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Mem 记忆平台                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  访问层                                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ REST API │  │   CLI    │  │   FUSE   │  │WebSocket │  │   MCP    │     │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘     │
├───────┴────────────┴─────────┴─────────┴─────────┴─────────┴───────────────┤
│  核心层                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  MemPlugin   │  │  MemoryGraph │  │ EmbeddingMgr │  │  LLM Client  │   │
│  │ (EVIF 插件)  │  │  (图引擎)    │  │ (嵌入管理)   │  │ (LLM 调用)   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│  EVIF 基础设施                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Mount Table  │  │ Plugin System│  │ Handle Mgr   │  │ VFS Layer    │   │
│  │ (Radix Tree) │  │              │  │              │  │              │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│  存储层                                                                      │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐       │
│  │Memory  │ │ RocksDB│ │  Sled  │ │   S3   │ │ Vector │ │ SQLite │       │
│  │(缓存)  │ │(持久化)│ │(快速)  │ │(云端)  │ │(向量)  │ │(轻量)  │       │
│  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘ └────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 MemPlugin 核心实现

```rust
// crates/evif-mem/src/mem_plugin.rs

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Profile,    // 个人资料
    Event,      // 事件记忆
    Knowledge,  // 知识记忆
    Behavior,   // 行为记忆
    Skill,      // 技能记忆
    Tool,       // 工具记忆
}

/// 原始资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub url: String,
    pub modality: String,           // conversation/document/image/video/audio
    pub local_path: String,
    pub caption: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 记忆项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub resource_id: Option<String>,
    pub memory_type: MemoryType,
    pub summary: String,
    pub embedding: Option<Vec<f32>>,
    pub happened_at: Option<DateTime<Utc>>,
    pub extra: HashMap<String, Value>,  // content_hash, reinforcement_count, ref_id
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 记忆类别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub embedding: Option<Vec<f32>>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 类别-项目关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryItem {
    pub id: String,
    pub item_id: String,
    pub category_id: String,
    pub created_at: DateTime<Utc>,
}

/// MemPlugin - EVIF 记忆插件
pub struct MemPlugin {
    // 内存存储
    resources: DashMap<String, Resource>,
    items: DashMap<String, MemoryItem>,
    categories: DashMap<String, MemoryCategory>,
    relations: DashMap<String, CategoryItem>,

    // 用户作用域索引
    user_resources: DashMap<String, Vec<String>>,      // user_id -> resource_ids
    user_items: DashMap<String, Vec<String>>,          // user_id -> item_ids
    user_categories: DashMap<String, Vec<String>>,     // user_id -> category_ids

    // 类别名称索引
    category_name_index: DashMap<String, String>,      // name -> category_id

    // 嵌入向量管理
    embedding_manager: Arc<EmbeddingManager>,

    // 记忆图（关系查询）
    graph: Arc<MemoryGraph>,

    // 持久化后端
    storage: Arc<dyn StorageBackend>,

    // LLM 客户端
    llm_client: Arc<dyn LLMClient>,

    // 配置
    config: MemPluginConfig,
}

/// 路径解析结果
enum MemPath {
    Root,
    UserScope { user_id: String },
    Resources { user_id: String },
    Resource { user_id: String, resource_id: String },
    Items { user_id: String },
    Item { user_id: String, item_id: String },
    ItemsByType { user_id: String, memory_type: MemoryType },
    Categories { user_id: String },
    Category { user_id: String, category_name: String },
    CategoryItems { user_id: String, category_name: String },
    Graph { user_id: String },
    System,
    SystemConfig,
    Search { user_id: String },
    Memorize { user_id: String },
}

impl MemPlugin {
    /// 解析路径
    fn parse_path(path: &str) -> EvifResult<MemPath> {
        let parts: Vec<&str> = path.trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        match parts.as_slice() {
            [] => Ok(MemPath::Root),
            ["_system"] => Ok(MemPath::System),
            ["_system", "config"] => Ok(MemPath::SystemConfig),
            [user_id] => Ok(MemPath::UserScope { user_id: user_id.to_string() }),
            [user_id, "resources"] => Ok(MemPath::Resources { user_id: user_id.to_string() }),
            [user_id, "resources", resource_id] => Ok(MemPath::Resource {
                user_id: user_id.to_string(),
                resource_id: resource_id.to_string()
            }),
            [user_id, "items"] => Ok(MemPath::Items { user_id: user_id.to_string() }),
            [user_id, "items", item_id] => Ok(MemPath::Item {
                user_id: user_id.to_string(),
                item_id: item_id.to_string()
            }),
            [user_id, "items", "by-type", type_str] => {
                let memory_type = Self::parse_memory_type(type_str)?;
                Ok(MemPath::ItemsByType {
                    user_id: user_id.to_string(),
                    memory_type
                })
            },
            [user_id, "categories"] => Ok(MemPath::Categories { user_id: user_id.to_string() }),
            [user_id, "categories", name] => Ok(MemPath::Category {
                user_id: user_id.to_string(),
                category_name: name.to_string()
            }),
            [user_id, "categories", name, "items"] => Ok(MemPath::CategoryItems {
                user_id: user_id.to_string(),
                category_name: name.to_string()
            }),
            [user_id, "graph"] => Ok(MemPath::Graph { user_id: user_id.to_string() }),
            [user_id, ".search"] => Ok(MemPath::Search { user_id: user_id.to_string() }),
            [user_id, ".memorize"] => Ok(MemPath::Memorize { user_id: user_id.to_string() }),
            _ => Err(EvifError::InvalidPath(path.to_string())),
        }
    }

    /// 记忆化（核心 API）
    pub async fn memorize(
        &self,
        user_id: &str,
        resource_url: &str,
        modality: &str,
    ) -> EvifResult<MemorizeResult> {
        // 1. 摄取资源
        let resource = self.ingest_resource(resource_url, modality).await?;

        // 2. 预处理
        let preprocessed = self.preprocess_multimodal(&resource).await?;

        // 3. 提取记忆项
        let items = self.extract_items(&resource, &preprocessed).await?;

        // 4. 生成嵌入向量
        let items_with_embeddings = self.generate_embeddings(items).await?;

        // 5. 自动分类
        let (items, relations) = self.categorize_items(user_id, items_with_embeddings).await?;

        // 6. 持久化
        self.persist_memories(user_id, &resource, &items, &relations).await?;

        // 7. 更新图索引
        self.update_graph_index(user_id, &items, &relations).await?;

        Ok(MemorizeResult {
            resource,
            items,
            relations,
        })
    }

    /// 检索（核心 API）
    pub async fn retrieve(
        &self,
        user_id: &str,
        query: &str,
        method: RetrieveMethod,
        limit: usize,
    ) -> EvifResult<RetrieveResult> {
        match method {
            RetrieveMethod::Rag => self.rag_retrieve(user_id, query, limit).await,
            RetrieveMethod::Llm => self.llm_retrieve(user_id, query, limit).await,
            RetrieveMethod::Hybrid => self.hybrid_retrieve(user_id, query, limit).await,
        }
    }
}

#[async_trait]
impl EvifPlugin for MemPlugin {
    fn name(&self) -> &str {
        "mem"
    }

    fn get_readme(&self) -> String {
        include_str!("../README.md").to_string()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![
            PluginConfigParam {
                name: "embedding_dim".to_string(),
                param_type: "int".to_string(),
                required: false,
                default: Some("1536".to_string()),
                description: Some("Embedding dimension (default: 1536 for OpenAI)".to_string()),
            },
            PluginConfigParam {
                name: "llm_provider".to_string(),
                param_type: "string".to_string(),
                required: false,
                default: Some("openai".to_string()),
                description: Some("LLM provider: openai, anthropic, openrouter".to_string()),
            },
        ]
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        match Self::parse_path(path)? {
            MemPath::Category { user_id, category_name } => {
                self.create_category(&user_id, &category_name).await
            },
            MemPath::Item { user_id, item_id } => {
                // 创建空记忆项（后续通过 write 填充内容）
                self.create_empty_item(&user_id, &item_id).await
            },
            _ => Err(EvifError::NotSupported),
        }
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        match Self::parse_path(path)? {
            MemPath::UserScope { user_id } => {
                self.init_user_scope(&user_id).await
            },
            MemPath::Category { user_id, category_name } => {
                self.create_category(&user_id, &category_name).await
            },
            _ => Err(EvifError::NotSupported),
        }
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let content = match Self::parse_path(path)? {
            MemPath::Resource { user_id, resource_id } => {
                let resource = self.get_resource(&user_id, &resource_id)?;
                serde_json::to_string_pretty(&resource)?
            },
            MemPath::Item { user_id, item_id } => {
                let item = self.get_item(&user_id, &item_id)?;
                serde_json::to_string_pretty(&item)?
            },
            MemPath::Category { user_id, category_name } => {
                let category = self.get_category_by_name(&user_id, &category_name)?;
                serde_json::to_string_pretty(&category)?
            },
            MemPath::Search { user_id } => {
                // 读取搜索结果（通过写入查询触发）
                self.get_search_results(&user_id)?
            },
            _ => return Err(EvifError::NotFound(path.to_string())),
        };

        let bytes = content.as_bytes();
        let start = offset as usize;
        let end = if size == 0 { bytes.len() } else {
            std::cmp::min(start + size as usize, bytes.len())
        };
        Ok(bytes[start..end].to_vec())
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<u64> {
        match Self::parse_path(path)? {
            MemPath::Memorize { user_id } => {
                // 写入到 .memorize 触发记忆化流程
                let request: MemorizeRequest = serde_json::from_slice(&data)?;
                let result = self.memorize(&user_id, &request.resource_url, &request.modality).await?;
                let result_json = serde_json::to_string_pretty(&result)?;
                // 存储结果供后续读取
                self.store_memorize_result(&user_id, result_json)?;
                Ok(data.len() as u64)
            },
            MemPath::Search { user_id } => {
                // 写入查询触发搜索
                let query: RetrieveRequest = serde_json::from_slice(&data)?;
                let result = self.retrieve(&user_id, &query.query, query.method, query.limit).await?;
                let result_json = serde_json::to_string_pretty(&result)?;
                self.store_search_result(&user_id, result_json)?;
                Ok(data.len() as u64)
            },
            MemPath::Item { user_id, item_id } => {
                // 直接写入/更新记忆项
                let item: MemoryItem = serde_json::from_slice(&data)?;
                self.update_item(&user_id, &item_id, item).await?;
                Ok(data.len() as u64)
            },
            _ => Err(EvifError::NotSupported),
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        match Self::parse_path(path)? {
            MemPath::Root => {
                // 列出所有用户作用域
                Ok(self.list_user_scopes()?)
            },
            MemPath::UserScope { user_id } => {
                Ok(vec![
                    FileInfo { name: "resources".to_string(), is_dir: true, ..Default::default() },
                    FileInfo { name: "items".to_string(), is_dir: true, ..Default::default() },
                    FileInfo { name: "categories".to_string(), is_dir: true, ..Default::default() },
                    FileInfo { name: "graph".to_string(), is_dir: true, ..Default::default() },
                    FileInfo { name: ".search".to_string(), is_dir: false, ..Default::default() },
                    FileInfo { name: ".memorize".to_string(), is_dir: false, ..Default::default() },
                ])
            },
            MemPath::Resources { user_id } => {
                self.list_resources(&user_id)
            },
            MemPath::Items { user_id } => {
                self.list_items(&user_id)
            },
            MemPath::ItemsByType { user_id, memory_type } => {
                self.list_items_by_type(&user_id, &memory_type)
            },
            MemPath::Categories { user_id } => {
                self.list_categories(&user_id)
            },
            MemPath::CategoryItems { user_id, category_name } => {
                self.list_category_items(&user_id, &category_name)
            },
            _ => Err(EvifError::NotSupported),
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        // 实现状态查询
        todo!()
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        match Self::parse_path(path)? {
            MemPath::Item { user_id, item_id } => {
                self.delete_item(&user_id, &item_id).await
            },
            MemPath::Category { user_id, category_name } => {
                self.delete_category(&user_id, &category_name).await
            },
            _ => Err(EvifError::NotSupported),
        }
    }

    async fn symlink(&self, target: &str, link: &str) -> EvifResult<()> {
        // 实现交叉引用
        // target: /mem/{user}/items/{item_id}
        // link: /mem/{user}/categories/{name}/items/{ref_id}
        self.create_cross_reference(target, link).await
    }
}
```

### 3.3 MemoryGraph 图引擎

```rust
// crates/evif-mem/src/memory_graph.rs

use evif_graph::{Graph, Node, Edge, NodeId, EdgeId, NodeType, EdgeType};

/// 记忆图 - 管理记忆项之间的关系
pub struct MemoryGraph {
    graph: Graph,
    // 索引
    item_nodes: DashMap<String, NodeId>,
    category_nodes: DashMap<String, NodeId>,
    resource_nodes: DashMap<String, NodeId>,
}

impl MemoryGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            item_nodes: DashMap::new(),
            category_nodes: DashMap::new(),
            resource_nodes: DashMap::new(),
        }
    }

    /// 添加记忆项节点
    pub fn add_memory_item(&self, item: &MemoryItem) -> Result<NodeId> {
        let mut node = Node::new(NodeType::Custom("MemoryItem"), &item.id);
        node.set_attribute("memory_type", item.memory_type.to_string());
        node.set_attribute("summary", item.summary.clone());
        if let Some(ref emb) = item.embedding {
            node.set_attribute("embedding_dim", emb.len() as i64);
        }

        let id = self.graph.add_node(node)?;
        self.item_nodes.insert(item.id.clone(), id);
        Ok(id)
    }

    /// 添加类别节点
    pub fn add_category(&self, category: &MemoryCategory) -> Result<NodeId> {
        let mut node = Node::new(NodeType::Custom("Category"), &category.id);
        node.set_attribute("name", category.name.clone());
        node.set_attribute("description", category.description.clone());

        let id = self.graph.add_node(node)?;
        self.category_nodes.insert(category.id.clone(), id);
        Ok(id)
    }

    /// 创建记忆-类别关联
    pub fn link_item_to_category(&self, item_id: &str, category_id: &str) -> Result<EdgeId> {
        let item_node = self.item_nodes.get(item_id)
            .ok_or_else(|| Error::NotFound(format!("Item not found: {}", item_id)))?;
        let cat_node = self.category_nodes.get(category_id)
            .ok_or_else(|| Error::NotFound(format!("Category not found: {}", category_id)))?;

        let edge = Edge::new(*item_node.value(), *cat_node.value(), EdgeType::Custom("BELONGS_TO"));
        self.graph.add_edge(edge)
    }

    /// 创建记忆-资源关联
    pub fn link_item_to_resource(&self, item_id: &str, resource_id: &str) -> Result<EdgeId> {
        let item_node = self.item_nodes.get(item_id)
            .ok_or_else(|| Error::NotFound(format!("Item not found: {}", item_id)))?;
        let res_node = self.resource_nodes.get(resource_id)
            .ok_or_else(|| Error::NotFound(format!("Resource not found: {}", resource_id)))?;

        let edge = Edge::new(*item_node.value(), *res_node.value(), EdgeType::Custom("DERIVED_FROM"));
        self.graph.add_edge(edge)
    }

    /// 创建交叉引用
    pub fn create_cross_reference(&self, from_item: &str, to_item: &str) -> Result<EdgeId> {
        let from_node = self.item_nodes.get(from_item)
            .ok_or_else(|| Error::NotFound(format!("Item not found: {}", from_item)))?;
        let to_node = self.item_nodes.get(to_item)
            .ok_or_else(|| Error::NotFound(format!("Item not found: {}", to_item)))?;

        let edge = Edge::new(*from_node.value(), *to_node.value(), EdgeType::Custom("REFERENCES"));
        self.graph.add_edge(edge)
    }

    /// 获取类别下的所有记忆项
    pub fn get_category_items(&self, category_id: &str) -> Result<Vec<String>> {
        let cat_node = self.category_nodes.get(category_id)
            .ok_or_else(|| Error::NotFound(format!("Category not found: {}", category_id)))?;

        let incoming = self.graph.incoming_edges(cat_node.value())?;
        let mut items = Vec::new();
        for edge in incoming {
            if edge.edge_type == EdgeType::Custom("BELONGS_TO") {
                // 查找源节点
                if let Some(node) = self.find_node_by_id(&edge.source) {
                    if node.node_type == NodeType::Custom("MemoryItem") {
                        items.push(node.name.clone());
                    }
                }
            }
        }
        Ok(items)
    }

    /// 获取记忆项的所有引用
    pub fn get_item_references(&self, item_id: &str) -> Result<Vec<String>> {
        let item_node = self.item_nodes.get(item_id)
            .ok_or_else(|| Error::NotFound(format!("Item not found: {}", item_id)))?;

        let outgoing = self.graph.outgoing_edges(item_node.value())?;
        let mut refs = Vec::new();
        for edge in outgoing {
            if edge.edge_type == EdgeType::Custom("REFERENCES") {
                if let Some(node) = self.find_node_by_id(&edge.target) {
                    refs.push(node.name.clone());
                }
            }
        }
        Ok(refs)
    }

    /// 相似记忆查询（基于图结构）
    pub fn find_similar_items(&self, item_id: &str, depth: usize) -> Result<Vec<String>> {
        let start_node = self.item_nodes.get(item_id)
            .ok_or_else(|| Error::NotFound(format!("Item not found: {}", item_id)))?;

        // BFS 遍历找相似记忆
        let engine = GraphEngine::new();
        let visited = engine.bfs(*start_node.value())?;

        let mut similar = Vec::new();
        for node_id in visited.iter().take(depth) {
            if let Some(node) = self.graph.get_node(node_id)? {
                if node.node_type == NodeType::Custom("MemoryItem") && node.name != item_id {
                    similar.push(node.name.clone());
                }
            }
        }
        Ok(similar)
    }
}
```

### 3.4 EmbeddingManager 嵌入管理

```rust
// crates/evif-mem/src/embedding.rs

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 嵌入客户端 trait
#[async_trait]
pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
}

/// OpenAI 嵌入客户端
pub struct OpenAIEmbeddingClient {
    api_key: String,
    model: String,
    dimension: usize,
    http_client: reqwest::Client,
}

#[async_trait]
impl EmbeddingClient for OpenAIEmbeddingClient {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let response = self.http_client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "input": texts,
            }))
            .send()
            .await?;

        let data: Value = response.json().await?;
        // 解析响应并返回向量
        todo!()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// 嵌入管理器（带缓存）
pub struct EmbeddingManager {
    client: Arc<dyn EmbeddingClient>,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl EmbeddingManager {
    pub fn new(client: Arc<dyn EmbeddingClient>, cache_size: usize) -> Self {
        let cache = LruCache::new(NonZeroUsize::new(cache_size).unwrap());
        Self {
            client,
            cache: Mutex::new(cache),
        }
    }

    /// 计算内容哈希
    fn compute_hash(text: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// 获取嵌入（带缓存）
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let hash = Self::compute_hash(text);

        // 检查缓存
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&hash) {
                return Ok(cached.clone());
            }
        }

        // 调用 API
        let embeddings = self.client.embed(&[text.to_string()]).await?;
        let embedding = embeddings.into_iter().next()
            .ok_or_else(|| Error::EmbeddingFailed("No embedding returned".to_string()))?;

        // 更新缓存
        {
            let mut cache = self.cache.lock().await;
            cache.put(hash, embedding.clone());
        }

        Ok(embedding)
    }

    /// 批量获取嵌入
    pub async fn get_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // 分离已缓存和未缓存的
        let mut results = vec![None; texts.len()];
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        {
            let mut cache = self.cache.lock().await;
            for (i, text) in texts.iter().enumerate() {
                let hash = Self::compute_hash(text);
                if let Some(cached) = cache.get(&hash) {
                    results[i] = Some(cached.clone());
                } else {
                    uncached_indices.push(i);
                    uncached_texts.push(text.clone());
                }
            }
        }

        // 批量获取未缓存的嵌入
        if !uncached_texts.is_empty() {
            let embeddings = self.client.embed(&uncached_texts).await?;

            {
                let mut cache = self.cache.lock().await;
                for (idx, (text, embedding)) in uncached_texts.iter().zip(embeddings.iter()).enumerate() {
                    let original_idx = uncached_indices[idx];
                    results[original_idx] = Some(embedding.clone());

                    let hash = Self::compute_hash(text);
                    cache.put(hash, embedding.clone());
                }
            }
        }

        // 收集结果
        results.into_iter()
            .map(|opt| opt.ok_or_else(|| Error::EmbeddingFailed("Missing embedding".to_string())))
            .collect()
    }
}
```

---

## 四、目录结构设计

### 4.1 文件系统路径映射

```
/mem/                           # 记忆根目录
├── {user_id}/                  # 用户作用域
│   ├── resources/              # 原始资源
│   │   ├── {resource_id}.json  # 资源元数据
│   │   └── {resource_id}.bin   # 资源内容（可选）
│   ├── items/                  # 记忆项
│   │   ├── {item_id}.json      # 记忆项数据
│   │   └── by-type/            # 按类型索引
│   │       ├── profile/        # 个人资料
│   │       ├── event/          # 事件记忆
│   │       ├── knowledge/      # 知识记忆
│   │       ├── behavior/       # 行为记忆
│   │       ├── skill/          # 技能记忆
│   │       └── tool/           # 工具记忆
│   ├── categories/             # 类别
│   │   ├── {category_name}/    # 类别目录
│   │   │   ├── meta.json       # 类别元数据
│   │   │   └── items/          # 类别下的记忆项链接
│   │   │       └── {item_id}.link
│   │   └── ...
│   ├── graph/                  # 图数据
│   │   ├── nodes.bin           # 节点数据
│   │   ├── edges.bin           # 边数据
│   │   └── index.bin           # 索引数据
│   ├── .search                 # 搜索入口（虚拟文件）
│   └── .memorize               # 记忆化入口（虚拟文件）
├── _system/                    # 系统目录
│   ├── config.json             # 配置
│   ├── stats.json              # 统计信息
│   └── embeddings/             # 嵌入向量缓存
└── README                      # 帮助文档
```

### 4.2 Crate 结构

```
crates/
├── evif-mem/                   # 记忆平台核心
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # 库入口
│       ├── mem_plugin.rs       # EVIF 插件实现
│       ├── memory_graph.rs     # 记忆图
│       ├── models.rs           # 数据模型
│       ├── embedding.rs        # 嵌入管理
│       ├── llm/                # LLM 客户端
│       │   ├── mod.rs
│       │   ├── client.rs       # trait 定义
│       │   ├── openai.rs
│       │   ├── anthropic.rs
│       │   └── openrouter.rs
│       ├── pipeline/           # 处理管道
│       │   ├── mod.rs
│       │   ├── memorize.rs     # 记忆化管道
│       │   ├── retrieve.rs     # 检索管道
│       │   └── steps/          # 管道步骤
│       │       ├── ingest.rs
│       │       ├── preprocess.rs
│       │       ├── extract.rs
│       │       ├── categorize.rs
│       │       └── persist.rs
│       ├── storage/            # 存储适配器
│       │   ├── mod.rs
│       │   ├── memory.rs       # 内存存储
│       │   ├── sqlite.rs       # SQLite 存储
│       │   ├── postgres.rs     # PostgreSQL 存储
│       │   └── vector.rs       # 向量存储
│       ├── proactive/          # 主动记忆
│       │   ├── mod.rs
│       │   ├── monitor.rs      # 监控器
│       │   ├── predictor.rs    # 意图预测
│       │   └── suggester.rs    # 建议生成
│       └── prompts/            # Prompt 模板
│           ├── mod.rs
│           ├── memory_type.rs
│           ├── category.rs
│           └── preprocess.rs
└── evif-mem-server/            # REST API 服务器
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── handlers/
        │   ├── memorize.rs
        │   ├── retrieve.rs
        │   └── categories.rs
        ├── routes.rs
        └── websocket.rs
```

---

## 五、实现计划

### 阶段一：基础框架（1-2 周）

#### 任务 1.1：创建 evif-mem crate
- [ ] 初始化 Cargo.toml（依赖 evif-core, evif-graph, evif-storage）
- [ ] 定义数据模型（Resource, MemoryItem, MemoryCategory, CategoryItem）
- [ ] 实现基础错误处理
- [ ] 编写单元测试框架

#### 任务 1.2：实现 MemPlugin 骨架
- [ ] 实现 EvifPlugin trait 基本方法
- [ ] 实现路径解析逻辑（MemPath enum）
- [ ] 实现基础 CRUD 操作（create, read, write, readdir, remove）
- [ ] 添加单元测试

#### 任务 1.3：集成 EVIF 图引擎
- [ ] 封装 MemoryGraph
- [ ] 实现节点操作（add_memory_item, add_category）
- [ ] 实现边操作（link_item_to_category, create_cross_reference）
- [ ] 实现基础查询（get_category_items, get_item_references）

### 阶段二：嵌入和 LLM（2 周）

#### 任务 2.1：嵌入管理
- [ ] 定义 EmbeddingClient trait
- [ ] 实现 OpenAIEmbeddingClient
- [ ] 实现 EmbeddingManager（带 LRU 缓存）
- [ ] 添加批量嵌入支持

#### 任务 2.2：LLM 客户端
- [ ] 定义 LLMClient trait
- [ ] 实现 OpenAI 客户端
- [ ] 实现 Anthropic 客户端
- [ ] 实现 OpenRouter 客户端
- [ ] 添加重试和错误处理

#### 任务 2.3：Prompt 模板
- [ ] 实现 MemoryType 提取 prompt（6种类型）
- [ ] 实现类别摘要 prompt
- [ ] 实现预处理 prompt（对话、图像、文档等）
- [ ] 实现 LLM 检索 prompt

### 阶段三：记忆管道（2-3 周）

#### 任务 3.1：记忆化管道
- [ ] 实现管道框架（Pipeline, Step, Context）
- [ ] 实现 ingest_resource 步骤
- [ ] 实现 preprocess_multimodal 步骤
- [ ] 实现 extract_items 步骤（LLM 提取）
- [ ] 实现 categorize_items 步骤
- [ ] 实现 persist 步骤

#### 任务 3.2：检索管道
- [ ] 实现 RAG 检索
- [ ] 实现 LLM 深度检索
- [ ] 实现混合检索
- [ ] 实现向量相似性搜索

#### 任务 3.3：去重和强化
- [ ] 实现内容哈希去重
- [ ] 实现记忆强化机制
- [ ] 实现引用追踪

### 阶段四：存储后端（1-2 周）

#### 任务 4.1：内存存储
- [ ] 实现基于 DashMap 的内存存储
- [ ] 实现向量索引（简化版：线性搜索）
- [ ] 添加并发安全测试

#### 任务 4.2：SQLite 存储
- [ ] 设计数据库 schema
- [ ] 实现 SQLite 存储后端
- [ ] 集成 sqlite-vec 向量扩展
- [ ] 添加迁移脚本

#### 任务 4.3：持久化和恢复
- [ ] 实现快照保存
- [ ] 实现从快照恢复
- [ ] 实现增量持久化

### 阶段五：API 和集成（1-2 周）

#### 任务 5.1：REST API
- [ ] 实现 `/api/v1/memorize` 端点
- [ ] 实现 `/api/v1/retrieve` 端点
- [ ] 实现 `/api/v1/categories` 端点
- [ ] 实现 `/api/v1/items` CRUD 端点
- [ ] 添加 OpenAPI 文档

#### 任务 5.2：CLI 工具
- [ ] 扩展 evif-cli 支持记忆操作
- [ ] 实现 `evif mem memorize` 命令
- [ ] 实现 `evif mem retrieve` 命令
- [ ] 实现 `evif mem list` 命令

#### 任务 5.3：MCP 集成
- [ ] 实现 evif-mcp 记忆协议
- [ ] 支持 Claude Desktop 集成
- [ ] 添加工具定义

### 阶段六：主动记忆（2-3 周）

#### 任务 6.1：监控器
- [ ] 实现输入/输出监控接口
- [ ] 实现对话跟踪
- [ ] 实现事件队列

#### 任务 6.2：意图预测
- [ ] 实现意图识别
- [ ] 实现下一步预测
- [ ] 实现上下文预加载

#### 任务 6.3：建议生成
- [ ] 实现主动建议引擎
- [ ] 实现推荐排序
- [ ] 实现通知机制

---

## 六、API 设计

### 6.1 文件系统 API

```bash
# 记忆化
echo '{"resource_url": "file:///path/to/conversation.json", "modality": "conversation"}' \
  | evif write /mem/user123/.memorize

# 检索
echo '{"query": "用户的编程偏好", "method": "rag", "limit": 10}' \
  | evif write /mem/user123/.search
evif cat /mem/user123/.search

# 列出记忆项
evif ls /mem/user123/items

# 按类型列出
evif ls /mem/user123/items/by-type/profile

# 列出类别
evif ls /mem/user123/categories

# 获取类别下的记忆
evif ls /mem/user123/categories/编程偏好/items
```

### 6.2 REST API

```yaml
# 记忆化
POST /api/v1/mem/{user_id}/memorize
Request:
  resource_url: string
  modality: "conversation" | "document" | "image" | "video" | "audio"
Response:
  resource: Resource
  items: MemoryItem[]
  categories: MemoryCategory[]

# 检索
POST /api/v1/mem/{user_id}/retrieve
Request:
  query: string
  method: "rag" | "llm" | "hybrid"
  limit: number
  where?: object
Response:
  categories: MemoryCategory[]
  items: MemoryItem[]
  resources: Resource[]
  next_step_query?: string

# CRUD 操作
GET    /api/v1/mem/{user_id}/items
GET    /api/v1/mem/{user_id}/items/{item_id}
POST   /api/v1/mem/{user_id}/items
PATCH  /api/v1/mem/{user_id}/items/{item_id}
DELETE /api/v1/mem/{user_id}/items/{item_id}

GET    /api/v1/mem/{user_id}/categories
GET    /api/v1/mem/{user_id}/categories/{name}
```

### 6.3 MCP 工具

```json
{
  "tools": [
    {
      "name": "memorize",
      "description": "将资源记忆化，提取结构化记忆",
      "inputSchema": {
        "type": "object",
        "properties": {
          "user_id": { "type": "string" },
          "resource_url": { "type": "string" },
          "modality": { "type": "string", "enum": ["conversation", "document", "image", "video", "audio"] }
        },
        "required": ["user_id", "resource_url", "modality"]
      }
    },
    {
      "name": "retrieve",
      "description": "检索相关记忆",
      "inputSchema": {
        "type": "object",
        "properties": {
          "user_id": { "type": "string" },
          "query": { "type": "string" },
          "method": { "type": "string", "enum": ["rag", "llm", "hybrid"], "default": "rag" },
          "limit": { "type": "number", "default": 10 }
        },
        "required": ["user_id", "query"]
      }
    }
  ]
}
```

---

## 七、关键技术决策

### 7.1 嵌入向量存储
- **决策**：使用 EVIF 现有的存储后端 + 向量索引
- **理由**：复用 EVIF 基础设施，保持一致性
- **实现**：内存存储用 DashMap + 线性搜索；SQLite 用 sqlite-vec
- **备选**：独立使用 Milvus/Qdrant（更复杂但性能更好）

### 7.2 LLM 调用策略
- **决策**：支持多种 LLM 提供商，默认 OpenAI
- **理由**：灵活性，用户可根据成本/性能选择
- **优化**：
  - 实现请求缓存（相同 prompt 不重复调用）
  - 实现批量调用（减少 API 请求次数）
  - 实现重试机制（处理临时故障）

### 7.3 图存储方案
- **决策**：基于 EVIF Graph + 存储后端持久化
- **理由**：保持架构一致性，利用现有图引擎
- **优化**：添加向量索引加速相似性查询
- **边类型**：
  - `BELONGS_TO`: 记忆项 → 类别
  - `DERIVED_FROM`: 记忆项 → 资源
  - `REFERENCES`: 记忆项 → 记忆项（交叉引用）
  - `SIMILAR_TO`: 相似记忆关联

### 7.4 主动记忆实现
- **决策**：后台任务 + 事件驱动
- **理由**：与 EVIF 的异步架构一致
- **实现**：
  - 使用 tokio 任务和通道
  - 监控器订阅输入/输出事件
  - 预测器分析模式并生成建议
  - 建议器通过 WebSocket 或 MCP 推送

---

## 八、测试策略

### 8.1 单元测试
- 数据模型序列化/反序列化
- 路径解析逻辑
- 图操作（节点、边、遍历）
- 嵌入缓存

### 8.2 集成测试
- 完整记忆化流程
- 检索流程（RAG/LLM/混合）
- 持久化和恢复
- 并发访问

### 8.3 性能测试
- 大规模记忆项（10万+）
- 并发读写
- 向量搜索延迟
- LLM 调用吞吐量

---

## 九、风险和缓解

### 9.1 技术风险
- **风险**：Rust 学习曲线陡峭
- **缓解**：参考现有 EVIF 插件实现，逐步迭代

### 9.2 性能风险
- **风险**：向量检索性能不足
- **缓解**：使用 HNSW 索引，支持近似搜索

### 9.3 集成风险
- **风险**：与 EVIF 其他组件冲突
- **缓解**：作为独立插件，通过标准接口交互

### 9.4 LLM 成本风险
- **风险**：大量 LLM 调用导致成本高
- **缓解**：实现缓存、批量处理、使用更便宜的模型

---

## 十、Prompt 模板设计

本节详细说明记忆提取和检索过程中使用的 Prompt 模板，参考 memU 的设计并针对 EVIF 架构进行适配。

### 10.1 记忆类型提取 Prompt

基于 memU 的实践，记忆提取使用 6 种记忆类型，每种类型有专门的 prompt：

```rust
// crates/evif-mem/src/prompts/memory_type.rs

/// 记忆类型枚举
pub enum MemoryType {
    Profile,    // 用户画像 - 基本信息、偏好、习惯
    Event,      // 事件记忆 - 重要事件、经历
    Knowledge,  // 知识记忆 - 学习到的知识、概念
    Behavior,   // 行为记忆 - 行为模式、习惯
    Skill,      // 技能记忆 - 技能、能力
    Tool,       // 工具记忆 - 工具使用经验
}

/// Profile Memory 提取 Prompt（用户画像）
const PROFILE_EXTRACT_PROMPT: &str = r#"
# Task Objective
You are a professional User Memory Extractor. Your core task is to extract independent user memory items about the user (e.g., basic info, preferences, habits, other long-term stable traits).

# Workflow
1. Read the full conversation to understand topics and meanings.
2. Extract memory items that contain valuable User Information.
3. Review & validate: Merge similar items, resolve contradictions.
4. Output User Information in JSON format.

# Rules
- Use "user" to refer to the user consistently.
- Each memory item must be complete and self-contained (< 30 words).
- Similar items must be merged into one.
- Extract only information confirmed by the user, not assistant suggestions.

# Memory Categories
{categories_str}

# Response Format (JSON)
{{
    "memories_items": [
        {{
            "content": "the memory item content",
            "categories": ["category1", "category2"]
        }}
    ]
}}

# Resource Content
{resource}
"#;

/// Event Memory 提取 Prompt
const EVENT_EXTRACT_PROMPT: &str = r#"
# Task Objective
Extract significant events and experiences from the conversation that the user shared.

# Rules
- Focus on memorable events: trips, achievements, milestones, conversations
- Include temporal context: when, where, who
- Each event should be a standalone memory item

# Categories
{categories_str}

# Response Format
{{
    "memories_items": [
        {{
            "content": "event description",
            "categories": [],
            "happened_at": "optional timestamp"
        }}
    ]
}}

{resource}
"#;
```

### 10.2 预处理 Prompt

用于不同模态资源的预处理：

```rust
// crates/evif-mem/src/prompts/preprocess.rs

/// 对话预处理 Prompt
const CONVERSATION_PREPROCESS_PROMPT: &str = r#"
Analyze this conversation and extract key information:

1. Topic segments: Divide into topic-based segments
2. Key turns: Identify important user messages
3. Summary: Provide a brief summary

Conversation:
{conversation}

Output format:
{{
    "segments": [
        {{ "start": 0, "end": 10, "topic": "topic name", "summary": "..." }}
    ],
    "key_turns": [5, 12, 20],
    "overall_summary": "..."
}}
"#;

/// 文档预处理 Prompt
const DOCUMENT_PREPROCESS_PROMPT: &str = r#"
Extract structured information from this document:

1. Main topics: List main topics covered
2. Key points: Extract important facts
3. Structure: Understand document structure

Document:
{document}

Output:
{{
    "topics": ["topic1", "topic2"],
    "key_points": ["point1", "point2"],
    "structure": "informative/narrativetechnical"
}}
"#;
```

### 10.3 类别摘要 Prompt

用于动态生成/更新类别摘要：

```rust
// crates/evif-mem/src/prompts/category.rs

const CATEGORY_SUMMARY_PROMPT: &str = r#"
# Task
Generate or update a summary for a memory category based on its items.

# Category Name
{category_name}

# Existing Items
{items}

# Existing Summary
{existing_summary}

# Instructions
1. Review existing items and summary
2. Update summary to reflect new items
3. Maintain [ref:item_id] references for specific memories
4. Keep summary concise but informative (max 200 words)

# Output
{{
    "summary": "updated summary with [ref:xxx] references",
    "key_themes": ["theme1", "theme2"],
    "item_count": N
}}
"#;
```

### 10.4 检索 Prompt

用于 LLM 模式检索：

```rust
// crates/evif-mem/src/prompts/retrieve.rs

/// 查询重写 Prompt
const QUERY_REWRITE_PROMPT: &str = r#"
Rewrite this query to be more effective for memory retrieval:

Original query: {query}

Context (previous queries):
{context}

Instructions:
1. Expand abbreviations
2. Add relevant synonyms
3. Consider user intent

Output:
{{
    "rewritten_query": "...",
    "intent": "informational/recall/fact"
}}
"#;

/// 结果排序 Prompt
const RESULT_RANKING_PROMPT: &str = r#"
Rank these memory items by relevance to the query:

Query: {query}

Items:
{items}

Output:
{{
    "ranked_items": [
        {{ "id": "item_id", "score": 0.95, "reason": "..." }}
    ]
}}
"#;
```

### 10.5 Prompt 管理器

```rust
// crates/evif-mem/src/prompts/mod.rs

use std::collections::HashMap;

pub struct PromptManager {
    templates: HashMap<String, String>,
}

impl PromptManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Memory type prompts
        templates.insert("profile_extract", PROFILE_EXTRACT_PROMPT);
        templates.insert("event_extract", EVENT_EXTRACT_PROMPT);
        templates.insert("knowledge_extract", KNOWLEDGE_EXTRACT_PROMPT);
        templates.insert("behavior_extract", BEHAVIOR_EXTRACT_PROMPT);
        templates.insert("skill_extract", SKILL_EXTRACT_PROMPT);
        templates.insert("tool_extract", TOOL_EXTRACT_PROMPT);

        // Preprocess prompts
        templates.insert("conversation_preprocess", CONVERSATION_PREPROCESS_PROMPT);
        templates.insert("document_preprocess", DOCUMENT_PREPROCESS_PROMPT);

        // Category prompts
        templates.insert("category_summary", CATEGORY_SUMMARY_PROMPT);

        // Retrieve prompts
        templates.insert("query_rewrite", QUERY_REWRITE_PROMPT);
        templates.insert("result_ranking", RESULT_RANKING_PROMPT);

        Self { templates }
    }

    pub fn render(&self, name: &str, variables: &HashMap<String, String>) -> String {
        let template = self.templates.get(name)
            .expect(&format!("Prompt template not found: {}", name));

        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}
```

---

## 十、下一步行动

1. **立即开始**：创建 `crates/evif-mem` 目录结构
2. **第一周**：实现基础 MemPlugin 和数据模型
3. **第二周**：集成嵌入管理和 LLM 客户端
4. **第三周**：实现 Memorize Pipeline
5. **第四周**：实现 Retrieve Pipeline
6. **持续**：添加测试和文档

---

## 十一、参考资源

### 11.1 memU 资源
- GitHub: https://github.com/NevaMind-AI/memU
- 文档: https://memu.pro/docs
- API: https://api.memu.so

### 11.2 EVIF 资源
- 本地路径: `/Users/louloulin/Documents/linchong/claude/evif`
- README: `evif/README-CN.md`
- 插件示例: `evif/crates/evif-plugins/src/`
- 图引擎: `evif/crates/evif-graph/`

### 11.3 相关技术
- OpenAI API: https://platform.openai.com/docs
- pgvector: https://github.com/pgvector/pgvector
- sqlite-vec: https://github.com/asg017/sqlite-vec
- HNSW 索引: https://github.com/nmslib/hnswlib

---

*文档版本: 2.0*
*创建日期: 2026-03-06*
*更新日期: 2026-03-06*
*作者: Claude (基于 memU 和 EVIF 深度分析)*
