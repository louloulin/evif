# EVIF 开发者指南

## 1. 快速开始

### 1.1 前置条件

- Rust 1.75+ (含 `cargo`)
- OpenSSL (用于 TLS)
- Protocol Buffers (可选，用于 gRPC)

### 1.2 构建

```bash
# 克隆仓库
git clone https://github.com/evif/evif
cd evif

# 构建所有 crate
cargo build --release

# 构建特定 crate
cargo build -p evif-rest

# 构建所有特性
cargo build --release --all-features
```

### 1.3 运行测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定测试
cargo test -p evif-core radix_mount_table

# 带输出运行
cargo test -p evif-rest -- --nocapture

# 运行集成测试 (需要服务器)
cargo test -p evif-e2e -- --test-threads=1
```

### 1.4 运行 Lint

```bash
# 格式化代码
cargo fmt

# Lint
cargo clippy --workspace -- -D warnings

# 检查
cargo check --workspace
```

## 2. 项目结构

```
evif/
├── Cargo.toml              # 工作空间定义
├── crates/
│   ├── evif-core/         # 核心引擎 (~26 模块)
│   │   ├── src/
│   │   │   ├── lib.rs    # 公共导出
│   │   │   ├── plugin.rs # 插件 trait
│   │   │   ├── error.rs  # 错误类型
│   │   │   └── ...       # 其他核心模块
│   │   └── Cargo.toml
│   │
│   ├── evif-plugins/     # 插件实现
│   │   ├── src/
│   │   │   ├── lib.rs    # 插件导出
│   │   │   ├── memfs.rs  # 内存插件
│   │   │   ├── localfs.rs # 本地 fs 插件
│   │   │   └── ...       # 40+ 其他插件
│   │   └── Cargo.toml
│   │
│   ├── evif-rest/        # REST API 服务器
│   ├── evif-cli/         # CLI 工具
│   ├── evif-python/      # Python SDK
│   ├── evif-sdk-ts/      # TypeScript SDK
│   ├── evif-sdk-go/      # Go SDK
│   ├── evif-mcp/         # MCP 服务器
│   ├── evif-auth/        # 认证
│   └── ...
│
├── tests/                 # 集成测试
├── examples/              # 示例代码
├── demos/                 # 演示应用
└── docs/                  # 文档 (含 zh/ 中文)
    └── zh/                # 中文文档
```

## 3. 创建新插件

### 3.1 插件结构

```rust
// crates/evif-plugins/src/myplugin.rs

use async_trait::async_trait;
use bytes::Bytes;
use evif_core::{EvifPlugin, EvifError, EvifResult, FileHandle, CreateOptions};
use std::path::Path;

/// 我的自定义插件
pub struct MyPlugin {
    config: MyConfig,
}

#[derive(Debug, Clone)]
pub struct MyConfig {
    pub option1: String,
    pub option2: bool,
}

impl MyPlugin {
    pub fn new(config: Option<&Value>) -> EvifResult<Self> {
        let config = config
            .and_then(|c| serde_json::from_value(c.clone()).ok())
            .unwrap_or(MyConfig {
                option1: "default".to_string(),
                option2: false,
            });

        Ok(Self { config })
    }
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    async fn create(&self, path: &Path, options: CreateOptions) -> EvifResult<FileHandle> {
        // 创建文件
        Ok(FileHandle::new(path))
    }

    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes> {
        // 读取文件内容
        Ok(Bytes::new())
    }

    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> EvifResult<u64> {
        // 写入文件内容
        Ok(data.len() as u64)
    }

    async fn readdir(&self, path: &Path, offset: u64) -> EvifResult<Vec<DirEntry>> {
        // 列出目录
        Ok(vec![])
    }

    async fn stat(&self, path: &Path) -> EvifResult<Metadata> {
        // 获取文件元数据
        Ok(Metadata::default())
    }

    async fn remove(&self, path: &Path) -> EvifResult<()> {
        // 删除文件
        Ok(())
    }

    async fn rename(&self, old: &Path, new: &Path) -> EvifResult<()> {
        // 重命名文件
        Ok(())
    }

    async fn remove_all(&self, path: &Path) -> EvifResult<u32> {
        // 递归删除目录
        Ok(0)
    }
}

/// 插件工厂函数
pub fn create_my_plugin(config: Option<&Value>) -> EvifResult<Arc<dyn EvifPlugin>> {
    Ok(Arc::new(MyPlugin::new(config)?))
}
```

### 3.2 注册插件

```rust
// crates/evif-plugins/src/lib.rs

pub mod myplugin;

pub use myplugin::create_my_plugin;
```

### 3.3 添加到构建

```toml
# crates/evif-plugins/Cargo.toml

[features]
default = ["memfs", "localfs", "contextfs", ...]
myplugin = []
```

## 4. 添加 REST 端点

### 4.1 创建处理器

```rust
// crates/evif-rest/src/my_handler.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

pub async fn my_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // 处理器逻辑
    (StatusCode::OK, "OK")
}

pub fn routes() -> Router {
    Router::new()
        .route("/my-endpoint", get(my_handler))
        .route("/my-endpoint/:id", post(my_handler))
}
```

### 4.2 注册路由

```rust
// crates/evif-rest/src/routes.rs

use crate::my_handler;

let app = Router::new()
    // ... 已有的路由
    .route("/api/v1/my-endpoint", get(my_handler::my_handler))
    .route("/api/v1/my-endpoint/:id", post(my_handler::my_handler_with_id))
    // ...
```

## 5. Python SDK 开发

### 5.1 项目结构

```
crates/evif-python/
├── evif/
│   ├── __init__.py      # 包导出
│   ├── client.py        # 异步客户端
│   ├── sync.py          # 同步封装
│   ├── models.py        # Pydantic 模型
│   ├── memory.py        # 内存 API mixin
│   ├── queue.py         # 队列 API mixin
│   ├── exceptions.py    # 错误类
│   └── types.py         # 类型定义
├── tests/
│   └── ...
├── pyproject.toml
└── README.md
```

### 5.2 添加新方法

```python
# evif/client.py

async def my_new_method(self, param: str) -> dict:
    """方法描述。

    Args:
        param: 参数描述

    Returns:
        返回值描述
    """
    result = await self._request("POST", "/api/v1/my-endpoint", json={"param": param})
    return result
```

### 5.3 添加测试

```python
# tests/test_my_method.py

import pytest
from evif import EvifClient

@pytest.mark.asyncio
async def test_my_new_method():
    async with EvifClient() as client:
        result = await client.my_new_method("test")
        assert "expected" in result
```

## 6. 测试

### 6.1 单元测试

```rust
// 在源文件中
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radix_lookup() {
        let table = RadixMountTable::new();
        // 测试逻辑
    }
}
```

### 6.2 集成测试

```rust
// tests/api/my_test.rs

use evif_core::EvifServer;
use evif_rest::EvifHandlers;

#[tokio::test]
async fn test_my_endpoint() {
    // 启动测试服务器
    let server = EvifServer::test_new().await;

    // 发起请求
    let response = server.make_request(
        http::Method::GET,
        "/api/v1/my-endpoint",
    ).await;

    assert_eq!(response.status(), StatusCode::OK);
}
```

### 6.3 E2E 测试

```rust
// tests/e2e/test_integration.rs

#[tokio::test]
async fn test_full_workflow() {
    // 启动真实服务器
    let port = find_available_port();
    let server = spawn_server(port).await;

    // 等待健康
    wait_for_health(port).await;

    // 测试工作流
    let client = EvifClient::new(port);
    client.mkdir("/mem/test").await;
    client.write("/mem/test/file.txt", "content").await;

    // 清理
    server.stop().await;
}
```

## 7. 文档

### 7.1 Rust 文档

```rust
//! 我的模块提供 X, Y, Z。
//!
//! # 示例
//!
//! ```
//! use evif_core::MyStruct;
//! let instance = MyStruct::new();
//! ```

/// 结构体的简要描述。
///
/// # 字段
///
/// * `field1` - field1 的描述
/// * `field2` - field2 的描述
///
/// # 示例
///
/// ```
/// let s = MyStruct {
///     field1: "value".to_string(),
///     field2: 42,
/// };
/// ```
pub struct MyStruct {
    field1: String,
    field2: u32,
}
```

### 7.2 Markdown 文档

```markdown
# 我的功能

## 概览
简要描述。

## 使用
```bash
evif my-command --flag value
```

## API 参考
描述 API 端点。

## 配置
| 选项 | 默认值 | 描述 |
|--------|---------|----------|
| flag | false | 启用功能 |
```

## 8. 代码风格

### 8.1 格式化

```bash
# 自动格式化
cargo fmt

# 检查格式化
cargo fmt -- --check
```

### 8.2 Lint

```bash
# 运行 clippy
cargo clippy --workspace -- -D warnings

# 修复自动修复问题
cargo clippy --workspace --fix --allow-dirty
```

### 8.3 命名约定

| 项目 | 约定 | 示例 |
|------|------|------|
| Struct | PascalCase | `MyStruct` |
| Enum | PascalCase | `MyEnum` |
| Function | snake_case | `my_function` |
| Module | snake_case | `my_module` |
| Constant | SCREAMING_SNAKE | `MY_CONSTANT` |
| Private field | snake_case | `my_field` |

### 8.4 错误处理

```rust
// 对 EVIF 特定错误使用 EvifError
pub type EvifResult<T> = Result<T, EvifError>;

// 应用程序错误优先使用 anyhow
pub type AppResult<T> = Result<T, anyhow::Error>;

// 库错误使用 thiserror
#[derive(Debug, Error)]
pub enum MyError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("operation failed")]
    OperationFailed(#[from] std::io::Error),
}
```

## 9. 性能

### 9.1 基准测试

```rust
// benches/my_bench.rs

use criterion::{black_box, criterion_group, Criterion};

fn bench_my_operation(c: &mut Criterion) {
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            let data = black_box(some_data());
            my_operation(data)
        })
    });
}

criterion_group!(benches, bench_my_operation);
```

### 9.2 性能分析

```bash
# CPU 性能分析
cargo flamegraph --bin evif-rest -- -port 8081

# 内存性能分析
cargo heap-profiler

# 异步性能分析
cargo instrument --output=trace.json
```

### 9.3 关键优化

- 使用 `Arc` 而非克隆
- 使用 `Bytes` 实现零拷贝操作
- 使用 `DashMap` 实现并发哈希映射
- 使用 `serde_json` 的 `no_cache` 特性
- 使用 `tracing` 的 `no Subscriber` guard

## 10. 发布流程

### 10.1 版本更新

```bash
# 更新 Cargo.toml 中的版本
cargo release patch  # 0.1.0 -> 0.1.1
cargo release minor # 0.1.0 -> 0.2.0
cargo release major # 0.1.0 -> 1.0.0
```

### 10.2 更新日志

```markdown
# Changelog

## [0.1.1] - 2026-04-27

### 新增
- 新增 `my-endpoint` API
- Python SDK 支持 X

### 修复
- 修复内存插件 bug

### 变更
- 更新依赖 X 到 2.0
```

### 10.3 发布

```bash
# 构建发布版本
cargo build --release

# 运行测试
cargo test --workspace

# 发布 crate
cargo publish -p evif-core
cargo publish -p evif-plugins
cargo publish -p evif-rest
# 等等
```

## 11. 相关文档

- [架构概览](00-overview.md)
- [插件系统](02-plugin-system.md)
- [REST API 参考](03-rest-api.md)
- [SDK 集成](04-sdk-integration.md)