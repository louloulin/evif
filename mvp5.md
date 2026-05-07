# EVIF MVP 5.0 路线图

> 创建时间：2026-05-06
> 最后更新：2026-05-06
> 目标：Production Ready - 完成技术债务清理、插件实现完善、测试覆盖提升
> 基础：MVP 4.0 (100%) + MVP 4.1 (100%) 已完成
> **状态：MVP 5.0 规划中**

---

## 执行摘要

| 维度 | 当前 | MVP 5.0 目标 |
|------|------|--------------|
| 代码质量 | ⚠️ 技术债务 | ✅ Production Ready |
| 测试覆盖 | ~70% | 85%+ |
| 插件实现 | 38% (5/13) | 85% (11/13) |
| 文档完善 | 部分 | 完整 |
| 安全 | ⚠️ 需审计 | ✅ 审计通过 |

---

## 零、CRITICAL 问题（必须在生产前修复）

### 0.1 PluginPool 内存泄漏 🔴 CRITICAL

**文件**：`crates/evif-core/src/plugin_pool.rs:296-304`

**问题**：`return_plugin()` 只减少计数器，不将插件返回到空闲池。插件使用后无法复用，导致持续重新创建和内存增长。

**影响**：长期运行时内存持续增长，最终 OOM。

**修复**：实现真正的连接池回收逻辑。

### 0.2 MCP Token 明文比较 🔴 CRITICAL

**文件**：`crates/evif-mcp/src/mcp_auth.rs`

**问题**：Token 密钥使用明文比较（`secret == hash`），注释明确标注"生产应使用 bcrypt 或 argon2"。

**影响**：凭证泄露风险。

**修复**：使用 `argon2` crate 实现密码哈希验证。

### 0.3 动态插件加载无签名验证 🔴 HIGH

**文件**：`crates/evif-core/src/dynamic_loader.rs:304-442`

**问题**：通过 `dlopen` 加载 `.so/.dylib` 文件，仅检查 ABI 版本，无密码学签名验证。搜索路径包括 `$HOME/.evif/plugins`。

**影响**：恶意插件可被加载执行。

**修复**：添加 Ed25519 签名验证或 SHA256 完整性校验。

---

## 零点五、HIGH 问题（应尽快修复）

| # | 问题 | 文件 | 影响 |
|---|------|------|------|
| 1 | SQLite `std::sync::Mutex` 阻塞 async runtime | `evif-mem/src/storage/sqlite.rs` | 高负载时性能严重下降 |
| 2 | API Key 明文存储在配置文件 | `evif-core/src/config.rs:178` | 凭证泄露 |
| 3 | 无优雅关闭协调 (no JoinSet) | `evif-rest/src/server.rs` | 请求中断 |
| 4 | Circuit Breaker 未集成外部调用 | 多个文件 | 级联故障 |
| 5 | CORS 生产环境默认允许所有来源 | `evif-rest/src/server.rs` | 跨域攻击 |

---

## 一、技术债务分析

### 1.1 代码质量现状

| 指标 | 当前状态 | 目标 |
|------|----------|------|
| `panic!()` 在非测试代码 | 20+ 处 | 0 处 |
| `unwrap()` 在非测试代码 | 100+ 处 | <5 处 |
| `expect()` 在非测试代码 | 50+ 处 | <10 处 |
| `#[allow(dead_code)]` 数量 | 100+ 处 | <20 处 |
| 无测试的源文件 | 49 个 (27%) | <15 个 |
| 公开 API 缺少文档 | 大量 | 完成核心 API 文档 |

### 1.2 错误处理详细问题

| 文件 | 问题数 | 说明 |
|------|--------|------|
| `evif-mem/src/storage/sqlite.rs` | 16 | `Mutex::lock().unwrap()` 阻塞式互斥锁 |
| `evif-plugins/src/sqlfs.rs` | 23 | `spawn_blocking` 错误映射过多 |
| `evif-mem/src/pipeline.rs` | 12 | `.expect()` 调用应返回 Result |
| `evif-core/src/plugin_pool.rs` | 4 | `.unwrap()` on std::sync::Mutex |
| `evif-mcp/src/lib.rs` | 3 | Regex `expect()` 编译失败会 panic |

### 1.2 插件实现状态

| 插件 | 实现程度 | 说明 |
|------|----------|------|
| `contextfs` | ✅ 完整 | L0/L1/L2 三层上下文 |
| `skillfs` | ✅ 完整 | SKILL.md 技能发现 |
| `pipefs` | ✅ 完整 | wait_for_result, try_claim |
| `memfs` | ✅ 完整 | 内存文件系统 |
| `vectorfs` | ✅ 完整 | 向量搜索 |
| `teamsfs` | ⚠️ Stub | 硬编码返回，无真实 API |
| `gmailfs` | ⚠️ Stub | 硬编码返回，无真实 API |
| `slackfs` | ⚠️ Stub | 硬编码返回，无真实 API |
| `discordfs` | ⚠️ Stub | 部分实现，大部分标记 dead_code |
| `telegramfs` | ⚠️ Stub | 硬编码返回，无真实 API |
| `shopifyfs` | ⚠️ Stub | 硬编码返回，无真实 API |
| `notionfs` | ⚠️ Stub | 硬编码返回 |
| `githubfs` | ⚠️ 部分 | 有结构定义，使用 stub |

---

## 一.5 测试覆盖现状 ⚠️

### 1.5.1 按 Crate 测试覆盖

| Crate | 覆盖文件 | 总文件 | 覆盖率 | 未覆盖文件 |
|-------|----------|--------|--------|------------|
| **evif-rest** | 8 | 22 | **36%** ⚠️ | middleware.rs, routes.rs, memory_handlers.rs 等 14 个 |
| **evif-bench** | 0 | 6 | **0%** ⚠️ | 全部未覆盖 |
| evif-auth | 4 | 6 | 66% | 2 个 |
| evif-cli | 7 | 10 | 70% | 3 个 |
| evif-core | 25 | 32 | 78% | 6 个 |
| evif-mem | 25 | 31 | 80% | 6 个 |
| evif-plugins | 41 | 47 | 87% | 6 个 |
| evif-mcp | 7 | 8 | 87% | 1 个 |
| **evif-client** | 3 | 3 | **100%** ✅ | 无 |

### 1.5.2 测试统计

| 指标 | 数值 |
|------|------|
| 测试总数 | ~2,488 |
| 内联测试 | 2,140 |
| 集成测试 | 338 |
| E2E 测试 | 31 |
| Error case 测试 | 仅 3 个 `#[should_panic]` |
| 属性测试 | 0 (无 proptest/quickcheck) |

### 1.5.3 测试质量缺口

| 问题 | 当前 | 目标 |
|------|------|------|
| 错误路径测试 | 1:8 (错误:正常) | 1:3 |
| 属性测试 | 0 | >10 |
| 模糊测试 | 0 | >5 |
| 睡眠等待 | 131 处 | <20 |
| 覆盖率工具 | 无 | cargo-llvm-cov |

---

## 二、依赖版本现状

### 2.1 需要更新的依赖

| 包名 | 当前版本 | 最新版本 | 优先级 |
|------|----------|----------|--------|
| tokio | 1.35 | 1.40+ | 中 |
| serde | 1.0.196 | 1.0.210+ | 中 |
| anyhow | 1.0.79 | 1.0.86+ | 低 |
| thiserror | 1.0.56 | 1.0.61+ | 低 |
| axum | 0.7.4 | 0.7.x | 中 |
| jsonwebtoken | 9.0.x | 9.3.x | 中 |

### 2.2 版本冲突问题

| 问题 | 影响 | 解决方案 |
|------|------|----------|
| rusqlite vs rusqlite2 | 双版本共存 | 统一选择一个 |
| wasmtime 27 vs 36 | 版本不匹配 | 统一到最新版本 |
| 多个 serde 版本 | 潜在冲突 | 检查 Cargo.lock |

---

## 三、生产就绪检查清单

### 3.1 错误处理完整性 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| Result 返回值一致性 | 分散在各个 crate | 高 |
| EvifError 定义完整 | 部分实现 | 高 |
| 错误日志记录 | 部分覆盖 | 中 |
| 错误恢复机制 | 缺失 | 高 |

**需要修复**：
- `evif-plugins/src/skill_runtime.rs` - 错误应返回 Result
- `evif-plugins/src/postgresfs.rs` - 路径解析失败应返回错误
- `evif-plugins/src/skillfs.rs` - match arms 应返回 Result

### 3.2 并发安全 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| Mutex/RwLock 使用模式 | ⚠️ 混用 std/tokio/parking_lot | 高 |
| PluginPool 内存泄漏 | 🔴 `return_plugin()` 不返回池 | 严重 |
| Send/Sync trait bounds | 部分验证 | 中 |
| 潜在死锁 | 未检查 | 高 |
| Async runtime 兼容性 | ⚠️ SQLite 用 std::Mutex | 高 |

### 3.3 资源管理 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| 连接池大小配置 | 硬编码 | 中 |
| 文件句柄泄漏 | 未检测 | 高 |
| 内存泄漏（长运行） | 🔴 PluginPool 不回收 | 严重 |
| 关闭时清理 | ⚠️ 无 JoinSet 协调 | 中 |

### 3.4 输入验证 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| 路径遍历防护 | 部分实现 | 高 |
| SQL 注入防护 | N/A (使用 ORM) | 低 |
| API 输入验证 | 部分实现 | 高 |
| 限流实现 | 已实现 | 中 |

### 3.5 可观测性 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| 错误日志 | 部分覆盖 | 高 |
| Tracing spans | 需审计 | 中 |
| Prometheus metrics | 已实现 | 中 |
| 健康检查端点 | 已有 | 低 |

---

## 四、安全审计清单

### 4.1 认证与授权 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| Auth 实现完整性 | 部分实现 | 高 |
| RBAC/权限系统 | 需审计 | 高 |
| API Key 管理 | 需审查 | 高 |
| OAuth 实现 | stub | 中 |

### 4.2 输入安全 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| 路径遍历防护 | 部分 | 高 |
| SQL 注入 | N/A | 低 |
| 命令注入 | 无风险 | 低 |
| Buffer overflow | Rust 安全 | 低 |

### 4.3 秘密管理 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| API Key 处理 | 通过配置 | 中 |
| Token 存储 | 需审计 | 高 |
| 环境变量使用 | 需审查 | 中 |
| 硬编码密钥 | 无发现 | 低 |

### 4.4 网络安全 ⚠️

| 检查项 | 当前状态 | 优先级 |
|--------|----------|--------|
| TLS/HTTPS 配置 | 未强制 | 高 |
| CORS 设置 | 未设置 | 中 |
| WebSocket 安全 | 需审查 | 中 |
| 限流 | 已实现 | 低 |

### 4.5 Unsafe 代码 ⚠️

| 文件 | 位置 | 风险 |
|------|------|------|
| `dynamic_loader.rs` | 304, 353, 370, 418, 442 | 中 |
| `example-dynamic-plugin/src/lib.rs` | 214 | 中 |

---

## 五、Priority 工作项

### P0（必须完成）

#### 5.1 消除 panic!() 在非测试代码

**目标**：将所有非测试代码中的 `panic!()` 替换为正确的错误处理。

**需要修复的文件**：

| 文件 | 行数 | 类型 |
|------|------|------|
| `evif-plugins/src/skill_runtime.rs` | 875, 922 | `panic!("Unexpected error: {:?}", e)` |
| `evif-plugins/src/postgresfs.rs` | 520 | `panic!("Failed to parse path")` |
| `evif-plugins/src/skillfs.rs` | 863, 883, 902, 922, 940, 960, 998 | match arms 中的 panic |

**修复策略**：
- 返回 `EvifResult<T>` 或 `Result<T, EvifError>`
- 使用 `?` 操作符传播错误
- 添加适当的错误日志

#### 5.2 减少 unwrap() 在非测试代码

**目标**：将关键路径上的 `.unwrap()` 替换为错误处理。

**高优先级文件**：
- `evif-mem/src/embedding.rs` (4 处)
- `evif-mem/src/pipeline.rs` (18 处)
- `evif-core/src/dynamic_loader.rs` (5 处)

#### 5.3 安全加固

| 任务 | 优先级 | 说明 |
|------|--------|------|
| TLS/HTTPS 强制 | P0 | 强制 HTTPS |
| 输入验证增强 | P0 | 所有 API 输入 |
| 限流增强 | P0 | 防止 DDoS |

---

### P1（重要）

#### 5.4 减少 dead_code 抑制

**目标**：删除未使用的代码或添加真正的实现。

**高优先级文件**：
| 文件 | dead_code 数量 | 行动 |
|------|-----------------|------|
| `teamsfs.rs` | 18 | 实现或删除 |
| `shopifyfs.rs` | 18 | 实现或删除 |
| `telegramfs.rs` | 13 | 实现或删除 |
| `gmailfs.rs` | 11 | 实现或删除 |
| `slackfs.rs` | 1 | 实现或删除 |
| `discordfs.rs` | 4 | 实现或删除 |

#### 5.5 实现插件 API

**目标**：完成以下插件的真实 API 实现，替换 stub。

##### TeamsFS 实现
```
当前：硬编码返回
目标：Microsoft Graph API 集成
需要：OAuth 2.0 认证、Teams API 调用
文件：crates/evif-plugins/src/teamsfs.rs
```

##### GmailFS 实现
```
当前：硬编码返回
目标：Gmail API v1 集成
需要：OAuth 2.0 认证、邮件 CRUD 操作
文件：crates/evif-plugins/src/gmailfs.rs
```

##### SlackFS 实现
```
当前：硬编码返回
目标：Slack Web API 集成
需要：Bot Token、消息/反应操作
文件：crates/evif-plugins/src/slackfs.rs
```

##### DiscordFS 实现
```
当前：部分实现
目标：Discord REST API 完整集成
需要：Bot Token、消息/Embed/反应
文件：crates/evif-plugins/src/discordfs.rs
```

##### TelegramFS 实现
```
当前：硬编码返回
目标：Telegram Bot API 集成
需要：Bot Token、消息/命令
文件：crates/evif-plugins/src/telegramfs.rs
```

##### ShopifyFS 实现
```
当前：硬编码返回
目标：Shopify Admin API 集成
需要：API Key、订单/产品操作
文件：crates/evif-plugins/src/shopifyfs.rs
```

#### 5.6 添加测试覆盖

**目标**：为缺少测试的核心文件添加单元测试。

**高优先级文件**（49 个无测试文件的子集）：

| 文件 | 重要性 | 测试数量目标 |
|------|--------|--------------|
| `evif-mem/src/llm.rs` | 核心 | 20+ |
| `evif-mem/src/embedding.rs` | 核心 | 15+ |
| `evif-mem/src/pipeline.rs` | 核心 | 30+ |
| `evif-plugins/src/pipefs.rs` | 核心 | 10+ |
| `evif-plugins/src/contextfs.rs` | 核心 | 15+ |
| `evif-core/src/dynamic_loader.rs` | 重要 | 10+ |
| `evif-rest/src/handlers.rs` | 重要 | 20+ |

---

### P2（增强）

#### 5.7 添加公开 API 文档

**目标**：为所有公开的 crate 公共 API 添加文档注释。

**优先级顺序**：
1. `evif-core` - 核心抽象
2. `evif-plugins` - 插件 Trait
3. `evif-mem` - 内存系统
4. `evif-rest` - REST API 类型
5. `evif-mcp` - MCP 工具定义

#### 5.8 更新依赖版本

**目标**：更新过时依赖并修复版本冲突。

**操作步骤**：
1. 检查 Cargo.lock 中的所有版本
2. 更新 tokio, serde, axum 等核心依赖
3. 解决 rusqlite vs rusqlite2 冲突
4. 统一 wasmtime 版本

#### 5.9 启用跳过的测试

**目标**：为需要外部服务的测试创建测试基础设施。

**需要的基础设施**：
- PostgreSQL 测试容器
- Qdrant 测试容器
- Prometheus 指标测试

#### 5.10 并发安全审查

| 任务 | 优先级 | 说明 |
|------|--------|------|
| 死锁检测 | P2 | 运行 stress 测试 |
| Send/Sync 验证 | P2 | 所有公共类型 |
| Async 边界 | P2 | 确保正确 |

---

### P3（优化）

#### 5.11 审查 unsafe 代码

**目标**：审查并确保所有 unsafe 代码的安全性。

**文件**：
- `evif-core/src/dynamic_loader.rs` (5 处)
- `example-dynamic-plugin/src/lib.rs` (1 处)

#### 5.12 可观测性增强

| 任务 | 优先级 | 说明 |
|------|--------|------|
| Tracing spans | P3 | 分布式追踪 |
| 结构化日志 | P3 | JSON 格式 |
| 告警规则 | P3 | Prometheus |

#### 5.13 性能优化

| 任务 | 优先级 | 说明 |
|------|--------|------|
| Profiling | P3 | 识别瓶颈 |
| 连接池调优 | P3 | 资源利用 |
| 缓存优化 | P3 | 减少延迟 |

---

## 六、实施计划

### Phase 1: CRITICAL 修复（1 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1 | PluginPool 内存泄漏修复 | `return_plugin()` 真正返回池 |
| Day 2 | MCP Token 安全加固 | argon2 密码哈希 |
| Day 3 | 动态插件签名验证 | Ed25519/SHA256 校验 |
| Day 4 | SQLite async 兼容 | `spawn_blocking` 或连接池 |
| Day 5 | 优雅关闭 | `JoinSet` + 取消协调 |

### Phase 2: 技术债务清理（2 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | 消除 panic!() | skill_runtime.rs, postgresfs.rs, skillfs.rs |
| Day 3-4 | 减少 unwrap()/expect() | embedding.rs, pipeline.rs, dynamic_loader.rs |
| Day 5-7 | 安全加固 | TLS 强制, CORS 限制, 输入验证 |
| Day 8-10 | 添加核心测试 | llm.rs, embedding.rs, pipeline.rs |
| Day 11-14 | 依赖更新 | 解决版本冲突 |

### Phase 2: 插件实现（2 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-3 | TeamsFS 实现 | Microsoft Graph API 集成 |
| Day 4-6 | GmailFS 实现 | Gmail REST API v1 集成 |
| Day 7-9 | SlackFS 实现 | Slack Web API 集成 |
| Day 10-12 | DiscordFS 实现 | Discord REST API 集成 |
| Day 13-14 | 测试补全 | 各插件单元测试 |

### Phase 3: 测试与安全（1 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | API 文档 | 核心 crate 公开 API 文档 |
| Day 3-4 | 安全审计 | 无硬编码密钥, TLS 强制 |
| Day 5 | 并发安全 | 死锁检测, Send/Sync |
| Day 6-7 | 测试启用 | 外部服务测试基础设施 |

### Phase 4: 生产准备（1 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | 可观测性 | Tracing, 结构化日志 |
| Day 3-4 | 性能测试 | Profiling, 调优 |
| Day 5-7 | 发布准备 | Docker, CI/CD |

---

## 七、成功指标

| 指标 | 当前 | MVP 5.0 目标 |
|------|------|--------------|
| `panic!()` 在非测试代码 | 20+ | 0 |
| `unwrap()` 在非测试代码 | 40+ | <5 |
| `#[allow(dead_code)]` | 100+ | <20 |
| 无测试的源文件 | 49 | <15 |
| 插件 API 实现 | 5/13 (38%) | 11/13 (85%) |
| 依赖版本过期 | 6 | 0 |
| 测试覆盖率 | ~70% | 85%+ |
| 无测试源文件 | 45 (23%) | <10 |
| Error case 测试 | 仅 3 个 | >50 |
| 属性测试 | 0 | >10 |
| 安全漏洞 | 未知 | 0 高危 |
| TLS 强制 | 否 | 是 |
| 输入验证覆盖 | 部分 | 100% |

---

## 八、关键里程碑

| 里程碑 | 完成标准 | 目标日期 |
|--------|----------|----------|
| **MVP 5.0 Alpha** | 技术债务清理 + 安全加固 | 2026-05-20 |
| **MVP 5.0 Beta** | 插件 API 实现 80% | 2026-05-27 |
| **MVP 5.0 RC** | 测试覆盖率 85%+ | 2026-06-03 |
| **MVP 5.0 Release** | 生产就绪 | 2026-06-10 |

---

## 九、参考资源

### API 文档

| 资源 | 说明 |
|------|------|
| Microsoft Graph API | learn.microsoft.com/graph/api/overview |
| Gmail API | developers.google.com/gmail/api/reference/rest |
| Slack Web API | api.slack.com/web |
| Discord REST API | discord.com/developers/docs/resources |
| Telegram Bot API | core.telegram.org/bots/api |
| Shopify Admin API | shopify.dev/docs/admin-api |

### Rust 最佳实践

| 资源 | 说明 |
|------|------|
| Error handling | doc.rust-lang.org/error_codes |
| Async runtime | tokio.rs |
| Unsafe code | rust-lang.github.io/unsafe-code-guidelines |

---

## 十、技术债务详细清单

### 10.1 panic!() 位置明细

```
evif-plugins/src/skill_runtime.rs:875
evif-plugins/src/skill_runtime.rs:922
evif-plugins/src/postgresfs.rs:520
evif-plugins/src/skillfs.rs:863
evif-plugins/src/skillfs.rs:883
evif-plugins/src/skillfs.rs:902
evif-plugins/src/skillfs.rs:922
evif-plugins/src/skillfs.rs:940
evif-plugins/src/skillfs.rs:960
evif-plugins/src/skillfs.rs:998
```

### 10.2 dead_code 分布

```
evif-plugins/src/teamsfs.rs: 18 处
evif-plugins/src/shopifyfs.rs: 18 处
evif-plugins/src/telegramfs.rs: 13 处
evif-plugins/src/gmailfs.rs: 11 处
evif-plugins/src/discordfs.rs: 4 处
evif-plugins/src/encryptedfs.rs: 2 处
evif-plugins/src/notionfs.rs: 1 处
evif-plugins/src/slackfs.rs: 1 处
evif-plugins/src/githubfs.rs: 1 处
evif-plugins/src/s3fs_opendal.rs: 1 处
evif-mem/src/*.rs: 12 处
evif-cli/src/*.rs: 5 处
evif-auth/src/*.rs: 2 处
evif-core/src/*.rs: 4 处
evif-mcp/src/*.rs: 2 处
总计: 83+ 处
```

### 10.3 未实现插件 API

```
/teams - Microsoft Teams (stub)
/gmail - Gmail (stub)
/slack - Slack (stub)
/discord - Discord (partial)
/telegram - Telegram (stub)
/shopify - Shopify (stub)
/notion - Notion (stub)
```

### 10.4 unsafe 代码位置

```
evif-core/src/dynamic_loader.rs:304
evif-core/src/dynamic_loader.rs:353
evif-core/src/dynamic_loader.rs:370
evif-core/src/dynamic_loader.rs:418
evif-core/src/dynamic_loader.rs:442
example-dynamic-plugin/src/lib.rs:214
总计: 6 处
```

---

## 十一、生产部署检查清单

### 11.1 必需项 (Production Must-Have)

- [ ] 所有 panic!() 替换为错误处理
- [ ] TLS/HTTPS 强制启用
- [ ] API 输入验证 100% 覆盖
- [ ] Rate limiting 配置
- [ ] 日志结构化
- [ ] 健康检查端点
- [ ] Prometheus metrics
- [ ] 优雅关闭处理
- [ ] 连接池配置
- [ ] 超时配置

### 11.2 推荐项 (Production Recommended)

- [ ] Distributed tracing (OpenTelemetry)
- [ ] Error tracking (Sentry)
- [ ] Log aggregation (Loki/ELK)
- [ ] Metrics dashboard (Grafana)
- [ ] Alert rules
- [ ] Runbook documentation

### 11.3 可选项 (Production Nice-to-Have)

- [ ] Chaos engineering (Litmus)
- [ ] Load testing (k6)
- [ ] Security scanning (Trivy)
- [ ] Dependency audit (cargo-audit)

---

*MVP 5.0 重点：Production Ready - 代码质量、安全、可观测性、测试覆盖全面达标*