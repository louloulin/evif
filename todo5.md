# EVIF 后续开发计划 (todo5.md)

**创建日期**: 2026-03-01
**更新日期**: 2026-03-01
**基于**: 全面代码分析、AGFS 竞品研究、AI Agent 时代趋势分析、Claude Code Skills 研究、Meta Tools 理念分析
**目标**: 将 EVIF 打造成 AI Agent 时代的统一文件系统抽象层

---

## ⚠️ 分析修正说明

本次更新修正了之前分析中存在的主要问题:

1. **Claude Code 集成方式错误** - 之前误以为 Claude Code 只使用 MCP，实际 Claude Code 使用 Skills 规范(轻量级提示词扩展) + CLI-offloaded MCP 模式
2. **AGFS 竞品信息不准确** - 补充了 AGFS GitHub 仓库链接 (c4pt0r/agfs) 和作者信息 (PingCAP 黄东旭)
3. **缺少 "Context is File" 概念** - 添加了 AI Agent 长期记忆新范式
4. **缺少 AWCP 协议** - 添加了工作空间委托协议支持
5. **技能系统分析不足** - 添加了 Skills vs MCP 的详细对比
6. **缺少 Meta Tools 理念** - 添加 AI Agent 元工具概念分析
7. **缺少最新论文参考** - 添加 arXiv 2026 年相关论文
8. **图系统状态错误** - **修正: evif-graph 代码已完整实现，问题是未集成到 REST API**
9. **缺少 AgentFS 竞品** - 添加 Turso 团队的 AgentFS
10. **gRPC 服务状态不明** - 确认已禁用
11. **部分插件禁用原因** - 添加 OpenDAL TLS 冲突说明

---

## 🔍 代码库深入分析结果

### 已实现的模块 (完整)

| 模块 | 路径 | 说明 |
|------|------|------|
| **evif-core** | crates/evif-core | 核心抽象：插件接口、挂载表、句柄管理 |
| **evif-graph** | crates/evif-graph | 图引擎：节点、边、索引、查询、执行器 |
| **evif-plugins** | crates/evif-plugins | 30+ 插件实现 |
| **evif-rest** | crates/evif-rest | REST API 服务 (Axum)，66+ 端点 |
| **evif-vfs** | crates/evif-vfs | 虚拟文件系统接口 |
| **evif-cli** | crates/evif-cli | 命令行工具 |
| **evif-mcp** | crates/evif-mcp | MCP 服务器 |
| **evif-fuse** | crates/evif-fuse | FUSE 集成 |

### 发现的问题 (需修复)

| 问题 | 位置 | 状态 | 说明 |
|------|------|------|------|
| **图查询 API 未集成** | evif-rest/handlers.rs | 返回错误 | evif-graph 代码完整但 REST 返回 "not implemented" |
| **gRPC 服务禁用** | evif-grpc/server.rs.disabled | 已禁用 | 所有方法标记为 TODO |
| **MD5 哈希未实现** | fs_handlers.rs:457 | 未实现 | md5 crate 0.7 API 变化 |
| **WebDAV/FTP/SFTP** | plugins/ | 已禁用 | OpenDAL 0.50.2 TLS 冲突 |
| **evif-auth 未集成** | 多个 crate | 未集成 | 代码存在但未连接到 REST |

### 未实现的功能

1. **认证授权** - evif-auth crate 存在但未集成到 REST API
2. **动态插件加载** - DynamicPluginLoader 框架存在但未完全集成
3. **Claude Code Skills** - 只有 MCP 实现，无 Skills 规范
4. **时序知识图谱** - evif-graph 基础代码存在，需扩展时间属性

---

## 🔬 最新研究补充 (2026-03-01)

### Meta Tools 理念

**核心观点**: 文件系统正在成为 AI Agent 的"元工具"(Meta Tool)，是连接所有其他工具的枢纽。

**关键发现**:
- **Bash Is All Agent Need**: Anthropic 的方法论强调文件系统作为上下文工程的核心
- **文件系统即记忆**: 相比数据库，LLM 更擅长处理文件系统操作（cat, grep, ls）
- **可调试性**: 文件系统操作可追溯、可检查，不同于黑盒数据库

**Meta Tool 架构**:
```
AI Agent
    ├── 小型工作内存缓冲区 (最近 N 次操作)
    │           ↓ read/write/ls/grep
    └── Context File System (元工具层)
        ├── memory/ (长期/短期记忆)
        ├── tools/ (工具定义)
        ├── artifacts/ (产物)
        └── external_knowledge/ (外部知识)
```

### Claude Skills vs MCP 详细对比

| 维度 | Claude Skills | MCP Protocol |
|------|---------------|---------------|
| **本质** | 工作流指令手册 | 工具接口标准 |
| **类比** | "大脑/专业知识" | "手臂和腿 (USB-C)" |
| **回答问题** | "如何做某事" | "如何连接到某物" |
| **定位** | 定义任务执行方法 | 定义模型如何连接外部工具/数据 |
| **层级** | 内部模型执行逻辑 | 模型与外部系统间的连接协议 |
| **复杂度** | 简单 (文本) | 复杂 (需协议实现) |
| **Token 消耗** | 低 (渐进加载) | 高 |
| **范围** | Claude 特化 | 跨平台 |

**协同工作**: MCP = 底层协议 (如 USB-C); Skills = 上层应用 (如 USB 设备)

### "Context is File" 核心范式

**问题与解决方案**:

| 问题 | 传统方案 | 新范式 |
|------|----------|--------|
| Context 溢出 | 全部放入 prompt | 外部记忆，按需获取 |
| Token 成本膨胀 | 每次重复历史 | 只保留最小决策信息 |
| 质量衰减 | 注意力随长上下文分散 | 即时上下文策略 |
| 跨会话持久化 | 会话结束后丢失 | 持久化文件存储 |

**Just-in-Time Context**: 只在需要时加载上下文，避免 Token 浪费

### 2026 年关键论文

1. **Graph-based Agent Memory: Taxonomy, Techniques, and Challenges** (arXiv 2602.05665v1)
   - 2026年2月发布
   - 提出 Agent 记忆分类体系：短期 vs 长期、知识 vs 经验
   - 图记忆成为研究前沿

2. **OpenSage: Self-programming Agent Generation Engine** (arXiv 2602.16891v1)
   - 2026年2月发布
   - 结合短期历史和长期系统知识
   - 引用 Mem0 作为生产级解决方案

3. **AWCP: A Workspace Delegation Protocol** (arXiv 2602.20493v1)
   - 2026年2月发布
   - 多 Agent 协作的工作空间委托协议

4. **MAGMA: Multi-Graph based Agentic Memory Architecture** (arXiv 2601.03236)
   - 创新使用四个正交记忆图：实体图、因果图、时间图、专用图
   - 根据问题类型智能选择查询哪些图
   - "谁做了什么"查询实体和因果图，"何时发生"优先时间图

5. **Memory in the Age of AI Agents: A Survey** (arXiv 2512.13564)
   - 多所大学联合发布（NUS, 人大, 复旦, 北大, 同济）
   - 使用 "Form-Function-Dynamics" 三维框架分析 200+ 论文
   - 全面综述 Agent 记忆系统

6. **Fork, Explore, Commit: OS Primitives for Agentic Exploration** (arXiv 2602.08199v1)
   - 提出使用 OS 层级原语（fork、commit）进行 Agent 探索
   - 文件系统作为 Agent 记忆的物理层

7. **MIRIX: Multi-Agent Memory System for LLM-Based Agents** (arXiv 2507.07957v1)
   - 2025年7月发布
   - 多 Agent 记忆系统设计
   - 解决跨 Agent 记忆共享问题

8. **Continuum Memory Architectures for Long-Horizon LLM Agents** (arXiv 2601.09913v1)
   - 2026年1月发布
   - 探索长周期 Agent 交互的记忆架构
   - 分离世界事实、Agent 经验和演变信念

9. **A-Mem: Agentic Memory for LLM Agents** (arXiv 2502.12110)
   - 2025年2月发布
   - Agent 记忆机制研究
   - 自主记忆管理框架

10. **Graphs Meet AI Agents: Taxonomy, Progress, and Future** (arXiv 2506.18019v1)
    - 2025年6月发布
    - 图方法在 AI Agent 中的应用综述
    - 知识图谱与 Agent 系统集成

### 生产级记忆框架对比

| 框架 | 特点 | GitHub Stars |
|------|------|--------------|
| **Mem0** | 生产级长期记忆，比 OpenAI Memory 高 26% 准确率 | 46k+ |
| **Graphiti** | 时间感知记忆，Neo4j 集成 | 22.9k+ |
| **Cognee** | 图结构记忆 | 2k+ |
| **OpenMemory** | MCP 服务器运行，跨代理共享 | 1k+ |

**Mem0 性能数据**:
- 准确率比 OpenAI Memory 高 26%
- 延迟比全上下文方法低 91%
- Token 节省 90%

### 新发现: AgentFS (Turso)

| 特性 | EVIF | AGFS | AgentFS | 差距 |
|------|------|------|---------|------|
| 核心方法 | 9/9 ✅ | 9/9 ✅ | 部分 | 无/小 |
| 插件数量 | 30+ | 17 | SDK | 超越 |
| Radix Tree | ✅ | ✅ | ❌ | 无 |
| 双层缓存 | ❌ | ✅ | ❌ | **差距** |
| 动态插件 | ⚠️ 基础 | ✅ | ❌ | **差距** |
| 图查询 | ❌ 未集成 | ✅ | ❌ | **差距** |
| AI 记忆 | ❌ | ❌ | ❌ | 共同差距 |
| Skills 集成 | ❌ | ❌ | ❌ | 共同机会 |
| FUSE 挂载 | ✅ | ❌ | ✅ | - |

---

## 📋 执行摘要

### 核心定位

**EVIF (Everything Is a Virtual filesystem)** 是基于 Rust 实现的虚拟文件系统，遵循 Plan 9 "万物皆文件" 哲学。在 AI Agent 时代，EVIF 有机会成为：

1. **AI Agent 的统一资源抽象层** - 通过文件系统接口统一访问数据库、API、云存储、向量数据库等
2. **Claude Code 的深度集成组件** - 通过 Skills 规范提供文件系统后端和知识管理
3. **多 Agent 协作的工作空间** - 借鉴 AWCP 协议，实现文件级协作原语

### 当前状态 (2026-03-01)

| 维度 | 完成度 | 说明 |
|------|--------|------|
| **核心 API** | 100% ✅ | 9/9 文件系统方法完整实现 |
| **插件系统** | 90% ✅ | 30+ 插件，3 个因 OpenDAL TLS 冲突禁用 |
| **REST 服务** | 80% ⚠️ | 66+ API 端点，**图功能代码已实现但未集成** |
| **前端界面** | 60% ⚠️ | 基础功能完整，高级功能未接入 |
| **图系统** | 100% 代码 ⚠️ | **evif-graph 代码完整，但 REST API 未集成** |
| **AI 能力** | 10% ❌ | 缺少时序记忆、向量检索优化 |
| **认证授权** | 0% ❌ | 未实现 (注: evif-auth crate 存在但未集成) |
| **Claude Code Skills** | 0% ❌ | 未实现 (当前只有 MCP) |
| **gRPC 服务** | 0% ❌ | 已禁用 |

### 与竞品对比

| 特性 | EVIF | AGFS | AgentFS | 差距 |
|-----|------|------|---------|------|
| 核心方法 | 9/9 ✅ | 9/9 ✅ | 部分 | 无 |
| 插件数量 | 30+ | 17 | SDK | 超越 |
| Radix Tree | ✅ | ✅ | ❌ | 无 |
| 双层缓存 | ❌ | ✅ | ❌ | **差距** |
| 动态插件 | ⚠️ 基础 | ✅ | ❌ | **差距** |
| 图查询 | ❌ 未集成 | ✅ | ❌ | **差距** |
| AI 记忆 | ❌ | ❌ | ❌ | 共同差距 |
| Skills 集成 | ❌ | ❌ | ❌ | 共同机会 |

---

## 🎯 EVIF 在 AI Agent 时代的战略定位

### 为什么 EVIF 是"元工具"的最佳载体

**核心理念**: EVIF 不仅仅是一个虚拟文件系统，而是 AI Agent 时代的"元工具"(Meta Tool)基础设施。

| 能力 | EVIF 现状 | AI Agent 需求 | 优先级 |
|------|----------|---------------|--------|
| **统一资源抽象** | 插件系统完善 | 访问数据库、API、云存储、向量库 | P0 |
| **上下文记忆** | 基础 | 长期记忆 + 即时上下文 | P0 |
| **工具编排** | 基础 | 多工具组合、链式调用 | P1 |
| **多 Agent 协作** | 无 | AWCP 协议支持 | P2 |
| **Skills 集成** | 无 | Claude Code 深度集成 | P1 |

### EVIF = 下一代上下文引擎

```
┌─────────────────────────────────────────────────────────┐
│                    AI Agent                              │
│  ┌─────────────────────────────────────────────────────┐│
│  │         小型工作内存缓冲区 (8-15 次操作)            ││
│  └─────────────────────────────────────────────────────┘│
│                          ↓                               │
│  ┌─────────────────────────────────────────────────────┐│
│  │              EVIF (元工具层)                        ││
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌───────┐ ││
│  │  │ memory/ │ │  tools/ │ │vector/ │ │graph/ │ ││
│  │  │ 记忆系统 │ │ 工具抽象 │ │向量检索 │ │图查询 │ ││
│  │  └─────────┘ └─────────┘ └─────────┘ └───────┘ ││
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌───────┐ ││
│  │  │  s3fs   │ │  sqlfs  │ │  gptfs  │ │kvfs   │ ││
│  │  │ 云存储   │ │  SQL    │ │  LLM    │ │ KV    │ ││
│  │  └─────────┘ └─────────┘ └─────────┘ └───────┘ ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

### 差异化竞争优势

1. **比 AGFS 更强的 AI 能力**
   - 内置图查询系统
   - 向量检索支持
   - 时序记忆（参考 Mem0/Graphiti）

2. **比 Mem0/Graphiti 更通用**
   - 统一的文件系统接口
   - 丰富的后端插件
   - FUSE 挂载支持

3. **比传统 VFS 更适合 AI**
   - 原生 MCP/Skills 集成
   - RAG 能力内置
   - 多租户支持

---

## 🎯 开发路线图

### Phase 1: 核心功能补全 (1-2 个月)

#### 1.1 图查询系统集成 🔴 P0

**目标**: 启用 evif-graph，实现 AI Agent 核心能力

**重要发现**: evif-graph 代码已完整实现（节点、边、索引、查询、执行器），但 REST API 中返回 "Graph functionality not implemented" 错误。这是集成问题，非代码缺失。

**工作项**:
1. **集成 evif-graph 到 REST API**
   - 将 `/nodes/:id` 从返回错误改为实际实现
   - 实现 `/query` 端点（图查询语言）
   - 实现 `/stats` 端点（图统计信息）
   - 连接 evif-core 的 MountTable 与 evif-graph 的 Graph

2. **插件与图的双向映射**
   - 文件系统路径 → 图节点
   - 图节点 → 文件系统路径
   - 边关系（父子、引用、依赖）

3. **图查询 API 设计**
   ```
   POST /api/v1/graph/query
   {
     "query": "MATCH (n:File)-[:CHILD_OF]->(p:Directory) WHERE p.path = '/mem' RETURN n",
     "parameters": {}
   }
   ```

**验收标准**:
- [ ] `/nodes/:id` 返回实际节点数据
- [ ] `/query` 执行基本图查询
- [ ] 图查询与文件操作互操作
- [ ] 单元测试覆盖率 > 80%

**预计工期**: 2-3 周

**依赖**: 无

---

#### 1.2 API 契约统一 ✅ 已解决

**状态**: 已验证一致，测试通过

**验证结果** (2026-03-01):
- `handlers.rs` 返回: `{ content, data: base64, size }`
- `evif-client` 读取: `json["data"]` (base64) 并解码
- 测试 `test_read_file_returns_data_and_content` 通过
- 测试 `test_write_file_accepts_base64_encoding` 通过

**原问题分析** (已过时):
- ~~`evif-client` 期望 read 返回 base64 编码的 `data` 字段~~
- ~~`handlers` 返回明文 JSON 的 `content` 字段~~
- ~~`write` 操作同样存在不一致~~

**当前实现**:
- Read: 返回 `{ content, data: base64, size }`
- Write: 接受 `{ data, encoding: "base64" }`

**结论**: 该问题已在代码中修复，todo5.md 原描述不准确

---

#### 1.3 双层缓存系统 🔴 P0

**目标**: 实现 dirCache + statCache，性能提升 100-500x

**借鉴**: AGFS S3FS 缓存设计

**工作项**:
1. **缓存框架实现**
   ```rust
   pub struct CacheManager {
       dir_cache: Arc<RwLock<LruCache<String, Vec<FileInfo>>>>,
       stat_cache: Arc<RwLock<LruCache<String, FileInfo>>>,
       ttl: Duration,
       max_size: usize,
   }
   ```

2. **LRU 淘汰策略**
   - 使用 `lru` crate
   - 配置化最大缓存大小
   - 内存使用监控

3. **TTL 自动失效**
   - 可配置 TTL（默认 30s）
   - 惰性过期检查
   - 主动失效（写入/删除操作）

4. **缓存一致性**
   - 写操作失效相关缓存
   - `Invalidate(prefix)` 支持批量失效
   - 跨实例一致性（可选 Redis）

**验收标准**:
- [ ] 缓存命中率 > 80%（压测）
- [ ] P99 延迟降低 100x
- [ ] 内存使用 < 500MB
- [ ] 缓存一致性正确

**预计工期**: 1-2 周

**依赖**: 无

---

#### 1.4 配置化挂载系统 🟡 P1

**目标**: 替代硬编码挂载点，支持配置文件

**当前问题**:
```rust
// evif-rest/src/server.rs 硬编码
mount_table.mount("/mem".to_string(), memfs_plugin).await?;
mount_table.mount("/hello".to_string(), hellofs_plugin).await?;
mount_table.mount("/local".to_string(), localfs_plugin).await?;
```

**工作项**:
1. **配置文件设计**
   ```yaml
   # evif.yaml
   server:
     host: "0.0.0.0"
     port: 8081

   mounts:
     - path: "/mem"
       plugin: "memfs"
       config: {}

     - path: "/s3"
       plugin: "s3fs"
       config:
         bucket: "my-bucket"
         region: "us-west-2"
         access_key: "${AWS_ACCESS_KEY_ID}"
         secret_key: "${AWS_SECRET_ACCESS_KEY}"
   ```

2. **环境变量支持**
   - `${VAR}` 语法
   - 默认值: `${VAR:-default}`

3. **热重载（可选）**
   - 监控配置文件变化
   - 重新挂载插件

**验收标准**:
- [ ] 支持至少 3 种配置格式（YAML/TOML/JSON）
- [ ] 环境变量正确展开
- [ ] 配置错误友好提示
- [ ] 向后兼容（无配置文件时使用默认）

**预计工期**: 1 周

**依赖**: 1.3 缓存系统（可选）

---

#### 1.5 前端高级功能接入 🟡 P1

**目标**: 将已存在但未接入的前端组件对接后端

**现有组件**:
- `plugin-manager`: PluginList, MountModal, PluginModal
- `monitor`: SystemStatus, TrafficChart, LogViewer
- `collaboration`: Comments, Permissions, Sharing

**工作项**:
1. **插件管理 UI**
   - 对接 `/api/v1/plugins` (列出插件)
   - 对接 `/api/v1/plugins/load` (加载插件)
   - 对接 `/api/v1/mount` (挂载操作)
   - 对接 `/api/v1/unmount` (卸载操作)

2. **监控面板**
   - 对接 `/api/v1/metrics/traffic` (流量统计)
   - 对接 `/api/v1/metrics/operations` (操作统计)
   - 对接 `/api/v1/metrics/status` (系统状态)
   - 实时 WebSocket 更新

3. **路由集成**
   - 添加 `/plugins` 路由
   - 添加 `/monitor` 路由
   - 添加 `/settings` 路由

**验收标准**:
- [ ] 插件管理 UI 可用
- [ ] 监控面板实时更新
- [ ] 无 Console 错误
- [ ] 响应式设计（移动端适配）

**预计工期**: 2-3 周

**依赖**: 1.4 配置化挂载

---

### Phase 2: AI Agent 时代能力 (3-6 个月)

#### 2.0 "Context is File" 核心范式 🟢 P2

**目标**: 将 EVIF 打造成 AI Agent 的 "终极上下文" 存储层

**核心理念**: 文件系统语义是 AI Agent 长期记忆的最佳交互层，即时上下文 (Just-in-Time Context) 控制 token 成本

**工作项**:
1. **记忆文件系统接口**
   ```
   /memory/
   ├── episodes/      # 会话/事件记忆
   ├── concepts/      # 概念/知识记忆
   ├── entities/      # 实体记忆
   └── context/       # 即时上下文
   ```

2. **自动记忆管理**
   - 访问频率加权
   - 重要性评分
   - 自动归档/清理

3. **上下文注入 API**
   ```
   POST /api/v1/context/inject
   {
     "query": "当前任务描述",
     "max_tokens": 8000,
     "strategy": "relevant" // relevant, recent, important
   }
   ```

**验收标准**:
- [ ] 记忆文件系统可用
- [ ] 上下文注入减少 token 使用
- [ ] 与 Claude Code Skills 集成

**预计工期**: 2-3 周

**依赖**: 1.1 图查询、2.1 时序知识图谱

#### 2.1 时序知识图谱 🔴 P0

**目标**: 为 AI Agent 提供时序感知的记忆系统

**参考**: Graphiti (Zep), Mem0, Cognee

**核心设计**:
1. **双时间线模型**
   ```
   EventTimeline (事件记忆)
   ├── Episode (会话/事件)
   ├── Timestamp (时间戳)
   └── Entities (参与实体)

   SemanticTimeline (语义记忆)
   ├── Concept (概念/知识)
   ├── Relationships (关系)
   └── Embeddings (向量嵌入)
   ```

2. **时间衰减与重要性**
   - 遗忘曲线模拟
   - 访问频率加权
   - 重要性评分

3. **图操作 API**
   ```
   POST /api/v1/memory/episodes
   POST /api/v1/memory/concepts
   POST /api/v1/memory/query
   GET /api/v1/memory/timeline?start=&end=
   ```

**工作项**:
1. **扩展 evif-graph**
   - 添加时间属性
   - 添加权重/衰减算法
   - 时间范围查询

2. **记忆管理 API**
   - 存储会话/事件
   - 提取知识三元组
   - 自动关联

3. **向量集成**
   - 节点向量化
   - 语义搜索
   - RAG 支持

**验收标准**:
- [ ] 存储 10k+ 节点性能无衰减
- [ ] 时间范围查询 < 100ms
- [ ] 向量检索召回率 > 90%
- [ ] 与 Claude Code MCP 集成

**预计工期**: 4-6 周

**依赖**: 1.1 图查询系统

---

#### 2.2 向量检索优化 🟡 P1

**目标**: 实现高性能向量检索，支持 RAG

**当前状态**: `vectorfs` 插件存在但未优化

**工作项**:
1. **向量存储后端**
   - 集成 FAISS
   - 集成 Qdrant（可选）
   - 支持多种距离度量（Cosine, L2, IP）

2. **索引优化**
   - HNSW 索引
   - IVF 索引
   - 自动索引选择

3. **RAG 能力**
   ```
   POST /api/v1/vector/search
   {
     "query": "用户问题",
     "top_k": 10,
     "filter": {
       "path": "/knowledge/*",
       "time_range": "2026-01-01:2026-03-01"
     }
   }
   ```

4. **向量缓存**
   - 查询缓存
   - 结果缓存
   - 预加载热门向量

**验收标准**:
- [ ] 1M 向量检索 < 10ms
- [ ] 召回率 > 95%
- [ ] 支持动态插入/删除
- [ ] 与时序图集成

**预计工期**: 3-4 周

**依赖**: 2.1 时序知识图谱

---

#### 2.3 动态插件系统 🟡 P1

**目标**: 运行时加载/卸载插件，提升扩展性

**借鉴**: AGFS 插件系统

**工作项**:
1. **插件生命周期**
   ```rust
   pub trait EvifPluginV2: EvifPlugin {
       fn name(&self) -> &str;
       fn version(&self) -> &str;
       fn validate_config(config: &Value) -> EvifResult<()>;
       fn initialize(config: Value) -> EvifResult<Self> where Self: Sized;
       fn shutdown(&self) -> EvifResult<()>;
   }
   ```

2. **动态加载**
   - dlopen 加载 .so/.dylib
   - 符号解析
   - 安全沙箱

3. **WASM 插件**
   - 扩展现有 extism 支持
   - WASI 绑定
   - 资源限制

4. **插件市场（可选）**
   - 插件注册表
   - 版本管理
   - 依赖解析

**验收标准**:
- [ ] 运行时加载 .so 插件
- [ ] 插件卸载不泄漏资源
- [ ] WASM 插件正常运行
- [ ] 插件错误不影响主进程

**预计工期**: 4-5 周

**依赖**: 1.4 配置化挂载

---

#### 2.4 Claude Code Skills 深度集成 🟡 P1

**目标**: 成为 Claude Code 的文件系统后端和知识管理核心

**重要发现**: Claude Code 使用 Skills 而非纯 MCP - 这是两种不同的集成方式:
- **Skills**: 轻量级、基于提示词的能力扩展，类似 npm 包
- **MCP**: 工具调用协议，类似 USB-C 标准
- Claude Code 采用 CLI-offloaded MCP 模式

**当前状态**: `evif-mcp` 基础实现，Skills 未实现

**工作项**:
1. **Skills 规范实现**
   - 遵循 Agent Skills 规范 (https://agentskills.io/specification)
   - 三层架构: 元数据层、指令层、资源层
   - 支持目录结构和渐进式提示加载

2. **EVIF Skills 包设计**
   ```
   evif-skills/
   ├── skill.yaml          # 元数据
   ├── prompts/
   │   ├── read.md        # 读取文件指令
   │   ├── write.md       # 写入文件指令
   │   ├── query.md       # 图查询指令
   │   └── memory.md      # 记忆管理指令
   └── resources/
       └── examples/      # 示例代码
   ```

3. **MCP + Skills 双通道**
   - MCP: 提供底层工具调用能力
   - Skills: 提供业务工作流和最佳实践
   - 两者协同工作

4. **知识管理集成**
   - Claude Code 读写图节点
   - RAG 检索集成
   - 上下文注入

**验收标准**:
- [ ] Skills 包符合规范
- [ ] 与 Claude Code 实际集成测试
- [ ] 性能满足交互需求（< 1s 响应）
- [ ] 文档和示例完整

**预计工期**: 3-4 周

**依赖**: 1.1 图查询、2.2 向量检索

---

### Phase 3: 企业级特性 (6-12 个月)

#### 3.0 AWCP 协议支持 🟢 P2

**目标**: 支持多 Agent 协作的工作空间委托协议

**参考**: [AWCP: A Workspace Delegation Protocol](https://arxiv.org/html/2602.20493v1)

**工作项**:
1. **AWCP 控制平面**
   - 工作空间委托机制
   - 文件级协作原语

2. **多 Agent 协作**
   - 文件锁机制
   - 变更通知 (WebSocket)
   - 冲突解决策略

3. **协议适配器**
   - 支持 MCP、A2A、ANP、HTTP

**验收标准**:
- [ ] AWCP 协议兼容
- [ ] 多 Agent 协作示例
- [ ] 文档完整

**预计工期**: 3-4 周

**依赖**: 2.1 时序知识图谱

#### 3.1 认证与授权 🔴 P0

**目标**: 企业级安全能力

**工作项**:
1. **用户认证**
   - JWT Token
   - OAuth 2.0 / OpenID Connect
   - API Key

2. **权限控制**
   - 基于能力的安全模型（evif-auth）
   - RBAC (Role-Based Access Control)
   - 资源级权限

3. **审计日志**
   - 操作日志
   - 访问日志
   - 合规报告

4. **安全加固**
   - TLS/HTTPS
   - 速率限制
   - 输入验证

**验收标准**:
- [ ] JWT 认证可用
- [ ] 细粒度权限控制
- [ ] 审计日志完整
- [ ] 通过安全扫描

**预计工期**: 4-6 周

**依赖**: 无

---

#### 3.2 高可用与分布式 🟡 P1

**目标**: 生产级可用性

**工作项**:
1. **分布式挂载表**
   - 一致性协议（Raft）
   - 节点发现
   - 故障转移

2. **数据复制**
   - 插件状态同步
   - 缓存一致性
   - 最终一致性保证

3. **负载均衡**
   - 请求路由
   - 连接池
   - 熔断降级

4. **监控告警**
   - Prometheus 指标
   - Grafana 面板
   - Alertmanager 告警

**验收标准**:
- [ ] 3 节点集群正常运行
- [ ] 故障转移 < 10s
- [ ] 数据一致性保证
- [ ] 99.9% 可用性 SLA

**预计工期**: 8-10 周

**依赖**: 3.1 认证授权

---

#### 3.3 高级 AI 能力 🟢 P2

**目标**: 差异化竞争优势

**工作项**:
1. **多模型支持**
   - GPT-4, Claude, Gemini
   - 本地模型 (Llama, Mistral)
   - 模型路由策略

2. **自动记忆管理**
   - 重要性评估
   - 自动归档
   - 自动清理

3. **上下文压缩**
   - 摘要生成
   - 去重
   - 分块策略

4. **Agent 编排**
   - 多 Agent 协作
   - 任务分解
   - 工作流引擎

**验收标准**:
- [ ] 支持 3+ LLM 提供商
- [ ] 记忆压缩率 > 50%
- [ ] 多 Agent 协作示例

**预计工期**: 6-8 周

**依赖**: 2.1 时序知识图谱

---

#### 3.4 生态建设 🟢 P2

**目标**: 社区与生态

**工作项**:
1. **SDK 开发**
   - Python SDK (evif-python 已存在)
   - Go SDK
   - JavaScript/TypeScript SDK

2. **插件市场**
   - 官方插件仓库
   - 社区贡献插件
   - 插件评级

3. **文档与示例**
   - 完整 API 文档
   - 教程 (Tutorial)
   - 示例项目
   - 视频教程

4. **社区运营**
   - GitHub Discussions
   - Discord/Slack 社区
   - 贡献指南
   - ISSUE 模板

**验收标准**:
- [ ] 3 个语言 SDK 可用
- [ ] 20+ 社区插件
- [ ] 文档完整度 > 80%
- [ ] 1k+ GitHub Stars

**预计工期**: 持续

**依赖**: 所有基础功能完成

---

## 📊 技术债务与重构

### 优先修复

1. **统一两条技术栈**
   - 问题: 图 + VFS 与 插件 + REST 并行存在
   - 方案: 图查询作为高级功能，插件系统保持简洁
   - 预计: 1 周

2. **测试覆盖率提升**
   - 当前: ~30% 单元测试
   - 目标: >80% 单元测试，>60% 集成测试
   - 方案: 补充 E2E 测试
   - 预计: 2-3 周

3. **错误处理标准化**
   - 统一 `EvifError` 类型
   - 结构化错误响应
   - 错误码规范
   - 预计: 1 周

### 性能优化

1. **零拷贝传输**
   - 使用 `bytes::Bytes`
   - 减少 `Vec<u8>` 克隆
   - 预计: 1 周

2. **流式接口**
   - 实现 `Open`/`OpenWrite` trait
   - 支持大文件流式传输
   - 预计: 1-2 周

3. **连接池**
   - HTTP 客户端连接池
   - 数据库连接池
   - 预计: 1 周

---

## 🎯 成功指标与里程碑

### Q2 2026 (2 个月)

- [ ] 图查询系统集成 ✅
- [ ] API 契约统一 ✅
- [ ] 双层缓存系统 ✅
- [ ] 配置化挂载 ✅
- [ ] 前端高级功能接入 ✅

**里程碑**: MVP 1.0 - 核心功能完整

### Q3 2026 (3-6 个月)

- [ ] 时序知识图谱 ✅
- [ ] 向量检索优化 ✅
- [ ] 动态插件系统 ✅
- [ ] MCP 深度集成 ✅

**里程碑**: AI Agent Edition - 智能能力就绪

### Q4 2026 (6-12 个月)

- [ ] 认证授权 ✅
- [ ] 高可用部署 ✅
- [ ] 多语言 SDK ✅
- [ ] 插件市场上线 ✅

**里程碑**: Enterprise Edition - 生产级可用

### 长期目标 (12 个月+)

- **技术指标**
  - P99 延迟 < 100ms
  - QPS > 10k
  - 可用性 99.9%

- **生态指标**
  - 1k+ GitHub Stars
  - 50+ 插件
  - 10+ 贡献者
  - 3+ 企业用户

- **应用指标**
  - 成为 Claude Code 标准组件
  - 3+ AI Agent 框架集成
  - 5+ 生产案例

---

## 🔗 参考资料

### 竞品分析

- **AGFS**: [GitHub - c4pt0r/agfs](https://github.com/c4pt0r/agfs) - PingCAP 联合创始人黄东旭创建
- **AGFS 文章**: [AGFS：致敬 Plan 9 "万物皆文件"理念的 Agent 文件系统](https://m.php.cn/faq/1790204.html)
- **AgentFS**: [GitHub - tursodatabase/agentfs](https://github.com/tursodatabase/agentfs) - Turso 团队的 Agent 文件系统
- **Mem0**: [GitHub](https://github.com/mem0ai/mem0) - 41k+ stars
- **Graphiti**: [Zep Graphiti](https://github.com/getzep/graphiti) - 22.9k+ stars
- **Cognee**: [GitHub](https://github.com/cognonymousai/cognee) - 2k+ stars

### Claude Code Skills

- **Skills 规范**: [agentskills.io/specification](https://agentskills.io/specification)
- **Skills vs MCP 区别**: [别搞混了！Agent Skill 和 MCP 到底有什么区别？](https://m.blog.csdn.net/m0_71746299/article/details/157431938)
- **Claude Skills 详解**: [Claude技能大揭秘：Anthropic的AI能力扩展黑科技](https://blog.csdn.net/2401_84494441/article/details/158458744)
- **CLI-offloaded MCP**: [LinkedIn Article](https://www.linkedin.com/pulse/cli-offloaded-mcp-context-engineering-hack-anthropic-guy-vago--vix1f)
- **Planning with Files Skill**: [GitHub](https://github.com/anthropics/planning-with-files) - 12k+ stars, Manus 方法论
- **Claude Code 2026 展望**: [Claude Code 发布一周年：2026年Agentic AI底层基础设施与标准化](https://blog.csdn.net/sD7O95O/article/details/158398776)

### "Context is File" 概念

- **文件系统才是 AI Agent 的长期记忆**: [稀土掘金](https://juejin.cn/post/7610580125619142682)
- **Manus 团队方案**: [火山引擎](https://developer.volcengine.com/articles/7586969696470040619)
- **AI 记忆系统大横评**: [今日头条](https://m.tou.tiao.com/a7578134834892030527/)
- **Agent Memory Paper List**: [GitHub](https://github.com/Shichun-Liu/Agent-Memory-Paper-List)
- **Everything is Context**: [51CTO](https://www.51cto.com/article/831717.html)
- **File System as Meta Tool**: [CSDN](https://m.blog.csdn.net/weixin_43749777/article/details/156836191)
- **Bash Is All Agent Need**: [今日头条](https://m.toutiao.com/a7592911811024814632/)
- **OpenClaw 会话机制**: [博客园](https://www.cnblogs.com/YzpJason/p/19631621) - 使用 MEMORY.md, SOUL.md 文件实现本地记忆

### Meta Tools 与 AI Agent 架构

- **2026: 大规模为 Agent 构建基础设施**: [阿里云开发者社区](https://developer.aliyun.com/article/1713567) - 2026年2月发布，API、数据、环境成为重心
- **MetaAgent 论文 (arXiv 2508.00271)**: [Tool Meta-Learning 自演化智能体](https://blog.csdn.net/weixin_52341477/article/details/155109848)
- **Foundation Agents 综述 (arXiv 2504.01990)**: [264页 Agent 综述](https://news.qq.com/rain/a/20250426A06QX000)
- **Cornell AI Agent 论文**: [你真的了解AI Agent吗？](https://m.blog.csdn.net/m0_59163425/article/details/148409670)
- **AI Agent 架构概况**: [2025版 AI Agent架构](https://m.blog.csdn.net/python_0011/article/details/148406641)
- **2026 Agent 元年**: [重点关注三大产业机会](https://xueqiu.com/8925920918/374524054)

### 图记忆系统

- **Graph-based Agent Memory (arXiv 2602.05665v1)**: [图记忆分类与技术](https://arxiv.org/html/2602.05665v1)
- **OpenSage (arXiv 2602.16891v1)**: [自编程 Agent 生成引擎](https://arxiv.org/html/2602.16891v1)
- **MAGMA (arXiv 2601.03236)**: [多图 Agent 记忆架构](https://arxiv.org/abs/2601.03236)
- **Memory in the Age of AI Agents (arXiv 2512.13564)**: [AI Agent 时代记忆综述](https://arxiv.org/abs/2512.13564)
- **Fork, Explore, Commit (arXiv 2602.08199v1)**: [OS 原语用于 Agent 探索](https://arxiv.org/html/2602.08199v1)
- **MIRIX (arXiv 2507.07957v1)**: [多 Agent 记忆系统](https://arxiv.org/html/2507.07957v1)
- **Continuum Memory (arXiv 2601.09913v1)**: [长周期 Agent 记忆架构](https://arxiv.org/html/2601.09913v1)
- **A-Mem (arXiv 2502.12110)**: [Agent 自主记忆](https://arxiv.org/abs/2502.12110)
- **Graphs Meet AI Agents (arXiv 2506.18019v1)**: [图方法在 AI Agent 中的应用综述](https://arxiv.org/html/2506.18019v1)
- **基于图的Agent记忆**: [CSDN](https://blog.csdn.net/shebao3333/article/details/158383142)
- **AI Agent 记忆机制综述**: [CSDN](https://m.blog.csdn.net/m0_57545130/article/details/157386487)
- **Mem0 实战**: [构建生产级 AI Agent 记忆](https://blog.csdn.net/sinat_15906013/article/details/156336689)

### 技术参考

- **Plan 9**: [Plan 9 from Bell Labs](https://9p.io/plan9/)
- **9P 协议**: [9P2000](https://9p.io/magic/man2html/5/intro)
- **AWCP 协议**: [arXiv: A Workspace Delegation Protocol](https://arxiv.org/html/2602.20493v1) (2026-02-25)
- **A2A 协议**: Google Agent-to-Agent 协议 (2025-04)
- **ACP 协议**: Agent Client Protocol (Apache)

---

## 📝 附录

### A. 术语表

- **EVIF**: Everything Is a Virtual filesystem
- **AGFS**: Abstract Graph File System / Agent File System (c4pt0r/agfs)
- **MCP**: Model Context Protocol - 工具调用协议
- **Skills**: Agent Skills - 轻量级提示词扩展规范
- **AWCP**: Agent Workspace Collaboration Protocol - 工作空间委托协议
- **RAG**: Retrieval-Augmented Generation
- **VFS**: Virtual File System
- **FUSE**: Filesystem in Userspace
- **LRU**: Least Recently Used
- **TTL**: Time To Live

### B. 架构决策记录 (ADR)

详见 `.ralph/agent/decisions.md`（需要建立）

### C. 更新日志

- **2026-03-01**: 创建 todo5.md，基于全面分析
- **2026-03-01**: 更新 - 添加 Claude Code Skills 集成分析、AGFS 竞品更新、"Context is File" 概念、AWCP 协议支持
- **2026-03-01**: 更新 - 添加 Meta Tools 理念分析、2026 年最新论文参考、生产级记忆框架对比
- **2026-03-01**: 更新 - 添加代码库深入分析、AgentFS 竞品、修正图系统状态、添加发现的问题清单
- **2026-03-01**: 更新 - 修正 Mem0/Graphiti star 数量，添加 4 篇新论文 (MIRIX, Continuum Memory, A-Mem, Graphs Meet AI Agents)
- **2026-03-01**: 更新 - 修正 Mem0 star 数量 (41k+ → 46k+)，验证代码库状态
- **2026-03-01**: 更新 - 修正 API 契约格式描述（经验证已一致，测试通过）
- 后续更新: 每个 Phase 完成时更新

---

**文档维护**: 本文档随项目进展动态更新，每个里程碑完成后进行回顾和调整。

**贡献者**: 欢迎社区反馈和建议，通过 GitHub Issues 或 Pull Requests 参与。
