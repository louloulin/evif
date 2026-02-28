# EVIF 全代码真实功能分析

**日期**: 2026-01-31  
**范围**: 全仓库 Rust crates + evif-web + 入口与数据流

---

## 一、项目定位与架构总览

### 1.1 项目是什么

- **EVIF**：Everything Is a File，基于 Rust 的「万物皆文件」图文件系统实现，对标 AGFS。
- **核心抽象**：**插件 (EvifPlugin)** + **挂载表 (RadixMountTable)**，路径经 Radix 最长前缀匹配到插件，由插件完成 create/read/write/readdir/stat/remove/rename 等 POSIX 风格操作。
- **当前主入口**：**evif-rest**（HTTP REST + WebSocket），启动时写死挂载 MemFs(/mem)、HelloFs(/hello)、LocalFs(/local)，无配置文件、无动态挂载。

### 1.2 两条技术栈（并行存在）

| 栈 | 用途 | 主要 crates | 当前是否为主路径 |
|----|------|-------------|------------------|
| **插件 + REST** | Web/CLI 文件操作 | evif-core, evif-plugins, evif-rest | ✅ 是 |
| **图 + VFS** | 图引擎、VFS 抽象、FUSE、gRPC、运行时 | evif-graph, evif-vfs, evif-storage, evif-auth, evif-runtime, evif-fuse, evif-grpc | 部分使用 |

- **REST 服务** 只依赖 **evif-core + evif-plugins**，使用 `RadixMountTable` 与 `EvifPlugin`，**未使用** evif-vfs、evif-graph 的业务逻辑（图相关路由全部返回 "not implemented"）。
- **evif-fuse** 使用 **evif-core::RadixMountTable** + fuser，将同一套插件挂载表以 FUSE 形式暴露。
- **evif-graph** 被 evif-storage、evif-vfs、evif-protocol、evif-client、evif-cli、evif-rest（仅依赖）、evif-fuse（仅依赖）引用，但 **REST 与 FUSE 主路径均不执行图查询**，图 API 在 REST 中为占位实现。

---

## 二、Crate 级真实功能

### 2.1 evif-core

- **职责**：插件 trait、挂载表、服务端运行时所需基础设施。
- **真实使用**：
  - **plugin.rs**：`EvifPlugin`（create/mkdir/read/write/readdir/stat/remove/rename/remove_all/symlink/readlink）、`FileInfo`、`WriteFlags`、`OpenFlags`；可选 `HandleFS`、`FileHandle`。
  - **radix_mount_table.rs**：`RadixMountTable`（mount/lookup/list_mounts），路径规范化与最长前缀匹配。
  - **server.rs**：`EvifServer` 仅类型与概念，实际启动在 evif-rest。
  - **handle_manager.rs**：全局句柄管理，供 REST Handle API 使用。
  - **batch_operations.rs**：批量复制/删除，供 REST batch 路由使用。
  - **cache/**：元数据/目录缓存抽象。
  - **config / config_validation / monitoring / file_monitor / acl**：配置、监控、审计等基础设施，**当前 REST 启动未用**。
- **可选**：`extism_plugin`（wasm feature）用于 WASM 插件加载。

### 2.2 evif-plugins

- **职责**：实现 `EvifPlugin` 的各种后端。
- **默认编译且被 REST 使用**：`MemFsPlugin`、`HelloFsPlugin`、`LocalFsPlugin`（server.rs 写死挂载）。
- **默认编译但未在 server 中挂载**：Kvfs, QueueFs, ServerInfoFs, HttpFs, StreamFs, ProxyFs, DevFs, HeartbeatFs, HandleFs 等。
- **按 feature 编译**：s3fs, sqlfs, gptfs, vectorfs, streamrotatefs, opendal/s3fs_opendal, azureblobfs, gcsfs, aliyunossfs, tencentcosfs, huaweiobsfs, miniofs 等；部分 webdav/ftp/sftp 注释掉。
- **evif-storage**：仅部分插件（如需要持久化或索引的）依赖，REST 主路径不直接依赖 storage。

### 2.3 evif-rest

- **入口**：`main.rs` → `EvifServer::run()`（server.rs）。
- **启动逻辑**：`RadixMountTable::new()`，挂载 `/mem`、`/hello`、`/local`，无配置、无动态加载。
- **路由分层**：
  - **/api/v1/fs/*（compat_fs）**：供 **evif-web** 使用，返回 `nodes`/`content` 等；list/read/write/create/delete 全实现。
  - **/api/v1/files、/api/v1/directories、/api/v1/stat、/api/v1/rename、/api/v1/grep、/api/v1/digest、/api/v1/touch**：handlers 层，供 **evif-client（CLI）** 使用，mount_table + plugin。
  - **/api/v1/mounts**：列出挂载点，实现。
  - **/api/v1/mount、/api/v1/unmount、/api/v1/plugins/load**：返回 “not yet supported”，仅服务端配置有效。
  - **/api/v1/plugins、/api/v1/plugins/list、/api/v1/plugins/wasm/load、unload**：列出/加载/卸载插件，wasm 有部分实现。
  - **/api/v1/handles/***：句柄 open/read/write/seek/sync/close，需插件实现 HandleFS。
  - **/api/v1/metrics/***：流量/操作/状态/重置，有实现。
  - **/health**：健康检查。
  - **/ws**：WebSocket 终端，ls/cat/stat/mounts/pwd/echo/clear/help，实现完整。
  - **/nodes/*、/query、/stats**：图 API，**全部返回 "Graph functionality not implemented"**，仅为占位。
- **依赖**：evif-core、evif-plugins、evif-vfs（仅 RestError 转换）、evif-graph（仅依赖，未用图逻辑）。

### 2.4 evif-web（前端）

- **实际接入**：仅使用 **/api/v1/fs/list|read|write|create|delete** 与 **/ws**；Vite 代理到后端 8081。
- **已用组件**：App、MenuBar、ActivityBar、FileTree、Editor（Monaco）、Panel（终端/问题/输出）、StatusBar、ContextMenu；终端内输入通过 WebSocket 发到后端执行。
- **未接入 App 的模块**：plugin-manager（PluginList, MountModal, PluginModal 等）、monitor（SystemStatus, TrafficChart, LogViewer 等）、collaboration（评论/权限/分享等）、editor 子组件（MiniMap, QuickOpen）、search、upload 等；多为独立 UI 或 mock，未与当前 REST 打通。

### 2.5 evif-client

- **用途**：CLI 使用的 HTTP 客户端，**直接调 REST**（reqwest），不用 gRPC。
- **API 对应**：`ls` → GET /api/v1/directories?path=；`cat`/`cat_bytes` → GET /api/v1/files?path=；`write` → PUT /api/v1/files?path=&offset=；`mkdir`/`remove`/`remove_all`/`rename`/`stat`/`health`/`mounts`/`mount`/`unmount` 等均有对应路径。
- **响应格式假设**：
  - list：`json["files"]` ✅ 与 handlers 的 `DirectoryListResponse { files }` 一致。
  - read：`json["data"]` 为 **base64** ❌ 与 handlers 的 `FileReadResponse { content: String }` 不一致，**CLI cat 会解析失败**。
  - write：PUT body 为 base64；handlers 的 write_file 期望 JSON body（FileWriteRequest），**格式不一致**。
- **同步构造**：`new_sync` 使用 `DummyTransport`，但 ls/cat/write 等走 `http_client` + `base_url`，故 **CLI 实际发 HTTP**；若 base_url 指到 8081，则与 REST 连通性取决于上述格式是否适配。

### 2.6 evif-cli

- **入口**：clap 解析，默认 `--server localhost:50051`（命名像 gRPC，实际 commands 用 evif-client 的 HTTP）。
- **命令**：Ls, Cat, Cp, Stats, Repl, Get, Create, Delete, Mount 等；Ls/Cat/Cp 等走 `EvifCommand` → `EvifClient` 的 HTTP 方法。
- **Mount**：调 FUSE 挂载（evif-fuse），需传入 RadixMountTable；CLI 侧若未注入挂载表则只能依赖本地 FUSE 二进制与配置。

### 2.7 evif-fuse

- **职责**：用 fuser 实现 FUSE，将 **同一套 RadixMountTable + EvifPlugin** 挂载到本地目录。
- **入口**：`evif-fuse-mount` 二进制，需传入挂载表与根路径；与 evif-rest 共享插件模型，但不共享进程（需单独起 FUSE 进程并注入挂载表）。

### 2.8 evif-graph

- **职责**：图数据结构与算法（Node, Edge, Graph, Query, Executor 等）。
- **在 REST/Web 主路径中的使用**：无；REST 仅返回 “Graph functionality not implemented”。
- **被引用处**：evif-storage、evif-vfs、evif-protocol、evif-client（类型）、evif-cli、evif-rest（依赖）、evif-fuse（依赖）；用于类型或未来扩展，当前不驱动主流程。

### 2.9 evif-vfs

- **职责**：POSIX 风格 VFS 抽象（Vfs, File, Directory, INode, DEntry 等）。
- **在 REST 主路径中的使用**：无；REST 用 mount_table + EvifPlugin。
- **使用方**：evif-fuse 不直接用 evif-vfs；evif-grpc 依赖 evif-vfs（grpc server 已 disable）。

### 2.10 evif-storage

- **职责**：可插拔存储后端（Memory, Sled, RocksDB, S3 等），供需要持久化的插件或图/运行时使用。
- **在 REST 主路径中的使用**：无；REST 不直接读写 storage。
- **evif-core** 依赖 evif-storage（如 batch_operations 等可能用），evif-plugins 中部分插件依赖。

### 2.11 evif-auth / evif-runtime

- **职责**：认证/能力/审计、运行时配置与编排。
- **在 REST 主路径中的使用**：无；evif-rest 启动未读配置、未挂认证中间件。
- **使用方**：evif-grpc、evif-vfs 等，当前非主路径。

### 2.12 evif-protocol

- **职责**：消息/请求/响应/编解码（用于图/客户端协议）。
- **使用方**：evif-client（get_node 等图操作走 Message；实际 CLI 文件操作走 HTTP REST，不经过 protocol）。

### 2.13 evif-grpc

- **状态**：server 相关代码多份 .disabled/.bak，当前 **未启用**；proto 与 client 存在，但非主流程。

### 2.14 evif-mcp

- **职责**：MCP 服务器，通过 HTTP 调用 EVIF（默认 EVIF_URL=http://localhost:8080），提供工具列表等。
- **与 REST 关系**：若 EVIF_URL 指到 8081，则实际调用 evif-rest；工具实现依赖具体 HTTP 接口格式。

### 2.15 evif-macros / evif-metrics

- **职责**：过程宏、指标类型；被其他 crate 引用，不单独暴露入口。

### 2.16 evif-python

- **职责**：Python 绑定（client、exceptions、models 等），可与 REST 或后续后端对接。

---

## 三、数据流与入口总结

### 3.1 当前“主路径”（真正跑通的）

1. **启动**：`cargo run -p evif-rest` → 创建挂载表，挂载 /mem、/hello、/local，监听 8081。
2. **Web**：evif-web → Vite 代理 → 8081 → `/api/v1/fs/*`（compat_fs）+ `/ws`（终端）；文件树、编辑、保存、删除、终端命令均可用。
3. **终端**：WebSocket 收命令，ws_handlers 调 mount_table.lookup + plugin.readdir/read/stat，返回 JSON 给前端。
4. **FUSE**：单独运行 `evif-fuse-mount` 并注入相同挂载表，可把同一套插件挂到本地目录（与 REST 同模型、不同进程）。

### 3.2 部分可用或格式未对齐的

1. **CLI**：evif-client 用 HTTP 调 /api/v1/directories、/api/v1/files 等；**list 格式一致，read/write 与 handlers 返回/请求格式不一致**（client 期望 base64，handlers 为明文 JSON），需适配或统一 API 约定。
2. **REST 图 API**：/nodes、/query、/stats 存在路由但一律返回 “Graph functionality not implemented”。
3. **动态挂载/卸载**：REST 的 mount/unmount/load_plugin 返回不支持，仅服务端写死挂载有效。

### 3.3 未接通的

1. **evif-runtime / evif-auth**：未参与 REST 启动与请求处理。
2. **evif-vfs**：未在 REST 或 FUSE 主路径中替代插件模型。
3. **evif-grpc**：服务端未启用。
4. **evif-web 中**：插件管理、监控、协作、搜索、上传等组件未与当前后端 API 对接。

---

## 四、API 表面一览（evif-rest）

| 类别 | 路径/方法 | 实现状态 | 主要消费者 |
|------|-----------|----------|------------|
| 兼容 Web | GET /api/v1/fs/list, read；POST write, create；DELETE delete | ✅ 完整 | evif-web |
| 文件/目录 | GET/PUT/POST/DELETE /api/v1/files, /api/v1/directories | ✅ 完整 | evif-client（格式部分不一致） |
| 元数据/高级 | /api/v1/stat, /api/v1/rename, /api/v1/grep, /api/v1/digest, /api/v1/touch | ✅ 有 | CLI/工具 |
| 挂载 | GET /api/v1/mounts | ✅ | POST mount, unmount | ❌ 不支持 |
| 插件 | GET /api/v1/plugins, list；POST load, wasm/load, unload | 部分 | - |
| 句柄 | /api/v1/handles/* | ✅ 需 HandleFS 插件 | - |
| 批量 | batch 路由 | ✅ | - |
| 指标 | /api/v1/metrics/* | ✅ | - |
| 图 | /nodes/*, /query, /stats | ❌ 占位 | - |
| WebSocket | /ws | ✅ | evif-web Terminal |

---

## 五、结论与建议

### 5.1 真实功能概览

- **核心**：**插件 + Radix 挂载表 + REST/WebSocket/FUSE** 已打通；evif-web 与 evif-rest 的 /api/v1/fs/* 和 /ws 构成当前主用功能。
- **图/VFS/认证/运行时**：存在于仓库且被引用，但 **未参与当前 REST/Web 主路径**；图 API 仅为占位。
- **CLI**：依赖 evif-client 的 HTTP，与 handlers 的目录列表兼容；**读/写与 handlers 的响应/请求体格式不一致**，需要统一或适配。
- **前端**：大量高级组件（插件管理、监控、协作等）未接入现有 REST，需按需对接或明确为远期功能。

### 5.2 建议

1. **统一 API 契约**：evif-client 与 handlers 对 read/write 的 body/响应格式（如 content vs base64）做统一，或在后端提供兼容层。
2. **配置化挂载**：将 /mem、/hello、/local 改为配置或环境变量，便于与 evif-runtime 或未来配置系统对接。
3. **图 API**：若不需要图能力，可移除或显式标记为 deprecated；若需要，则基于 evif-graph 实现 /nodes、/query 等。
4. **evif-web**：若需插件管理、监控等，需为对应组件指定后端 API（如 /api/v1/plugins、/api/v1/metrics）并在前端接好。

以上为对 EVIF 全代码的真实功能与数据流分析，聚焦已接通路径与未接通/占位部分，便于后续排障与演进。
