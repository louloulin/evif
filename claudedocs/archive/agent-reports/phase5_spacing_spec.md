# Phase 5 规范: 优化组件间距和布局

## 任务信息
- **任务ID**: task-1770642289-4392
- **优先级**: P2 (高优先级)
- **创建日期**: 2026-02-09
- **状态**: 规范编写完成

---

## 验收标准 (Given-When-Then)

**GIVEN** 任何UI组件
**WHEN** 渲染时
**THEN** 内部间距遵循8px基准网格

**GIVEN** 相邻组件
**WHEN** 排列时
**THEN** 间距使用16px倍数

**GIVEN** 相似类型的组件(按钮、输入框、卡片)
**WHEN** 对比时
**THEN** 使用相同的内间距和外间距

**GIVEN** 用户在移动端设备(<768px)
**WHEN** 查看UI时
**THEN** 间距适当缩小(0.75倍)但保持8px基准

---

## 具体要求

### 1. 间距系统定义

基于8px基准网格的间距系统:

```
间距级别    数值    Tailwind类名    用途
--------------------------------------------
xs         4px     gap-1, p-1      最小组件间距
sm         8px     gap-2, p-2      标准间距基准
md         12px    gap-3, p-3      中等间距
lg         16px    gap-4, p-4      组件间距基准
xl         24px    gap-6, p-6      大组件间距
2xl        32px    gap-8, p-8      模块间距
3xl        48px    gap-12, p-12   章节间距
```

### 2. 组件内间距标准

#### 按钮
```tsx
// 按钮尺寸规范
<Button size="sm">  // className: px-4 py-2 (8px 16px)
<Button size="md">  // className: px-5 py-2.5 (10px 20px)
<Button>            // className: px-6 py-3 (12px 24px)
<Button size="lg">  // className: px-8 py-4 (16px 32px)
<Button size="icon">// className: p-2 (8px)
```

#### 输入框
```tsx
// 输入框padding
<Input>  // className: px-3 py-2 (8px 12px) - 标准
<Input size="lg">  // className: px-4 py-2.5 (10px 16px) - 大尺寸
```

#### 卡片
```tsx
// 卡片padding
<Card>  // className: p-4 (16px) - 标准卡片
<Card className="p-6">  // 24px - 大卡片
<Card className="p-3">  // 12px - 小卡片
```

#### 列表项
```tsx
// 列表项padding
<ListItem>  // className: px-3 py-2 (8px 12px) - 紧凑
<ListItem className="py-3 px-4">  // 12px 16px - 标准
<ListItem className="py-4 px-4">  // 16px 16px - 宽松
```

### 3. 组件间间距标准

#### 垂直间距
```tsx
// 标题到内容
<h2 className="mb-2">标题</h2>  // 8px

// 表单字段之间
<form className="space-y-4">  // 16px

// 卡片之间
<div className="grid gap-4">  // 16px

// 章节之间
<section className="mb-6">  // 24px

// 模块之间
<div className="mb-8">  // 32px
```

#### 水平间距
```tsx
// 相邻按钮
<div className="flex gap-2">  // 8px

// 表单元素
<div className="flex gap-4">  // 16px

// 卡片网格
<div className="grid grid-cols-3 gap-4">  // 16px
```

### 4. 布局尺寸标准

```tsx
// 活动栏
<ActivityBar className="w-12">  // 48px

// 侧边栏
<Sidebar className="w-64">  // 256px (220-300px可调)

// 面板(终端、输出)
<Panel className="h-52">  // 208px (约200px)
```

---

## 需要修改的组件清单

### 1. FileTree.tsx
**当前问题**: padding不一致 (6px 12px)
**修改方案**:
```tsx
// Before
className="px-3 py-1.5"  // 12px 6px

// After
className="px-4 py-2.5"  // 16px 10px (标准)
// 或紧凑模式
className="px-3 py-2"  // 12px 8px
```

### 2. MenuBar.tsx
**当前问题**: gap太小 (4px)
**修改方案**:
```tsx
// Before
className="gap-1"  // 4px

// After
className="gap-2"  // 8px
```

### 3. MonitorView.tsx
**当前问题**: 卡片间距和内部padding不统一
**修改方案**:
```tsx
// MetricCard
className="p-4"  // 16px standard
<div className="grid gap-4">  // 16px grid间距

// 图表容器
className="space-y-4"  // 16px vertical间距
```

### 4. PluginManagerView.tsx
**当前问题**: 卡片和列表间距不统一
**修改方案**:
```tsx
// 插件卡片
className="p-4"  // 16px
<div className="grid gap-4">  // 16px grid间距

// 列表项
className="px-4 py-2"  // 16px 8px
```

### 5. Terminal.tsx
**当前问题**: 容器间距不规范
**修改方案**:
```tsx
// Terminal容器
className="p-4"  // 16px

// 工具栏
className="px-4 py-2"  // 16px 8px
```

### 6. 所有按钮组件
**统一标准**:
```tsx
// 主要操作按钮
<Button className="px-6 py-3">  // 24px 12px

// 次要按钮
<Button variant="outline" className="px-5 py-2.5">  // 20px 10px

// 图标按钮
<Button size="icon" className="p-2">  // 8px
```

### 7. 所有输入框组件
**统一标准**:
```tsx
// 标准输入框
<Input className="px-3 py-2">  // 12px 8px

// 大输入框
<Input className="px-4 py-2.5">  // 16px 10px
```

---

## Tailwind工具类映射

### Padding (p)
```tsx
p-0    0px
p-1    4px     (xs)
p-2    8px     (sm) ✓ 基准
p-3    12px    (md)
p-4    16px    (lg) ✓ 标准
p-5    20px
p-6    24px    (xl)
p-8    32px    (2xl)
p-10   40px
p-12   48px    (3xl)

px-2   8px水平    py-2   8px垂直    ✓ 最小组件
px-3   12px水平   py-3   12px垂直   ✓ 输入框标准
px-4   16px水平   py-4   16px垂直   ✓ 组件标准
px-6   24px水平   py-6   24px垂直   ✓ 大组件
```

### Margin (m)
```tsx
// 用于组件外间距
m-2    8px     (sm)
m-4    16px    (lg)  ✓ 标准
m-6    24px    (xl)
m-8    32px    (2xl)

mb-2   8px下间距     (标题到内容)
mb-4   16px下间距    (表单字段)
mb-6   24px下间距    (章节)
mb-8   32px下间距    (模块)
```

### Gap (用于flex/grid)
```tsx
gap-1   4px     (最小)
gap-2   8px     ✓ 按钮组
gap-3   12px
gap-4   16px    ✓ 卡片网格
gap-6   24px    ✓ 大组件
gap-8   32px    ✓ 模块间距
```

### Height
```tsx
h-12   48px    (按钮标准高度)
h-10   40px    (输入框标准高度)
h-8    32px    (小按钮)
```

---

## 输入示例

### Before: 不一致的间距
```tsx
// 问题: 各种不规范间距值
<div style={{ padding: '6px 12px' }}>文件</div>
<div style={{ marginBottom: '14px' }}>标题</div>
<div style={{ gap: '4px' }}>按钮组</div>
<Card style={{ padding: '20px' }}>卡片</Card>
```

### After: 标准化Tailwind类
```tsx
// 解决: 统一使用8px基准的Tailwind类
<div className="px-3 py-2">文件</div>      // 12px 8px
<div className="mb-2">标题</div>           // 8px
<div className="flex gap-2">按钮组</div>  // 8px
<Card className="p-5">卡片</Card>         // 20px
```

---

## 边界情况

### 1. 紧凑模式
```tsx
// 密集信息展示场景
<div className="p-2">  // 8px (最小可用)
<div className="gap-1">  // 4px (最小可用)
<div className="min-h-[32px]">  // 最小行高
```

### 2. 密集列表(文件树)
```tsx
// 保持可读性的最小间距
<ListItem className="px-4 py-2">  // 16px 8px
// 缩进保持16px的倍数
<Folder className="pl-4">  // 16px
<Folder className="pl-8">  // 32px
<Folder className="pl-12"> // 48px
```

### 3. 触摸优化(移动端)
```tsx
// 最小点击区域 44x44px
<button className="min-h-[44px] px-6 py-3">
<input className="min-h-[44px] px-4 py-3">
```

### 4. 边框补偿
```tsx
// 当元素有边框时,减少1px补偿
<div className="border p-4">  // 视觉上15px因为1px边框
// 解决: 使用border-box盒模型
@layer base {
  *, *::before, *::after {
    box-sizing: border-box;
  }
}
```

---

## 非功能需求

### 性能
- 使用Tailwind静态CSS类,无运行时计算开销
- 避免内联style,使用类名
- CSS Purge将移除未使用的样式

### 可维护性
- 所有间距使用8px基准倍数
- 使用语义化的Tailwind类名
- 统一的组件间距模式

### 可访问性
- 最小点击区域: 44x44px (WCAG标准)
- 间距足够大,避免误触
- 焦点元素有足够padding

### 响应式
```tsx
// 移动端适当缩小
<div className="p-4 md:p-6">  // 移动16px, 桌面24px
<div className="gap-2 md:gap-4">  // 移动8px, 桌面16px
```

---

## 超出范围

以下内容**不在**此任务范围内:

1. **Monaco编辑器**内部样式和间距
2. **xterm.js终端**内部样式
3. 整体布局结构调整(活动栏、侧边栏、主编辑器区域)
4. split-pane分割器的拖拽行为
5. 第三方组件的默认间距(图表库等)
6. 动画和过渡效果(任务3)
7. 阴影和边框样式(任务4)
8. 响应式断点系统(任务5)

---

## 实施检查清单

### 审查阶段
- [ ] 审查所有组件的当前间距
- [ ] 标记不符合8px基准的间距值
- [ ] 记录需要修改的组件清单

### 修改阶段
- [ ] 修改FileTree.tsx间距
- [ ] 修改MenuBar.tsx间距
- [ ] 修改MonitorView.tsx间距
- [ ] 修改PluginManagerView.tsx间距
- [ ] 修改Terminal.tsx间距
- [ ] 统一所有按钮组件padding
- [ ] 统一所有输入框组件padding
- [ ] 统一所有卡片组件padding

### 验证阶段
- [ ] 项目构建成功
- [ ] TypeScript编译无错误
- [ ] 代码审查: 所有间距符合8px基准
- [ ] 视觉检查: UI看起来协调一致

### 测试阶段
- [ ] 手动测试: 检查所有组件间距
- [ ] 移动端测试: 确认触摸目标足够大
- [ ] MCP Playwright: 截图验证UI一致性

---

## 验证方法

### 自动化验证
```bash
# 1. 构建验证
npm run build

# 2. TypeScript检查
npm run typecheck

# 3. 代码审查(搜索不规范间距)
grep -r "style.*padding" src/
grep -r "gap-1\|gap-\[5px\]" src/
```

### 手动验证
- 视觉检查所有组件间距一致
- 测试密集列表的可读性
- 验证触摸目标大小(移动端)

### MCP Playwright验证
```bash
# UI截图对比
# 检查组件间距是否符合规范
```

---

## 规范版本
- **版本**: 1.0
- **创建日期**: 2026-02-09
- **作者**: Ralph (📋 Spec Writer Hat)
- **状态**: 待审查

---

## 附录: 间距速查表

```
4px   (xs)  - 图标、最小间距
8px   (sm)  - ✓ 基准单位、按钮组gap
12px  (md)  - 输入框、表单字段
16px  (lg)  - ✓ 标准间距、卡片padding、组件gap
20px  -     中大按钮padding
24px  (xl)  - 大组件间距、章节间距
32px  (2xl) - 模块间距
48px  (3xl) - 章节标题间距

最小触摸目标: 44x44px (移动端)
文件树缩进: 16px倍数 (16, 32, 48px)
```
