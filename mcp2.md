# EVIF MCP Tools 分析报告

> 创建时间：2026-05-04
> 更新：2026-05-04
> 分析：75 个 MCP Tools 的分类、合并、删除、子工具设计

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

### 7.2 简化收益

| 收益 | 说明 |
|------|------|
| Token 节省 | tools/list 响应减少 60% |
| AI 决策简化 | 从 75 选 1 降到 24 选 1 |
| 维护成本 | 代码量减少 40% |
| 一致性 | 统一的 action 参数模式 |

### 7.3 实施优先级

| 阶段 | 时间 | 任务 |
|------|------|------|
| Phase 1 | 第 1 周 | 删除 46 个冗余工具 |
| Phase 2 | 第 2 周 | 合并 11 个子工具 |
| Phase 3 | 第 3 周 | 文档和测试 |

---

## 八、附录：完整 75 工具对照表

| 工具 | 行号 | 分类 | 状态 |
|------|------|------|------|
| evif_ls | 1341 | 文件 | ✅ 保留 |
| evif_cat | 1359 | 文件 | ✅ 保留 |
| evif_write | 1381 | 文件 | ✅ 保留 |
| evif_mkdir | 1407 | 文件 | ✅ 保留 |
| evif_rm | 1425 | 文件 | ✅ 保留 |
| evif_stat | 1443 | 文件 | ❌ 删除 |
| evif_mv | 1457 | 文件 | ❌ 删除 |
| evif_cp | 1475 | 文件 | ❌ 删除 |
| evif_mount | 1494 | 挂载 | ✅ 保留（合并） |
| evif_unmount | 1516 | 挂载 | 🔄 合并到 mount |
| evif_mounts | 1530 | 挂载 | 🔄 合并到 mount |
| evif_grep | 1540 | 搜索 | ✅ 保留 |
| evif_health | 1562 | 健康 | ✅ 保留 |
| evif_ping_with_stats | 1571 | 健康 | ❌ 删除 |
| evif_latency_test | 1586 | 诊断 | ❌ 删除 |
| evif_request_trace | 1606 | 诊断 | ❌ 删除 |
| evif_cache_stats | 1624 | 诊断 | ❌ 删除 |
| evif_log_query | 1638 | 诊断 | ❌ 删除 |
| evif_metrics_export | 1662 | 指标 | ❌ 删除 |
| evif_config_get | 1682 | 配置 | ✅ 保留（合并） |
| evif_event_subscribe | 1700 | 事件 | ❌ 删除 |
| evif_event_list | 1719 | 事件 | ❌ 删除 |
| evif_cron_schedule | 1733 | 定时 | ❌ 删除 |
| evif_event_unsubscribe | 1755 | 事件 | ❌ 删除 |
| evif_cron_list | 1769 | 定时 | ❌ 删除 |
| evif_cron_remove | 1783 | 定时 | ❌ 删除 |
| evif_session_load | 1797 | 会话 | ✅ 保留（合并） |
| evif_subagent_kill | 1811 | Agent | ❌ 删除 |
| evif_skill_create | 1829 | 技能 | 🔄 合并到 skill |
| evif_skill_delete | 1851 | 技能 | 🔄 合并到 skill |
| evif_memory_search | 1869 | 记忆 | ✅ 保留 |
| evif_memory_stats | 1891 | 记忆 | ❌ 删除 |
| evif_pipe_create | 1905 | 管道 | ✅ 保留（合并） |
| evif_pipe_list | 1923 | 管道 | 🔄 合并到 pipe |
| evif_health_detailed | 1937 | 健康 | ❌ 删除 |
| evif_server_restart | 1951 | 元工具 | ❌ 删除 |
| evif_log_level | 1965 | 配置 | ❌ 删除 |
| evif_version | 1983 | 配置 | ❌ 删除 |
| evif_config_set | 1997 | 配置 | ✅ 保留（合并） |
| evif_config_list | 2019 | 配置 | 🔄 合并到 config |
| evif_plugin_load | 2033 | 插件 | ✅ 保留（合并） |
| evif_plugin_unload | 2051 | 插件 | 🔄 合并到 plugin |
| evif_plugin_info | 2069 | 插件 | 🔄 合并到 plugin |
| evif_subagent_status | 2083 | Agent | 🔄 合并到 subagent |
| evif_queue_list | 2097 | 队列 | ❌ 删除 |
| evif_queue_stats | 2115 | 队列 | ❌ 删除 |
| evif_session_delete | 2129 | 会话 | ❌ 删除 |
| evif_memory_clear | 2147 | 记忆 | ❌ 删除 |
| evif_find | 2166 | 搜索 | ✅ 保留 |
| evif_wc | 2188 | 搜索 | ❌ 删除 |
| evif_tail | 2214 | 搜索 | 🔄 合并到 cat |
| evif_open_handle | 2233 | Handle | ✅ 保留（合并） |
| evif_close_handle | 2259 | Handle | 🔄 合并到 handle |
| evif_memorize | 2274 | 记忆 | ✅ 保留 |
| evif_retrieve | 2303 | 记忆 | ✅ 保留 |
| evif_skill_list | 2330 | 技能 | ✅ 保留（合并） |
| evif_skill_info | 2339 | 技能 | 🔄 合并到 skill |
| evif_skill_execute | 2353 | 技能 | 🔄 合并到 skill |
| evif_claude_md_generate | 2377 | 技能 | ❌ 删除 |
| evif_session_save | 2402 | 会话 | ✅ 保留（合并） |
| evif_session_list | 2424 | 会话 | 🔄 合并到 session |
| evif_subagent_create | 2443 | Agent | ✅ 保留（合并） |
| evif_subagent_send | 2465 | Agent | 🔄 合并到 subagent |
| evif_subagent_list | 2483 | Agent | 🔄 合并到 subagent |
| evif_mcp_capabilities | 2493 | 元工具 | ❌ 删除 |
| evif_plugin_catalog | 2516 | 插件 | ❌ 删除 |
| evif_server_stats | 2535 | 诊断 | ❌ 删除 |
| evif_batch | 2554 | 批量 | ✅ 保留 |
| evif_search | 2595 | 搜索 | ❌ 删除 |
| evif_diff | 2622 | 实用 | ❌ 删除 |
| evif_watch | 2649 | 实用 | ❌ 删除 |
| evif_tree | 2676 | 实用 | 🔄 合并到 ls |
| evif_archive | 2703 | 实用 | ❌ 删除 |
| evif_hash | 2738 | 实用 | ❌ 删除 |
| evif_du | 2757 | 实用 | ❌ 删除 |

**图例**:
- ✅ 保留：作为主工具保留
- 🔄 合并：通过 action 参数合并到主工具
- ❌ 删除：完全删除，CLI 用