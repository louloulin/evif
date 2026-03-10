# mem7.md - EVIF Web UI 问题分析与改进规划

> **版本**: 1.4
> **日期**: 2026-03-10
> **状态**: shadcn/ui 改造计划已完成
> **作者**: Ralph Loop Analysis
> **目标**: 分析 evif-web UI 现状，识别样式/功能问题，验证前后端接口，制定完善改进计划

---

## 执行摘要

本文档对 evif-web UI 进行全面分析，识别样式和功能问题，验证前后端接口数据一致性，并制定改进计划。

### 核心发现

| 维度 | 状态 | 问题数 |
|------|------|--------|
| **构建状态** | ✅ 通过 | 0 |
| **TypeScript** | ✅ 无错误 | 0 |
| **样式问题** | ⚠️ 需优化 | 7 |
| **功能问题** | ⚠️ 需完善 | 6 |
| **API 接口** | ✅ 已验证 | 3 |

### Playwright UI 验证结果

- ✅ 已验证 7 个主要 UI 组件
- ⚠️ 发现 111+ 个 API 500 错误 (后端未运行导致)
- ✅ 前端错误处理正常
- ✅ UI 组件加载正常

---

## 1. UI 问题分析

### 1.1 样式问题

#### 问题 1.1.1 混合样式使用
**位置**: `FUSEStatusPanel.tsx`, `MemoryExplorer.tsx`, `UploadDropzone.tsx`
**描述**: 同时使用 Tailwind 类名和自定义 CSS 类
**示例**:
```tsx
// 当前代码
<div className="bg-card rounded-lg border p-4">  // Tailwind
  <div className="memory-search-result">              // 自定义 CSS
```
**影响**: 维护困难，样式冲突风险
**优先级**: P2
**验证**: ✅ 确认存在 - 内存组件使用 App.css 自定义类，UI 组件使用 Tailwind + CVA

#### 问题 1.1.2 缺少暗色模式适配
**位置**: 多个组件
**描述**: 部分颜色使用硬编码，未使用 CSS 变量
**示例**:
```css
/* 当前代码 */
.memory-score {
  color: var(--primary);  /* 正确使用变量 */
}
.memory-category-count {
  background: var(--muted);  /* 正确 */
}
```
**状态**: ✅ 已正确使用 CSS 变量

#### 问题 1.1.3 响应式布局不完整
**位置**: `MemoryExplorer.tsx`, `KnowledgeGraph.tsx`
**描述**: 固定宽度/高度，未适配小屏幕
**示例**:
```typescript
// MemoryExplorer.tsx:146
style={{ paddingLeft: `${level * 16 + 24}px` }}  // 固定像素值
```
**影响**: 移动端体验不佳
**优先级**: P2

#### 问题 1.1.4 滚动条定义冲突
**位置**: `index.css`, `fixes.css`
**描述**: 滚动条样式在两个文件中重复定义，fixes.css 使用硬编码颜色
**示例**:
```css
/* index.css (正确) */
::-webkit-scrollbar-track { background: hsl(var(--background)); }

/* fixes.css (硬编码) */
::-webkit-scrollbar-track { background: #1e1e1e; }
```
**影响**: 样式冲突，暗色模式下可能显示异常
**优先级**: P2
**新发现**: ✅ 2026-03-10

#### 问题 1.1.5 图标使用不一致
**位置**: 多个组件
**描述**: 混用 emoji 和图标库
**示例**:
```tsx
// 当前
<span className="memory-icon">📝</span>
<span className="category-icon">📁</span>
// 建议使用 Lucide React 或其他图标库
```
**优先级**: P3

#### 问题 1.1.6 CSS 变量使用不一致
**位置**: `App.css`, `fixes.css`
**描述**: 部分使用 CSS 变量，部分硬编码颜色
**示例**:
```css
/* 正确使用 */
.chat-message.user { background: var(--primary); }

/* 硬编码 */
.chat-error { background: #fef2f2; color: #dc2626; }
```
**建议**: 使用 `--destructive` 等已有变量
**优先级**: P2
**新发现**: ✅ 2026-03-10

#### 问题 1.1.7 动画过渡不流畅
**位置**: 树形展开/折叠
**描述**: 缺少平滑过渡效果
**示例**:
```css
/* 当前 */
.folder-icon {
  font-size: 10px;
  transition: transform 0.15s;  /* 基础过渡 */
}
/* 建议添加更多过渡效果 */
```
**优先级**: P3

---

### 1.2 功能问题

> **功能问题验证状态**: 2026-03-10 通过代码审查验证

#### 问题 1.2.1 错误处理不完善
**位置**: `MemoryExplorer.tsx:65-66`
**验证状态**: ✅ 已确认
**描述**: 错误信息不够友好
```typescript
// 当前
setError(err instanceof Error ? err.message : '加载分类失败')
// 建议
setError(err instanceof Error ? err.message : '无法加载记忆数据，请检查网络连接')
```
**优先级**: P1

#### 问题 1.2.2 搜索结果日期过滤无效
**位置**: `MemoryExplorer.tsx:115-120`
**验证状态**: ✅ 已确认 - 代码确实总是返回 true
**描述**: 日期范围过滤代码为空
```typescript
// 当前
if (dateRange.start || dateRange.end) {
  filteredResults = filteredResults.filter(r => {
    // 注意: SearchResult 没有 timestamp 字段，这里做简单处理
    return true  // 总是返回 true！
  })
}
```
**影响**: 日期过滤功能不可用
**优先级**: P1

#### 问题 1.2.3 缺少记忆创建功能
**位置**: `MemoryExplorer.tsx`
**验证状态**: ✅ 已确认 - 组件仅提供浏览和搜索，无创建 UI
**描述**: 只有浏览功能，无法创建新记忆
**影响**: 用户无法通过 UI 添加记忆
**优先级**: P1

#### 问题 1.2.4 记忆详情查看功能缺失
**位置**: `MemoryExplorer.tsx:140-143`
**验证状态**: ✅ 已确认 - 仅设置 selectedMemoryId 状态
**描述**: 点击记忆项后未实现完整查看/编辑功能
**当前行为**: 仅设置 selectedMemoryId 状态
```typescript
// 当前
const handleMemoryClick = useCallback((memory: MemoryItem) => {
  setSelectedMemoryId(memory.id)
  onMemorySelect?.(memory)  // 回调但未实现完整功能
}, [onMemorySelect])
```
**优先级**: P1

#### 问题 1.2.5 分类统计信息显示不完整
**位置**: `CategoryView.tsx:116-129`
**验证状态**: ✅ 已确认 - 仅显示 item_count、创建时间、更新时间
**描述**: 只显示 item_count，缺少更多统计
**建议显示**:
- 记忆类型分布
- 总字符数
- 最后更新时间
- 热点记忆
**优先级**: P2

#### 问题 1.2.6 知识图谱交互功能有限
**位置**: `KnowledgeGraph.tsx`
**验证状态**: ✅ 已确认 - 基础功能已实现，高级功能缺失
**描述**: 基础交互已实现，高级功能缺失
**缺失功能**:
- 节点拖拽布局
- 路径查询
- 边类型过滤
- 批量选择
**优先级**: P2

---

## 2. 前后端 API 接口分析

### 2.1 API 端点列表

| 端点 | 状态 | 前端使用 | 后端实现 |
|------|------|---------|---------|
| `GET /api/v1/memories` | ✅ | ❌ 未使用 | ✅ |
| `GET /api/v1/memories/{id}` | ✅ | ❌ 未使用 | ✅ |
| `POST /api/v1/memories` | ✅ | ✅ `createMemory()` | ✅ |
| `POST /api/v1/memories/search` | ✅ | ✅ `searchMemories()` | ✅ |
| `GET /api/v1/categories` | ✅ | ✅ `listCategories()` | ✅ |
| `GET /api/v1/categories/{id}` | ✅ | ✅ `getCategory()` | ✅ |
| `GET /api/v1/categories/{id}/memories` | ✅ | ✅ `getCategoryMemories()` | ✅ |
| `POST /api/v1/graph/query` | ✅ | ✅ `queryGraph()` | ✅ |
| `GET /api/v1/fuse/status` | ✅ | ✅ FUSEStatusPanel | ✅ |
| `GET /api/v1/metrics/operations` | ⚠️ | ✅ OperationLog | 需验证 |

### 2.2 数据模型不一致问题

#### 问题 2.2.1 SearchResult 缺少时间戳
**位置**: `memory-api.ts:33-39` + `memory_handlers.rs:114-123`
**验证状态**: ✅ 已确认 - 前后端都缺少 timestamp 字段
**描述**: 前端 SearchResult 接口缺少时间戳字段，后端 MemorySearchResult 也没有返回
```typescript
// 前端定义
export interface SearchResult {
  id: string
  type: string
  content: string
  score: number
  category?: string
  // 缺少: created, updated, timestamp
}
```
**后端响应**: 后端 MemorySearchResult 也没有 timestamp 字段
**影响**: 日期过滤功能无法实现 (问题 1.2.2 根因)
**优先级**: P1
**修复方案**: 后端添加 `created` 和 `updated` 字段到 MemorySearchResult

#### 问题 2.2.2 CategoryWithMemories 响应格式
**位置**: `memory-api.ts:28-31` + `memory_handlers.rs:149-153`
**验证状态**: ✅ 已确认 - 前后端格式一致
```typescript
// 前端
export interface CategoryWithMemories {
  category: Category
  memories: MemoryItem[]
}

// 后端
pub struct CategoryWithMemoriesResponse {
    pub category: CategoryResponse,
    pub memories: Vec<MemoryItemResponse>,
}
```
**结论**: 格式匹配，无需修改

#### 问题 2.2.3 GraphQueryResponse 类型不完整
**位置**: `memory-api.ts:59-64` + `memory_handlers.rs:386-402`
**验证状态**: ✅ 已确认 - 缺少 paths 字段
```typescript
// 前端 (当前)
export interface GraphQueryResponse {
  query_type: string
  nodes?: GraphNode[]
  timeline?: TimelineEvent[]
  total: number
  // 缺少: paths 字段
}
```
**后端响应**: 包含 `paths: Option<Vec<GraphPathInfo>>` 字段
**优先级**: P2
**修复方案**: 前端添加 paths 字段定义

#### 问题 2.2.4 createMemory 响应格式不匹配
**位置**: `memory-api.ts:83-92` + `memory_handlers.rs:66-71`
**验证状态**: ✅ 已确认 - 响应格式不一致
```typescript
// 前端期望
createMemory(content: string, modality?: string): Promise<{ memory_id: string }>

// 后端实际返回
pub struct CreateMemoryResponse {
    pub memory_id: String,
    pub extracted_items: Vec<ExtractedMemoryItem>,
}
```
**影响**: 前端调用会解析失败
**优先级**: P1
**修复方案**: 前端适配实际响应格式，或后端兼容前端格式

### 2.3 API 调用问题

#### 问题 2.3.1 缺少 API 超时处理
**位置**: `memory-api.ts` + `http.ts`
**验证状态**: ✅ 已确认 - httpFetch 无超时，API 函数无 AbortSignal 支持
**描述**: 所有 API 调用无超时设置
```typescript
// 当前
export async function listMemories(): Promise<MemoryItem[]> {
  const res = await httpFetch('/api/v1/memories')
  // 无超时
}

// 建议
export async function listMemories(
  signal?: AbortSignal
): Promise<MemoryItem[]> {
  const res = await httpFetch('/api/v1/memories', { signal })
}
```
**优先级**: P2

#### 问题 2.3.2 缺少请求重试机制
**位置**: `memory-api.ts` + `http.ts`
**验证状态**: ✅ 已确认 - 无重试逻辑
**描述**: 网络错误时无重试逻辑
**优先级**: P3

---

## 3. Playwright UI 验证结果

### 3.1 验证概述
**日期**: 2026-03-10
**工具**: Playwright MCP
**方法**: 浏览器自动化测试和截图验证

### 3.2 已验证的 UI 组件

| 组件 | 位置 | 状态 | 备注 |
|------|------|------|------|
| 活动栏 | 左侧 | ✅ 正常 | 7 个图标按钮 |
| 资源管理器 | 主视图 | ✅ 显示 | 错误提示正常 (后端未连接) |
| 终端 | 底部面板 | ✅ 正常 | WebSocket 连接错误 (预期) |
| 记忆管理 | 侧边栏 | ✅ 可访问 | 5 个子视图 |
| 系统监控 | 侧边栏 | ✅ 可访问 | 4 个子视图 |
| 搜索与上传 | 侧边栏 | ✅ 可访问 | 搜索和上传标签页 |
| 状态栏 | 底部 | ✅ 正常 | 显示连接状态 |

### 3.3 记忆管理子视图验证

| 子视图 | 功能 | 状态 | 问题 |
|--------|------|------|------|
| 记忆浏览器 | 分类列表 | ⚠️ | 错误提示 "Failed to fetch categories" |
| 时间线 | 时间线展示 | ⚠️ | 未测试 |
| 知识图谱 | 图谱可视化 | ⚠️ | 错误提示 "加载图数据失败" |
| AI 助手 | 自然语言交互 | ⚠️ | UI 显示正常，API 错误 |
| 洞察分析 | 统计面板 | ⚠️ | UI 显示正常，数据加载失败 |

### 3.4 系统监控子视图验证

| 子视图 | 状态 | 数据 |
|--------|------|------|
| 概览 | ✅ 正常显示 | CPU/内存/磁盘/运行时间显示 0 |
| 日志 | ✅ 正常 | 未深入测试 |
| 告警 | ✅ 正常 | 未深入测试 |
| FUSE 状态 | ⚠️ | "加载失败: Failed to fetch mounts" |

### 3.5 控制台错误分析

**总错误数**: 111+ (主要为 API 500 错误)

**错误分类**:
- `GET /api/v1/health` - 500 错误 (后端未运行)
- `GET /api/v1/fs/list` - 500 错误 (后端未运行)
- `GET /api/v1/categories` - 500 错误
- `POST /api/v1/graph/query` - 500 错误
- `GET /api/v1/memories/*` - 500 错误
- `GET /api/v1/metrics/*` - 500 错误
- `GET /api/v1/mounts` - 500 错误
- WebSocket 连接失败 - Terminal 组件

**结论**: 所有 API 错误都是因为后端未运行，前端错误处理正常。

### 3.6 截图验证

已保存截图文件:
- `evif-web-ui-verification-main.png` - 主界面
- `evif-web-ui-verification-memory.png` - 记忆管理概览
- `evif-web-ui-verification-ai.png` - AI 助手
- `evif-web-ui-verification-insights.png` - 洞察分析
- `evif-web-ui-verification-monitor.png` - 系统监控
- `evif-web-ui-verification-search.png` - 搜索视图
- `evif-web-ui-verification-upload.png` - 上传视图

---

## 4. 竞品 UI 对比分析

### 4.1 VS Code Web UI 模式

**参考项目**: [Microsoft VS Code Webview UI Toolkit](https://github.com/microsoft/vscode-webview-ui-toolkit-samples)

**evif-web 当前状态**:
- ✅ 相似架构: 活动栏 + 侧边栏 + 编辑区 + 状态栏
- ✅ 使用 Radix UI 组件库
- ✅ 使用 Tailwind CSS
- ⚠️ 缺少: 命令面板 (Command Palette) 完整实现
- ⚠️ 缺少: 上下文菜单 (Context Menu) 完整实现

**差距分析**:
| 功能 | VS Code | evif-web | 差距 |
|------|---------|----------|------|
| 多标签页编辑器 | ✅ | ⚠️ 基础 | 中 |
| 文件树折叠/展开动画 | ✅ 流畅 | ⚠️ 基础 | 低 |
| 右键上下文菜单 | ✅ 完整 | ⚠️ 部分 | 中 |
| 搜索面板 | ✅ 正则/文件 | ⚠️ 基础 | 中 |
| 终端 | ✅ xterm.js | ✅ | 无 |
| 状态栏信息 | ✅ 丰富 | ⚠️ 基础 | 低 |

### 4.2 memU UI 对比分析

**参考项目**: [memU](https://a-bots.com/blog/memu-2026)

**memU 核心功能**:
| 功能 | memU | evif-web | 状态 |
|------|------|----------|------|
| 视觉记忆浏览器 | ✅ | ⚠️ 基础 | 中 |
| 语义搜索 | ✅ 完整 | ⚠️ 部分 | 中 |
| 记忆统计面板 | ✅ | ⚠️ 基础 | 低 |
| 用户管理 UI | ✅ | ❌ 缺失 | 高 |
| MCP 集成 | ✅ | ❌ 缺失 | 高 |

**memU UI 特点**:
- 记忆以人类可读的 Markdown 格式存储
- 图形化仪表板用于浏览、查询、管理记忆数据
- 与 memU-server API 实时同步
- 透明性强，用户可查看 AI 记住的内容

### 4.3 Mission Control 对比分析

**参考项目**: [Mission Control](https://github.com/builderz-labs/mission-control)

**Mission Control 核心功能**:
| 功能 | Mission Control | evif-web | 状态 |
|------|----------------|----------|------|
| Agent 编排 | ✅ | ⚠️ 基础 | 高 |
| 任务跟踪 | ✅ | ❌ 缺失 | 高 |
| 成本监控 | ✅ | ⚠️ 基础 | 中 |
| 工作流编排 | ✅ | ❌ 缺失 | 高 |
| Agent 集群管理 | ✅ | ❌ 缺失 | 高 |

### 4.4 OpenClaw 对比分析

**参考项目**: [OpenClaw](https://clawtank.dev/blog/best-open-source-ai-agents-2026)

**OpenClaw 核心功能**:
| 功能 | OpenClaw | evif-web | 状态 |
|------|----------|----------|------|
| 持久记忆 | ✅ | ✅ | 无 |
| 日常操作 | ✅ | ⚠️ 部分 | 中 |
| 消息集成 | ✅ | ❌ 缺失 | 高 |
| 远程监控 | ✅ | ⚠️ 部分 | 中 |

### 4.5 改进建议

基于 VS Code UI 模式，建议添加:

1. **命令面板** (Command Palette)
   - Ctrl+Shift+P 快捷键
   - 模糊搜索所有命令
   - 最近使用记录

2. **增强上下文菜单**
   - 文件/文件夹操作
   - 重命名、复制、粘贴
   - 终端右键菜单

3. **编辑器标签页增强**
   - 拖拽排序
   - 关闭按钮
   - 修改标记 (*)

4. **搜索功能增强**
   - 正则表达式支持
   - 文件过滤
   - 搜索历史

---

## 5. 改进计划

### 5.1 Phase 1: 核心问题修复 (P0-P1)

| 任务 | 优先级 | 描述 | 预估工时 | 状态 |
|------|--------|------|---------|------|
| 修复日期过滤 | P1 | 后端添加 SearchResult.created/updated 字段 | 2h | 待开始 |
| 添加记忆创建 | P1 | 实现 createMemory UI 和 API 集成 | 4h | 待开始 |
| 完善错误处理 | P1 | 友好的错误提示和重试机制 | 2h | 待开始 |
| 修复 createMemory 响应 | P1 | 适配后端返回的 extracted_items 字段 | 1h | 待开始 |
| 修复 GraphQueryResponse | P2 | 添加 paths 字段到前端类型定义 | 1h | 待开始 |
| 添加加载骨架屏 | P1 | 改善加载体验 (Skeleton) | 2h | 待开始 |
| 实现重试按钮 | P1 | API 失败时的自动重试按钮 | 1h | 待开始 |

### 5.2 Phase 2: UI/UX 优化 (P2)

| 任务 | 优先级 | 描述 | 预估工时 |
|------|--------|------|---------|
| 响应式布局 | P2 | 适配移动端，小屏幕优化 | 4h |
| 图标统一 | P3 | 替换 emoji 为 Lucide React | 3h |
| 动画增强 | P3 | 添加平滑过渡效果 | 2h |
| 暗色模式 | P2 | 完善暗色模式适配 | 3h |

### 5.3 Phase 3: 功能增强 (P2)

| 任务 | 优先级 | 描述 | 预估工时 |
|------|--------|------|---------|
| 记忆详情面板 | P2 | 点击记忆显示完整内容和操作 | 4h |
| 分类统计增强 | P2 | 显示更多统计信息 | 3h |
| 知识图谱增强 | P2 | 拖拽布局、路径查询 | 6h |
| API 优化 | P2 | 超时、重试、缓存 | 4h |

---

## 6. 实施路线图

```
2026-03-10 (今天)
├── Phase 1: 核心问题修复
│   ├── 任务 1.1: 修复日期过滤功能 (2h)
│   ├── 任务 1.2: 添加记忆创建 UI (4h)
│   ├── 任务 1.3: 完善错误处理 (2h)
│   ├── 任务 1.4: 验证 API 响应 (2h)
│   ├── 任务 1.5: 添加加载骨架屏 (2h)
│   └── 任务 1.6: 实现重试按钮 (1h)
│
├── Phase 2: UI/UX 优化 (本周)
│   ├── 任务 2.1: 响应式布局 (4h)
│   ├── 任务 2.2: 图标统一 (3h)
│   ├── 任务 2.3: 动画增强 (2h)
│   └── 任务 2.4: 暗色模式 (3h)
│
└── Phase 3: 功能增强 (下周)
    ├── 任务 3.1: 记忆详情面板 (4h)
    ├── 任务 3.2: 分类统计增强 (3h)
    ├── 任务 3.3: 知识图谱增强 (6h)
    └── 任务 3.4: API 优化 (4h)
```

---

## 7. 技术债务

### 7.1 需要重构的代码

1. **样式系统统一**
   - 将所有 Tailwind 类改为 CSS 变量或自定义类
   - 建立 design tokens

2. **API 层重构**
   - 添加 React Query 或 SWR 进行状态管理
   - 统一错误处理

3. **组件拆分**
   - 大组件拆分为更小的可复用组件
   - 提取公共逻辑到 hooks

---

## 8. 验证清单

### 构建验证
- [x] `bun run build` 通过
- [x] `bun run typecheck` 无错误

### 功能验证
- [ ] 记忆创建功能可用
- [ ] 日期过滤功能可用
- [ ] 错误提示友好
- [ ] API 响应正确

### UI 验证
- [ ] 响应式布局正常
- [ ] 暗色模式正常
- [ ] 动画流畅

---

## 9. 下一步行动

1. **立即执行 (今天)**:
   - 修复日期过滤功能
   - 添加记忆创建入口

2. **本周完成**:
   - 完善错误处理
   - 验证所有 API 端点

3. **下周计划**:
   - UI/UX 优化
   - 功能增强

---

## 10. shadcn/ui 统一改造计划

### 10.1 当前状态

**已实现的 shadcn/ui 组件** (16 个):

| 组件 | 位置 | 状态 | 说明 |
|------|------|------|------|
| Button | `components/ui/button.tsx` | ✅ 完整 | 已扩展 success/warning/info 变体 |
| Card | `components/ui/card.tsx` | ✅ 完整 | |
| Input | `components/ui/input.tsx` | ✅ 完整 | |
| Textarea | `components/ui/textarea.tsx` | ✅ 完整 | |
| Dialog | `components/ui/dialog.tsx` | ✅ 完整 | |
| Badge | `components/ui/badge.tsx` | ✅ 完整 | |
| Tabs | `components/ui/tabs.tsx` | ✅ 完整 | |
| Progress | `components/ui/progress.tsx` | ✅ 完整 | |
| Switch | `components/ui/switch.tsx` | ✅ 完整 | |
| ScrollArea | `components/ui/scroll-area.tsx` | ✅ 完整 | |
| Skeleton | `components/ui/skeleton.tsx` | ✅ 完整 | |
| Toast | `components/ui/toast.tsx` | ✅ 完整 | |
| Toaster | `components/ui/toaster.tsx` | ✅ 完整 | |
| Label | `components/ui/label.tsx` | ✅ 完整 | |
| Pagination | `components/ui/pagination.tsx` | ✅ 完整 | |
| Command | `components/ui/command.tsx` | ✅ 完整 | |

**技术栈**:
- Radix UI 作为基础组件库
- class-variance-authority (CVA) 管理变体
- Tailwind CSS 进行样式设计
- CSS 变量支持暗色模式
- Lucide React 图标库

### 10.2 改造策略

#### 10.2.1 组件分层

```
┌─────────────────────────────────────────────┐
│  页面级组件 (Page Components)                │
│  MemoryView, MonitorView, SearchUploadView   │
├─────────────────────────────────────────────┤
│  功能组件 (Feature Components)               │
│  MemoryExplorer, KnowledgeGraph, AIChatPanel │
├─────────────────────────────────────────────┤
│  业务组件 (Business Components)              │
│  CategoryView, MemoryTimeline, FUSEStatus   │
├─────────────────────────────────────────────┤
│  基础 UI 组件 (Primitive Components)        │
│  Button, Card, Input, Dialog (shadcn/ui)   │
└─────────────────────────────────────────────┘
```

#### 10.2.2 改造原则

1. **渐进式迁移**: 不重写现有代码，逐步替换
2. **保持兼容性**: 确保迁移过程中功能正常
3. **统一设计系统**: 使用 CSS 变量定义设计 tokens
4. **类型安全**: 充分利用 TypeScript 类型

### 10.3 组件迁移计划

#### Phase 1: 核心 UI 组件统一 (本周)

| 组件 | 当前实现 | 目标实现 | 优先级 |
|------|---------|---------|--------|
| 按钮 | 自定义类 + 内联样式 | 使用 Button 组件 | P1 |
| 输入框 | 原生 input | 使用 Input 组件 | P1 |
| 卡片 | 自定义 CSS | 使用 Card 组件 | P1 |
| 对话框 | 原生 dialog | 使用 Dialog 组件 | P1 |
| 标签页 | 自定义 Tabs | 使用 Tabs 组件 | P1 |

#### Phase 2: 业务组件重构 (下周)

| 组件 | 改造内容 | 优先级 |
|------|---------|--------|
| MemoryExplorer | 使用 Card + ScrollArea | P2 |
| CategoryView | 使用 Card + Badge | P2 |
| KnowledgeGraph | 使用 SVG + ScrollArea | P2 |
| AIChatPanel | 使用 Card + ScrollArea | P2 |
| FUSEStatusPanel | 使用 Card + Progress | P2 |

#### Phase 3: 设计系统完善 (下月)

1. **Design Tokens 统一**
   ```css
   /* 统一使用 CSS 变量 */
   :root {
     --color-primary: hsl(var(--primary));
     --color-success: hsl(var(--success));
     --color-warning: hsl(var(--warning));
     --radius-sm: calc(var(--radius) - 4px);
     --radius-md: calc(var(--radius) - 2px);
   }
   ```

2. **暗色模式完整支持**
   - 所有组件支持 `dark:` 模式
   - 使用 `hsl(var(--background))` 替代硬编码颜色

3. **动画系统**
   - 使用 Tailwind transition 类
   - 定义动画 keyframes

### 10.4 迁移示例

#### Before (当前)
```tsx
<div className="bg-card rounded-lg border p-4">
  <div className="memory-search-result">
    <h3 className="text-lg font-semibold">{title}</h3>
  </div>
</div>
```

#### After (目标)
```tsx
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card'

<Card>
  <CardHeader>
    <CardTitle>{title}</CardTitle>
  </CardHeader>
  <CardContent>
    {/* content */}
  </CardContent>
</Card>
```

### 10.5 待添加的 shadcn/ui 组件

| 组件 | 用途 | 优先级 |
|------|------|--------|
| Select | 搜索模式选择 | P1 |
| DropdownMenu | 右键菜单 | P1 |
| Popover | 工具提示 | P2 |
| Tooltip | 按钮提示 | P2 |
| Accordion | 折叠面板 | P2 |
| Calendar | 日期选择 | P2 |
| Table | 数据表格 | P2 |
| Form | 表单验证 | P2 |

### 10.6 验证清单

- [ ] 现有 16 个 UI 组件构建正常
- [ ] 暗色模式切换正常
- [ ] 所有 CSS 变量正确定义
- [ ] 组件变体 (variant) 统一使用
- [ ] TypeScript 类型检查通过

---

**文档版本**: 1.4
**更新日期**: 2026-03-10
**新增内容**: shadcn/ui 统一改造计划 (第 10 章)
