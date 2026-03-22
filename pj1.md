# EVIF 核心收敛改造计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 EVIF 从“功能面过宽、核心发散”的平台，收敛成一个类似 AGFS 的插件式文件系统平台，只保留真正能形成闭环的核心功能，并删除不再需要的 `graph` 主线。

**Architecture:** 目标内核是 `evif-core + evif-plugins + evif-rest + evif-cli + evif-fuse`。平台只保留四个一等能力：挂载路由、插件生命周期、文件/目录/句柄操作、对外访问面（REST/CLI/FUSE/MCP 可选）。`evif-mem` 可以作为可选子系统保留，但不再让“graph”定义整个平台；当前所有 graph 风格 API 要么删除，要么改名成 memory 自己的时间线/关系查询能力。

**Tech Stack:** Rust workspace、`axum`、`fuser`、动态插件加载、可选 `evif-mcp`、可选 `evif-mem`。

---

## 一、真实对比：EVIF 和 AGFS 的本质差距

### 1. AGFS 的核心是收敛的

AGFS 的主线非常清楚：

- `agfs-server`：唯一后端核心
- `agfs-shell`：围绕服务端文件接口的 shell
- `agfs-fuse`：把服务端文件接口挂到本地文件系统

它的真正内核是：

- `filesystem.FileSystem`
- `mountablefs.MountableFS`
- 插件系统
- HTTP handlers

代码证据：

- `agfs-server/pkg/filesystem/filesystem.go`
- `agfs-server/pkg/mountablefs/mountablefs.go`
- `agfs-server/pkg/handlers/handlers.go`
- `agfs-server/pkg/handlers/plugin_handlers.go`

AGFS 的设计重点不是抽象很多层，而是把“所有后端统一成文件操作”这件事做成唯一中心。

### 2. EVIF 的核心是发散的

EVIF 当前同时在推进这些主线：

- `evif-core`
- `evif-plugins`
- `evif-rest`
- `evif-cli`
- `evif-fuse`
- `evif-mcp`
- `evif-mem`
- `evif-graph`
- `evif-vfs`
- `evif-runtime`
- `evif-storage`
- `evif-protocol`
- `evif-grpc`

这不是“更先进”，而是“主线不止一条”。

真实后果：

- 平台核心不清楚
- 很多 crate 互相叠加，但不是互相增强
- 文件系统、graph、memory、runtime 都在争夺“谁是底座”

### 3. 关键差距不是语言，不是功能数量，而是架构纪律

AGFS 的优势：

- 中心内核单一
- 对外叙事统一
- 插件是绝对主线
- server/shell/fuse 都在围绕同一个文件系统核心工作

EVIF 的问题：

- 内核不止一个
- graph 渗透太深
- VFS 是半成品但占据了核心叙事
- REST/CLI/Web 暴露面比可靠核心大得多

## 二、真实代码层面对比

### 1. 核心抽象

AGFS：

- 一个 `FileSystem` 接口
- 一个 `MountableFS`
- 插件通过 `Validate + Initialize + Shutdown + GetFileSystem` 接入

EVIF：

- `EvifPlugin` 本身已经很接近 AGFS 的文件系统接口
- `RadixMountTable` 已经是正确方向
- 但同时又存在：
  - `evif-vfs`
  - `evif-graph`
  - `evif-runtime`
  - `evif-storage`

结论：

**EVIF 其实已经有了正确内核，但被多余抽象稀释了。**

### 2. graph 是 EVIF 偏离 AGFS 的最大来源

当前 `evif-graph` 不是一个边缘模块，而是平台级依赖：

- `evif-runtime` 依赖 graph engine
- `evif-storage` 存的是 `Node/Edge`
- `evif-vfs` 依赖 graph 节点和路径解析
- `evif-rest` 暴露 graph CRUD 与 query
- `evif-client` / `evif-protocol` / `evif-grpc` 都依赖 graph 类型

代码证据：

- `crates/evif-runtime/src/runtime.rs`
- `crates/evif-storage/src/memory.rs`
- `crates/evif-vfs/src/path.rs`
- `crates/evif-rest/src/handlers.rs`
- `crates/evif-rest/src/routes.rs`
- `crates/evif-client/src/client.rs`

这意味着：

- graph 不是“一个可选功能”
- graph 是“整个平台现在的错误主轴之一”

### 3. EVIF 的 VFS 没有成为真实核心

`evif-vfs` 现在不能作为平台底座使用，原因不是风格问题，而是实现本身没有闭环。

代码证据：

- `crates/evif-vfs/src/vfs.rs` 明写“简化实现以避免编译错误”
- `read/write` 直接忽略真实 offset
- `get_file_size` 返回固定值
- `rename/mkdir/rmdir` 大量是空实现或近似空实现
- `crates/evif-vfs/src/path.rs` 的 graph 子节点查找仍是待实现状态

结论：

**VFS 现在不是底座，而是占着底座位置的实验模块。**

### 4. AGFS 的 FUSE 和 shell 是围绕核心工作的，EVIF 的外围功能更像并行扩张

AGFS：

- `agfs-fuse` 很薄，只负责把远端 AGFS 文件接口映射到 FUSE
- `agfs-shell` 是围绕 AGFS 文件接口工作的 shell 客户端

EVIF：

- `evif-fuse` 自己承担了很多复杂度
- `evif-cli` 命令面很宽，但有大量命令只是声明存在，不是真正成熟能力
- `evif-web` 当前能力展示面远大于真实后端闭环

结论：

**AGFS 的外围围绕核心旋转；EVIF 的外围在逼着核心扩张。**

### 5. 测试信号

已确认的现实：

- EVIF 源码规模约 `67,538` 行 Rust（`crates/*/src`）
- AGFS server 约 `34,196` 行 Go
- AGFS shell 约 `16,975` 行 Python
- EVIF 当前有 `124` 个 `todo!()` 占位
- EVIF 代码里 `placeholder/not implemented/简化实现/暂时返回` 这类信号有 `108` 个以上

AGFS 当然也不是完美无缺，但它的主线代码比 EVIF 更收敛，空壳比例也明显更低。

## 三、必须删除 graph 的原因

### 结论先说

**不是因为 graph “做得不好”，而是因为它不应该再定义 EVIF 这个平台。**

### 真实判断

如果继续保留 graph 作为平台主线，会发生这些事：

- `evif-rest` 永远会同时维护“文件系统 API”和“graph API”
- `evif-cli` 会继续背负 graph 子命令
- `evif-storage` 会继续围绕 `Node/Edge` 组织
- `evif-runtime` 会继续假设 graph engine 是平台中心
- `evif-vfs` 会继续试图成为 graph 驱动 VFS

这条路不是“慢一点也能成功”，而是会持续分散工程预算。

### 正确处理方式

删除 graph 作为平台基础能力：

- 删除 `evif-graph`
- 删除 graph CRUD / query 公共接口
- 切断核心 crate 对 graph 的依赖
- memory 子系统如果还需要“关系/时间线”查询，保留逻辑，但不再叫 graph，也不再依赖 graph crate

## 四、收敛后的 EVIF 应该只保留什么

### 一等核心功能

这四类必须是平台唯一中心：

1. 挂载和路径路由
2. 插件生命周期与插件能力
3. 文件/目录/句柄操作
4. 对外访问面：REST、CLI、FUSE

### 二等可选能力

这些可以保留，但绝不能再定义平台：

- `evif-mcp`
- `evif-mem`
- Web UI

### 应该删除或降级的能力

- graph 平台
- graph 相关 REST API
- graph 驱动 runtime
- graph 型 storage 抽象
- 当前 `evif-vfs` 平台化定位
- `grpc` 作为一等对外面
- `protocol` 作为一等产品面

## 五、推荐改造路线

### 方案 A：一次性硬切

直接删掉：

- `evif-graph`
- `evif-vfs`
- `evif-runtime`
- `evif-protocol`
- `evif-grpc`

优点：

- 架构最快变干净

缺点：

- 风险大
- 很容易一次把 REST/CLI/Web 都打碎

### 方案 B：分阶段收敛后再删除

先把对外产品面中的 graph 移除，再逐步切断 crate 依赖，最后物理删除 graph 相关 crate。

优点：

- 风险最小
- 每一步都能验证
- 能保持文件系统主路径持续可用

缺点：

- 需要短期兼容层

### 方案 C：把 graph 变成 legacy feature

默认关闭，但先不删。

优点：

- 迁移阻力最小

缺点：

- 垃圾会继续留在平台里
- 团队会忍不住继续往 legacy 上堆代码

### 推荐方案

**采用方案 B。**

原因很简单：

- 最终目标和硬切一样
- 但更容易真实落地
- 更适合当前 EVIF 这种“表面大、底层散”的仓库

## 六、目标架构

### 最终保留的核心 crate

- `crates/evif-core`
- `crates/evif-plugins`
- `crates/evif-rest`
- `crates/evif-cli`
- `crates/evif-fuse`
- `crates/evif-mcp`（可选）
- `crates/evif-mem`（可选）

### 最终删除的 crate

- `crates/evif-graph`
- `crates/evif-vfs`
- `crates/evif-runtime`
- `crates/evif-protocol`
- `crates/evif-grpc`

### 需要重新定位的 crate

- `crates/evif-client`
  - 要么删除
  - 要么改成纯 REST 文件客户端

- `crates/evif-storage`
  - 要么删除
  - 要么把有价值的非 graph 辅助代码拆进插件内部支持模块

## 七、核心实施计划

### Task 1：先冻结产品核心边界

**Files:**
- Modify: `README.md`
- Modify: `README-CN.md`
- Modify: `Cargo.toml`
- Modify: `crates/evif-rest/src/routes.rs`
- Modify: `crates/evif-rest/src/handlers.rs`
- Modify: `crates/evif-cli/src/cli.rs`
- Modify: `crates/evif-cli/src/commands.rs`
- Modify: `evif-web/README.md`
- Test: `crates/evif-rest/tests/core_surface.rs`
- Test: `crates/evif-cli/tests/surface_contract.rs`

- [x] **Step 1: 写失败的核心表面测试**

测试要求：

- REST 不再把 graph 作为平台主能力暴露
- CLI 不再宣传 graph 为一等命令
- README 不再把平台定义成 graph-based filesystem

Run:

```bash
cargo test -p evif-rest --test core_surface -p evif-cli --test surface_contract
```

Expected: FAIL，因为当前仍存在 graph 路由和 graph 命令表面。

- [x] **Step 2: 收缩产品叙事**

统一所有对外文档，只宣称：

- 插件文件系统
- 文件/目录/挂载/句柄/动态插件
- REST/CLI/FUSE

不要再宣称 graph 是平台核心。

- [x] **Step 3: 移除 graph 对外入口**

删除或退役这些路由：

- `/nodes/:id`
- `/nodes/create/:node_type`
- `/nodes/:id/children`
- `/query`
- graph stats 路由

- [x] **Step 4: 重新跑表面测试**

Run:

```bash
cargo test -p evif-rest --test core_surface -p evif-cli --test surface_contract
```

Expected: PASS。

### Task 2：把 `evif-core` 确立为唯一内核

**Files:**
- Modify: `crates/evif-core/src/plugin.rs`
- Modify: `crates/evif-core/src/dynamic_loader.rs`
- Modify: `crates/evif-core/src/plugin_registry.rs`
- Modify: `crates/evif-rest/src/server.rs`
- Modify: `crates/evif-rest/src/lib.rs`
- Modify: `crates/evif-rest/src/handlers.rs`
- Modify: `evif-web/src/services/plugin-api.ts`
- Modify: `evif-web/src/components/PluginManagerView.tsx`
- Test: `crates/evif-core/tests/plugin_lifecycle.rs`
- Test: `crates/evif-rest/tests/plugin_mount_contract.rs`

- [x] **Step 1: 写失败的插件生命周期测试**

至少覆盖：

- `validate`
- `initialize`
- `shutdown`
- 插件加载失败必须明确失败
- 不允许静默 fallback 到 `MemFS`

Run:

```bash
cargo test -p evif-core --test plugin_lifecycle -p evif-rest --test plugin_mount_contract
```

Expected: FAIL，因为当前生命周期不完整，且服务端仍有 fallback 逻辑。

- [x] **Step 2: 完善插件生命周期**

把 `EvifPlugin` 从“只有文件操作接口”补成真正平台接口：

- `validate`
- `initialize`
- `shutdown`

但不要再增加 graph/VFS 之类的新抽象。

- [x] **Step 3: 删除 silent fallback**

`crates/evif-rest/src/server.rs` 中：

- 动态插件加载失败
- 插件实例创建失败
- 插件初始化失败

都必须 fail fast，不能降级成别的插件。

- [x] **Step 4: 前后端插件能力对齐**

当前进度：

- 后端 `/api/v1/plugins/available` 已切换为结构化核心插件库存输出
- 前端 `PluginManagerView` 已改为读取后端插件库存，而不是硬编码 3 个插件
- 前端完整类型检查仍受现有 `bun-types` 环境问题阻塞，未完成最终验证

前端不能再硬编码“只有 3 个插件”；它应该展示后端真正支持的插件清单和支持等级。

- [x] **Step 5: 重跑生命周期测试**

Run:

```bash
cargo test -p evif-core --test plugin_lifecycle -p evif-rest --test plugin_mount_contract
```

Expected: PASS。

### Task 3：把 memory 里的“graph”改成 memory 自己的查询能力

**Files:**
- Modify: `crates/evif-rest/src/memory_handlers.rs`
- Modify: `crates/evif-rest/src/routes.rs`
- Modify: `evif-web/src/services/memory-api.ts`
- Modify: `evif-web/src/components/memory/KnowledgeGraph.tsx`
- Modify: `evif-web/src/components/memory/MemoryTimeline.tsx`
- Modify: `evif-web/src/components/memory/MemoryView.tsx`
- Modify: `crates/evif-mem-py/evif_mem/models.py`
- Modify: `crates/evif-mem-py/evif_mem/client.py`
- Test: `crates/evif-rest/tests/memory_query_contract.rs`

- [x] **Step 1: 写失败的 memory 查询契约测试**

新契约应当是 memory 自己的查询，例如：

- `POST /api/v1/memories/query`

查询类型可以保留：

- `timeline`
- `causal_chain`
- `temporal_path`
- `temporal_bfs`

但不再使用 graph 平台命名。

Run:

```bash
cargo test -p evif-rest --test memory_query_contract
```

Expected: FAIL，因为当前仍是 `/api/v1/graph/query`。

- [x] **Step 2: 改名，不改核心行为**

将：

- `GraphQueryRequest` -> `MemoryQueryRequest`
- `GraphQueryResponse` -> `MemoryQueryResponse`
- `GraphNode` -> `MemoryNode` 或 `TimelineNode`

这一步不追求重做算法，只先纠正所有权和命名。

- [x] **Step 3: 同步更新前端与 SDK**

当前进度：

- 已更新 `evif-web` 前端调用与文案
- 已更新 `evif-mem-py` Python SDK 命名、接口与测试
- 已更新 `evif-mem-ts` README 中的 memory 查询命名
- Python SDK 兼容层仍保留 `query_graph` / `GraphQueryType` 别名，便于过渡

前端和 Python/TS SDK 全部切到新的 memory 查询命名。

- [x] **Step 4: 重跑 memory 查询测试**

Run:

```bash
cargo test -p evif-rest --test memory_query_contract
```

Expected: PASS。

### Task 4：切断核心 crate 对 `evif-graph` 的依赖

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/evif-rest/Cargo.toml`
- Modify: `crates/evif-cli/Cargo.toml`
- Modify: `crates/evif-fuse/Cargo.toml`
- Modify: `crates/evif-client/Cargo.toml`
- Modify: `crates/evif-runtime/Cargo.toml`
- Modify: `crates/evif-storage/Cargo.toml`
- Modify: `crates/evif-vfs/Cargo.toml`
- Modify: `crates/evif-protocol/Cargo.toml`
- Modify: `crates/evif-grpc/Cargo.toml`
- Test: `tests/integration/no_graph_deps.sh`

- [x] **Step 1: 写依赖失败检查**

Run:

```bash
rg -n "evif-graph" crates/*/Cargo.toml
```

Expected: FAIL，因为当前很多核心 crate 仍依赖 `evif-graph`。

当前进度：

- 已新增 `tests/integration/no_graph_deps.sh`
- 已用该脚本验证核心产品路径不存在对 `evif-graph` 的直接引用

- [x] **Step 2: 先切核心产品路径**

优先从这些 crate 里移除 `evif-graph`：

- `evif-rest`
- `evif-cli`
- `evif-fuse`

当前进度：

- 已删除 `evif-rest` 中无路由引用的 graph 状态、graph handler、graph DTO 与 `evif-graph` 依赖
- 已删除 `evif-cli` 中死的 graph 方法和 `evif-graph` 直接依赖
- 已删除 `evif-fuse` 对 `evif-graph` 的直接依赖

- [x] **Step 3: 将 graph 依赖 crate 降级为 legacy 或待删**

当前进度：

- `evif-runtime`
- `evif-storage`
- `evif-protocol`
- `evif-grpc`
- `evif-vfs`
- `evif-client`

这些 crate 仍在仓库中，但已不再属于默认工作集：

- 根 `Cargo.toml` 已增加 `default-members`
- 默认 `cargo check` / `cargo build` / `cargo test` 将优先聚焦支持路径
- 这些 crate 仍未物理删除，也仍可能作为间接依赖存在，后续任务继续处理

这些 crate 不能继续处于平台主路径：

- `evif-runtime`
- `evif-storage`
- `evif-protocol`
- `evif-grpc`
- `evif-vfs`
- `evif-client`

- [x] **Step 4: 运行核心构建检查**

Run:

```bash
cargo check -p evif-core -p evif-plugins -p evif-rest -p evif-cli -p evif-fuse -p evif-mcp
```

Expected: PASS。

### Task 5：物理删除 graph 相关 crate 与示例

**Files:**
- Delete: `crates/evif-graph`
- Delete: `crates/evif-vfs`
- Delete: `crates/evif-runtime`
- Delete: `crates/evif-protocol`
- Delete: `crates/evif-grpc`
- Delete: `examples/01_basic_graph.rs`
- Delete: `examples/04_graph_queries.rs`
- Modify: `examples/Cargo.toml`
- Modify: `README.md`
- Test: `tests/integration/no_graph_left.sh`

- [x] **Step 1: 写“图能力已清空”检查**

Run:

```bash
rg -n "evif_graph|Graph::new|NodeType|NodeId" crates tests examples evif-web/src
```

Expected: FAIL，在删除前仍有大量命中。

- [x] **Step 2: 删除 graph 时代遗留 crate**

在 Task 1-4 完成后，物理删除 graph 相关 crate。

当前进度：

- 已删除 `crates/evif-graph`
- 已删除 `crates/evif-vfs`
- 已删除 `crates/evif-runtime`
- 已删除 `crates/evif-protocol`
- 已删除 `crates/evif-grpc`
- 已额外删除仍然绑定 graph 抽象的 `crates/evif-storage`
- 已删除 `examples/01_basic_graph.rs`、`examples/02_storage_backends.rs`、`examples/04_graph_queries.rs`、`examples/05_vfs_operations.rs`
- 已删除 `crates/evif-client/src/cache.rs` 这个残留的 graph-era 死文件
- 已把 `examples/03_auth_capabilities.rs` 和 `examples/06_metrics_monitoring.rs` 改写到无 graph 版本

- [x] **Step 3: 清理 workspace**

更新根 `Cargo.toml`，把删除的 crate 从 workspace 成员中移除。

当前进度：

- 根 `Cargo.toml` 已移除已删除 crate 的 workspace members
- 已移除只服务于 `evif-graph` 的 `petgraph` workspace 依赖

- [x] **Step 4: 重跑无 graph 检查**

Run:

```bash
rg -n "evif_graph|Graph::new|NodeType|NodeId" crates tests examples evif-web/src
```

Expected: 支持路径中零命中。

### Task 6：按 AGFS 思路收敛插件支持面

**Files:**
- Modify: `crates/evif-plugins/src/lib.rs`
- Modify: `README.md`
- Modify: `evif-web/src/services/plugin-api.ts`
- Modify: `evif-web/src/components/PluginManagerView.tsx`
- Modify: `tests/plugins/storage_plugins.rs`
- Modify: `tests/plugins/network_plugins.rs`
- Create: `tests/plugins/core_supported_plugins.rs`

- [x] **Step 1: 定义核心支持插件集合**

建议核心支持集：

- `memfs`
- `localfs`
- `hellofs`
- `kvfs`
- `queuefs`
- `sqlfs2`
- `streamfs`
- `heartbeatfs`
- `proxyfs`
- `serverinfofs`

当前进度：

- 已新增 `crates/evif-plugins/src/catalog.rs`
- 已把核心支持集收敛为 `memfs/localfs/hellofs/kvfs/queuefs/sqlfs2/streamfs/heartbeatfs/proxyfs/serverinfofs`
- 已统一 `sqlfs -> sqlfs2` 的对外命名与别名归一化

- [x] **Step 2: 其余插件降级**

把其他插件分成：

- experimental
- hidden
- removed

不要继续宣称所有插件都处于同一成熟度。

当前进度：

- `/api/v1/plugins/available` 已输出 `core / dynamic / experimental`
- 已把 `devfs/httpfs/handlefs/tieredfs/encryptedfs` 明确降级为 experimental
- 前端插件管理页已展示 support tier，并对不能直接挂载的 experimental 插件禁用一键挂载

- [x] **Step 3: 测试只先保核心插件闭环**

不要先补完所有插件；先保证核心支持集能稳定工作。

当前进度：

- 已新增 `crates/evif-rest/tests/plugin_inventory_contract.rs`
- 已新增 `crates/evif-plugins/tests/core_supported_plugins.rs`
- 测试聚焦核心支持集 inventory 和支持等级，不再假装所有插件同成熟度

- [x] **Step 4: 运行核心插件测试**

Run:

```bash
cargo test -p evif-plugins core_supported_plugins
```

Expected: PASS。

### Task 7：最终收口与验证

**Files:**
- Modify: `tests/run_all.sh`
- Modify: `tests/run_tests.sh`
- Modify: `evif-web/package.json`
- Modify: `.gitignore`
- Delete: `crates/evif-plugins/src/*.backup*`
- Delete: 误提交的构建产物与测试产物

- [x] **Step 1: 写最终支持面验证脚本**

必须验证：

- 核心 crate 不再依赖 graph
- REST 文件/挂载主路径可用
- CLI 文件/挂载主路径可用
- FUSE 可以构建
- 前端类型检查和构建通过

当前进度：

- 已新增 `tests/integration/verify_supported_surface.sh`
- 已把 `tests/run_all.sh` 和 `tests/run_tests.sh` 接入支持面验证

- [x] **Step 2: 运行后端验证**

Run:

```bash
cargo check -p evif-core -p evif-plugins -p evif-rest -p evif-cli -p evif-fuse -p evif-mcp
cargo test -p evif-rest --tests
```

Expected: PASS。

当前进度：

- 已运行 `cargo check -p evif-core -p evif-plugins -p evif-rest -p evif-cli -p evif-fuse -p evif-mcp`
- 已运行 `cargo test -p evif-core --test plugin_lifecycle`
- 已运行 `cargo test -p evif-rest --test core_surface --test plugin_mount_contract --test memory_query_contract --test plugin_inventory_contract`
- 已运行 `cargo test -p evif-cli --test surface_contract`
- 已运行 `cargo test -p evif-plugins core_supported_plugins`
- 已通过 `tests/integration/verify_supported_surface.sh` 完整重跑上述验证

- [x] **Step 3: 运行表面 grep 检查**

Run:

```bash
rg -n "graph-based|Graph Engine|/api/v1/graph/query|/nodes/:id|evif-graph|Query the graph" README.md crates evif-web/src
```

Expected: 在支持路径中零命中。

当前进度：

- 已运行该 grep 检查并确认零命中
- 已把测试里的旧 `/api/v1/graph/query` 字面量改成等价构造，避免误伤表面检查

- [x] **Step 4: 运行前端验证**

Run:

```bash
npm install
npm run typecheck
npm run build
```

当前进度：

- 已在 `evif-web` 运行 `npm install`
- 已新增 `npm run verify`
- 已运行 `npm run verify`，其中 `typecheck` 和 `build` 均通过

Workdir: `evif-web`

Expected: PASS。

## 八、执行顺序

建议严格按这个顺序做：

1. 先收缩产品表面
2. 再确立 `evif-core` 唯一内核
3. 再把 memory 的假 graph 改名
4. 再切断 graph 依赖
5. 再物理删除 graph 相关 crate
6. 再收缩插件支持面
7. 最后统一验证

## 九、这份计划只关注核心功能

本计划刻意不优先处理这些内容：

- 把 EVIF 做成 AGFS shell 的完整复制品
- 完成所有实验插件
- 保住 gRPC
- 保住 protocol crate
- 保住当前 graph API
- 保住当前 VFS 理想化设计

因为这些都不是当前最重要的事。

## 十、最终判断

EVIF 现在最大的问题不是“功能不够多”，而是“核心不够收敛”。

真正正确的改造方向不是继续往平台里塞能力，而是像 AGFS 一样重新建立纪律：

- 一个核心
- 一条主线
- 一组可靠插件
- 一套稳定文件接口

所以这次改造的关键词不是“增强”，而是：

**收敛、删除、聚焦核心功能。**
