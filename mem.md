# Mem 平台综合设计计划
**基于 EVIF 构建的 AI 原生记忆系统**

> 版本: 1.0
> 日期: 2026-03-07
> 状态: 实现中 (Phase 1: 约 85% 完成)
> 进度: ✅ Categorizer 已集成到 MemorizePipeline

---

## 📋 执行摘要

...

---

## 📋 执行摘要

Mem 是一个构建在 EVIF 虚拟文件系统之上的 AI 原生记忆平台，融合了 memU 的三层记忆架构和 EVIF 的 Plan 9 "万物皆文件" 哲学。核心设计理念：

- **文件即记忆**：使用 MD + YAML Frontmatter 格式存储，AI 友好、Git 友好、FUSE 友好
- **三层架构**：Resource（原始数据）→ MemoryItem（提取的记忆）→ MemoryCategory（自动分类）
- **双模式检索**：向量相似度（RAG）+ LLM 推理（直接读取 MD 文件）
- **时序知识图谱**：基于 evif-graph 的时序关系查询和因果推理
- **主动演化**：24/7 监控、意图预测、自演化知识库

---

## 🏗️ 整体架构设计

### 架构层次图

```
┌─────────────────────────────────────────────────────────────────┐
│                      应用层 (Application Layer)                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ Mem API  │  │  CLI     │  │  FUSE    │  │  MCP     │       │
│  │ (REST)   │  │  Tools   │  │  Mount   │  │  Server  │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
├───────┴────────────┴─────────┴─────────┴─────────┴──────────────┤
│                    记忆处理层 (Memory Processing Layer)          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ Memorize     │  │ Retrieve     │  │  Evolve      │         │
│  │ Pipeline     │  │ Pipeline     │  │  Pipeline    │         │
│  │              │  │              │  │              │         │
│  │ • Ingest     │  │ • Vector     │  │ • Monitor    │         │
│  │ • Preprocess │  │   Search     │  │ • Predict    │         │
│  │ • Extract    │  │ • LLM Read   │  │ • Self-Update│         │
│  │ • Dedupe     │  │ • Graph      │  │ • Prune      │         │
│  │ • Categorize │  │   Query      │  │ • Merge      │         │
│  │ • Persist    │  │ • Rerank     │  │ • Reinforce  │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
├─────────────────────────────────────────────────────────────────┤
│                    核心引擎层 (Core Engine Layer)                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ Vector Index │  │ Temporal     │  │ Embedding    │         │
│  │              │  │ Graph        │  │ Manager      │         │
│  │ • Cosine     │  │              │  │              │         │
│  │ • Euclidean  │  │ • Timeline   │  │ • LRU Cache  │         │
│  │ • Dot Prod   │  │ • Causal     │  │ • OpenAI     │         │
│  │ • FAISS*     │  │ • Temporal   │  │ • Local      │         │
│  │ • Qdrant*    │  │   BFS        │  │   Models     │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
├─────────────────────────────────────────────────────────────────┤
│                    存储抽象层 (Storage Abstraction Layer)        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              EVIF Plugin: MemPlugin                       │  │
│  │  • MD + YAML Frontmatter format                          │  │
│  │  • Auto-parse on write, auto-serialize on read           │  │
│  │  • Symlink cross-references                              │  │
│  │  • Directory structure: /{type}/{item}.md                │  │
│  └──────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                    底层存储层 (Storage Backend Layer)            │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐      │
│  │Memory  │ │ Sled   │ │RocksDB │ │ SQLite │ │  S3*   │      │
│  │(DashMap)│ │(KV)    │ │(KV)    │ │ (SQL)  │ │(Cloud) │      │
│  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘      │
└─────────────────────────────────────────────────────────────────┘

* 标记为未来扩展
```

### 数据流架构

```
用户输入 (Conversation/Document/Image)
    ↓
┌─────────────────────────────────────────────────┐
│ 1. Resource Layer (原始数据层)                   │
│    • 存储: /resource/{id}.md                    │
│    • 内容: 原始内容 + 元数据                     │
│    • 模态: text, image, audio, video, code      │
└─────────────────────────────────────────────────┘
    ↓ (LLM Extract)
┌─────────────────────────────────────────────────┐
│ 2. MemoryItem Layer (记忆项层)                  │
│    • 存储: /{type}/{id}.md                      │
│    • 类型: profile, event, knowledge,           │
│           behavior, skill, tool                 │
│    • 内容: 事实、偏好、技能提取                  │
│    • 去重: Hash-based deduplication             │
└─────────────────────────────────────────────────┘
    ↓ (Auto Categorize)
┌─────────────────────────────────────────────────┐
│ 3. MemoryCategory Layer (分类层)                │
│    • 存储: /category/{name}.md                  │
│    • 自动: 基于向量相似度的自动分类              │
│    • 关系: CategoryItem (多对多)                │
│    • 符号链接: /category/{name}/{item}.md       │
│      → /{type}/{id}.md                          │
└─────────────────────────────────────────────────┘
    ↓ (Temporal Graph)
┌─────────────────────────────────────────────────┐
│ 4. Temporal Knowledge Graph (时序图谱)          │
│    • 节点: Resource, MemoryItem, Category       │
│    • 时序边: Before, After, Simultaneous,       │
│              Causes                             │
│    • 记忆边: BelongsTo, DerivedFrom,            │
│              References, SimilarTo              │
│    • 查询: temporal_bfs, find_causal_chain,     │
│            get_event_timeline                   │
└─────────────────────────────────────────────────┘
```

---

## 🎯 核心设计决策

### 1. MD 格式 vs JSON 格式

**选择**: MD + YAML Frontmatter

**理由**:
- ✅ **AI 友好**: LLM 直接读取 MD 文件进行推理（Mode 2 检索）
- ✅ **Git 友好**: 版本控制清晰，diff 可读
- ✅ **FUSE 友好**: 符合文件系统语义，可 mount 到本地
- ✅ **人类可读**: 开发调试方便
- ❌ JSON: 需要额外解析，不利于 LLM 直接读取

**格式示例**:
```markdown
---
id: 550e8400-e29b-41d4-a716-446655440000
type: knowledge
created: 2026-03-07T00:00:00Z
updated: 2026-03-07T01:00:00Z
tags: [rust, async, plugin]
references:
  - 660e8400-e29b-41d4-a716-446655440001
embedding_hash: abc123def456
---

# Rust 异步插件系统

EVIF 使用 `async_trait` 实现异步插件接口，支持 `async fn` 方法...

## 关键点

1. 所有插件方法都是 async
2. 使用 `async_trait` 宏
3. 支持 Tokio runtime
```

### 2. 三层记忆架构

**来源**: memU 核心设计

**层次**:
1. **Resource Layer**: 原始数据（对话、文档、图片）
   - 模态: text, image, audio, video, code
   - 存储: `/resource/{id}.md`
   - 追溯: 可追溯到原始来源

2. **MemoryItem Layer**: 提取的记忆项
   - 类型: profile（偏好）, event（事件）, knowledge（知识）, behavior（行为）, skill（技能）, tool（工具）
   - 存储: `/{type}/{id}.md`
   - 去重: 基于 content hash

3. **MemoryCategory Layer**: 自动分类
   - 组织: 基于向量相似度的自动聚类
   - 存储: `/category/{name}.md` + 符号链接
   - 关系: 多对多（一个 MemoryItem 可属于多个 Category）

**追溯性**: 完整的双向追溯
- MemoryItem → Resource（来源）
- Category → MemoryItems（成员）
- Graph Edge → Nodes（关系）

### 3. 双模式检索

**Mode 1: 向量相似度检索（RAG）**
- 用途: 快速相似度匹配
- 算法: Cosine / Euclidean / Dot Product
- 索引: InMemoryVectorIndex（当前）/ FAISS（未来）/ Qdrant（未来）
- 场景: "找到类似的知识"

**Mode 2: LLM 直接读取（Reasoning）**
- 用途: 深度推理和关联分析
- 方式: LLM 直接读取 MD 文件内容
- 场景: "分析这个记忆类别中的所有知识"
- 优势: 不依赖向量，使用 LLM 的推理能力

**混合检索**:
1. 先用向量检索快速过滤 Top-K
2. 再用 LLM 对 Top-K 进行深度分析
3. 结合图谱查询进行关联推理

### 4. 时序知识图谱

**基于**: evif-graph/temporal.rs

**节点类型**:
- `MemoryItem`: 记忆项节点
- `Category`: 分类节点
- `Resource`: 资源节点
- `Event`: 事件节点
- `Profile`: 档案节点
- `Skill`: 技能节点
- `Tool`: 工具节点

**边类型**:
**时序边**:
- `Before`: A 在 B 之前
- `After`: A 在 B 之后
- `Simultaneous`: A 和 B 同时发生
- `Causes`: A 导致 B

**记忆边**:
- `BelongsTo`: MemoryItem 属于 Category
- `DerivedFrom`: MemoryItem 派生自 Resource
- `References`: MemoryItem 引用另一个 MemoryItem
- `SimilarTo`: MemoryItem 相似于另一个 MemoryItem

**查询能力**:
- `temporal_bfs`: 时间顺序的广度优先遍历
- `find_temporal_path`: 查找时间路径
- `get_event_timeline`: 获取事件时间线
- `find_causal_chain`: 查找因果链
- `find_periodic_patterns`: 发现周期性模式

### 5. 主动演化系统

**来源**: memU 24/7 Proactive Agent

**核心能力**:
1. **背景监控**: 持续监控用户新输入
2. **意图预测**: 预测用户可能的需求
3. **自动提取**: 从对话中自动提取记忆项
4. **自演化知识**: 根据新信息更新旧知识
5. **周期性任务**: 定期清理、合并、强化记忆

**演化机制**:
- **强化 (Reinforce)**: 频繁访问的记忆权重增加
- **衰减 (Decay)**: 长时间未访问的记忆权重降低
- **合并 (Merge)**: 相似记忆合并为更抽象的知识
- **修剪 (Prune)**: 低权重、冗余记忆删除
- **更新 (Update)**: 新信息覆盖旧信息

---

## 🔧 技术实现设计

### 1. 已实现组件

#### 1.1 核心数据模型 (crates/evif-mem/src/models.rs)

```rust
/// 记忆类型
pub enum MemoryType {
    Profile,    // 用户偏好、习惯
    Event,      // 发生的事件
    Knowledge,  // 学到的知识
    Behavior,   // 行为模式
    Skill,      // 掌握的技能
    Tool,       // 使用的工具
}

/// 记忆项
pub struct MemoryItem {
    pub id: Uuid,
    pub memory_type: MemoryType,
    pub content: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub happened_at: Option<DateTime<Utc>>,
    pub references: Vec<Uuid>,
}

/// 记忆分类
pub struct MemoryCategory {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub summary: String,
    pub embedding: Option<Vec<f32>>,
}

/// 资源（原始数据）
pub struct Resource {
    pub id: Uuid,
    pub url: Option<String>,
    pub modality: Modality,  // Text, Image, Audio, Video, Code
    pub local_path: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

/// MD Frontmatter（YAML 元数据）
pub struct MdFrontmatter {
    pub id: Uuid,
    pub memory_type: MemoryType,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub tags: Vec<String>,
    pub references: Vec<Uuid>,
    pub embedding_hash: Option<String>,
}
```

#### 1.2 向量检索模块 (crates/evif-mem/src/vector/)

```rust
/// 向量索引 trait（后端灵活）
pub trait VectorIndex: Send + Sync {
    /// 添加向量
    async fn add(&mut self, id: Uuid, vector: Vec<f32>) -> Result<()>;

    /// 删除向量
    async fn remove(&mut self, id: &Uuid) -> Result<()>;

    /// Top-K 相似度检索
    async fn search(&self, query: &[f32], k: usize) -> Result<Vec<(Uuid, f32)>>;

    /// 阈值过滤检索
    async fn search_with_threshold(
        &self,
        query: &[f32],
        k: usize,
        threshold: f32
    ) -> Result<Vec<(Uuid, f32)>>;
}

/// 内存向量索引（当前实现）
pub struct InMemoryVectorIndex {
    vectors: HashMap<Uuid, Vec<f32>>,
    metric: SimilarityMetric,  // Cosine, Euclidean, DotProduct
}

// 未来扩展
// pub struct FaissIndex { ... }
// pub struct QdrantIndex { ... }
```

**测试覆盖**: 21 tests passing

#### 1.3 MemPlugin EVIF 插件 (crates/evif-mem/src/plugin/)

```rust
/// MemPlugin - EVIF 插件，将记忆平台暴露为文件系统
pub struct MemPlugin {
    root: Arc<RwLock<MemDir>>,
    storage: Arc<dyn MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
}

impl EvifPlugin for MemPlugin {
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;
    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<u64>;
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;
    async fn remove(&self, path: &str) -> EvifResult<()>;
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;
    async fn symlink(&self, target_path: &str, link_path: &str) -> EvifResult<()>;
    async fn readlink(&self, link_path: &str) -> EvifResult<String>;
}
```

**关键特性**:
- ✅ MD 文件格式：YAML frontmatter + Markdown 内容
- ✅ 自动解析：写入 `.md` 文件时自动解析为 `MemoryItem`
- ✅ 目录结构：`/{profile,event,knowledge,behavior,skill,tool}/{item}.md`
- ✅ 符号链接：支持交叉引用
- ✅ 异步安全：修复了 `sync_from_storage` 的借用检查问题

**测试覆盖**: 24 tests passing
**提交**: 9f6944e

#### 1.4 嵌入管理 (crates/evif-mem/src/embedding.rs)

```rust
/// 嵌入管理器（LRU 缓存）
pub struct EmbeddingManager {
    client: Box<dyn EmbeddingClient>,
    cache: LruCache<String, Vec<f32>>,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

/// 嵌入客户端 trait
pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

/// OpenAI 嵌入客户端
pub struct OpenAIEmbeddingClient {
    api_key: String,
    model: String,  // text-embedding-3-small
    client: reqwest::Client,
}
```

**特性**:
- LRU 缓存避免重复计算
- 支持批量嵌入
- OpenAI API 集成
- 未来可扩展本地模型

#### 1.5 LLM 客户端抽象 (crates/evif-mem/src/llm.rs)

```rust
/// LLM 客户端 trait
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// 生成文本
    async fn generate(&self, prompt: &str) -> MemResult<String>;

    /// 提取结构化记忆
    async fn extract_memories(&self, text: &str) -> MemResult<Vec<MemoryItem>>;

    /// 生成嵌入向量
    async fn embed(&self, text: &str) -> MemResult<Vec<f32>>;

    /// 分析记忆类别
    async fn analyze_category(&self, memories: &[String]) -> MemResult<CategoryAnalysis>;

    /// 重排序
    async fn rerank(&self, query: &str, items: Vec<MemoryItem>) -> MemResult<Vec<MemoryItem>>;
}

/// OpenAI 客户端
pub struct OpenAIClient {
    api_key: String,
    model: String,  // gpt-4o
    embedding_model: String,  // text-embedding-3-small
    client: reqwest::Client,
    base_url: String,
}

/// 分类分析结果
pub struct CategoryAnalysis {
    pub name: String,
    pub description: String,
    pub themes: Vec<String>,
    pub tags: Vec<String>,
}
```

**特性**:
- 统一的 LLM 接口抽象
- OpenAI API 完整集成
- 支持自定义配置 (model, embedding_model, base_url)
- 完整的错误处理和类型安全

**测试覆盖**: 23 tests passing
**提交**: 9f6944e

### 2. 待实现组件

#### 2.1 记忆化管道 (Memorize Pipeline)

```rust
/// 记忆化管道
pub struct MemorizePipeline {
    resource_loader: ResourceLoader,
    preprocessor: Preprocessor,
    extractor: Extractor,
    deduplicator: Deduplicator,
    categorizer: Categorizer,
    persister: Persister,
}

impl MemorizePipeline {
    /// 完整记忆化流程
    pub async fn memorize(&self, input: Input) -> Result<Vec<MemoryItem>> {
        // Step 1: 加载资源
        let resource = self.resource_loader.load(input).await?;

        // Step 2: 预处理（多模态转文本）
        let text = self.preprocessor.process(&resource).await?;

        // Step 3: LLM 提取记忆项
        let items = self.extractor.extract(&text).await?;

        // Step 4: 去重
        let unique_items = self.deduplicator.dedupe(items).await?;

        // Step 5: 自动分类
        let categorized = self.categorizer.categorize(&unique_items).await?;

        // Step 6: 持久化 + 索引
        self.persister.persist(&categorized).await?;

        Ok(categorized)
    }
}
```

**实现要点**:
- **ResourceLoader**: 支持多种输入格式（text, file, url）
- **Preprocessor**: 多模态处理（图片用 OCR，音频用 ASR）
- **Extractor**: LLM prompt engineering 提取结构化记忆
- **Deduplicator**: 基于 content hash 的去重
- **Categorizer**: 向量相似度聚类 + LLM 生成分类描述
- **Persister**: 写入 MD 文件 + 更新向量索引 + 更新图谱

#### 2.2 检索管道 (Retrieve Pipeline)

```rust
/// 检索管道
pub struct RetrievePipeline {
    vector_index: Arc<dyn VectorIndex>,
    graph: Arc<TemporalGraph>,
    storage: Arc<dyn MemoryStorage>,
    llm_client: Box<dyn LLMClient>,
}

pub enum RetrieveMode {
    VectorSearch { k: usize, threshold: f32 },
    LLMRead { category: String },
    GraphQuery { query: GraphQuery },
    Hybrid { vector_k: usize, llm_top_n: usize },
}

impl RetrievePipeline {
    /// 检索记忆
    pub async fn retrieve(&self, query: &str, mode: RetrieveMode) -> Result<Vec<MemoryItem>> {
        match mode {
            RetrieveMode::VectorSearch { k, threshold } => {
                // Mode 1: 向量检索
                let query_vec = self.llm_client.embed(query).await?;
                let results = self.vector_index.search_with_threshold(&query_vec, k, threshold).await?;
                let mut items = Vec::new();
                for (id, score) in results {
                    if let Some(item) = self.storage.get(&id).await? {
                        items.push(item);
                    }
                }
                Ok(items)
            }

            RetrieveMode::LLMRead { category } => {
                // Mode 2: LLM 直接读取
                let category_path = format!("/category/{}.md", category);
                let category_content = self.storage.read_file(&category_path).await?;

                // 读取分类下的所有记忆
                let items = self.storage.readdir(&format!("/category/{}", category)).await?;
                let mut memories = Vec::new();
                for item in items {
                    let content = self.storage.read_file(&item.path).await?;
                    memories.push(content);
                }

                // LLM 分析
                let analysis = self.llm_client.analyze(query, &memories).await?;
                // ... 返回相关记忆
                Ok(vec![])
            }

            RetrieveMode::GraphQuery { query } => {
                // Mode 3: 图谱查询
                let results = self.graph.execute(query).await?;
                // ... 转换为 MemoryItem
                Ok(vec![])
            }

            RetrieveMode::Hybrid { vector_k, llm_top_n } => {
                // Mode 4: 混合检索
                // 1. 向量检索 Top-K
                let vector_results = self.retrieve(query, RetrieveMode::VectorSearch {
                    k: vector_k,
                    threshold: 0.7
                }).await?;

                // 2. LLM 深度分析 Top-N
                let top_n = vector_results.into_iter().take(llm_top_n).collect();
                let refined = self.llm_client.rerank(query, top_n).await?;

                Ok(refined)
            }
        }
    }
}
```

#### 2.3 演化管道 (Evolve Pipeline)

```rust
/// 演化管道（24/7 主动代理）
pub struct EvolvePipeline {
    monitor: Monitor,
    predictor: Predictor,
    updater: Updater,
    pruner: Pruner,
    merger: Merger,
}

impl EvolvePipeline {
    /// 背景演化循环
    pub async fn run_evolution_loop(&self) -> Result<()> {
        loop {
            // 1. 监控新输入
            if let Some(new_input) = self.monitor.check_new_input().await? {
                // 2. 预测用户意图
                let intent = self.predictor.predict(&new_input).await?;

                // 3. 自动提取记忆
                let items = self.extract_proactive(&new_input, &intent).await?;

                // 4. 更新知识库
                self.updater.update(items).await?;
            }

            // 5. 周期性任务（每天）
            if self.should_run_daily_tasks() {
                // 衰减低权重记忆
                self.pruner.decay_weights().await?;

                // 合并相似记忆
                self.merger.merge_similar().await?;

                // 删除冗余记忆
                self.pruner.prune_redundant().await?;
            }

            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    /// 主动提取（基于意图预测）
    async fn extract_proactive(&self, input: &Input, intent: &Intent) -> Result<Vec<MemoryItem>> {
        // 实现主动提取逻辑
        Ok(vec![])
    }
}
```

**演化机制实现**:
- **强化 (Reinforce)**: 访问次数增加权重
- **衰减 (Decay)**: `weight *= decay_factor ^ days_since_access`
- **合并 (Merge)**: 向量相似度 > 0.95 的记忆合并
- **修剪 (Prune)**: `weight < threshold` 的记忆删除
- **更新 (Update)**: 新信息覆盖旧信息（版本控制）

#### 2.4 LLM 客户端抽象

```rust
/// LLM 客户端 trait
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// 生成文本
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// 提取结构化记忆
    async fn extract_memories(&self, text: &str) -> Result<Vec<MemoryItem>>;

    /// 生成嵌入向量
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// 分析记忆类别
    async fn analyze_category(&self, memories: &[String]) -> Result<CategoryAnalysis>;

    /// 重排序
    async fn rerank(&self, query: &str, items: Vec<MemoryItem>) -> Result<Vec<MemoryItem>>;
}

/// OpenAI 客户端
pub struct OpenAIClient {
    api_key: String,
    model: String,  // gpt-4o
    client: reqwest::Client,
}

/// Anthropic 客户端
pub struct AnthropicClient {
    api_key: String,
    model: String,  // claude-3-5-sonnet-20241022
    client: reqwest::Client,
}

/// 本地模型客户端（Ollama）
pub struct OllamaClient {
    base_url: String,
    model: String,  // llama3.2
    client: reqwest::Client,
}
```

#### 2.5 多模态预处理

```rust
/// 多模态预处理器
pub struct Preprocessor {
    ocr_client: Option<OCRClient>,
    asr_client: Option<ASRClient>,
}

#[derive(Debug, Clone)]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
    Code,
}

impl Preprocessor {
    /// 预处理（转换为文本）
    pub async fn process(&self, resource: &Resource) -> Result<String> {
        match resource.modality {
            Modality::Text => Ok(resource.content.clone()),
            Modality::Image => {
                // OCR 识别
                if let Some(ocr) = &self.ocr_client {
                    let text = ocr.recognize(&resource.data).await?;
                    Ok(text)
                } else {
                    Err(Error::OCRNotAvailable)
                }
            }
            Modality::Audio => {
                // ASR 语音识别
                if let Some(asr) = &self.asr_client {
                    let text = asr.transcribe(&resource.data).await?;
                    Ok(text)
                } else {
                    Err(Error::ASRNotAvailable)
                }
            }
            Modality::Video => {
                // 提取关键帧 + OCR + ASR
                // 实现视频处理
                Ok(String::new())
            }
            Modality::Code => {
                // 代码直接作为文本
                Ok(resource.content.clone())
            }
        }
    }
}
```

#### 2.6 持久化存储

**当前**: DashMap 内存存储（测试用）
**目标**: 多后端支持

```rust
/// 存储后端 trait
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// CRUD 操作
    async fn get(&self, id: &Uuid) -> Result<Option<MemoryItem>>;
    async fn put(&self, item: &MemoryItem) -> Result<()>;
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// 批量操作
    async fn batch_get(&self, ids: &[Uuid]) -> Result<Vec<MemoryItem>>;
    async fn batch_put(&self, items: &[MemoryItem]) -> Result<()>;

    /// 查询操作
    async fn query_by_type(&self, memory_type: MemoryType) -> Result<Vec<MemoryItem>>;
    async fn query_by_tags(&self, tags: &[String]) -> Result<Vec<MemoryItem>>;
    async fn query_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<MemoryItem>>;
}

/// Sled 存储（嵌入式 KV）
pub struct SledStorage {
    db: sled::Db,
}

/// RocksDB 存储（高性能 KV）
pub struct RocksDBStorage {
    db: rocksdb::DB,
}

/// SQLite 存储（关系型）
pub struct SQLiteStorage {
    conn: rusqlite::Connection,
}
```

**集成 EVIF**:
- MemPlugin 使用 EVIF 的存储抽象
- 可配置后端: `--storage-backend=sled|rocksdb|sqlite`
- 利用 EVIF 的 S3/Azure/GCS 插件实现云端备份

---

## 📊 API 设计

### REST API

#### 记忆操作

```bash
# 创建记忆（自动提取）
POST /api/v1/memories
Content-Type: application/json

{
  "content": "用户提到他喜欢使用 Rust 编写系统工具",
  "modality": "text",
  "metadata": {
    "source": "conversation",
    "timestamp": "2026-03-07T00:00:00Z"
  }
}

Response:
{
  "memory_id": "550e8400-e29b-41d4-a716-446655440000",
  "extracted_items": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "type": "profile",
      "summary": "用户偏好：喜欢使用 Rust",
      "category": "programming_preferences"
    }
  ]
}
```

```bash
# 检索记忆
POST /api/v1/memories/search
Content-Type: application/json

{
  "query": "用户对 Rust 的偏好",
  "mode": "hybrid",
  "vector_k": 10,
  "llm_top_n": 5
}

Response:
{
  "results": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "type": "profile",
      "content": "...",
      "score": 0.95,
      "category": "programming_preferences"
    }
  ],
  "total": 5
}
```

```bash
# 获取记忆
GET /api/v1/memories/{id}

Response:
{
  "id": "660e8400-e29b-41d4-a716-446655440001",
  "type": "profile",
  "content": "用户喜欢使用 Rust 编写系统工具...",
  "tags": ["rust", "programming"],
  "created": "2026-03-07T00:00:00Z",
  "updated": "2026-03-07T01:00:00Z",
  "references": ["770e8400-..."],
  "category_ids": ["880e8400-..."]
}
```

#### 分类操作

```bash
# 获取所有分类
GET /api/v1/categories

Response:
{
  "categories": [
    {
      "id": "880e8400-...",
      "name": "programming_preferences",
      "description": "用户的编程语言偏好",
      "item_count": 15
    }
  ]
}
```

```bash
# 获取分类下的记忆
GET /api/v1/categories/{name}/memories

Response:
{
  "category": {
    "id": "880e8400-...",
    "name": "programming_preferences",
    "description": "..."
  },
  "memories": [
    { "id": "660e8400-...", "summary": "..." },
    ...
  ]
}
```

#### 图谱操作

```bash
# 查询因果链
POST /api/v1/graph/query
Content-Type: application/json

{
  "query_type": "causal_chain",
  "start_node": "660e8400-...",
  "max_depth": 5
}

Response:
{
  "chains": [
    {
      "nodes": ["660e8400-...", "770e8400-...", "880e8400-..."],
      "edges": ["Causes", "Before"],
      "narrative": "用户学习 Rust → 开始使用 Rust 编写工具 → 偏好 Rust"
    }
  ]
}
```

```bash
# 获取时间线
POST /api/v1/graph/query
Content-Type: application/json

{
  "query_type": "timeline",
  "category": "programming_preferences",
  "start_time": "2026-01-01T00:00:00Z",
  "end_time": "2026-03-07T00:00:00Z"
}

Response:
{
  "timeline": [
    {
      "timestamp": "2026-01-15T10:00:00Z",
      "event": "用户首次提到 Rust",
      "node_id": "..."
    },
    {
      "timestamp": "2026-02-20T14:30:00Z",
      "event": "用户使用 Rust 完成项目",
      "node_id": "..."
    }
  ]
}
```

### CLI 命令

```bash
# 记忆管理
evif-mem add --content "用户喜欢 Rust" --type profile
evif-mem search "Rust 偏好" --mode hybrid --k 10
evif-mem get <memory-id>
evif-mem list --type knowledge --tags rust,async
evif-mem update <memory-id> --tags +new-tag
evif-mem delete <memory-id>

# 分类管理
evif-mem categories
evif-mem category <name>
evif-mem category <name> add <memory-id>
evif-mem category <name> remove <memory-id>

# 图谱查询
evif-mem graph causal-chain <memory-id> --depth 5
evif-mem graph timeline --category programming --from 2026-01-01
evif-mem graph similar <memory-id> --threshold 0.8

# 演化控制
evif-mem evolve start
evif-mem evolve stop
evif-mem evolve status
evif-mem evolve run-once --task decay

# 统计信息
evif-mem stats
evif-mem stats --type knowledge
evif-mem stats --category programming
```

### MCP Server 集成

```typescript
// Claude Code MCP Server
{
  "name": "evif-mem",
  "tools": [
    {
      "name": "memorize",
      "description": "Extract and store memories from text",
      "parameters": {
        "content": "string",
        "modality": "text|image|audio|video|code",
        "metadata": "object"
      }
    },
    {
      "name": "retrieve",
      "description": "Search memories with hybrid retrieval",
      "parameters": {
        "query": "string",
        "mode": "vector|llm|graph|hybrid",
        "k": "number"
      }
    },
    {
      "name": "query_graph",
      "description": "Query temporal knowledge graph",
      "parameters": {
        "query_type": "causal_chain|timeline|similar",
        "params": "object"
      }
    },
    {
      "name": "evolve",
      "description": "Trigger memory evolution tasks",
      "parameters": {
        "task": "decay|merge|prune|reinforce"
      }
    }
  ]
}
```

---

## 🗺️ 实施路线图

### Phase 1: 核心管道实现 (Q1 2026)

**目标**: 实现基本的记忆化和检索能力

**任务**:
- [x] 实现 `MemorizePipeline`
  - [x] ResourceLoader（支持 text）
  - [x] Preprocessor（text 模态）
  - [x] Extractor（LLM 提取）
  - [x] Deduplicator（content hash）
  - [x] Categorizer（向量聚类）✅ 新增
  - [x] Persister（MD 文件 + 向量索引）
- [x] 实现 `RetrievePipeline`
  - [x] VectorSearch 模式
  - [x] LLMRead 模式 ✅ 新增
  - [x] Hybrid 模式
- [x] 实现 `LLMClient` 抽象
  - [x] OpenAI 客户端
  - [ ] Anthropic 客户端
- [x] 实现 REST API
  - [x] POST /api/v1/memories
  - [x] POST /api/v1/memories/search
  - [x] GET /api/v1/memories/{id}
  - [x] GET /api/v1/memories
  - [x] GET /api/v1/categories (新增)
  - [x] GET /api/v1/categories/{id} (新增)
  - [x] GET /api/v1/categories/{id}/memories (新增)
  - [x] POST /api/v1/graph/query (Phase 2 提前实现)
- [ ] 编写单元测试和集成测试
- [ ] 编写文档和示例

**交付物**:
- 可工作的记忆化管道
- 双模式检索系统
- REST API 服务器
- CLI 工具
- 测试覆盖率 > 80%

### Phase 2: 时序图谱集成 (Q2 2026)

**目标**: 集成 evif-graph，实现时序关系查询

**任务**:
- [ ] 扩展 evif-graph 节点和边类型
  - [ ] MemoryItem/Category/Resource 节点
  - [ ] BelongsTo/DerivedFrom/References/SimilarTo 边
  - [ ] Before/After/Simultaneous/Causes 时序边
- [ ] 实现图谱持久化
  - [ ] Graph → SQLite/RocksDB 序列化
  - [ ] 图谱加载和恢复
- [ ] 实现图谱查询 API
  - [ ] causal_chain 查询
  - [ ] timeline 查询
  - [ ] similar 查询
  - [ ] temporal_bfs 查询
- [x] REST API 端点 (Phase 2 提前实现)
  - [x] POST /api/v1/graph/query
- [ ] CLI 命令
  - [ ] evif-mem graph causal-chain
  - [ ] evif-mem graph timeline
- [ ] 可视化工具（Web UI）

**交付物**:
- 完整的时序知识图谱系统
- 图谱查询 API
- 可视化界面
- 性能基准测试

### Phase 3: 主动演化系统 (Q3 2026)

**目标**: 实现 24/7 主动代理和自演化机制

**任务**:
- [ ] 实现 `EvolvePipeline`
  - [ ] Monitor（监控新输入）
  - [ ] Predictor（意图预测）
  - [ ] Updater（自动更新）
- [ ] 实现演化机制
  - [ ] Reinforce（强化）
  - [ ] Decay（衰减）
  - [ ] Merge（合并）
  - [ ] Prune（修剪）
- [ ] 实现后台任务调度
  - [ ] Tokio 后台任务
  - [ ] 定时任务（每天/每周）
- [ ] 实现 MCP Server
  - [ ] memorize 工具
  - [ ] retrieve 工具
  - [ ] evolve 工具
- [ ] Claude Code 集成测试

**交付物**:
- 主动演化系统
- MCP Server
- Claude Code 集成
- 用户文档

### Phase 4: 多模态支持 (Q4 2026)

**目标**: 支持图片、音频、视频等多模态记忆

**任务**:
- [ ] 实现 OCR 客户端
  - [ ] 集成 Tesseract/OpenAI Vision
- [ ] 实现 ASR 客户端
  - [ ] 集成 Whisper/Azure Speech
- [ ] 实现视频处理
  - [ ] 关键帧提取
  - [ ] OCR + ASR 融合
- [ ] 扩展 Resource 模型
  - [ ] 存储多模态数据
  - [ ] 元数据管理
- [ ] 测试和优化

**交付物**:
- 多模态记忆支持
- OCR/ASR 集成
- 性能优化
- 文档更新

### Phase 5: 企业级特性 (2027)

**目标**: 实现企业级功能和性能优化

**任务**:
- [ ] 多用户支持
  - [ ] 用户隔离
  - [ ] 权限管理
- [ ] 高级存储后端
  - [ ] PostgreSQL 集成
  - [ ] 云端备份（S3/Azure）
- [ ] 高级向量索引
  - [ ] FAISS 集成
  - [ ] Qdrant 集成
- [ ] 性能优化
  - [ ] 批量操作优化
  - [ ] 缓存策略
  - [ ] 并行处理
- [ ] 监控和告警
  - [ ] Prometheus 指标
  - [ ] Grafana 仪表盘
- [ ] 安全加固
  - [ ] 加密存储
  - [ ] 访问控制

**交付物**:
- 企业级部署
- 性能基准
- 安全审计报告
- 运维文档

---

## 📈 性能目标

### 响应时间

| 操作 | 目标 | 优化策略 |
|------|------|----------|
| 记忆化（单条） | < 2s | 并行提取、批量嵌入 |
| 向量检索（Top-10） | < 100ms | 向量索引优化、缓存 |
| LLM 读取（分类） | < 5s | 流式响应、并行读取 |
| 图谱查询（因果链） | < 500ms | 图索引、路径缓存 |
| 混合检索 | < 3s | 并行执行、早停优化 |

### 吞吐量

| 指标 | 目标 | 优化策略 |
|------|------|----------|
| 记忆化吞吐 | 100 条/秒 | 批量处理、异步管道 |
| 检索吞吐 | 1000 次/秒 | 连接池、缓存 |
| 并发用户 | 100 用户 | 负载均衡、水平扩展 |

### 存储效率

| 指标 | 目标 | 优化策略 |
|------|------|----------|
| MD 文件大小 | < 10KB/条 | 压缩、去重 |
| 向量索引大小 | < 1GB/100万条 | 量化、分区 |
| 图谱大小 | < 5GB/100万节点 | 压缩、归档 |

---

## 🔬 研究论文参考

### 核心架构论文

1. **Memory in the Age of AI Agents** (arXiv:2512.13564)
   - AI Agent 记忆研究全景
   - 记忆范围和边界定义

2. **Anatomy of Agentic Memory** (arXiv:2602.19320)
   - 记忆系统分类法
   - 实证分析方法

3. **Animesis: Constitutional Memory Architecture** (arXiv:2603.04740)
   - 四层记忆架构
   - 宪法式记忆设计

### 知识图谱记忆

4. **Zep: Temporal Knowledge Graph Architecture** (arXiv:2501.13956)
   - 时序知识图谱设计
   - 动态图谱引擎

5. **Graph-based Agent Memory Survey** (arXiv:2602.05665)
   - 图谱记忆分类法
   - 技术综述

6. **Empowering LLM Agents with Trainable Graph Memory** (arXiv:2511.07800)
   - 可训练图谱记忆
   - 多层记忆框架

### 向量检索与 RAG

7. **Performance Evaluation: Vector vs Graph-Based RAG** (arXiv:2602.17856)
   - 向量 vs 图谱性能对比
   - RAG 系统评估

8. **Hybrid RAG Systems with Embedding Vector Databases** (ResearchGate)
   - 混合检索系统
   - 向量数据库集成

### 自演化代理

9. **MemSkill: Learning and Evolving Memory Skills** (arXiv:2602.02474)
   - 自演化记忆技能
   - 自适应记忆管理

10. **SAGE: Self-evolving Agents with Reflective and Memory Augmented Abilities** (ACM)
    - 反思能力
    - 记忆增强

### 主动代理

11. **Proactive AI Agents** (memu.pro)
    - 24/7 主动监控
    - 意图预测

12. **ReasoningBank: Scaling Agent Self-Evolving** (OpenReview)
    - 推理记忆机制
    - 规模化自演化

---

## 📚 相关项目对比

### memU vs Mem

| 维度 | memU | Mem |
|------|------|-----|
| **语言** | Python | Rust |
| **存储格式** | JSON (PostgreSQL/SQLite) | MD + YAML (EVIF 抽象) |
| **向量检索** | pgvector | 自定义 VectorIndex trait |
| **图谱引擎** | 无 | evif-graph（时序扩展） |
| **文件系统** | 无 | EVIF FUSE mount |
| **AI 友好** | 中等（需解析 JSON） | 高（直接读 MD） |
| **版本控制** | 困难（数据库） | 容易（Git） |
| **性能** | 中等（Python + DB） | 高（Rust + 内存/KV） |
| **扩展性** | 插件系统 | EVIF 插件生态 |
| **主动代理** | 有（MemU Bot） | 计划中（EvolvePipeline） |
| **多模态** | 有 | 计划中（Phase 4） |
| **开源** | 是 | 是 |

### AGFS vs EVIF

| 维度 | AGFS (Turso) | EVIF |
|------|--------------|------|
| **语言** | Go | Rust |
| **理念** | Plan 9 "万物皆文件" | Plan 9 + 图谱 |
| **插件** | Go interface | Rust trait |
| **存储** | libsql (SQLite fork) | 多后端（Memory/KV/SQL/Cloud） |
| **图谱** | 无 | evif-graph |
| **向量** | 无 | vectorfs 插件 |
| **FUSE** | 无 | 有 |
| **动态加载** | 无 | 有（.so/.dylib） |
| **WASM** | 无 | 有（Extism） |

---

## 🎓 关键学习与洞察

### 设计洞察

1. **文件即记忆**: MD 格式是 AI 原生记忆的最佳存储格式
   - LLM 可直接读取和推理
   - Git 友好，版本控制清晰
   - FUSE 友好，可 mount 到本地

2. **三层架构的价值**:
   - Resource Layer: 保留原始数据，可追溯
   - MemoryItem Layer: 结构化提取，易检索
   - Category Layer: 自动组织，易浏览

3. **双模式检索的必要性**:
   - 向量检索: 快速，适合相似度匹配
   - LLM 读取: 深度，适合推理和关联
   - 混合模式: 兼顾速度和质量

4. **时序图谱的力量**:
   - 因果推理: 理解事件之间的因果关系
   - 时间线: 重构用户历史
   - 周期性模式: 发现用户习惯

5. **主动演化的意义**:
   - 自动化: 无需用户显式操作
   - 持续改进: 知识库质量随时间提升
   - 个性化: 适应用户习惯

### 技术洞察

1. **Rust 的优势**:
   - 性能: 零成本抽象，无 GC 停顿
   - 安全: 编译时借用检查，无数据竞争
   - 异步: Tokio 生态，高效并发

2. **EVIF 的价值**:
   - 统一抽象: 所有存储后端统一接口
   - 插件生态: 30+ 现成插件
   - FUSE 集成: 透明文件系统访问

3. **异步编程模式**:
   - 分离数据构建和锁获取（避免死锁）
   - 使用 `Arc<RwLock<T>>` 实现共享状态
   - 批量操作优于单次操作

4. **测试驱动开发**:
   - 单元测试: 每个模块独立测试
   - 集成测试: 端到端流程验证
   - 测试覆盖率: >80% 保证质量

### 产品洞察

1. **用户体验优先**:
   - 简单 API: POST /memories 即可记忆
   - 灵活检索: 多种检索模式适应不同场景
   - 透明存储: MD 文件可直接查看和编辑

2. **可扩展性**:
   - 插件架构: 易于添加新功能
   - 多后端存储: 适应不同规模需求
   - MCP 集成: 与 Claude Code 无缝协作

3. **企业就绪**:
   - 多用户支持: 租户隔离
   - 安全加固: 加密和访问控制
   - 监控告警: Prometheus + Grafana

---

## 🚀 未来展望

### 短期（2026）

- ✅ 完成核心管道实现（Phase 1）
- ✅ 集成时序图谱（Phase 2）
- ✅ 实现主动演化（Phase 3）
- ✅ 支持多模态（Phase 4）
- 📊 达到 1000+ 用户

### 中期（2027）

- 🔧 企业级特性完善
- 📈 性能优化（10x 提升）
- 🌐 云端托管服务
- 🔗 第三方集成（LangChain, LlamaIndex）
- 📊 达到 10000+ 用户

### 长期（2028+）

- 🧠 AGI 记忆系统
- 🌍 多语言支持
- 🤖 自主学习和适应
- 🔬 学术研究合作
- 📊 达到 100000+ 用户

---

## 📞 联系与贡献

**项目负责人**: EVIF Team
**许可证**: MIT OR Apache-2.0
**代码仓库**: https://github.com/your-org/evif-mem
**文档**: https://docs.evif-mem.io
**社区**: Discord, GitHub Discussions

---

## 📝 更新日志

### v1.0 (2026-03-07)
- ✅ 完成整体架构设计
- ✅ 融合 memU 和 EVIF 设计理念
- ✅ 制定详细实施路线图
- ✅ 整理相关论文和研究
- ✅ 明确技术选型和实现方案

---

**文档结束**

*本计划是一个活的文档，将随着项目进展不断更新和完善。*
