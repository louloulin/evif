# EVIF - 万物皆文件系统

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://opensource.org/licenses/)

> 基于 Rust 构建的强大、可扩展的插件式虚拟文件系统，遵循 Plan 9 "万物皆文件" 哲学。

[English](README.md)

## 概述

EVIF 是一个模块化的虚拟文件系统，通过插件架构为各种存储后端提供统一接口。它以插件挂载、文件/目录操作、句柄管理和多种访问方式为核心，在保持 POSIX 风格交互的同时为不同后端提供统一入口。

### 核心特性

- **插件架构**: 30+ 内置插件，支持各种存储后端
- **插件内核**: 基于挂载路由、插件生命周期和句柄管理的统一文件系统内核
- **多种访问方式**: REST API、CLI、FUSE 挂载、WebSocket
- **存储后端**: 内存、本地文件系统、S3、Azure Blob、GCS、阿里云 OSS 等
- **高级功能**: 批量操作、流式传输、加密、分层存储、监控
- **动态插件加载**: 运行时加载 `.so`/`.dylib`/`.dll` 插件
- **WASM 插件支持**: 基于 WebAssembly 的插件扩展

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      访问层                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ REST API │  │   CLI    │  │   FUSE   │  │WebSocket │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
├───────┴────────────┴─────────┴─────────┴─────────┴─────────┤
│                      核心层                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   挂载表     │  │  插件系统    │  │  句柄管理器  │     │
│  │ (Radix Tree) │  │              │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
├─────────────────────────────────────────────────────────────┤
│                    存储层 (插件)                            │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │
│  │ 内存   │ │ 本地   │ │   S3   │ │ Azure  │ │  SQL   │  │
│  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/your-org/evif.git
cd evif

# 构建所有组件
cargo build --release

# 安装 CLI 工具
cargo install --path crates/evif-cli
```

### 启动服务器

```bash
# 启动 REST API 服务器（默认端口 8081）
evif-rest

# 或指定自定义端口
evif-rest --port 3000
```

### 基本使用

```bash
# 检查服务器健康状态
evif health

# 列出根目录
evif ls /

# 在内存文件系统中创建文件
evif write /mem/hello.txt --content "Hello, EVIF!"

# 读取文件内容
evif cat /mem/hello.txt

# 创建目录
evif mkdir /mem/mydir

# 挂载本地文件系统
evif mount-plugin local /local --config root=/tmp

# 列出已挂载的插件
evif list-mounts
```

## 核心组件

| Crate | 描述 |
|-------|------|
| **evif-core** | 核心抽象、插件系统、挂载表、句柄管理器 |
| **evif-rest** | HTTP/JSON REST API 服务器 |
| **evif-cli** | 命令行接口（60+ 命令） |
| **evif-client** | Rust 客户端 SDK |
| **evif-fuse** | FUSE 文件系统集成（Linux/macOS） |
| **evif-auth** | 认证和授权层 |
| **evif-macros** | 过程宏（`#[node]`、`#[builder]`、`#[error_macro]`） |
| **evif-metrics** | Prometheus 指标收集和导出 |
| **evif-mem** | 可选 memory 子系统，提供时间线与关系查询 |

## 可用插件

### 核心支持插件
| 插件 | 描述 | 默认挂载点 |
|------|------|-----------|
| `memfs` | 内存文件系统 | `/mem` |
| `localfs` | 本地文件系统访问 | - |
| `hellofs` | Hello world 示例插件 | `/hello` |
| `serverinfofs` | 服务器状态和指标 | `/serverinfo` |
| `kvfs` | 键值存储接口 | `/kv` |
| `queuefs` | 消息队列接口 | `/queue` |
| `sqlfs2` | 基于 SQLite 的结构化数据文件系统 | `/sqlfs2` |
| `proxyfs` | 代理到其他路径 | - |
| `streamfs` | 流式数据接口 | - |
| `heartbeatfs` | 存活与心跳接口 | - |

### 云存储插件
| 插件 | 描述 | 特性标志 |
|------|------|----------|
| `s3fs` | Amazon S3 | `s3fs` |
| `azureblobfs` | Azure Blob 存储 | `azureblobfs` |
| `gcsfs` | Google Cloud Storage | `gcsfs` |
| `aliyunossfs` | 阿里云 OSS | `aliyunossfs` |
| `tencentcosfs` | 腾讯云 COS | `tencentcosfs` |
| `huaweiobsfs` | 华为 OBS | `huaweiobsfs` |
| `miniofs` | MinIO | `miniofs` |

### OpenDAL 插件 (EVIF 2.1)
基于 OpenDAL 0.50.x 实现统一存储接口。可用服务参见 `evif-plugins/src/opendal.rs`。

### 实验性插件
| 插件 | 描述 | 特性标志 |
|------|------|----------|
| `httpfs` | 基于 HTTP 的文件系统 | - |
| `devfs` | 设备与伪文件示例 | - |
| `encryptedfs` | 加密文件系统层 | - |
| `tieredfs` | 分层存储（热/温/冷） | - |
| `handlefs` | 文件句柄管理 | - |
| `gptfs` | GPT/AI 模型接口 | `gptfs` |
| `vectorfs` | 向量数据库接口 | `vectorfs` |
| `streamrotatefs` | 流轮转 | `streamrotatefs` |

## REST API

### 基础 URL
默认: `http://localhost:8081`（可通过 `EVIF_PORT` 配置）

### 文件操作

```bash
# 读取文件（返回 content + base64 data）
GET /api/v1/files?path=/mem/hello.txt&offset=0&size=0

# 写入文件
PUT /api/v1/files?path=/mem/hello.txt
Body: { "content": "base64编码数据", "encoding": "base64" }

# 删除文件
DELETE /api/v1/files?path=/mem/hello.txt

# 列出目录
GET /api/v1/directories?path=/mem

# 创建目录
POST /api/v1/directories
Body: { "path": "/mem/newdir", "parents": true }

# 文件元数据
GET /api/v1/stat?path=/mem/hello.txt
```

### 挂载管理

```bash
# 列出挂载
GET /api/v1/mounts

# 挂载插件
POST /api/v1/mount
Body: { "plugin": "localfs", "path": "/local", "config": {"root": "/tmp"} }

# 卸载
POST /api/v1/unmount
Body: { "path": "/local" }
```

### 批量操作

```bash
# 批量复制
POST /api/v1/batch/copy
Body: { "sources": ["/mem/a"], "destination": "/mem/dest", "concurrency": 4 }

# 批量删除
POST /api/v1/batch/delete
Body: { "paths": ["/mem/a", "/mem/b"] }

# 查询进度
GET /api/v1/batch/progress/:operation_id
```

### WebSocket

```bash
# 连接 WebSocket
ws://localhost:8081/ws
```

完整 API 文档请参见 [docs/API.md](docs/API.md)。

## CLI 命令

EVIF CLI 提供 60+ 命令：

### 文件操作
- `ls`, `cat`, `write`, `mkdir`, `rm`, `mv`, `cp`, `stat`, `touch`, `tree`
- `head`, `tail`, `grep`, `digest`, `wc`, `sort`, `uniq`, `cut`, `tr`, `base`

### 挂载管理
- `mount`, `umount`, `list-mounts`, `mount-plugin`, `unmount-plugin`

### 高级操作
- `upload`, `download`, `find`, `locate`, `diff`, `du`, `file`
- `ln`, `readlink`, `realpath`, `basename`, `dirname`, `truncate`, `split`
- `rev`, `tac`

### REPL 模式
```bash
evif repl
```

### 环境命令
- `env`, `export`, `unset`, `pwd`, `cd`, `echo`, `date`, `sleep`, `true`, `false`

## 插件开发

### 创建基础插件

```rust
use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult};
use async_trait::async_trait;

pub struct MyPlugin {
    // 插件状态
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    async fn create(&self, path: &str, mode: u32) -> EvifResult<()> {
        // 实现
        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        // 实现
        Ok(Vec::new())
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<i32> {
        // 实现
        Ok(data.len() as i32)
    }

    // 实现其他必要方法...
}
```

### 创建动态插件

```rust
use evif_core::EvifPlugin;

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

详细插件开发指南请参见 [docs/plugin-development.md](docs/plugin-development.md)。

## FUSE 集成

将 EVIF 挂载为用户空间文件系统：

```bash
# 挂载（默认只读）
evif mount /mnt/evif

# 启用写支持挂载
evif mount /mnt/evif --write

# 自定义缓存设置
evif mount /mnt/evif --write --cache-size 5000 --cache-timeout 120

# 卸载
evif umount /mnt/evif
```

## 配置

### 环境变量

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `EVIF_PORT` | 8081 | REST API 服务器端口 |
| `EVIF_HOST` | 0.0.0.0 | 服务器绑定地址 |
| `EVIF_LOG_LEVEL` | info | 日志级别 |
| `EVIF_CACHE_SIZE` | 10000 | Inode 缓存大小 |
| `EVIF_CACHE_TIMEOUT` | 60 | 缓存超时（秒） |

### 配置文件

EVIF 支持通过 `evif.toml` 配置：

```toml
[server]
port = 8081
host = "0.0.0.0"

[cache]
size = 10000
timeout = 60

[logging]
level = "info"
```

## 项目结构

```
evif/
├── crates/
│   ├── evif-core/        # 核心抽象和插件系统
│   ├── evif-rest/        # REST API 服务器
│   ├── evif-cli/         # CLI 工具
│   ├── evif-client/      # 客户端 SDK
│   ├── evif-fuse/        # FUSE 集成
│   ├── evif-auth/        # 认证层
│   ├── evif-macros/      # 过程宏
│   ├── evif-metrics/     # 指标收集
│   ├── evif-mem/         # 可选 memory 子系统
│   └── evif-plugins/     # 插件实现与 catalog
├── docs/                  # 文档
├── benches/               # 基准测试
├── tests/                 # 集成测试
├── examples/              # 示例代码
└── skills/                # 仓颉技能系统
```

## 性能

EVIF 使用基于 Radix 树的挂载表，实现 O(k) 路径查找，其中 k 是路径长度。

### 关键优化
- Inode 缓存实现快速属性查找
- 目录缓存优化 readdir 操作
- 流式传输处理大文件操作
- 并发批量操作
- 句柄管理实现高效文件访问

## 路线图

### 已完成 ✅
- [x] 核心插件系统（30+ 插件）
- [x] REST API（完整 CRUD 操作）
- [x] CLI 工具（60+ 命令）
- [x] FUSE 集成（Linux/macOS）
- [x] WebSocket 支持
- [x] 批量操作（复制、删除）
- [x] 文件监控
- [x] ACL 支持
- [x] 动态插件加载
- [x] 指标收集（Prometheus）
- [x] WASM 插件支持

### 进行中 🚧
- [ ] 双层缓存系统
- [ ] 可配置挂载系统
- [ ] 向量检索优化
- [ ] 增强 MCP 集成

## 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 测试
cargo test -p evif-plugins core_supported_plugins
cargo test -p evif-rest

# 启用所有特性运行测试
cargo test --workspace --all-features
```

## 贡献

欢迎贡献！请：

1. Fork 本仓库
2. 创建特性分支（`git checkout -b feature/amazing-feature`）
3. 提交更改（`git commit -m 'Add amazing feature'`）
4. 推送到分支（`git push origin feature/amazing-feature`）
5. 提交 Pull Request

## 许可证

可选择以下任一许可证：
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

## 致谢

- 灵感来自 [贝尔实验室 Plan 9](https://9p.io/) 及其"万物皆文件"哲学
- 灵感来自 [AGFS](https://github.com/c4pt0r/agfs) - 黄东旭（PingCAP 联合创始人）创建的 Agent 文件系统
- 使用 [Rust](https://www.rust-lang.org/) 构建
- 使用 [OpenDAL](https://github.com/apache/opendal) 实现统一存储访问

---

**文档**: [docs/](docs/) | **API 参考**: [docs/API.md](docs/API.md) | **入门指南**: [docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)
