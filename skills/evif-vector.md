---
name: evif-vector
description: "EVIF 向量搜索技能 - 语义搜索、文档索引、相似度匹配"
parent: "evif"
tags: ["evif", "vector-search", "semantic-search", "embeddings", "ai"]
trigger_keywords: ["evif vector", "向量搜索", "语义搜索", "文档检索", "相似度"]
---

# EVIF 向量搜索

本文档详细介绍 EVIF VectorFS 的向量搜索、文档索引和语义匹配功能。

## 核心概念

VectorFS 将文档存储在 S3,向量索引存储在 TiDB,使用 OpenAI API 生成嵌入向量,支持高效的语义搜索。

### 工作流程

```
文档上传
    ↓
分块 (Chunking)
    ↓
生成嵌入 (Embedding via OpenAI)
    ↓
存储到 S3 + 向量索引到 TiDB
    ↓
支持语义搜索
```

## 基础操作

### 1. 挂载 VectorFS

```bash
evif mount vectorfs /vector \
  --s3-bucket=vector-docs \
  --tidb-host=localhost:4000 \
  --tidb-user=root \
  --tidb-password=yourpassword \
  --openai-api-key=sk-...
```

### 2. 创建命名空间

命名空间用于隔离不同类型的文档:

```bash
# 创建命名空间
evif vector create-ns /vector docs

# 列出所有命名空间
evif vector list-ns /vector
```

### 3. 添加文档

**添加单个文档:**
```bash
evif vector add /vector docs /path/to/document.pdf
```

**添加目录下所有文档:**
```bash
evif vector add /vector docs /path/to/documents/ --recursive
```

**添加多个文件:**
```bash
evif vector add /vector docs \
  file1.txt \
  file2.pdf \
  file3.md
```

**从 URL 添加:**
```bash
evif vector add /vector docs --url=https://example.com/article.html
```

### 4. 搜索文档

**基础语义搜索:**
```bash
evif vector search /vector docs "机器学习算法原理"
```

**返回前 K 个结果:**
```bash
evif vector search /vector docs "深度学习模型" --top-k=10
```

**指定相似度阈值:**
```bash
evif vector search /vector docs "Rust编程" --threshold=0.7
```

**返回 JSON 格式:**
```bash
evif vector search /vector docs "异步编程" --json
```

**组合过滤:**
```bash
evif vector search /vector docs \
  "数据库事务" \
  --top-k=5 \
  --threshold=0.8 \
  --filters=filetype:pdf,date:2024-01-01:2024-12-31
```

### 5. 查看索引状态

```bash
# 查看命名空间状态
evif vector status /vector docs

# 查看所有文档
evif vector list /vector docs

# 查看特定文档
evif vector get /vector docs document-id-123
```

### 6. 删除文档

```bash
# 删除单个文档
evif vector delete /vector docs document-id-123

# 批量删除
evif vector delete /vector docs doc-1 doc-2 doc-3

# 清空命名空间
evif vector clear /vector docs
```

## 高级功能

### 1. 自定义分块配置

**默认配置:**
- chunk_size: 1000 字符
- chunk_overlap: 200 字符

**自定义分块:**
```bash
evif vector add /vector docs /path/to/doc.txt \
  --chunk-size=2000 \
  --chunk-overlap=500
```

**不同文档类型使用不同配置:**
```bash
# PDF 文档 (更小的块)
evif vector add /vector docs article.pdf \
  --chunk-size=800 \
  --chunk-overlap=200

# 代码文件 (更大的块)
evif vector add /vector docs code.rs \
  --chunk-size=1500 \
  --chunk-overlap=300
```

### 2. 实时索引监控

**查看索引队列:**
```bash
evif vector queue /vector docs
```

输出:
```
Index Queue Status:
  Pending: 45 documents
  Processing: 3 documents
  Completed: 1,234 documents
  Failed: 2 documents
  Workers: 4 active
```

**查看失败的任务:**
```bash
evif vector failed /vector docs
```

**重试失败的任务:**
```bash
evif vector retry /vector docs --task-id=task-123
```

### 3. 多模态搜索

**文本 + 代码混合搜索:**
```bash
evif vector search /vector codebase "实现二叉树遍历" \
  --filters=language:rust
```

**多语言搜索:**
```bash
# 中文查询搜索英文文档
evif vector search /vector docs "异步编程模式" \
  --query-lang=zh \
  --doc-lang=en
```

### 4. 批量操作

**批量添加:**
```bash
# 从文件列表读取
cat file-list.txt | evif vector batch-add /vector docs

# 并发处理
evif vector batch-add /vector docs \
  --files=file-list.txt \
  --concurrent=10
```

**批量搜索:**
```bash
# 从文件读取查询
cat queries.txt | evif vector batch-search /vector docs
```

### 5. 增量更新

**更新已有文档:**
```bash
evif vector update /vector docs document-id-123 /new/path/to/doc.txt
```

**重新索引:**
```bash
# 重新生成嵌入
evif vector reindex /vector docs document-id-123

# 重新索引整个命名空间
evif vector reindex /vector docs --all
```

## 配置选项

### 分块配置

```toml
[plugins.vectorfs.chunking]
chunk_size = 1000
chunk_overlap = 200
min_chunk_size = 100
max_chunk_size = 5000
```

### 嵌入配置

```toml
[plugins.vectorfs.embedding]
provider = "openai"  # "openai", "custom"
model = "text-embedding-3-small"
dimensions = 1536
batch_size = 100
timeout = 30
```

### 索引配置

```toml
[plugins.vectorfs.indexing]
workers = 4
queue_size = 1000
retry_attempts = 3
retry_delay = 5
```

### 搜索配置

```toml
[plugins.vectorfs.search]
default_top_k = 5
min_similarity = 0.7
enable_hybrid_search = true  # vector + keyword
```

## 使用场景

### 场景1: 文档问答系统

```bash
# 1. 添加知识库文档
evif vector add /vector kb ./docs/ --recursive

# 2. 搜索相关文档
QUERY="如何配置 EVIF 插件?"
evif vector search /vector kb "$QUERY" --top-k=3

# 3. 使用检索到的文档回答问题
# (可以结合 GPTFS 实现完整 RAG)
```

### 场景2: 代码搜索

```bash
# 添加代码库
evif vector add /vector codebase ./src/ \
  --filters="extension:rs,go,py" \
  --chunk-size=500 \
  --chunk-overlap=100

# 语义搜索代码
evif vector search /vector codebase \
  "实现文件句柄管理的函数" \
  --filters=language:rust
```

### 场景3: 学术论文检索

```bash
# 添加论文
evif vector add /vector papers ./papers/ \
  --recursive \
  --filters="extension:pdf"

# 搜索相关论文
evif vector search /vector papers \
  "transformer architecture in natural language processing" \
  --top-k=10 \
  --filters="year:2020:2024"
```

### 场景4: 日志分析

```bash
# 添加日志文件
evif vector add /vector logs ./logs/ \
  --recursive \
  --filters="extension:log" \
  --chunk-size=2000 \
  --chunk-overlap=500

# 语义搜索错误日志
evif vector search /vector logs \
  "database connection timeout errors" \
  --filters="level:error,date:2024-01-25"
```

### 场景5: RAG (检索增强生成)

```bash
# 1. 添加知识库
evif vector add /vector rag ./knowledge-base/ --recursive

# 2. 搜索相关文档
CONTEXT=$(evif vector search /vector rag "用户认证流程" --top-k=3 --json)

# 3. 结合 GPTFS 生成回答
evif gpt prompt \
  "根据以下文档回答问题:\n$CONTEXT\n\n问题: 如何实现用户认证?"
```

## 性能优化

### 1. 批量处理

```bash
# 批量添加文档 (减少 API 调用)
evif vector add /vector docs ./docs/ \
  --batch-size=100 \
  --concurrent=10
```

### 2. 缓存

```bash
# 启用查询缓存
evif mount vectorfs /vector \
  --cache-enabled=true \
  --cache-size=1000 \
  --cache-ttl=3600
```

### 3. 索引优化

```bash
# 调整 worker 数量
evif mount vectorfs /vector \
  --index-workers=8 \
  --queue-size=5000
```

## 故障排查

### 常见问题

**1. 文档添加失败**

```bash
# 检查日志
evif vector log /vector docs

# 查看失败任务
evif vector failed /vector docs

# 重试
evif vector retry /vector docs --all
```

**2. 搜索结果不准确**

```bash
# 调整分块参数
evif vector add /vector docs doc.txt \
  --chunk-size=1500 \
  --chunk-overlap=300

# 使用更好的嵌入模型
evif mount vectorfs /vector \
  --embedding-model=text-embedding-3-large
```

**3. 索引速度慢**

```bash
# 增加 worker 数量
evif mount vectorfs /vector \
  --index-workers=8

# 使用批量处理
evif vector add /vector docs ./docs/ \
  --batch-size=200
```

### 调试模式

```bash
# 启用详细日志
RUST_LOG=evif_vector=debug evif-server

# 查看索引统计
evif vector stats /vector docs
```

输出:
```
VectorFS Statistics:
  Namespaces: 5
  Total documents: 1,234,567
  Total chunks: 12,345,678
  Average chunks per doc: 10
  Index queue size: 123
  Index workers: 4
  Search requests: 45,678
  Average search latency: 23ms
```

## 最佳实践

### 1. 文档预处理

**清理文本:**
```bash
# 移除格式字符
cat document.txt | sed 's/\x1b\[[0-9;]*m//g' > clean.txt

evif vector add /vector docs clean.txt
```

**元数据提取:**
```bash
# 添加文档时附带元数据
evif vector add /vector docs doc.pdf \
  --metadata=title:"Deep Learning",author:"Goodfellow",year:2016
```

### 2. 分块策略

**短文档 (文章、博客):**
- chunk_size: 800-1000
- chunk_overlap: 150-200

**长文档 (书籍、论文):**
- chunk_size: 1500-2000
- chunk_overlap: 300-500

**代码文件:**
- chunk_size: 500-800 (按函数分割)
- chunk_overlap: 100-150

### 3. 命名空间组织

```
/vector
  ├── docs/        # 文档
  ├── code/        # 代码
  ├── papers/      # 论文
  ├── logs/        # 日志
  └── kb/          # 知识库
```

### 4. 查询优化

**使用具体的关键词:**
```bash
# ❌ 模糊查询
evif vector search /vector docs "关于这个主题的信息"

# ✅ 具体查询
evif vector search /vector docs "Rust 异步编程最佳实践"
```

**结合过滤条件:**
```bash
evif vector search /vector docs "机器学习" \
  --filters="filetype:pdf,year:2023:2024"
```

## API 示例

### REST API

**添加文档:**
```bash
curl -X POST http://localhost:8080/api/v1/vector/add \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "docs",
    "file_path": "/path/to/document.pdf",
    "metadata": {
      "title": "EVIF User Guide",
      "author": "EVIF Team"
    }
  }'
```

**搜索:**
```bash
curl -X POST http://localhost:8080/api/v1/vector/search \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "docs",
    "query": "如何配置 S3 插件",
    "top_k": 5,
    "threshold": 0.7
  }'
```

### Python SDK

```python
import asyncio
from evif import EvifClient

async def main():
    async with EvifClient("http://localhost:8080") as client:
        # 添加文档
        await client.vector_add(
            namespace="docs",
            file_path="/path/to/doc.pdf",
            metadata={"title": "My Document"}
        )

        # 搜索
        results = await client.vector_search(
            namespace="docs",
            query="machine learning algorithms",
            top_k=5
        )

        for result in results:
            print(f"Score: {result.score:.3f}")
            print(f"Content: {result.content[:100]}...")
            print()

asyncio.run(main())
```

## 高级主题

### 1. 自定义嵌入函数

```bash
# 使用本地嵌入模型
evif mount vectorfs /vector \
  --embedding-provider=custom \
  --embedding-endpoint=http://localhost:8080/embeddings \
  --embedding-dimensions=768
```

### 2. 混合搜索 (向量 + 关键词)

```bash
evif vector search /vector docs "数据库优化" \
  --hybrid=true \
  --vector-weight=0.7 \
  --keyword-weight=0.3
```

### 3. 多向量索引

```bash
# 使用不同的嵌入模型
evif mount vectorfs /vector \
  --embedding-model-1=text-embedding-3-small \
  --embedding-model-2=text-embedding-3-large

# 搜索时选择模型
evif vector search /vector docs "query" \
  --embedding-model=text-embedding-3-large
```

---

**相关技能:**
- `SKILL.md` - EVIF 主技能
- `evif-manage.md` - 插件管理
- `evif-gpt.md` - GPT 集成 (RAG 应用)
