# EVIF 全功能分析报告

**日期**: 2026-01-31  
**范围**: evif-web 前端 + evif-rest 兼容 API (/api/v1/fs/*) + WebSocket 终端

---

## 一、核心功能概览

| 功能 | 前端 | 后端 | 状态 |
|------|------|------|------|
| 根目录列表 | GET /api/v1/fs/list?path=/ | compat_fs list | ✅ 正常 |
| 子目录列表 | fetchFiles(path) → list?path= | compat_fs list | ✅ 正常 |
| 读文件 | GET /api/v1/fs/read?path= | compat_fs read (0,0=全量) | ✅ 正常 |
| 写文件 | POST /api/v1/fs/write?path= + body | compat_fs write | ✅ 正常 |
| 创建文件 | POST /api/v1/fs/create + body.path | compat_fs create | ✅ 正常 |
| 删除 | DELETE /api/v1/fs/delete?path= | compat_fs delete | ✅ 已修复方法 |
| WebSocket 终端 | /ws → 8081 | ws_handlers | ✅ 正常 |

---

## 二、已发现并需关注的问题

### 2.1 严重 / 影响核心使用

#### 1. 编辑器无法正常输入（受控组件未同步）

- **位置**: `evif-web/src/App.tsx`，未向 `Editor` 传入 `onChange`
- **现象**: 使用 `value={content}` 受控 Monaco，但输入时未通过 `onChange` 回写 `fileContent`，导致每次渲染都用旧 content 覆盖，**输入内容会丢失或无法编辑**
- **修复**: 在 App 中增加 `handleEditorChange` 并传给 Editor：`onChange={handleEditorChange}`，在回调里 `setFileContent(value ?? '')`

#### 2. 新建文件无挂载时无用户可见反馈

- **位置**: `App.tsx` → `handleNewFile`
- **现象**: 无挂载时 `firstMount` 为 undefined，请求 `/api/v1/fs/create` 会 404，仅 `console.error`，界面无提示
- **建议**: 无挂载时禁用「New File」或先弹提示「请先配置挂载」；有挂载但创建失败时，用 problems 或 toast 展示后端错误信息

#### 3. 保存 / 删除失败无用户可见反馈

- **位置**: `handleFileSave`、`handleDelete`
- **现象**: 失败时仅 `console.error`，用户不知道是否保存成功或删除是否失败
- **建议**: 成功时可选简短提示（如 StatusBar 文案）；失败时在「问题」面板或 toast 中展示 `error.message` 或后端返回的 `message`

---

### 2.2 中等问题（体验与健壮性）

#### 4. API 错误未解析后端 message

- **位置**: `fetchFiles`、`handleFileSelect` 等
- **现象**: `if (!response.ok) throw new Error('Failed to ...')`，未读 body，后端返回的 `{ "message": "..." }` 未展示
- **建议**: 先 `const data = await response.json().catch(() => ({}));`，再 `throw new Error(data?.message || data?.error || 'Failed to ...')`

#### 5. 展开子目录无加载态

- **位置**: `FileTree` + App 的 `childrenMap` / `fetchFiles`
- **现象**: 展开某目录时先请求再写入 `childrenMap`，请求期间该目录下无任何占位，易被误认为空目录
- **建议**: 对「已展开但尚未收到子项」的 path 显示「加载中…」（需在 App 或 FileTree 增加 pending 状态，如 `loadingPaths: Set<string>`）

#### 6. 刷新只更新根列表，子目录缓存陈旧

- **位置**: `handleRefresh` → 仅 `fetchFiles()`（即 fetchFiles('/')）
- **现象**: 只更新 `files`，不清理或重新拉取已展开目录的 `childrenMap`，其他端写入的新文件可能看不到
- **建议**: 刷新时清空 `childrenMap` 或对当前已展开的 path 再请求一次

#### 7. 删除时先关右键菜单再等结果

- **位置**: `handleDelete` 末尾统一 `handleContextMenuClose()`
- **现象**: 无论成功失败都关菜单，若删除失败用户无法在菜单处重试
- **建议**: 仅在删除成功时关菜单；失败时保留菜单并展示错误（或 problems/toast）

---

### 2.3 后端与协议

#### 8. compat_fs 未校验 path 空或非法

- **位置**: `crates/evif-rest/src/compat_fs.rs`
- **现象**: 未显式校验 `path` 为空、`..` 等，依赖各 plugin 行为
- **建议**: 根路径仅允许 `"/"`；非根路径拒绝空、含 `..` 或明显非法格式，返回 400

#### 9. WebSocket 终端 cat 只读 4096 字节

- **位置**: `ws_handlers.rs` → `plugin.read(path, 0, 4096)`
- **现象**: 大文件只显示前 4KB
- **建议**: 与 compat_fs 一致，用 `read(path, 0, 0)` 表示全量读取（若 plugin 约定 size=0 为全部）；或分块读并流式输出

#### 10. 终端离线时本地 fallback 不完整

- **位置**: `Terminal.tsx`，WS 未连接时对 `ls` 等命令的本地处理
- **现象**: 如 `ls` 仅写「Fetching files...」未真正调 API
- **建议**: 要么移除本地 fallback 并统一提示「请连接后端」；要么对 ls/cat 等用 httpFetch 调 REST 并格式化输出

---

### 2.4 小问题与改进

#### 11. 问题面板仅展示 filesError

- **现象**: 仅把「列表错误」放进 problems，保存失败、删除失败、读文件失败等未进入
- **建议**: 将写/删/读等错误也推入 problems（或统一错误展示通道）

#### 12. StatusBar 的「连接」仅反映列表错误

- **现象**: `connected={!filesError}` 只表示根列表是否出错，不能代表 WebSocket 或其它 API
- **建议**: 可增加 WS 连接状态或更明确的「后端连接」含义

#### 13. 右键菜单 Rename 未实现

- **位置**: ContextMenu 中 Rename 为 `disabled: true`，`onClick: () => {}`
- **建议**: 后续若支持重命名，可调后端 rename 或 PATCH 并刷新列表

#### 14. 输出面板为静态占位

- **位置**: Panel 「输出」tab 仅显示固定文案
- **建议**: 后续可接日志/构建输出流

---

## 三、前后端对接一致性

| 项目 | 前端 | 后端 | 说明 |
|------|------|------|------|
| list 响应 | `data.nodes` 数组，项含 path/name/is_dir | FsListResponse { nodes: Vec<FsNode> } | ✅ 一致 |
| read 响应 | `data.content` 字符串 | FsReadResponse { content } | ✅ 一致 |
| write body | `{ content }` | FsWriteBody { content } | ✅ 一致 |
| create body | `{ path }` | FsQuery/body.path | ✅ 一致 |
| delete | DELETE + query path | DELETE + Query path | ✅ 已统一 |
| 错误体 | 未解析 | `{ "error", "message" }` | 建议前端解析 message |

---

## 四、修复优先级建议

1. **立即**: 编辑器 `onChange` 未传导致无法编辑 → 必须修复
2. **高**: 新建/保存/删除的用户可见错误反馈
3. **中**: API 错误解析后端 message、展开目录加载态、刷新时子目录缓存
4. **低**: 后端 path 校验、终端 cat 全量读、问题面板与 StatusBar 含义扩展

---

## 五、已完成的修复（历史）

- `/api/v1/fs/delete` 由 GET 改为 DELETE，与前端一致
- Terminal ResizeObserver 用 `requestAnimationFrame` 包裹，消除控制台「ResizeObserver loop」警告

---

## 六、真实验证与本次修复（2026-01-31）

### 6.1 已实现并验证的修复

| 项目 | 修改内容 | 验证 |
|------|----------|------|
| **编辑器输入** | App 传入 `onChange={handleEditorChange}`，同步 `fileContent` | 受控输入可正常编辑 |
| **API 错误展示** | 所有 fs 请求先 `response.json().catch(() => ({}))`，`!response.ok` 时用 `data.message \|\| data.error \|\| fallback` 抛错 | 后端 message 会进入 problems/界面 |
| **新建文件无挂载** | `MenuBar` 增加 `newFileDisabled={files.length === 0}`，无挂载时禁用「New File」并 title 提示 | 无挂载时按钮禁用 |
| **保存/删除/创建失败反馈** | 失败时 `setOperationProblems` 推入一条，problems 面板展示 | 问题面板显示保存/删除/创建错误 |
| **刷新子目录缓存** | `handleRefresh` 中 `setChildrenMap({})`、`setOperationProblems([])` 再 `fetchFiles()` | 刷新后子目录会重新拉取 |
| **删除仅成功关菜单** | `handleContextMenuClose()` 移到 try 内成功分支，失败不关菜单 | 删除失败时菜单保持可重试 |
| **展开子目录加载态** | `loadingPaths: Set<string>`，fetchFiles 非根时 add/remove；FileTree 接收 `loadingPaths`，展开且无 children 且 path 在 loadingPaths 时显示「加载中…」 | 展开目录时显示加载中 |

### 6.2 涉及文件

- `evif-web/src/App.tsx`：state（operationProblems、loadingPaths）、fetchFiles/read/save/create/delete 错误解析与 problems、handleRefresh 清缓存、handleDelete 仅成功关菜单、MenuBar/FileTree 新 props
- `evif-web/src/lib/http.ts`：新增 `parseErrorResponse`（供未先 parse body 的场景复用）
- `evif-web/src/components/MenuBar.tsx`：`newFileDisabled`、禁用样式与 title
- `evif-web/src/components/FileTree.tsx`：`loadingPaths`、展开时「加载中…」占位

### 6.3 浏览器验证结果

- 前端：http://localhost:3001/（或 3000）加载正常
- `GET /api/v1/fs/list?path=/` 返回 200，根列表正常
- WebSocket `/ws` 返回 101，终端连接正常
- 无 ResizeObserver 控制台警告
