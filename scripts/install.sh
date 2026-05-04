#\!/bin/bash
# EVIF 一键安装脚本
# 用法: curl -fsSL https://evif.dev/install.sh | bash

set -euo pipefail

EVIF_VERSION="${EVIF_VERSION:-latest}"
EVIF_HOME="${EVIF_HOME:-$HOME/.evif}"
INSTALL_DIR="$EVIF_HOME/bin"
REPO="evif-io/evif"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info() { echo -e "${GREEN}[EVIF]${NC} $*"; }
warn() { echo -e "${YELLOW}[EVIF]${NC} $*"; }
error() { echo -e "${RED}[EVIF]${NC} $*" >&2; }

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        *)      error "不支持: $os"; exit 1 ;;
    esac
    case "$arch" in
        x86_64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)       error "不支持: $arch"; exit 1 ;;
    esac
    echo "${arch}-${os}"
}

install() {
    local platform version url
    platform="$(detect_platform)"
    version="${EVIF_VERSION}"
    
    if [ "$version" = "latest" ]; then
        info "获取最新版本..."
        version=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | sed 's/.*"v\?\([^"]*\)".*/\1/' || echo "0.0.1")
    fi
    
    info "安装 evif $version ($platform)..."
    url="https://github.com/${REPO}/releases/download/v${version}/evif-${platform}.tar.gz"
    
    mkdir -p "$INSTALL_DIR"
    
    if curl -fsSL "$url" -o /tmp/evif.tar.gz 2>/dev/null; then
        tar xzf /tmp/evif.tar.gz -C "$INSTALL_DIR"
        rm -f /tmp/evif.tar.gz
        chmod +x "$INSTALL_DIR/evif"
        info "二进制安装成功"
    else
        warn "下载失败，尝试 cargo install..."
        if command -v cargo &> /dev/null; then
            cargo install evif-mcp --locked 2>/dev/null || cargo install evif-mcp
        else
            error "请安装 Rust (https://rustup.rs) 或检查网络"
            exit 1
        fi
    fi
}

init_config() {
    mkdir -p "$EVIF_HOME/config"
    mkdir -p "$EVIF_HOME/skills"
    
    if [ \! -f "$EVIF_HOME/config/default.toml" ]; then
        cat > "$EVIF_HOME/config/default.toml" << 'CONFIG_EOF'
[evif]
version = "4.0.0"

[mcp]
protocol_version = "2024-11-05"
server_name = "evif-mcp"

[skills]
path = "~/.evif/skills"
auto_discover = true

[memory]
provider = "vector"
CONFIG_EOF
        info "配置文件已创建"
    fi
}

setup_path() {
    local shell_config=""
    case "$SHELL" in
        */zsh)  shell_config="$HOME/.zshrc" ;;
        */bash) shell_config="$HOME/.bashrc" ;;
        *)      shell_config="$HOME/.profile" ;;
    esac
    
    if [ -f "$shell_config" ]; then
        if \! grep -q ".evif/bin" "$shell_config" 2>/dev/null; then
            echo '' >> "$shell_config"
            echo '# EVIF' >> "$shell_config"
            echo 'export PATH="$HOME/.evif/bin:$PATH"' >> "$shell_config"
            info "已添加到 PATH (source $shell_config 或重启终端)"
        fi
    fi
}

main() {
    echo ""
    echo "  ███████╗██╗  ██╗ █████╗ "
    echo "  ██╔════╝██║ ██╔╝██╔══██╗"
    echo "  ███████╗█████╔╝ ╚█████╔╝"
    echo "  ╚════██║██╔═██╗ ██╔══██╗"
    echo "  ███████║██║ ██╗ ╚█████╔╝"
    echo "  ╚══════╝╚═╝╚═╝  ╚════╝"
    echo ""
    echo "  EVIF - Everything Is a File"
    echo "  AI Agent 统一存储基础设施"
    echo ""
    
    install
    init_config
    setup_path
    
    echo ""
    echo "✅ 安装完成\!"
    echo ""
    echo "下一步:"
    echo "  evif --help                       # 查看帮助"
    echo "  evif integrate --platform claude-desktop  # 配置 Claude Desktop"
    echo "  evif skill ls                    # 列出技能"
    echo "  evif mcp serve                   # 启动 MCP Server"
    echo ""
}

main "$@"
