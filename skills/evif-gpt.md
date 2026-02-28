---
name: evif-gpt
description: "EVIF GPT/AI集成技能 - 文本处理、摘要、翻译、RAG应用"
parent: "evif"
tags: ["evif", "gpt", "ai", "llm", "text-processing", "openai"]
trigger_keywords: ["evif gpt", "gpt处理", "文本摘要", "翻译", "ai分析"]
---

# EVIF GPT/AI 集成

本文档详细介绍 EVIF GPTFS 的 AI 文本处理、自动摘要、翻译、RAG (检索增强生成) 等功能。

## 核心概念

GPTFS 将文件系统操作与 AI 能力结合,支持:
- **异步任务处理**: 提交任务 → 后台处理 → 获取结果
- **多种操作类型**: 摘要、翻译、改写、分析、生成
- **任务状态管理**: 查询进度、取消任务、重试失败
- **RAG 集成**: 结合 VectorFS 实现检索增强生成

## 基础操作

### 1. 挂载 GPTFS

**OpenAI:**
```bash
evif mount gptfs /gpt \
  --api-key=sk-... \
  --model=gpt-4 \
  --max-concurrent-jobs=5
```

**Azure OpenAI:**
```bash
evif mount gptfs /gpt \
  --api-key=your-azure-key \
  --endpoint=https://your-resource.openai.azure.com \
  --deployment-name=gpt-4 \
  --api-version=2024-02-15-preview
```

**使用其他模型:**
```bash
# GPT-3.5 (更快更便宜)
evif mount gptfs /gpt \
  --api-key=sk-... \
  --model=gpt-3.5-turbo

# Claude API (兼容OpenAI格式)
evif mount gptfs /gpt \
  --api-key=sk-ant-... \
  --endpoint=https://api.anthropic.com/v1 \
  --model=claude-3-opus-20240229
```

### 2. 文本处理任务

#### 摘要生成

**基础摘要:**
```bash
# 提交摘要任务
JOB_ID=$(evif write /gpt/jobs/summary \
  --input-file=/local/article.txt \
  --action=summary)

# 查询任务状态
evif cat /gpt/jobs/$JOB_ID/status

# 获取结果
evif cat /gpt/jobs/$JOB_ID/result
```

**指定摘要长度:**
```bash
evif write /gpt/jobs/summary \
  --input-file=/local/long-document.txt \
  --action=summary \
  --max-length=200
```

**结构化摘要:**
```bash
evif write /gpt/jobs/summary \
  --input-file=/local/report.txt \
  --action=summary \
  --format=bullets \
  --max-points=5
```

#### 翻译

**基础翻译:**
```bash
# 翻译为英文
evif write /gpt/jobs/translate \
  --input-file=/local/chinese-text.txt \
  --action=translate \
  --target-lang=en

# 翻译为中文
evif write /gpt/jobs/translate \
  --input-file=/local/english-paper.txt \
  --action=translate \
  --target-lang=zh

# 翻译为多种语言
evif write /gpt/jobs/translate \
  --input-file=/local/document.txt \
  --action=translate \
  --target-lang=en,ja,ko,fr,de
```

**保持格式:**
```bash
evif write /gpt/jobs/translate \
  --input-file=/local/markdown.md \
  --action=translate \
  --target-lang=en \
  --preserve-format=true
```

#### 文本改写

**改写风格:**
```bash
# 更正式
evif write /gpt/jobs/rewrite \
  --input-file=/local/informal.txt \
  --action=rewrite \
  --style=formal

# 更简洁
evif write /gpt/jobs/rewrite \
  --input-file=/local/verbose.txt \
  --action=rewrite \
  --style=concise

# 更详细
evif write /gpt/jobs/rewrite \
  --input-file=/local/brief.txt \
  --action=rewrite \
  --style=detailed
```

**转换为特定格式:**
```bash
# 转换为 Markdown
evif write /gpt/jobs/rewrite \
  --input-file=/local/plain-text.txt \
  --action=rewrite \
  --format=markdown

# 转换为 HTML
evif write /gpt/jobs/rewrite \
  --input-file=/local/document.txt \
  --action=rewrite \
  --format=html
```

#### 代码分析

**代码审查:**
```bash
evif write /gpt/jobs/analyze \
  --input-file=/local/code.rs \
  --action=code-review \
  --language=rust
```

**生成注释:**
```bash
evif write /gpt/jobs/analyze \
  --input-file=/local/script.py \
  --action=add-comments \
  --language=python
```

**重构建议:**
```bash
evif write /gpt/jobs/analyze \
  --input-file=/local/legacy.js \
  --action=refactor \
  --target-style=modern
```

#### 文档生成

**从代码生成文档:**
```bash
evif write /gpt/jobs/generate \
  --input-file=/local/evif-core/src/lib.rs \
  --action=generate-docs \
  --format=markdown
```

**API文档生成:**
```bash
evif write /gpt/jobs/generate \
  --input-file=/local/api.rs \
  --action=generate-api-docs \
  --format=openapi
```

### 3. 任务管理

#### 查询任务状态

```bash
# 查看所有任务
evif ls /gpt/jobs/

# 查看特定任务状态
evif cat /gpt/jobs/<job-id>/status

# 详细状态
evif cat /gpt/jobs/<job-id>/status --verbose
```

状态输出示例:
```json
{
  "job_id": "job-123",
  "status": "processing",
  "progress": 0.6,
  "created_at": "2025-01-25T10:30:00Z",
  "started_at": "2025-01-25T10:30:05Z",
  "estimated_completion": "2025-01-25T10:31:00Z",
  "input_file": "/local/article.txt",
  "action": "summary",
  "model": "gpt-4"
}
```

#### 取消任务

```bash
# 取消指定任务
evif write /gpt/jobs/<job-id>/cancel "cancel"

# 批量取消
for job_id in $(evif ls /gpt/jobs/ | grep "pending"); do
    evif write "/gpt/jobs/$job_id/cancel" "cancel"
done
```

#### 重试失败任务

```bash
# 重试一次
evif write /gpt/jobs/<job-id>/retry "retry"

# 自动重试 (最多3次)
evif write /gpt/jobs/<job-id>/retry \
  --max-attempts=3 \
  --backoff=exponential
```

#### 任务队列管理

```bash
# 查看队列状态
evif cat /gpt/queue/status

# 队列统计
evif cat /gpt/queue/stats
```

输出示例:
```
Queue Statistics:
  Pending: 15 tasks
  Processing: 3 tasks
  Completed: 1,234 tasks
  Failed: 12 tasks
  Cancelled: 5 tasks
  Workers: 5 active
  Average wait time: 23s
  Average processing time: 45s
```

## 高级功能

### 1. RAG (检索增强生成)

**基础 RAG:**
```bash
# 1. 添加知识库文档到 VectorFS
evif vector add /vector/kb ./knowledge-base/ --recursive

# 2. 搜索相关文档
CONTEXT=$(evif vector search /vector/kb \
  "如何配置 EVIF 插件" \
  --top-k=3 \
  --json)

# 3. 使用检索到的上下文生成回答
evif write /gpt/jobs/rag \
  --query="如何配置 EVIF 的 S3 插件?" \
  --context="$CONTEXT" \
  --action=answer
```

**完整 RAG 流程:**
```bash
# 创建 RAG 任务
JOB_ID=$(evif write /gpt/jobs/rag \
  --query="解释 EVIF 的 HandleFS 工作原理" \
  --vector-namespace=/vector/docs \
  --top-k=5 \
  --action=rag)

# 获取结果
evif cat /gpt/jobs/$JOB_ID/result
```

### 2. 批量处理

**批量摘要:**
```bash
# 批量处理目录中的所有文件
evif write /gpt/batch/summary \
  --input-dir=/local/articles/ \
  --pattern="*.txt" \
  --concurrent=3
```

**批量翻译:**
```bash
# 翻译整个项目
evif write /gpt/batch/translate \
  --input-dir=/local/docs-zh/ \
  --target-lang=en \
  --output-dir=/local/docs-en/ \
  --concurrent=5
```

### 3. 自定义提示模板

**创建模板:**
```bash
# 保存提示模板
evif write /gpt/templates/code-review "
请审查以下代码,重点关注:
1. 内存安全性
2. 错误处理
3. 性能优化
4. 代码风格

代码:
{input}

语言: {language}
"
```

**使用模板:**
```bash
evif write /gpt/jobs/process \
  --input-file=/local/code.rs \
  --template=code-review \
  --language=rust
```

### 4. 流式输出

**启用流式输出:**
```bash
# 实时查看生成过程
evif write /gpt/jobs/stream \
  --input-file=/local/article.txt \
  --action=summary \
  --stream=true

# 监控输出
evif cat /gpt/jobs/<job-id>/stream
```

### 5. 任务优先级

**设置优先级:**
```bash
# 高优先级任务
evif write /gpt/jobs/summary \
  --input-file=/local/urgent.txt \
  --action=summary \
  --priority=high

# 低优先级任务
evif write /gpt/jobs/translate \
  --input-file=/local/batch.txt \
  --action=translate \
  --target-lang=en \
  --priority=low
```

## 使用场景

### 场景1: 智能文档摘要

```bash
# 自动为文档库生成摘要
for file in /local/docs/*.pdf; do
    echo "Processing: $file"
    evif write "/gpt/jobs/summary" \
      --input-file="$file" \
      --action=summary \
      --max-length=300
done

# 查看所有完成的摘要
evif ls /gpt/jobs/ | grep "completed" | while read job; do
    echo "=== Summary for $job ==="
    evif cat "/gpt/jobs/$job/result"
    echo ""
done
```

### 场景2: 代码审查自动化

```bash
# 审查所有新提交的代码
evif write /gpt/batch/code-review \
  --input-dir=/local/new-code/ \
  --action=code-review \
  --output-format=markdown \
  --concurrent=5

# 生成审查报告
evif cat /gpt/reports/code-review-<timestamp>.md
```

### 场景3: 多语言文档生成

```bash
# 从源文档生成多语言版本
SOURCE_DOC=/local/manual-en.md

# 翻译为中文、日文、韩文
for lang in zh ja ko; do
    evif write "/gpt/jobs/translate-$lang" \
      --input-file="$SOURCE_DOC" \
      --action=translate \
      --target-lang=$lang \
      --preserve-format=true
done

# 等待所有翻译完成
evif wait /gpt/jobs/

# 收集结果
for lang in zh ja ko; do
    evif cat "/gpt/jobs/translate-$lang/result" > "/local/manual-$lang.md"
done
```

### 场景4: 智能客服系统

```bash
# 1. 构建知识库
evif vector add /vector/kb ./customer-support/ --recursive

# 2. 处理客户查询
QUERY="如何退款?"

# 检索相关知识
CONTEXT=$(evif vector search /vector/kb "$QUERY" --top-k=3 --json)

# 生成回答
evif write /gpt/jobs/answer \
  --query="$QUERY" \
  --context="$CONTEXT" \
  --action=customer-support \
  --tone=professional

# 获取回答
evif cat /gpt/jobs/<job-id>/result
```

### 场景5: 内容聚合与分析

```bash
# 聚合多个新闻源
for source in /local/news/*/; do
    evif write "/gpt/jobs/aggregate-$(basename $source)" \
      --input-dir="$source" \
      --action=summarize \
      --format=headlines
done

# 生成综合报告
evif write /gpt/jobs/generate-report \
  --input-dir=/gpt/jobs/ \
  --action=aggregate \
  --format=markdown
```

## 配置选项

### 模型配置

```toml
[plugins.gptfs]
api_key = "sk-..."
model = "gpt-4"
max_tokens = 4096
temperature = 0.7
top_p = 0.9
frequency_penalty = 0.0
presence_penalty = 0.0
```

### 任务队列配置

```toml
[plugins.gptfs.queue]
max_concurrent_jobs = 5
queue_size = 1000
default_priority = "normal"
retry_attempts = 3
retry_delay = 5  # seconds
```

### 超时配置

```toml
[plugins.gptfs.timeouts]
request_timeout = 30  # seconds
max_job_time = 300     # 5 minutes
queue_timeout = 3600   # 1 hour
```

### 结果存储配置

```toml
[plugins.gptfs.storage]
store_results = true
result_ttl = 86400  # 24 hours
compress_results = true
storage_path = "/var/lib/evif/gpt-results"
```

## 性能优化

### 1. 批量请求

```bash
# 使用批量API减少请求次数
evif write /gpt/batch/summary \
  --input-files=file1.txt,file2.txt,file3.txt \
  --batch-size=10
```

### 2. 并发控制

```bash
# 根据API限制调整并发数
evif mount gptfs /gpt \
  --api-key=sk-... \
  --max-concurrent-jobs=10  # OpenAI 限制: 350 RPM
```

### 3. 缓存结果

```bash
# 启用结果缓存
evif mount gptfs /gpt \
  --api-key=sk-... \
  --cache-enabled=true \
  --cache-ttl=3600
```

### 4. 使用更快的模型

```bash
# 对于简单任务使用 GPT-3.5
evif write /gpt/jobs/simple \
  --input-file=/local/simple.txt \
  --action=summary \
  --model=gpt-3.5-turbo \
  --max-tokens=500
```

## 故障排查

### 常见问题

**1. API 配额超限**
```bash
# 检查 API 使用情况
evif cat /gpt/stats/api-usage

# 降低并发数
evif write /gpt/config "max_concurrent_jobs=3"
```

**2. 任务超时**
```bash
# 增加超时时间
evif write /gpt/jobs/<job-id>/timeout 600

# 或拆分大任务
evif write /gpt/jobs/chunk \
  --input-file=/local/huge.txt \
  --action=summary \
  --chunk-size=50000
```

**3. 结果质量不佳**
```bash
# 调整模型参数
evif write /gpt/jobs/summary \
  --input-file=/local/article.txt \
  --action=summary \
  --temperature=0.3 \
  --max-tokens=1000

# 或使用更好的模型
--model=gpt-4
```

### 调试模式

```bash
# 启用详细日志
RUST_LOG=evif_gpt=debug evif-server

# 查看任务日志
evif cat /gpt/jobs/<job-id>/logs

# API 调用日志
evif cat /gpt/logs/api-calls
```

## 最佳实践

### 1. 提示工程

**明确指令:**
```bash
# ❌ 差
evif write /gpt/jobs/process \
  --input-file=/local/doc.txt \
  --action="总结"

# ✅ 好
evif write /gpt/jobs/process \
  --input-file=/local/doc.txt \
  --action="用3-5个要点总结这篇技术文档的核心内容,每个要点不超过50字"
```

**提供上下文:**
```bash
evif write /gpt/jobs/translate \
  --input-file=/local/legal.txt \
  --action=translate \
  --target-lang=en \
  --context="这是一份法律文档,需要保持专业术语的准确性"
```

### 2. 成本控制

**使用合适的模型:**
- 简单任务: `gpt-3.5-turbo` (便宜10倍)
- 复杂任务: `gpt-4` (质量最高)
- 代码任务: `gpt-4` (理解力强)

**限制输出长度:**
```bash
evif write /gpt/jobs/summary \
  --input-file=/local/article.txt \
  --action=summary \
  --max-tokens=500  # 节省成本
```

### 3. 错误处理

**自动重试:**
```bash
evif write /gpt/jobs/process \
  --input-file=/local/data.txt \
  --action=analyze \
  --retry-on-failure=true \
  --max-retries=3
```

**失败回调:**
```bash
evif write /gpt/jobs/process \
  --input-file=/local/data.txt \
  --action=analyze \
  --on-failure-hook="/scripts/notify-failure.sh"
```

## API 示例

### REST API

**提交任务:**
```bash
curl -X POST http://localhost:8080/api/v1/gpt/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "input_file": "/local/article.txt",
    "action": "summary",
    "model": "gpt-4",
    "max_tokens": 500
  }'
```

**查询状态:**
```bash
curl http://localhost:8080/api/v1/gpt/jobs/<job-id>/status
```

**获取结果:**
```bash
curl http://localhost:8080/api/v1/gpt/jobs/<job-id>/result
```

### Python SDK

```python
import asyncio
from evif import EvifClient

async def main():
    async with EvifClient("http://localhost:8080") as client:
        # 提交摘要任务
        job_id = await client.gpt_submit(
            input_file="/local/article.txt",
            action="summary",
            max_tokens=500
        )

        # 等待完成
        await client.gpt_wait(job_id, timeout=60)

        # 获取结果
        result = await client.gpt_get_result(job_id)
        print(result)

asyncio.run(main())
```

## 高级主题

### 1. 自定义模型

```bash
# 使用本地模型 (通过 OpenAI 兼容 API)
evif mount gptfs /gpt \
  --endpoint=http://localhost:8080/v1 \
  --api-key=unused \
  --model=llama-2-70b
```

### 2. 多模态处理

```bash
# 分析图片中的文本 (需要 vision 模型)
evif write /gpt/jobs/analyze-image \
  --input-file=/local/chart.png \
  --action=extract-text \
  --model=gpt-4-vision
```

### 3. 工作流编排

```bash
# 复杂的 AI 处理流程
# 1. 翻译
JOB1=$(evif write /gpt/jobs/translate \
  --input-file=/local/zh-doc.txt \
  --target-lang=en)

# 2. 等待翻译完成
evif wait /gpt/jobs/$JOB1

# 3. 摘要翻译后的文档
JOB2=$(evif write /gpt/jobs/summary \
  --input-file=/gpt/jobs/$JOB1/result)

# 4. 格式化输出
evif write /gpt/jobs/format \
  --input-file=/gpt/jobs/$JOB2/result \
  --format=markdown
```

---

**相关技能:**
- `SKILL.md` - EVIF 主技能
- `evif-vector.md` - 向量搜索 (RAG 应用)
- `evif-manage.md` - 插件管理
