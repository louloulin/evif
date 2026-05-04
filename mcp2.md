# EVIF MCP Tools 分析报告

> 创建时间：2026-05-04
> 分析：75 个 MCP Tools 的分类、用途、可删除性评估

---

## 一、工具总览（75 个）

### 1.1 文件操作类 (8)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_ls` | 列出目录 | ✅ 必须 | ❌ |
| `evif_cat` | 读取文件 | ✅ 必须 | ❌ |
| `evif_write` | 写入文件 | ✅ 必须 | ❌ |
| `evif_mkdir` | 创建目录 | ✅ 必须 | ❌ |
| `evif_rm` | 删除文件 | ⚠️ 危险 | ⭐ 可合并 |
| `evif_stat` | 文件信息 | ⚠️ 边缘 | ⭐ 可合并 |
| `evif_mv` | 重命名/移动 | ⚠️ 边缘 | ⭐ 可合并 |
| `evif_cp` | 复制文件 | ⚠️ 边缘 | ⭐ 可合并 |

**分析**: 核心文件操作 4 个必需，其余 4 个可考虑合并或删除。

### 1.2 搜索工具类 (4)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_find` | 查找文件 | ✅ 必须 | ❌ |
| `evif_grep` | 文本搜索 | ✅ 必须 | ❌ |
| `evif_wc` | 字数统计 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_tail` | 查看尾部 | ⚠️ 边缘 | ⭐ 合并 |

**分析**: `evif_find` 和 `evif_grep` 必须保留，`wc`/`tail` 可考虑删除或合并到 `evif_cat`。

### 1.3 挂载管理类 (3)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_mount` | 挂载插件 | ✅ 必须 | ❌ |
| `evif_unmount` | 卸载插件 | ✅ 必须 | ❌ |
| `evif_mounts` | 列出挂载 | ⚠️ 边缘 | ⭐ 合并 |

### 1.4 Handle 操作类 (2)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_open_handle` | 打开句柄 | ✅ 必须 | ❌ |
| `evif_close_handle` | 关闭句柄 | ✅ 必须 | ❌ |

### 1.5 记忆系统类 (4)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_memorize` | 存储记忆 | ✅ 必须 | ❌ |
| `evif_retrieve` | 检索记忆 | ✅ 必须 | ❌ |
| `evif_memory_search` | 语义搜索 | ✅ 必须 | ❌ |
| `evif_memory_stats` | 记忆统计 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_memory_clear` | 清除记忆 | ⚠️ 边缘 | ⭐ 合并 |

### 1.6 技能系统类 (6)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_skill_list` | 列出技能 | ✅ 必须 | ❌ |
| `evif_skill_info` | 技能详情 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_skill_execute` | 执行技能 | ✅ 必须 | ❌ |
| `evif_skill_create` | 创建技能 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_skill_delete` | 删除技能 | ⚠️ 边缘 | ⭐ 合并 |

### 1.7 会话管理类 (4)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_session_save` | 保存会话 | ✅ 必须 | ❌ |
| `evif_session_load` | 加载会话 | ✅ 必须 | ❌ |
| `evif_session_list` | 列出会话 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_session_delete` | 删除会话 | ⚠️ 边缘 | ⭐ 合并 |

### 1.8 子 Agent 类 (4)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_subagent_create` | 创建子 Agent | ✅ 必须 | ❌ |
| `evif_subagent_send` | 发送消息 | ✅ 必须 | ❌ |
| `evif_subagent_list` | 列出子 Agent | ⚠️ 边缘 | ⭐ 合并 |
| `evif_subagent_status` | Agent 状态 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_subagent_kill` | 终止子 Agent | ⚠️ 边缘 | ⭐ 合并 |

### 1.9 工具类 (6)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_health` | 健康检查 | ✅ 必须 | ❌ |
| `evif_ping_with_stats` | Ping 统计 | ⚠️ 冗余 | ⭐ 删除 |
| `evif_health_detailed` | 详细健康 | ⚠️ 冗余 | ⭐ 删除 |
| `evif_claude_md_generate` | 生成 CLAUDE.md | ⭐ 可选 | ⭐ 删除 |
| `evif_search` | 语义搜索 | ⚠️ 冗余 | ⭐ 删除 |
| `evif_diff` | 文件对比 | ⭐ 可选 | ⭐ 删除 |

### 1.10 诊断工具类 (5)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_latency_test` | 延迟测试 | ⭐ 可选 | ⭐ 删除 |
| `evif_request_trace` | 请求追踪 | ⭐ 可选 | ⭐ 删除 |
| `evif_cache_stats` | 缓存统计 | ⭐ 可选 | ⭐ 删除 |
| `evif_log_query` | 日志查询 | ⭐ 可选 | ⭐ 删除 |
| `evif_server_stats` | 服务器统计 | ⭐ 可选 | ⭐ 删除 |

### 1.11 配置管理类 (3)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_config_get` | 获取配置 | ✅ 必须 | ❌ |
| `evif_config_set` | 设置配置 | ✅ 必须 | ❌ |
| `evif_config_list` | 列出配置 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_version` | 版本信息 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_log_level` | 日志级别 | ⭐ 可选 | ⭐ 删除 |

### 1.12 事件与定时类 (6)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_event_subscribe` | 事件订阅 | ⭐ 可选 | ⭐ 删除 |
| `evif_event_list` | 事件列表 | ⭐ 可选 | ⭐ 删除 |
| `evif_event_unsubscribe` | 取消订阅 | ⭐ 可选 | ⭐ 删除 |
| `evif_cron_schedule` | 定时任务 | ⭐ 可选 | ⭐ 删除 |
| `evif_cron_list` | 定时列表 | ⭐ 可选 | ⭐ 删除 |
| `evif_cron_remove` | 删除定时 | ⭐ 可选 | ⭐ 删除 |

### 1.13 管道类 (2)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_pipe_create` | 创建管道 | ✅ 必须 | ❌ |
| `evif_pipe_list` | 管道列表 | ⚠️ 边缘 | ⭐ 合并 |

### 1.14 插件管理类 (3)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_plugin_load` | 加载插件 | ✅ 必须 | ❌ |
| `evif_plugin_unload` | 卸载插件 | ✅ 必须 | ❌ |
| `evif_plugin_info` | 插件信息 | ⚠️ 边缘 | ⭐ 合并 |
| `evif_plugin_catalog` | 插件目录 | ⭐ 可选 | ⭐ 删除 |

### 1.15 队列类 (2)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_queue_list` | 队列列表 | ⭐ 可选 | ⭐ 删除 |
| `evif_queue_stats` | 队列统计 | ⭐ 可选 | ⭐ 删除 |

### 1.16 实用工具类 (8)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_watch` | 文件监视 | ⭐ 可选 | ⭐ 删除 |
| `evif_tree` | 目录树 | ⭐ 可选 | ⭐ 删除 |
| `evif_archive` | 压缩包 | ⭐ 可选 | ⭐ 删除 |
| `evif_hash` | 文件哈希 | ⭐ 可选 | ⭐ 删除 |
| `evif_du` | 磁盘使用 | ⭐ 可选 | ⭐ 删除 |

### 1.17 元工具类 (2)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_mcp_capabilities` | MCP 能力 | ⭐ 可选 | ⭐ 删除 |
| `evif_server_restart` | 服务器重启 | ⭐ 可选 | ⭐ 删除 |

### 1.18 批量操作类 (1)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_batch` | 批量操作 | ✅ 必须 | ❌ |

### 1.19 指标导出类 (1)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_metrics_export` | 指标导出 | ⭐ 可选 | ⭐ 删除 |

### 1.20 指标导出类 (1)

| 工具 | 描述 | 核心 | 可删除 |
|------|------|------|--------|
| `evif_metrics_export` | 指标导出 | ⭐ 可选 | ⭐ 删除 |

---

## 二、可删除工具清单

### 2.1 高优先级删除（对 AI 用途小）

| 工具 | 原因 | 替代方案 |
|------|------|----------|
| `evif_ping_with_stats` | 与 `evif_health` 重复 | 使用 `evif_health` |
| `evif_health_detailed` | 与 `evif_health` 重复 | 使用 `evif_health` |
| `evif_claude_md_generate` | 一次性生成，不常用 | CLI 生成 |
| `evif_search` | 与 `evif_memory_search` 重复 | 使用 `evif_memory_search` |
| `evif_latency_test` | 调试用，AI 不需要 | 手动测试 |
| `evif_request_trace` | 调试用，AI 不需要 | 手动测试 |
| `evif_cache_stats` | 调试用，AI 不需要 | 手动检查 |
| `evif_log_query` | 调试用，AI 不需要 | 手动查看 |
| `evif_server_stats` | 调试用，AI 不需要 | 手动查看 |
| `evif_log_level` | 调试用，AI 不需要 | 配置文件 |
| `evif_server_restart` | 危险操作，AI 不需要 | CLI 手动 |
| `evif_mcp_capabilities` | 元工具，AI 初始化时用 | 初始化时调用一次 |
| `evif_plugin_catalog` | 元工具，AI 初始化时用 | 初始化时调用一次 |
| `evif_metrics_export` | 调试用，AI 不需要 | 手动导出 |

### 2.2 中优先级合并或删除

| 工具 | 原因 | 建议 |
|------|------|------|
| `evif_stat` | 可合并到 `evif_ls` 返回值 | 合并到 `evif_ls` |
| `evif_mv` | 可合并到 `evif_cp` + `evif_rm` | 删除，CLI 用 |
| `evif_cp` | 可合并到 `evif_write` | 删除，CLI 用 |
| `evif_wc` | 可合并到 `evif_cat` | 删除，CLI 用 |
| `evif_tail` | 可合并到 `evif_cat` | 删除，CLI 用 |
| `evif_mounts` | 可合并到 `evif_mount --list` | 合并 |
| `evif_skill_info` | 可合并到 `evif_skill_list` | 合并 |
| `evif_skill_create` | 一次性操作 | 删除，CLI 用 |
| `evif_skill_delete` | 一次性操作 | 删除，CLI 用 |
| `evif_session_list` | 可合并到 `evif_session_save` 返回 | 合并 |
| `evif_session_delete` | 一次性操作 | 删除，CLI 用 |
| `evif_subagent_list` | 可合并到 `evif_subagent_create` 返回 | 合并 |
| `evif_subagent_status` | 可合并到 `evif_subagent_list` | 合并 |
| `evif_subagent_kill` | 一次性操作 | 删除，CLI 用 |
| `evif_config_list` | 可合并到 `evif_config_get` | 合并 |
| `evif_memory_stats` | 一次性操作 | 删除，CLI 用 |
| `evif_memory_clear` | 危险操作 | 删除，CLI 用 |
| `evif_pipe_list` | 可合并到 `evif_pipe_create` | 合并 |
| `evif_plugin_info` | 可合并到 `evif_plugin_load` 返回 | 合并 |
| `evif_queue_list` | 一次性操作 | 删除，CLI 用 |
| `evif_queue_stats` | 一次性操作 | 删除，CLI 用 |
| `evif_watch` | 交互式操作，AI 不需要 | 删除 |
| `evif_tree` | 可合并到 `evif_ls --recursive` | 合并到 `evif_ls` |
| `evif_archive` | 一次性操作 | 删除，CLI 用 |
| `evif_hash` | 一次性操作 | 删除，CLI 用 |
| `evif_du` | 一次性操作 | 删除，CLI 用 |
| `evif_diff` | 一次性操作 | 删除，CLI 用 |

### 2.3 事件与定时类（可全部删除）

| 工具 | 原因 |
|------|------|
| `evif_event_subscribe` | AI 不使用事件 |
| `evif_event_list` | AI 不使用事件 |
| `evif_event_unsubscribe` | AI 不使用事件 |
| `evif_cron_schedule` | AI 不使用定时 |
| `evif_cron_list` | AI 不使用定时 |
| `evif_cron_remove` | AI 不使用定时 |

---

## 三、建议保留的核心工具（25 个）

| 工具 | 分类 | 说明 |
|------|------|------|
| `evif_ls` | 文件操作 | 列出目录（扩展返回 stat） |
| `evif_cat` | 文件操作 | 读取文件 |
| `evif_write` | 文件操作 | 写入文件 |
| `evif_mkdir` | 文件操作 | 创建目录 |
| `evif_rm` | 文件操作 | 删除文件 |
| `evif_find` | 搜索工具 | 查找文件 |
| `evif_grep` | 搜索工具 | 文本搜索 |
| `evif_mount` | 挂载管理 | 挂载插件（--unmount, --list 合并） |
| `evif_open_handle` | Handle | 打开句柄 |
| `evif_close_handle` | Handle | 关闭句柄 |
| `evif_memorize` | 记忆系统 | 存储记忆 |
| `evif_retrieve` | 记忆系统 | 检索记忆 |
| `evif_memory_search` | 记忆系统 | 语义搜索 |
| `evif_skill_list` | 技能系统 | 列出技能（扩展返回 info） |
| `evif_skill_execute` | 技能系统 | 执行技能 |
| `evif_session_save` | 会话管理 | 保存会话（扩展返回 list, load） |
| `evif_session_load` | 会话管理 | 加载会话 |
| `evif_subagent_create` | 子 Agent | 创建子 Agent（扩展返回 list, status） |
| `evif_subagent_send` | 子 Agent | 发送消息 |
| `evif_config_get` | 配置管理 | 获取配置（扩展返回 list, set） |
| `evif_config_set` | 配置管理 | 设置配置 |
| `evif_health` | 健康检查 | 健康检查 |
| `evif_plugin_load` | 插件管理 | 加载插件（扩展返回 unload, info） |
| `evif_pipe_create` | 管道类 | 创建管道（扩展返回 list） |
| `evif_batch` | 批量操作 | 批量操作 |

---

## 四、简化后的工具数量

| 类别 | 原数量 | 建议保留 | 删除 |
|------|--------|----------|------|
| 文件操作 | 8 | 4-5 | 3-4 |
| 搜索工具 | 4 | 2 | 2 |
| 挂载管理 | 3 | 1 | 2 |
| Handle | 2 | 2 | 0 |
| 记忆系统 | 5 | 3 | 2 |
| 技能系统 | 6 | 2 | 4 |
| 会话管理 | 4 | 2 | 2 |
| 子 Agent | 5 | 2 | 3 |
| 工具类 | 6 | 1-2 | 4-5 |
| 诊断工具 | 5 | 0-1 | 4-5 |
| 配置管理 | 4 | 2 | 2 |
| 事件与定时 | 6 | 0 | 6 |
| 管道类 | 2 | 1 | 1 |
| 插件管理 | 4 | 1 | 3 |
| 队列类 | 2 | 0 | 2 |
| 实用工具 | 8 | 0-1 | 7-8 |
| 元工具类 | 2 | 0 | 2 |
| 批量操作 | 1 | 1 | 0 |
| 指标导出 | 1 | 0 | 1 |
| **总计** | **75** | **25-30** | **45-50** |

---

## 五、实施建议

### 5.1 方案 A：保留 30 个核心工具

```rust
// 简化后的工具列表
const CORE_TOOLS: [&str; 30] = [
    "evif_ls", "evif_cat", "evif_write", "evif_mkdir", "evif_rm",
    "evif_find", "evif_grep",
    "evif_mount",
    "evif_open_handle", "evif_close_handle",
    "evif_memorize", "evif_retrieve", "evif_memory_search",
    "evif_skill_list", "evif_skill_execute",
    "evif_session_save", "evif_session_load",
    "evif_subagent_create", "evif_subagent_send",
    "evif_config_get", "evif_config_set",
    "evif_health",
    "evif_plugin_load",
    "evif_pipe_create",
    "evif_batch",
    "evif_watch", "evif_tree",  // 可选保留 1-2 个实用工具
];
```

### 5.2 方案 B：保留 50 个工具（当前 75 个）

保留所有核心工具，只删除明显的冗余和一次性操作。

### 5.3 方案 C：功能模块化

```rust
// 按功能模块组织，AI 按需启用
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ToolConfig {
    pub file_ops: bool,      // 文件操作 (5)
    pub search: bool,         // 搜索工具 (2)
    pub memory: bool,        // 记忆系统 (3)
    pub skills: bool,        // 技能系统 (2)
    pub agents: bool,        // Agent 系统 (4)
    pub debug: bool,         // 诊断工具 (默认关闭)
    pub events: bool,         // 事件定时 (默认关闭)
    pub utils: bool,          // 实用工具 (默认关闭)
}
```

---

## 六、结论

### 6.1 工具过多的问题

1. **AI 认知负担**：75 个工具对 AI 来说太多，难以选择
2. **Token 消耗**：每次 `tools/list` 返回 75 个工具定义
3. **维护成本**：每个工具需要文档、测试、错误处理

### 6.2 推荐方案

**保留 30-40 个核心工具**，将边缘工具作为 CLI 命令或按需启用：

| 方案 | 工具数量 | 适用场景 |
|------|----------|----------|
| 极简 | 20-25 | 对 Token 敏感的场景 |
| 推荐 | 30-40 | 平衡功能和复杂度 |
| 完整 | 50-60 | 需要完整功能的场景 |
| 当前 | 75 | 不推荐 |

### 6.3 实施优先级

1. **Phase 1**：删除 30+ 明显冗余工具
2. **Phase 2**：合并可合并的工具
3. **Phase 3**：实现工具配置系统（按需启用）

---

## 七、工具分类详细列表

### 7.1 必须保留 (25)

```
evif_ls, evif_cat, evif_write, evif_mkdir, evif_rm
evif_find, evif_grep
evif_mount, evif_unmount
evif_open_handle, evif_close_handle
evif_memorize, evif_retrieve, evif_memory_search
evif_skill_list, evif_skill_execute
evif_session_save, evif_session_load
evif_subagent_create, evif_subagent_send
evif_config_get, evif_config_set
evif_health
evif_plugin_load, evif_plugin_unload
evif_pipe_create
evif_batch
```

### 7.2 可删除 (50)

```
evif_stat, evif_mv, evif_cp
evif_wc, evif_tail
evif_mounts
evif_ping_with_stats, evif_health_detailed, evif_claude_md_generate
evif_search, evif_diff
evif_latency_test, evif_request_trace, evif_cache_stats
evif_log_query, evif_server_stats, evif_log_level
evif_version
evif_event_subscribe, evif_event_list, evif_event_unsubscribe
evif_cron_schedule, evif_cron_list, evif_cron_remove
evif_skill_info, evif_skill_create, evif_skill_delete
evif_session_list, evif_session_delete
evif_subagent_list, evif_subagent_status, evif_subagent_kill
evif_config_list
evif_memory_stats, evif_memory_clear
evif_pipe_list
evif_plugin_info, evif_plugin_catalog
evif_queue_list, evif_queue_stats
evif_watch, evif_tree, evif_archive, evif_hash, evif_du
evif_mcp_capabilities, evif_server_restart
evif_metrics_export
```