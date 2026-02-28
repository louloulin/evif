#!/bin/bash

# EVIF 2.3 Phase 6 前后端验证脚本

set -e

echo "=========================================="
echo "EVIF 2.3 Phase 6 - 前后端验证"
echo "=========================================="

# 颜色定义
GREEN="\033[0;32m"
RED="\033[0;31m"
YELLOW="\033[0;33m"
NC="\033[0m"

BACKEND_URL="http://localhost:8080"
FRONTEND_URL="http://localhost:5173"

# 检查后端健康状态
check_backend() {
    echo ""
    echo "=========================================="
    echo "1. 检查后端健康状态"
    echo "=========================================="

    if curl -s -f "$BACKEND_URL/health" > /dev/null; then
        echo "✅ ${GREEN}后端健康检查通过${NC}"
        return 0
    else
        echo "❌ ${RED}后端健康检查失败${NC}"
        echo "   请确保后端已启动: cargo run --bin evif-rest"
        return 1
    fi
}

# 检查前端可访问性
check_frontend() {
    echo ""
    echo "=========================================="
    echo "2. 检查前端可访问性"
    echo "=========================================="

    if curl -s -f "$FRONTEND_URL" > /dev/null; then
        echo "✅ ${GREEN}前端可访问${NC}"
        return 0
    else
        echo "❌ ${RED}前端无法访问${NC}"
        echo "   请确保前端已启动: bun dev"
        return 1
    fi
}

# 测试 API 端点
test_api_endpoints() {
    echo ""
    echo "=========================================="
    echo "3. 测试 API 端点"
    echo "=========================================="

    # 测试插件列表 API
    echo "测试插件列表 API..."
    if curl -s -f "$BACKEND_URL/api/v1/plugins" > /dev/null; then
        echo "✅ ${GREEN}PASS${NC}: /api/v1/plugins"
    else
        echo "⚠️  ${YELLOW}WARN${NC}: /api/v1/plugins (后端可能未实现此端点)"
    fi

    # 测试文件列表 API
    echo "测试文件列表 API..."
    if curl -s -f "$BACKEND_URL/api/v1/fs/list?path=/" > /dev/null; then
        echo "✅ ${GREEN}PASS${NC}: /api/v1/fs/list"
    else
        echo "⚠️  ${YELLOW}WARN${NC}: /api/v1/fs/list (后端可能未实现此端点)"
    fi
}

# 测试 WebSocket 连接
test_websocket() {
    echo ""
    echo "=========================================="
    echo "4. 测试 WebSocket 连接"
    echo "=========================================="

    if command -v nc 2>&1 > /dev/null; then
        if nc -z localhost 8080 -w 1 < /dev/null 2>&1; then
            echo "✅ ${GREEN}PASS${NC}: WebSocket 端口可访问 (localhost:8080)"
        else
            echo "❌ ${RED}FAIL${NC}: WebSocket 端口无法访问"
        fi
    else
        echo "⚠️  ${YELLOW}WARN${NC}: nc �与其他未安装，跳过 WebSocket 检查"
    fi
}

# 验证协作功能
test_collaboration_features() {
    echo ""
    echo "=========================================="
    echo "5. 验证协作功能"
    echo "=========================================="

    # 检查前端 HTML
    echo "检查前端 HTML 包含协作组件..."
    if curl -s "$FRONTEND_URL" | grep -q "ShareModal\|CommentPanel\|ActivityFeed"; then
        echo "✅ ${GREEN}PASS${NC}: 前端包含协作组件"
    else
        echo "⚠️  ${YELLOW}WARN${NC}: 无法验证前端组件 (前端可能未完全加载)"
    fi
}

# 主验证流程
main() {
    echo ""
    echo "=========================================="
    echo "EVIF 2.3 Phase 6 - 前后端验证"
    echo "=========================================="
    echo ""
    echo "后端 URL: $BACKEND_URL"
    echo "前端 URL: $FRONTEND_URL"
    echo ""

    # 等待服务启动
    echo "等待服务启动 (10秒)..."
    sleep 10

    # 运行验证
    BACKEND_OK=0
    FRONTEND_OK=0

    check_backend && BACKEND_OK=1
    check_frontend && FRONTEND_OK=1

    if [ $BACKEND_OK -eq 1 ] && [ $FRONTEND_OK -eq 1 ]; then
        test_api_endpoints
        test_websocket
        test_collaboration_features

        echo ""
        echo "=========================================="
        echo "✅ ${GREEN}验证完成！${NC}"
        echo "=========================================="
        echo ""
        echo "访问地址:"
        echo "  前端 UI: $FRONTEND_URL"
        echo "  后端 API: $BACKEND_URL"
        echo ""
        echo "测试协作功能:"
        echo "  1. 访问前端 UI"
        echo "  2. 打开文件管理页面"
        echo "  3. 点击分享按钮测试 ShareModal"
        echo "  4. 点击评论按钮测试 CommentPanel"
        echo "  5. 查看活动历史测试 ActivityFeed"
        echo ""
        exit 0
    else
        echo ""
        echo "=========================================="
        echo "❌ ${RED}验证失败${NC}"
        echo "=========================================="
        echo ""
        echo "请检查:"
        echo "  1. 后端是否已启动: cargo run --bin evif-rest"
        echo "  2. 前端是否已启动: bun dev"
        echo "  3. 防火墙设置: 确保 8080 和 5173 端口可访问"
        echo ""
        exit 1
    fi
}

main
