# EVIF MVP 5.0 路线图

> 创建时间：2026-05-06
> 最后更新：2026-05-06
> 目标：Production Ready - 完成技术债务清理、插件实现完善、测试覆盖提升
> 基础：MVP 4.0 (100%) + MVP 4.1 (100%) 已完成
> **状态：MVP 5.0 规划中**

---

## 一、技术债务分析

### 1.1 代码质量现状

| 指标 | 当前状态 | 目标 |
|------|----------|------|
| `panic!()` 在非测试代码 | 20+ 处 | 0 处 |
| `unwrap()` 在非测试代码 | 40+ 处 | <5 处 |
| `#[allow(dead_code)]` 数量 | 100+ 处 | <20 处 |
| 无测试的源文件 | 49 个 (27%) | <15 个 |
| 公开 API 缺少文档 | 大量 | 完成核心 API 文档 |

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

## 三、Priority 工作项

### P0（必须完成）

#### 3.1 消除 panic!() 在非测试代码

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

#### 3.2 减少 unwrap() 在非测试代码

**目标**：将关键路径上的 `.unwrap()` 替换为错误处理。

**高优先级文件**：
- `evif-mem/src/embedding.rs` (4 处)
- `evif-mem/src/pipeline.rs` (18 处)
- `evif-core/src/dynamic_loader.rs` (5 处)

#### 3.3 减少 dead_code 抑制

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

---

### P1（重要）

#### 3.4 实现插件 API

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

#### 3.5 添加测试覆盖

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

#### 3.6 添加公开 API 文档

**目标**：为所有公开的 crate 公共 API 添加文档注释。

**优先级顺序**：
1. `evif-core` - 核心抽象
2. `evif-plugins` - 插件 Trait
3. `evif-mem` - 内存系统
4. `evif-rest` - REST API 类型
5. `evif-mcp` - MCP 工具定义

#### 3.7 更新依赖版本

**目标**：更新过时依赖并修复版本冲突。

**操作步骤**：
1. 检查 Cargo.lock 中的所有版本
2. 更新 tokio, serde, axum 等核心依赖
3. 解决 rusqlite vs rusqlite2 冲突
4. 统一 wasmtime 版本

#### 3.8 启用跳过的测试

**目标**：为需要外部服务的测试创建测试基础设施。

**需要的基础设施**：
- PostgreSQL 测试容器
- Qdrant 测试容器
- Prometheus 指标测试

---

### P3（优化）

#### 3.9 审查 unsafe 代码

**目标**：审查并确保所有 unsafe 代码的安全性。

**文件**：
- `evif-core/src/dynamic_loader.rs` (5 处)
- `example-dynamic-plugin/src/lib.rs` (1 处)

#### 3.10 安全审计

**目标**：确保无硬编码密钥或敏感信息。

**检查项**：
- 无硬编码 API Key
- 无硬编码密码
- 无硬编码 Token

---

## 四、实施计划

### Phase 1: 技术债务清理（2 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | 消除 panic!() | skill_runtime.rs, postgresfs.rs, skillfs.rs |
| Day 3-4 | 减少 unwrap() | embedding.rs, pipeline.rs, dynamic_loader.rs |
| Day 5-7 | 清理 dead_code | 删除未使用的结构和方法 |
| Day 8-10 | 添加核心测试 | llm.rs, embedding.rs, pipeline.rs |
| Day 11-14 | 更新依赖 | 解决版本冲突 |

### Phase 2: 插件实现（2 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-3 | TeamsFS 实现 | Microsoft Graph API 集成 |
| Day 4-6 | GmailFS 实现 | Gmail REST API v1 集成 |
| Day 7-9 | SlackFS 实现 | Slack Web API 集成 |
| Day 10-12 | DiscordFS 实现 | Discord REST API 集成 |
| Day 13-14 | 测试补全 | 各插件单元测试 |

### Phase 3: 文档与安全（1 周）

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | API 文档 | 核心 crate 公开 API 文档 |
| Day 3-4 | 安全审计 | 无硬编码密钥 |
| Day 5 | 依赖更新 | tokio, serde 等最新版本 |
| Day 6-7 | 测试启用 | 外部服务测试基础设施 |

---

## 五、成功指标

| 指标 | 当前 | MVP 5.0 目标 |
|------|------|--------------|
| `panic!()` 在非测试代码 | 20+ | 0 |
| `unwrap()` 在非测试代码 | 40+ | <5 |
| `#[allow(dead_code)]` | 100+ | <20 |
| 无测试的源文件 | 49 | <15 |
| 插件 API 实现 | 5/13 (38%) | 11/13 (85%) |
| 依赖版本过期 | 6 | 0 |
| 测试覆盖率 | ~70% | 85%+ |

---

## 六、关键里程碑

| 里程碑 | 完成标准 | 目标日期 |
|--------|----------|----------|
| **MVP 5.0 Alpha** | 技术债务清理完成 | 2026-05-20 |
| **MVP 5.0 Beta** | 插件 API 实现 80% | 2026-05-27 |
| **MVP 5.0 RC** | 测试覆盖率 85%+ | 2026-06-03 |
| **MVP 5.0 Release** | 生产就绪 | 2026-06-10 |

---

## 七、参考资源

| 资源 | 说明 |
|------|------|
| Microsoft Graph API | learn.microsoft.com/graph/api/overview |
| Gmail API | developers.google.com/gmail/api/reference/rest |
| Slack Web API | api.slack.com/web |
| Discord REST API | discord.com/developers/docs/resources |
| Telegram Bot API | core.telegram.org/bots/api |
| Shopify Admin API |shopify.dev/docs/admin-api |

---

## 八、技术债务详细清单

### 8.1 panic!() 位置明细

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

### 8.2 dead_code 分布

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

### 8.3 未实现插件 API

```
/teams - Microsoft Teams (stub)
/gmail - Gmail (stub)
/slack - Slack (stub)
/discord - Discord (partial)
/telegram - Telegram (stub)
/shopify - Shopify (stub)
/notion - Notion (stub)
```

---

*MVP 5.0 重点：Production Ready - 代码质量、测试覆盖、文档完善*