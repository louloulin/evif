# EVIF vs AGFS Real Gap Analysis and Rust-First Convergence Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `pj1.md` 已完成核心收敛之后，基于对 AGFS 全代码的真实对比，找出 EVIF 还剩下的核心差距，并用**尽量复用现有 Rust 代码、最小改造**的方式继续收敛成 AGFS-style 产品主线。

**Architecture:** 后续目标不是“继续扩平台”，而是把 EVIF 固定成一条 Rust 主线：`evif-rest + evif-client + evif-cli(repl/script) + evif-fuse + core plugins + dynamic/wasm plugin ABI`。不新增 Python shell，不再引入第二套运行时，不把 experimental 能力重新推回平台中心。

**Tech Stack:** Rust workspace、Axum、Reqwest、Reedline、fuser、动态插件 ABI、WASM 插件。AGFS 的 Go/Python 代码只作为产品和架构基线，不作为 EVIF 下一阶段的实现语言。

---

## 一、对比基线

### 1. 对比仓库

- AGFS：`/Users/louloulin/Documents/linchong/claude/agfs`
- EVIF：`/Users/louloulin/Documents/linchong/claude/evif`

### 2. 本次真实对比阅读的核心文件

AGFS：

- `agfs-server/pkg/filesystem/filesystem.go`
- `agfs-server/pkg/mountablefs/mountablefs.go`
- `agfs-server/pkg/handlers/handlers.go`
- `agfs-server/pkg/handlers/plugin_handlers.go`
- `agfs-server/pkg/plugin/plugin.go`
- `agfs-fuse/pkg/fusefs/fs.go`
- `agfs-shell/ARCHITECTURE.md`
- `agfs-shell/agfs_shell/*`
- `agfs-server/Dockerfile`

EVIF：

- `crates/evif-core/src/plugin.rs`
- `crates/evif-core/src/radix_mount_table.rs`
- `crates/evif-rest/src/routes.rs`
- `crates/evif-rest/src/server.rs`
- `crates/evif-cli/src/cli.rs`
- `crates/evif-cli/src/commands.rs`
- `crates/evif-cli/src/repl.rs`
- `crates/evif-cli/src/script.rs`
- `crates/evif-client/src/client.rs`
- `crates/evif-fuse/src/lib.rs`
- `docs/plugin-development.md`
- `Dockerfile`

### 3. 量化信号

- AGFS server Go：约 `34,196` 行
- AGFS shell Python：约 `21,871` 行
- AGFS fuse Go：约 `1,829` 行
- AGFS 主路径合计：约 `57,896` 行
- EVIF Rust crates：约 `57,097` 行
- EVIF web TS/TSX：约 `15,394` 行

这说明：**EVIF 现在并不比 AGFS “更小更专注”，反而是总量已经不小，但产品闭环仍没形成。**

- AGFS TODO-like 命中：`89`
- EVIF TODO-like 命中：`207`

这说明：**EVIF 当前依然有明显更高的未完成密度。**

## 二、先说清楚：EVIF 已经解决的差距

`pj1.md` 完成后，下面这些已经不是当前主矛盾：

1. **graph 主线已经从支持路径中移除**
   - `tests/integration/no_graph_deps.sh`
   - `tests/integration/no_graph_left.sh`

2. **挂载内核方向已经对齐 AGFS**
   - AGFS：`MountableFS`
   - EVIF：`RadixMountTable`

3. **插件生命周期接口基本对齐**
   - AGFS：`Validate + Initialize + Shutdown`
   - EVIF：`validate_and_initialize_plugin()` + `shutdown()`

4. **REST 核心表面已经不再被 graph 劫持**

所以，EVIF 当前的真实问题已经不是 “graph 没删干净”，而是：

**核心已经收敛，但产品主线还没彻底闭环。**

## 三、真实差距：EVIF 相比 AGFS 还差什么

### 差距 1：AGFS 是完整产品线，EVIF 还只是“服务端为主、其余接口并列存在”

AGFS 的产品线是一条：

- `agfs-server`
- `agfs-shell`
- `agfs-fuse`

而且交付链也围绕这条主线：

- `agfs-server/Dockerfile` 同时构建 server、shell、fuse
- runtime 镜像直接包含 `agfs-shell` 和 `agfs-fuse`

EVIF 当前不是这样：

- `Dockerfile` 只交付 `evif-rest` 和 `evif-cli`
- `evif-web` 单独存在
- `evif-fuse` 不在默认交付链里
- `evif-cli` 既不是 AGFS shell 等价物，也不是独立 shell 产品

**结论：EVIF 现在已经有多个访问面，但它们还没有被组织成一条统一产品线。**

### 差距 2：AGFS shell 已经是成熟运行时，EVIF 在 Rust 里已经有雏形，但只做到了半截

AGFS shell 的能力来自一整套运行时，而不是命令列表：

- `parser.py`
- `lexer.py`
- `executor.py`
- `pipeline.py`
- `control_parser.py`
- `shell.py`

它支持：

- pipelines
- redirection
- variables
- command substitution
- control flow
- functions
- scripts
- background jobs

EVIF 这里不能说“完全没有”，因为现在已经有 Rust 雏形：

- `crates/evif-cli/src/repl.rs`
- `crates/evif-cli/src/script.rs`

但它们的完成度明显不够：

1. `repl.rs` 自己写了注释：
   - “简化实现：仅支持外部命令管道”
   - built-in command 不支持 pipe

2. `script.rs` 现在仍然有明显占位：
   - “Full EVIF command integration requires EvifClient”
   - 只支持很小的命令子集
   - 解析和执行模型还是半成品

3. `evif-cli` 的主模型仍然是 clap 子命令，而不是 shell runtime。

**结论：相对 AGFS，EVIF 缺的不是一套新语言的 shell，而是把现有 Rust REPL/script 雏形补成真正可用的 shell/runtime。**

### 差距 3：AGFS FUSE 是薄客户端，EVIF FUSE 还是厚接入点

AGFS：

- `agfs-fuse/pkg/fusefs/fs.go`
- 通过 SDK/HTTP 消费 server
- 主要做 cache / handle / FUSE glue

EVIF：

- `crates/evif-fuse/src/lib.rs`
- 直接依赖 `RadixMountTable`
- 直接 lookup plugin
- 直接调 plugin `stat/read/write/readdir`

这会导致：

- server/cli/fuse 三条路径不是同一语义源
- FUSE 成为第二套内核接入点
- server 和 FUSE 难以保持一致

**结论：如果要继续向 AGFS 靠拢，EVIF 下一步应当把 FUSE 改成基于 `evif-client`/REST 的消费者，而不是继续内嵌服务端插件语义。**

### 差距 4：AGFS 的插件生态是“平台级”，EVIF 现在仍然主要是“仓库内插件”

AGFS 已经把外部插件开发面做得比较完整：

- 动态插件接口：`pkg/plugin/plugin.go`
- loader：`pkg/plugin/loader/*`
- Rust FFI SDK：`examples/hellofs-rust/agfs-ffi/*`
- Rust WASM SDK：`examples/agfs-wasm-ffi/*`
- C/C++/WASM 示例

EVIF 当前虽然有：

- `docs/plugin-development.md`
- `crates/example-dynamic-plugin`
- `examples/wasm-plugin`

但还停在“机制存在、平台面不完整”的状态：

1. `docs/plugin-development.md` 仍然混有“去 `evif-rest` 手工加内置插件分支”的流程
2. `crates/example-dynamic-plugin` 是示例，不是稳定 SDK
3. `examples/wasm-plugin` 是例子，不是完整的 Rust/WASM plugin authoring kit

**结论：EVIF 已经有插件 ABI 的基础，但还没有形成 AGFS 那种可持续复用的外部插件开发面。**

### 差距 5：EVIF 的核心插件支持面已经定义出来，但行为闭环还没完成

`pj1.md` 之后，EVIF 现在已经有：

- core / dynamic / experimental 分类
- core plugin inventory
- plugin inventory contract tests

这一步方向是对的，但还停在“支持面治理”，没有完全进入“行为闭环”。

证据很直接：

- `crates/evif-plugins/tests/sqlfs2_tests.rs` 还是 placeholder
- `tests/plugins/storage_plugins.rs` 里大量 `todo!()`
- `tests/plugins/network_plugins.rs` 里大量 `todo!()`

而 AGFS 的 `queuefs/sqlfs2/proxyfs/...` 至少已经被组织成：

- 插件实现
- 插件 README
- 一部分插件自测/集成测试

**结论：EVIF 现在已经知道哪些插件是核心，但还没有把这些插件的核心行为真正打实。**

### 差距 6：AGFS 的 shell/FUSE/插件生态围绕 server 旋转，EVIF 现在仍然有“多头产品面”

AGFS 的所有外围都围绕 server：

- shell 消费 server
- FUSE 消费 server
- 插件接口服务于 server

EVIF 当前还有多头面：

- `evif-rest`
- `evif-cli`
- `evif-web`
- `evif-fuse`
- `evif-mcp`

这里不是说这些都不该有，而是说：

**当前它们还没有被清晰分成“核心产品面”和“附属/可选面”。**

如果继续扩下去，EVIF 很容易重新回到 `pj1` 之前那种“什么都想做”的状态。

## 四、关键修正：下一阶段不能照搬 AGFS 的实现语言

这里必须明确修正上一版思路：

- AGFS 的 shell 是 Python，不代表 EVIF 也应该再引入一套 Python shell
- 你明确要求“基于 Rust 开发而不是其他语言”
- 从当前代码现实看，**Rust-first 也是更小改造路径**

原因：

1. EVIF 已经有现成的 Rust 客户端：
   - `crates/evif-client/src/client.rs`

2. EVIF 已经有现成的 Rust REPL 雏形：
   - `crates/evif-cli/src/repl.rs`

3. EVIF 已经有现成的 Rust script 雏形：
   - `crates/evif-cli/src/script.rs`

4. EVIF 已经有 Rust FUSE 和 Rust plugin ABI

所以最小改造路线不是：

- 新起一个 Python shell 仓库

而是：

- **继续在 Rust 里把 `evif-cli` 收敛成真正的 shell/runtime**
- **把 `evif-client` 补齐为共享消费层**
- **把 `evif-fuse` 改成 thin client**

## 五、Rust-first、最小改造、最大复用的推荐路线

### 核心判断

下一阶段不应该做的事：

- 不要新建 Python shell
- 不要继续给 `evif-cli` 只堆 clap 子命令
- 不要再把更多逻辑嵌到 `evif-fuse` 里
- 不要先补全所有 experimental 插件

下一阶段应该做的事：

1. **先把核心插件行为闭环打实**
2. **把 `evif-cli` 的 REPL/script 真正做成 Rust shell runtime**
3. **把 FUSE 改成基于 `evif-client` 的 thin client**
4. **把现有动态/WASM 插件能力整理成 Rust-first SDK 和 smoke path**
5. **最后统一交付链**

## 六、实施计划

### Task 1：把核心插件从“库存已定义”推进到“行为可验证”

**Files:**
- Modify: `crates/evif-plugins/Cargo.toml`
- Modify: `crates/evif-plugins/src/localfs.rs`
- Modify: `crates/evif-plugins/src/queuefs.rs`
- Modify: `crates/evif-plugins/src/sqlfs.rs`
- Modify: `crates/evif-plugins/src/streamfs.rs`
- Modify: `crates/evif-plugins/src/heartbeatfs.rs`
- Modify: `crates/evif-plugins/src/proxyfs.rs`
- Modify: `crates/evif-plugins/src/serverinfofs.rs`
- Delete: `crates/evif-plugins/tests/sqlfs2_tests.rs`
- Delete: `tests/plugins/storage_plugins.rs`
- Delete: `tests/plugins/network_plugins.rs`
- Create: `crates/evif-plugins/tests/queuefs_behavior.rs`
- Create: `crates/evif-plugins/tests/sqlfs2_behavior.rs`
- Create: `crates/evif-plugins/tests/streamfs_behavior.rs`
- Create: `crates/evif-plugins/tests/heartbeatfs_behavior.rs`
- Create: `crates/evif-plugins/tests/proxyfs_behavior.rs`
- Create: `crates/evif-plugins/tests/serverinfofs_behavior.rs`
- Test: `cargo test -p evif-plugins`

- [x] **Step 1: 先把 placeholder 和 todo 测试替换成失败的真实行为测试**

Run:

```bash
cargo test -p evif-plugins sqlfs2_behavior
```

Expected: FAIL，因为当前 `sqlfs2` 测试仍是 placeholder。

- [x] **Step 2: 按 AGFS 对位最强的插件语义补最小闭环**

优先顺序：

- `queuefs`
- `sqlfs2`
- `streamfs`
- `heartbeatfs`
- `proxyfs`
- `serverinfofs`

先不扩功能，只先把已有能力做扎实。

- [x] **Step 3: 删除误导性的“大而空”测试表**

`tests/plugins/storage_plugins.rs` 和 `tests/plugins/network_plugins.rs` 现在主要制造“好像覆盖很多”的假象，应该拆掉。

- [x] **Step 4: 重跑插件行为测试**

Run:

```bash
cargo test -p evif-plugins
```

Expected: PASS。

当前进度：

- [x] 已新增 `crates/evif-plugins/tests/queuefs_behavior.rs`
- [x] 已新增 `crates/evif-plugins/tests/sqlfs2_behavior.rs`
- [x] 已新增 `crates/evif-plugins/tests/heartbeatfs_behavior.rs`
- [x] 已新增 `crates/evif-plugins/tests/proxyfs_behavior.rs`
- [x] 已新增 `crates/evif-plugins/tests/serverinfofs_behavior.rs`
- [x] 已新增 `crates/evif-plugins/tests/streamfs_behavior.rs`
- [x] 已删除占位的 `crates/evif-plugins/tests/sqlfs2_tests.rs`
- [x] 已把 `sqlfs2` 核心支持接到默认 `evif-plugins` 测试面，并把插件运行时名称从 `sqlfs` 统一到 `sqlfs2`
- [x] 已修复 `queuefs`/`heartbeatfs` 的 README 暴露与不存在路径 `readdir` 行为
- [x] 已把 `proxyfs` 的远端协议从旧 AGFS 风格路径改成消费当前 EVIF REST 契约
- [x] 已修复 `serverinfofs` 的未知路径 `stat` 与非根目录 `readdir` 行为
- [x] 已修复 `streamfs` 根目录 `stat("/")` 行为
- [x] 已顺手修复现有 `handlefs` 测试暴露出的 `localfs` 写入句柄 bug，使 `cargo test -p evif-plugins` 可通过
- [x] 已删除 `tests/plugins/storage_plugins.rs` 与 `tests/plugins/network_plugins.rs` 两份误导性的 todo 大表
- [x] 已验证 `cargo test -p evif-plugins`
- [x] 已验证 `bash tests/integration/verify_supported_surface.sh`

### Task 2：不要另起语言栈，直接把 `evif-cli` 收敛成 Rust shell/runtime

**Files:**
- Modify: `crates/evif-cli/src/cli.rs`
- Modify: `crates/evif-cli/src/commands.rs`
- Modify: `crates/evif-cli/src/repl.rs`
- Modify: `crates/evif-cli/src/script.rs`
- Create: `crates/evif-cli/src/lexer.rs`
- Create: `crates/evif-cli/src/parser.rs`
- Create: `crates/evif-cli/src/pipeline.rs`
- Create: `crates/evif-cli/src/ast.rs`
- Create: `crates/evif-cli/src/executor.rs`
- Create: `crates/evif-cli/tests/repl_pipeline_contract.rs`
- Create: `crates/evif-cli/tests/script_contract.rs`
- Create: `crates/evif-cli/tests/redirection_contract.rs`
- Test: `cargo test -p evif-cli`

- [x] **Step 1: 先写失败测试，锁定当前 REPL/script 的真实缺口**

至少覆盖：

- built-in command 不能进入 pipe
- script executor 不能真正通过 `EvifCommand` 完整执行
- redirection 只支持极少数情况

- [ ] **Step 2: 把 `repl.rs` 从“外部命令 pipe demo”升级为真正的内置命令执行器**

不要追求一次做出 AGFS shell 全功能。

第一阶段只做：

- pipelines
- redirection
- variables
- command substitution
- source/script

先不做函数、job control、完整 bash 兼容。

- [x] **Step 3: 复用现有 `EvifCommand` 和 `EvifClient`，不要另造第二套命令系统**

原则：

- `commands.rs` 继续是动作执行层
- 新增的 parser/executor 只负责把 shell 语法编排到这些动作上

- [x] **Step 4: 重跑 CLI/shell 测试**

Run:

```bash
cargo test -p evif-cli
```

Expected: PASS。

当前进度：

- [x] 已新增内置命令输出型复用层，避免在 `repl.rs` 里复制命令语义：
  - `EvifCommand::ls_output`
  - `EvifCommand::cat_output`
  - `EvifCommand::echo_output`
  - `EvifCommand::pwd_output`
- [x] 已新增正式 `script` CLI 子命令，复用现有 `ScriptExecutor::execute_script_with_client`
- [x] 已让 `repl.rs` 支持最小 shell-line 执行层：
  - built-in 作为管道首段（当前覆盖 `echo/cat/ls/pwd`）
  - 基础 `>` / `>>` 输出重定向
- [x] 已新增并通过相关测试：
  - `repl::tests::test_builtin_echo_can_feed_external_pipeline`
  - `repl::tests::test_builtin_echo_can_redirect_to_local_file`
  - `integration_tests::test_cli_script_execution`
- [x] 已运行 `cargo test -p evif-cli`
- [x] 已运行 `bash tests/integration/verify_supported_surface.sh`
- [ ] `command substitution` 仍未进入这轮最小实现
- [ ] 仍未拆出 `lexer/parser/pipeline/ast/executor` 独立模块，这部分留待 Task 2 后续继续收敛

### Task 3：把 `evif-fuse` 从厚接入点改成 thin client

**Files:**
- Modify: `crates/evif-fuse/src/lib.rs`
- Modify: `crates/evif-fuse/src/bin/evif-fuse-mount.rs`
- Create: `crates/evif-fuse/src/client_fs.rs`
- Create: `crates/evif-fuse/tests/client_backed_fuse.rs`
- Modify: `crates/evif-client/src/client.rs`
- Test: `cargo test -p evif-fuse`

- [ ] **Step 1: 先给 `evif-client` 补 FUSE 需要的只读接口**

第一阶段先满足：

- `stat`
- `readdir`
- `read`
- path lookup 相关错误映射

- [ ] **Step 2: 在 `evif-fuse` 里增加 client-backed 路径**

不要一上来就删嵌入式模式。

推荐策略：

- 先加新实现
- 用配置切换
- 验证稳定后再把 client-backed 变成默认路径

- [ ] **Step 3: 先完成只读核心闭环**

只读闭环稳定后，再处理写路径。

- [ ] **Step 4: 跑 FUSE 测试**

Run:

```bash
cargo test -p evif-fuse
```

Expected: PASS。

### Task 4：基于现有 Rust ABI/WASM 例子整理 Rust-first 插件开发面

**Files:**
- Modify: `docs/plugin-development.md`
- Modify: `crates/example-dynamic-plugin/src/lib.rs`
- Modify: `examples/wasm-plugin/src/lib.rs`
- Modify: `crates/evif-core/src/plugin.rs`
- Create: `crates/evif-core/tests/dynamic_plugin_smoke.rs`
- Create: `docs/plugin-sdk-rust.md`
- Test: `cargo test -p evif-core`

- [ ] **Step 1: 先把文档改成“Rust-first 外部插件流程”**

删除或降级这些旧思路：

- “去 evif-rest 手工加内置插件分支”
- 把仓库内置插件开发和外部动态插件开发混为一谈

- [ ] **Step 2: 先用现有 ABI 路线做 smoke path，而不是先拆新 SDK crate**

这是最小改造关键点：

- 先不急着创建很多新 crate
- 先让 `example-dynamic-plugin` 和 `examples/wasm-plugin` 成为真正可加载、可验证的基线

- [ ] **Step 3: 如果 smoke path 稳定，再考虑是否从 `evif-core` 抽出薄 SDK**

只有在 `evif-core` 暴露面明显不适合插件作者时，才新增 `evif-plugin-sdk`。

- [ ] **Step 4: 跑动态插件 smoke tests**

Run:

```bash
cargo test -p evif-core
```

Expected: PASS，并能覆盖 example plugin load smoke case。

### Task 5：统一交付链，但继续坚持 Rust 主线最小化

**Files:**
- Modify: `Dockerfile`
- Modify: `docker-compose.yml`
- Modify: `README.md`
- Modify: `README-CN.md`
- Modify: `tests/integration/verify_supported_surface.sh`
- Create: `scripts/smoke_release.sh`

- [ ] **Step 1: 明确主产品面**

下一阶段只交付：

- `evif-rest`
- `evif-cli`（包含 shell/repl/script 运行时）
- `evif-fuse`

`evif-web`、`evif-mcp` 继续保留，但不再定义产品主叙事。

- [ ] **Step 2: 让 Dockerfile 至少覆盖主路径构建**

最小目标不是复制 AGFS 的所有打包细节，而是让主产品面都能从同一套发布链出来。

- [ ] **Step 3: 更新 release smoke**

至少验证：

- REST 启动
- CLI/REPL 可用
- FUSE 可构建

- [ ] **Step 4: 跑交付 smoke**

Run:

```bash
bash scripts/smoke_release.sh
```

Expected: PASS。

## 七、执行顺序

建议按这个顺序：

1. 先补核心插件行为闭环
2. 再把 `evif-cli` 的 Rust REPL/script 做实
3. 再薄化 FUSE
4. 再整理 Rust-first 插件开发面
5. 最后统一交付链

## 八、这一阶段不该优先做的事

- 不要新起 Python shell
- 不要先补完所有 experimental 插件
- 不要继续扩大 web 面
- 不要在 FUSE 中继续叠更多本地插件逻辑
- 不要为了“看起来像 AGFS”而照搬 Go/Python 技术栈

## 九、最终判断

`pj1.md` 完成后，EVIF 的核心方向已经对了，但和 AGFS 全代码相比，仍然还有真实差距。

这些差距里，**最大的问题已经不是架构主线错误，而是产品闭环不完整**：

- Rust REPL/script 只做到一半
- FUSE 仍然太厚
- 核心插件行为没有彻底打实
- 外部插件开发面还是半成品
- 交付链没有统一主线

在你的约束下，最优路线不是复制 AGFS 的语言结构，而是：

**继续基于现有 Rust 代码，把 EVIF 收敛成一条 Rust-first 的 AGFS-style 产品主线。**
