#!/bin/bash

# EVIF Web UI 测试脚本
# 测试构建、类型检查和基本功能

set -e

echo "🧪 EVIF Web UI 测试套件"
echo "========================"
echo ""

# 1. 类型检查
echo "📋 1. 运行 TypeScript 类型检查..."
bun run typecheck
if [ $? -eq 0 ]; then
    echo "✅ 类型检查通过"
else
    echo "❌ 类型检查失败"
    exit 1
fi
echo ""

# 2. 生产构建
echo "🔨 2. 运行生产构建..."
bun run build
if [ $? -eq 0 ]; then
    echo "✅ 生产构建成功"

    # 检查构建输出
    if [ -f "build/main.js" ]; then
        SIZE=$(du -h build/main.js | cut -f1)
        echo "   📦 main.js: $SIZE"
    fi

    if [ -f "build/main.css" ]; then
        SIZE=$(du -h build/main.css | cut -f1)
        echo "   📦 main.css: $SIZE"
    fi
else
    echo "❌ 生产构建失败"
    exit 1
fi
echo ""

# 3. 检查源文件
echo "📂 3. 检查源文件结构..."

FILES=(
    "src/main.tsx"
    "src/App.tsx"
    "src/App.css"
    "src/components/MenuBar.tsx"
    "src/components/FileTree.tsx"
    "src/components/Editor.tsx"
    "src/components/Terminal.tsx"
    "src/components/ContextMenu.tsx"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        LINES=$(wc -l < "$file")
        echo "   ✅ $file ($LINES lines)"
    else
        echo "   ❌ $file 缺失"
        exit 1
    fi
done
echo ""

# 4. 配置文件检查
echo "📄 4. 检查配置文件..."
CONFIG_FILES=(
    "package.json"
    "tsconfig.json"
    ".gitignore"
    "index.html"
    "README.md"
)

for file in "${CONFIG_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "   ✅ $file"
    else
        echo "   ❌ $file 缺失"
        exit 1
    fi
done
echo ""

# 5. 统计信息
echo "📊 5. 代码统计..."
TSX_FILES=$(find src -name "*.tsx" -type f | wc -l)
TOTAL_LINES=$(find src -name "*.tsx" -o -name "*.css" | xargs wc -l | tail -1 | awk '{print $1}')
echo "   📝 TSX 组件数: $TSX_FILES"
echo "   📏 总代码行数: $TOTAL_LINES"
echo ""

echo "========================"
echo "✅ 所有测试通过！"
echo ""
echo "🚀 启动开发服务器:"
echo "   bun run dev"
echo ""
echo "📦 构建生产版本:"
echo "   bun run build"
