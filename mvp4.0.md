# EVIF MVP 4.0 路线图

> 创建时间：2026-05-04
> 目标：大规模使用的完整规划、AI Agent 集成、一键安装
> 核心设计：**Skill + MCP 统一架构**，对外命令统一为 `evif`

---

## 一、核心设计理念

### 1.1 Skill + MCP 统一架构

EVIF 的核心创新是**Skill 与 MCP 的深度融合**：

```
┌─────────────────────────────────────────────────────────────────────┐
│                        用户交互层                                   │
│                     (用户只看见 evif 命令)                          │
└─────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────┐
│                     EVIF Command Layer (evif)                        │
│                                                                     │
│   evif skill ls          # 列出技能                                │
│   evif skill run <name>  # 执行技能                                │
│   evif memory search     # 记忆搜索                                │
│   evif context read L0   # 上下文读取                              │
│   evif pipe create       # 管道创建                                │
│   evif mount add         # 挂载管理                                │
└─────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────┐
│                   MCP Protocol Layer (stdio)                        │
│                                                                     │
│   75 Tools + 4 Prompts + 3 Resources → AI Agent 平台              │
│   (Claude Code / Codex / Cursor / Gemini / Copilot)                │
└─────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    VFS Core + Plugin Registry                       │
│                                                                     │
│   SkillFS → /skills/*.md → MCP Prompts                             │
│   MemoryFS → 向量搜索 → MCP Tools                                   │
│   ContextFS → 三层上下文 → AI Agent 记忆                           │
│   PipeFS → 多 Agent 协调 → MCP Sampling                            │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 EVIF 核心定位

**EVIF 是 AI Agent 的虚拟文件系统**，为 AI Agent 提供统一的存储、记忆、上下文和协同接口。

```
EVIF = Everything Is a File
核心价值：为 AI Agent 提供持久化上下文、可复用技能和多智能体协同
```

**三大核心能力**：
1. **持久化上下文** - ContextFS（L0/L1/L2 三层上下文）
2. **可复用技能** - SkillFS（SKILL.md 技能发现与执行）
3. **多 Agent 协同** - PipeFS（轻量级任务协调）

**参考 RTK 的两个方向**（不是做 RTK 的替代品）：

| RTK 方面 | EVIF 借鉴点 |
|----------|------------|
| **多 Agent 集成方式** | RTK 支持 10+ Agent 平台的 Hook/MCP 注册模式，EVIF 参考 `evif connect` 设计 |
| **Token 优化策略** | RTK 的输出过滤 Pipeline（60-90% 节省），EVIF 参考 MCP 工具输出优化 |

---

## 二、EVIF 统一命令架构

### 2.1 命令设计

```bash
# 核心命令
evif                         # 交互式 REPL
evif --help                  # 帮助

# Skill 系统 (SkillFS → MCP Prompts)
evif skill ls                # 列出所有技能
evif skill info <name>       # 查看技能详情
evif skill run <name> [args] # 执行技能
evif skill create <name>     # 创建新技能
evif skill delete <name>     # 删除技能

# Memory 系统 (MemoryFS → MCP Tools)
evif memory memorize <text>   # 记忆存储
evif memory retrieve <query> # 记忆检索
evif memory search <query>   # 语义搜索
evif memory stats            # 记忆统计

# Context 系统 (ContextFS → AI Agent 记忆)
evif context ls              # 列出上下文层
evif context read L0         # 读取 L0 当前任务
evif context read L1          # 读取 L1 决策
evif context read L2          # 读取 L2 项目知识
evif context write L0 <text>  # 写入当前任务
evif context write L1 <text>  # 写入决策

# Pipe 系统 (PipeFS → Multi-Agent)
evif pipe ls                 # 列出管道
evif pipe create <name>      # 创建管道
evif pipe send <name> <msg>  # 发送消息
evif pipe recv <name>        # 接收消息

# File 系统 (VFS)
evif ls <path>               # 列出目录
evif cat <path>              # 读取文件
evif write <path> <content>  # 写入文件
evif mkdir <path>            # 创建目录
evif rm <path>               # 删除文件

# Mount 管理
evif mount ls                # 列出挂载
evif mount add <name> <path> # 添加挂载
evif mount remove <name>     # 移除挂载

# MCP 集成
evif mcp ls                  # 列出 MCP Server
evif mcp add <name> <cmd>    # 添加 MCP Server
evif mcp remove <name>       # 移除 MCP Server

# 配置
evif config get <key>        # 获取配置
evif config set <key> <val>   # 设置配置
evif config export           # 导出配置
evif config import           # 导入配置

# 诊断
evif health                  # 健康检查
evif version                 # 版本信息
evif stats                   # 统计信息
```

### 2.2 Token 优化输出模式

```bash
# EVIF 输出默认针对 LLM 优化（紧凑格式）
evif cat /context/L0/current          # 自动截断大文件
evif ls /mem/                          # 紧凑目录列表
evif memory search "query"            # 默认返回摘要

# 可选完整模式
evif cat --full /path/to/file         # 不截断

# 配置 ~/.evif/hooks
git() {
    evif git "$@"
}

# 当运行 git status 时，自动替换为 evif git status
# 输出被压缩，只返回关键信息给 AI context

# 或者更简单的 alias 模式
alias git=evif git
alias docker=evif docker
alias npm=evif npm
```

### 2.3 MCP 自动发现

```bash
# 自动发现 MCP Servers
evif mcp discover

# 输出示例：
# ✓ GitHub MCP Server (本地)
# ✓ Slack MCP Server (本地)
# ✓ Filesystem MCP Server (本地)
# ✓ EVIF MCP (远程: localhost:8081)
```

---

## 三、AI Agent 集成

### 3.1 统一架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                    AI Agent 平台层 (MCP 客户端)                     │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────┐ │
│  │ Claude   │  │ OpenAI   │  │ Cursor   │  │ Gemini   │  │ GitHub│ │
│  │ Code     │  │ Codex    │  │          │  │ CLI      │  │Copilot│ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └───┬──┘ │
│       └────────────┴────────────┴────────────┴────────────┘       │
│                              │                                     │
│                    MCP Protocol (stdio/SSE)                        │
│                              ↓                                     │
├─────────────────────────────────────────────────────────────────────┤
│                 EVIF MCP Server (evif mcp serve)                   │
│                                                                     │
│   75 Tools:  evif_ls, evif_cat, evif_write, evif_memorize, ...     │
│   4 Prompts: file_explorer, batch_operations, data_analysis, ...   │
│   3 Resources: file:///, /context/L0/current, /context/L1/...     │
│                              ↓                                     │
├─────────────────────────────────────────────────────────────────────┤
│                    Skill + VFS Core                                │
│                                                                     │
│   SkillFS (/skills/*.md) → MCP Prompts 自动发现                    │
│   MemoryFS (向量存储) → evif_memorize/retrieve Tools                │
│   ContextFS (三层上下文) → AI Agent 记忆                           │
│   PipeFS (多 Agent 协调) → sampling/create                          │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 集成架构设计（参考 RTK Hook 系统）

RTK 支持 10+ AI Agent 平台，使用**三种集成模式**：

| 模式 | 适用平台 | 原理 |
|------|----------|------|
| **Hook（JSON 协议）** | Claude Code, Cursor, Gemini, Copilot | 读写 stdin/stdout JSON，重写命令 |
| **MCP Server 注册** | Claude Desktop, Claude Code | 注册为 MCP Server，提供 Tools |
| **Rules/Instructions 文件** | Codex, Windsurf, Cline | 追加指令文件，靠 Agent 遵守 |

EVIF 作为 MCP Server，主要使用**模式 2（MCP 注册）**+ **模式 3（Rules 文件）**。

### 3.3 各平台集成方案

#### Claude Desktop（MCP Server 注册）

```bash
evif connect claude

# 自动执行：
# 1. 备份 ~/.claude/claude_desktop_config.json → .json.bak
# 2. 添加 MCP Server 配置
# 3. 验证 evif mcp serve 可启动
# 4. 重启 Claude Desktop
```

配置文件 `~/.claude/claude_desktop_config.json`：
```json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"],
      "env": {
        "EVIF_REST_URL": "http://localhost:8081",
        "EVIF_SKILLS_PATH": "~/.evif/skills"
      }
    }
  }
}
```

#### Claude Code（MCP Server + Hook）

```bash
evif connect claude-code

# 自动执行：
# 1. 注册 MCP Server 到 ~/.claude/settings.json
# 2. 添加 CLAUDE.md 路由规则
# 3. 添加 PreToolUse hook（可选：token 优化）
```

MCP 注册（`~/.claude/settings.json`）：
```json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"],
      "env": {
        "EVIF_REST_URL": "http://localhost:8081"
      }
    }
  }
}
```

Hook 注册（可选，用于输出压缩）：
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "mcp__evif__evif_cat",
        "hooks": [
          {
            "type": "command",
            "command": "evif hook compress"
          }
        ]
      }
    ]
  }
}
```

#### Cursor（MCP Server 注册）

```bash
evif connect cursor
```

配置文件 `~/.cursor/mcp.json`：
```json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"]
    }
  }
}
```

#### Gemini CLI（MCP Server 注册）

```bash
evif connect gemini
```

配置文件 `~/.gemini/settings.json`：
```json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"]
    }
  }
}
```

追加到 `~/.gemini/GEMINI.md`：
```markdown
## EVIF MCP Server
When you need persistent memory, file operations, or skill execution, use EVIF MCP tools:
- `evif_cat` to read files, `evif_write` to write
- `evif_memorize` to store knowledge, `evif_retrieve` to search
- `evif_skill_run` to execute workflows
```

#### OpenAI Codex（Instructions 文件）

```bash
evif connect codex
```

配置 `~/.codex/AGENTS.md` + `~/.codex/EVIF.md`：
```markdown
@EVIF.md

## EVIF Integration
Use `evif` CLI for file operations and memory:
- `evif cat <path>` - Read files
- `evif write <path> <content>` - Write files
- `evif memory memorize <text>` - Store memories
- `evif memory search <query>` - Search memories
```

#### GitHub Copilot（Hook + Instructions）

```bash
evif connect copilot
```

配置文件 `.github/hooks/evif-mcp.json`：
```json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"]
    }
  }
}
```

追加到 `.github/copilot-instructions.md`：
```markdown
## EVIF MCP Integration
Use EVIF MCP tools for persistent context and memory.
```

#### Windsurf / Cline（Rules 文件）

```bash
evif connect windsurf
evif connect cline
```

追加到 `.windsurfrules` 或 `.clinerules`：
```markdown
## EVIF Context System
When you need persistent memory across sessions, use EVIF CLI commands:
- `evif cat /context/L0/current` - Read current task
- `evif write /context/L0/current "<text>"` - Update task
- `evif memory search "<query>"` - Search memories
```

### 3.4 连接命令设计

```bash
# 查看支持的平台
evif connect --list

# 输出：
# Platform         | Method    | Config Path
# claude           | MCP       | ~/.claude/claude_desktop_config.json
# claude-code      | MCP+Hook  | ~/.claude/settings.json
# cursor           | MCP       | ~/.cursor/mcp.json
# gemini           | MCP       | ~/.gemini/settings.json
# codex            | Rules     | ~/.codex/AGENTS.md
# copilot          | Hook      | .github/hooks/evif-mcp.json
# windsurf         | Rules     | .windsurfrules
# cline            | Rules     | .clinerules

# 连接到平台（交互式选择）
evif connect

# 直接指定平台
evif connect claude

# 断开连接
evif connect claude --disconnect

# 检查连接状态
evif connect --check
```

### 3.5 集成设计原则

| 原则 | EVIF 实现 |
|------|-----------|
| 原子写入：tempfile + rename | 集成配置修改使用相同模式 |
| 幂等性：检查已存在再修改 | 重复执行 `evif connect` 安全 |
| 备份：.json.bak | 修改前自动备份配置文件 |
| 迁移：检测旧版本自动迁移 | 检测旧 evif-mcp 配置自动更新 |
| 环境覆盖：EVIF_CLAUDE_DIR | 自定义平台目录 |
| PatchMode：Ask/Auto/Skip | `--yes` 自动确认，默认交互 |

---

## 四、一键安装系统

### 4.1 核心安装命令

```bash
# 一键安装
curl -fsSL https://evif.dev/install.sh | bash

# 或
brew install evif-io/evif/evif

# 安装后
evif --help
```

### 4.2 安装脚本设计

```bash
#!/bin/bash
# install.sh - 一键安装 EVIF

set -euo pipefail

EVIF_VERSION="${EVIF_VERSION:-latest}"
EVIF_HOME="${EVIF_HOME:-$HOME/.evif}"
REPO="evif-io/evif"

# 检测平台
detect_platform() {
    local os="$(uname -s)"
    local arch="$(uname -m)"
    case "$os" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        *)      echo "不支持: $os"; exit 1 ;;
    esac
    case "$arch" in
        x86_64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)       echo "不支持: $arch"; exit 1 ;;
    esac
    echo "${arch}-${os}"
}

# 下载并安装
install() {
    local platform="$(detect_platform)"
    local version="${EVIF_VERSION}"
    
    if [ "$version" = "latest" ]; then
        version=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"v\?\([^"]*\)".*/\1/')
    fi
    
    local url="https://github.com/${REPO}/releases/download/v${version}/evif-${platform}.tar.gz"
    local install_dir="$EVIF_HOME/bin"
    
    mkdir -p "$install_dir"
    curl -fsSL "$url" | tar xz -C "$install_dir"
    chmod +x "$install_dir/evif"
}

# 初始化配置
init_config() {
    mkdir -p "$EVIF_HOME/skills"
    mkdir -p "$EVIF_HOME/config"
    
    # 默认配置
    cat > "$EVIF_HOME/config/default.toml" << 'EOF'
[evif]
version = "4.0.0"

[mcp]
protocol_version = "2024-11-05"

[skills]
path = "~/.evif/skills"
auto_discover = true

[memory]
provider = "vector"
EOF
}

# 添加到 PATH
setup_path() {
    local shell_config=""
    case "$SHELL" in
        */zsh)  shell_config="$HOME/.zshrc" ;;
        */bash) shell_config="$HOME/.bashrc" ;;
        *)      shell_config="$HOME/.profile" ;;
    esac
    
    if [ -f "$shell_config" ]; then
        if ! grep -q ".evif/bin" "$shell_config"; then
            echo 'export PATH="$HOME/.evif/bin:$PATH"' >> "$shell_config"
        fi
    fi
}

# 主流程
main() {
    echo "🚀 安装 EVIF..."
    install
    init_config
    setup_path
    
    echo ""
    echo "✅ 安装成功!"
    echo ""
    echo "下一步:"
    echo "  evif --help                    # 查看帮助"
    echo "  evif connect claude  # 连接到 Claude Desktop"
    echo "  evif skill ls                  # 列出技能"
    echo "  evif mcp serve                 # 启动 MCP Server"
}

main "$@"
```

### 4.3 卸载脚本

```bash
#!/bin/bash
# uninstall.sh - 卸载 EVIF

read -p "确定要卸载 EVIF 吗? (y/N) " confirm
if [ "$confirm" != "y" ]; then
    exit 0
fi

rm -rf ~/.evif
sed -i '/\.evif\/bin/d' ~/.zshrc ~/.bashrc 2>/dev/null || true

echo "✅ 卸载完成"
```

---

## 4.5 Token 优化策略（参考 RTK 输出过滤模式）

### 问题分析

EVIF 当前**零**响应压缩/过滤/截断。每次工具调用返回原始数据。

| 问题 | 文件 | 影响 |
|------|------|------|
| `evif_cat` 无大小限制 | `lib.rs:2200` | 100KB 文件 = ~25,000 tokens |
| Tool Schema 冗余 | `lib.rs:1551` | ~2,600 tokens/session |
| Memory Search 返回全文 | `lib.rs:2230` | 10 结果 = ~1,500 tokens |
| REST API base64 编码 | `fs_handlers.rs:180` | 33% 体积膨胀 |
| REST 状态响应冗余 | `handlers.rs:200-278` | 30-50% 多余字段 |
| CLI 输出面向人类 | `commands.rs` | 50-65% 可压缩 |

### Token 减少方案

#### Phase 1: evif_cat 内容截断（P0）

```rust
// lib.rs evif_cat 改造
ToolParam { name: "max_lines", type: "number", description: "最大返回行数" },
ToolParam { name: "mode", type: "string", description: "head|tail|snippet|full" },
```

- 默认 `max_lines=100`（~2,500 tokens）
- 超出时返回截断提示：`[截断，共 N 行，用 offset/size 读取更多]`
- **预计节省：60-90% 每次 cat 调用**

#### Phase 2: Tool Schema 精简（P1）

```rust
// 当前：每个属性重复说明适用 action
ToolParam { name: "action", description: "stat (get info) / mv (rename) / cp (copy)" },

// 优化：移除冗余描述
ToolParam { name: "action", description: "stat|mv|cp" },
```

- 移除 `evif_memorize` 的 deprecated `text` 别名
- 短化所有描述字符串
- **预计节省：~1,000 tokens/session**

#### Phase 3: Memory Search 紧凑模式（P1）

```rust
// 新增 compact 参数
ToolParam { name: "compact", type: "boolean", description: "紧凑模式（仅返回 id+score+摘要）" },
ToolParam { name: "max_results", type: "number", description: "最大结果数（默认 3）" },
```

- 紧凑模式：仅返回 `id`, `score`, 前 100 字符
- 默认 `max_results=3`（原来 5）
- **预计节省：40-60% 每次 search 调用**

#### Phase 4: 输出过滤 Pipeline（P2，借鉴 RTK 8 阶段）

```
MCP 工具输出 → strip_ansi → truncate_lines → compact_json → 最终输出
```

参考 RTK 的 `toml_filter.rs`，实现声明式输出过滤：

```toml
# ~/.evif/filters/default.toml
[output]
strip_ansi = true
max_lines = 200
compact_json = true    # 移除 null 字段
max_string_length = 10000  # 单个字符串最大长度
```

#### Phase 5: 分层工具加载（P2）

当前 37 个工具一次性加载到 context。改为按需加载：

```rust
// 核心 8 个工具始终加载
const CORE_TOOLS = ["evif_ls", "evif_cat", "evif_write", "evif_mkdir",
                     "evif_rm", "evif_health", "evif_mount", "evif_stat"];

// 按需工具（通过 evif_tool_enable 启用）
// evif_memorize, evif_retrieve, evif_skill, evif_batch, etc.
```

- 核心 8 工具 ~600 tokens
- 按需加载减少 ~2,000 tokens 初始开销

### Token 减少效果预估

| 场景 | 当前 Tokens | 优化后 | 节省率 |
|------|-------------|--------|--------|
| 初始化（tools/list） | ~2,600 | ~600 | **77%** |
| cat 100KB 文件 | ~25,000 | ~2,500 | **90%** |
| memory search 10 结果 | ~1,500 | ~600 | **60%** |
| ls 大目录 (200 文件) | ~3,000 | ~1,200 | **60%** |
| 典型会话 (10 次调用) | ~50,000 | ~15,000 | **70%** |

**综合预估：实现 Phase 1-3 后，典型 MCP 会话 token 消耗降低 40-60%**

### 5.1 插件架构

```bash
# 内置插件 (编译时决定)
evif plugin ls                   # 列出已安装插件

# WASM 动态插件
evif plugin install <path>       # 安装 WASM 插件
evif plugin uninstall <name>     # 卸载插件
evif plugin update <name>        # 更新插件

# 远程插件
evif plugin add <name> --from registry.evif.dev/<name>
```

### 5.2 插件分类

| 分类 | 插件数 | 说明 |
|------|--------|------|
| **存储** | 5 | localfs, memfs, encryptedfs, tieredfs, streamrotatefs |
| **数据库** | 4 | sqlfs, sqlfs2, kvfs, queuefs |
| **云存储** | 8 | s3fs, azureblobfs, gcsfs, aliyunossfs, tencentcosfs, huaweiobsfs, miniofs |
| **SaaS** | 11 | githubfs, gmailfs, postgresfs, teamsfs, telegramfs, shopifyfs, notionfs, discordfs, slackfs |
| **网络** | 5 | httpfs, proxyfs, webdavfs, ftpfs, sftpfs |
| **AI** | 4 | gptfs, vectorfs, contextfs, context_manager |
| **Agent** | 5 | skillfs, skill_runtime, pipefs, devfs, streamfs |
| **系统** | 5 | serverinfofs, heartbeatfs, handlefs, hellofs, catalog |

**总计**: 47 个插件

### 5.3 OpenDAL 统一接入

OpenDAL 支持 50+ 存储后端，EVIF 通过 `opendal` 插件统一接入：

```toml
# ~/.evif/config/plugins.toml
[opendal]
default_backend = "memory"

[[opendal.backends]]
name = "s3-us-east"
type = "s3"
config = { bucket = "my-bucket", region = "us-east-1" }

[[opendal.backends]]
name = "oss-cn-hangzhou"
type = "oss"
config = { bucket = "my-bucket", endpoint = "oss-cn-hangzhou.aliyuncs.com" }
```

---

## 六、生产就绪差距分析

> 基于 EVIF 全代码库审查，参考 RTK 生产级模式

### 6.0 当前代码规模

| Crate | 源文件 | 测试数 | 代码行 | 编译警告 |
|-------|--------|--------|--------|----------|
| evif-core | 32 | 61 | 11,258 | 少量 |
| evif-rest | 22 | 51 | 13,858 | 86 个 |
| evif-mcp | 7 | 136 | 12,361 | 90 个 |
| evif-cli | 8 | 36 | 5,944 | - |
| evif-plugins | 47 | 66 | 27,905 | 74 个 |
| evif-mem | 31 | 155 | 20,881 | 少量 |
| evif-auth | 6 | 31 | 1,931 | 少量 |
| evif-fuse | 5 | 28 | 2,543 | 少量 |
| evif-client | 3 | **2** | 698 | - |
| evif-metrics | 5 | **0** | 860 | - |
| **总计** | **~166** | **~566** | **~113,757** | **~250** |

### 6.1 P0 - 严重问题（生产会崩溃）

| # | 问题 | 文件 | 严重性 | 说明 |
|---|------|------|--------|------|
| 1 | **RwLock unwrap 崩溃** | `mcp_auth.rs`, `mcp_router.rs` | 🔴 CRITICAL | 22 处 `.unwrap()` 在 RwLock 上，任一线程 panic 会导致锁中毒、整个 MCP Server 崩溃。生产环境应使用 `catch_unwind` 或 `tokio::sync::RwLock` 防止级联故障 |
| 2 | **SQLite expect 崩溃** | `contextfs.rs:223,230` | 🔴 CRITICAL | `expect()` 在生产环境中 panic，DB 不可访问时整个进程挂掉 |
| 3 | **CI 测试排除** | `.github/workflows/ci.yml` | 🔴 CRITICAL | api-tests、cli-tests、e2e-tests 全部被排除，从未在 CI 中运行 |
| 4 | **evif-client 仅 2 个测试** | `evif-client/` | 🔴 HIGH | 公共 SDK 只有 2 个单元测试，用户无信心 |

**参考**：生产级 Rust 服务应使用 RAII 资源守护 + `catch_unwind`（防 panic 传播）+ 三层降级（Full → Degraded → Passthrough），确保任何错误都不会导致进程崩溃。

### 6.2 P1 - 编译警告 & 代码质量

| # | 问题 | 范围 | 说明 |
|---|------|------|------|
| 5 | **257 个编译警告** | evif-rest(86), evif-mcp(90), evif-plugins(74) | unused variables、dead code、未使用 imports |
| 6 | **公共 API 文档不足** | evif-rest(21%), evif-client(20%) | 生产库至少需要 80%+ 文档覆盖 |
| 7 | **硬编码 URL** | 6 处 localhost 默认值 | evif-mem、evif-client、evif-plugins 中 |
| 8 | **evif-cli 无 lib target** | `evif-cli/Cargo.toml` | 只有 `[[bin]]`，无法做单元测试 |
| 9 | **E2E 测试套件为空** | `tests/e2e/src/lib.rs` | 只有占位符，100 bytes |
| 10 | **evif-metrics 零测试** | `evif-metrics/` | types.rs、error.rs、prometheus.rs、lib.rs 无测试 |

**参考**：生产级 Rust 项目应使用 `#[serde(deny_unknown_fields)]` 防配置 typo，build.rs 编译时验证配置，测试确保零组件无测试覆盖。

### 6.3 P2 - 安全 & 运维

| # | 问题 | 说明 |
|---|------|------|
| 11 | **MCP Server 无限速** | REST 有 IP 限速，MCP 无，可被 DoS |
| 12 | **路径遍历防护不一致** | localfs 有防护，其他文件系统插件未验证 |
| 13 | **unsafe impl Send/Sync 无安全注释** | `dynamic_loader.rs:162-163`，应说明为何安全 |
| 14 | **无 CHANGELOG.md** | 版本统一为 0.1.0，无版本管理策略 |
| 15 | **CI 分支不匹配** | CI 引用 main/develop，实际分支是 feature-1.2 |

**参考**：生产级项目应有信任验证模型（SHA-256 信任存储）、Permission 协议（语义明确的退出码）、CI-gated override（需 CI 环境变量双重验证）。

### 6.4 生产级模式（参考学习）

| 模式 | 说明 | EVIF 应用 |
|------|------|-----------|
| **三层降级** | Full → Degraded → Passthrough | MCP 工具调用失败时应用相同模式 |
| **RAII 资源守护** | Drop 中自动释放资源 | 防止连接泄漏、句柄泄漏 |
| **信任验证** | SHA-256 信任存储 | Skill 加载时验证 SKILL.md 完整性 |
| **声明式输出过滤** | TOML 输出过滤 Pipeline | MCP 工具输出过滤（减少 token 消耗） |
| **SQLite WAL + 忙等待** | 并发安全 + 5s timeout | evif-mcp 缓存层使用相同模式 |
| **隐私遥测** | SHA-256 设备哈希 + 同意门控 | 生产使用分析 |
| **编译时验证** | build.rs 验证配置 | 验证 Skill 配置、插件配置 |
| **数量守护测试** | 确保内置组件数量不意外变化 | 确保 MCP Tools 数量一致性 |

### 6.5 功能完善清单

| # | 任务 | 说明 | 状态 | 预估 |
|---|------|------|------|------|
| 16 | **evif 命令统一** | 所有命令统一为 `evif` 前缀 | ⚠️ 待实现 | 2 天 |
| 17 | **MCP Server** | `evif mcp serve` 启动 MCP Server | ✅ 已实现 | - |
| 18 | **Skill 自动发现** | 扫描 /skills/*.md 生成 Prompts | ⚠️ 待实现 | 2 天 |
| 19 | **Memory 集成** | 向量存储 + MCP Tools | ✅ 部分实现 | 3 天 |
| 20 | **安装脚本** | 一键安装 `curl ... \| bash` | ✅ 已实现 | - |
| 21 | **Claude Desktop 连接** | `evif connect claude` | ⚠️ 待实现 | 1 天 |
| 22 | **Codex/Cursor/Gemini 集成** | 多平台一键集成 | ⚠️ 待实现 | 3 天 |
| 23 | **写操作补全** | gmailfs/slackfs/discordfs 等写入 | ⚠️ 待实现 | 5 天 |
| 24 | **Token 优化输出** | MCP 工具响应压缩 | ⚠️ 待实现 | 2 天 |
| 25 | **E2E 测试** | 50 场景测试 | ⚠️ 待实现 | 3 天 |
| 26 | **Homebrew 发布** | `brew install evif-io/evif/evif` | ⚠️ 待实现 | 1 天 |
| 27 | **性能基准测试** | 延迟/吞吐量测试 | ⚠️ 待实现 | 1 周 |

### 6.6 修复优先级排序

```
Phase 1（1 周）：生产安全
├── 修复 22 处 RwLock unwrap → 使用 tokio::sync::RwLock 或 unwrap_or_else
├── 修复 contextfs.rs expect → 返回 Result
├── 清除 257 个编译警告（deny warnings in CI）
├── evif-client 测试 → 至少 30 个
└── 启用 CI 中被排除的测试套件

Phase 2（1 周）：代码质量
├── 公共 API 文档 → evif-rest 80%+、evif-client 90%+
├── evif-cli 添加 [lib] target
├── evif-metrics 测试 → 至少 20 个
├── E2E 测试框架搭建
├── 硬编码 URL → 环境变量
└── 添加 CHANGELOG.md

Phase 3（2 周）：功能完善
├── evif 命令统一重构
├── Skill 自动发现
├── 多平台集成（Codex/Cursor/Gemini）
├── MCP Server 限速
├── 路径遍历审计
└── 安全注释（unsafe impl）

Phase 4（1 周）：发布准备
├── E2E 测试 50 场景
├── 性能基准测试
├── Homebrew formula
├── Docker 多架构构建
└── 生产文档完善
```

### 6.7 进度评估

| 维度 | 完成度 | 说明 |
|------|--------|------|
| **核心功能** | 85% | MCP Server 完成，Skill/Memory 部分完成 |
| **代码质量** | 60% | 257 警告、unwrap 崩溃风险、文档不足 |
| **测试覆盖** | 55% | 单元测试可以，E2E/集成/client 几乎空白 |
| **安全加固** | 50% | 基础 auth 有，限速/路径防护/信任验证缺失 |
| **发布准备** | 40% | 安装脚本有，缺少 CHANGELOG/Homebrew/CI 对齐 |
| **综合评估** | **58%** | 距离生产还需 ~5 周集中工作 |

---

## 七、实施时间表

### Week 1: 统一命令 + 一键安装

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | `evif` 命令重构 | 所有子命令统一为 `evif <sub>` |
| Day 3 | `evif mcp serve` | MCP Server 启动命令 |
| Day 4 | `evif connect` | AI 平台连接 |
| Day 5 | `evif skill` | Skill 发现与管理 |
| Day 6-7 | 安装脚本完善 | `curl ... \| bash` 验证 |

### Week 2: Skill + Memory 完善

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 8-9 | Skill 自动发现 | 扫描 /skills/*.md 生成 Prompts |
| Day 10-11 | Memory 增强 | 向量搜索 + MCP Tools |
| Day 12-13 | Token 优化 | MCP 输出压缩 |
| Day 14 | 集成测试 | 全部功能验证 |

### Week 3-4: 多平台 + E2E

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Week 3 | Codex/Cursor/Gemini 集成 | 各平台验证 |
| Week 3 | E2E 测试 | 50 场景测试 |
| Week 4 | 性能基准 | 延迟/吞吐量 |
| Week 4 | 文档完善 | 使用指南 |

---

## 八、关键里程碑

| 里程碑 | 完成标准 | 目标日期 |
|--------|----------|----------|
| **MVP 4.0 Alpha** | `evif` 命令统一 + MCP Server | 2026-05-11 |
| **MVP 4.0 Beta** | Skill+MCP 融合 + 一键安装 | 2026-05-18 |
| **MVP 4.0 RC** | 多平台集成 + E2E 测试通过 | 2026-05-25 |
| **大规模使用** | 100 并发 + Homebrew 发布 | 2026-06-01 |

---

## 九、真实验证

### 9.1 当前测试状态

| 测试类型 | 测试数 | 当前状态 |
|----------|--------|----------|
| Rust 单元测试 | 285+ | ✅ 通过 |
| MCP 协议测试 | 81 | ✅ 通过 |
| CLI 集成测试 | 56 | ✅ 通过 |
| E2E 场景测试 | 50 | ⚠️ 待实现 |
| 安装测试 | 5 | ⚠️ 待实现 |
| 多平台集成测试 | 6 | ⚠️ 待实现 |

### 9.2 验证命令

```bash
# 验证安装
curl -fsSL https://evif.dev/install.sh | bash
evif --version

# 验证 MCP Server
evif mcp serve --mock &
sleep 2
echo '{"jsonrpc":"2.0","id":0,"method":"tools/list"}' | evif mcp serve --mock

# 验证 Skill
evif skill ls

# 验证 Memory
evif memory memorize "测试记忆"
evif memory search "测试"

# 验证 Claude Desktop 集成
evif connect claude
```

---

## 十、竞品对比

| 特性 | EVIF | MCP Official |
|------|------|--------------|
| **核心定位** | AI Agent 虚拟文件系统 | 协议规范 |
| **存储后端** | 50+ | 0 |
| **Skill 系统** | ✅ | ❌ |
| **MCP Server** | ✅ | ✅ |
| **一键安装** | ✅ | ❌ |
| **AI 平台支持** | 8+ | 1 |
| **Token 优化** | 参考实现 | ❌ |

---

## 十一、EVIF 价值分析：解决 AI Agent 核心痛点

### 11.1 AI Agent 的五大痛点（2026 行业现状）

基于 Claude Code、OpenAI Codex、Cursor 等 AI Agent 的实际用户反馈：

| # | 痛点 | 具体表现 | 影响 |
|---|------|----------|------|
| 1 | **上下文丢失** | Claude Code 会话结束后所有决策、代码风格偏好丢失，新会话从零开始 | 重复劳动、风格不一致 |
| 2 | **Context Window 耗尽** | Codex CLI 在 ~258k tokens 处硬性截断，自动压缩经常失效，任务中途崩溃 | 工作中断、需重新提供全部上下文 |
| 3 | **无持久记忆** | Agent 无法记住上次学到的架构决策、用户偏好、项目约定 | 每次对话都是"新人" |
| 4 | **多 Agent 无法协同** | Claude Code 和 Codex 无法共享状态，无法分工协作 | 大型任务无法分解 |
| 5 | **存储碎片化** | 代码在 GitHub，文档在 Notion，知识在 Slack，Agent 需要统一接口 | 数据孤岛 |

来源：
- [Codex Context Window Issues](https://community.openai.com/t/auto-compression-not-triggering-codex-still-runs-out-of-context-window/1376334)
- [Building Persistent Memory for AI Agents with MCP](https://dev.to/ghostslk/building-persistent-memory-for-ai-agents-with-mcp-55lc)
- [6 Critical Challenges Facing MCP in 2026](https://medium.com/@MattLeads/6-critical-challenges-facing-the-mcp-in-2026-06258e914402)
- [AI Agents and Memory: Privacy and Power](https://newamerica.org/insights/ai-agents-and-memory/)

### 11.2 EVIF 如何解决每个痛点

| 痛点 | EVIF 解决方案 | 具体机制 |
|------|--------------|----------|
| **上下文丢失** | ContextFS 三层上下文 | L0(当前任务) + L1(决策记录) + L2(项目知识)，会话结束写入 L1/L2，新会话自动恢复 |
| **Context Window 耗尽** | Token 优化 + 按需加载 | evif_cat 自动截断、Memory Search 紧凑模式、分层工具加载，减少 40-60% token 消耗 |
| **无持久记忆** | MemoryFS 向量存储 | `evif_memorize` 存储任意知识，`evif_retrieve` 语义搜索，跨会话持久化 |
| **多 Agent 无法协同** | PipeFS 任务协调 | `evif pipe create` 创建共享管道，Agent A 写入 input，Agent B 读取 output |
| **存储碎片化** | 50+ 统一存储后端 | S3/SQLite/PostgreSQL/Notion/GitHub 统一为文件接口，Agent 无需关心底层 |

### 11.3 技术价值

| 价值维度 | 说明 | 量化指标 |
|----------|------|----------|
| **减少重复劳动** | Agent 每次新会话不需要重新学习项目上下文 | 节省 30-50% 初始上下文构建时间 |
| **Token 成本节省** | 输出压缩 + 按需加载减少 API 调用量 | 每会话节省 40-60% token 消耗 |
| **提高代码质量** | L2 层持久化项目约定（代码风格、架构模式） | 代码风格一致性提升 |
| **多 Agent 协同** | PipeFS 实现任务分发和结果收集 | 支持 2+ Agent 并行工作 |
| **存储统一** | 50+ 后端统一为文件语义 | Agent 只需学一套接口 |

### 11.4 商业价值

| 价值维度 | 说明 | 目标用户 |
|----------|------|----------|
| **AI 开发者效率** | 减少重复上下文构建，Agent 真正"记住"项目 | 使用 Claude Code/Codex 的独立开发者 |
| **团队协作** | 共享 L2 上下文层，团队成员共享项目知识 | AI 辅助开发团队 |
| **企业合规** | 私有化部署，数据不离开企业网络 | 企业级 AI 开发 |
| **成本控制** | Token 优化直接降低 API 费用 | 高频 AI Agent 用户 |
| **生态壁垒** | MCP Server 生态，Skill 系统可扩展 | 平台型开发者 |

### 11.5 典型使用场景

#### 场景 1：Claude Code 持久化开发

```
问题：每次 Claude Code 新会话，Agent 忘记上次的架构决策
解决：
  会话中: Agent 自动将决策写入 /context/L1/decisions.md
  会话结束: L0(当前任务) 写入 L1
  新会话: Agent 读取 /context/L1/decisions.md，恢复上下文
  
效果：Agent 从"每次都是新人"变为"有项目记忆的资深开发者"
```

#### 场景 2：多 Agent 分工协作

```
问题：Claude Code 做代码审查，Codex 做实现，但无法共享状态
解决：
  Claude Code: evif pipe create /pipes/review-pr-123
  Claude Code: evif write "审查意见..." /pipes/review-pr-123/input
  Codex: evif cat /pipes/review-pr-123/input  → 读取审查意见
  Codex: evif write "修复代码..." /pipes/review-pr-123/output
  
效果：两个 Agent 通过 EVIF 管道协同完成代码审查 + 修复
```

#### 场景 3：企业知识库统一接口

```
问题：企业知识分散在 Notion、GitHub、Slack，Agent 需要逐一接入
解决：
  evif mount add /knowledge notionfs --token xxx
  evif mount add /code githubfs --repo my-org/project
  Agent: evif ls /knowledge  → 看到 Notion 文档
  Agent: evif cat /code/src/main.rs  → 看到 GitHub 代码
  
效果：Agent 通过统一文件接口访问所有企业知识
```

### 11.6 竞争优势分析

| 能力 | EVIF | 竞品（如 Memory MCP Server） |
|------|------|-------------------------------|
| 持久化上下文 | ✅ L0/L1/L2 三层 | ❌ 通常只有单层 |
| 向量记忆 | ✅ 内置 | ⚠️ 需要外部 Qdrant |
| 多 Agent 协同 | ✅ PipeFS | ❌ 无 |
| 统一存储 | ✅ 50+ 后端 | ❌ 只有内存/SQLite |
| Skill 系统 | ✅ 可复用工作流 | ❌ 无 |
| Token 优化 | ✅ 输出压缩 | ❌ 无 |
| 多平台支持 | ✅ 8+ Agent 平台 | ⚠️ 通常只支持 Claude |
| 开源 | ✅ MIT/Apache 2.0 | ⚠️ 部分开源 |

### 11.7 关键指标

| 指标 | 目标 | 当前 |
|------|------|------|
| 安装时间 | < 1 分钟 | ✅ |
| Token 节省 | 40-60% | ⚠️ 待实现 |
| 上下文恢复率 | > 90% 决策可恢复 | ⚠️ 待验证 |
| 多 Agent 支持 | 2+ Agent 并行 | ⚠️ 待实现 |
| MCP Tools | 75 | ✅ |
| AI 平台支持 | 8+ | ⚠️ 待实现 |
| 存储后端 | 50+ | ✅ |

---

## 附录 A：完整命令参考

```bash
# Skill 系统
evif skill ls                    # 列出所有技能
evif skill info <name>            # 技能详情
evif skill run <name> [args]     # 执行技能
evif skill create <name>         # 创建技能
evif skill delete <name>         # 删除技能

# Memory 系统
evif memory memorize <text>       # 存储记忆
evif memory retrieve <query>     # 检索记忆
evif memory search <query>       # 语义搜索
evif memory stats                # 统计

# Context 系统
evif context ls                  # 列出上下文层
evif context read [L0|L1|L2]     # 读取上下文
evif context write [L0|L1|L2] <text>  # 写入上下文

# Pipe 系统
evif pipe ls                     # 列出管道
evif pipe create <name>          # 创建管道
evif pipe send <name> <msg>      # 发送消息
evif pipe recv <name>            # 接收消息

# File 系统 (VFS)
evif ls [path]                   # 列出目录
evif cat <path>                  # 读取文件
evif write <path> <content>      # 写入文件
evif mkdir <path>                # 创建目录
evif rm <path>                   # 删除文件

# MCP 系统
evif mcp serve                   # 启动 MCP Server
evif mcp ls                      # 列出 MCP Server
evif mcp add <name> <cmd>        # 添加 MCP Server

# 连接
evif connect [platform]         # 连接到 AI 平台
evif connect --list              # 列出支持的平台
evif connect --check             # 检查连接状态

# 配置
evif config get <key>             # 获取配置
evif config set <key> <value>     # 设置配置

# 诊断
evif health                      # 健康检查
evif version                     # 版本
evif stats                       # 统计
evif help                        # 帮助
```

---

## 附录 B：参考资源

| 资源 | 链接 | 说明 |
|------|------|------|
| RTK | [github.com/rtk-ai/rtk](https://github.com/rtk-ai/rtk) | Agent 集成模式参考 |
| Claude Desktop Extensions | [anthropic.com/engineering/desktop-extensions](https://www.anthropic.com/engineering/desktop-extensions) | 一键安装参考 |
| OpenAI MCP | [developers.openai.com/apps-sdk/concepts/mcp-server](https://developers.openai.com/apps-sdk/concepts/mcp-server) | Codex 集成 |
| MCP Protocol | [modelcontextprotocol.io](https://modelcontextprotocol.io) | 协议规范 |

---

## 附录 C：文件结构

```
~/.evif/
├── bin/
│   └── evif                    # 主程序
├── config/
│   ├── default.toml            # 默认配置
│   ├── mcp.toml               # MCP 配置
│   └── plugins.toml            # 插件配置
├── skills/                     # Skill 目录
│   ├── evif-context/
│   │   └── SKILL.md
│   ├── evif-memory/
│   │   └── SKILL.md
│   ├── evif-workflows/
│   │   └── SKILL.md
│   └── evif-pipes/
│       └── SKILL.md
├── hooks/                      # Agent Hook 配置
│   └── evif.toml
├── cache/                      # 缓存目录
└── logs/                       # 日志目录
```