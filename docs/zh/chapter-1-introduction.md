# 第一章 概述

## 什么是 EVIF？

**EVIF** (Extensible Virtual File System) 是一个用 Rust 编写的强大的、可扩展的虚拟文件系统框架。它实现了"万物皆文件"的 Unix 哲学,将各种存储后端、数据源和服务统一为一致的文件系统接口。

### 核心理念

EVIF 的核心设计理念是通过**插件系统**和**统一挂载表**来实现:

- **插件化架构**: 每个文件系统后端都实现为独立的插件
- **路径抽象**: 使用 Radix 树进行高效的最长前缀匹配路由
- **POSIX 兼容**: 提供类似传统文件系统的操作接口(创建、读取、写入、删除等)
- **多协议支持**: 支持 REST API、WebSocket、FUSE、gRPC 等多种访问方式

### 与 AGFS 的关系

EVIF 在设计和功能上与 AGFS (Another Graph File System) 类似,提供:

- **EvifPlugin trait**: 对标 AGFS 的 ServicePlugin/FileSystem
- **RadixMountTable**: 对标 AGFS 的挂载表机制
- **多种后端支持**: 内存、本地文件系统、云存储(S3、Azure、GCS 等)、数据库等

## 关键特性

### 1. 插件系统

EVIF 的核心是强大的插件系统,包含 20+ 内置插件:

**基础插件**:
- `memfs`: 内存文件系统,适合测试和临时数据
- `localfs`: 本地文件系统挂载
- `hellofs`: 示例插件

**云存储插件**:
- `s3fs`: AWS S3 集成
- `azureblobfs`: Azure Blob Storage
- `gcsfs`: Google Cloud Storage
- `aliyunossfs`: 阿里云对象存储
- `miniofs`: MinIO 对象存储

**数据库插件**:
- `sqlfs`: SQL 数据库后端
- `kvfs`: 键值存储后端

**特殊用途插件**:
- `queuefs`: 队列文件系统
- `streamfs`: 流式数据访问
- `httpfs`: HTTP 文件访问
- `proxyfs`: 代理文件系统

每个插件实现统一的 `EvifPlugin` trait,提供一致的文件操作接口。

### 2. 灵活的挂载机制

使用 Radix 树挂载表,支持:

- **路径映射**: 将不同插件挂载到不同路径
- **最长前缀匹配**: 智能路由文件操作到正确的插件
- **动态挂载**: 运行时挂载和卸载插件
- **嵌套挂载**: 支持多层路径嵌套

示例挂载结构:
```
/           → (root)
  /mem      → memfs (内存文件系统)
  /local    → localfs (本地文件系统)
  /s3       → s3fs (AWS S3)
  /local/home/user/docs → localfs (嵌套挂载)
```

### 3. 多种访问接口

EVIF 提供多种访问方式,满足不同场景需求:

**REST API**:
- HTTP/HTTPS 接口
- JSON 请求/响应
- 完整的文件操作支持
- 端口: 默认 8081

**WebSocket**:
- 实时终端接口
- 支持交互式命令
- 适合 Web 应用集成

**FUSE**:
- 将 EVIF 文件系统挂载为本地目录
- 与标准文件系统工具兼容(ls, cp, cat 等)
- 支持缓存和性能优化

**CLI**:
- 命令行工具 `evif`
- 支持 REPL 模式和批处理模式
- 61+ 内置命令

**gRPC**:
- 高性能 RPC 接口(开发中)
- 支持流式传输

### 4. 可扩展架构

**动态插件加载**:
- 支持运行时加载 .so/.dylib/.dll 插件
- 标准 ABI 接口
- 插件信息查询

**认证与安全**:
- 基于 Capability 的访问控制
- 支持多种认证机制
- 审计日志

**监控与指标**:
- Prometheus 指标集成
- 流量监控
- 操作统计
- 性能分析

### 5. 高性能特性

- **异步 I/O**: 基于 Tokio 的异步运行时
- **并发安全**: 使用 Arc 和 Mutex 保证线程安全
- **缓存机制**: 元数据缓存和目录缓存
- **批量操作**: 支持批量文件操作

### 6. 图模型支持(规划中)

EVIF 包含图数据结构和算法层,未来将支持:

- 将文件和目录表示为图节点
- 复杂的图查询和遍历
- 关系分析和图谱构建

## 架构概览

### 核心组件

EVIF 由多个功能模块组成:

```
evif-core          核心抽象和基础设施
  ├── plugin.rs          EvifPlugin trait 定义
  ├── radix_mount_table  Radix 树挂载表
  ├── server.rs          服务器抽象
  └── handle_manager.rs  句柄管理

evif-plugins       插件实现集合
  ├── memfs             内存文件系统
  ├── localfs           本地文件系统
  ├── s3fs              S3 文件系统
  └── ...               其他 20+ 插件

evif-rest          REST API 服务器
  ├── handlers          文件操作处理
  ├── compat_fs         兼容层 API
  └── ws_handlers       WebSocket 处理

evif-fuse          FUSE 集成
evif-client        HTTP 客户端库
evif-cli           命令行工具
evif-auth          认证和授权
evif-storage        存储抽象层
evif-graph          图数据结构
evif-runtime        运行时配置
evif-metrics        指标收集
evif-mcp            MCP 服务器集成
```

### 数据流

```
客户端请求
    ↓
evif-rest (HTTP/WebSocket) / evif-fuse (FUSE) / evif-cli (CLI)
    ↓
RadixMountTable (路径解析)
    ↓
EvifPlugin (插件执行)
    ↓
实际存储 (内存/磁盘/云存储/数据库)
```

### 技术栈

**语言和运行时**:
- Rust 1.70+
- Tokio 异步运行时
- async-trait 异步 trait

**主要依赖**:
- serde: 序列化/反序列化
- tokio: 异步 I/O
- petgraph: 图算法
- dashmap: 并发哈希表
- fuser: FUSE 绑定
- reqwest: HTTP 客户端

## 应用场景

### 1. 云存储统一访问

将多个云存储服务(S3、Azure、GCS)统一为本地文件系统接口:

```bash
# 挂载 S3
mount s3fs /my-bucket --bucket my-bucket --region us-east-1

# 挂载 Azure
mount azureblobfs /azure --account myaccount --container data

# 统一访问
ls /my-bucket/documents
cp /my-bucket/file.txt /azure/backup/
```

### 2. 数据处理管道

使用队列和流插件构建数据处理管道:

```bash
# 创建处理队列
queuefs create /input-queue
queuefs create /output-queue

# 流式处理
streamfs read /input-queue | process | streamfs write /output-queue
```

### 3. 开发和测试

使用内存文件系统进行快速测试:

```bash
# 挂载内存文件系统
mount memfs /test

# 创建测试数据
create /test/config.json '{"key":"value"}'

# 运行测试
run_tests.sh

# 清理
umount /test  # 数据自动清理
```

### 4. 多租户文件系统

为不同用户或团队挂载独立的存储后端:

```bash
# 团队 A 使用 S3
mount s3fs /team-a --bucket team-a-bucket

# 团队 B 使用本地存储
mount localfs /team-b --root /data/team-b

# 团队 C 使用 Azure
mount azureblobfs /team-c --account team-c --container files
```

### 5. 备份和同步

通过统一接口实现跨存储备份和同步:

```bash
# 本地到 S3
copy /local/documents/* /s3/backup/

# S3 到 Azure
copy /s3/data/* /azure/archive/

# 增量同步
sync /source/ /destination/
```

### 6. FUSE 本地挂载

将云存储挂载为本地目录:

```bash
# 启动 FUSE 挂载
evif-fuse-mount /mnt/cloud --plugin s3fs --bucket my-bucket

# 使用标准工具
ls /mnt/cloud
cp local_file.txt /mnt/cloud/
```

## 项目状态

### 当前可用功能

✅ **核心功能**:
- 插件系统完整实现
- Radix 树挂载表
- REST API 服务
- WebSocket 终端
- FUSE 集成
- CLI 工具

✅ **可用插件**:
- memfs, localfs, hellofs
- s3fs, azureblobfs, gcsfs, aliyunossfs, miniofs
- sqlfs, kvfs
- queuefs, streamfs, httpfs, proxyfs
- 以及其他 10+ 特殊用途插件

✅ **监控和指标**:
- Prometheus 指标端点
- 流量监控
- 操作统计

### 开发中功能

⚠️ **图 API**: 图查询功能当前为占位实现

⚠️ **动态挂载**: REST API 的挂载接口返回"暂不支持"

### 未来计划

🔮 **增强功能**:
- 配置文件支持
- 动态插件加载完善
- 图查询功能实现
- gRPC 服务启用
- Web UI 功能扩展

## 为什么选择 EVIF?

### 相比传统文件系统

**优势**:
- 统一接口访问多种存储
- 无需修改应用程序代码
- 灵活的插件扩展机制
- 云原生设计

### 相比其他虚拟文件系统

**独特之处**:
- Rust 实现的内存安全保证
- 原生异步高性能
- 内置多种云存储插件
- 图模型支持(规划中)
- 多协议访问

### 适用场景

EVIF 特别适合:

- **云应用**: 需要统一访问多个云存储
- **微服务**: 需要灵活的存储抽象
- **数据处理**: 需要构建数据管道
- **开发测试**: 需要快速搭建测试环境
- **多租户**: 需要隔离的存储空间

## 社区和资源

### 文档

- 📖 [快速开始](../en/chapter-2-getting-started.md)
- 🏗️ [架构设计](../en/chapter-3-architecture.md)
- 🔌 [插件开发](../en/chapter-5-plugin-development.md)
- 📡 [FUSE 集成](../en/chapter-6-fuse-integration.md)
- 📘 [API 参考](../en/chapter-7-api-reference.md)

### 项目资源

- 📦 代码仓库: https://github.com/evif/evif
- 🐛 问题追踪: https://github.com/evif/evif/issues
- 💬 讨论区: https://github.com/evif/evif/discussions
- 📄 许可证: MIT OR Apache-2.0

### 相关项目

- **AGFS**: Another Graph File System - EVIF 的灵感来源
- **FUSE**: Filesystem in Userspace
- **Tokio**: Rust 异步运行时

## 下一步

准备开始使用 EVIF? 查看 [第二章:快速开始](../en/chapter-2-getting-started.md) 了解安装和基本使用。

对插件开发感兴趣? 跳到 [第五章:插件开发](../en/chapter-5-plugin-development.md) 学习如何创建自定义插件。

想深入了解架构? 阅读 [第三章:架构设计](../en/chapter-3-architecture.md) 了解系统设计和组件交互。
