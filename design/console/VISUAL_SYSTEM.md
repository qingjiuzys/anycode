# 818Cloud Console — 视觉系统

> 暗夜电紫 · 毛玻璃 · 生产级控制平面  
> 基于 [`design/glass-skins`](../glass-skins/TOKENS.md) `indigo` 皮肤扩展，**暗色为默认且推荐模式**。

---

## 1. 设计哲学

| 维度 | 方向 |
|------|------|
| 气质 | 可信赖的 AI 基础设施，非消费级 SaaS 炫技 |
| 背景 | 近黑紫底色 + 克制径向光晕（紫/蓝），避免大面积渐变噪点 |
| 表面 | 半透明毛玻璃卡片，`backdrop-filter: blur(18px)`，细紫调边框 |
| 强调 | 电光紫为主 CTA；青绿仅用于健康/在线；琥珀/红用于警告/错误 |
| 密度 | 控制平面偏紧凑：14px 正文、清晰表格、充足行高 |

---

## 2. 色彩令牌（Source of Truth）

实现时写入 `design/console/shared/tokens.css`，React 迁移映射到 Tailwind `@theme`。

### 2.1 品牌与强调色

| Token | 值 | 用途 |
|-------|-----|------|
| `--accent` | `#7B5CFF` | 主 CTA、选中态、进度条填充、链接 |
| `--accent-hover` | `#9580FF` | 按钮 hover、焦点环 |
| `--accent-muted` | `rgba(123, 92, 255, 0.18)` | 选中背景、标签底 |
| `--accent-glow` | `rgba(123, 92, 255, 0.22)` | Hero 光晕、卡片外发光 |
| `--accent-secondary` | `#5261FF` | 渐变辅色（与原型 logo 一致） |
| `--accent-tertiary` | `#72D686` | 成功/在线辅助（谨慎使用） |

> 品牌渐变（Logo / 特色卡片）：`linear-gradient(135deg, #25251F 0%, #5261FF 48%, #72D686 100%)`

### 2.2 暗色表面（默认 `html.dark`）

| Token | 值 | 用途 |
|-------|-----|------|
| `--bg` | `#0A0812` | 页面底色 |
| `--bg-elevated` | `#12101C` | 侧栏、顶栏底层 |
| `--bg-work` | `#14121F` | 主工作区底色 |
| `--glass-bg` | `rgba(255, 255, 255, 0.04)` | 普通玻璃卡片 |
| `--glass-bg-strong` | `rgba(255, 255, 255, 0.08)` | 强调卡片、输入框 |
| `--glass-bg-rail` | `rgba(255, 255, 255, 0.03)` | 右侧上下文栏 |
| `--on-surface` | `#F5F3FF` | 主文字（微紫白） |
| `--on-surface-variant` | `#9B97B0` | 次要文字 |
| `--on-surface-muted` | `#6E6A82` | 标签、表头、禁用 |
| `--border` | `rgba(123, 92, 255, 0.12)` | 默认边框 |
| `--border-strong` | `rgba(123, 92, 255, 0.22)` | hover、选中边框 |
| `--solid-bubble` | `#1E1B2E` | 代码块、验证码格 |

### 2.3 背景光晕（页面级）

```css
body {
  background:
    radial-gradient(ellipse 60% 40% at 75% 10%, rgba(123, 92, 255, 0.14), transparent),
    radial-gradient(ellipse 50% 35% at 15% 5%, rgba(82, 97, 255, 0.10), transparent),
    var(--bg);
}
```

### 2.4 语义色

| Token | 暗色值 | 用途 |
|-------|--------|------|
| `--success` | `#4ADE80` | 在线、已支付、绑定成功 |
| `--warn` | `#FACC15` | 额度即将用尽、待支付 |
| `--error` | `#F87171` | 网关故障、支付失败、解绑 |
| `--info` | `#8EB8FF` | 信息提示、BYOK 标签 |

### 2.5 浅色模式（可选，非默认）

Console 以暗色为主；若提供浅色切换，复用 glass-skins light 表面，accent 保持 `#7B5CFF`。

| Token | 浅色值 |
|-------|--------|
| `--bg` | `#F7F6FC` |
| `--glass-bg` | `rgba(255, 255, 255, 0.82)` |
| `--on-surface` | `#1A1625` |

---

## 3. 毛玻璃规范

### 3.1 基础玻璃面板 `.glass-panel`

```css
.glass-panel {
  background: var(--glass-bg);
  border: 1px solid var(--border);
  backdrop-filter: blur(18px) saturate(1.2);
  -webkit-backdrop-filter: blur(18px) saturate(1.2);
  border-radius: var(--radius-md);
}
```

### 3.2 强调玻璃 `.glass-card`

```css
.glass-card {
  background: var(--glass-bg-strong);
  border: 1px solid var(--border);
  backdrop-filter: blur(20px);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
}
```

### 3.3 产品舞台框（营销 mockup 用）

参考原型的 `.product-stage`：外框渐变 + 内阴影，用于首页 hero 中的控制台预览，Console 内不使用。

### 3.4 降级策略

```css
@supports not (backdrop-filter: blur(1px)) {
  .glass-panel, .glass-card {
    background: var(--bg-elevated);
    backdrop-filter: none;
  }
}
```

---

## 4. 阴影与层级

| Token | 值 | 用途 |
|-------|-----|------|
| `--shadow-card` | `0 8px 32px rgba(0, 0, 0, 0.32)` | 卡片 |
| `--shadow-float` | `0 16px 48px rgba(123, 92, 255, 0.12)` | hover 抬升 |
| `--shadow-modal` | `0 24px 64px rgba(0, 0, 0, 0.48)` | 模态、支付 QR |
| `--shadow-inset` | `inset 0 0 0 1px rgba(255, 255, 255, 0.06)` | 输入框内描边 |

**Z 轴层级**：

| 层级 | z-index | 元素 |
|------|---------|------|
| Base | 0 | 主内容 |
| Sticky | 10 | 顶栏、侧栏 |
| Dropdown | 20 | 菜单、Command Palette |
| Modal | 30 | 支付 QR、确认对话框 |
| Toast | 40 | 全局通知 |

---

## 5. 排版

| Token | 值 |
|-------|-----|
| `--font-sans` | `"Inter", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif` |
| `--font-mono` | `"JetBrains Mono", ui-monospace, monospace` |

### 字号阶梯

| 级别 | 大小 | 字重 | 用途 |
|------|------|------|------|
| Display | 34px / -0.035em | 700 | 页面主标题（总览 h1） |
| Title | 21px / -0.02em | 600 | 卡片标题、区段标题 |
| Body | 14px | 400 | 正文、表格 |
| Caption | 12px | 500 | 标签、状态徽章、表头 |
| Mono | 13px | 400 | 验证码、API Key 前缀、账期 |

行高：正文 `1.55`，标题 `1.2`，表格行 `44px` 最小高度。

---

## 6. 圆角与间距

| Token | 值 | 用途 |
|-------|-----|------|
| `--radius-sm` | `8px` | 标签、小按钮 |
| `--radius-md` | `12px` | 输入框、导航项 |
| `--radius-lg` | `16px` | 卡片 |
| `--radius-xl` | `20px` | 大面板、流程图 |
| `--radius-full` | `9999px` | 药丸按钮、进度条 |

| Token | 值 |
|-------|-----|
| `--sidebar-width` | `260px` |
| `--rail-width` | `330px` |
| `--topbar-height` | `56px` |
| `--page-padding` | `24px` |
| `--card-gap` | `12px` |
| `--section-gap` | `24px` |

---

## 7. 组件样式

### 7.1 按钮

| 变体 | 样式 |
|------|------|
| Primary `.btn-primary` | `background: var(--accent)`，白字，hover `--accent-hover` |
| Secondary `.btn-secondary` | 玻璃底 + `--border-strong` 边框 |
| Ghost `.btn-ghost` | 透明底，hover `var(--glass-bg-strong)` |
| Danger `.btn-danger` | `background: var(--error)` 或描边红 |

高度 `40px`，圆角 `--radius-full`，字重 600。

### 7.2 状态徽章 `.status-badge`

| 状态 | 背景 | 文字 | 图标 |
|------|------|------|------|
| ok / online | `rgba(74, 222, 128, 0.14)` | `--success` | ● |
| warn | `rgba(250, 204, 21, 0.14)` | `--warn` | ▲ |
| error | `rgba(248, 113, 113, 0.14)` | `--error` | ✕ |
| pending | `rgba(142, 184, 255, 0.14)` | `--info` | ◷ |

### 7.3 进度条 `.quota-meter`

- 轨道：`rgba(255,255,255,0.10)`，高 8px，圆角 full
- 填充：`linear-gradient(90deg, var(--accent-secondary), var(--accent))`
- ≥80% 用量：填充色切 `--warn`；≥95% 切 `--error`

### 7.4 验证码格 `.code-cell`

4–6 格，每格 `42×42px`，`--solid-bubble` 底，等宽数字，字重 780。

### 7.5 表格 `.console-table`

- 表头：`--on-surface-muted`，12px，大写或全小写一致
- 行分隔：`rgba(255,255,255,0.08)`
- 数字列：`tabular-nums`
- 行 hover：`rgba(123, 92, 255, 0.06)`

### 7.6 Command Bar `.command-bar`

顶栏搜索条，placeholder「搜索模型、账单、设备」，右侧 `⌘K` 快捷键提示，玻璃底 + mono 快捷键徽章。

---

## 8. 图标

- 尺寸：导航 20×20，内联 16×16
- 风格：线性 SVG，`stroke: currentColor`，`stroke-width: 1.5`
- **禁止** Material Symbols；与 glass-skins 保持一致
- 所有图标按钮必须有 `aria-label` + tooltip

### 导航图标映射

| 页面 | 图标语义 |
|------|----------|
| 总览 | grid / dashboard |
| 模型网关 | route / nodes |
| 设备绑定 | link / devices |
| 用量日志 | chart / activity |
| 套餐与账单 | credit-card |
| API Keys | key |
| 团队权限 | users |

---

## 9. 动效

复用 [`design/glass-skins/MOTION.md`](../glass-skins/MOTION.md)：

| Token | 值 | 用途 |
|-------|-----|------|
| `--motion-fast` | `150ms` | 焦点环、按钮 hover |
| `--motion-normal` | `200ms` | 卡片 hover 抬升 |
| `--motion-slow` | `300ms` | 侧栏折叠、面板展开 |

- 卡片 hover：`translateY(-2px)` + `--shadow-float`
- 导航选中：左侧 2px `--accent` 竖条 + `--accent-muted` 背景
- `prefers-reduced-motion: reduce` 时禁用位移与 pulse

---

## 10. 无障碍与对比度

### 10.1 对比度目标（WCAG 2.1 AA）

| 组合 | 最低对比度 |
|------|------------|
| `--on-surface` on `--bg` | ≥ 4.5:1 |
| `--on-surface-variant` on `--glass-bg` | ≥ 4.5:1 |
| `--accent` 按钮白字 | ≥ 4.5:1 |
| 状态色文字 | 同时使用图标 + 文字，不仅依赖颜色 |

### 10.2 焦点态

```css
:focus-visible {
  outline: 2px solid var(--accent-hover);
  outline-offset: 2px;
}
```

### 10.3 键盘导航顺序

顶栏搜索 → 侧栏导航 → 主内容 → 右侧栏（若有）→ 页脚

---

## 11. 响应式断点

| 断点 | 宽度 | 布局变化 |
|------|------|----------|
| Desktop | ≥ 1280px | 三栏：侧栏 + 主区 + 右栏 |
| Tablet | 980–1279px | 隐藏右栏，主区全宽 |
| Mobile | < 980px | 侧栏折叠为抽屉；KPI 卡片单列；表格横向滚动 |

---

## 12. Logo 与品牌

```css
.brand-logo {
  width: 34px;
  height: 34px;
  border-radius: 10px;
  background:
    linear-gradient(135deg, rgba(255,255,255,0.24), transparent 38%),
    linear-gradient(135deg, #25251F, #5261FF 48%, #72D686);
  box-shadow: inset 0 0 0 1px rgba(255,255,255,0.24);
}
```

侧栏缩小版 `24×24px`，圆角 `8px`。

---

## 13. React / CSS 迁移映射

| CSS Token | Tailwind / Theme |
|-----------|------------------|
| `--accent` | `--color-primary` |
| `--bg` | `--color-background` |
| `--on-surface` | `--color-on-surface` |
| `--glass-bg` | utility `.glass-panel` |
| `--sidebar-width` | `--sidebar-width` |

Storage keys（与 Desktop 区分）：

- `818cloud-console-theme`: `dark` | `light`（默认 `dark`）
- `818cloud-console-locale`: `zh` | `en`
