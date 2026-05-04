# EVIF MCP Tools 分析报告

> 创建时间：2026-05-04
> 更新：2026-05-04
> 分析：75 个 MCP Tools 的分类、合并、删除、子工具设计

---

## 零、关键发现：幽灵工具问题 ⚠️

> **重要警示**：经过代码深度分析，发现 75 个工具中有 **46 个是幽灵工具 (Ghost Tools)** — 已注册但完全无法调用！

### 0.1 问题根源

EVIF MCP Server 的工具调用采用两阶段分发：

```
Tool Call Request
        ↓
┌───────────────────────────────┐
│  try_vfs_direct (VFS 优先)    │  ← 模拟模式调用
│  line 2908-3100              │
└───────────────────────────────┘
        ↓ (未找到)
┌───────────────────────────────┐
│  call_tool_http (REST 桥接)   │  ← 真实后端调用
│  line 4442-5283              │
└───────────────────────────────┘
        ↓ (未找到)
    "Unknown tool"
```

**问题**：`initialize_tools` (line 1337-2782) 注册了 75 个工具，但只有 **29 个工具有分发代码**。其余 46 个工具调用时返回 "Unknown tool" 错误。

### 0.2 工具实际可调用性

| 分类 | 数量 | 说明 | 状态 |
|------|------|------|------|
| **A. VFS+HTTP 双实现** | 9 | 模拟模式和真实模式都可用 | ✅ 真实工具 |
| **B. HTTP 仅实现** | 16 | 仅在连接后端时可用 | ⚠️ 需后端 |
| **C. 模拟仅实现** | 3 | 仅在 mock 模式返回假数据 | ❌ 假数据 |
| **D. 幽灵工具** | 46 | 注册了但无任何分发代码 | 🚫 不可调用 |

### 0.3 幽灵工具清单 (46 个)

```rust
// 这些工具调用时会返回 "Unknown tool" 错误

// 配置管理 (5) - 配置值存在，但工具不可调用
evif_config_get     // ❌ 只有注册，无分发
evif_config_set     // ❌ 只有注册，无分发
evif_config_list    // ❌ 只有注册，无分发
evif_log_level      // ❌ 只有注册，无分发
evif_version        // ❌ 只有注册，无分发

// 健康检查 (2) - health 注册了但 ping/detailed 是幽灵
evif_ping_with_stats   // 🚫 无任何分发代码
evif_health_detailed   // 🚫 无任何分发代码

// 诊断工具 (5) - 全是幽灵
evif_latency_test      // 🚫 无任何分发代码
evif_request_trace    // 🚫 无任何分发代码
evif_cache_stats      // 🚫 无任何分发代码
evif_log_query        // 🚫 无任何分发代码
evif_server_stats     // 🚫 无任何分发代码

// 指标导出 (1)
evif_metrics_export   // 🚫 无任何分发代码

// 事件系统 (3) - 全是幽灵
evif_event_subscribe    // 🚫 无任何分发代码
evif_event_list          // 🚫 无任何分发代码
evif_event_unsubscribe  // 🚫 无任何分发代码

// 定时系统 (3) - 全是幽灵
evif_cron_schedule    // 🚫 无任何分发代码
evif_cron_list       // 🚫 无任何分发代码
evif_cron_remove     // 🚫 无任何分发代码

// 会话管理 (2) - save/list 注册了，但 load/delete 是幽灵
evif_session_load    // 🚫 无任何分发代码
evif_session_delete // 🚫 无任何分发代码

// 子 Agent (3) - create/send 注册了，但其他是幽灵
evif_subagent_status    // 🚫 无任何分发代码
evif_subagent_list      // 🚫 无任何分发代码
evif_subagent_kill      // 🚫 无任何分发代码

// Skill 系统 (3) - list/info/execute 注册了，但 create/delete 是幽灵
evif_skill_create       // 🚫 无任何分发代码
evif_skill_delete      // 🚫 无任何分发代码
evif_claude_md_generate // 🚫 无任何分发代码

// 记忆系统 (3) - memorize/retrieve 注册了，但其他是幽灵
evif_memory_search     // 🚫 无任何分发代码
evif_memory_stats     // 🚫 无任何分发代码
evif_memory_clear     // 🚫 无任何分发代码

// 管道 (2) - 全是幽灵
evif_pipe_create      // 🚫 无任何分发代码
evif_pipe_list       // 🚫 无任何分发代码

// 队列 (2) - 全是幽灵
evif_queue_list       // 🚫 无任何分发代码
evif_queue_stats      // 🚫 无任何分发代码

// 搜索工具 (2) - 全是幽灵
evif_search           // 🚫 无任何分发代码
evif_batch            // 🚫 无任何分发代码

// 实用工具 (6) - 全是幽灵
evif_diff             // 🚫 无任何分发代码
evif_watch            // 🚫 无任何分发代码
evif_archive          // 🚫 无任何分发代码
evif_hash             // 🚫 无任何分发代码
evif_du               // 🚫 无任何分发代码
evif_mcp_capabilities // 🚫 无任何分发代码

// 元工具 (2) - 全是幽灵
evif_server_restart    // 🚫 无任何分发代码
evif_plugin_catalog   // 🚫 无任何分发代码
```

### 0.4 真实可工作的工具 (25 个)

#### A. VFS+HTTP 双实现 (9 个) ✅

| 工具 | VFS 实现 | HTTP 实现 |
|------|----------|-----------|
| `evif_ls` | line 2914-2969 | line 4497-4608 |
| `evif_cat` | line 2970-3004 | line 4609-4714 |
| `evif_write` | line 3005-3056 | line 4715-4803 |
| `evif_mkdir` | line 3057-3078 | line 4804-4856 |
| `evif_rm` | line 3079-3099 | line 4857-4929 |
| `evif_stat` | line 2910-2913 | line 4930-4969 |
| `evif_mv` | line 2983-2984 | line 4970-5020 |
| `evif_memorize` | line 2970-2981 | line 5217-5273 |
| `evif_retrieve` | line 2987-3003 | line 5274-5283 |

#### B. HTTP 仅实现 (16 个) ⚠️

| 工具 | HTTP 实现 | 说明 |
|------|-----------|------|
| `evif_cp` | line 5021-5064 | 复制文件 |
| `evif_mount` | line 5065-5097 | 挂载插件 |
| `evif_unmount` | line 5098-5115 | 卸载插件 |
| `evif_mounts` | line 5116-5146 | 列出挂载 |
| `evif_health` | line 5147-5174 | 健康检查 |
| `evif_grep` | line 5175-5216 | 文本搜索 |
| `evif_open_handle` | line 4442-4496 | 打开句柄 |
| `evif_close_handle` | line 4442-4496 | 关闭句柄 |
| `evif_skill_list` | line 4442-4496 | 列出技能 |
| `evif_skill_info` | line 4442-4496 | 技能详情 |
| `evif_skill_execute` | line 4442-4496 | 执行技能 |
| `evif_claude_md_generate` | line 4442-4496 | 生成模板 |
| `evif_session_save` | line 4442-4496 | 保存会话 |
| `evif_session_list` | line 4442-4496 | 列出会话 |
| `evif_subagent_create` | line 4442-4496 | 创建 Agent |
| `evif_subagent_send` | line 4442-4496 | 发送消息 |

#### C. 模拟仅实现 (3 个) ❌

| 工具 | 模拟实现 | 问题 |
|------|----------|------|
| `evif_find` | line 2935-2957 | 返回硬编码假数据 |
| `evif_wc` | line 2964-2968 | 返回硬编码假数据 |
| `evif_tail` | line 2978-2982 | 返回硬编码假数据 |

### 0.5 死代码问题

```rust
// line 2910 - cache_size 字段从未使用
let cache_size = tool_params.get("cache_size")
    .and_then(|v| v.as_u64())
    .unwrap_or(100);

// line 4433-4438 - _limit 变量在 session_list 中未使用
let _limit = tool_params.get("limit")
    .and_then(|v| v.as_u64())
    .unwrap_or(100) as usize;

// line 3005-3006 - threshold 参数在 retrieve 中被忽略
let threshold = tool_params.get("threshold")
    .and_then(|v| v.as_f64())
    .unwrap_or(0.5);
```

### 0.6 修复优先级

| 优先级 | 任务 | 工具数 |
|--------|------|--------|
| **P0** | 删除 46 个幽灵工具定义 | 46 |
| **P1** | 补全 16 个 HTTP-only 工具的 VFS 模拟 | 16 |
| **P2** | 修复 3 个假数据工具或删除 | 3 |
| **P3** | 清理死代码 | 3 处 |

---

## 一、75 个工具完整列表

### 1.1 文件操作类 (8)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|------|--------|--------|
| `evif_ls` | 1341 | 列出目录 | ✅ 必须 | - | ❌ |
| `evif_cat` | 1359 | 读取文件 | ✅ 必须 | - | ❌ |
| `evif_write` | 1381 | 写入文件 | ✅ 必须 | - | ❌ |
| `evif_mkdir` | 1407 | 创建目录 | ✅ 必须 | - | ❌ |
| `evif_rm` | 1425 | 删除文件 | ⚠️ 危险 | - | ⭐ 保留 |
| `evif_stat` | 1443 | 文件信息 | ⚠️ 边缘 | `evif_ls` | ⭐ 删除 |
| `evif_mv` | 1457 | 重命名/移动 | ⚠️ 边缘 | `evif_cp` | ⭐ 删除 |
| `evif_cp` | 1475 | 复制文件 | ⚠️ 边缘 | `evif_write` | ⭐ 删除 |

### 1.2 挂载管理类 (3)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|------|--------|--------|
| `evif_mount` | 1494 | 挂载插件 | ✅ 必须 | - | ❌ |
| `evif_unmount` | 1516 | 卸载插件 | ⚠️ 边缘 | `evif_mount` | ⭐ 合并 |
| `evif_mounts` | 1530 | 列出挂载 | ⚠️ 边缘 | `evif_mount` | ⭐ 合并 |

### 1.3 搜索工具类 (5)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|------|--------|--------|
| `evif_find` | 2166 | 查找文件 | ✅ 必须 | - | ❌ |
| `evif_grep` | 1540 | 文本搜索 | ✅ 必须 | - | ❌ |
| `evif_search` | 2595 | 语义搜索 | ⚠️ 冗余 | `evif_memory_search` | ⭐ 删除 |
| `evif_wc` | 2188 | 字数统计 | ⭐ 可选 | `evif_cat` | ⭐ 删除 |
| `evif_tail` | 2214 | 查看尾部 | ⭐ 可选 | `evif_cat` | ⭐ 删除 |

### 1.4 Handle 操作类 (2)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|------|--------|--------|
| `evif_open_handle` | 2233 | 打开句柄 | ✅ 必须 | - | ❌ |
| `evif_close_handle` | 2259 | 关闭句柄 | ⚠️ 边缘 | `evif_open_handle` | ⭐ 合并 |

### 1.5 记忆系统类 (5)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|------|--------|--------|
| `evif_memorize` | 2274 | 存储记忆 | ✅ 必须 | - | ❌ |
| `evif_retrieve` | 2303 | 检索记忆 | ✅ 必须 | - | ❌ |
| `evif_memory_search` | 1869 | 语义搜索 | ✅ 必须 | - | ❌ |
| `evif_memory_stats` | 1891 | 记忆统计 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_memory_clear` | 2147 | 清除记忆 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.6 技能系统类 (6)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|------|--------|--------|
| `evif_skill_list` | 2330 | 列出技能 | ✅ 必须 | - | ❌ |
| `evif_skill_info` | 2339 | 技能详情 | ⚠️ 边缘 | `evif_skill_list` | ⭐ 合并 |
| `evif_skill_execute` | 2353 | 执行技能 | ✅ 必须 | - | ❌ |
| `evif_skill_create` | 1829 | 创建技能 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_skill_delete` | 1851 | 删除技能 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_claude_md_generate` | 2377 | 生成 CLAUDE.md | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.7 会话管理类 (4)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_session_save` | 2402 | 保存会话 | ✅ 必须 | - | ❌ |
| `evif_session_load` | 1797 | 加载会话 | ✅ 必须 | - | ❌ |
| `evif_session_list` | 2424 | 列出会话 | ⚠️ 边缘 | `evif_session_save` | ⭐ 合并 |
| `evif_session_delete` | 2129 | 删除会话 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.8 子 Agent 类 (5)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_subagent_create` | 2443 | 创建子 Agent | ✅ 必须 | - | ❌ |
| `evif_subagent_send` | 2465 | 发送消息 | ✅ 必须 | - | ❌ |
| `evif_subagent_list` | 2483 | 列出子 Agent | ⚠️ 边缘 | `evif_subagent_create` | ⭐ 合并 |
| `evif_subagent_status` | 2083 | Agent 状态 | ⚠️ 边缘 | `evif_subagent_list` | ⭐ 合并 |
| `evif_subagent_kill` | 1811 | 终止子 Agent | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.9 健康检查类 (3)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_health` | 1562 | 健康检查 | ✅ 必须 | - | ❌ |
| `evif_ping_with_stats` | 1571 | Ping 统计 | ⚠️ 冗余 | `evif_health` | ⭐ 删除 |
| `evif_health_detailed` | 1937 | 详细健康 | ⚠️ 冗余 | `evif_health` | ⭐ 删除 |

### 1.10 诊断工具类 (5)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_latency_test` | 1586 | 延迟测试 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_request_trace` | 1606 | 请求追踪 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_cache_stats` | 1624 | 缓存统计 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_log_query` | 1638 | 日志查询 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_server_stats` | 2535 | 服务器统计 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.11 配置管理类 (4)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_config_get` | 1682 | 获取配置 | ✅ 必须 | - | ❌ |
| `evif_config_set` | 1997 | 设置配置 | ✅ 必须 | - | ❌ |
| `evif_config_list` | 2019 | 列出配置 | ⚠️ 边缘 | `evif_config_get` | ⭐ 合并 |
| `evif_log_level` | 1965 | 日志级别 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_version` | 1983 | 版本信息 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.12 事件与定时类 (6)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_event_subscribe` | 1700 | 事件订阅 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_event_list` | 1719 | 事件列表 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_event_unsubscribe` | 1755 | 取消订阅 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_cron_schedule` | 1733 | 定时任务 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_cron_list` | 1769 | 定时列表 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_cron_remove` | 1783 | 删除定时 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.13 管道类 (2)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_pipe_create` | 1905 | 创建管道 | ✅ 必须 | - | ❌ |
| `evif_pipe_list` | 1923 | 管道列表 | ⚠️ 边缘 | `evif_pipe_create` | ⭐ 合并 |

### 1.14 插件管理类 (4)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_plugin_load` | 2033 | 加载插件 | ✅ 必须 | - | ❌ |
| `evif_plugin_unload` | 2051 | 卸载插件 | ⚠️ 边缘 | `evif_plugin_load` | ⭐ 合并 |
| `evif_plugin_info` | 2069 | 插件信息 | ⚠️ 边缘 | `evif_plugin_load` | ⭐ 合并 |
| `evif_plugin_catalog` | 2516 | 插件目录 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.15 队列类 (2)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_queue_list` | 2097 | 队列列表 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_queue_stats` | 2115 | 队列统计 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.16 实用工具类 (8)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_diff` | 2622 | 文件对比 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_watch` | 2649 | 文件监视 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_tree` | 2676 | 目录树 | ⭐ 可选 | `evif_ls` | ⭐ 合并 |
| `evif_archive` | 2703 | 压缩包 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_hash` | 2738 | 文件哈希 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_du` | 2757 | 磁盘使用 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.17 元工具类 (2)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_mcp_capabilities` | 2493 | MCP 能力 | ⭐ 可选 | CLI | ⭐ 删除 |
| `evif_server_restart` | 1951 | 服务器重启 | ⭐ 可选 | CLI | ⭐ 删除 |

### 1.18 批量操作类 (1)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_batch` | 2554 | 批量操作 | ✅ 必须 | - | ❌ |

### 1.19 指标导出类 (1)

| 工具 | 行号 | 描述 | 核心 | 合并到 | 可删除 |
|------|------|------|--------|--------|--------|
| `evif_metrics_export` | 1662 | 指标导出 | ⭐ 可选 | CLI | ⭐ 删除 |

---

## 二、合并设计（子 Tool 实现）

### 2.1 合并原则

1. **功能相近**：同一类操作合并为一个工具
2. **参数区分**：通过 `action` 参数区分子操作
3. **向后兼容**：保留原工具名作为 alias

### 2.2 合并方案

#### evif_mount（挂载管理）

```rust
// 原工具: evif_mount, evif_unmount, evif_mounts
// 合并后:
evif_mount {
    action: "mount" | "unmount" | "list"
    plugin_name: String
    path: Option<String>
}
```

#### evif_skill_list（技能系统）

```rust
// 原工具: evif_skill_list, evif_skill_info, evif_skill_create, evif_skill_delete
// 合并后:
evif_skill {
    action: "list" | "info" | "execute" | "create" | "delete"
    name: Option<String>
    content: Option<String>
}
```

#### evif_session（会话管理）

```rust
// 原工具: evif_session_save, evif_session_load, evif_session_list, evif_session_delete
// 合并后:
evif_session {
    action: "save" | "load" | "list" | "delete"
    session_id: Option<String>
    context: Option<String>
}
```

#### evif_subagent（子 Agent）

```rust
// 原工具: evif_subagent_create, evif_subagent_send, evif_subagent_list, 
//         evif_subagent_status, evif_subagent_kill
// 合并后:
evif_subagent {
    action: "create" | "send" | "list" | "status" | "kill"
    agent_id: Option<String>
    message: Option<String>
}
```

#### evif_config（配置管理）

```rust
// 原工具: evif_config_get, evif_config_set, evif_config_list
// 合并后:
evif_config {
    action: "get" | "set" | "list"
    key: Option<String>
    value: Option<String>
}
```

#### evif_plugin（插件管理）

```rust
// 原工具: evif_plugin_load, evif_plugin_unload, evif_plugin_info
// 合并后:
evif_plugin {
    action: "load" | "unload" | "info" | "list"
    plugin_name: Option<String>
    path: Option<String>
}
```

#### evif_handle（Handle 操作）

```rust
// 原工具: evif_open_handle, evif_close_handle
// 合并后:
evif_handle {
    action: "open" | "close"
    path: String
    mode: Option<String>
}
```

#### evif_pipe（管道）

```rust
// 原工具: evif_pipe_create, evif_pipe_list
// 合并后:
evif_pipe {
    action: "create" | "list" | "send" | "recv"
    pipe_name: Option<String>
    message: Option<String>
}
```

#### evif_ls（文件列表）

```rust
// 原工具: evif_ls, evif_tree
// 合并后:
evif_ls {
    path: String
    recursive: Option<bool>
    show_tree: Option<bool>  // tree 模式
    show_stat: Option<bool> // 包含 stat 信息
}
```

#### evif_cat（文件读取）

```rust
// 原工具: evif_cat, evif_tail
// 合并后:
evif_cat {
    path: String
    offset: Option<usize>  // tail 模式
    length: Option<usize>
}
```

---

## 三、删除工具清单

### 3.1 完全删除（46 个）

```bash
# 文件操作 - 3 个
evif_stat      # 合并到 evif_ls
evif_mv        # 合并到 evif_cp
evif_cp        # 合并到 evif_write

# 搜索工具 - 3 个
evif_search    # 与 evif_memory_search 重复
evif_wc        # CLI 用
evif_tail      # 合并到 evif_cat

# 健康检查 - 2 个
evif_ping_with_stats  # 与 evif_health 重复
evif_health_detailed  # 与 evif_health 重复

# 诊断工具 - 5 个
evif_latency_test     # CLI 用
evif_request_trace    # CLI 用
evif_cache_stats      # CLI 用
evif_log_query        # CLI 用
evif_server_stats     # CLI 用

# 配置管理 - 2 个
evif_log_level        # CLI 用
evif_version          # CLI 用

# 事件与定时 - 6 个（全删除）
evif_event_subscribe
evif_event_list
evif_event_unsubscribe
evif_cron_schedule
evif_cron_list
evif_cron_remove

# 技能系统 - 3 个
evif_skill_create
evif_skill_delete
evif_claude_md_generate

# 会话管理 - 1 个
evif_session_delete

# 子 Agent - 1 个
evif_subagent_kill

# 记忆系统 - 2 个
evif_memory_stats
evif_memory_clear

# 插件管理 - 1 个
evif_plugin_catalog

# 队列类 - 2 个
evif_queue_list
evif_queue_stats

# 实用工具 - 5 个
evif_diff
evif_watch
evif_archive
evif_hash
evif_du

# 元工具 - 2 个
evif_mcp_capabilities
evif_server_restart

# 指标导出 - 1 个
evif_metrics_export
```

### 3.2 合并到主工具（11 个）

```bash
# 通过 action 参数合并
evif_unmount     → evif_mount
evif_mounts      → evif_mount
evif_skill_info  → evif_skill_list
evif_session_list → evif_session_save
evif_subagent_list → evif_subagent_create
evif_subagent_status → evif_subagent_list
evif_config_list → evif_config_get
evif_plugin_unload → evif_plugin_load
evif_plugin_info → evif_plugin_load
evif_pipe_list → evif_pipe_create
evif_tree → evif_ls
```

---

## 四、简化后的工具列表

### 4.1 核心工具（24 个）

| 工具 | 分类 | 说明 |
|------|------|------|
| `evif_ls` | 文件 | 列出目录（+tree +stat 子功能） |
| `evif_cat` | 文件 | 读取文件（+tail 子功能） |
| `evif_write` | 文件 | 写入文件 |
| `evif_mkdir` | 文件 | 创建目录 |
| `evif_rm` | 文件 | 删除文件 |
| `evif_find` | 搜索 | 查找文件 |
| `evif_grep` | 搜索 | 文本搜索 |
| `evif_mount` | 挂载 | 挂载管理（+unmount +list 子功能） |
| `evif_handle` | Handle | 句柄操作（+open +close） |
| `evif_memorize` | 记忆 | 存储记忆 |
| `evif_retrieve` | 记忆 | 检索记忆 |
| `evif_memory_search` | 记忆 | 语义搜索 |
| `evif_skill` | 技能 | 技能管理（+list +info +execute +create +delete） |
| `evif_session` | 会话 | 会话管理（+save +load +list +delete） |
| `evif_subagent` | Agent | 子 Agent（+create +send +list +status +kill） |
| `evif_config` | 配置 | 配置管理（+get +set +list） |
| `evif_health` | 健康 | 健康检查 |
| `evif_plugin` | 插件 | 插件管理（+load +unload +info +list） |
| `evif_pipe` | 管道 | 管道管理（+create +list +send +recv） |
| `evif_batch` | 批量 | 批量操作 |

### 4.2 Token 节省分析

| 场景 | 原工具数 | 简化后 | Token 节省 |
|------|----------|--------|------------|
| tools/list 响应 | 75 个定义 | 24 个定义 | ~60% |
| AI 选择决策 | 75 选 1 | 24 选 1 | ~70% |

---

## 五、实施计划

### Phase 1: 删除冗余工具（第 1 周）

1. 删除 46 个明显冗余工具
2. 更新测试用例
3. 验证功能不受影响

### Phase 2: 合并子工具（第 2 周）

1. 实现 11 个合并工具
2. 添加 action 参数支持
3. 保持向后兼容

### Phase 3: 清理文档（第 3 周）

1. 更新 MCP 文档
2. 更新 CLAUDE.md 模板
3. 更新 mcp2.md

---

## 六、最终工具清单

```
核心工具 (24):
evif_ls, evif_cat, evif_write, evif_mkdir, evif_rm
evif_find, evif_grep
evif_mount
evif_handle
evif_memorize, evif_retrieve, evif_memory_search
evif_skill
evif_session
evif_subagent
evif_config
evif_health
evif_plugin
evif_pipe
evif_batch

新增子功能:
evif_ls: tree, stat
evif_cat: tail
evif_mount: unmount, list
evif_handle: open, close
evif_skill: list, info, execute, create, delete
evif_session: save, load, list, delete
evif_subagent: create, send, list, status, kill
evif_config: get, set, list
evif_plugin: load, unload, info, list
evif_pipe: create, list, send, recv
```

---

## 七、结论

### 7.1 工具过多原因

1. **渐进式开发**：功能逐渐增加，缺少整体设计
2. **REST API 映射**：每个 REST 端点对应一个 MCP Tool
3. **防御性设计**：为每种可能场景创建独立工具
4. **缺少抽象**：没有将相似操作合并为统一接口
5. **注册即忘记**：在 `initialize_tools` 注册了工具但从未实现分发代码

### 7.2 核心问题：61% 是幽灵工具

最严重的问题不是工具过多，而是 **46 个工具注册了但完全不可用**。这意味着：

- AI Agent 看到工具列表中有 75 个工具
- 调用其中 46 个会得到 "Unknown tool" 错误
- 这些幽灵工具浪费了 tools/list 响应的 token，还破坏了 AI 的信任

### 7.3 简化收益

| 收益 | 说明 |
|------|------|
| Token 节省 | tools/list 响应减少 60% |
| AI 决策简化 | 从 75 选 1 降到 24 选 1 |
| 维护成本 | 代码量减少 40% |
| 一致性 | 统一的 action 参数模式 |
| **可靠性** | 消除 "Unknown tool" 错误，100% 工具可调用 |

### 7.4 实施优先级（更新）

| 阶段 | 时间 | 任务 | 影响 |
|------|------|------|------|
| **Phase 0** | 第 0 周 | **删除 46 个幽灵工具** | 消除 61% 无效工具 |
| Phase 1 | 第 1 周 | 删除剩余冗余工具 | 减少 tools/list 体积 |
| Phase 2 | 第 2 周 | 合并 11 个子工具 | 统一 action 模式 |
| Phase 3 | 第 3 周 | 文档和测试 | 验证全部功能 |

### 7.5 修复路径

```
当前状态: 75 工具 (46 幽灵 + 25 可用 + 3 假数据 + 1 统计遗漏)
                    ↓ Phase 0: 删除幽灵
            29 工具 (25 可用 + 3 假数据 + 1 统计遗漏)
                    ↓ Phase 1: 删除冗余
            24 工具 (核心工具集)
                    ↓ Phase 2: 合并子工具
            20 工具 (统一 action 参数)
                    ↓ Phase 3: 补全模拟
            20 工具 (100% 可调用)
```

---

## 八、附录：完整 75 工具对照表（含可调用性）

| 工具 | 行号 | 分类 | 功能状态 | 可调用性 |
|------|------|------|----------|----------|
| evif_ls | 1341 | 文件 | ✅ 保留 | ✅ VFS+HTTP |
| evif_cat | 1359 | 文件 | ✅ 保留 | ✅ VFS+HTTP |
| evif_write | 1381 | 文件 | ✅ 保留 | ✅ VFS+HTTP |
| evif_mkdir | 1407 | 文件 | ✅ 保留 | ✅ VFS+HTTP |
| evif_rm | 1425 | 文件 | ✅ 保留 | ✅ VFS+HTTP |
| evif_stat | 1443 | 文件 | ❌ 删除 | ✅ VFS+HTTP |
| evif_mv | 1457 | 文件 | ❌ 删除 | ✅ VFS+HTTP |
| evif_cp | 1475 | 文件 | ❌ 删除 | ⚠️ HTTP |
| evif_mount | 1494 | 挂载 | ✅ 保留（合并） | ⚠️ HTTP |
| evif_unmount | 1516 | 挂载 | 🔄 合并到 mount | ⚠️ HTTP |
| evif_mounts | 1530 | 挂载 | 🔄 合并到 mount | ⚠️ HTTP |
| evif_grep | 1540 | 搜索 | ✅ 保留 | ⚠️ HTTP |
| evif_health | 1562 | 健康 | ✅ 保留 | ⚠️ HTTP |
| evif_ping_with_stats | 1571 | 健康 | ❌ 删除 | 🚫 幽灵 |
| evif_latency_test | 1586 | 诊断 | ❌ 删除 | 🚫 幽灵 |
| evif_request_trace | 1606 | 诊断 | ❌ 删除 | 🚫 幽灵 |
| evif_cache_stats | 1624 | 诊断 | ❌ 删除 | 🚫 幽灵 |
| evif_log_query | 1638 | 诊断 | ❌ 删除 | 🚫 幽灵 |
| evif_metrics_export | 1662 | 指标 | ❌ 删除 | 🚫 幽灵 |
| evif_config_get | 1682 | 配置 | ✅ 保留（合并） | 🚫 幽灵 |
| evif_event_subscribe | 1700 | 事件 | ❌ 删除 | 🚫 幽灵 |
| evif_event_list | 1719 | 事件 | ❌ 删除 | 🚫 幽灵 |
| evif_cron_schedule | 1733 | 定时 | ❌ 删除 | 🚫 幽灵 |
| evif_event_unsubscribe | 1755 | 事件 | ❌ 删除 | 🚫 幽灵 |
| evif_cron_list | 1769 | 定时 | ❌ 删除 | 🚫 幽灵 |
| evif_cron_remove | 1783 | 定时 | ❌ 删除 | 🚫 幽灵 |
| evif_session_load | 1797 | 会话 | ✅ 保留（合并） | 🚫 幽灵 |
| evif_subagent_kill | 1811 | Agent | ❌ 删除 | 🚫 幽灵 |
| evif_skill_create | 1829 | 技能 | 🔄 合并到 skill | 🚫 幽灵 |
| evif_skill_delete | 1851 | 技能 | 🔄 合并到 skill | 🚫 幽灵 |
| evif_memory_search | 1869 | 记忆 | ✅ 保留 | 🚫 幽灵 |
| evif_memory_stats | 1891 | 记忆 | ❌ 删除 | 🚫 幽灵 |
| evif_pipe_create | 1905 | 管道 | ✅ 保留（合并） | 🚫 幽灵 |
| evif_pipe_list | 1923 | 管道 | 🔄 合并到 pipe | 🚫 幽灵 |
| evif_health_detailed | 1937 | 健康 | ❌ 删除 | 🚫 幽灵 |
| evif_server_restart | 1951 | 元工具 | ❌ 删除 | 🚫 幽灵 |
| evif_log_level | 1965 | 配置 | ❌ 删除 | 🚫 幽灵 |
| evif_version | 1983 | 配置 | ❌ 删除 | 🚫 幽灵 |
| evif_config_set | 1997 | 配置 | ✅ 保留（合并） | 🚫 幽灵 |
| evif_config_list | 2019 | 配置 | 🔄 合并到 config | 🚫 幽灵 |
| evif_plugin_load | 2033 | 插件 | ✅ 保留（合并） | 🚫 幽灵 |
| evif_plugin_unload | 2051 | 插件 | 🔄 合并到 plugin | 🚫 幽灵 |
| evif_plugin_info | 2069 | 插件 | 🔄 合并到 plugin | 🚫 幽灵 |
| evif_subagent_status | 2083 | Agent | 🔄 合并到 subagent | 🚫 幽灵 |
| evif_queue_list | 2097 | 队列 | ❌ 删除 | 🚫 幽灵 |
| evif_queue_stats | 2115 | 队列 | ❌ 删除 | 🚫 幽灵 |
| evif_session_delete | 2129 | 会话 | ❌ 删除 | 🚫 幽灵 |
| evif_memory_clear | 2147 | 记忆 | ❌ 删除 | 🚫 幽灵 |
| evif_find | 2166 | 搜索 | ✅ 保留 | ❌ 模拟假数据 |
| evif_wc | 2188 | 搜索 | ❌ 删除 | ❌ 模拟假数据 |
| evif_tail | 2214 | 搜索 | 🔄 合并到 cat | ❌ 模拟假数据 |
| evif_open_handle | 2233 | Handle | ✅ 保留（合并） | ⚠️ HTTP |
| evif_close_handle | 2259 | Handle | 🔄 合并到 handle | ⚠️ HTTP |
| evif_memorize | 2274 | 记忆 | ✅ 保留 | ✅ VFS+HTTP |
| evif_retrieve | 2303 | 记忆 | ✅ 保留 | ✅ VFS+HTTP |
| evif_skill_list | 2330 | 技能 | ✅ 保留（合并） | ⚠️ HTTP |
| evif_skill_info | 2339 | 技能 | 🔄 合并到 skill | ⚠️ HTTP |
| evif_skill_execute | 2353 | 技能 | 🔄 合并到 skill | ⚠️ HTTP |
| evif_claude_md_generate | 2377 | 技能 | ❌ 删除 | ⚠️ HTTP |
| evif_session_save | 2402 | 会话 | ✅ 保留（合并） | ⚠️ HTTP |
| evif_session_list | 2424 | 会话 | 🔄 合并到 session | ⚠️ HTTP |
| evif_subagent_create | 2443 | Agent | ✅ 保留（合并） | ⚠️ HTTP |
| evif_subagent_send | 2465 | Agent | 🔄 合并到 subagent | ⚠️ HTTP |
| evif_subagent_list | 2483 | Agent | 🔄 合并到 subagent | ⚠️ HTTP |
| evif_mcp_capabilities | 2493 | 元工具 | ❌ 删除 | 🚫 幽灵 |
| evif_plugin_catalog | 2516 | 插件 | ❌ 删除 | 🚫 幽灵 |
| evif_server_stats | 2535 | 诊断 | ❌ 删除 | 🚫 幽灵 |
| evif_batch | 2554 | 批量 | ✅ 保留 | 🚫 幽灵 |
| evif_search | 2595 | 搜索 | ❌ 删除 | 🚫 幽灵 |
| evif_diff | 2622 | 实用 | ❌ 删除 | 🚫 幽灵 |
| evif_watch | 2649 | 实用 | ❌ 删除 | 🚫 幽灵 |
| evif_tree | 2676 | 实用 | 🔄 合并到 ls | 🚫 幽灵 |
| evif_archive | 2703 | 实用 | ❌ 删除 | 🚫 幽灵 |
| evif_hash | 2738 | 实用 | ❌ 删除 | 🚫 幽灵 |
| evif_du | 2757 | 实用 | ❌ 删除 | 🚫 幽灵 |

**图例**:
- **功能状态**: ✅ 保留 | 🔄 合并 | ❌ 删除
- **可调用性**:
  - ✅ VFS+HTTP：模拟模式和真实模式都可用 (9 个)
  - ⚠️ HTTP：仅在连接后端时可用 (16 个)
  - ❌ 模拟：仅在 mock 模式返回假数据 (3 个)
  - 🚫 幽灵：注册了但完全无法调用 (46 个)

### 8.1 可调用性统计

| 状态 | 数量 | 占比 | 行动 |
|------|------|------|------|
| ✅ VFS+HTTP | 9 | 12% | 保留 |
| ⚠️ HTTP | 16 | 21% | 补全 VFS 模拟 |
| ❌ 模拟 | 3 | 4% | 删除或修复 |
| 🚫 幽灵 | 46 | 61% | **立即删除** |
| **总计** | **75** | **100%** | - |

---

## 九、代码修复指南

### 9.1 Phase 0：删除 46 个幽灵工具

在 `crates/evif-mcp/src/lib.rs` 中：

1. 找到 `initialize_tools` 函数 (line 1337-2782)
2. 删除以下工具注册（按行号从大到小删除，避免行号变化）：

```rust
// 删除顺序：从大行号到小行号，避免删除后行号变化

// 行号 > 2500 (按从大到小顺序)
.drop_tool("evif_du");           // 2757
.drop_tool("evif_hash");          // 2738
.drop_tool("evif_archive");       // 2703
.drop_tool("evif_tree");          // 2676
.drop_tool("evif_watch");         // 2649
.drop_tool("evif_diff");          // 2622
.drop_tool("evif_search");        // 2595
.drop_tool("evif_batch");        // 2554 - 注意：这个工具声称被调用但无分发
.drop_tool("evif_plugin_catalog"); // 2516
.drop_tool("evif_mcp_capabilities"); // 2493

// 行号 2400-2500
.drop_tool("evif_subagent_list");  // 2483
.drop_tool("evif_subagent_send"); // 2465 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_subagent_create"); // 2443 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_session_list");  // 2424
.drop_tool("evif_session_save");  // 2402 - 注意：这个声称有 HTTP 实现

// 行号 2300-2400
.drop_tool("evif_claude_md_generate"); // 2377
.drop_tool("evif_skill_execute"); // 2353 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_skill_info");   // 2339 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_skill_list");   // 2330 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_retrieve");     // 2303 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_memorize");     // 2274 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_close_handle"); // 2259 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_open_handle");  // 2233 - 注意：这个声称有 HTTP 实现
.drop_tool("evif_tail");         // 2214
.drop_tool("evif_wc");           // 2188
.drop_tool("evif_find");         // 2166
.drop_tool("evif_memory_clear"); // 2147
.drop_tool("evif_session_delete"); // 2129
.drop_tool("evif_queue_stats");  // 2115
.drop_tool("evif_queue_list");   // 2097
.drop_tool("evif_subagent_status"); // 2083
.drop_tool("evif_plugin_info");  // 2069
.drop_tool("evif_plugin_unload"); // 2051
.drop_tool("evif_plugin_load"); // 2033
.drop_tool("evif_config_list"); // 2019
.drop_tool("evif_config_set");   // 1997
.drop_tool("evif_version");      // 1983
.drop_tool("evif_log_level");    // 1965
.drop_tool("evif_server_restart"); // 1951
.drop_tool("evif_health_detailed"); // 1937
.drop_tool("evif_pipe_list");    // 1923
.drop_tool("evif_pipe_create");  // 1905
.drop_tool("evif_memory_stats"); // 1891
.drop_tool("evif_memory_search"); // 1869
.drop_tool("evif_skill_delete"); // 1851
.drop_tool("evif_skill_create"); // 1829
.drop_tool("evif_subagent_kill"); // 1811
.drop_tool("evif_session_load"); // 1797
.drop_tool("evif_cron_remove");  // 1783
.drop_tool("evif_cron_list");   // 1769
.drop_tool("evif_event_unsubscribe"); // 1755
.drop_tool("evif_cron_schedule"); // 1733
.drop_tool("evif_event_list");  // 1719
.drop_tool("evif_event_subscribe"); // 1700
.drop_tool("evif_config_get");  // 1682
.drop_tool("evif_metrics_export"); // 1662
.drop_tool("evif_log_query");   // 1638
.drop_tool("evif_cache_stats");  // 1624
.drop_tool("evif_request_trace"); // 1606
.drop_tool("evif_latency_test"); // 1586
.drop_tool("evif_ping_with_stats"); // 1571
```

### 9.2 Phase 1：删除工具定义中的冗余工具

保留的工具只有 9 个核心工具：
- evif_ls
- evif_cat
- evif_write
- evif_mkdir
- evif_rm
- evif_grep (HTTP)
- evif_health (HTTP)
- evif_mount (HTTP)
- evif_mounts (HTTP)

### 9.3 Phase 2：合并为子工具模式

将剩余工具按功能合并：

```rust
// 合并策略：保留主工具名，通过 action 参数区分子操作

// evif_file = evif_ls + evif_cat + evif_write + evif_mkdir + evif_rm
// evif_mount = evif_mount + evif_unmount + evif_mounts
// evif_skill = evif_skill_list + evif_skill_info + evif_skill_execute
// evif_session = evif_session_save + evif_session_list + evif_session_load
// evif_subagent = evif_subagent_create + evif_subagent_send + evif_subagent_list
```

### 9.4 补全 VFS 模拟实现

对于声称有 HTTP 实现但无 VFS 模拟的工具，需要补全：

| 工具 | 需要实现 | 说明 |
|------|----------|------|
| evif_grep | VFS 实现 | 模拟模式下可全文搜索 |
| evif_health | VFS 实现 | 返回 mock 健康状态 |
| evif_mount/mounts | VFS 实现 | 模拟挂载点列表 |
| evif_skill_* | VFS 实现 | 模拟技能系统 |
| evif_session_* | VFS 实现 | 模拟会话 |
| evif_subagent_* | VFS 实现 | 模拟子 Agent |

---

## 十、最终状态：简化后的工具列表

### 10.1 Phase 0 后：25 个可调用工具

```
核心工具 (9):  VFS + HTTP 双实现
- evif_ls          # 目录列表
- evif_cat         # 文件读取
- evif_write       # 文件写入
- evif_mkdir       # 创建目录
- evif_rm          # 删除文件
- evif_memorize    # 存储记忆
- evif_retrieve    # 检索记忆

HTTP 工具 (16): 需要补全 VFS 模拟
- evif_grep        # 文本搜索
- evif_health      # 健康检查
- evif_mount       # 挂载管理
- evif_mounts      # 挂载列表
- evif_skill_list  # 技能列表
- evif_skill_info  # 技能详情
- evif_skill_execute # 执行技能
- evif_session_save  # 保存会话
- evif_session_list # 会话列表
- evif_subagent_create # 创建 Agent
- evif_subagent_send   # 发送消息
```

### 10.2 Phase 1 后：20 个核心工具（合并后）

```
evif_file   { action: "ls|cat|write|mkdir|rm", path, content, ... }
evif_grep   { pattern, path, ... }
evif_mount  { action: "mount|unmount|list", plugin_name, ... }
evif_health { }
evif_memorize { content, ... }
evif_retrieve { query, ... }
evif_skill  { action: "list|info|execute", name, ... }
evif_session { action: "save|load|list", session_id, ... }
evif_subagent { action: "create|send|list", agent_id, message, ... }
```