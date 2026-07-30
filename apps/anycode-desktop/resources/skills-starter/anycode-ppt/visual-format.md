# anyCode PPT 视觉格式（FDE Editorial）

金标准：`digital-fde-platform/class/bootcamp/day-05/section-01-worldview-plain/video/index.html`

本 skill 交付 **静态 HTML 分页幻灯片**（1920×1080），不含 `<video>` / `<audio>` / GSAP timing / avatar-lipsync。不导出 pptx。

## 设计令牌

```css
:root {
  --bg: #f2f5f0;
  --ink: #231f20;
  --ink-60: rgba(35, 31, 32, 0.72);
  --ink-40: rgba(35, 31, 32, 0.55);
  --ink-08: rgba(35, 31, 32, 0.08);
  --accent: #1400ff;
  --serif: "Noto Serif SC", "Songti SC", serif;
  --sans: "Noto Sans SC", "PingFang SC", sans-serif;
  --mono: "JetBrains Mono", "SF Mono", monospace;
}
```

## 画布

- 1920×1080，一页一 HTML 文件
- padding：`72px 96px`
- 可选 `#brand-bar`：左下 mono 小字品牌/项目标识

## 必用组件类名（勿改名）

- `.sec-label` + `.num` — mono 编号标签 + 6px 粗线
- `.display` / `.statement` / `.lede` — 标题层级
- `.ladder` + `.rung.on|.hot` — 流程梯子
- `.layer-stack` + `.layer-card` — 分层图
- `.agent-cycle` — 环状循环
- `.duo` / `.trio` + `.card` — 对比 / 三列
- `.metrics` + `.stat` — KPI
- `.timeline` + `.mile` — 路线图
- `.checklist` + `.check-item` — 行动清单
- `.quote` — 金句块

## 禁忌

- 渐变背景、卡片阴影、大圆角、emoji 图标
- 只有标题无主视觉的空页
