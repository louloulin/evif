#!/bin/bash
#
# EVIF FUSE 环境自动安装脚本
#
# 功能：
# - 检测操作系统
# - 自动安装 FUSE 库
# - 配置 FUSE 环境
# - 验证安装
#
# 支持平台：
# - Linux (FUSE)
# - macOS (macFUSE/FUSE-T)
# - FreeBSD (FUSE)
#
# 使用方法：
#   bash scripts/install_fuse.sh [--help]
#

set -e  # 遇到错误时退出

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 显示帮助信息
show_help() {
    cat << EOF
EVIF FUSE 环境自动安装脚本

用法：
    bash scripts/install_fuse.sh [选项]

选项：
    --help          显示此帮助信息
    --check-only    仅检查 FUSE 环境，不安装
    --force         强制重新安装

示例：
    bash scripts/install_fuse.sh          # 检测并安装 FUSE
    bash scripts/install_fuse.sh --check-only  # 仅检查环境
    bash scripts/install_fuse.sh --force     # 强制重新安装

EOF
}

# 检测操作系统
detect_os() {
    log_info "检测操作系统..."

    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
        DISTRO=$(lsb_release -si 2>/dev/null || echo "Unknown")
        log_success "检测到 Linux 系统: $DISTRO"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        MACOS_VERSION=$(sw_vers -productVersion)
        log_success "检测到 macOS 系统: $MACOS_VERSION"
    elif [[ "$OSTYPE" == "freebsd"* ]]; then
        OS="freebsd"
        log_success "检测到 FreeBSD 系统"
    else
        log_error "不支持的操作系统: $OSTYPE"
        exit 1
    fi
}

# 检测包管理器
detect_package_manager() {
    log_info "检测包管理器..."

    if command -v apt-get &> /dev/null; then
        PKG_MANAGER="apt"
    elif command -v yum &> /dev/null; then
        PKG_MANAGER="yum"
    elif command -v dnf &> /dev/null; then
        PKG_MANAGER="dnf"
    elif command -v brew &> /dev/null; then
        PKG_MANAGER="brew"
    elif command -v pacman &> /dev/null; then
        PKG_MANAGER="pacman"
    else
        log_error "未检测到支持的包管理器"
        exit 1
    fi

    log_success "包管理器: $PKG_MANAGER"
}

# 检查 FUSE 是否已安装
check_fuse() {
    log_info "检查 FUSE 环境..."

    case $OS in
        linux|freebsd)
            if [ -c /dev/fuse ]; then
                if [ -r /dev/fuse ] && [ -w /dev/fuse ]; then
                    log_success "FUSE 设备已安装并可访问: /dev/fuse"
                    return 0
                else
                    log_warning "FUSE 设备存在但权限不足"
                    return 1
                fi
            else
                log_warning "FUSE 设备不存在: /dev/fuse"
                return 1
            fi
            ;;
        macos)
            # macOS 使用 macFUSE 或 FUSE-T
            if pkgutil --pkgs com.macfuse.filesystems.macfuse > /dev/null 2>&1; then
                log_success "macFUSE 已安装"
                return 0
            elif command -v mount_fuse &> /dev/null; then
                log_success "FUSE-T 已安装"
                return 0
            else
                log_warning "未检测到 macFUSE 或 FUSE-T"
                return 1
            fi
            ;;
        *)
            log_error "不支持的操作系统: $OS"
            return 1
            ;;
    esac
}

# 检查 Rust 环境
check_rust() {
    log_info "检查 Rust 环境..."

    if command -v rustc &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        log_success "Rust 已安装: $RUST_VERSION"
        return 0
    else
        log_warning "未检测到 Rust 环境"
        return 1
    fi
}

# 安装 FUSE (Linux)
install_fuse_linux() {
    log_info "安装 Linux FUSE..."

    case $PKG_MANAGER in
        apt)
            sudo apt-get update
            sudo apt-get install -y fuse libfuse-dev
            ;;
        yum|dnf)
            sudo $PKG_MANAGER install -y fuse fuse-devel
            ;;
        pacman)
            sudo pacman -S --noconfirm fuse2 fuse3
            ;;
        *)
            log_error "不支持的包管理器: $PKG_MANAGER"
            return 1
            ;;
    esac

    # 创建 FUSE 组（如果不存在）
    if ! getent group fuse > /dev/null 2>&1; then
        log_info "创建 fuse 组..."
        sudo groupadd fuse
    fi

    # 将当前用户添加到 fuse 组
    if ! groups | grep -q fuse; then
        log_info "将当前用户添加到 fuse 组..."
        sudo usermod -a -G fuse $USER
        log_warning "请注销并重新登录以使组权限生效"
    fi

    # 配置 FUSE 权限
    sudo chmod 666 /dev/fuse || true

    log_success "Linux FUSE 安装完成"
}

# 安装 FUSE (macOS)
install_fuse_macos() {
    log_info "安装 macOS FUSE..."

    # 检查 Homebrew 是否安装
    if ! command -v brew &> /dev/null; then
        log_error "Homebrew 未安装，请先安装 Homebrew"
        log_info "安装 Homebrew: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        return 1
    fi

    log_info "使用 Homebrew 安装 macFUSE..."

    # 选择安装方式
    echo "请选择安装方式:"
    echo "1) macFUSE (推荐，需要内核扩展)"
    echo "2) FUSE-T (macOS 10.15+，无需内核扩展)"
    read -p "选择 [1/2]: " choice

    case $choice in
        1)
            log_info "安装 macFUSE..."
            brew install macfuse

            log_warning "macFUSE 安装后需要:"
            log_warning "1. 打开 系统偏好设置 > 安全性与隐私"
            log_warning "2. 找到 'Benjamin Fleischer' 并允许"
            log_warning "3. 或在终端运行: brew install --cask macfuse"
            ;;
        2)
            log_info "安装 FUSE-T..."
            brew install --cask macfuse
            ;;
        *)
            log_error "无效选择"
            return 1
            ;;
    esac

    log_success "macOS FUSE 安装完成"
}

# 安装 FUSE (FreeBSD)
install_fuse_freebsd() {
    log_info "安装 FreeBSD FUSE..."

    log_info "加载 FUSE 内核模块..."
    sudo kldload fuse

    log_info "启用 FUSE 服务..."
    sudo sysrc fusefs_enable="YES"
    sudo service fusefs start

    log_success "FreeBSD FUSE 安装完成"
}

# 验证安装
verify_installation() {
    log_info "验证 FUSE 安装..."

    if check_fuse; then
        log_success "FUSE 安装验证成功"

        # 测试 FUSE 功能
        log_info "测试 FUSE 功能..."

        case $OS in
            linux|freebsd)
                if [ -c /dev/fuse ] && [ -r /dev/fuse ] && [ -w /dev/fuse ]; then
                    log_success "FUSE 设备测试通过"
                else
                    log_error "FUSE 设备测试失败"
                    return 1
                fi
                ;;
            macos)
                if command brew ls --versions macfuse 2>/dev/null | grep -q macfuse; then
                    log_success "macFUSE 测试通过"
                elif command -v mount_fuse &> /dev/null; then
                    log_success "FUSE-T 测试通过"
                else
                    log_error "macOS FUSE 测试失败"
                    return 1
                fi
                ;;
        esac

        return 0
    else
        log_error "FUSE 安装验证失败"
        return 1
    fi
}

# 编译 EVIF FUSE 模块
compile_evif_fuse() {
    log_info "编译 EVIF FUSE 模块..."

    if [ ! -f "Cargo.toml" ]; then
        log_warning "未找到 Cargo.toml，跳过编译"
        return 0
    fi

    # 尝试编译 evif
    log_info "编译 evif-fuse crate..."
    if cargo build -p evif-fuse --release 2>&1 | grep -q "Compiling evif-fuse"; then
        log_success "EVIF FUSE 编译成功"

        # 显示编译的二进制文件
        if [ -f "target/release/libevif_fuse.a" ] || [ -f "target/release/libevif_fuse.rlib" ]; then
            log_info "FUSE 库位置: target/release/libevif_fuse.*"
        fi

        return 0
    else
        log_error "EVIF FUSE 编译失败"
        log_info "请检查 evif-fuse crate 的依赖和配置"
        return 1
    fi
}

# 主函数
main() {
    # 解析命令行参数
    CHECK_ONLY=false
    FORCE=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                show_help
                exit 0
                ;;
            --check-only)
                CHECK_ONLY=true
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            *)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done

    echo "=============================================="
    echo "EVIF FUSE 环境自动安装脚本"
    echo "=============================================="
    echo ""

    # 检测操作系统和包管理器
    detect_os
    detect_package_manager
    echo ""

    # 检查 Rust 环境
    if ! check_rust; then
        log_warning "Rust 未安装，将无法编译 EVIF FUSE 模块"
        log_info "安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
    echo ""

    # 检查 FUSE
    if check_fuse; then
        if [ "$FORCE" = true ]; then
            log_info "强制重新安装 FUSE (--force)"
        elif [ "$CHECK_ONLY" = true ]; then
            log_success "FUSE 环境检查通过 (--check-only)"
            exit 0
        else
            log_success "FUSE 已安装，跳过安装"
            echo ""

            # 尝试编译 EVIF FUSE
            compile_evif_fuse

            echo ""
            echo "=============================================="
            log_success "EVIF FUSE 环境就绪"
            echo "=============================================="
            exit 0
        fi
    else
        log_warning "FUSE 未安装"

        if [ "$CHECK_ONLY" = true ]; then
            log_error "FUSE 环境检查失败 (--check-only)"
            exit 1
        fi
    fi

    # 安装 FUSE
    echo ""
    log_info "开始安装 FUSE..."

    case $OS in
        linux)
            install_fuse_linux
            ;;
        macos)
            install_fuse_macos
            ;;
        freebsd)
            install_fuse_freebsd
            ;;
    esac

    echo ""

    # 验证安装
    if verify_installation; then
        echo ""

        # 尝试编译 EVIF FUSE
        compile_evif_fuse

        echo ""
        echo "=============================================="
        log_success "FUSE 安装完成"
        echo "=============================================="

        case $OS in
            linux|freebsd)
                echo ""
                echo "后续步骤:"
                echo "1. 如果添加了 fuse 组，请注销并重新登录"
                echo "2. 运行: cargo build --release"
                echo "3. 使用 evif-fuse 挂载文件系统"
                ;;
            macos)
                echo ""
                echo "后续步骤:"
                echo "1. 如果是 macFUSE，请允许内核扩展"
                echo "2. 运行: cargo build --release"
                echo "3. 使用 evif-fuse 挂载文件系统"
                ;;
        esac

        exit 0
    else
        log_error "FUSE 安装验证失败"
        exit 1
    fi
}

# 执行主函数
main "$@"
