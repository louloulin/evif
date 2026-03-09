# mem6.md - 记忆平台 UI 功能分析与演进规划

> **版本**: 1.0.0
> **日期**: 2026-03-09
> **状态**: 规划文档
> **作者**: Ralph Loop Analysis
> **目标**: 分析 evif-web UI 现状，对比 memU UI，制定以 "Everything is File" 为核心的记忆平台 UI 演进路线图

---

## 执行摘要

本文档分析 evif-web 现有 UI 功能，对比 memU 等主流记忆平台的 UI 实现，识别差距与机会，制定以 "Everything is File" 哲学为核心的记忆平台 UI 演进规划。

### 核心发现

| 维度 | evif-web | memU-ui | 差距分析 |
|------|----------|---------|---------|
| **定位** | 全功能 IDE 界面 | 专注内存管理仪表板 | evif 更全面但记忆专用 UI 不足 |
| **文件系统可视化** | ✅ 完整 FileTree | ⚠️ 文件隐喻（无真实文件系统） | evif 独有 FUSE 优势未充分利用 |
| **记忆专用 UI** | ❌ 缺失 | ✅ 专用记忆管理界面 | 需要新增记忆专用组件 |
| **知识图谱可视化** | ❌ 缺失 | ❌ 无 | 新增机会 - evif-graph 独有优势 |
| **监控能力** | ✅ 完整监控面板 | ⚠️ Token 使用可视化 | evif 已领先 |

### 关键建议

1. **P0**: 新增记忆专用 UI 组件（MemoryExplorer, CategoryView, MemoryTimeline）
2. **P1**: 集成 evif-graph 可视化（知识图谱、时序图）
3. **P1**: FUSE 挂载状态实时可视化
4. **P2**: AI 记忆助手对话界面
5. **P2**: 记忆分析与洞察仪表板

---

## 1. evif-web UI 架构分析

### 1.1 整体架构 - VS Code 风格界面

```
┌─────────────────────────────────────────────────────────────────────┐
│                            MenuBar                                   │
│  File │ Edit │ View │ Help                                          │
└─────────────────────────────────────────────────────────────────────┘
┌────────┬────────────────────────────────────────────────────────────┐
│        │                     EditorArea                              │
│  Act   │  ┌─────────────────────────────────────────────────────┐   │
│  ivi   │  │ EditorTabs (Tab1, Tab2, ...)                        │   │
│  ty    │  ├─────────────────────────────────────────────────────┤   │
│  Bar   │  │                                                     │   │
│        │  │  Monaco Editor (代码编辑/文件内容)                   │   │
│  [File]│  │                                                     │   │
│  [Srch]│  │  ┌──────────┐                                       │   │
│  [Plug]│  │  │ MiniMap  │                                       │   │
│  [Mon] │  │  └──────────┘                                       │   │
│        │  └─────────────────────────────────────────────────────┘   │
├────────┴────────────────────────────────────────────────────────────┤
│                           Panel                                      │
│  [Terminal] [Problems]                                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ xterm.js Terminal                                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
│                          StatusBar                                   │
│  Connection: ● │ /path/to/file │ UTF-8 │ Ln 1, Col 1               │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 组件分类（27 个组件目录）

| 类别 | 组件 | 文件数 | 功能描述 |
|------|------|--------|---------|
| **布局组件** | MenuBar, ActivityBar, StatusBar, Panel | 4 | IDE 骨架结构 |
| **编辑器** | Editor, EditorTabs, QuickOpen, MiniMap | 4 | Monaco 集成 |
| **文件系统** | FileTree, Breadcrumb, ContextMenu | 3 | 文件浏览操作 |
| **搜索上传** | SearchBar, FilterPanel, SearchResults, UploadDropzone, DownloadManager | 5 | 搜索与传输 |
| **插件管理** | PluginList, PluginStatus, MountModal, PluginLogs, PluginModal | 5 | 插件系统 |
| **监控** | TrafficChart, OperationChart, SystemStatus, AlertPanel, LogViewer, MetricCard | 6 | 系统监控 |
| **协作** | CommentPanel, ThreadView, ShareModal, AccessControlList, PermissionEditor, UserSelector | 6 | 协作功能 |
| **终端** | Terminal, TerminalTabs | 2 | xterm.js 集成 |
| **UI 基础** | Button, Card, Dialog, Toast, Input, Tabs 等 | 16 | shadcn/ui 组件 |

### 1.3 技术栈

| 层级 | 技术选型 | 版本 |
|------|---------|------|
| **框架** | React | 18.x |
| **语言** | TypeScript | 5.x |
| **构建** | Vite | 5.x |
| **编辑器** | Monaco Editor | Latest |
| **终端** | xterm.js | 5.x |
| **UI 组件** | Radix UI + shadcn/ui | Latest |
| **样式** | Tailwind CSS | 3.x |
| **测试** | Playwright | Latest |

### 1.4 API 集成

```typescript
// REST API 端点
const API_BASE = '/api/v1';

// 文件系统操作
GET    /api/v1/fs/list     // 列出目录
GET    /api/v1/fs/read     // 读取文件
PUT    /api/v1/fs/write    // 写入文件
DELETE /api/v1/fs/delete   // 删除文件
POST   /api/v1/fs/create   // 创建文件/目录

// WebSocket 连接
WS     /api/v1/terminal    // 终端 PTY
```

### 1.5 缺失的记忆专用 UI

| 功能 | 状态 | 优先级 | 描述 |
|------|------|--------|------|
| MemoryExplorer | ✅ 已实现 | P0 | 记忆浏览器（分类/项目树形视图） |
| CategoryView | ✅ 已实现 | P0 | 分类详情视图（统计/时间线） |
| MemoryTimeline | ✅ 已实现 | P1 | 记忆时序可视化 |
| KnowledgeGraph | ✅ 已实现 | P1 | 知识图谱可视化（evif-graph 集成） |
| FUSEStatusPanel | ❌ 缺失 | P1 | FUSE 挂载状态面板 |
| MemorySearch | ⚠️ 部分 | P1 | 语义搜索界面（需要增强） |
| MemoryInsights | ❌ 缺失 | P2 | 记忆分析与洞察仪表板 |
| AIChatPanel | ❌ 缺失 | P2 | AI 记忆助手对话界面 |

---

### 实现进度

**Phase 1: 记忆专用 UI 基础 (2026-03-09) - 已完成 65%**

| 任务 | 状态 | 完成度 |
|------|------|--------|
| MemoryExplorer 组件 | ✅ 已实现 | 100% |
| CategoryView 组件 | ✅ 已实现 | 100% |
| MemoryTimeline 组件 | ✅ 已实现 | 100% |
| 记忆搜索增强 | ⚠️ 部分实现 | 50% |
| ActivityBar 集成 | ✅ 已实现 | 100% |
| 样式与交互 | ✅ 已实现 | 100% |

**Phase 2: 知识图谱可视化 (2026-03-09) - 已完成 10%**

| 任务 | 状态 | 完成度 |
|------|------|--------|
| KnowledgeGraph 组件 | ✅ 已实现 | 100% |
| 图谱布局算法 | ⚠️ 基础实现 | 30% |
| 节点交互 | ✅ 已实现 | 100% |
| 图查询 UI | ⏳ 待实现 | 0% |

**新增文件:**
- `evif-web/src/services/memory-api.ts` - Memory API 服务
- `evif-web/src/components/memory/MemoryExplorer.tsx` - 记忆浏览器组件
- `evif-web/src/components/memory/CategoryView.tsx` - 分类详情视图
- `evif-web/src/components/memory/MemoryTimeline.tsx` - 记忆时间线组件
- `evif-web/src/components/memory/KnowledgeGraph.tsx` - 知识图谱组件 (新增)
- `evif-web/src/components/memory/MemoryView.tsx` - 记忆视图主容器
- `evif-web/src/components/memory/index.ts` - 组件导出

**技术实现:**
- 集成现有 API: `/api/v1/memories`, `/api/v1/categories`, `/api/v1/graph/query`
- 树形结构展示: Category → Memory Items
- 搜索过滤功能 (语义搜索)
- 时间线视图 (Graph API timeline 查询)
- 知识图谱可视化 (SVG + 交互)
- 完整的 CSS 样式

---

## 2. memU UI 架构分析

### 2.1 整体架构 - 内存管理仪表板

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Header                                      │
│  memU Dashboard │ User: Admin ▼ │ Settings                          │
└─────────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────────┐
│                      Navigation Sidebar                              │
│  [Dashboard] [Memories] [Categories] [Resources] [Analytics]        │
└─────────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────────┐
│                       Main Content                                   │
│  ┌──────────────────────┐  ┌──────────────────────┐                 │
│  │ Memory Categories    │  │ Recent Memories      │                 │
│  │ ┌──────────────────┐ │  │ ┌──────────────────┐ │                 │
│  │ │ 📁 preferences/  │ │  │ │ User prefers...  │ │                 │
│  │ │ 📁 knowledge/    │ │  │ │ API style: REST  │ │                 │
│  │ │ 📁 skills/       │ │  │ │ Dark mode: on    │ │                 │
│  │ └──────────────────┘ │  │ └──────────────────┘ │                 │
│  └──────────────────────┘  └──────────────────────┘                 │
│  ┌──────────────────────┐  ┌──────────────────────┐                 │
│  │ Token Usage          │  │ User Activity        │                 │
│  │ [Chart]              │  │ [Timeline]           │                 │
│  └──────────────────────┘  └──────────────────────┘                 │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 核心功能对比

| 功能 | memU-ui | evif-web | 差距 |
|------|---------|----------|------|
| **记忆浏览** | ✅ 专用视图 | ⚠️ 通用 FileTree | memU 更直观 |
| **分类管理** | ✅ 自动分类可视化 | ❌ 无 | evif 需新增 |
| **Token 监控** | ✅ LLM 使用可视化 | ⚠️ 通用监控 | memU 更针对 |
| **用户权限** | ✅ RBAC (Developer/Admin/User) | ⚠️ 协作权限 | 各有侧重 |
| **API 集成** | ✅ memU-server REST | ✅ evif-rest | 相当 |
| **文件系统** | ⚠️ 文件隐喻 | ✅ 真实 FUSE | evif 独有优势 |

### 2.3 技术栈（推测）

| 层级 | 技术选型 | 说明 |
|------|---------|------|
| **框架** | Next.js (推测) | SSR/SSG 支持 |
| **部署** | Docker | 一键部署 |
| **后端** | memU-server | RESTful API |
| **存储** | PostgreSQL + pgvector | 向量检索 |

---

## 3. Everything is File UI 设计哲学

### 3.1 核心原则

```
┌─────────────────────────────────────────────────────────────────────┐
│                   Everything is File UI 设计原则                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 文件即界面                                                       │
│     - 所有记忆以 MD 文件形式呈现                                      │
│     - 用户可以直接编辑文件来修改记忆                                   │
│     - Git 版本控制自然集成                                            │
│                                                                     │
│  2. 目录即分类                                                       │
│     - 记忆分类 = 目录结构                                             │
│     - 拖拽移动 = 重新分类                                             │
│     - 符号链接 = 交叉引用                                             │
│                                                                     │
│  3. 编辑器即工具                                                     │
│     - 使用熟悉的编辑器操作记忆                                        │
│     - 快捷键、搜索、替换自然工作                                       │
│     - AI 辅助编辑集成                                                │
│                                                                     │
│  4. 命令行即 API                                                     │
│     - 所有 UI 操作可通过 CLI 完成                                     │
│     - 脚本自动化友好                                                  │
│     - 管道组合能力                                                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 文件系统映射 UI

| 记忆概念 | 文件系统 | UI 呈现 | 交互方式 |
|---------|---------|---------|---------|
| **Memory Item** | `.md` 文件 | 编辑器标签页 | 编辑/保存 |
| **Category** | 目录 | FileTree 节点 | 展开/折叠/创建 |
| **Resource** | 原始文件 | 预览面板 | 查看/挂载 |
| **References** | 符号链接 | 虚线图标 | 跳转/追踪 |
| **Metadata** | YAML Frontmatter | 侧边属性面板 | 编辑/搜索 |
| **Tags** | frontmatter 数组 | 标签云 | 点击过滤 |
| **Timeline** | 文件时间戳 | 时间线视图 | 滚动浏览 |
| **Graph** | 目录关系 | 图谱视图 | 拖拽/缩放 |

### 3.3 独特 UI 机会

**evif-mem 独有的 UI 能力**（基于 FUSE + evif-graph）:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    evif-mem 独有 UI 优势                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. FUSE 挂载状态实时可视化                                          │
│     - 挂载点状态指示器                                               │
│     - 存储后端连接状态                                               │
│     - 读写操作实时反馈                                               │
│                                                                     │
│  2. 知识图谱交互式可视化                                             │
│     - 节点：Memory Item / Category / Resource                       │
│     - 边：References / Temporal / Causal                            │
│     - 力导向图布局                                                   │
│     - 点击节点 → 打开对应 MD 文件                                    │
│                                                                     │
│  3. 时序图谱可视化                                                   │
│     - 时间线：记忆演化历史                                            │
│     - 因果链：Before/After/Causes 关系                              │
│     - 事件流：记忆创建/更新/合并事件                                  │
│                                                                     │
│  4. LLM 直接读取 MD 文件                                             │
│     - 编辑器内嵌 AI 助手                                             │
│     - 选中文字 → AI 分析/总结                                        │
│     - 自然语言搜索 → 文件定位                                        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 4. UI 组件演进规划

### 4.1 Phase 1: 记忆专用 UI 基础 (Q2 2026)

**目标**: 补齐记忆管理核心 UI 能力

#### 4.1.1 MemoryExplorer 组件

```
MemoryExplorer/
├── index.tsx              # 主组件
├── MemoryTree.tsx         # 记忆树形视图
├── CategoryNode.tsx       # 分类节点
├── MemoryNode.tsx         # 记忆项节点
├── types.ts               # 类型定义
└── hooks/
    ├── useMemoryTree.ts   # 树形数据管理
    └── useMemorySearch.ts # 搜索过滤
```

**功能**:
- 三层树形结构：Category → Memory Type → Memory Item
- 拖拽重新分类
- 右键菜单（查看/编辑/删除/引用）
- 搜索过滤（语义 + 关键字）
- 实时同步 FUSE 文件系统

**API 集成**:
```typescript
GET /api/v1/memories/categories        // 获取分类树
GET /api/v1/memories?category=work     // 获取分类下记忆
GET /api/v1/memories/{id}              // 获取记忆详情
```

#### 4.1.2 CategoryView 组件

```
CategoryView/
├── index.tsx              # 分类详情页
├── CategoryHeader.tsx     # 分类头部（名称/描述/统计）
├── CategoryStats.tsx      # 统计信息
├── MemoryList.tsx         # 记忆列表
├── CategoryTimeline.tsx   # 时间线
└── types.ts
```

**功能**:
- 分类概览：名称、描述、记忆数量、最后更新
- 统计图表：记忆类型分布、时间分布
- 记忆列表：支持排序、过滤
- 时间线：记忆演化历史

#### 4.1.3 MemoryTimeline 组件

```
MemoryTimeline/
├── index.tsx              # 时间线主组件
├── TimelineEvent.tsx      # 事件节点
├── TimelineFilter.tsx     # 过滤器
└── types.ts
```

**功能**:
- 垂直时间线布局
- 事件类型：创建/更新/合并/强化/衰减
- 时间范围选择
- 事件详情展开

### 4.2 Phase 2: 知识图谱可视化 (Q2-Q3 2026)

**目标**: 集成 evif-graph，提供交互式图谱视图

#### 4.2.1 KnowledgeGraph 组件

```
KnowledgeGraph/
├── index.tsx              # 图谱主组件
├── GraphCanvas.tsx        # Canvas 渲染层
├── GraphControls.tsx      # 控制面板（缩放/布局/过滤）
├── NodeDetails.tsx        # 节点详情面板
├── layouts/
│   ├── forceDirected.ts   # 力导向布局
│   ├── hierarchical.ts    # 层次布局
│   └── temporal.ts        # 时序布局
├── types.ts
└── hooks/
    ├── useGraphData.ts    # 图谱数据获取
    └── useGraphLayout.ts  # 布局算法
```

**技术选型**:
- 渲染：D3.js 或 React Flow
- 布局：d3-force / elk.js
- 交互：拖拽、缩放、点击、悬停

**功能**:
- 节点类型：MemoryItem / Category / Resource / Event
- 边类型：References / Before / After / Causes / SimilarTo
- 布局切换：力导向 / 层次 / 时序
- 节点过滤：按类型、时间范围
- 点击节点 → 打开对应文件
- 路径查询：A → B 最短路径

**API 集成**:
```typescript
GET /api/v1/graph/nodes                 // 获取所有节点
GET /api/v1/graph/edges                 // 获取所有边
POST /api/v1/graph/query                // 图查询
  - causal_chain: 因果链查询
  - timeline: 时间线查询
  - temporal_bfs: 时序 BFS
  - temporal_path: 时序路径
```

#### 4.2.2 TemporalGraphView 组件

```
TemporalGraphView/
├── index.tsx              # 时序图谱视图
├── TimeSlider.tsx         # 时间滑块
├── EventStream.tsx        # 事件流
├── CausalChain.tsx        # 因果链可视化
└── types.ts
```

**功能**:
- 时间滑块：查看特定时间点的图谱状态
- 事件流：记忆演化动画
- 因果链：高亮因果关系路径
- 回放：历史变化回放

### 4.3 Phase 3: FUSE 状态可视化 (Q3 2026)

**目标**: 实时展示 FUSE 挂载状态和操作

#### 4.3.1 FUSEStatusPanel 组件

```
FUSEStatusPanel/
├── index.tsx              # FUSE 状态面板
├── MountPointStatus.tsx   # 挂载点状态
├── StorageBackend.tsx     # 存储后端状态
├── OperationLog.tsx       # 操作日志
├── PerformanceMetrics.tsx # 性能指标
└── types.ts
```

**功能**:
- 挂载点状态：已挂载/未挂载/错误
- 存储后端连接：SQLite/PostgreSQL/S3/...
- 实时操作日志：read/write/delete 事件
- 性能指标：IOPS、延迟、吞吐量
- 快速操作：挂载/卸载/重新挂载

**API 集成**:
```typescript
GET /api/v1/fuse/status           // 获取挂载状态
POST /api/v1/fuse/mount           // 挂载
POST /api/v1/fuse/unmount         // 卸载
GET /api/v1/fuse/operations       // 操作日志
GET /api/v1/fuse/metrics          // 性能指标
```

### 4.4 Phase 4: AI 辅助功能 (Q3-Q4 2026)

**目标**: AI 记忆助手和分析洞察

#### 4.4.1 AIChatPanel 组件

```
AIChatPanel/
├── index.tsx              # AI 对话面板
├── ChatMessage.tsx        # 消息组件
├── ChatInput.tsx          # 输入框
├── SuggestedActions.tsx   # 建议操作
├── ContextPreview.tsx     # 上下文预览
└── types.ts
```

**功能**:
- 自然语言查询记忆
- AI 辅助记忆整理
- 上下文预览：当前对话相关的记忆
- 建议操作：基于意图的主动建议

#### 4.4.2 MemoryInsights 组件

```
MemoryInsights/
├── index.tsx              # 洞察仪表板
├── UsageStats.tsx         # 使用统计
├── TrendAnalysis.tsx      # 趋势分析
├── MemoryHealth.tsx       # 记忆健康度
├── Recommendations.tsx    # 优化建议
└── types.ts
```

**功能**:
- 使用统计：查询频率、热点记忆
- 趋势分析：记忆增长、分类分布变化
- 记忆健康度：陈旧度、冗余度、覆盖率
- 优化建议：合并建议、清理建议

---

## 5. 架构图

### 5.1 UI 组件架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         evif-web UI Layer                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐    │
│  │  Layout Layer   │  │  Memory Layer   │  │  Graph Layer    │    │
│  │                 │  │                 │  │                 │    │
│  │  MenuBar        │  │  MemoryExplorer │  │  KnowledgeGraph │    │
│  │  ActivityBar    │  │  CategoryView   │  │  TemporalView   │    │
│  │  StatusBar      │  │  MemoryTimeline │  │  CausalChain    │    │
│  │  Panel          │  │  MemorySearch   │  │  GraphControls  │    │
│  │                 │  │                 │  │                 │    │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘    │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐    │
│  │  FUSE Layer     │  │  AI Layer       │  │  Monitor Layer  │    │
│  │                 │  │                 │  │                 │    │
│  │  FUSEStatus     │  │  AIChatPanel    │  │  TrafficChart   │    │
│  │  OperationLog   │  │  MemoryInsights │  │  OperationChart │    │
│  │  StorageStatus  │  │  ContextPreview │  │  SystemStatus   │    │
│  │                 │  │                 │  │  AlertPanel     │    │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘    │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│                         API Integration Layer                       │
│                                                                     │
│  REST API: /api/v1/fs/*  /api/v1/memories/*  /api/v1/graph/*       │
│  WebSocket: /api/v1/terminal  /api/v1/fuse/events                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 5.2 数据流架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         User Interaction                            │
│                                                                     │
│  [Browse] [Edit] [Search] [Query] [Visualize] [Chat]               │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         UI Components                               │
│                                                                     │
│  MemoryExplorer  CategoryView  KnowledgeGraph  AIChatPanel         │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         State Management                            │
│                                                                     │
│  React Query (Server State)  +  Zustand (Client State)             │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         API Layer                                   │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │  evif-rest   │  │  evif-fuse   │  │  evif-graph  │             │
│  │  (REST API)  │  │  (FUSE Ops)  │  │  (Graph API) │             │
│  └──────────────┘  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Core Engine                                 │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │  evif-mem    │  │  evif-core   │  │  evif-graph  │             │
│  │  (Memory)    │  │  (Plugins)   │  │  (Graph)     │             │
│  └──────────────┘  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Storage Layer                               │
│                                                                     │
│  SQLite │ PostgreSQL │ Memory │ S3 │ FUSE Mount                   │
└─────────────────────────────────────────────────────────────────────┘
```

### 5.3 FUSE 集成架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         evif-web UI                                 │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    FUSEStatusPanel                          │   │
│  │                                                             │   │
│  │  Mount Point: /mnt/evif-mem                                │   │
│  │  Status: ● Mounted                                          │   │
│  │  Backend: sqlite:///memories.db                            │   │
│  │  Operations: 1,234 reads, 567 writes                       │   │
│  │  Latency: p50=2ms, p95=8ms, p99=15ms                       │   │
│  │                                                             │   │
│  │  [Unmount] [Remount] [View Logs] [Performance]             │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│                              ▼                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    FileTree (FUSE-aware)                    │   │
│  │                                                             │   │
│  │  📁 /mnt/evif-mem                                          │   │
│  │  ├── 📁 categories/                                        │   │
│  │  │   ├── 📁 work/                                          │   │
│  │  │   │   └── 📄 projects.md (FUSE → Memory Category)       │   │
│  │  │   └── 📁 personal/                                      │   │
│  │  ├── 📁 memories/                                          │   │
│  │  │   ├── 📁 profile/                                       │   │
│  │  │   │   └── 📄 api-preferences.md (FUSE → Memory Item)    │   │
│  │  │   └── 📁 skills/                                        │   │
│  │  └── 📁 resources/                                         │   │
│  │      └── 📄 conversation-2026-03-09.md                     │   │
│  │                                                             │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│                              ▼                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Editor (MD with Frontmatter)             │   │
│  │                                                             │   │
│  │  ---                                                       │   │
│  │  id: 550e8400-e29b-41d4-a716-446655440000                  │   │
│  │  memory_type: profile                                      │   │
│  │  created: 2026-03-09T10:30:00Z                             │   │
│  │  tags: [api, rest, preferences]                            │   │
│  │  weight: 0.85                                              │   │
│  │  reinforcement_count: 3                                    │   │
│  │  ---                                                       │   │
│  │  User prefers RESTful APIs over GraphQL for simplicity.    │   │
│  │  This preference was observed during multiple API          │   │
│  │  design discussions.                                       │   │
│  │                                                             │   │
│  │  [Save] → FUSE Write → evif-mem Pipeline                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         FUSE Layer                                  │
│                                                                     │
│  evif-mount (FUSE daemon) ←→ evif-mem (MemPlugin)                  │
│                                                                     │
│  read() → fetch from storage    write() → trigger pipeline         │
│  readdir() → list categories    create() → create memory           │
│  unlink() → delete memory       symlink() → create reference       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 6. 实施路线图

### 6.1 Phase 1: 记忆专用 UI (Q2 2026) - P0

| 任务 | 优先级 | 预估工时 | 依赖 |
|------|--------|---------|------|
| MemoryExplorer 组件 | P0 | 5 天 | evif-rest memories API |
| CategoryView 组件 | P0 | 3 天 | MemoryExplorer |
| MemoryTimeline 组件 | P1 | 3 天 | evif-rest timeline API |
| 记忆搜索增强 | P1 | 2 天 | evif-mem search API |
| 单元测试 | P0 | 2 天 | 所有组件 |
| Playwright E2E 测试 | P1 | 2 天 | 所有组件 |

**总计**: ~17 天

### 6.2 Phase 2: 知识图谱可视化 (Q2-Q3 2026) - P1

| 任务 | 优先级 | 预估工时 | 依赖 |
|------|--------|---------|------|
| KnowledgeGraph 组件 | P1 | 7 天 | evif-graph API |
| 图谱布局算法 | P1 | 3 天 | D3.js / elk.js |
| TemporalGraphView 组件 | P1 | 4 天 | KnowledgeGraph |
| 图查询 UI | P2 | 3 天 | evif-graph query API |
| 性能优化（大图谱） | P2 | 2 天 | KnowledgeGraph |
| 单元测试 | P1 | 2 天 | 所有组件 |
| E2E 测试 | P2 | 1 天 | 所有组件 |

**总计**: ~22 天

### 6.3 Phase 3: FUSE 状态可视化 (Q3 2026) - P1

| 任务 | 优先级 | 预估工时 | 依赖 |
|------|--------|---------|------|
| FUSEStatusPanel 组件 | P1 | 3 天 | evif-fuse status API |
| OperationLog 组件 | P1 | 2 天 | WebSocket events |
| StorageBackend 状态 | P2 | 2 天 | evif-core storage API |
| 性能指标可视化 | P2 | 2 天 | Prometheus metrics |
| 单元测试 | P1 | 1 天 | 所有组件 |

**总计**: ~10 天

### 6.4 Phase 4: AI 辅助功能 (Q3-Q4 2026) - P2

| 任务 | 优先级 | 预估工时 | 依赖 |
|------|--------|---------|------|
| AIChatPanel 组件 | P2 | 5 天 | evif-mem LLM API |
| MemoryInsights 组件 | P2 | 4 天 | evif-mem analytics API |
| 上下文预览 | P2 | 2 天 | evif-mem retrieve API |
| 建议系统 | P3 | 3 天 | evif-mem proactive API |
| 单元测试 | P2 | 2 天 | 所有组件 |

**总计**: ~16 天

### 6.5 总体时间线

```
2026 Q2 (4-6月)
├── Phase 1: 记忆专用 UI (17 天)
│   ├── Week 1-2: MemoryExplorer + CategoryView
│   └── Week 3: MemoryTimeline + 搜索增强 + 测试
│
└── Phase 2: 知识图谱可视化 (22 天)
    ├── Week 4-5: KnowledgeGraph + 布局算法
    └── Week 6: TemporalGraphView + 图查询 + 测试

2026 Q3 (7-9月)
├── Phase 3: FUSE 状态可视化 (10 天)
│   └── Week 7-8: FUSEStatusPanel + OperationLog + 测试
│
└── Phase 4: AI 辅助功能 (16 天)
    ├── Week 9-10: AIChatPanel + MemoryInsights
    └── Week 11: 上下文预览 + 建议系统 + 测试

2026 Q4 (10-12月)
├── 集成测试与性能优化
├── 文档完善
└── 发布准备
```

---

## 7. 测试策略

### 7.1 单元测试

```bash
# 组件测试
npm run test -- --coverage

# 关键测试用例
- MemoryExplorer: 树形数据渲染、拖拽、搜索过滤
- CategoryView: 统计计算、时间线渲染
- KnowledgeGraph: 节点/边渲染、布局算法、交互
- FUSEStatusPanel: 状态更新、操作日志
```

### 7.2 E2E 测试 (Playwright)

```typescript
// tests/memory-ui.spec.ts
test('MemoryExplorer 应该正确渲染记忆树', async ({ page }) => {
  await page.goto('/memories');
  await expect(page.locator('[data-testid="memory-tree"]')).toBeVisible();

  // 展开分类
  await page.click('[data-testid="category-work"]');
  await expect(page.locator('[data-testid="memory-item"]')).toHaveCount(5);
});

test('KnowledgeGraph 应该支持节点点击导航', async ({ page }) => {
  await page.goto('/graph');
  await page.click('[data-testid="node-memory-1"]');
  await expect(page.locator('[data-testid="editor-tab"]')).toContainText('memory-1.md');
});
```

### 7.3 Playwright MCP 验证

```bash
# 使用 Playwright MCP 进行 UI 验证
1. 启动 evif-web 开发服务器
2. 通过 MCP 工具执行 UI 交互测试
3. 验证 FUSE 挂载状态可视化
4. 验证记忆文件编辑保存流程
5. 验证知识图谱交互
```

---

## 8. 性能目标

| 指标 | 目标值 | 测量方法 |
|------|--------|---------|
| **首次内容绘制 (FCP)** | < 1.5s | Lighthouse |
| **最大内容绘制 (LCP)** | < 2.5s | Lighthouse |
| **首次输入延迟 (FID)** | < 100ms | Lighthouse |
| **累积布局偏移 (CLS)** | < 0.1 | Lighthouse |
| **MemoryExplorer 渲染 (1000 节点)** | < 200ms | Performance API |
| **KnowledgeGraph 渲染 (500 节点)** | < 500ms | Performance API |
| **搜索响应时间** | < 100ms | evif-mem API |

---

## 9. 总结

### 9.1 关键差距

1. **记忆专用 UI 不足**: evif-web 是通用 IDE，缺乏记忆管理专用界面
2. **图谱可视化缺失**: evif-graph 独有优势未在 UI 展现
3. **FUSE 状态不可见**: 用户无法直观了解挂载状态

### 9.2 核心机会

1. **Everything is File 独特优势**: FUSE + MD 格式 + 编辑器集成
2. **evif-graph 集成**: 时序图谱可视化是竞争差异点
3. **Rust 性能**: 前端可利用后端 10x 性能优势

### 9.3 下一步行动

1. **立即开始 (P0)**: MemoryExplorer 组件开发
2. **同步进行**: evif-rest API 扩展（memories/graph 端点）
3. **准备中**: Playwright MCP 测试环境搭建
4. **规划中**: 知识图谱可视化技术选型

---

**文档版本**: 1.0.0
**最后更新**: 2026-03-09
**下次评审**: Phase 1 完成后 (预计 2026-06)
