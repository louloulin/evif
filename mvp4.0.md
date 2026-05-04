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

### 1.2 RTK 启示

RTK ([github.com/rtk-ai/rtk](https://github.com/rtk-ai/rtk)) 证明了一个关键设计模式：

| RTK 设计原则 | EVIF 应用 |
|-------------|-----------|
| **单二进制、零依赖** | `evif` 一个 Rust binary |
| **CLI proxy 模式** | `evif <cmd>` 作为统一入口 |
| **60-90% token 节省** | 缓存 + 批量操作 |
| **30+ 命令支持** | 47 插件 + 75 MCP Tools |
| **Hook 透明拦截** | `evif git status` → 自动 token 优化 |

**EVIF 要做的是 AI Agent 存储领域的 RTK**：一个 `evif` 命令统一所有操作。

---

## 二、EVIF 统一命令架构

### 2.1 命令设计

```bash
# 核心命令 (类比 RTK)
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

# MCP 集成 (类比 RTK proxy)
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

### 2.2 RTK Hook 模式

```bash
# RTK 风格的透明 Hook
# 在 ~/.evif/hooks 中配置需要拦截的命令

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

### 3.2 Claude Code 集成

#### 安装后一行配置

```bash
# 一键配置 Claude Desktop
evif integrate --platform claude-desktop

# 这会自动：
# 1. 检测 Claude Desktop 配置
# 2. 添加 EVIF MCP Server
# 3. 配置 Skill 发现
# 4. 添加 CLAUDE.md 路由规则
```

#### 配置文件格式

```json
// ~/.claude/settings.json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"],
      "env": {
        "EVIF_SKILLS_PATH": "~/.evif/skills"
      }
    }
  }
}
```

### 3.3 OpenAI Codex 集成

```bash
# 一键配置 Codex
evif integrate --platform codex

# ~/.codex/config.json
{
  "mcpServers": {
    "evif": {
      "command": "evif",
      "args": ["mcp", "serve"]
    }
  }
}
```

### 3.4 多平台一键集成

```bash
# 支持的平台
evif integrate --platform claude-desktop  # Claude Desktop
evif integrate --platform claude-code     # Claude Code CLI
evif integrate --platform codex            # OpenAI Codex
evif integrate --platform cursor          # Cursor
evif integrate --platform gemini          # Gemini CLI
evif integrate --platform copilot        # GitHub Copilot

# 列出支持的平台
evif integrate --list

# 交互式配置
evif integrate
```

---

## 四、一键安装系统

### 4.1 核心安装命令

```bash
# 一键安装 (类比 RTK)
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
    echo "  evif integrate --platform claude-desktop  # 配置 Claude Desktop"
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

## 五、插件系统

### 5.1 插件架构

```bash
# 内置插件 (编译时决定)
evif plugin ls                   # 列出已安装插件

# WASM 动态插件 (类比 RTK 扩展)
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

## 六、大规模使用前待完成

### 6.1 P0 - 生产就绪

| 任务 | 说明 | 状态 |
|------|------|------|
| **evif 命令统一** | 所有命令统一为 `evif` 前缀 | ⚠️ 待实现 |
| **MCP Server** | `evif mcp serve` 启动 MCP Server | ✅ 已实现 |
| **Skill 发现** | 自动扫描 /skills/*.md 生成 Prompts | ⚠️ 待实现 |
| **Memory 集成** | 向量存储 + MCP Tools | ✅ 部分实现 |
| **安装脚本** | 一键安装 `curl ... \| bash` | ✅ 已实现 |
| **Claude Desktop 集成** | `evif integrate --platform claude-desktop` | ⚠️ 待实现 |

### 6.2 P1 - 完善功能

| 任务 | 说明 | 预估 |
|------|------|------|
| **Codex 集成** | `evif integrate --platform codex` | 1 天 |
| **Cursor 集成** | `evif integrate --platform cursor` | 1 天 |
| **Gemini CLI 集成** | `evif integrate --platform gemini` | 1 天 |
| **写操作补全** | gmailfs/slackfs/discordfs 等写入 | 5 天 |
| **RTK Hook 模式** | `alias git=evif git` 透明拦截 | 2 天 |
| **E2E 测试** | 50 场景测试 | 3 天 |

### 6.3 P2 - 锦上添花

| 任务 | 说明 | 预估 |
|------|------|------|
| **100 MCP Server** | mem33.md 路线图 | 3-6 月 |
| **性能基准测试** | 延迟/吞吐量测试 | 1 周 |
| **Homebrew 发布** | `brew install evif-io/evif/evif` | 1 天 |
| **Docker 镜像** | 多架构构建 | 1 天 |

---

## 七、实施时间表

### Week 1: 统一命令 + 一键安装

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 1-2 | `evif` 命令重构 | 所有子命令统一为 `evif <sub>` |
| Day 3 | `evif mcp serve` | MCP Server 启动命令 |
| Day 4 | `evif integrate` | AI 平台一键集成 |
| Day 5 | `evif skill` | Skill 发现与管理 |
| Day 6-7 | 安装脚本完善 | `curl ... \| bash` 验证 |

### Week 2: Skill + Memory 完善

| 日期 | 任务 | 交付物 |
|------|------|--------|
| Day 8-9 | Skill 自动发现 | 扫描 /skills/*.md 生成 Prompts |
| Day 10-11 | Memory 增强 | 向量搜索 + MCP Tools |
| Day 12-13 | RTK Hook 模式 | 透明命令拦截 |
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
evif integrate --platform claude-desktop
```

---

## 十、竞品对比

| 特性 | EVIF | RTK | MCP Official |
|------|------|-----|--------------|
| **命令统一** | ✅ `evif` | ✅ `rtk` | ❌ 分散 |
| **存储后端** | 50+ | 0 | 0 |
| **Skill 系统** | ✅ | ❌ | ❌ |
| **MCP Server** | ✅ | ❌ | ✅ |
| **一键安装** | ✅ | ✅ | ❌ |
| **AI 平台支持** | 6+ | 0 | 1 |

---

## 十一、总结

### EVIF 的核心价值

1. **命令统一**：`evif` 一个命令覆盖所有操作
2. **Skill + MCP 融合**：技能系统与 MCP 协议深度结合
3. **一键安装**：`curl ... | bash` 即可使用
4. **多平台支持**：Claude / Codex / Cursor / Gemini / Copilot
5. **50+ 存储后端**：通过 OpenDAL 统一接入

### 关键指标

| 指标 | 目标 | 当前 |
|------|------|------|
| 安装时间 | < 1 分钟 | ✅ |
| 命令数量 | < 20 子命令 | ⚠️ |
| Skill 数量 | 自动发现 | ⚠️ |
| MCP Tools | 75 | ✅ |
| MCP Prompts | 自动生成 | ⚠️ |
| AI 平台支持 | 6+ | ⚠️ |

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

# 集成
evif integrate [platform]         # AI 平台集成
evif integrate --list            # 列出支持的平台

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
| RTK | [github.com/rtk-ai/rtk](https://github.com/rtk-ai/rtk) | CLI 设计参考 |
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
├── hooks/                      # RTK Hook 配置
│   └── evif.toml
├── cache/                      # 缓存目录
└── logs/                       # 日志目录
```