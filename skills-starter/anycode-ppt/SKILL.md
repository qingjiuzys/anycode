---
name: anycode-ppt
description: >-
  anyCode editorial HTML slides — COPY fde-editorial templates (ladder/layer/agent-cycle/trio).
  Delivers paginated 1920×1080 HTML deck + index.html viewer. No pptx export.
  Use for ppt, slides, presentation, pitch deck, 幻灯片, 演示文稿, html ppt, anycode ppt.
description_zh: >-
  anyCode HTML 幻灯片：从 templates/ 复制 FDE Editorial 样式，交付分页 HTML + 浏览器预览，不导出 pptx。
name_zh: anyCode HTML 幻灯片
category: business
version: 2.0.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [presentation.author]
priority: 125
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# anycode-ppt

**HTML 幻灯片唯一正确路径** — FDE Editorial 视觉（`#f2f5f0` / `#231f20` / `#1400ff`），交付 **分页 HTML + index.html 预览器**。

> 不导出 `.pptx`。HTML 保真、可 diff、浏览器直接演示，比 OOXML 转译更可靠。

## 禁止（违反则 validate 失败）

- **禁止从零写 CSS** / 禁止 `presentation-design/templates/lingqi/` / 禁止企业蓝 `#1B3A5C`、绿 `#00B050`
- 禁止渐变、阴影、大圆角（>8px）、footer 写 `lingqi`
- 禁止跳过本 skill 直接用 Write 造 slides
- **禁止** `presentation-commercial-delivery` / `fill_potx` / 生成 `.pptx`（除非用户**另外**明确要求 pptx）

## 必须工作流

1. **Read** `components.md` + `templates/`（见下方组件表）
2. **Copy** 最接近的模板 → `slides/NN-name.html`，**只改文案与数据**，保留 `:root` 令牌与 class 名
3. 页数按内容定（≥2）；选对组件（四层架构 → `layer-stack-4`；Agent 循环 → `agent-cycle`）
4. `run slides/` — 等价于 design + validate + 生成 `index.html`
5. 交付物：`slides/*.html` + `index.html` + `slide_manifest.json` + `evidence/*.png`（预览缩略）

## 组件表（完整列表见 components.md）

| 场景 | 模板 |
|------|------|
| 封面 | `cover.html` |
| 章节 | `section.html` |
| 线性流程 | `ladder-flow.html` |
| 四层架构 | `layer-stack-4.html` |
| Agent 五步循环 | `agent-cycle.html` |
| 双卡对比 | `duo-compare.html` |
| 三列要点 | `trio-cards.html` |
| KPI 数据 | `metrics-kpi.html` |
| 金句 / 结论 | `quote-insight.html` |
| 路线图 | `timeline.html` |
| 行动清单 | `checklist.html` |
| 插图页 | `diagram-image.html` |
| 收尾 | `closing.html` |

## 视觉契约

- `visual-format.md` + `docs/design/fde-editorial-contract.md`
- 密度：`diagram-density.md`（content 页需主视觉 class 或 `<img>`）

## 预览与交付

- **`index.html`** — 左侧目录 + iframe 16:9 预览，`←` `→` 翻页，`F` 新标签打开当前页
- **`evidence/*.png`** — Playwright 截图，供 Workbench 缩略预览，**不是**终稿
- **终稿** = `slides/` 目录下的 HTML 分页文件
