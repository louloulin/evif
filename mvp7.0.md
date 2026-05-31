# EVIF MVP 7.0 Token 优化与功能完善计划

> 创建时间：2026-05-23  
> 更新历史：2026-05-23 (v2.0 - 整合 RTK 理念与 Token 优化)  
> 分析方式：代码审查、RTK 参考、EVIF 能力分析  
> 核心结论：EVIF 已具备强大的 Token 优化基础设施，需要**完善默认策略**、**增强可视化**、**扩展复用能力**

---

## 一、RTK 理念与 EVIF 对照分析

### 1.1 RTK (Rust Token Killer) 核心原则

RTK 是 Codeox CLI 的 Token 优化代理，核心设计：

```bash
rtk <cmd>    # Token 优化的命令执行
rtk gain     # Token 节省分析
rtk proxy    # 原始命令（不优化）
```

**RTK 的三大原则**：
1. **最小输出** - 只返回必要信息
2. **增量可见** - 进度输出，不累积
3. **Token 感知** - 量化节省了多少 Token

### 1.2 EVIF 当前 Token 优化能力

| 能力 | 实现位置 | 状态 | 说明 |
|------|----------|------|------|
| `max_lines` | `lib.rs:66` | ✅ | 文件读取行数限制，默认 100 |
| `mode=head/tail/snippet` | `lib.rs:66` | ✅ | 多种截断模式 |
| `compact=true` | `lib.rs:3325` | ✅ | 记忆搜索紧凑模式 |
| `limit` | 多个工具 | ✅ | 搜索结果数量限制 |
| `OutputFilter` | `output_filter.rs` | ✅ | 完整过滤管道 |
| LRU Cache | `lib.rs:227` | ✅ | 工具调用缓存 |
| Circuit Breaker | `circuit_breaker.rs` | ✅ | 失败保护 |
| ANSI Strip | `output_filter.rs` | ✅ | 去除颜色代码 |
| JSON Compact | `output_filter.rs` | ✅ | 去除 null 字段 |
| String Truncate | `output_filter.rs` | ✅ | 长字符串截断 |

### 1.3 EVIF 缺失的 RTK 能力

| 缺失能力 | RTK 模式 | 建议实现 | 优先级 |
|----------|----------|----------|--------|
| Token 节省统计 | `rtk gain` | `/metrics/token-stats` 端点 | P1 |
| CLI 输出代理 | `rtk <cmd>` | EVIF Shell 包装器 | P2 |
| 上下文压缩 | Prompt Summarization | Context 自动摘要 | P2 |
| 语义缓存 | Similar Query Cache | Embedding 相似缓存 | P2 |
| 流式输出 | Streaming | 大文件流式读取 | P2 |
| Token 预算 | Budget Tracking | Per-操作 Token 预算 | P3 |

---

## 二、Token 优化现状分析

### 2.1 当前默认值分析

```rust
// output_filter.rs
impl Default for OutputFilterConfig {
    fn default() -> Self {
        Self {
            strip_ansi: true,
            max_lines: 200,           // ⚠️ 可能偏大
            compact_json: true,
            max_string_length: 10000,  // ⚠️ 可能偏大
        }
    }
}

// lib.rs
// Token 优化注释
//! - `evif_cat`: 最多返回 100 行 (max_lines=100)  ✅
//! - `evif_search`: 最多返回 10 个结果 (limit=10)  ✅
//! - `evif_memory_search`: 紧凑输出模式 (compact=true)  ✅
```

### 2.2 发现的问题

| 问题 | 位置 | 影响 | 建议 |
|------|------|------|------|
| 默认 `max_lines=200` | `output_filter.rs` | Token 消耗偏高 | 降低到 50-100 |
| `max_string_length=10000` | `output_filter.rs` | 大 JSON 截断不足 | 降低到 5000 |
| CLI 缺少 `--compact` | `evif-cli` | CLI 输出未优化 | 添加全局 `--compact` |
| 无 Token 统计 | 全局 | 无法量化节省 | 添加 metrics |
| 缓存无 TTL | `lib.rs:235` | 可能返回过期数据 | 添加缓存 TTL |

---

## 三、Token 优化实现计划

### Phase 1: 默认值调优 (P0)

| 任务 | 文件 | 修改 | 影响 |
|------|------|------|------|
| 降低 `max_lines` | `output_filter.rs` | `200 → 50` | 节省 75% 行输出 |
| 降低 `max_string_length` | `output_filter.rs` | `10000 → 5000` | 节省 50% 字符串 |
| 文档化默认值 | `output_filter.rs` | 添加注释说明 | 可追溯 |

**实现代码**:
```rust
impl Default for OutputFilterConfig {
    fn default() -> Self {
        Self {
            strip_ansi: true,
            max_lines: 50,              // 降低默认行数
            compact_json: true,
            max_string_length: 5000,    // 降低默认字符串长度
        }
    }
}
```

---

### Phase 2: Token 统计与可视化 (P1)

| 任务 | 端点 | 说明 | 输出 |
|------|------|------|------|
| Token 统计端点 | `GET /metrics/tokens` | 累计节省统计 | JSON |
| 操作粒度统计 | `/metrics/ops` | 每种操作节省量 | JSON |
| Token 节省报告 | `evif-cli token-stats` | CLI 命令 | Table |

**API 设计**:
```json
{
  "total_tokens_saved": 125000,
  "by_operation": {
    "evif_cat": { "calls": 500, "avg_truncated": 200 },
    "evif_search": { "calls": 100, "avg_truncated": 5 },
    "memory_search": { "calls": 50, "avg_truncated": 10 }
  },
  "savings_percentage": 45.2
}
```

---

### Phase 3: CLI Token 代理 (P2)

| 任务 | 命令 | 说明 |
|------|------|------|
| EVIF Shell 包装 | `evif shell` | 交互式 Shell，自动优化输出 |
| 命令代理 | `evif run <cmd>` | 执行命令并优化输出 |
| 增量模式 | `evif watch <cmd>` | 实时输出，不累积 |

**实现示例**:
```bash
# 原始命令 (RTK 风格)
evif run cargo test --workspace 2>&1 | evif-truncate --lines 30

# EVIF Shell
evif shell
> cargo build
[building...]  2/50 crates
[building...] 10/50 crates
...
(最终只显示摘要)

# 节省效果
Tokens: 1200 → 180 (节省 85%)
```

---

### Phase 4: 上下文压缩 (P2)

| 任务 | 说明 | 实现方式 |
|------|------|----------|
| Context 自动摘要 | L0/L1 超过阈值时自动摘要 | LLM Summarization |
| 增量上下文 | 只传递变化部分 | Diff-based |
| Context 分页 | 大上下文分段加载 | Cursor-based |

**实现代码**:
```rust
// context_compression.rs
pub struct ContextCompressor {
    llm: Arc<dyn LlmClient>,
    max_context_lines: usize,
}

impl ContextCompressor {
    pub async fn compress(&self, content: &str) -> EvifResult<String> {
        if content.lines().count() <= self.max_context_lines {
            return Ok(content.to_string());
        }
        // 调用 LLM 进行摘要
        self.llm.summarize(content).await
    }
}
```

---

### Phase 5: 语义缓存 (P2)

| 任务 | 说明 | 实现方式 |
|------|------|----------|
| Embedding 相似缓存 | 相似查询返回缓存结果 | Vector similarity |
| TTL 缓存 | 缓存带过期时间 | Redis/SQLite |
| 缓存失效策略 | 基于依赖文件的智能失效 | File monitor |

**实现代码**:
```rust
// semantic_cache.rs
pub struct SemanticCache {
    embedding: Arc<dyn Embedder>,
    cache: Arc<SqliteCache>,
    similarity_threshold: f32,
}

impl SemanticCache {
    pub async fn get_or_compute(
        &self, 
        query: &str, 
        compute: impl FnOnce() -> EvifResult<String>
    ) -> EvifResult<String> {
        // 1. 计算 query embedding
        let embedding = self.embedding.embed(query).await?;
        
        // 2. 查找相似缓存
        if let Some(cached) = self.cache.find_similar(&embedding, self.similarity_threshold).await {
            return Ok(cached.value);
        }
        
        // 3. 计算并缓存
        let result = compute()?;
        self.cache.store(&embedding, &result).await?;
        Ok(result)
    }
}
```

---

### Phase 6: 流式输出 (P3)

| 任务 | 说明 | 使用场景 |
|------|------|----------|
| 大文件流式读取 | 分块返回，不阻塞 | tail -f 日志 |
| SSE 支持 | Server-Sent Events | 实时进度 |
| WebSocket 流 | 双向流 | 交互式操作 |

---

## 四、EVIF 能力复用指南

### 4.1 已有的 Token 优化能力

```
┌─────────────────────────────────────────────────────────┐
│                    EVIF Token 优化层                     │
├─────────────────────────────────────────────────────────┤
│  MCP 工具层                                              │
│  ├── evif_cat (max_lines, mode)                        │
│  ├── evif_search (limit)                               │
│  ├── evif_memory_search (compact)                      │
│  └── evif_grep (max_matches)                           │
├─────────────────────────────────────────────────────────┤
│  Output Filter 层                                       │
│  ├── strip_ansi → truncate_lines                       │
│  ├── compact_json → max_string_length                   │
│  └── 完整过滤管道可配置                                  │
├─────────────────────────────────────────────────────────┤
│  缓存层                                                  │
│  ├── LRU Call Cache                                    │
│  ├── Tool Cache                                        │
│  └── Prompt Cache                                      │
├─────────────────────────────────────────────────────────┤
│  上下文层                                                │
│  ├── /context/L0 (当前任务)                            │
│  ├── /context/L1 (决策记录)                            │
│  └── /context/L2 (项目知识)                            │
├─────────────────────────────────────────────────────────┤
│  记忆层                                                  │
│  ├── Vector Memory                                     │
│  ├── Semantic Search                                   │
│  └── Proactive Extraction                              │
└─────────────────────────────────────────────────────────┘
```

### 4.2 如何充分利用 EVIF 降低 Token

| 场景 | 使用 EVIF 能力 | 节省比例 |
|------|---------------|----------|
| 读取大文件 | `evif_cat --max-lines 50` | ~80% |
| 搜索代码 | `evif_search --limit 5` | ~70% |
| 重复查询 | LRU Cache | ~90% |
| 记忆检索 | `compact=true` | ~60% |
| 上下文恢复 | `/context/L0,L1` | ~50% |

---

## 五、功能完善清单

### 5.1 MVP 7.0 必须实现 (P0)

| 功能 | 说明 | 验收标准 |
|------|------|----------|
| 降低默认 `max_lines` | 200 → 50 | 输出验证 |
| 降低默认 `max_string_length` | 10000 → 5000 | 输出验证 |
| Token 统计端点 | `GET /metrics/tokens` | 返回 JSON 统计 |
| CLI `--compact` 全局参数 | `evif ls --compact` | 紧凑输出 |

### 5.2 MVP 7.0 应该实现 (P1)

| 功能 | 说明 | 验收标准 |
|------|------|----------|
| CLI Token 报告 | `evif token-stats` | 显示节省统计 |
| Context 压缩 | L0 超过 100 行自动摘要 | 摘要文件生成 |
| 缓存 TTL | 工具缓存过期时间 | 可配置 TTL |
| 操作粒度统计 | 每种操作单独统计 | 详细 metrics |

### 5.3 MVP 7.0 可以实现 (P2)

| 功能 | 说明 | 验收标准 |
|------|------|----------|
| EVIF Shell | 交互式优化 Shell | 手动测试 |
| 语义缓存 | 相似查询缓存 | 命中率 > 30% |
| 流式输出 | SSE 支持 | 大文件实时流 |
| Embedding 缓存 | 减少 embedding 计算 | 延迟降低 |

---

## 六、架构改进

### 6.1 模块重组建议

```
evif-mcp/src/
├── lib.rs                    # 主入口 (保持)
├── mcp_tools/               # 新目录：MCP 工具
│   ├── mod.rs
│   ├── file_tools.rs        # evif_cat, evif_ls, etc.
│   ├── search_tools.rs      # evif_search, evif_grep
│   ├── memory_tools.rs      # evif_memorize, evif_memory_search
│   ├── context_tools.rs     # L0/L1/L2 操作
│   └── pipe_tools.rs        # Pipe/Queue 操作
├── optimization/            # 新目录：Token 优化
│   ├── mod.rs
│   ├── output_filter.rs     # 保持
│   ├── token_stats.rs       # 新：Token 统计
│   ├── semantic_cache.rs    # 新：语义缓存
│   └── context_compress.rs  # 新：上下文压缩
├── cache/                   # 新目录：缓存层
│   ├── mod.rs
│   ├── lru_cache.rs         # 保持
│   └── ttl_cache.rs         # 新：TTL 缓存
└── metrics/                 # 新目录：指标
    ├── mod.rs
    ├── token_metrics.rs    # 新
    └── operation_metrics.rs # 新
```

### 6.2 配置分层

```yaml
# evif.toml

# Token 优化配置
[optimization]
default_max_lines = 50          # 降低默认
default_max_string = 5000       # 降低默认
compact_json = true
strip_ansi = true

# 缓存配置
[cache]
tool_cache_ttl_secs = 300       # 5分钟
semantic_cache_enabled = true
semantic_similarity_threshold = 0.95

# 上下文配置
[context]
l0_max_lines = 100              # 超过后摘要
l1_auto_summary = true
l2_index_enabled = true

# 流式配置
[streaming]
enabled = true
chunk_size = 1024
heartbeat_secs = 30
```

---

## 七、测试计划

### 7.1 Token 优化测试

```rust
#[cfg(test)]
mod token_optimization_tests {
    #[tokio::test]
    async fn test_max_lines_truncation() {
        let large_content = "line\n".repeat(500);
        let result = apply_line_truncation(&large_content, 50, "head");
        assert_eq!(result.lines().count(), 50);
    }
    
    #[tokio::test]
    async fn test_token_savings_tracking() {
        let stats = TokenStats::new();
        stats.record_savings("evif_cat", 1000, 200);
        assert_eq!(stats.total_saved(), 800);
    }
}
```

### 7.2 集成测试

```bash
# Token 优化集成测试
cargo test --package evif-mcp token_optimization

# CLI 紧凑模式测试  
cargo test --package evif-cli -- --compact

# 端到端 Token 节省测试
./scripts/test-token-savings.sh
```

---

## 八、执行路线图

### Week 1: 基础优化

| Day | 任务 | 交付物 |
|-----|------|--------|
| 1 | 调优默认值 | `max_lines=50`, `max_string=5000` |
| 2 | Token 统计端点 | `GET /metrics/tokens` |
| 3 | CLI `--compact` | `evif ls --compact` |
| 4 | 测试验证 | 单元测试 + 集成测试 |
| 5 | 文档更新 | README 更新 |

### Week 2: 高级特性

| Day | 任务 | 交付物 |
|-----|------|--------|
| 6-7 | Context 压缩 | 自动摘要功能 |
| 8-9 | CLI Token 报告 | `evif token-stats` |
| 10 | 缓存 TTL | 可配置过期 |

### Week 3-4: 高级优化

| Week | 任务 | 交付物 |
|------|------|--------|
| 3 | 语义缓存 | 相似查询缓存 |
| 4 | EVIF Shell | 交互式 Shell |
| 4 | 流式输出 | SSE 支持 |

---

## 九、成功标准

### 9.1 定量指标

| 指标 | 当前 | MVP 7.0 目标 |
|------|------|--------------|
| 平均工具响应大小 | ~2000 bytes | ~500 bytes |
| Token 节省比例 | N/A | >40% |
| 缓存命中率 | ~20% | >50% |
| CLI 紧凑输出 | ❌ | ✅ |

### 9.2 定性指标

- [ ] 所有 MCP 工具都有 Token 优化参数
- [ ] CLI 支持 `--compact` 全局参数
- [ ] Token 节省可量化显示
- [ ] 上下文超过阈值自动压缩
- [ ] 相似查询自动缓存

---

## 十、参考文档

### 10.1 RTK 参考

```
位置: /Users/louloulin/.codex/RTK.md

核心原则:
- rtk <cmd>  # Token 优化的命令执行
- rtk gain   # Token 节省统计
- rtk proxy  # 原始命令
```

### 10.2 EVIF 现有文档

```
docs/cost-optimization-matrix.md  # 成本优化矩阵
docs/mcp-product-design.md       # MCP 产品设计
mvp6.0.md                        # 历史分析
```

### 10.3 相关代码

```
crates/evif-mcp/src/output_filter.rs  # Output 过滤器
crates/evif-mcp/src/lib.rs           # MCP 主逻辑
crates/evif-cli/src/commands.rs      # CLI 命令
```

---

*文档版本: 2.0*  
*维护者: EVIF Team*  
*下次审查: Week 1 结束后*
