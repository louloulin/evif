# EVIF - 万物皆文件系统

> 基于 Rust 的图文件系统实现，遵循 Plan 9 "万物皆文件" 哲学

## 🎯 第一阶段完成情况

### 已实现模块

| Crate | 描述 | 状态 | 测试 |
|-------|------|------|------|
| `evif-graph` | 图引擎：节点、边、图结构、遍历算法 | ✅ | 12/12 ✓ |
| `evif-storage` | 存储层：trait-based 抽象、内存后端 | ✅ | 7/7 ✓ |
| `evif-auth` | 认证授权：能力系统、权限管理 | ✅ | 7/7 ✓ |
| `evif-runtime` | 运行时核心：配置、组件编排 | ✅ | 4/4 ✓ |

**总测试数**: 30 个单元测试，全部通过 ✅

## 🚀 快速开始

### 构建项目

\`\`\`bash
# 构建所有 crates
cargo build --release

# 构建特定 crate
cargo build -p evif-graph --release
\`\`\`

### 运行测试

\`\`\`bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 测试
cargo test -p evif-graph
cargo test -p evif-storage
cargo test -p evif-auth
cargo test -p evif-runtime
\`\`\`

## 📦 架构设计

### 高内聚低耦合

- **evif-graph**: 图数据结构和算法，完全独立
- **evif-storage**: trait-based 存储抽象，可插拔后端
- **evif-auth**: 独立的安全层，基于能力的安全模型
- **evif-runtime**: 配置驱动的组件初始化和编排

## 📋 开发计划

详见 [plan1.1.md](./plan1.1.md)

---

*第一阶段完成于 2025-01-14*
