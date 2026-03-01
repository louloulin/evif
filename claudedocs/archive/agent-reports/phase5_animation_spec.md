# Phase 5 Task 3: 改进动画和过渡效果规范

## 创建日期: 2026-02-09
## 任务优先级: P3 (低优先级)
## 任务ID: task-1770642290-8d24

---

## 规范概述 (Summary)

改进EVIF Web应用中的动画和过渡效果,参考shadcn/ui的设计风格,实现简洁、流畅、非侵入式的动画系统,提升用户体验的精致度和专业感。

---

## 验收标准 (Acceptance Criteria)

### 任务1: 标准化过渡效果 (Transition System)

**1.1 ✅ GIVEN 任何交互式元素(按钮、链接、输入框) WHEN 用户悬停(hover)时 THEN 应用150ms缓动过渡**

- 验证方法: 检查所有可交互元素是否有 `transition-{property}` 类
- 期望结果: 所有按钮、链接、输入框使用 `transition-all duration-150` 或 `transition-colors duration-150`
- 边界情况: 禁用状态的按钮不应该有悬停效果

**1.2 ✅ GIVEN 任何交互式元素 WHEN 用户激活(active/focus)时 THEN 应用100ms快速响应过渡**

- 验证方法: 检查按钮和输入框的 `active:` 和 `focus:` 状态
- 期望结果: 使用 `transition-all duration-100` 实现即时反馈
- 边界情况: focus-visible状态应该有明确的视觉指示

**1.3 ✅ GIVEN 模态框/对话框 WHEN 打开或关闭时 THEN 应用200ms淡入淡出过渡**

- 验证方法: 检查Dialog组件的过渡效果
- 期望结果: 使用 `transition-all duration-200` 或 `animate-in fade-in` 模式
- 边界情况: 快速打开/关闭不应该有动画堆积

### 任务2: 微交互动画 (Micro-interactions)

**2.1 ✅ GIVEN 用户点击按钮时 THEN 按钮应该有缩放反馈(scale transform)**

- 验证方法: 检查按钮是否有 `active:scale-95` 或 `active:scale-[0.98]` 类
- 期望结果: 点击时按钮轻微缩小,提供触觉反馈
- 边界情况: 加载状态的按钮不应该有缩放效果

**2.2 ✅ GIVEN 用户悬停在可点击元素上时 THEN 元素应该有轻微的视觉增强**

- 验证方法: 检查悬停状态的变化(颜色、阴影、透明度)
- 期望结果: 使用 `hover:opacity-80` 或 `hover:bg-accent/50` 等效果
- 边界情况: 禁用元素不应该有悬停效果

**2.3 ✅ GIVEN 加载状态时 THEN 显示旋转的spinner动画**

- 验证方法: 检查加载状态的视觉反馈
- 期望结果: 使用 `animate-spin` 类实现旋转动画
- 边界情况: 快速加载( < 100ms)时可选显示spinner

### 任务3: 列表和网格动画 (List & Grid Animations)

**3.1 ✅ GIVEN 列表项添加或删除时 THEN 应用淡入淡出动画**

- 验证方法: 检查FileTree、PluginList等列表组件
- 期望结果: 新项淡入,删除项淡出
- 边界情况: 批量操作时应该使用stagger动画(交错动画)

**3.2 ✅ GIVEN 卡片布局 WHEN 网格重新排列时 THEN 应用平滑的布局过渡**

- 验证方法: 检查MonitorView的metric cards布局
- 期望结果: 使用 `transition-all duration-300` 实现布局过渡
- 边界情况: 响应式断点变化时不应该有跳动

### 任务4: 页面过渡动画 (Page Transitions)

**4.1 ✅ GIVEN 用户切换视图时 THEN 应用150ms淡入淡出过渡**

- 验证方法: 检查MonitorView、PluginManagerView等视图切换
- 期望结果: 使用 `transition-opacity duration-150` 实现平滑切换
- 边界情况: 快速切换时应该取消前一个动画

**4.2 ✅ GIVEN 内容区域加载时 THEN 应用从下往上的slide-in动画**

- 验证方法: 检查主要内容区域的初始加载
- 期望结果: 使用 `animate-in slide-in-from-bottom-4` 或类似效果
- 边界情况: 后续更新不应该重复slide-in动画

### 任务5: 高级动画效果 (Advanced Animations)

**5.1 ✅ GIVEN 通知/提示消息 WHEN 显示时 THEN 应用从右侧滑入动画**

- 验证方法: 检查NotificationCenter组件
- 期望结果: 使用 `animate-in slide-in-from-right` 或CSS动画
- 边界情况: 多个通知应该堆叠显示,每个都有独立动画

**5.2 ✅ GIVEN 下拉菜单 WHEN 打开时 THEN 应用从上往下的展开动画**

- 验证方法: 检查CommandPalette等下拉组件
- 期望结果: 使用 `animate-in slide-in-from-top-1` 或类似效果
- 边界情况: 菜单项较多时应该有滚动,动画不应该被截断

**5.3 ✅ GIVEN 图表数据更新时 THEN 应用平滑的数据过渡动画**

- 验证方法: 检查TrafficChart、OperationChart的更新
- 期望结果: 使用图表库的内置过渡效果(如recharts的animationDuration)
- 边界情况: 实时更新频繁时应该适当降低动画时长

---

## 具体要求 (Specific Requirements)

### 1. Tailwind CSS动画工具类配置

**1.1 在 `tailwind.config.js` 或 `index.css` 中添加自定义动画**

```css
/* index.css */
@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

@keyframes slide-in-from-right {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

@keyframes slide-in-from-top {
  from {
    transform: translateY(-10px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

.animate-fade-in {
  animation: fade-in 150ms ease-out;
}

.animate-fade-out {
  animation: fade-out 150ms ease-in;
}

.animate-slide-in-right {
  animation: slide-in-from-right 200ms ease-out;
}

.animate-slide-in-top {
  animation: slide-in-from-top 150ms ease-out;
}
```

**1.2 在 `tailwind.config.js` 中扩展动画配置**

```javascript
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      transitionDuration: {
        '100': '100ms',
        '150': '150ms',
        '200': '200ms',
        '300': '300ms',
      },
      transitionTimingFunction: {
        'ease-out-subtle': 'cubic-bezier(0.4, 0, 0.2, 1)',
        'ease-in-subtle': 'cubic-bezier(0.4, 0, 1, 1)',
      },
    },
  },
}
```

### 2. 组件级动画实现

**2.1 按钮组件动画**

- 默认: `transition-all duration-150 ease-out-subtle`
- 悬停: `hover:opacity-80` 或 `hover:bg-accent/50`
- 激活: `active:scale-95 transition-transform duration-100`
- 焦点: `focus-visible:ring-2 focus-visible:ring-ring`

**2.2 输入框组件动画**

- 默认: `transition-all duration-150 ease-out-subtle`
- 焦点: `focus:ring-2 focus:ring-ring transition-all duration-150`
- 错误: `border-destructive transition-colors duration-150`

**2.3 卡片组件动画**

- 默认: `transition-all duration-200 ease-out-subtle`
- 悬停: `hover:shadow-md hover:scale-[1.01]`
- 加载: `animate-pulse`

**2.4 Dialog/Modal组件动画**

- 遮罩: `transition-opacity duration-200`
- 内容: `transition-all duration-200 animate-in fade-in slide-in-from-top-4`
- 关闭: `animate-out fade-out slide-out-to-top-4`

**2.5 列表项动画**

- 新增: `animate-in fade-in slide-in-from-left-2 duration-200`
- 删除: `animate-out fade-out slide-out-to-left-2 duration-150`
- 悬停: `hover:bg-accent/50 transition-colors duration-100`

### 3. 性能要求

**3.1 使用GPU加速的属性**

- 优先使用 `transform` 和 `opacity` 实现动画
- 避免 `width`、`height`、`left`、`top` 等触发布局重排的属性
- 使用 `will-change: transform, opacity` 优化复杂动画

**3.2 减少动画复杂度**

- 同时运行的动画不超过3个
- 避免嵌套动画元素
- 使用 `requestAnimationFrame` 优化JavaScript动画

**3.3 尊重用户偏好**

- 检测 `prefers-reduced-motion` 媒体查询
- 对于启用了减少动画的用户,禁用所有非必要动画

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 输入示例 (Input Examples)

### 示例1: 带动画的按钮

```tsx
// Before
<button className="px-4 py-2 bg-primary text-primary-foreground">
  点击我
</button>

// After
<button className="px-4 py-2 bg-primary text-primary-foreground transition-all duration-150 ease-out-subtle hover:opacity-80 active:scale-95">
  点击我
</button>
```

### 示例2: 带动画的卡片

```tsx
// Before
<div className="border rounded-lg p-4">
  内容
</div>

// After
<div className="border rounded-lg p-4 transition-all duration-200 ease-out-subtle hover:shadow-md hover:scale-[1.01]">
  内容
</div>
```

### 示例3: 带动画的Dialog

```tsx
// Before
<Dialog open={isOpen} onOpenChange={setIsOpen}>
  <DialogContent>
    内容
  </DialogContent>
</Dialog>

// After
<Dialog open={isOpen} onOpenChange={setIsOpen}>
  <DialogContent className="transition-all duration-200 animate-in fade-in slide-in-from-top-4">
    内容
  </DialogContent>
</Dialog>
```

### 示例4: 带动画的列表项

```tsx
// Before
<li className="p-2 border-b">
  列表项
</li>

// After
<li className="p-2 border-b transition-colors duration-100 hover:bg-accent/50 animate-in fade-in slide-in-from-left-2 duration-200">
  列表项
</li>
```

---

## 边界情况 (Edge Cases)

### 1. 禁用状态的元素

- 禁用按钮不应该有悬停和激活动画
- 使用 `disabled:opacity-50 disabled:cursor-not-allowed disabled:transition-none`
- 示例: `<button disabled className="transition-all duration-150 hover:opacity-80 disabled:transition-none">`

### 2. 加载状态

- 加载中的元素应该有持续动画(如spinner)
- 但不应该有悬停效果
- 示例: `<button className="animate-spin disabled:hover:opacity-100">`

### 3. 快速连续操作

- 用户快速点击按钮时,不应该有动画堆积
- 使用CSS动画的 `animation-iteration-count: 1` 防止重复
- JavaScript中添加防抖(debounce)逻辑

### 4. 大列表渲染

- 虚拟滚动列表中,只对可见项应用动画
- 使用 `transition-delay` 实现stagger效果
- 避免同时渲染过多动画元素

### 5. 低端设备

- 检测设备性能,适当降低动画复杂度
- 使用CSS `@media (prefers-reduced-motion: reduce)` 查询
- 提供禁用动画的用户设置

---

## 非功能需求 (Non-functional Requirements)

### 1. 性能要求

- 动画帧率应该保持在60fps或更高
- 使用Chrome DevTools的Performance面板验证
- 避免长任务( > 50ms)阻塞主线程

### 2. 可访问性要求

- 遵循WCAG 2.1指南:动画不应导致闪烁或眩晕
- 尊重用户的 `prefers-reduced-motion` 设置
- 提供禁用动画的选项

### 3. 浏览器兼容性

- 使用CSS `transition` 和 `@keyframes` 的标准语法
- 为旧浏览器提供降级方案(无动画)
- 测试目标浏览器:Chrome 90+, Firefox 88+, Safari 14+, Edge 90+

### 4. 可维护性要求

- 所有动画效果应该使用Tailwind工具类
- 不使用内联样式或魔术数字
- 在组件文档中记录特殊动画效果

### 5. 用户体验要求

- 动画应该感觉自然,不应该突兀
- 过渡时长应该符合用户预期(100-300ms)
- 提供清晰的视觉反馈,不要让用户困惑

---

## 超出范围 (Out of Scope)

### 本规范不包含:

1. **复杂的3D动画或WebGL效果** - 这些超出了shadcn/ui风格的范畴
2. **音频反馈** - 声音效果不在本规范内
3. **手势动画** - 移动端特定手势(如swipe)的动画效果
4. **页面加载动画** - 全屏loading动画不在本规范内(已有loading骨架屏)
5. **数据可视化动画** - 图表库的内置动画不需要额外配置
6. **游戏化动画** - 如进度条庆祝效果、徽章动画等

### 如果需要这些功能,应该创建独立的规范文档。

---

## 参考资源 (References)

1. **shadcn/ui动画风格**: https://ui.shadcn.com/docs/installation
2. **Tailwind CSS动画文档**: https://tailwindcss.com/docs/transition-property
3. **Framer Motion**: https://www.framer.com/motion/ (可选的高级动画库)
4. **WCAG 2.1动画指南**: https://www.w3.org/WAI/WCAG21/Understanding/animation-from-interactions
5. **Web动画性能最佳实践**: https://web.dev/animations-guide/

---

## 实施优先级 (Implementation Priority)

### P0 (必须实施)
- 所有按钮的悬停和激活动画
- 输入框的焦点过渡
- Dialog的淡入淡出动画

### P1 (高优先级)
- 卡片的悬停效果
- 列表项的淡入动画
- 通知的滑入动画

### P2 (中优先级)
- 页面视图的过渡动画
- 下拉菜单的展开动画
- 高级微交互效果

### P3 (低优先级,可选)
- 复杂的stagger动画
- 自定义缓动函数
- 数据可视化的过渡效果

---

## 验收测试用例 (Acceptance Test Cases)

### 测试用例1: 按钮动画

```
GIVEN 用户查看EVIF Web应用
WHEN 用户悬停在"刷新"按钮上
THEN 按钮应该有透明度变化(hover:opacity-80)
AND 过渡时长应该是150ms
```

### 测试用例2: 输入框焦点动画

```
GIVEN 用户查看快速打开对话框
WHEN 用户点击搜索输入框
THEN 输入框应该有ring效果(focus:ring-2)
AND ring效果应该在150ms内完成过渡
```

### 测试用例3: Dialog动画

```
GIVEN 用户查看EVIF Web应用
WHEN 用户打开插件管理对话框
THEN Dialog应该淡入(fade-in)
AND 内容应该从上往下滑入(slide-in-from-top-4)
AND 动画时长应该是200ms
```

### 测试用例4: 列表项动画

```
GIVEN 用户查看文件树
WHEN 用户展开一个文件夹
THEN 子文件应该淡入(fade-in)
AND 子文件应该从左侧滑入(slide-in-from-left-2)
AND 动画时长应该是200ms
```

### 测试用例5: 卡片悬停动画

```
GIVEN 用户查看监控视图
WHEN 用户悬停在metric card上
THEN 卡片应该有轻微阴影(hover:shadow-md)
AND 卡片应该有轻微缩放(hover:scale-[1.01])
AND 过渡时长应该是200ms
```

### 测试用例6: 减少动画模式

```
GIVEN 用户启用了操作系统的减少动画设置
WHEN 用户查看任何UI元素
THEN 所有动画应该被禁用或降至最小
```

---

## 规范完成标准 (Spec Completion Criteria)

本规范被认为完整且可实施,当且仅当:

1. ✅ 所有验收标准都有明确的GIVEN-WHEN-THEN格式
2. ✅ 所有具体要求都有代码示例或配置说明
3. ✅ 所有边界情况都有明确的处理方案
4. ✅ 所有非功能需求都有可测量的指标
5. ✅ 超出范围的内容已经明确列出
6. ✅ 实施优先级已经明确划分
7. ✅ 验收测试用例覆盖主要功能点

---

**Spec Writer签名**: Ralph (📋 Spec Writer Hat)
**完成时间**: 2026-02-09
**规范状态**: ✅ 完成,等待Spec Critic审查
**下一步**: 发布 spec.ready 事件,交由Spec Critic审查
