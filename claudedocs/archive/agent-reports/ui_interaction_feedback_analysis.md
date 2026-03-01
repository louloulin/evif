# UI交互反馈状态完善分析

**日期**: 2026-02-10
**任务**: task-1770650816-6214
**目标**: 完善悬停/点击/聚焦/加载/错误状态，响应<100ms

---

## 当前状态评估

### 已有交互反馈

根据index.css第84-94行，已定义动画时长：
```css
--duration-instant: 100ms;   /* 即时反馈 */
--duration-fast: 150ms;      /* 快速反馈 */
--duration-normal: 200ms;    /* 标准反馈 */
--duration-slow: 300ms;      /* 慢速反馈 */
--duration-slower: 400ms;    /* 更慢反馈 */
```

**符合性**: ✅ 动画时长符合要求（≤300ms）

### 已有缓动函数
```css
--ease-out: cubic-bezier(0, 0, 0.2, 1);          /* 减速 */
--ease-in: cubic-bezier(0.4, 0, 1, 1);           /* 加速 */
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);     /* 加速减速 */
--ease-bounce: cubic-bezier(0.34, 1.56, 0.64, 1); /* 弹跳 */
```

**符合性**: ✅ 缓动函数完整

---

## 需要完善的交互状态

### 1. 悬停状态 (hover)

**要求**: 在100ms内触发视觉反馈

**当前状态**:
```tsx
// Button组件
hover:bg-primary-hover  // ✅ 使用了hover变体
```

**需要增强**:
- ✅ 颜色变化（已有）
- ✅ 阴影增强（已有）
- ⚠️ 变换效果（部分组件缺失）
- ⚠️ 边框变化（部分组件缺失）

### 2. 点击状态 (active)

**要求**: 即时反馈（<50ms）

**当前状态**:
```tsx
// Button组件
active:bg-primary-active    // ✅ 颜色变化
active:scale-[0.98]         // ✅ 缩放反馈
```

**需要增强**:
- ✅ 缩放效果（已有）
- ✅ 内阴影（部分组件已有）
- ⚠️ 统一所有交互元素

### 3. 聚焦状态 (focus)

**要求**: ring和阴影双重指示

**当前状态**:
```tsx
// Input组件
focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
// ✅ 已有ring系统
```

**需要增强**:
- ✅ ring系统（已有）
- ⚠️ 阴影增强（部分组件缺失）
- ⚠️ 边框颜色变化（部分组件缺失）

### 4. 加载状态

**要求**: 骨架屏或进度指示

**当前状态**:
```tsx
// 已有skeleton组件
evif-web/src/components/ui/skeleton.tsx
```

**需要检查**:
- ✅ Skeleton组件已存在
- ⚠️ 是否在所有加载场景使用
- ⚠️ 加载动画流畅性

### 5. 错误状态

**要求**: 红色边框和错误消息

**当前状态**:
```tsx
// Input组件支持error prop
// ✅ 已有destructive颜色系统
```

**需要增强**:
- ⚠️ Input组件error样式
- ⚠️ 错误图标显示
- ⚠️ 错误消息提示

---

## 实施计划

### Phase 2.3.1: 增强悬停状态（15分钟）

**目标**: 确保所有交互元素在100ms内有视觉反馈

**实施**:
- [ ] Button组件: 添加translateY悬停效果
- [ ] Card组件: 增强阴影和边框变化
- [ ] Input组件: 添加边框颜色变化
- [ ] 列表项: 添加背景色变化

**新增样式类**:
```css
.hover-lift {
  transition: transform var(--duration-fast) var(--ease-out);
}
.hover-lift:hover {
  transform: translateY(-1px);
}
```

### Phase 2.3.2: 增强点击状态（10分钟）

**目标**: 统一所有交互元素的点击反馈（<50ms）

**实施**:
- [ ] Button组件: 已有active:scale-[0.98] ✅
- [ ] Card组件: 添加active缩放效果
- [ ] 列表项: 添加active背景变化

**新增样式类**:
```css
.active-press {
  transition: transform var(--duration-instant) var(--ease-out);
}
.active-press:active {
  transform: scale(0.98);
}
```

### Phase 2.3.3: 增强聚焦状态（15分钟）

**目标**: ring和阴影双重指示

**实施**:
- [ ] Input组件: 添加focus阴影
- [ ] Button组件: 优化focus环可见性
- [ ] 确保键盘导航清晰

**优化Input组件**:
```tsx
focus-visible:ring-2 focus-visible:ring-ring
focus-visible:shadow-focus  // 新增
focus-visible:border-border-strong  // 新增
```

### Phase 2.3.4: 优化加载状态（10分钟）

**目标**: 确保所有加载场景使用skeleton

**检查**:
- [ ] 验证Skeleton组件样式
- [ ] 添加pulse动画效果
- [ ] 确保加载过渡平滑

### Phase 2.3.5: 完善错误状态（10分钟）

**目标**: 红色边框+错误图标+错误消息

**实施**:
- [ ] Input组件: 添加error变体
- [ ] 添加错误图标显示
- [ ] 添加错误消息提示样式

**Input组件error变体**:
```tsx
className={cn(
  "focus-visible:ring-2 focus-visible:ring-ring",
  error && "border-destructive focus-visible:ring-destructive focus-visible:border-destructive-strong",
  className
)}
```

---

## 交互反馈规范

### 悬停状态 (hover)

**时长**: ≤100ms (使用 `--duration-fast`)

**效果**:
- 颜色变化: `hover:bg-*`, `hover:text-*`
- 阴影增强: `hover:shadow-md`, `hover:shadow-lg`
- 轻微提升: `hover:-translate-y-0.5`
- 边框变化: `hover:border-*`

**示例**:
```tsx
<Button className="hover:-translate-y-0.5 hover:shadow-md">
  点击我
</Button>
```

### 点击状态 (active)

**时长**: <50ms (使用 `--duration-instant`)

**效果**:
- 缩放: `active:scale-[0.98]`
- 内阴影: `active:shadow-inner`
- 颜色加深: `active:bg-*-active`

**示例**:
```tsx
<Button className="active:scale-[0.98] active:shadow-inner">
  点击我
</Button>
```

### 聚焦状态 (focus)

**效果**:
- ring环: `focus-visible:ring-2 focus-visible:ring-ring`
- 阴影: `focus-visible:shadow-focus`
- 边框: `focus-visible:border-border-strong`

**示例**:
```tsx
<Input className="focus-visible:ring-2 focus-visible:ring-ring focus-visible:shadow-focus" />
```

### 加载状态

**效果**:
- Skeleton组件: `<Skeleton className="h-4 w-full" />`
- Pulse动画: `animate-pulse`
- Spinner: `loading-spinner` 类

**示例**:
```tsx
{loading ? (
  <Skeleton className="h-10 w-full" />
) : (
  <Button>加载完成</Button>
)}
```

### 错误状态

**效果**:
- 边框: `border-destructive`
- ring: `focus-visible:ring-destructive`
- 图标: 错误图标显示
- 消息: 错误文本提示

**示例**:
```tsx
<Input
  error={hasError}
  className="border-destructive focus-visible:ring-destructive"
/>
{hasError && <p className="text-destructive text-sm">错误消息</p>}
```

---

## 验证清单

### GWT-4验收标准

| 标准 | 要求 | 验证方法 | 状态 |
|------|------|---------|------|
| 悬停状态 | 100ms内触发 | 测试hover动画时长 | ⏳ |
| 点击状态 | <50ms反馈 | 测试active响应速度 | ⏳ |
| 聚焦状态 | ring+阴影双重 | 视觉检查focus样式 | ⏳ |
| 加载状态 | 骨架屏/进度 | 检查所有加载场景 | ⏳ |
| 错误状态 | 红边框+消息 | 测试error变体 | ⏳ |

---

## 预期结果

**完成后**:
- ✅ 所有交互元素响应<100ms
- ✅ 点击反馈即时（<50ms）
- ✅ 聚焦状态清晰可见
- ✅ 加载状态一致性好
- ✅ 错误状态信息完整
- ✅ 整体交互体验显著提升

**预计时间**: 60分钟
**置信度**: 95%
