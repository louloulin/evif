# EVIF 功能验证测试计划
真实基于命令测试，而不是单元测试
## 项目概述

EVIF (Everything Is a Virtual filesystem) 是一个基于 Rust 实现的虚拟文件系统，遵循 Plan 9 "万物皆文件" 哲学。项目包含 CLI 工具、REST API、FUSE 挂载、多种存储后端和丰富的插件系统。

### 规模概览

| 类别 | 数量 | 说明 |
|------|------|------|
| CLI 命令 | 68+ | 命令行接口功能 |
| REST API | 66+ | HTTP API 端点 |
| 插件系统 | 30+ | 文件系统插件 |
| 存储后端 | 4+ | 内存、S3、RocksDB、Sled |
| 核心模块 | 16+ | evif-core、evif-vfs、evif-auth 等 |

---

## 1. CLI 命令测试 (68+ 命令)

### 1.1 文件操作命令 (P0)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `ls [path]` | 列出目录内容 | 基本列表 | 正确显示文件和子目录 |
| `ls -l [path]` | 长格式列表 | 详细格式 | 显示权限、大小、日期 |
| `ls -r [path]` | 递归列表 | 递归显示 | 显示所有子目录内容 |
| `cat <path>` | 显示文件内容 | 文本文件 | 正确输出文件内容 |
| `write <path> -c <content>` | 写入文件 | 新建文件 | 文件创建成功，内容正确 |
| `write <path> -c <content> -a` | 追加内容 | 追加模式 | 内容追加到文件末尾 |
| `mkdir <path>` | 创建目录 | 基本创建 | 目录创建成功 |
| `mkdir <path> -p` | 递归创建 | 父目录不存在 | 自动创建父目录 |
| `rm <path>` | 删除文件 | 基本删除 | 文件删除成功 |
| `rm <path> -r` | 递归删除 | 删除目录 | 目录及其内容删除成功 |
| `mv <src> <dst>` | 移动/重命名 | 文件移动 | 文件移动到目标位置 |
| `cp <src> <dst>` | 复制文件 | 基本复制 | 文件复制成功，内容一致 |
| `stat <path>` | 文件状态 | 显示信息 | 返回类型、大小、时间、权限 |
| `touch <path>` | 创建空文件 | 新建文件 | 文件创建成功 |
| `head <path> [-n lines]` | 文件头部 | 默认10行 | 显示前N行 |
| `tail <path> [-n lines]` | 文件尾部 | 默认10行 | 显示最后N行 |
| `tree <path> [-d depth]` | 目录树 | 指定深度 | 按层级显示结构 |

### 1.2 插件管理命令 (P0)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `mount <plugin> <path>` | 挂载插件 | 挂载memfs | 插件挂载成功 |
| `mount <plugin> <path> -c <config>` | 带配置挂载 | 挂载s3fs | 使用指定配置挂载 |
| `unmount <path>` | 卸载插件 | 卸载插件 | 插件卸载成功 |
| `mounts` | 列出挂载点 | 查看挂载 | 显示所有挂载点及插件 |

### 1.3 系统命令 (P0)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `health` | 健康检查 | 服务状态 | 返回状态、版本、运行时间 |
| `stats` | 统计信息 | 服务器统计 | 返回连接状态等信息 |
| `repl` | REPL 交互模式 | 进入交互 | 进入交互式命令行 |

### 1.4 批量操作命令 (P1)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `batch-copy` | 批量复制 | 多文件并发 | 并发复制多个文件 |
| `batch-delete` | 批量删除 | 递归删除 | 批量删除文件/目录 |
| `batch-list` | 列出批量操作 | 查看活动 | 列出所有活动操作 |
| `batch-progress <id>` | 操作进度 | 实时进度 | 显示操作进度百分比 |
| `batch-cancel <id>` | 取消操作 | 取消进行中 | 操作被取消 |

### 1.5 搜索与分析命令 (P1)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `grep <pattern> <path>` | 正则搜索 | 基本搜索 | 返回匹配的行及行号 |
| `grep -r <pattern> <path>` | 递归搜索 | 目录搜索 | 搜索目录下所有文件 |
| `checksum <path> -a <algo>` | 文件校验 | MD5/SHA256 | 返回文件哈希值 |
| `diff <path1> <path2>` | 文件对比 | 比较差异 | 显示两文件差异 |
| `du <path> [-r]` | 磁盘使用 | 目录大小 | 返回文件数、总大小 |
| `find <path> [-n <name>] [-t <type>]` | 查找文件 | 名称/类型 | 返回匹配的文件路径 |
| `locate <pattern>` | 定位文件 | 快速搜索 | 返回匹配文件路径 |
| `watch <path> [-i <sec>]` | 监控变化 | 文件变化 | 实时显示文件增删 |
| `file <path>` | 文件类型 | 类型检测 | 显示文件类型信息 |

### 1.6 文本处理命令 (P2)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `sort [file] [-r] [-n] [-u]` | 排序 | 多种排序 | -r倒序/-n数字/-u去重 |
| `uniq [file] [-c]` | 去重 | 相邻去重 | -c显示计数 |
| `wc [file] [-l] [-w] [-c]` | 统计 | 行词字节 | -l行/-w词/-c字节 |
| `cut [file] -b|-c|-f <ranges> -d <delim>` | 提取字段 | 字节/字符/字段 | 提取指定部分 |
| `tr [file] <from> <to> [-d]` | 字符转换 | 替换/删除 | -d删除字符 |
| `base64 [file] [-d]` | Base64编解码 | 编码/解码 | 正确转换 |
| `rev [file]` | 反转内容 | 字节反转 | 每行内容反转 |
| `tac [file]` | 反转文件 | 行反转 | 从尾到头输出 |
| `split [file] -l <lines>` | 分割文件 | 按行分割 | 分割为多个部分 |

### 1.7 Shell 工具命令 (P2)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `echo <text>` | 输出文本 | 文本输出 | 输出文本内容 |
| `cd <path>` | 切换目录 | 路径切换 | 工作目录变更 |
| `pwd` | 当前目录 | 显示路径 | 返回当前工作目录 |
| `env` | 环境变量 | 列出变量 | 显示所有环境变量 |
| `export <NAME=value>` | 导出变量 | 设置变量 | 环境变量生效 |
| `unset <NAME>` | 删除变量 | 移除变量 | 变量被移除 |
| `date [-f <format>]` | 日期时间 | 格式化输出 | 支持自定义格式 |
| `sleep <seconds>` | 延迟 | 等待 | 指定秒数等待 |
| `true` | 返回成功 | 成功 | 退出码0 |
| `false` | 返回失败 | 失败 | 退出码1 |

### 1.8 路径工具命令 (P2)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `basename <path>` | 文件名 | 提取文件名 | 返回文件名部分 |
| `dirname <path>` | 目录名 | 提取目录 | 返回目录路径 |
| `realpath <path>` | 真实路径 | 路径解析 | 返回绝对路径 |
| `readlink <path>` | 读取链接 | 链接目标 | 返回目标路径(需要后端支持) |
| `ln <target> <link> [-s]` | 创建链接 | 符号/硬链接 | 创建链接(需要后端支持) |
| `which <command>` | 命令路径 | 查找命令 | 返回命令路径 |
| `type <command>` | 命令类型 | 类型显示 | 显示命令类型 |

### 1.9 传输命令 (P1)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `upload <local> <remote>` | 上传文件 | 本地到EVIF | 文件上传成功 |
| `download <remote> <local>` | 下载文件 | EVIF到本地 | 文件下载成功 |

### 1.10 其他工具命令 (P2)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `truncate <path> -s <size>` | 截断文件 | 调整大小 | 文件调整到指定大小 |
| `file_type <path>` | 文件类型 | 类型检测 | 显示文件类型 |

### 1.11 交互式命令 (P2)

| 命令 | 功能 | 测试用例 | 预期结果 |
|------|------|----------|----------|
| `query <query>` | 图查询 | 图查询(未实现) | 返回错误提示 |
| `get <id>` | 获取节点 | 节点查询(未实现) | 返回错误提示 |
| `create -t <type> -n <name>` | 创建节点 | 节点创建(未实现) | 返回错误提示 |
| `delete <id>` | 删除节点 | 节点删除(未实现) | 返回错误提示 |

---

## 2. REST API 测试 (66+ 端点)

### 2.1 健康检查 (P0)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/health` | GET | 健康检查 | 返回 { status: "ok" } |
| `/api/v1/health` | GET | V1健康检查 | 返回status、version、uptime |

### 2.2 文件操作 (P0)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/files` | GET | 读取文件 | query参数指定路径 |
| `/api/v1/files` | PUT | 写入文件 | 覆盖文件内容 |
| `/api/v1/files` | POST | 创建文件 | 创建新文件 |
| `/api/v1/files` | DELETE | 删除文件 | 删除指定文件 |

### 2.3 目录操作 (P0)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/directories` | GET | 列出目录 | query参数指定路径 |
| `/api/v1/directories` | POST | 创建目录 | 创建新目录 |
| `/api/v1/directories` | DELETE | 删除目录 | 删除指定目录 |

### 2.4 元数据操作 (P1)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/stat` | GET | 文件状态 | 返回文件元数据 |
| `/api/v1/touch` | POST | 更新时间戳 | 更新文件mtime |
| `/api/v1/digest` | POST | 计算哈希 | 返回文件校验和 |
| `/api/v1/rename` | POST | 重命名/移动 | 重命名文件或移动 |

### 2.5 高级操作 (P1)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/grep` | POST | 正则搜索 | 搜索文件内容 |

### 2.6 挂载管理 (P0)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/mounts` | GET | 列出挂载点 | 返回所有挂载点 |
| `/api/v1/mount` | POST | 挂载插件 | 挂载指定插件 |
| `/api/v1/unmount` | POST | 卸载插件 | 卸载指定路径 |

### 2.7 插件管理 (P1)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/plugins` | GET | 列出插件 | 返回插件列表 |
| `/api/v1/plugins/:name/readme` | GET | 插件说明 | 返回README内容 |
| `/api/v1/plugins/:name/config` | GET | 插件配置 | 返回配置参数 |
| `/api/v1/plugins/load` | POST | 加载插件 | 加载外部插件 |
| `/api/v1/plugins/unload` | POST | 卸载插件 | 卸载指定插件 |
| `/api/v1/plugins/list` | GET | 插件详情 | 返回详细信息 |
| `/api/v1/plugins/wasm/load` | POST | 加载WASM | 加载WASM插件 |

### 2.8 句柄管理 (P1 - 核心功能)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/handles/open` | POST | 打开句柄 | 返回句柄ID |
| `/api/v1/handles/:id` | GET | 获取句柄 | 返回句柄详情 |
| `/api/v1/handles/:id/read` | POST | 读取句柄 | 返回读取数据 |
| `/api/v1/handles/:id/write` | POST | 写入句柄 | 写入数据成功 |
| `/api/v1/handles/:id/seek` | POST | 定位句柄 | 位置跳转 |
| `/api/v1/handles/:id/sync` | POST | 同步句柄 | 同步到存储 |
| `/api/v1/handles/:id/close` | POST | 关闭句柄 | 句柄关闭 |
| `/api/v1/handles/:id/renew` | POST | 续期句柄 | TTL续期 |
| `/api/v1/handles` | GET | 列出句柄 | 返回所有句柄 |
| `/api/v1/handles/stats` | GET | 句柄统计 | 返回统计信息 |

### 2.9 批量操作 (P1)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/batch/copy` | POST | 批量复制 | 复制多个文件 |
| `/api/v1/batch/delete` | POST | 批量删除 | 删除多个文件 |
| `/api/v1/batch/progress/:id` | GET | 操作进度 | 查询进度 |
| `/api/v1/batch/operations` | GET | 列出操作 | 所有活动操作 |
| `/api/v1/batch/operation/:id` | DELETE | 取消操作 | 取消指定操作 |

### 2.10 指标监控 (P2)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/metrics/traffic` | GET | 流量统计 | 返回流量数据 |
| `/api/v1/metrics/operations` | GET | 操作统计 | 返回操作计数 |
| `/api/v1/metrics/status` | GET | 系统状态 | 返回系统状态 |
| `/api/v1/metrics/reset` | POST | 重置指标 | 重置所有指标 |

### 2.11 协作功能 (P2)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/share/create` | POST | 创建分享 | 分享创建 |
| `/api/v1/share/list` | GET | 列出分享 | 所有分享 |
| `/api/v1/share/revoke` | POST | 撤销分享 | 撤销分享 |
| `/api/v1/permissions/set` | POST | 设置权限 | 权限设置 |
| `/api/v1/permissions/get` | GET | 获取权限 | 权限信息 |
| `/api/v1/comments` | GET/POST | 评论操作 | 评论列表/添加 |
| `/api/v1/comments/:id` | PUT/DELETE | 评论更新/删除 | 更新或删除 |
| `/api/v1/comments/:id/resolve` | PUT | 标记解决 | 评论解决 |
| `/api/v1/activities` | GET | 活动记录 | 活动列表 |
| `/api/v1/users` | GET | 用户列表 | 所有用户 |

### 2.12 兼容层API (P1)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/api/v1/fs/list` | GET | 兼容列出 | AGFS兼容 |
| `/api/v1/fs/read` | GET | 兼容读取 | AGFS兼容 |
| `/api/v1/fs/write` | POST | 兼容写入 | AGFS兼容 |
| `/api/v1/fs/create` | POST | 兼容创建 | AGFS兼容 |
| `/api/v1/fs/delete` | DELETE | 兼容删除 | AGFS兼容 |

### 2.13 图查询 (P3 - 未完全实现)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/nodes/:id` | GET | 获取节点 | 返回节点数据 |
| `/nodes/:id` | DELETE | 删除节点 | 节点删除 |
| `/nodes/create/:node_type` | POST | 创建节点 | 节点创建 |
| `/query` | POST | 图查询 | 查询执行 |
| `/nodes/:id/children` | GET | 子节点 | 子节点列表 |
| `/stats` | GET | 图统计 | 图统计信息 |

### 2.14 WebSocket (P2)

| 端点 | 方法 | 功能 | 测试用例 |
|------|------|------|----------|
| `/ws` | GET | WebSocket连接 | 实时通信 |

---

## 3. 插件系统测试 (30+ 插件)

### 3.1 基础存储插件 (P0)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `memfs` | 内存文件系统 | 读写性能测试 |
| `localfs` | 本地文件系统 | 路径解析测试 |
| `hellofs` | 示例插件 | 基础功能验证 |

### 3.2 云存储插件 (P1)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `s3fs` | AWS S3 | 桶操作、对象读写 |
| `s3fs_opendal` | S3 (OpenDAL) | OpenDAL S3操作 |
| `azureblobfs` | Azure Blob | Blob存储操作 |
| `gcsfs` | Google Cloud Storage | GCS对象操作 |
| `aliyunossfs` | 阿里云OSS | OSS对象操作 |
| `tencentcosfs` | 腾讯云COS | COS对象操作 |
| `huaweiobsfs` | 华为云OBS | OBS对象操作 |
| `miniofs` | MinIO | S3兼容操作 |

### 3.3 数据库插件 (P1)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `sqlfs` | SQL文件系统 | 数据库映射测试 |
| `sqlfs2` | SQL文件系统V2 | 高级查询测试 |
| `kvfs` | 键值存储 | CRUD操作测试 |

### 3.4 网络协议插件 (P1)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `httpfs` | HTTP文件系统 | HTTP GET/PUT |
| `httpsfs` | HTTPS文件系统 | SSL/TLS支持 |
| `ftpfs` | FTP文件系统 | FTP协议 |
| `sftpfs` | SFTP文件系统 | SSH文件传输 |
| `webdavfs` | WebDAV文件系统 | WebDAV协议 |

### 3.5 高级功能插件 (P2)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `proxyfs` | 代理文件系统 | 请求转发 |
| `streamfs` | 流式存储 | 流读写 |
| `streamrotatefs` | 日志轮转 | 自动轮转 |
| `queuefs` | 队列文件系统 | 队列操作 |
| `vectorfs` | 向量存储 | 向量检索 |
| `gptfs` | GPT集成 | AI问答 |
| `devfs` | 设备文件系统 | 设备模拟 |
| `heartbeatfs` | 心跳服务 | 健康检测 |
| `serverinfofs` | 服务器信息 | 信息查询 |

### 3.6 增强插件 (P2)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `tieredfs` | 分层存储 | 冷热数据迁移 |
| `encryptedfs` | 加密文件系统 | 数据加密 |
| `handlefs` | 句柄文件系统 | 句柄管理 |
| `opendal` | OpenDAL抽象 | 多后端支持 |

### 3.7 WASM插件 (P2)

| 插件 | 描述 | 测试用例 |
|------|------|----------|
| `wasm-plugin` | WASM模块 | 动态加载测试 |

---

## 4. 核心模块测试

### 4.1 evif-core (核心)

| 模块 | 测试重点 |
|------|----------|
| mount_table | 挂载点管理 |
| handle_manager | 句柄管理(TTL) |
| server | 服务器核心 |
| config | 配置加载 |
| cache | 缓存机制 |
| batch_operations | 批量操作 |
| file_monitor | 文件监控 |
| streaming | 流式传输 |
| extism_plugin | Extism插件支持 |

### 4.2 evif-vfs (虚拟文件系统)

| 模块 | 测试重点 |
|------|----------|
| vfs | VFS接口 |
| file | 文件操作 |
| dir | 目录操作 |
| dentry | 目录项 |
| inode | inode管理 |
| vnode | 虚拟节点 |
| path | 路径解析 |

### 4.3 evif-storage (存储层)

| 后端 | 测试重点 |
|------|----------|
| memory | 内存存储性能 |
| sled | Sled持久化 |
| rocksdb | RocksDB性能 |
| s3 | S3远程存储 |

### 4.4 evif-auth (认证授权)

| 功能 | 测试重点 |
|------|----------|
| capability | 能力系统 |
| auth | 认证流程 |
| audit | 审计日志 |

### 4.5 evif-runtime (运行时)

| 功能 | 测试重点 |
|------|----------|
| config | 配置管理 |
| runtime | 组件初始化 |

### 4.6 其他模块

| 模块 | 测试重点 |
|------|----------|
| evif-protocol | 协议编解码 |
| evif-client | 客户端功能 |
| evif-grpc | gRPC服务 |
| evif-metrics | 指标收集 |
| evif-fuse | FUSE挂载 |
| evif-mcp | MCP服务器 |
| evif-macros | 宏功能 |

---

## 5. FUSE 挂载测试

### 5.1 FUSE 基本操作

| 操作 | 测试用例 | 预期结果 |
|------|----------|----------|
| 挂载 | evif-fuse-mount <mount_point> | 成功挂载 |
| 卸载 | fusermount -u <mount_point> | 成功卸载 |
| 读写 | 在挂载点执行文件操作 | 正常工作 |

---

## 6. 性能测试

| 测试项 | 指标 | 目标值 |
|--------|------|--------|
| API响应时间 | P99延迟 | <100ms |
| 文件操作吞吐量 | ops/s | >1000 |
| 并发连接数 | 最大连接 | >1000 |
| 内存使用 | 峰值内存 | <500MB |
| 批量操作 | 多文件并发 | >500文件/秒 |

---

## 7. 测试优先级

| 优先级 | 类别 | 占比 | 说明 |
|--------|------|------|------|
| P0 - 关键 | CLI基础、API文件操作、句柄管理、基础插件 | 50% | 核心功能必须通过 |
| P1 - 重要 | 批量操作、传输命令、云存储插件、FUSE | 30% | 重要功能 |
| P2 - 增强 | 文本处理、监控指标、协作功能、高级插件 | 15% | 增强功能 |
| P3 - 可选 | 图查询、WASM插件、高级认证 | 5% | 实验性功能 |

---

## 8. 测试环境要求

- Rust 1.70+
- cargo 工具链
- 可选: MinIO (S3兼容存储)
- 可选: SQLite (测试数据库)
- 可选: Playwright (E2E测试)

---

## 9. 测试执行计划

### 第一阶段: 单元测试
```bash
cargo test --workspace
```

### 第二阶段: CLI功能测试
```bash
# 启动服务
cargo run -p evif-rest &

# 执行CLI测试
evif ls /
evif cat /test.txt
evif write /new.txt -c "hello"
```

### 第三阶段: API测试
```bash
# 使用curl测试
curl http://localhost:8080/health
curl http://localhost:8080/api/v1/files?path=/
curl -X POST http://localhost:8080/api/v1/handles/open
```

### 第四阶段: 插件测试
```bash
# 挂载各类型插件
evif mount memfs /memory
evif mount localfs /local -c '{"root":"/tmp"}'
```

### 第五阶段: 压力测试
```bash
# 使用wrk进行压测
wrk -t4 -c100 -d30s http://localhost:8080/api/v1/files
```

---

## 10. 验收标准

- [ ] 所有单元测试通过 (30+ 测试)
- [ ] CLI 68+ 命令可用 (P0级别)
- [ ] REST API 66+ 端点响应正常 (P0级别)
- [ ] 核心插件可正常挂载/卸载 (memfs, localfs)
- [ ] 句柄管理功能完整 (10个端点)
- [ ] 批量操作功能正常 (5个端点)
- [ ] 性能指标达标
- [ ] 无内存泄漏
- [ ] 错误处理正确

---

*测试计划版本: 2.0*
*创建日期: 2026-02-26*
*基于实际代码库分析生成*
