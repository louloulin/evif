# EVIF 快速入门指南

欢迎使用 EVIF (Extensible Virtual File System) - 一个强大的、可扩展的图文件系统。

## 目录

- [系统要求](#系统要求)
- [安装](#安装)
- [快速开始](#快速开始)
- [基本概念](#基本概念)
- [CLI 使用](#cli-使用)
- [REST API](#rest-api)
- [插件开发](#插件开发)
- [下一步](#下一步)

## 系统要求

- Rust 1.70+ (用于从源码构建)
- macOS 或 Linux (Windows 支持正在开发中)
- 至少 512MB 可用内存

## 安装

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/your-org/evif.git
cd evif

# 构建所有组件
cargo build --release

# 安装 EVIF CLI
cargo install --path crates/evif-cli

# 安装 EVIF REST 服务器
cargo install --path crates/evif-rest
```

## 快速开始

### 1. 启动 REST 服务器

```bash
# 启动 EVIF REST 服务器 (默认端口 8081)
evif-rest

# 或者指定端口
evif-rest --port 3000
```

服务器启动后，您可以访问 `http://localhost:8081` 查看健康状态：

```bash
curl http://localhost:8081/health
```

响应：
```json
{
  "status": "healthy",
  "timestamp": "2026-02-11T03:24:45.794276+00:00",
  "version": "1.0.0"
}
```

### 2. 使用 CLI

```bash
# 启动 REPL 模式
evif

# 在 REPL 中执行命令
evif> mount localfs /local --root /tmp
evif> ls /local
evif> create /local/test.txt "Hello EVIF!"
evif> read /local/test.txt
evif> umount /local
```

### 3. 编写脚本

EVIF 支持脚本执行，使用变量和控制流：

```bash
# 脚本示例: setup.evif
set ROOT_DIR=/tmp/myproject
mount localfs /project --root $ROOT_DIR
create /project/config.json '{"name":"myproject"}'
create /project/README.md '# My Project'
ls /project
```

执行脚本：

```bash
evif source setup.evif
```

## 基本概念

### 挂载表 (Mount Table)

EVIF 使用挂载表将路径映射到不同的文件系统插件：

```
/           → (root)
  /mem      → memfs (内存文件系统)
  /local    → localfs (本地文件系统)
  /s3       → s3fs (AWS S3)
```

### 插件 (Plugins)

EVIF 的核心是插件系统，每个插件提供不同的文件系统实现：

- **memfs**: 内存文件系统，适合测试
- **localfs**: 本地文件系统
- **hellofs**: 示例插件
- **动态插件**: 运行时加载的 .so/.dylib/.dll 文件

### 图模型 (Graph Model)

EVIF 将文件和目录表示为图中的节点，边表示关系（如父子关系）。这使得复杂的查询和遍历成为可能。

## CLI 使用

### REPL 模式

启动 REPL 进行交互式操作：

```bash
evif
```

可用命令（61 个）：

- **挂载管理**: `mount`, `umount`, `ls_mounts`
- **文件操作**: `create`, `read`, `write`, `delete`, `rename`
- **目录操作**: `mkdir`, `rmdir`, `ls`, `cd`
- **元数据**: `stat`, `digest`, `touch`
- **高级功能**: `grep`, `find`, `copy`, `batch`

使用 `help` 查看所有命令。

### 批处理模式

直接执行命令：

```bash
# 创建文件
evif create /mem/test.txt "Hello World"

# 读取文件
evif read /mem/test.txt

# 列出目录
evif ls /mem
```

## REST API

### 基本操作

创建文件：
```bash
curl -X POST http://localhost:8081/api/v1/files \
  -H "Content-Type: application/json" \
  -d '{"path": "/mem/test.txt", "content": "SGVsbG8="}'
```

读取文件：
```bash
curl "http://localhost:8081/api/v1/files?path=/mem/test.txt"
```

列出目录：
```bash
curl "http://localhost:8081/api/v1/directories?path=/mem"
```

删除文件：
```bash
curl -X DELETE "http://localhost:8081/api/v1/files?path=/mem/test.txt"
```

### 挂载管理

列出挂载：
```bash
curl http://localhost:8081/api/v1/mounts
```

挂载插件：
```bash
curl -X POST http://localhost:8081/api/v1/mount \
  -H "Content-Type: application/json" \
  -d '{"plugin": "localfs", "path": "/local", "config": {"root": "/tmp"}}'
```

卸载：
```bash
curl -X POST http://localhost:8081/api/v1/unmount \
  -H "Content-Type: application/json" \
  -d '{"path": "/local"}'
```

## 插件开发

### 创建基础插件

```rust
use evif_core::{EvifPlugin, FileInfo, OpenFlags, EvifResult};
use async_trait::async_trait;

pub struct MyPlugin {
    // 插件状态
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    async fn create(&self, path: &str, mode: u32) -> EvifResult<()> {
        // 实现创建文件逻辑
        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: usize) -> EvifResult<Vec<u8>> {
        // 实现读取文件逻辑
        Ok(Vec::new())
    }

    // 实现其他必要的方法...
}
```

### 创建动态插件

动态插件可以被运行时加载：

1. 创建新的 crate：
```bash
cargo new --lib my-dynamic-plugin
```

2. 添加依赖：
```toml
[dependencies]
evif-core = { path = "../evif/crates/evif-core" }
evif-macros = { path = "../evif/crates/evif-macros" }
async-trait = "0.1"
```

3. 实现 ABI：
```rust
use evif_core::EvifPlugin;
use std::sync::Arc;

#[no_mangle]
pub static evif_plugin_abi_version: u32 = 1;

#[no_mangle]
pub extern "C" fn evif_plugin_info() -> *const u8 {
    // 返回插件信息 JSON
}

#[no_mangle]
pub extern "C" fn evif_plugin_create() -> *mut std::os::raw::c_void {
    // 创建并返回插件实例
}
```

4. 编译为动态库：
```bash
cargo build --release
# 输出: target/release/libmy_dynamic_plugin.so
```

5. 加载插件：
```bash
curl -X POST http://localhost:8081/api/v1/plugins/load \
  -H "Content-Type: application/json" \
  -d '{"path": "/path/to/libmy_dynamic_plugin.so"}'
```

## 下一步

- 📖 阅读 [完整 API 文档](https://docs.rs/evif)
- 🔌 探索 [插件开发指南](plugin-development.md)
- 📊 了解 [指标和监控](metrics.md)
- 🔗 查看 [FUSE 集成](fuse.md)
- 💬 加入社区讨论

## 故障排除

### 端口已被使用

如果端口 8081 已被占用，使用其他端口：

```bash
evif-rest --port 3000
```

### 权限错误

某些操作可能需要特殊权限。确保运行用户有必要的文件系统访问权限。

### 内存不足

对于大型文件系统操作，考虑增加系统限制或调整 EVIF 的缓存配置。

## 获取帮助

- GitHub Issues: https://github.com/your-org/evif/issues
- 文档: https://docs.rs/evif
- Discord: https://discord.gg/evif
