# EVIF Web E2E测试完整实施规范

**规范ID**: SPEC-E2E-2026-001
**任务关联**: task-1770650819-b957, task-1770650820-ffe1
**优先级**: P1 (关键)
**创建日期**: 2026-02-10
**规范编写者**: 📋 Spec Writer

---

## 摘要

本规范定义了EVIF Web应用的完整端到端测试实施计划,覆盖所有核心UI功能、API集成、响应式设计、可访问性和性能验证。基于现有代码分析,EVIF Web包含以下主要功能模块:

1. **文件操作**: 创建、打开、保存、删除文件
2. **搜索功能**: 文本搜索、过滤、结果导航
3. **上传功能**: 拖拽上传、文件选择、进度显示
4. **监控功能**: 系统指标、告警面板、实时数据
5. **协作功能**: 访问控制、评论系统
6. **插件管理**: 插件列表、状态切换、配置
7. **全局Handle**: 文件句柄管理、TTL清理
8. **响应式布局**: 移动端/平板/桌面适配
9. **可访问性**: 键盘导航、屏幕阅读器、颜色对比度
10. **性能优化**: 加载时间、交互响应、资源优化

---

## 验收标准总览

### GWT覆盖范围

| 功能模块 | GWT数量 | 优先级 | 测试类型 |
|---------|---------|--------|---------|
| 文件操作 | 5 | P1 | 功能+API |
| 搜索功能 | 3 | P1 | 功能+UI |
| 上传功能 | 3 | P1 | 功能+UI |
| 监控功能 | 2 | P2 | 功能+API |
| 协作功能 | 2 | P2 | 功能+API |
| 插件管理 | 2 | P2 | 功能+API |
| 全局Handle | 5 | P1 | 功能+API |
| 响应式设计 | 3 | P1 | UI+UX |
| 可访问性 | 3 | P1 | a11y |
| 性能测试 | 2 | P2 | perf |
| **总计** | **30** | - | - |

---

## 详细验收标准

### 模块1: 文件操作测试 (File Operations)

#### GWT-E2E-FILE-001: 创建新文件
**优先级**: P1
**测试类型**: 功能测试 + API集成

**GIVEN** 用户在EVIF Web应用主页 (http://localhost:5173)
**WHEN** 用户执行以下操作之一:
  - 点击MenuBar的"File" → "New File"菜单项
  - 使用快捷键 Cmd+N (Mac) 或 Ctrl+N (Windows/Linux)
  - 点击CommandPalette并输入"New File"
**THEN** 系统应该:

1. **UI验证**:
   - 显示加载状态(骨架屏或spinner)<100ms
   - 在编辑器区域打开新标签页
   - 标签标题显示"untitled"或"未命名"
   - 面包屑导航显示"/local/untitled"
   - 编辑器获得焦点,可以输入内容
   - 文件树中显示新文件(如果文件树展开)

2. **API验证**:
   - 发送POST请求到`/api/v1/files`
   - 请求体包含: `{"path": "/local/untitled"}`
   - 响应状态码: 201 Created
   - 响应体包含:
     ```json
     {
       "path": "/local/untitled",
       "created_at": "2026-02-10T10:00:00Z",
       "size": 0
     }
     ```

3. **状态验证**:
   - 激活标签列表包含新文件tab
   - `activeTabId`设置为新文件的tab ID
   - 文件未保存状态指示器显示(如"*"标记)
   - 无错误提示或警告

**边界条件**:
- 如果创建失败(如权限不足),显示错误toast提示
- 如果文件已存在,提示用户覆盖或取消

---

#### GWT-E2E-FILE-002: 保存文件内容
**优先级**: P1
**测试类型**: 功能测试 + API集成

**GIVEN** 用户在编辑器中打开了一个文件
**WHEN** 用户:
  1. 在编辑器中输入或修改内容
  2. 点击"File" → "Save"菜单
  3. 或使用快捷键 Cmd+S / Ctrl+S
**THEN** 系统应该:

1. **UI验证**:
   - 显示"保存中..."加载状态
   - 保存完成后显示"文件已保存"toast提示
   - 标签页的未保存指示器("*")消失
   - StatusBar显示"已保存"状态

2. **API验证**:
   - 发送POST请求到`/api/v1/files/{path}`
   - 请求体包含文件内容:
     ```json
     {
       "content": "文件内容...",
       "encoding": "utf-8"
     }
     ```
   - 响应状态码: 200 OK
   - 响应体包含:
     ```json
     {
       "path": "/local/untitled",
       "saved_at": "2026-02-10T10:05:00Z",
       "size": 1024
     }
     ```

3. **错误处理**:
   - 如果保存失败,显示错误提示并保持"未保存"状态
   - 如果网络错误,显示"保存失败,请检查网络连接"

**自动化测试步骤**:
```typescript
test('should save file content', async ({ page }) => {
  // 1. 创建新文件
  await page.goto('/');
  await page.getByRole('menuitem', { name: 'File' }).click();
  await page.getByRole('menuitem', { name: 'New File' }).click();

  // 2. 输入内容
  const editor = page.locator('.monaco-editor');
  await editor.click();
  await editor.type('Hello EVIF!');

  // 3. 保存文件
  await page.keyboard.press('Meta+S'); // Mac

  // 4. 验证API调用
  await expect(page).toHaveURL(/\/api\/v1\/files\/local\/untitled/);

  // 5. 验证UI状态
  const toast = page.locator('.toast').filter({ hasText: '保存' });
  await expect(toast).toBeVisible();
});
```

---

#### GWT-E2E-FILE-003: 打开现有文件
**优先级**: P1
**测试类型**: 功能测试 + API集成

**GIVEN** 文件系统中有可用的文件
**WHEN** 用户执行以下操作之一:
  - 在FileTree中点击文件名
  - 使用快捷键 Cmd+K / Ctrl+K 打开QuickOpen
  - 使用CommandPalette输入"Open File"
**THEN** 系统应该:

1. **UI验证**:
   - 显示文件加载状态(<100ms)
   - 在编辑器中打开文件内容
   - 激活对应的标签页
   - 更新面包屑导航显示文件路径
   - 文件树中高亮当前文件

2. **API验证**:
   - 发送GET请求到`/api/v1/files/{path}`
   - 响应状态码: 200 OK
   - 响应体包含:
     ```json
     {
       "path": "/local/test.txt",
       "content": "文件内容",
       "size": 1024,
       "modified_at": "2026-02-10T09:00:00Z"
     }
     ```

3. **QuickOpen功能**:
   - 快捷键触发后,QuickOpen对话框在<100ms内显示
   - 输入文件名时实时过滤文件列表
   - 支持模糊搜索
   - 按上下键选择文件
   - 按Enter键打开选中的文件

---

#### GWT-E2E-FILE-004: 删除文件
**优先级**: P1
**测试类型**: 功能测试 + API集成

**GIVEN** 用户选中了一个文件(在FileTree中或当前激活的标签)
**WHEN** 用户:
  1. 右键点击文件,选择"Delete"
  2. 或使用CommandPalette输入"Delete File"
  3. 在确认对话框中点击"确认"
**THEN** 系统应该:

1. **UI验证**:
   - 显示确认对话框:"确定要删除文件{filename}吗?"
   - 确认后从文件树中移除该文件
   - 如果文件已打开,关闭对应的标签页
   - 显示"文件已删除"toast提示
   - 如果删除的是当前激活文件,切换到其他标签或显示空状态

2. **API验证**:
   - 发送DELETE请求到`/api/v1/files/{path}`
   - 响应状态码: 204 No Content
   - 后续GET请求返回404 Not Found

3. **错误处理**:
   - 如果文件不存在,显示错误提示
   - 如果权限不足,显示"权限不足"错误

---

#### GWT-E2E-FILE-005: 多标签页管理
**优先级**: P2
**测试类型**: UI交互测试

**GIVEN** 用户打开了多个文件标签
**WHEN** 用户执行标签操作:
  - 点击标签页切换文件
  - 点击标签页的"x"按钮关闭标签
  - 使用快捷键 Cmd+W / Ctrl+W 关闭当前标签
  - 使用Cmd+Tab / Ctrl+Tab切换标签
**THEN** 系统应该:

1. **标签切换**:
   - 点击标签后,编辑器内容切换到对应文件
   - 激活标签高亮显示
   - 面包屑导航更新为当前文件路径
   - 文件树高亮当前文件

2. **标签关闭**:
   - 如果文件有未保存修改,提示用户保存
   - 关闭后激活相邻标签
   - 如果是最后一个标签,显示空状态
   - 标签栏宽度自动调整

3. **标签顺序**:
   - 最近打开的标签显示在最右侧
   - 激活的标签保持在可见区域
   - 标签过多时显示左右滚动按钮

---

### 模块2: 搜索功能测试 (Search)

#### GWT-E2E-SEARCH-001: 基本文本搜索
**优先级**: P1
**测试类型**: 功能测试 + UI测试

**GIVEN** 用户在EVIF Web应用主页
**WHEN** 用户:
  1. 在搜索栏输入关键词(如"test")
  2. 点击"搜索"按钮或按Enter键
**THEN** 系统应该:

1. **UI验证**:
   - 搜索按钮在输入前禁用(灰色)
   - 输入内容后搜索按钮启用(蓝色高亮)
   - 点击搜索后显示加载状态
   - SearchResults组件在<200ms内显示结果
   - 显示找到的文件数量(如"找到5个结果")
   - 每个结果显示:文件路径、匹配行数、预览片段

2. **API验证**:
   - 发送POST请求到`/api/v1/grep`
   - 请求体包含:
     ```json
     {
       "pattern": "test",
       "path": "/",
       "case_sensitive": false
     }
     ```
   - 响应状态码: 200 OK
   - 响应体包含:
     ```json
     {
       "results": [
         {
           "path": "/local/test.txt",
           "matches": [
             {"line": 1, "content": "this is a test"},
             {"line": 5, "content": "another test"}
           ]
         }
       ],
       "total_files": 5,
       "total_matches": 12
     }
     ```

3. **结果交互**:
   - 点击结果项在编辑器中打开文件
   - 自动跳转到匹配的行
   - 高亮显示匹配的文本

---

#### GWT-E2E-SEARCH-002: 高级搜索过滤
**优先级**: P2
**测试类型**: 功能测试 + UI测试

**GIVEN** 用户打开了搜索面板
**WHEN** 用户设置过滤条件:
  - 文件类型过滤(如*.txt, *.md)
  - 日期范围过滤(修改时间)
  - 大小过滤
  - 路径范围
**THEN** 系统应该:

1. **FilterPanel UI**:
   - 显示所有可用的过滤选项
   - 过滤条件实时更新URL查询参数
   - URL格式: `?q=test&type=txt&after=2026-02-01`
   - 支持保存常用的过滤条件为预设

2. **API验证**:
   - 发送带过滤参数的请求到`/api/v1/grep`
   - 请求体包含过滤条件:
     ```json
     {
       "pattern": "test",
       "file_pattern": "*.txt",
       "after": "2026-02-01T00:00:00Z",
       "max_size": 1048576
     }
     ```
   - 响应只包含符合条件的结果

3. **状态保持**:
   - 浏览器前进/后退按钮保持过滤状态
   - 刷新页面后过滤条件恢复
   - 导航到其他页面后返回,过滤条件保留

---

#### GWT-E2E-SEARCH-003: 搜索结果导航
**优先级**: P2
**测试类型**: UI交互测试

**GIVEN** 搜索结果包含多个匹配项
**WHEN** 用户:
  - 点击搜索结果中的某一项
  - 使用键盘导航(上下键)
  - 点击"下一个"/"上一个"按钮
**THEN** 系统应该:

1. **结果导航**:
   - 点击结果项在编辑器中打开对应文件
   - 光标定位到匹配的行
   - 匹配的文本高亮显示
   - 编辑器滚动到匹配位置(如果在视图外)

2. **键盘导航**:
   - 上键选择上一个结果
   - 下键选择下一个结果
   - Enter键打开选中的结果
   - Esc键关闭搜索面板

3. **结果高亮**:
   - 当前选中的结果项高亮显示
   - 已访问的结果项标记为不同颜色
   - 显示当前位置(如"3/12")

---

### 模块3: 上传功能测试 (Upload)

#### GWT-E2E-UPLOAD-001: 拖拽上传
**优先级**: P1
**测试类型**: 功能测试 + UI测试

**GIVEN** 用户在EVIF Web应用主页
**WHEN** 用户拖拽文件到UploadDropzone区域
**THEN** 系统应该:

1. **拖拽交互**:
   - 文件拖入dropzone区域时,区域高亮显示(蓝色边框)
   - 显示"释放以上传文件"提示文本
   - 显示拖拽的文件名和文件大小
   - 释放文件后立即开始上传

2. **上传进度**:
   - UploadManager组件显示上传进度条
   - 进度条实时更新百分比
   - 显示上传速度和剩余时间
   - 上传完成后显示"上传成功"提示

3. **API验证**:
   - 发送PUT请求到`/api/v1/files?path={destination_path}`
   - 请求Content-Type: `application/octet-stream`
   - 请求体包含文件的base64编码或binary数据
   - 响应状态码: 201 Created
   - 响应体包含:
     ```json
     {
       "path": "/local/uploaded-file.txt",
       "size": 1024,
       "uploaded_at": "2026-02-10T10:10:00Z"
     }
     ```

4. **完成状态**:
   - 文件列表自动刷新显示新上传的文件
   - 可以点击打开文件进行编辑
   - 上传区域恢复初始状态

**自动化测试步骤**:
```typescript
test('should upload file via drag and drop', async ({ page }) => {
  await page.goto('/');

  // 创建测试文件
  const file = new File(['test content'], 'test.txt', { type: 'text/plain' });

  // 模拟拖拽上传
  const dropzone = page.locator('.upload-dropzone');
  const dataTransfer = await page.evaluateHandle((file) => {
    const dt = new DataTransfer();
    dt.items.add(file);
    return dt;
  }, file);

  await dropzone.dispatchEvent('drop', { dataTransfer });

  // 验证上传成功
  await expect(page.locator('.toast')).toContainText('上传成功');
});
```

---

#### GWT-E2E-UPLOAD-002: 文件选择上传
**优先级**: P1
**测试类型**: 功能测试 + UI测试

**GIVEN** 用户在EVIF Web应用主页
**WHEN** 用户:
  1. 点击"选择文件"按钮
  2. 在文件选择对话框中选择一个或多个文件
  3. 点击"打开"
**THEN** 系统应该:

1. **文件选择**:
   - 点击按钮打开系统文件选择对话框
   - 支持选择多个文件(多选)
   - 显示选中的文件列表
   - 每个文件显示:文件名、大小、类型图标

2. **上传队列**:
   - 文件加入上传队列
   - 按选择顺序依次上传
   - 支持取消单个文件上传
   - 支持清空上传队列

3. **批量上传**:
   - 显示总体上传进度(如"3/5文件已上传")
   - 每个文件独立显示进度
   - 失败的文件可以重试
   - 全部完成后显示摘要

---

#### GWT-E2E-UPLOAD-003: 目录选择上传
**优先级**: P2
**测试类型**: 功能测试 + UI测试

**GIVEN** 用户在EVIF Web应用主页
**WHEN** 用户:
  1. 点击"选择目录"按钮
  2. 选择一个本地目录
**THEN** 系统应该:

1. **目录上传**:
   - 递归上传目录中的所有文件
   - 保持目录结构
   - 显示上传进度(文件数/总文件数)

2. **进度显示**:
   - 显示当前正在上传的文件
   - 显示已上传的文件列表
   - 支持暂停/恢复上传

3. **API验证**:
   - 每个文件发送独立的PUT请求
   - 路径包含目录结构(如`/local/folder/file.txt`)
   - 支持大文件分块上传

---

### 模块4: 监控功能测试 (Monitor)

#### GWT-E2E-MONITOR-001: 系统指标显示
**优先级**: P2
**测试类型**: 功能测试 + API集成

**GIVEN** 用户导航到Monitor页面
**WHEN** MonitorView组件加载完成
**THEN** 系统应该:

1. **UI验证**:
   - 显示CPU使用率卡片(百分比 + 趋势图)
   - 显示内存使用卡片(已用/总计 + 趋势图)
   - 显示磁盘I/O卡片(读/写速率)
   - 显示网络活动卡片(上传/下载速率)
   - 所有MetricCard包含:当前值、单位、趋势箭头、小图表

2. **API验证**:
   - 发送GET请求到`/api/v1/metrics/status`
   - 响应状态码: 200 OK
   - 响应体包含:
     ```json
     {
       "cpu": {
         "usage_percent": 45.2,
         "cores": 8,
         "frequency": "2.4GHz"
       },
       "memory": {
         "total": 17179869184,
         "used": 8589934592,
         "available": 8589934592,
         "percent": 50.0
       },
       "disk": {
         "read_bytes_per_sec": 1024000,
         "write_bytes_per_sec": 512000
       },
       "network": {
         "rx_bytes_per_sec": 1024000,
         "tx_bytes_per_sec": 512000
       }
     }
     ```

3. **实时更新**:
   - 指标每5秒自动刷新
   - 趋势图显示最近60秒的数据
   - 手动刷新按钮可用

---

#### GWT-E2E-MONITOR-002: 告警面板
**优先级**: P2
**测试类型**: UI测试 + 功能测试

**GIVEN** 系统产生了告警
**WHEN** 用户查看AlertPanel
**THEN** 系统应该:

1. **告警显示**:
   - 按严重程度分类显示:critical(红色)、warning(橙色)、info(蓝色)
   - 每个告警显示:时间、消息、来源、状态
   - 最新的告警显示在顶部
   - 未读告警加粗显示

2. **告警交互**:
   - 点击告警项展开详细信息
   - 提供"标记为已读"操作
   - 提供"删除告警"操作
   - 提供"全部标记为已读"按钮

3. **告警过滤**:
   - 支持按严重程度过滤
   - 支持按时间范围过滤
   - 支持按来源过滤

---

### 模块5: 协作功能测试 (Collaboration)

#### GWT-E2E-COLLAB-001: 访问控制列表
**优先级**: P2
**测试类型**: 功能测试 + API集成

**GIVEN** 用户打开ShareModal
**WHEN** 用户查看AccessControlList
**THEN** 系统应该:

1. **UI验证**:
   - 显示所有已授权用户的列表
   - 每个用户显示:用户名、权限级别、头像
   - 权限级别:Read、Write、Admin
   - 提供"添加用户"按钮
   - 提供修改权限下拉菜单

2. **API验证**:
   - GET `/api/v1/files/{path}/acl` 返回访问控制列表
   - POST `/api/v1/files/{path}/acl` 添加用户权限
   - PATCH `/api/v1/files/{path}/acl/{user_id}` 修改权限
   - DELETE `/api/v1/files/{path}/acl/{user_id}` 移除权限

---

#### GWT-E2E-COLLAB-002: 评论功能
**优先级**: P2
**测试类型**: 功能测试 + API集成

**GIVEN** 用户打开CommentPanel
**WHEN** 用户添加新评论
**THEN** 系统应该:

1. **UI验证**:
   - 显示评论输入框(多行文本)
   - 显示评论列表(按时间倒序)
   - 每条评论显示:作者、时间戳、内容
   - 提供"提交"按钮

2. **API验证**:
   - POST `/api/v1/files/{path}/comments` 创建评论
   - GET `/api/v1/files/{path}/comments` 获取评论列表
   - DELETE `/api/v1/files/{path}/comments/{comment_id}` 删除评论

---

### 模块6: 插件管理测试 (Plugin Manager)

#### GWT-E2E-PLUGIN-001: 插件列表显示
**优先级**: P2
**测试类型**: UI测试 + API集成

**GIVEN** 用户打开PluginManager
**WHEN** PluginList加载完成
**THEN** 系统应该:

1. **UI验证**:
   - 显示所有已安装插件的列表
   - 每个插件卡片显示:名称、描述、版本、状态(启用/禁用)
   - 提供状态切换开关(toggle)
   - 提供"查看详情"按钮
   - 提供"设置"按钮(如果可用)

2. **API验证**:
   - GET `/api/v1/plugins` 返回插件列表
   - GET `/api/v1/plugins/:name/readme` 返回插件说明
   - GET `/api/v1/plugins/:name/config` 返回插件配置

---

#### GWT-E2E-PLUGIN-002: 插件状态切换
**优先级**: P2
**测试类型**: 功能测试 + API集成

**GIVEN** 用户在PluginManager中
**WHEN** 用户切换插件状态
**THEN** 系统应该:

1. **UI交互**:
   - 点击toggle开关
   - 显示"切换中..."加载状态
   - 完成后更新状态显示
   - 显示"插件已启用/禁用"toast提示

2. **API验证**:
   - POST `/api/v1/mount` 启用插件
   - POST `/api/v1/unmount` 禁用插件
   - 响应包含新的插件状态

---

### 模块7: 全局Handle管理测试

#### GWT-E2E-HANDLE-001: 打开文件句柄
**优先级**: P1
**测试类型**: API集成测试

**GIVEN** 用户想要打开文件进行读写
**WHEN** 系统调用open API
**THEN** 系统应该:

1. **API验证**:
   - POST `/api/v1/handles/open`
   - 请求体:
     ```json
     {
       "path": "/local/file.txt",
       "mode": "read_write"
     }
     ```
   - 响应状态码: 201 Created
   - 响应体:
     ```json
     {
       "handle_id": 12345,
       "path": "/local/file.txt",
       "mode": "read_write",
       "created_at": "2026-02-10T10:00:00Z",
       "ttl": 1800
     }
     ```

2. **状态验证**:
   - handle注册到GlobalHandleManager
   - 后续可使用handle_id进行操作
   - TTL开始计时

---

#### GWT-E2E-HANDLE-002: 读取文件句柄
**优先级**: P1
**测试类型**: API集成测试

**GIVEN** 用户有一个打开的文件句柄(ID:12345)
**WHEN** 用户读取文件内容
**THEN** 系统应该:

1. **API验证**:
   - POST `/api/v1/handles/12345/read`
   - 请求体:
     ```json
     {
       "size": 1024
     }
     ```
   - 响应状态码: 200 OK
   - 响应体:
     ```json
     {
       "data": "base64encodedcontent",
       "bytes_read": 1024,
       "position": 1024
     }
     ```

---

#### GWT-E2E-HANDLE-003: 写入文件句柄
**优先级**: P1
**测试类型**: API集成测试

**GIVEN** 用户有一个打开的文件句柄
**WHEN** 用户写入数据
**THEN** 系统应该:

1. **API验证**:
   - POST `/api/v1/handles/{id}/write`
   - 请求体:
     ```json
     {
       "data": "base64encodeddata"
     }
     ```
   - 响应状态码: 200 OK
   - 响应体:
     ```json
     {
       "bytes_written": 1024,
       "position": 1024
     }
     ```

---

#### GWT-E2E-HANDLE-004: 关闭文件句柄
**优先级**: P1
**测试类型**: API集成测试

**GIVEN** 用户完成文件操作
**WHEN** 用户关闭句柄
**THEN** 系统应该:

1. **API验证**:
   - POST `/api/v1/handles/{id}/close`
   - 响应状态码: 200 OK
   - GlobalHandleManager移除该句柄

---

#### GWT-E2E-HANDLE-005: TTL自动清理
**优先级**: P1
**测试类型**: 功能测试 + API验证

**GIVEN** 用户有一个打开的句柄
**WHEN** 句柄超过TTL时间(30分钟)未使用
**THEN** 系统应该:

1. **自动清理**:
   - 后台任务定期检查过期句柄
   - 过期句柄自动关闭
   - 释放相关资源

2. **API验证**:
   - GET `/api/v1/handles/stats`
   - 响应体:
     ```json
     {
       "total_handles": 100,
       "active_handles": 80,
       "expired_handles": 20
     }
     ```

---

### 模块8: 响应式设计测试

#### GWT-E2E-RESPONSIVE-001: 移动端视图(<768px)
**优先级**: P1
**测试类型**: UI测试 + 可视化回归测试

**GIVEN** 用户在移动设备(viewport宽度<768px)
**WHEN** 页面加载完成
**THEN** 系统应该:

1. **布局调整**:
   - MenuBar显示汉堡菜单按钮(三条横线图标)
   - 点击汉堡菜单展开/收起侧边栏
   - 侧边栏默认隐藏,覆盖式显示
   - 编辑器宽度为100%
   - Panel默认隐藏,点击图标从底部滑入

2. **触摸优化**:
   - 所有按钮至少44x44px(iOS标准)
   - 增加间距防止误触
   - 支持滑动手势(左滑关闭侧边栏)

3. **测试验证**:
   ```typescript
   test('mobile layout', async ({ page }) => {
     await page.setViewportSize({ width: 375, height: 667 }); // iPhone
     await page.goto('/');

     // 验证汉堡菜单
     const hamburger = page.locator('[aria-label="Menu"]');
     await expect(hamburger).toBeVisible();

     // 点击展开侧边栏
     await hamburger.click();
     const sidebar = page.locator('.sidebar');
     await expect(sidebar).toHaveCSS('transform', 'translateX(0%)');
   });
   ```

---

#### GWT-E2E-RESPONSIVE-002: 平板视图(768px-1024px)
**优先级**: P1
**测试类型**: UI测试

**GIVEN** 用户在平板设备(768px<=宽度<1024px)
**WHEN** 页面加载完成
**THEN** 系统应该:

1. **布局调整**:
   - 侧边栏简化显示(只显示图标)
   - 悬停显示完整标签
   - 编辑器和面板并排显示

---

#### GWT-E2E-RESPONSIVE-003: 桌面视图(>=1024px)
**优先级**: P1
**测试类型**: UI测试

**GIVEN** 用户在桌面设备(宽度>=1024px)
**WHEN** 页面加载完成
**THEN** 系统应该:

1. **完整布局**:
   - 侧边栏完整显示(图标+标签)
   - 多列编辑器布局可用
   - 所有面板和工具栏可见

---

### 模块9: 可访问性测试

#### GWT-E2E-A11Y-001: 键盘导航
**优先级**: P1
**测试类型**: 可访问性测试

**GIVEN** 用户只使用键盘
**WHEN** 用户使用Tab键导航
**THEN** 系统应该:

1. **Tab顺序**:
   - Tab顺序符合视觉顺序(从左到右,从上到下)
   - 焦点可见(明显的focus ring或outline)
   - 焦点不跳过隐藏元素

2. **快捷键**:
   - Enter/Space键激活按钮和链接
   - Escape键关闭模态框和下拉菜单
   - 方向键导航列表和菜单

3. **焦点陷阱**:
   - 模态框打开时焦点限制在模态框内
   - Tab键在模态框内循环
   - 关闭模态框后焦点返回触发元素

**自动化测试**:
```typescript
test('keyboard navigation', async ({ page }) => {
  await page.goto('/');

  // Tab键导航
  await page.keyboard.press('Tab');
  const firstButton = page.locator('button').first();
  await expect(firstButton).toBeFocused();

  // Enter键激活
  await page.keyboard.press('Enter');

  // Escape键关闭
  await page.keyboard.press('Escape');
});
```

---

#### GWT-E2E-A11Y-002: 屏幕阅读器支持
**优先级**: P1
**测试类型**: 可访问性测试

**GIVEN** 用户使用屏幕阅读器(如NVDA、VoiceOver)
**WHEN** 用户导航页面
**THEN** 系统应该:

1. **语义化HTML**:
   - 使用正确的HTML5标签(header、nav、main、footer)
   - 表单元素有关联的label
   - 图片有alt属性
   - 使用ARIA属性增强(如aria-label、aria-describedby)

2. **ARIA属性**:
   - 所有交互元素有role属性
   - 状态变化用aria-live通知
   - 加载状态用aria-busy标记

**测试验证**:
使用axe-core进行可访问性审计:
```typescript
test('accessibility with axe', async ({ page }) => {
  await page.goto('/');
  const accessibilityScanResults = await axePlaywright(page);
  expect(accessibilityScanResults.violations).toEqual([]);
});
```

---

#### GWT-E2E-A11Y-003: 颜色对比度
**优先级**: P1
**测试类型**: 可视化测试

**GIVEN** 用户有视觉障碍
**WHEN** 用户查看页面
**THEN** 系统应该:

1. **对比度要求**:
   - 所有文本对比度≥4.5:1(WCAG AA)
   - 主要交互元素对比度≥7:1(WCAG AAA)
   - 不只依赖颜色传达信息(使用图标+颜色)

2. **验证方法**:
   - 使用Playwright的对比度检查工具
   - 使用axe-core的color-contrast规则
   - 手动验证: Chrome DevTools的Contrast Ratio工具

**测试验证**:
```typescript
test('color contrast', async ({ page }) => {
  await page.goto('/');

  // 检查所有文本元素的颜色对比度
  const textElements = await page.$$('p, span, h1, h2, h3, label, button');
  for (const el of textElements) {
    const contrast = await getContrastRatio(el);
    expect(contrast).toBeGreaterThanOrEqual(4.5);
  }
});
```

---

### 模块10: 性能测试

#### GWT-E2E-PERF-001: 页面加载性能
**优先级**: P2
**测试类型**: 性能测试

**GIVEN** 用户首次访问应用
**WHEN** 页面加载
**THEN** 系统应该:

1. **Web Vitals指标**:
   - FCP(First Contentful Paint)<1.8秒
   - LCP(Largest Contentful Paint)<2.5秒
   - FID(First Input Delay)<100ms
   - CLS(Cumulative Layout Shift)<0.1

2. **测试方法**:
   ```typescript
   test('page load performance', async ({ page }) => {
     const startTime = Date.now();
     await page.goto('/');

     // 测量FCP
     const fcp = await page.evaluate(() => {
       return new Promise((resolve) => {
         new PerformanceObserver((list) => {
           const entries = list.getEntries();
           const fcpEntry = entries.find(
             (entry) => entry.name === 'first-contentful-paint'
           );
           resolve(fcpEntry?.startTime);
         }).observe({ entryTypes: ['paint'] });
       });
     });
     expect(fcp).toBeLessThan(1800);
   });
   ```

---

#### GWT-E2E-PERF-002: 交互响应性能
**优先级**: P2
**测试类型**: 性能测试

**GIVEN** 用户与UI交互
**WHEN** 用户点击按钮或输入
**THEN** 系统应该:

1. **响应时间**:
   - 视觉反馈在100ms内显示
   - 动画帧率≥60fps
   - 主线程不阻塞(长任务<50ms)

2. **测试方法**:
   ```typescript
   test('interaction performance', async ({ page }) => {
     await page.goto('/');

     const button = page.locator('button').first();
     const startTime = Date.now();
     await button.click();
     const endTime = Date.now();

     expect(endTime - startTime).toBeLessThan(100);
   });
   ```

---

## 测试工具和配置

### Playwright配置

文件:`playwright.config.ts`

```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  testMatch: /.*\.spec\.ts$/,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 1,
  workers: process.env.CI ? 1 : undefined,
  reporter: [
    ['html', { open: 'never' }],
    ['list'],
    ['json', { outputFile: 'test-results/results.json' }],
  ],
  use: {
    baseURL: process.env.BASE_URL || 'http://localhost:5173',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 15000,
    navigationTimeout: 30000,
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
    {
      name: 'Mobile Chrome',
      use: { ...devices['Pixel 5'] },
    },
    {
      name: 'Mobile Safari',
      use: { ...devices['iPhone 12'] },
    },
  ],

  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});
```

### 测试目录结构

```
evif-web/e2e/
├── fixtures/
│   ├── auth.ts                 # 认证fixture
│   ├── api-mock.ts             # API mock fixture
│   ├── test-data.ts            # 测试数据生成器
│   └── server.ts               # 测试服务器设置
├── pages/
│   ├── app.page.ts             # 应用主页页面对象
│   ├── editor.page.ts          # 编辑器页面对象
│   ├── search.page.ts          # 搜索页面对象
│   ├── upload.page.ts          # 上传页面对象
│   ├── monitor.page.ts         # 监控页面对象
│   └── plugin-manager.page.ts  # 插件管理页面对象
├── specs/
│   ├── file-operations.spec.ts     # 文件操作测试(5个GWT)
│   ├── search.spec.ts              # 搜索功能测试(3个GWT)
│   ├── upload.spec.ts              # 上传功能测试(3个GWT)
│   ├── monitor.spec.ts             # 监控功能测试(2个GWT)
│   ├── collaboration.spec.ts       # 协作功能测试(2个GWT)
│   ├── plugin-manager.spec.ts      # 插件管理测试(2个GWT)
│   ├── handles.spec.ts             # 全局Handle测试(5个GWT)
│   ├── responsive.spec.ts          # 响应式设计测试(3个GWT)
│   ├── accessibility.spec.ts       # 可访问性测试(3个GWT)
│   └── performance.spec.ts         # 性能测试(2个GWT)
├── utils/
│   ├── helpers.ts                 # 辅助函数
│   ├── selectors.ts               # 选择器定义
│   └── assertions.ts              # 自定义断言
├── playwright.config.ts           # Playwright配置
└── tsconfig.json                  # TypeScript配置
```

### Page Object Model示例

```typescript
// e2e/pages/app.page.ts
import type { Page, Locator } from '@playwright/test';

export class AppPage {
  readonly page: Page;
  readonly menuBar: Locator;
  readonly activityBar: Locator;
  readonly sidebar: Locator;
  readonly editor: Locator;
  readonly panel: Locator;
  readonly statusBar: Locator;

  constructor(page: Page) {
    this.page = page;
    this.menuBar = page.locator('header');
    this.activityBar = page.locator('.activity-bar');
    this.sidebar = page.locator('.sidebar');
    this.editor = page.locator('.editor-container');
    this.panel = page.locator('.panel');
    this.statusBar = page.locator('footer');
  }

  async goto() {
    await this.page.goto('/');
  }

  async waitForLoad() {
    await this.page.waitForSelector('.app', { state: 'visible' });
  }

  async newFile() {
    await this.menuBar.getByRole('menuitem', { name: 'File' }).click();
    await this.page.getByRole('menuitem', { name: 'New File' }).click();
  }

  async saveFile() {
    await this.page.keyboard.press('Meta+S');
  }

  async openCommandPalette() {
    await this.page.keyboard.press('Meta+Shift+P');
  }
}

// e2e/pages/editor.page.ts
export class EditorPage {
  readonly page: Page;
  readonly editor: Locator;
  readonly tabs: Locator;
  readonly breadcrumb: Locator;

  constructor(page: Page) {
    this.page = page;
    this.editor = page.locator('.monaco-editor');
    this.tabs = page.locator('.editor-tabs');
    this.breadcrumb = page.locator('[data-testid="breadcrumb"]');
  }

  async getContent() {
    return await this.editor.textContent();
  }

  async type(text: string) {
    await this.editor.click();
    await this.editor.type(text);
  }

  async getActiveTab() {
    return await this.tabs.locator('.tab.active').textContent();
  }

  async waitForFileLoaded(filename: string) {
    await this.breadcrumb.waitFor({ state: 'visible' });
    await expect(this.breadcrumb).toContainText(filename);
  }
}
```

### 测试辅助函数

```typescript
// e2e/utils/helpers.ts
import type { Page } from '@playwright/test';

/**
 * 等待API响应
 */
export async function waitForApiResponse(
  page: Page,
  urlPattern: string | RegExp,
  timeout = 5000
) {
  return await page.waitForResponse(
    (response) =>
      response.url().match(urlPattern) && response.status() === 200,
    { timeout }
  );
}

/**
 * 模拟文件上传
 */
export async function uploadFile(
  page: Page,
  selector: string,
  fileName: string,
  content: string
) {
  const fileInput = page.locator(selector);
  const file = new File([content], fileName, { type: 'text/plain' });
  await fileInput.setInputFiles(fileName);
}

/**
 * 获取元素颜色对比度
 */
export async function getContrastRatio(
  page: Page,
  selector: string
): Promise<number> {
  const foreground = await page.locator(selector).evaluate(
    (el) => window.getComputedStyle(el).color
  );
  const background = await page.locator(selector).evaluate(
    (el) => window.getComputedStyle(el).backgroundColor
  );
  // 计算对比度的算法...
  return 4.5; // 示例值
}
```

---

## 测试执行标准

### 通过标准

1. **测试通过率**:
   - 所有E2E测试通过率≥95%
   - P1级别测试必须100%通过
   - P2级别测试允许少量失败(需说明原因)

2. **可访问性标准**:
   - 0个critical级别问题
   - 0个serious级别问题
   - minor级别问题<5个

3. **性能标准**:
   - Lighthouse性能分数≥90
   - 所有Web Vitals指标达标

### 测试报告

每次测试运行生成以下报告:

1. **HTML报告**:
   - 包含失败测试的截图
   - 失败测试的视频录制
   - 测试执行时间统计
   - 命令:`npx playwright show-report`

2. **可访问性报告**:
   - axe-core扫描结果
   - 违反规则的详细说明
   - 修复建议

3. **性能报告**:
   - Lighthouse分数
   - Web Vitals指标
   - 资源加载时间分析

4. **API覆盖报告**:
   - 已测试的API端点列表
   - 未测试的端点(需补充)

### CI/CD集成

```yaml
# .github/workflows/e2e.yml
name: E2E Tests

on:
  push:
    branches: [main, develop]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: 18

      - name: Install dependencies
        run: npm ci

      - name: Install Playwright browsers
        run: npx playwright install --with-deps

      - name: Run E2E tests
        run: npm run test:e2e

      - name: Upload test report
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
```

---

## 非功能性需求

### 可维护性

1. **Page Object Model**:
   - 所有页面对象继承自基类
   - 选择器集中定义在`selectors.ts`
   - 复用通用方法

2. **测试数据管理**:
   - 使用fixtures创建测试数据
   - 每个测试独立数据(不依赖其他测试)
   - 测试后清理数据

3. **清晰的命名**:
   - 测试文件名:`{feature}.spec.ts`
   - 测试名称:`should {expected behavior}`
   - 变量名:描述性命名

### 可靠性

1. **等待策略**:
   - 使用`waitForSelector`代替固定延迟
   - 使用`waitForResponse`验证API调用
   - 合理设置timeout(默认15秒)

2. **重试机制**:
   - CI环境重试2次
   - 本地开发重试1次
   - 失败时保存trace文件

3. **测试隔离**:
   - 每个测试独立运行
   - 不依赖测试执行顺序
   - 测试前重置应用状态

---

## 依赖关系

### 前置条件

1. **开发服务器**:
   - 前端:`npm run dev`(端口5173)
   - 后端:EVIF REST服务(端口8080)

2. **测试数据**:
   - 准备测试文件和目录
   - Mock API响应(如果后端未完成)

3. **浏览器**:
   - Chromium、Firefox、WebKit已安装
   - 命令:`npx playwright install`

### 外部依赖

```json
{
  "devDependencies": {
    "@playwright/test": "^1.40.0",
    "@axe-core/playwright": "^4.8.0",
    "typescript": "^5.3.0"
  }
}
```

---

## 超出范围

以下内容不在本规范范围内:

1. **后端单元测试**: 已在Phase 1完成
2. **前端组件单元测试**: 需要单独规范(Vitest + React Testing Library)
3. **负载测试**: 需要专门的性能测试工具(k6)
4. **安全渗透测试**: 需要安全专家
5. **跨浏览器测试**: 已覆盖主要浏览器,不包括IE

---

## 验收标准总结

### GWT-SPEC-001: 规范完整性

**GIVEN** 开发者阅读本规范
**WHEN** 实施E2E测试
**THEN** 应该能够:
  - 理解所有30个GWT测试场景
  - 知道如何组织测试代码
  - 知道如何验证结果
  - 无需额外解释即可实施

### GWT-SPEC-002: 测试覆盖完整性

**GIVEN** 本规范定义的测试范围
**WHEN** 审查测试覆盖
**THEN** 应该:
  - 覆盖所有10个功能模块
  - 包含30个GWT测试场景
  - 包含API集成验证
  - 包含可访问性和性能测试

### GWT-SPEC-003: 可测试性

**GIVEN** 测试实施者
**WHEN** 编写测试代码
**THEN** 每个GWT应该:
  - 有明确的前置条件(Given)
  - 有清晰的触发操作(When)
  - 有可验证的预期结果(Then)
  - 包含自动化测试示例代码

---

## 实施优先级

### Phase 1: P1测试(必需)
**预计时间**: 2-3天

1. 文件操作测试(5个GWT)
2. 搜索功能测试(3个GWT)
3. 上传功能测试(3个GWT)
4. 全局Handle测试(5个GWT)
5. 响应式设计测试(3个GWT)
6. 可访问性测试(3个GWT)

**总计**: 22个GWT

### Phase 2: P2测试(重要)
**预计时间**: 1-2天

1. 监控功能测试(2个GWT)
2. 协作功能测试(2个GWT)
3. 插件管理测试(2个GWT)
4. 性能测试(2个GWT)

**总计**: 8个GWT

---

## 附录A: 测试检查清单

### 测试前检查

- [ ] 开发服务器正在运行(端口5173)
- [ ] 后端API服务可用(端口8080)
- [ ] Playwright浏览器已安装
- [ ] 测试数据已准备
- [ ] 网络连接正常

### 测试执行检查

- [ ] 所有P1测试通过
- [ ] 可访问性扫描无critical问题
- [ ] 性能指标符合标准
- [ ] 测试报告已生成

### 测试后检查

- [ ] 失败测试已分析
- [ ] Bug已记录到问题跟踪系统
- [ ] 测试报告已归档
- [ ] 测试环境已清理

---

## 附录B: 常见问题

### Q1: 测试超时怎么办?
**A**: 增加`actionTimeout`和`navigationTimeout`配置,或检查网络/服务器性能。

### Q2: 如何调试失败的测试?
**A**:
1. 使用`npx playwright show-report`查看HTML报告
2. 查看截图和视频
3. 使用`--debug`模式运行:`npx playwright test --debug`

### Q3: 如何处理动态内容?
**A**: 使用`waitForSelector`、`waitForResponse`等等待策略,避免固定延迟。

### Q4: 测试数据如何管理?
**A**: 使用fixtures创建测试数据,每个测试独立,测试后清理。

---

## 规范版本历史

| 版本 | 日期 | 变更说明 | 作者 |
|-----|------|---------|------|
| 1.0 | 2026-02-10 | 初始版本,定义30个GWT | Spec Writer |

---

**规范状态**: ✅ **已完成,等待审查**

**下一步行动**:
1. 发布`spec.ready`事件
2. 等待Spec Critic审查
3. 根据反馈修改规范
4. 获得批准后开始实施
