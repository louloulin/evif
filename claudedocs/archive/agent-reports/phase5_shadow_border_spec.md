# Phase 5 规范: 统一阴影和边框样式

## 任务信息
- **任务ID**: task-1770642291-9c69
- **优先级**: P2 (高优先级)
- **创建日期**: 2026-02-09
- **状态**: 规范编写完成

---

## 验收标准 (Given-When-Then)

**GIVEN** 任何可交互元素(按钮、卡片、输入框)
**WHEN** 渲染时
**THEN** 阴影级别与视觉层次匹配

**GIVEN** 卡片、按钮、输入框
**WHEN** 悬停时
**THEN** 阴影级别提升一级

**GIVEN** 任何UI元素
**WHEN** 需要边框时
**THEN** 使用标准化的4级边框样式(subtle/standard/hover/strong)

**GIVEN** 所有圆角元素
**WHEN** 渲染时
**THEN** 使用标准化的5级圆角系统(sm/md/lg/xl/full)

---

## 具体要求

### 1. 阴影系统定义 (8级)

基于shadcn/ui的阴影系统,从最subtle到最prominent:

```css
/* 阴影级别定义 (已在index.css中实现) */
--shadow-xs:   0 1px 2px 0 rgb(0 0 0 / 0.1);              /* 最轻微阴影 */
--shadow-sm:   0 1px 3px 0 rgb(0 0 0 / 0.15),              /* 小阴影 */
                0 1px 2px -1px rgb(0 0 0 / 0.1);
--shadow:      0 2px 4px -1px rgb(0 0 0 / 0.15),           /* 标准阴影 */
                0 4px 6px -2px rgb(0 0 0 / 0.1);
--shadow-md:   0 4px 8px -2px rgb(0 0 0 / 0.18),           /* 中等阴影 */
                0 2px 6px -2px rgb(0 0 0 / 0.12);
--shadow-lg:   0 10px 18px -3px rgb(0 0 0 / 0.2),          /* 大阴影 */
                0 4px 8px -4px rgb(0 0 0 / 0.15);
--shadow-xl:   0 20px 28px -5px rgb(0 0 0 / 0.22),         /* 超大阴影 */
                0 8px 14px -6px rgb(0 0 0 / 0.18);
--shadow-2xl:  0 25px 50px -12px rgb(0 0 0 / 0.35);        /* 极大阴影 */
--shadow-inner: inset 0 2px 6px 0 rgb(0 0 0 / 0.08);      /* 内阴影 */
```

#### Tailwind工具类映射
```tsx
className="shadow-xs"    // 最轻微
className="shadow-sm"    // 小阴影
className="shadow"       // 标准阴影
className="shadow-md"    // 中等阴影
className="shadow-lg"    // 大阴影
className="shadow-xl"    // 超大阴影
className="shadow-2xl"   // 极大阴影
className="shadow-inner" // 内阴影
```

#### 特殊阴影组合
```tsx
// 卡片阴影 (已在index.css中定义)
className="shadow-card"        // = shadow-sm
className="shadow-card-hover"  // = shadow-md (悬停时)

// 下拉菜单阴影 (带边框)
className="shadow-dropdown"    // = shadow-lg + 1px边框

// 模态框阴影 (带边框)
className="shadow-modal"       // = shadow-2xl + 1px边框

// 内阴影 (用于凹槽效果)
className="shadow-inset"       // = shadow-inner

// 发光效果 (用于焦点、强调)
className="shadow-glow-sm"     // 8px primary发光
className="shadow-glow"        // 16px primary发光
className="shadow-glow-lg"     // 24px primary发光
```

---

### 2. 阴影使用规范

#### 按钮阴影
```tsx
// 主要按钮 - 无阴影或轻微阴影
<Button className="shadow-sm">         // 标准状态
<Button className="shadow-md">         // 悬停状态 (提升一级)

// 次要按钮/轮廓按钮 - 无阴影
<Button variant="outline">            // 无阴影

// 图标按钮 - 无阴影或悬停时轻微阴影
<IconButton className="hover:shadow-sm">
```

#### 卡片阴影
```tsx
// 标准卡片
<Card className="shadow-card">         // shadow-sm
<Card className="shadow-card hover:shadow-card-hover">  // 悬停提升

// 重要卡片(需要突出)
<Card className="shadow-md">          // 中等阴影
<Card className="shadow-md hover:shadow-lg">           // 悬停提升

// 浮动卡片(下拉菜单、弹出层)
<Card className="shadow-dropdown">    // shadow-lg + 边框
```

#### 输入框阴影
```tsx
// 标准输入框 - 无阴影
<Input className="border">            // 仅边框

// 焦点状态 - 内阴影或发光
<Input className="focus:shadow-inset">           // 凹槽效果
<Input className="focus:shadow-glow-sm">         // 发光效果

// 错误状态 - 红色发光
<Input className="focus:shadow-glow-sm destructive">
```

#### 模态框/对话框阴影
```tsx
// 标准模态框
<Dialog className="shadow-modal">       // shadow-2xl + 边框

// 小弹出框/提示
<Popover className="shadow-lg">         // shadow-lg
```

#### 侧边栏/面板阴影
```tsx
// 侧边栏 - 无阴影或轻微阴影
<Sidebar className="border">           // 仅边框

// 浮动面板(移动端侧边栏)
<Sidebar className="shadow-2xl">       // 移动端全屏时
```

---

### 3. 边框系统定义 (4级)

基于现有的CSS变量系统,标准化边框颜色:

```css
/* 边框级别定义 (已在index.css中实现) */
--border:         217 33% 22%;   /* 标准边框 - 16% 亮度 */
--border-subtle:  217 33% 16%;   /* 弱边框 - 12% 亮度 */
--border-hover:   217 33% 28%;   /* 悬停边框 - 20% 亮度 */
--border-strong:  217 33% 34%;   /* 强边框 - 24% 亮度 */
```

#### Tailwind工具类映射
```tsx
// 标准边框 (已在index.css中定义)
className="border"              // 1px solid hsl(var(--border))
className="border-subtle"       // 1px solid hsl(var(--border-subtle))
className="border-hover"        // 1px solid hsl(var(--border-hover))
className="border-strong"       // 1px solid hsl(var(--border-strong))

// 原子边框类 (Tailwind内置)
className="border"              // 添加边框
className="border-x"            // 左右边框
className="border-y"            // 上下边框
className="border-t"            // 顶部边框
className="border-r"            // 右侧边框
className="border-b"            // 底部边框
className="border-l"            // 左侧边框
```

#### 边框宽度
```tsx
className="border"              // 1px (标准)
className="border-2"            // 2px (强调)
className="border-4"            // 4px (重强调)
className="border-0"            // 无边框
```

---

### 4. 边框使用规范

#### 标准边框使用场景
```tsx
// 输入框、卡片 - 标准边框
<Input className="border">                    // hsl(var(--border))
<Card className="border">                    // hsl(var(--border))

// 分隔线 - 弱边框
<hr className="border-subtle">               // hsl(var(--border-subtle))
<div className="border-b border-subtle">

// 悬停/焦点状态 - hover边框
<Button className="border hover:border-hover">
<Input className="focus:border-hover">

// 强调元素 - strong边框
<Alert className="border-strong">
<div className="border-l-2 border-primary">  // 左侧强调边框
```

#### 边框与阴影组合
```tsx
// 下拉菜单 - 边框 + 阴影
<Dropdown className="border shadow-dropdown">

// 模态框 - 边框 + 阴影
<Dialog className="border shadow-modal">

// 卡片 - 边框 + 阴影
<Card className="border shadow-card">
```

---

### 5. 圆角系统定义 (5级)

基于现有的CSS变量,标准化圆角半径:

```css
/* 圆角级别定义 (已在index.css中实现) */
--radius-sm:   4px;    /* 小圆角 - 按钮、徽章 */
--radius:      6px;    /* 标准圆角 - 卡片、输入框 */
--radius-md:   8px;    /* 中等圆角 - 大卡片 */
--radius-lg:   12px;   /* 大圆角 - 模态框 */
--radius-xl:   16px;   /* 超大圆角 - 特殊容器 */
```

#### Tailwind工具类映射
```tsx
className="rounded-sm"    // 4px (小圆角)
className="rounded"       // 6px (标准圆角)
className="rounded-md"    // 8px (中等圆角)
className="rounded-lg"    // 12px (大圆角)
className="rounded-xl"    // 16px (超大圆角)
className="rounded-full"  // 9999px (完全圆形 - 头像、徽章)

// 方向性圆角
className="rounded-t"     // 顶部圆角
className="rounded-r"     // 右侧圆角
className="rounded-b"     // 底部圆角
className="rounded-l"     // 左侧圆角
className="rounded-tl"    // 左上圆角
className="rounded-tr"    // 右上圆角
className="rounded-bl"    // 左下圆角
className="rounded-br"    // 右下圆角
```

#### 圆角使用场景
```tsx
// 按钮 - 小圆角或标准圆角
<Button className="rounded-sm">     // 4px
<Button className="rounded">        // 6px

// 输入框 - 标准圆角
<Input className="rounded">         // 6px

// 卡片 - 标准圆角或中等圆角
<Card className="rounded">          // 6px
<Card className="rounded-md">       // 8px

// 模态框 - 大圆角
<Dialog className="rounded-lg">     // 12px

// 头像、徽章 - 完全圆形
<Avatar className="rounded-full">
<Badge className="rounded-full">
```

---

## 需要修改的组件清单

### 1. Button组件 (统一阴影和圆角)
**当前问题**: 阴影使用不一致,圆角不统一
**修改方案**:
```tsx
// Before - 不一致的阴影
<Button style={{ boxShadow: '0 2px 4px rgba(0,0,0,0.1)' }}>
<Button style={{ borderRadius: '4px' }}>

// After - 标准化Tailwind类
<Button className="shadow-sm hover:shadow-md rounded-sm">
<Button className="shadow-md hover:shadow-lg rounded">
```

### 2. Card组件 (统一样式)
**当前问题**: App.css中定义了.card类,但没有标准化阴影
**修改方案**:
```tsx
// Before
<Card className="shadow" style={{ borderRadius: 'var(--radius)' }}>

// After
<Card className="shadow-card hover:shadow-card-hover rounded">
```

### 3. Input组件 (焦点阴影)
**当前问题**: 焦点状态阴影不统一
**修改方案**:
```tsx
// Before
<Input className="focus:ring-2">

// After
<Input className="rounded focus:shadow-inset">
// 或
<Input className="rounded focus:shadow-glow-sm">
```

### 4. Dialog/Modal组件 (阴影和边框)
**当前问题**: 模态框阴影不统一,缺少边框
**修改方案**:
```tsx
// Before
<Dialog style={{ boxShadow: '0 25px 50px -12px rgba(0,0,0,0.35)' }}>

// After
<Dialog className="shadow-modal border rounded-lg">
```

### 5. Dropdown/ContextMenu组件 (阴影和边框)
**当前问题**: 下拉菜单阴影不统一
**修改方案**:
```tsx
// Before
<Dropdown style={{ boxShadow: '0 10px 18px rgba(0,0,0,0.2)' }}>

// After
<Dropdown className="shadow-dropdown border rounded-md">
```

### 6. 所有边框样式 (统一使用边框级别)
**当前问题**: 直接使用HSL颜色值,不统一
**修改方案**:
```tsx
// Before
<div style={{ border: '1px solid hsl(217, 33%, 22%)' }}>
<div style={{ borderColor: 'hsl(217, 33%, 28%)' }}>

// After
<div className="border">
<div className="border hover:border-hover">
```

### 7. 所有圆角样式 (统一使用圆角级别)
**当前问题**: 混用px值和CSS变量
**修改方案**:
```tsx
// Before
<div style={{ borderRadius: '4px' }}>
<div style={{ borderRadius: 'var(--radius)' }}>
<div style={{ borderRadius: '9999px' }}>

// After
<div className="rounded-sm">
<div className="rounded">
<div className="rounded-full">
```

---

## 输入示例

### Before: 不一致的阴影和边框
```tsx
// 问题: 各种自定义阴影值
<Card style={{ boxShadow: '0 2px 4px rgba(0,0,0,0.15)' }}>
<Button style={{ boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
<Dialog style={{ boxShadow: '0 25px 50px -12px rgba(0,0,0,0.35)' }}>

// 问题: 各种自定义边框颜色
<div style={{ border: '1px solid hsl(217, 33%, 22%)' }}>
<input style={{ borderColor: 'hsl(217, 33%, 28%)' }}>

// 问题: 各种自定义圆角值
<Button style={{ borderRadius: '4px' }}>
<Card style={{ borderRadius: '6px' }}>
<Avatar style={{ borderRadius: '9999px' }}>
```

### After: 标准化Tailwind类
```tsx
// 解决: 使用8级阴影系统
<Card className="shadow-card hover:shadow-card-hover">
<Button className="shadow-sm hover:shadow-md">
<Dialog className="shadow-modal">

// 解决: 使用4级边框系统
<div className="border">
<input className="border focus:border-hover">

// 解决: 使用5级圆角系统
<Button className="rounded-sm">
<Card className="rounded">
<Avatar className="rounded-full">
```

---

## 边界情况

### 1. 无阴影场景
```tsx
// 平面设计风格、密集UI
<Button className="border">              // 仅边框,无阴影
<Card className="border">               // 仅边框,无阴影
```

### 2. 禁用状态
```tsx
// 禁用元素 - 无阴影或极轻微阴影
<Button disabled className="opacity-50">
<Input disabled className="opacity-50">
```

### 3. 深色模式
```tsx
// 阴影在深色模式下需要调整
// 使用现有的CSS变量系统(已支持深色模式)
--shadow: 0 2px 4px -1px rgb(0 0 0 / 0.15);  // 浅色模式
--shadow: 0 2px 4px -1px rgb(0 0 0 / 0.5);   // 深色模式(更透明)
```

### 4. 移动端触摸反馈
```tsx
// 触摸时阴影消失或加强
<button className="active:shadow-none">     // 触摸时阴影消失
<button className="active:shadow-inner">    // 触摸时内阴影(按下效果)
```

---

## 非功能需求

### 性能
- 使用Tailwind静态CSS类,无运行时计算开销
- 避免内联style,使用类名
- 使用CSS变量,便于主题切换

### 可维护性
- 所有阴影使用8级系统
- 所有边框使用4级颜色系统
- 所有圆角使用5级半径系统
- 使用语义化的Tailwind类名

### 可访问性
- 焦点状态有清晰的视觉反馈(阴影或发光)
- 禁用状态有明确的视觉区分(无阴影或降低透明度)
- 阴影不影响文本可读性

### 响应式
```tsx
// 移动端减少阴影强度
<Card className="shadow-sm md:shadow-card">
<Dialog className="shadow-lg md:shadow-modal">
```

---

## 超出范围

以下内容**不在**此任务范围内:

1. Monaco编辑器内部样式
2. xterm.js终端内部样式
3. 动画和过渡效果(任务3)
4. 组件间距(任务2)
5. 颜色对比度(任务1)
6. 响应式断点系统(任务5)
7. 第三方组件的默认样式(图表库等)

---

## 实施检查清单

### 审查阶段
- [ ] 审查所有组件的当前阴影样式
- [ ] 审查所有组件的当前边框样式
- [ ] 审查所有组件的当前圆角样式
- [ ] 记录需要修改的组件清单

### 修改阶段
- [ ] 统一所有按钮组件的阴影和圆角
- [ ] 统一所有卡片组件的阴影、边框和圆角
- [ ] 统一所有输入框组件的焦点阴影和边框
- [ ] 统一所有模态框组件的阴影、边框和圆角
- [ ] 统一所有下拉菜单组件的阴影、边框和圆角
- [ ] 替换所有内联阴影style为Tailwind类
- [ ] 替换所有内联边框style为Tailwind类
- [ ] 替换所有内联圆角style为Tailwind类

### 验证阶段
- [ ] 项目构建成功
- [ ] TypeScript编译无错误
- [ ] 代码审查: 所有阴影符合8级系统
- [ ] 代码审查: 所有边框符合4级系统
- [ ] 代码审查: 所有圆角符合5级系统

### 测试阶段
- [ ] 手动测试: 检查所有组件阴影效果
- [ ] 手动测试: 检查悬停状态阴影提升
- [ ] 手动测试: 检查焦点状态阴影/发光
- [ ] 移动端测试: 确认触摸反馈正常
- [ ] 深色模式测试: 确认阴影效果正确
- [ ] MCP Playwright: 截图验证UI一致性

---

## 验证方法

### 自动化验证
```bash
# 1. 构建验证
npm run build

# 2. TypeScript检查
npm run typecheck

# 3. 代码审查(搜索不规范样式)
# 搜索内联阴影
grep -r "style.*boxShadow" src/
grep -r "style.*shadow" src/

# 搜索内联边框
grep -r "style.*border" src/
grep -r "borderColor.*hsl" src/

# 搜索内联圆角
grep -r "style.*borderRadius" src/
```

### 手动验证
- 视觉检查所有组件阴影一致
- 测试悬停状态阴影提升效果
- 验证焦点状态阴影/发光效果
- 检查边框颜色层次是否清晰
- 检查圆角大小是否协调

### MCP Playwright验证
```bash
# UI截图对比
# 检查组件阴影、边框、圆角是否符合规范
```

---

## 规范版本
- **版本**: 1.0
- **创建日期**: 2026-02-09
- **作者**: Ralph (📋 Spec Writer Hat)
- **状态**: 待审查

---

## 附录: 阴影/边框/圆角速查表

### 阴影速查表
```
shadow-xs      最轻微阴影  - 提示、徽章
shadow-sm      小阴影      - 卡片标准状态
shadow         标准阴影    - 通用元素
shadow-md      中等阴影    - 按钮悬停、重要卡片
shadow-lg      大阴影      - 下拉菜单、浮动面板
shadow-xl      超大阴影    - 大型弹出层
shadow-2xl     极大阴影    - 模态框
shadow-inner   内阴影      - 凹槽效果、焦点状态
```

### 边框速查表
```
border         标准边框    - 输入框、卡片
border-subtle  弱边框      - 分隔线
border-hover   悬停边框    - 悬停/焦点状态
border-strong  强边框      - 警告、强调元素
```

### 圆角速查表
```
rounded-sm     4px  - 小按钮、徽章
rounded        6px  - 标准按钮、输入框、卡片
rounded-md     8px  - 大卡片
rounded-lg     12px - 模态框
rounded-xl     16px - 特殊容器
rounded-full   完全圆形 - 头像、徽章
```

### 常用组合
```
卡片:      shadow-card hover:shadow-card-hover rounded
按钮:      shadow-sm hover:shadow-md rounded-sm
输入框:    rounded focus:shadow-inset
模态框:    shadow-modal border rounded-lg
下拉菜单:  shadow-dropdown border rounded-md
```
