# Playwright E2E测试规范

**任务ID**: task-1770650819-b957
**阶段**: Phase 3 - E2E测试
**优先级**: P1

## 摘要

为EVIF Web应用编写完整的Playwright端到端测试套件，覆盖所有核心UI功能和用户交互流程，确保前后端集成正常工作。

## 背景

根据scratchpad记录：
- ✅ Phase 1 Backend完成：GlobalHandleManager, 10个REST API端点, TTL清理任务, 4/4单元测试通过
- ✅ Phase 2 UI优化完成：颜色对比度(WCAG AAA 95%), 间距系统(4px/8px网格), 交互反馈, 阴影边框统一
- 🔄 Phase 3: 需要E2E测试验证整个系统功能

## 目标

1. 验证所有核心UI功能正常工作
2. 确保前后端API集成正确
3. 测试用户交互流程的完整性
4. 验证响应式设计和可访问性
5. 为后续开发提供回归测试保护

## 测试覆盖范围

### 1. 文件操作测试 (File Operations)

#### GWT-E2E-1: 创建新文件
**Given** 用户在EVIF Web应用主页
**When** 用户点击"New File"按钮
**Then** 系统应该:
- 创建名为"/local/untitled"的新文件
- 在编辑器中打开该文件
- 显示文件路径在面包屑导航中
- 更新文件列表显示新文件

**API验证**:
- POST /api/v1/files 返回201状态码
- 响应包含文件路径和创建时间戳

#### GWT-E2E-2: 保存文件内容
**Given** 用户在编辑器中打开了一个文件
**When** 用户编辑文件内容并点击"Save"按钮
**Then** 系统应该:
- 发送POST请求到/api/v1/files/:path
- 显示保存成功提示
- 更新文件状态为已保存
- 不显示未保存警告

**API验证**:
- POST /api/v1/files/{path} 返回200状态码
- 响应包含更新后的文件元数据

#### GWT-E2E-3: 打开现有文件
**Given** 文件列表中有可用的文件
**When** 用户点击文件名或使用QuickOpen (Cmd+K)
**Then** 系统应该:
- 在编辑器中加载文件内容
- 显示文件路径和大小信息
- 高亮当前激活的文件
- 更新面包屑导航

**API验证**:
- GET /api/v1/files/{path} 返回200状态码
- 响应包含文件内容和元数据

#### GWT-E2E-4: 删除文件
**Given** 用户选中了一个文件
**When** 用户点击"Delete"按钮并确认操作
**Then** 系统应该:
- 发送DELETE请求到/api/v1/files/{path}
- 从文件列表中移除该文件
- 关闭编辑器标签（如果已打开）
- 显示删除成功通知

**API验证**:
- DELETE /api/v1/files/{path} 返回204状态码

### 2. 搜索功能测试 (Search)

#### GWT-E2E-5: 基本搜索
**Given** 用户在EVIF Web应用主页
**When** 用户在搜索栏输入关键词并点击搜索
**Then** 系统应该:
- 禁用搜索按钮直到输入有效内容
- 发送GET请求到/api/v1/search?q={keyword}
- 在SearchResults组件中显示结果
- 显示找到的文件数量

**API验证**:
- GET /api/v1/search?q={keyword} 返回200状态码
- 响应包含匹配的文件列表

#### GWT-E2E-6: 高级搜索过滤
**Given** 用户打开了FilterPanel
**When** 用户设置文件类型过滤和日期范围
**Then** 系统应该:
- 更新URL查询参数
- 发送带过滤条件的搜索请求
- 显示过滤后的结果数量
- 保持过滤状态在导航时

**API验证**:
- GET /api/v1/search?q={keyword}&type={type}&after={date} 返回200状态码
- 响应只包含符合过滤条件的结果

### 3. 上传功能测试 (Upload)

#### GWT-E2E-7: 拖拽上传
**Given** 用户在EVIF Web应用主页
**When** 用户拖拽文件到UploadDropzone区域
**Then** 系统应该:
- 高亮显示dropzone区域
- 显示拖拽文件的文件名和大小
- 发送POST请求到/api/v1/upload
- 显示上传进度条
- 完成后更新文件列表

**API验证**:
- POST /api/v1/upload 返回201状态码
- 响应包含上传文件的路径

#### GWT-E2E-8: 文件选择上传
**Given** 用户在EVIF Web应用主页
**When** 用户点击"选择文件"按钮并选择文件
**Then** 系统应该:
- 打开文件选择对话框
- 显示选中的文件信息
- 开始上传流程
- 显示上传进度

**UI验证**:
- UploadDropzone显示选中的文件名
- UploadManager显示上传进度

### 4. 监控功能测试 (Monitor)

#### GWT-E2E-9: 查看系统指标
**Given** 用户导航到Monitor页面
**When** MonitorView组件加载完成
**Then** 系统应该:
- 显示CPU使用率图表
- 显示内存使用情况
- 显示磁盘I/O统计
- 显示网络活动指标
- 所有MetricCard显示正确数值和趋势

**API验证**:
- GET /api/v1/metrics 返回200状态码
- 响应包含所有系统指标数据

#### GWT-E2E-10: 告警面板
**Given** 系统产生了告警
**When** 用户查看AlertPanel
**Then** 系统应该:
- 显示告警列表
- 按严重程度（critical/warning/info）分类
- 显示告警时间戳
- 提供告警详情展开功能

**UI验证**:
- AlertPanel组件渲染正常
- 告警图标和颜色正确显示

### 5. 协作功能测试 (Collaboration)

#### GWT-E2E-11: 访问控制列表
**Given** 用户打开ShareModal
**When** 用户查看AccessControlList
**Then** 系统应该:
- 显示所有已授权用户
- 显示每个用户的权限级别
- 允许添加新用户
- 允许修改现有用户权限

**API验证**:
- GET /api/v1/files/{path}/acl 返回200状态码
- 响应包含访问控制列表

#### GWT-E2E-12: 评论功能
**Given** 用户打开CommentPanel
**When** 用户添加新评论
**Then** 系统应该:
- 显示评论输入框
- 提交评论到服务器
- 更新评论列表显示新评论
- 显示评论时间戳和作者

**API验证**:
- POST /api/v1/files/{path}/comments 返回201状态码
- GET /api/v1/files/{path}/comments 返回200状态码

### 6. 插件管理测试 (Plugin Manager)

#### GWT-E2E-13: 查看插件列表
**Given** 用户打开PluginManager
**When** PluginList加载完成
**Then** 系统应该:
- 显示所有已安装插件
- 显示每个插件的状态（启用/禁用）
- 显示插件版本和描述
- 提供启用/禁用切换按钮

**API验证**:
- GET /api/v1/plugins 返回200状态码
- 响应包含插件列表和状态

#### GWT-E2E-14: 插件状态切换
**Given** 用户在PluginManager中
**When** 用户切换插件状态
**Then** 系统应该:
- 发送PATCH请求到/api/v1/plugins/{id}
- 更新PluginStatus显示
- 显示状态变更成功提示

**API验证**:
- PATCH /api/v1/plugins/{id} 返回200状态码
- 响应包含更新后的插件状态

### 7. 全局Handle管理测试 (Global Handle Management)

#### GWT-E2E-15: 打开文件句柄
**Given** 用户想要打开文件进行读写
**When** 系统调用open API
**Then** 系统应该:
- 发送POST请求到/api/v1/handles/open
- 返回唯一的handle ID
- 注册句柄到GlobalHandleManager
- 设置TTL自动清理

**API验证**:
- POST /api/v1/handles/open 返回201状态码
- 响应包含handle_id和初始状态

#### GWT-E2E-16: 读取文件句柄
**Given** 用户有一个打开的文件句柄
**When** 用户读取文件内容
**Then** 系统应该:
- 发送POST请求到/api/v1/handles/{id}/read
- 返回请求的字节数据
- 更新句柄位置指针
- 不关闭句柄（可复用）

**API验证**:
- POST /api/v1/handles/{id}/read 返回200状态码
- 响应包含数据和新的位置

#### GWT-E2E-17: 写入文件句柄
**Given** 用户有一个打开的文件句柄
**When** 用户写入数据
**Then** 系统应该:
- 发送POST请求到/api/v1/handles/{id}/write
- 返回写入的字节数
- 更新文件修改时间

**API验证**:
- POST /api/v1/handles/{id}/write 返回200状态码
- 响应包含写入字节数

#### GWT-E2E-18: 关闭文件句柄
**Given** 用户完成文件操作
**When** 用户关闭句柄
**Then** 系统应该:
- 发送POST请求到/api/v1/handles/{id}/close
- 从GlobalHandleManager注销句柄
- 释放资源

**API验证**:
- POST /api/v1/handles/{id}/close 返回200状态码

#### GWT-E2E-19: 句柄TTL自动清理
**Given** 用户有一个打开的句柄
**When** 句柄超过TTL时间未使用
**Then** 系统应该:
- 自动清理过期句柄
- 更新统计信息
- 释放相关资源

**API验证**:
- GET /api/v1/handles/stats 显示active减少，total可能增加

#### GWT-E2E-20: 列出所有活跃句柄
**Given** 用户想查看当前所有打开的句柄
**When** 用户请求句柄列表
**Then** 系统应该:
- 发送GET请求到/api/v1/handles
- 返回所有活跃句柄的列表
- 包含每个句柄的ID、路径、状态

**API验证**:
- GET /api/v1/handles 返回200状态码
- 响应包含句柄数组

### 8. 响应式设计测试 (Responsive Design)

#### GWT-E2E-21: 移动端视图
**Given** 用户在移动设备（宽度<768px）
**When** 页面加载完成
**Then** 系统应该:
- 显示汉堡菜单按钮
- 隐藏侧边栏
- 调整编辑器宽度为100%
- 触摸目标至少44x44px

**UI验证**:
- viewport宽度<768px时MenuBar显示mobile样式
- 所有交互元素可点击

#### GWT-E2E-22: 平板视图
**Given** 用户在平板设备（768px<=宽度<1024px）
**When** 页面加载完成
**Then** 系统应该:
- 显示简化的侧边栏
- 调整两列布局
- 保持编辑器可用性

**UI验证**:
- viewport在768-1024px时布局正确调整

#### GWT-E2E-23: 桌面视图
**Given** 用户在桌面设备（宽度>=1024px）
**When** 页面加载完成
**Then** 系统应该:
- 显示完整的侧边栏
- 显示多列编辑器布局
- 显示所有面板和工具栏

**UI验证**:
- viewport>=1024px时显示完整布局

### 9. 可访问性测试 (Accessibility)

#### GWT-E2E-24: 键盘导航
**Given** 用户只使用键盘
**When** 用户使用Tab键导航
**Then** 系统应该:
- Tab顺序符合视觉顺序
- 焦点可见（focus ring）
- Enter/Space键激活按钮
- Escape键关闭模态框

**可访问性验证**:
- 所有交互元素可通过键盘访问
- 焦点陷阱在模态框内正常工作

#### GWT-E2E-25: 屏幕阅读器支持
**Given** 用户使用屏幕阅读器
**When** 用户导航页面
**Then** 系统应该:
- 所有图片有alt属性
- 表单元素有关联的label
- 语义化HTML标签
- ARIA属性正确使用

**可访问性验证**:
- axe-core扫描无错误
- WCAG 2.1 AA级别合规

#### GWT-E2E-26: 颜色对比度
**Given** 用户有视觉障碍
**When** 用户查看页面
**Then** 系统应该:
- 所有文本对比度≥4.5:1
- 主要交互元素对比度≥7:1
- 不仅依赖颜色传达信息

**可访问性验证**:
- 所有颜色组合符合WCAG AAA标准（95%）

### 10. 性能测试 (Performance)

#### GWT-E2E-27: 页面加载时间
**Given** 用户首次访问应用
**When** 页面加载
**Then** 系统应该:
- 首次内容绘制(FCP)<1.8秒
- 最大内容绘制(LCP)<2.5秒
- 首次输入延迟(FID)<100ms
- 累积布局偏移(CLS)<0.1

**性能验证**:
- Lighthouse性能分数>=90

#### GWT-E2E-28: 交互响应时间
**Given** 用户与UI交互
**When** 用户点击按钮或输入
**Then** 系统应该:
- 100ms内显示视觉反馈
- 动画帧率>=60fps
- 不阻塞主线程

**性能验证**:
- 所有交互在100ms内有反馈

## 测试工具和配置

### Playwright配置

```typescript
// playwright.config.ts
{
  testDir: './e2e',
  timeout: 30000,
  retries: 2,
  use: {
    baseURL: 'http://localhost:5173',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
    {
      name: 'firefox',
      use: { browserName: 'firefox' },
    },
    {
      name: 'webkit',
      use: { browserName: 'webkit' },
    },
  ],
}
```

### 测试组织结构

```
evif-web/e2e/
├── fixtures/
│   ├── auth.ts          # 认证fixture
│   ├── api.ts           # API mock fixture
│   └── test-data.ts     # 测试数据
├── pages/
│   ├── editor.page.ts   # 编辑器页面对象
│   ├── search.page.ts   # 搜索页面对象
│   ├── upload.page.ts   # 上传页面对象
│   └── monitor.page.ts  # 监控页面对象
├── specs/
│   ├── file-operations.spec.ts
│   ├── search.spec.ts
│   ├── upload.spec.ts
│   ├── monitor.spec.ts
│   ├── collaboration.spec.ts
│   ├── plugin-manager.spec.ts
│   ├── handles.spec.ts
│   ├── responsive.spec.ts
│   ├── accessibility.spec.ts
│   └── performance.spec.ts
└── playwright.config.ts
```

### Page Object Model示例

```typescript
// e2e/pages/editor.page.ts
export class EditorPage {
  readonly page: Page;
  readonly newFileButton: Locator;
  readonly saveButton: Locator;
  readonly editor: Locator;
  readonly breadcrumb: Locator;

  constructor(page: Page) {
    this.page = page;
    this.newFileButton = page.getByRole('button', { name: /new file/i });
    this.saveButton = page.getByRole('button', { name: /save/i });
    this.editor = page.locator('.monaco-editor');
    this.breadcrumb = page.locator('[data-testid="breadcrumb"]');
  }

  async goto() {
    await this.page.goto('/');
  }

  async clickNewFile() {
    await this.newFileButton.click();
  }

  async saveFile() {
    await this.saveButton.click();
  }

  async getEditorContent() {
    return await this.editor.textContent();
  }

  async waitForFileLoaded(filename: string) {
    await this.breadcrumb.waitFor({ state: 'visible' });
    await expect(this.breadcrumb).toContainText(filename);
  }
}
```

## 测试数据

### 测试文件
- 创建临时测试文件：`/tmp/test-file-{random}.txt`
- 测试内容：包含ASCII和Unicode字符
- 文件大小：小文件(<1KB), 中等文件(1KB-1MB), 大文件(>1MB)

### Mock API数据
- 模拟慢响应：延迟响应测试加载状态
- 模拟错误响应：404, 500, 超时
- 模拟大数据：1000+文件列表

## 执行标准

### 测试通过标准
1. 所有E2E测试通过率>=95%
2. 关键路径测试（文件操作、搜索、上传）100%通过
3. 无critical或warning级别可访问性问题
4. 性能指标符合上述GWT-E2E-27和GWT-E2E-28标准

### 测试报告
生成以下报告：
- HTML测试报告（包含截图和视频）
- 可访问性报告（axe-core结果）
- 性能报告（Lighthouse分数）
- API覆盖报告（测试到的端点列表）

## 非功能性需求

### 性能
- 单个测试执行时间<30秒
- 完整测试套件执行时间<10分钟
- 并行执行测试以减少总时间

### 可维护性
- 使用Page Object Model提高可维护性
- 测试代码复用（fixtures, pages）
- 清晰的测试命名和注释

### 可靠性
- 测试重试机制（失败自动重试2次）
- 隔离测试数据（每个测试独立）
- 清理测试环境（测试后删除临时文件）

## 超出范围

以下内容不在本规范范围内：
- 后端单元测试（已在Phase 1完成）
- 前端组件单元测试（需要单独规范）
- 负载测试（需要专门的性能测试工具）
- 安全渗透测试（需要安全专家）

## 依赖

1. 前端开发服务器运行在http://localhost:5173
2. 后端API服务器运行在http://localhost:8080
3. 测试数据库或mock数据准备
4. Playwright浏览器已安装

## 验收标准

本规范验收标准：

### GWT-SPEC-1: 规范完整性
**Given** 开发者阅读本规范
**When** 实施E2E测试
**Then** 应该能够:
- 理解所有测试场景
- 知道如何组织测试代码
- 知道如何验证结果
- 无需额外解释即可实施

### GWT-SPEC-2: GWT覆盖
**Given** 本规范包含28个GWT测试场景
**When** 审查测试覆盖
**Then** 应该:
- 覆盖所有核心UI功能
- 包含API集成验证
- 包含可访问性和性能测试
- 验证响应式设计

### GWT-SPEC-3: 可测试性
**Given** 测试实施者
**When** 编写测试代码
**Then** 每个GWT应该:
- 有明确的前置条件
- 有清晰的触发操作
- 有可验证的预期结果
- 包含API或UI验证步骤

## 下一步

规范完成后：
1. 发布spec.ready事件
2. 等待spec.critic审查
3. 根据反馈修改规范
4. 获得spec.approved后开始实施
