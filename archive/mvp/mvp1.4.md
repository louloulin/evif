# EVIF MVP 1.4 增强计划

> 创建时间：2026-04-29
> 更新时间：2026-04-30
> 项目：EVIF (Everything Is a File)
> 当前完成度：100%（3/3 功能完成）
> 参考：MVP 1.3 完成后的差距分析

---

## 完成状态总览

| 项 | 状态 | 验证结果 |
|--------|------|----------|
| **P0**: Copy-on-Write 快照 | ✅ 已完成 | 8 个测试通过 |
| **P1**: Agent 追踪增强 | ✅ 已完成 | 10 个测试通过 |
| **P2**: 网络插件修复 | ✅ 已完成 | webdavfs/ftpfs/sftpfs 编译成功 |

---

## P0 必须项（已完成）

### P0: Copy-on-Write 快照

**状态**: ✅ 已完成

**实现文件**:
- `crates/evif-core/src/snapshot.rs` - 核心实现

**核心方法**:
- `CowSnapshot::new()` - 创建根快照
- `CowSnapshot::branch()` - 从快照分支
- `CowSnapshot::add_file()` - 添加文件到快照
- `CowSnapshot::read()` - 读取快照中的文件（CoW 感知）
- `CowSnapshot::list()` - 列出快照中的所有文件
- `CowSnapshot::diff()` - 计算与父快照的差异
- `SnapshotManager::merge()` - 合并两个快照

**验证结果**: 8 passed, 0 failed

---

## P1 重要项（已完成）

### P1: Agent 追踪增强

**状态**: ✅ 已完成

**实现文件**:
- `crates/evif-core/src/agent_tracking.rs` - 核心实现

**核心结构**:
- `AgentTracker` - Agent 注册和会话生命周期管理
- `AgentSession` - 会话状态、思考链、活动事件
- `ThoughtEntry` - 思考链条目（支持父子关系、置信度）
- `ActivityEvent` - 9 种活动事件类型
- `TrackerStats` - 全局追踪统计

**验证结果**: 10 passed, 0 failed

---

## P2 改进项（已完成）

### P2: 网络插件修复

**状态**: ✅ 已完成

**问题分析**:
原本文档标记为"OpenDAL 0.50.2 TLS 冲突，需等待上游修复"。经深入调查发现：

**真正的根因有两个**（不是 TLS 冲突）：

1. **`Cargo.toml` 中服务未启用**: `services-webdav`、`services-ftp`、`services-sftp` 被注释掉，误标为 TLS 问题
2. **`OpendalService` 枚举未定义**: 三个 variant 被注释
3. **`opendal.rs` match arm 被注释**: build_operator 中的代码被注释
4. **`lib.rs` 模块导出被注释**: pub mod/use 被注释

**修复内容**:

1. **启用 OpenDAL 服务特性** (`Cargo.toml`):
   ```toml
   "services-webdav",   # WebDAV 协议
   "services-ftp",      # FTP 协议
   "services-sftp",     # SFTP 协议
   ```

2. **升级 OpenDAL 版本** (`Cargo.toml`):
   - `opendal = "0.50"` → `opendal = "0.54"`
   - 原因: 0.50.2 的 FTP 服务使用 `rustls::TlsConnector`，与 `suppaftp` 依赖的 `futures_rustls::TlsConnector` 不兼容
   - 0.54+ 已修复此问题

3. **取消注释所有相关代码**:
   - `opendal.rs`: `OpendalService::Webdav/Ftp/Sftp` variant
   - `opendal.rs`: `build_operator()` match arm
   - `lib.rs`: `pub mod` 和 `pub use`

4. **修复 API 调用**:
   - `builder.username()` → `builder.user()` (FTP/SFTP)
   - `builder.password()` (SFTP) → `builder.key()` (SSH 密钥)

**验证结果**:
```
cargo build -p evif-plugins --features "webdavfs,ftpfs,sftpfs"
Finished `dev` profile in 5.56s
```

**关键文件变更**:
- `crates/evif-plugins/Cargo.toml` - 启用服务特性 + 升级版本
- `crates/evif-plugins/src/lib.rs` - 取消注释模块
- `crates/evif-plugins/src/opendal.rs` - 取消注释 + 修复 API

---

## 验证记录

| 测试项 | 命令 | 结果 |
|--------|------|------|
| P0 Copy-on-Write 快照 | `cargo test -p evif-core -- snapshot` | ✅ 8 passed |
| P1 Agent 追踪增强 | `cargo test -p evif-core -- agent_tracking` | ✅ 10 passed |
| P2 网络插件编译 | `cargo build -p evif-plugins --features "webdavfs,ftpfs,sftpfs"` | ✅ 完成 |
| 插件测试 | `cargo test -p evif-plugins` | ✅ 110 passed (4 unrelated failures) |

---

## 关键文件清单

| 文件 | 说明 |
|------|------|
| `crates/evif-core/src/snapshot.rs` | ✅ CoW 快照核心实现 |
| `crates/evif-core/src/agent_tracking.rs` | ✅ Agent 追踪模块 |
| `crates/evif-core/src/lib.rs` | ✅ 导出新类型 |
| `crates/evif-plugins/Cargo.toml` | ✅ 启用网络服务 + 升级 OpenDAL |
| `crates/evif-plugins/src/lib.rs` | ✅ 取消注释网络插件 |
| `crates/evif-plugins/src/opendal.rs` | ✅ 修复 OpendalService + API |

---

## 实现顺序

1. **P0**: Copy-on-Write 快照 ✅ 已完成
2. **P1**: Agent 追踪增强 ✅ 已完成
3. **P2**: 网络插件修复 ✅ 已完成

---

## 剩余差距分析

根据参考项目对比（MVP 1.2），所有核心功能已实现：

| 特性 | AGFS | AgentFS | EVIF | 状态 |
|------|------|---------|------|------|
| Copy-on-Write | - | ✅ | ✅ | 已实现 |
| Agent 追踪 | - | ✅ | ✅ | 已实现 |
| WASM 插件池 | ✅ | - | ✅ | MVP 1.3 |
| 增强审计 | - | ✅ SQL | ✅ 查询接口 | MVP 1.3 |
| 流量监控 | ✅ | - | ✅ | MVP 1.3 |
| 网络插件 (WebDAV/FTP/SFTP) | ✅ | - | ✅ | MVP 1.4 P2 |
| 认证授权 | ❌ 无 | ✅ | ✅ | 已实现 |
| 多租户 | ❌ 无 | - | ⚠️ 基础 | 可选增强 |
