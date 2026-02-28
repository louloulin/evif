---
name: evif
description: "EVIF - Everything is a File System. 智能文件系统,支持多种存储后端、向量搜索、GPT集成、消息队列等功能"
version: "1.8.0"
author: "EVIF Team"
tags: ["filesystem", "storage", "vector-search", "ai", "cloud"]
trigger_keywords: ["evif", "文件系统", "存储", "挂载", "文件操作"]
---

# EVIF - Everything is a File System

EVIF 是一个强大的文件系统抽象层,将一切服务通过统一的文件系统接口暴露。基于 Rust 实现,提供高性能、内存安全的文件系统操作。

## 核心能力

### 1. 基础文件操作
- 列出文件: `evif ls <path>`
- 读取文件: `evif cat <path>`
- 写入文件: `evif write <path> <content>`
- 删除文件: `evif rm <path>`
- 创建目录: `evif mkdir <path>`
- 移动文件: `evif mv <old> <new>`
- 文件信息: `evif stat <path>`

### 2. 插件系统

EVIF 支持多种存储后端插件:

**存储插件:**
- **localfs**: 本地文件系统
- **s3fs**: AWS S3 兼容存储
- **memfs**: 内存文件系统
- **sqlfs**: SQL 数据库文件系统

**智能插件:**
- **vectorfs**: 向量搜索文件系统 (支持语义搜索)
- **gptfs**: GPT/AI 集成 (文本处理、摘要)
- **queuefs**: 消息队列 (支持内存/SQLite/TiDB后端)

**流式插件:**
- **streamfs**: 流式数据支持
- **streamrotatefs**: 轮转流文件 (日志场景)

**工具插件:**
- **kvfs**: 键值存储
- **httpfs**: HTTP 文件访问
- **proxyfs**: 远程代理
- **devfs**: 设备文件
- **heartbeatfs**: 心跳监控
- **serverinfofs**: 服务器信息

### 3. REST API

EVIF 提供完整的 REST API (默认端口 8080):

**文件操作:**
- `GET /api/v1/files?path=<path>` - 读取文件
- `PUT /api/v1/files?path=<path>` - 写入文件
- `DELETE /api/v1/files?path=<path>` - 删除文件

**目录操作:**
- `GET /api/v1/directories?path=<path>` - 列出目录
- `POST /api/v1/directories?path=<path>` - 创建目录

**元数据:**
- `GET /api/v1/stat?path=<path>` - 获取文件信息
- `POST /api/v1/rename` - 重命名文件
- `POST /api/v1/chmod` - 修改权限

**插件管理:**
- `POST /api/v1/mount` - 挂载插件
- `DELETE /api/v1/mount` - 卸载插件
- `GET /api/v1/mounts` - 列出挂载点

**句柄操作:**
- `POST /api/v1/handles` - 打开文件句柄
- `POST /api/v1/handles/:id/read` - 从句柄读取
- `POST /api/v1/handles/:id/write` - 向句柄写入
- `DELETE /api/v1/handles/:id` - 关闭句柄

## 使用指南

### 场景1: 读取本地文件

**用户请求**: "查看 /home/user/project/README.md"
**EVIF 命令**:
```bash
evif cat /local/home/user/project/README.md
```

**使用 REST API**:
```bash
curl "http://localhost:8080/api/v1/files?path=/local/home/user/project/README.md"
```

### 场景2: S3 文件操作

**列出 S3 bucket:**
```bash
evif ls /s3/my-bucket/
```

**上传文件到 S3:**
```bash
evif write /s3/my-bucket/data.json "$(cat local-data.json)"
```

**下载 S3 文件:**
```bash
evif cat /s3/my-bucket/backup.zip > backup.zip
```

### 场景3: 向量搜索

**添加文档到向量库:**
```bash
evif vector add /vector/docs /path/to/document.pdf
```

**语义搜索:**
```bash
evif vector search /vector/docs "机器学习算法原理" --top-k=5
```

### 场景4: 消息队列

**入队:**
```bash
echo "task data" | evif write /queue/tasks/enqueue
```

**出队:**
```bash
evif cat /queue/tasks/dequeue
```

**查看队列状态:**
```bash
evif cat /queue/tasks/size
evif cat /queue/tasks/peek
```

### 场景5: GPT 文本处理

**文本摘要:**
```bash
evif gpt process /local/article.txt --action=summary
```

**翻译:**
```bash
evif gpt process /local/text.txt --action=translate --target-lang=en
```

**查看任务状态:**
```bash
evif gpt status <job-id>
```

### 场景6: 大文件处理 (使用HandleFS)

**打开文件句柄:**
```bash
HANDLE_ID=$(evif handle open /local/large-file.bin)
```

**分块读取:**
```bash
evif handle read $HANDLE_ID --offset=0 --size=4096
evif handle read $HANDLE_ID --offset=4096 --size=4096
```

**关闭句柄:**
```bash
evif handle close $HANDLE_ID
```

## 最佳实践

### 1. 路径规范

不同插件使用不同的路径前缀:

- **本地文件**: `/local/...`
- **S3对象**: `/s3/<bucket>/...`
- **内存**: `/mem/...`
- **向量搜索**: `/vector/<namespace>/...`
- **消息队列**: `/queue/<name>/...`
- **键值存储**: `/kv/...`
- **HTTP文件**: `/http/<url-path>`

### 2. 大文件处理

对于大文件操作,建议使用 HandleFS 进行分块读写:

1. 打开文件句柄 (`evif handle open`)
2. 分块读取/写入 (`evif handle read/write`)
3. 关闭句柄 (`evif handle close`)

这样可以:
- 避免一次性加载整个文件到内存
- 支持断点续传
- 实现流式处理

### 3. 批量操作

使用 shell 管道进行批量操作:

```bash
# 查找并列出所有 .log 文件
evif ls /local/data/ | grep ".log" | while read f; do
    echo "Processing: $f"
    evif cat "/local/data/$f"
done
```

### 4. 错误处理

常见错误及解决方法:

**`EvifError::NotFound`**: 文件或路径不存在
- 检查路径是否正确
- 使用 `evif ls` 查看目录内容

**`EvifError::AlreadyMounted`**: 挂载点已被占用
- 使用 `evif mounts` 查看已挂载的插件
- 先卸载旧挂载点: `evif unmount <path>`

**`EvifError::PermissionDenied`**: 权限不足
- 检查文件/目录权限
- 确认运行用户有访问权限

**`EvifError::NotSupported`**: 操作不被该插件支持
- 检查插件是否支持该操作
- 查看插件文档: `evif mount --help`

## 配置

EVIF 配置文件: `~/.evif/config.toml`

### 服务器配置

```toml
[server]
bind_addr = "0.0.0.0"
port = 8080
max_connections = 1000
request_timeout = 30  # seconds
```

### 插件配置

```toml
[plugins.localfs]
storage_path = "/tmp/evif/local"

[plugins.s3fs]
region = "us-east-1"
bucket = "my-bucket"
access_key = "YOUR_ACCESS_KEY"
secret_key = "YOUR_SECRET_KEY"
endpoint = "https://s3.amazonaws.com"  # 可选,用于兼容S3服务

[plugins.vectorfs]
s3_bucket = "vector-docs"
tidb_host = "localhost:4000"
tidb_user = "root"
tidb_password = ""
openai_api_key = "sk-..."
embedding_model = "text-embedding-3-small"
chunk_size = 1000
chunk_overlap = 200

[plugins.queuefs]
backend = "sqlite"  # "memory", "sqlite", or "tidb"
db_path = "/tmp/evif/queue.db"

[plugins.gptfs]
api_key = "sk-..."
model = "gpt-4"
max_concurrent_jobs = 5
```

### 日志配置

```toml
[logging]
level = "info"  # "trace", "debug", "info", "warn", "error"
format = "json"  # "json" or "pretty"
output = "/var/log/evif/evif.log"
```

## 安装

### 方法1: 从源码编译

```bash
# 克隆仓库
git clone https://github.com/evif/evif.git
cd evif

# 编译
cargo build --release

# 安装到系统路径
sudo cp target/release/evif /usr/local/bin/
sudo cp target/release/evif-server /usr/local/bin/
```

### 方法2: 使用预编译二进制

```bash
# 下载最新版本
curl -L https://github.com/evif/evif/releases/latest/download/evif-linux-amd64.tar.gz -o evif.tar.gz
tar -xzf evif.tar.gz
sudo cp evif /usr/local/bin/
```

### Agent Skills 安装

将 EVIF Agent Skills 安装到 Claude Code:

```bash
# 复制到用户技能目录
cp -r skills/* ~/.claude/skills/

# 或使用符号链接 (开发模式)
ln -s $(pwd)/skills/evif ~/.claude/skills/evif
```

### 验证安装

```bash
# 检查版本
evif --version

# 启动服务器
evif-server --config ~/.evif/config.toml

# 在另一个终端测试
evif ls /local/
```

在 Claude Code 中验证 Agent Skills:

```
用户: "使用 evif 列出当前目录"
```

如果 Claude 自动识别并调用 evif 命令,说明安装成功。

## 开发模式

如果正在开发 EVIF 本身,建议使用符号链接方式:

```bash
# Agent Skills
ln -s /path/to/evif/skills/evif ~/.claude/skills/evif

# 二进制
cargo build
ln -s /path/to/evif/target/debug/evif ~/.local/bin/evif
```

这样修改后会立即生效,无需重新安装。

## 高级功能

详见专项技能:
- `evif-manage.md` - 插件管理详解
- `evif-vector.md` - 向量搜索高级用法
- `evif-gpt.md` - GPT 集成完整指南
- `evif-queue.md` - 消息队列生产模式
- `evif-s3.md` - S3 最佳实践

## 性能优化

### 1. 连接池
EVIF 自动管理 HTTP 连接池,无需手动配置。

### 2. 缓存
- S3FS: 自动缓存目录列表 (30s TTL)
- 元数据缓存: 减少重复 stat 调用

### 3. 并发
所有插件都是异步的,支持高并发操作。

## 故障排查

### 启用调试日志

```bash
RUST_LOG=debug evif-server --config ~/.evif/config.toml
```

### 检查插件状态

```bash
evif mounts
evif health
```

### 查看服务器信息

```bash
evif server-info
```

## 参考资料

- **官方文档**: https://evif.io/docs
- **GitHub**: https://github.com/evif/evif
- **AGFS 参考**: https://github.com/c4pt0r/agfs
- **Rust 文档**: https://doc.rust-lang.org/

## 贡献

欢迎贡献!请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)

## 许可证

MIT OR Apache-2.0

---

**版本**: 1.8.0
**最后更新**: 2025-01-25
