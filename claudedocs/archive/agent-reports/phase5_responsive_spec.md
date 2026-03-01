# Phase 5 Task 5: 优化响应式设计规范

**任务ID**: task-1770642291-4a5e
**优先级**: P2
**Phase**: 5
**创建日期**: 2026-02-09
**参考**: shadcn/ui 响应式设计最佳实践

---

## 规范概述

优化EVIF Web应用的响应式设计系统,统一断点、间距、字体和交互模式,遵循shadcn/ui风格和WCAG 2.1可访问性标准。

---

## 任务1: 统一Tailwind断点系统 (P2)

### 目标
将自定义CSS媒体查询迁移到Tailwind标准断点,确保一致性。

### 验收标准

#### 1. ✅ GIVEN Tailwind配置文件 WHEN 读取断点配置 THEN 使用标准Tailwind断点
**期望结果**:
```javascript
// tailwind.config.js
screens: {
  'sm': '640px',   // 移动端横屏
  'md': '768px',   // 平板竖屏
  'lg': '1024px',  // 平板横屏/小桌面
  'xl': '1280px',  // 桌面
  '2xl': '1536px', // 大屏
}
```

#### 2. ✅ GIVEN 任何响应式样式 WHEN 编写时 THEN 使用Tailwind类而非自定义媒体查询
**错误示例**:
```css
/* ❌ 不要使用 */
@media (max-width: 767px) {
  .sidebar { width: 85vw; }
}
```

**正确示例**:
```tsx
/* ✅ 使用Tailwind类 */
<div className="w-full md:w-64 lg:w-72 xl:w-80">
```

#### 3. ✅ GIVEN 需要响应式的组件 WHEN 渲染时 THEN 使用Tailwind响应式前缀
**示例**:
```tsx
// 移动端: 1列, 平板: 2列, 桌面: 3列
<div className="grid-cols-1 md:grid-cols-2 lg:grid-cols-3">

// 移动端: 小字体, 桌面: 正常字体
<h1 className="text-xl md:text-2xl lg:text-3xl">
```

### 具体要求

#### 断点映射表

| 断点 | 屏幕宽度 | 设备类型 | 使用场景 |
|------|----------|----------|----------|
| `sm` | ≥640px | 移动端横屏 | 小优化 |
| `md` | ≥768px | 平板竖屏 | 中等布局 |
| `lg` | ≥1024px | 平板横屏/小桌面 | 标准布局 |
| `xl` | ≥1280px | 桌面 | 大布局 |
| `2xl` | ≥1536px | 大屏 | 超大布局 |

#### 移除自定义媒体查询

**需要移除的CSS**:
```css
/* ❌ 移除这些 */
@media (max-width: 767px) { ... }
@media (min-width: 768px) and (max-width: 1023px) { ... }
@media (min-width: 1024px) { ... }
@media (min-width: 1536px) { ... }
```

**替换为Tailwind响应式类**:
```tsx
// 移动端优先
<div className="text-sm md:text-base lg:text-lg">

// 桌面优先
<div className="hidden md:block">
```

### 输入示例

#### Before (自定义CSS)
```css
@media (max-width: 767px) {
  .sidebar {
    width: 85vw;
    max-width: 320px;
  }
}

@media (min-width: 768px) and (max-width: 1023px) {
  .sidebar {
    width: 220px;
  }
}

@media (min-width: 1024px) {
  .sidebar {
    width: 260px;
  }
}
```

#### After (Tailwind类)
```tsx
<div className="
  w-[85vw] max-w-[320px]
  md:w-[220px]
  lg:w-[260px]
  xl:w-[300px]
">
```

### 边界情况

1. **超小屏幕 (<375px)**: 保持全宽,添加最小宽度保护
2. **超宽屏 (>1920px)**: 使用 `max-w-screen-2xl` 限制内容宽度
3. **横竖屏切换**: 使用 `orientation` 媒体查询
4. **高DPI屏幕**: 使用 `@media (min-resolution: 2dppx)`

### 非功能需求

- **性能**: 避免过多的响应式类(每个元素不超过5个断点)
- **可维护性**: 使用语义化的断点名称
- **兼容性**: 支持iOS Safari, Chrome, Firefox, Edge最新版本

### 超出范围

- 打印样式(留待未来Phase)
- 特定浏览器hack(如旧版IE)
- 容器查询(实验性功能)

---

## 任务2: 响应式间距系统 (P2)

### 目标
创建统一的响应式间距工具类,遵循8px基准网格。

### 验收标准

#### 1. ✅ GIVEN 任何容器 WHEN 设置内边距时 THEN 使用响应式间距类
**示例**:
```tsx
// 移动端: 16px, 平板: 24px, 桌面: 32px
<div className="p-4 md:p-6 lg:p-8">
```

#### 2. ✅ GIVEN 相邻元素 WHEN 设置间距时 THEN 使用响应式gap类
**示例**:
```tsx
// 移动端: 8px, 平板: 16px, 桌面: 24px
<div className="flex gap-2 md:gap-4 lg:gap-6">
```

#### 3. ✅ GIVEN 任何组件 WHEN 设置外边距时 THEN 使用响应式margin类
**示例**:
```tsx
// 移动端: 8px, 桌面: 16px
<div className="mb-2 md:mb-4">
```

### 具体要求

#### 响应式间距对照表

| 类名 | 移动端 | 平板 | 桌面 | 大屏 |
|------|--------|------|------|------|
| padding | `p-4` (16px) | `md:p-6` (24px) | `lg:p-8` (32px) | `xl:p-10` (40px) |
| gap | `gap-2` (8px) | `md:gap-4` (16px) | `lg:gap-6` (24px) | `xl:gap-8` (32px) |
| margin | `m-2` (8px) | `md:m-4` (16px) | `lg:m-6` (24px) | `xl:m-8` (32px) |

#### 间距应用场景

1. **卡片内边距**: `p-4 md:p-6 lg:p-8`
2. **列表间距**: `space-y-2 md:space-y-4 lg:space-y-6`
3. **按钮间距**: `gap-2 md:gap-3 lg:gap-4`
4. **表单间距**: `space-y-3 md:space-y-4 lg:space-y-5`

### 输入示例

```tsx
// 卡片组件
<Card className="p-4 md:p-6 lg:p-8">
  <CardContent className="space-y-2 md:space-y-4 lg:space-y-6">
    {/* 内容 */}
  </CardContent>
</Card>

// 按钮组
<div className="flex gap-2 md:gap-3 lg:gap-4">
  <Button>取消</Button>
  <Button>确认</Button>
</div>
```

### 边界情况

1. **零间距**: `p-0 m-0 gap-0` (所有断点一致)
2. **负间距**: `-m-2 md:-m-4` (谨慎使用)
3. **不对称间距**: `px-4 py-2 md:px-6 md:py-3`

---

## 任务3: 响应式字体系统 (P2)

### 目标
创建统一的响应式字体系统,确保可读性和层次感。

### 验收标准

#### 1. ✅ GIVEN 任何文本 WHEN 设置字体大小时 THEN 使用响应式text类
**示例**:
```tsx
// 移动端: 18px, 平板: 20px, 桌面: 24px
<h1 className="text-xl md:text-2xl lg:text-3xl">
```

#### 2. ✅ GIVEN 标题层次 WHEN 设置时 THEN 使用响应式heading类
**示例**:
```tsx
// H1: 移动30px → 桌面36px → 大屏48px
<h1 className="text-3xl md:text-4xl lg:text-5xl xl:text-6xl">

// H2: 移动24px → 桌面30px → 大屏36px
<h2 className="text-2xl md:text-3xl lg:text-4xl">
```

#### 3. ✅ GIVEN 正文文本 WHEN 设置时 THEN 使用响应式body类
**示例**:
```tsx
// 移动: 14px, 平板: 15px, 桌面: 16px
<p className="text-sm md:text-base lg:text-lg">
```

### 具体要求

#### 响应式字体对照表

| 元素 | 移动端 | 平板 | 桌面 | 大屏 |
|------|--------|------|------|------|
| H1 | `text-3xl` (30px) | `md:text-4xl` (36px) | `lg:text-5xl` (48px) | `xl:text-6xl` (60px) |
| H2 | `text-2xl` (24px) | `md:text-3xl` (30px) | `lg:text-4xl` (36px) | `xl:text-5xl` (48px) |
| H3 | `text-xl` (20px) | `md:text-2xl` (24px) | `lg:text-3xl` (30px) | `xl:text-4xl` (36px) |
| Body | `text-sm` (14px) | `md:text-base` (16px) | `lg:text-lg` (18px) | - |
| Small | `text-xs` (12px) | `md:text-sm` (14px) | - | - |

#### 行高响应式

```tsx
// 移动端: 1.5, 桌面: 1.6
<p className="leading-relaxed md:leading-loose">
```

### 输入示例

```tsx
// 文章页面
<article>
  <h1 className="text-3xl md:text-4xl lg:text-5xl font-bold mb-4 md:mb-6 lg:mb-8">
    标题
  </h1>
  <p className="text-sm md:text-base lg:text-lg leading-relaxed">
    正文内容...
  </p>
</article>
```

### 边界情况

1. **最小字体**: 不小于 `text-xs` (12px),符合WCAG可读性
2. **最大字体**: 不超过 `text-6xl` (60px),避免过度缩放
3. **等宽字体**: 代码字体不缩放,保持 `text-xs` 或 `text-sm`

---

## 任务4: 响应式显示/隐藏工具 (P2)

### 目标
添加标准化的响应式显示控制类,替代自定义CSS。

### 验收标准

#### 1. ✅ GIVEN 需要隐藏的元素 WHEN 在移动端时 THEN 使用hidden类
**示例**:
```tsx
// 移动端隐藏,桌面显示
<div className="hidden md:block">
```

#### 2. ✅ GIVEN 需要隐藏的元素 WHEN 在桌面端时 THEN 使用响应式hidden类
**示例**:
```tsx
// 桌面隐藏,移动显示
<div className="block md:hidden">
```

#### 3. ✅ GIVEN 条件渲染元素 WHEN 响应式显示时 THEN 使用逻辑与而非CSS隐藏
**示例**:
```tsx
// ❌ 不推荐
<div className="hidden md:block">{children}</div>

// ✅ 推荐
{isMobile ? null : <div>{children}</div>}
```

### 具体要求

#### 响应式显示类对照表

| 类名 | 移动端 | 平板 | 桌面 | 大屏 |
|------|--------|------|------|------|
| `hidden md:block` | 隐藏 | 隐藏 | 显示 | 显示 |
| `block md:hidden` | 显示 | 显示 | 隐藏 | 隐藏 |
| `hidden lg:block` | 隐藏 | 隐藏 | 隐藏 | 显示 |
| `block lg:hidden` | 显示 | 显示 | 显示 | 隐藏 |

#### 常用场景

1. **移动端菜单按钮**: `block md:hidden`
2. **桌面侧边栏**: `hidden md:block`
3. **移动端简化UI**: `hidden lg:block`
4. **桌面增强功能**: `hidden md:block`

### 输入示例

```tsx
// 移动端菜单按钮
<button className="block md:hidden lg:hidden xl:hidden">
  <MenuIcon />
</button>

// 桌面导航栏
<nav className="hidden md:block lg:block">
  {/* 导航链接 */}
</nav>

// 响应式布局
<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
  <Card className="block md:hidden">移动专属</Card>
  <Card className="hidden md:block">桌面专属</Card>
</div>
```

### 边界情况

1. **动画过渡**: 使用 `animate-in fade-in` 代替直接切换
2. **打印隐藏**: `print:hidden` (预留)
3. **焦点陷阱**: 隐藏时移除 `tabindex`

---

## 任务5: 移动端触摸优化 (P2)

### 目标
优化移动端触摸体验,遵循WCAG 2.1级别AA标准。

### 验收标准

#### 1. ✅ GIVEN 任何可点击元素 WHEN 在移动端时 THEN 最小触摸区域44x44px
**示例**:
```tsx
// 按钮(默认已满足)
<Button className="min-h-[44px] min-w-[44px]">

// 自定义元素
<div className="p-3 cursor-pointer min-h-[44px]">
```

#### 2. ✅ GIVEN 触摸反馈 WHEN 用户点击时 THEN 使用active状态样式
**示例**:
```tsx
<button className="active:scale-95 active:bg-accent">
  点击
</button>
```

#### 3. ✅ GIVEN 表单输入 WHEN 在移动端时 THEN 增加内边距到12px
**示例**:
```tsx
<input className="px-3 py-2 md:px-4 md:py-3">
```

### 具体要求

#### 触摸目标大小

| 元素类型 | 最小尺寸 | 推荐尺寸 | 类名 |
|---------|----------|----------|------|
| 按钮 | 44x44px | 48x48px | `h-11 w-11 md:h-12 md:w-12` |
| 链接 | 44x44px | 44x44px | `min-h-[44px] min-w-[44px]` |
| 图标按钮 | 44x44px | 48x48px | `p-3 md:p-4` |
| 复选框 | 44x44px | 48x48px | `h-11 w-11` |
| 单选框 | 44x44px | 48x48px | `h-11 w-11` |

#### 触摸反馈样式

```tsx
// 缩放反馈
<button className="active:scale-95 transition-transform">

// 背景反馈
<button className="active:bg-accent-hover">

// 边框反馈
<button className="active:border-primary">
```

#### 间距优化

```tsx
// 防止误触: 增加间距
<div className="space-y-4 md:space-y-6">

// 增大点击区域: 使用padding而非width/height
<button className="px-6 py-3 md:px-8 md:py-4">
```

### 输入示例

```tsx
// 按钮组(移动优化)
<div className="flex gap-3 md:gap-4">
  <Button className="h-11 px-6 md:h-12 md:px-8 active:scale-95">
    取消
  </Button>
  <Button className="h-11 px-6 md:h-12 md:px-8 active:scale-95">
    确认
  </Button>
</div>

// 图标按钮
<button className="p-3 md:p-4 min-h-[44px] min-w-[44px] active:bg-accent rounded-md">
  <Icon className="w-6 h-6" />
</button>
```

### 边界情况

1. **密集列表**: 使用 `min-h-[48px]` 增加行高
2. **小屏幕**: <375px时保持44px最小值
3. **横屏模式**: 保持相同触摸目标大小
4. **键盘导航**: 保留 `:focus-visible` 样式

---

## 任务6: 横屏模式优化 (P3)

### 目标
优化移动设备横屏模式下的布局和交互。

### 验收标准

#### 1. ✅ GIVEN 移动设备横屏 WHEN 渲染布局时 THEN 调整为双列布局
**示例**:
```tsx
// 竖屏: 1列, 横屏: 2列
<div className="grid grid-cols-1 landscape:grid-cols-2">
```

#### 2. ✅ GIVEN 横屏模式 WHEN 显示侧边栏时 THEN 缩小宽度到20vw
**示例**:
```tsx
<aside className="w-[85vw] landscape:w-[20vw]">
```

#### 3. ✅ GIVEN 横屏视频内容 WHEN 渲染时 THEN 全屏显示
**示例**:
```tsx
<video className="w-full h-full landscape:object-cover">
```

### 具体要求

#### 横屏断点

```javascript
// tailwind.config.js
screens: {
  'landscape': 'landscape orientation', // 需要插件
}
```

**替代方案**(使用自定义CSS):
```css
@supports (orientation: landscape) {
  .landscape\:grid-cols-2 {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
```

#### 横屏优化场景

1. **表单**: 双列布局
2. **卡片网格**: 2列变为3列
3. **侧边栏**: 固定宽度而非百分比
4. **视频**: 全屏显示

### 输入示例

```tsx
// 表单(横屏双列)
<form className="grid grid-cols-1 gap-4 landscape:grid-cols-2 landscape:gap-6">
  <Input label="姓名" />
  <Input label="邮箱" />
</form>

// 卡片网格
<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 landscape:lg:grid-cols-4">
  {/* 卡片 */}
</div>
```

### 边界情况

1. **iPad横屏**: 视为桌面,使用 `md` 断点
2. **小屏横屏**: <600px横屏保持单列
3. **旋转动画**: 添加 `transition-all duration-300`

---

## 实施优先级

1. **P2 (高)**: 任务1-5 (断点系统、间距、字体、显示控制、触摸优化)
2. **P3 (低)**: 任务6 (横屏模式优化)

---

## 质量标准

### 功能要求
- ✅ 所有响应式样式使用Tailwind类,无自定义 `@media`
- ✅ 遵循WCAG 2.1级别AA标准(触摸目标、可读性)
- ✅ 移动端优先响应式设计
- ✅ 所有断点测试通过(375px, 768px, 1024px, 1280px, 1536px)

### 性能要求
- ✅ 响应式类不超过5个断点/元素
- ✅ 避免过度嵌套的响应式类
- ✅ 使用Tailwind JIT按需生成CSS

### 代码质量
- ✅ 遵循shadcn/ui命名约定
- ✅ 保持中文本地化100%
- ✅ TypeScript类型检查通过
- ✅ 构建成功无警告

---

## 技术约束

1. **Tailwind CSS**: 使用v3.x,不使用v4实验性功能
2. **浏览器支持**: iOS Safari 14+, Chrome 90+, Firefox 88+, Edge 90+
3. **断点系统**: 使用Tailwind默认断点,不修改 `screens` 配置
4. **向后兼容**: 保持现有功能正常工作

---

## 测试计划

### 手动测试
1. **Chrome DevTools**: 测试所有断点(375px → 1920px)
2. **真机测试**: iPhone SE, iPad, Android手机
3. **旋转测试**: 竖屏 ↔ 横屏切换
4. **触摸测试**: 验证所有可点击元素 ≥44x44px

### 自动化测试
```tsx
// 示例: 测试响应式类
describe('Responsive Design', () => {
  it('should hide sidebar on mobile', () => {
    render(<Sidebar />)
    expect(screen.getByTestId('sidebar')).toHaveClass('hidden')
  })

  it('should show sidebar on desktop', () => {
    window.resizeTo(1024, 768)
    render(<Sidebar />)
    expect(screen.getByTestId('sidebar')).not.toHaveClass('hidden')
  })
})
```

---

## 验收标准总结

### 必须满足 (P0)
- ✅ 所有自定义 `@media` 查询替换为Tailwind响应式类
- ✅ 移动端触摸目标 ≥44x44px (WCAG 2.1 AA)
- ✅ 响应式间距遵循8px基准网格
- ✅ 构建成功,无TypeScript错误

### 应该满足 (P1)
- ✅ 响应式字体系统完整
- ✅ 显示/隐藏工具类齐全
- ✅ 中文本地化100%保持
- ✅ 所有断点测试通过

### 可以满足 (P2)
- ✅ 横屏模式优化
- ✅ 容器查询准备(预留)
- ✅ 打印样式(预留)

---

## 超出范围

- 容器查询(CSS Contain,实验性)
- 打印样式优化(`@media print`)
- 特定浏览器hack(如旧版Safari)
- 响应式图片(`<picture>`, `srcset`)
- 服务端响应式(SSR检测)

---

## 参考资料

- [Tailwind CSS - Responsive Design](https://tailwindcss.com/docs/responsive-design)
- [shadcn/ui - Responsive Examples](https://ui.shadcn.com/examples)
- [WCAG 2.1 - Understanding SC 2.5.5 (Target Size)](https://www.w3.org/WAI/WCAG21/Understanding/target-size.html)
- [MDN - Responsive Design](https://developer.mozilla.org/en-US/docs/Learn/CSS/CSS_layout/Responsive_Design)

---

**规范状态**: ✅ 完成,等待 Spec Critic 审查

**编写日期**: 2026-02-09

**预计实施时间**: 2-3小时

**风险等级**: 低(主要是迁移现有代码到Tailwind类)
