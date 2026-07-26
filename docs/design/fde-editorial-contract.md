# FDE Editorial 风格契约（交付物对齐标准）

来源：`digital-fde-platform/class/schedule/index.html` + `styles.css`。
任何 anycode 生成的 HTML 页面 / PPT / 办公文档，默认对齐此契约；brand-kit tokens 见 `brand-kits/fde-editorial/tokens.json`。

## 色彩

| 角色 | 值 | 用途 |
|------|-----|------|
| 画布 surface | `#f2f5f0` | 浅暖灰绿底 |
| 墨色 ink | `#231f20` | 正文、粗规则线、深色区块底 |
| 强调 accent | `#1400ff` | 编号、链接、关键高亮、终态 |
| 强调软 accent-soft | `#7d8cff` | 深色底上的强调文字 |
| 墨色 60/40/08/05 | `rgba(35,31,32,.6/.4/.08/.05)` | 次级文字 / 更弱 / 分隔线 / hover 底 |
| 语义七色 | arch `#1400ff` · fe `#f24e54` · be `#5f52a0` · db `#1cd5b0` · cloud `#f5a947` · llm `#e0479e` · agent `#2f9e44` | 分类标签、图例 |

## 字体

- **展示标题**：中文宋体系 serif（Songti SC / STSong / Noto Serif SC），weight 900，`clamp()` 流式字号；副行用 400 细重 + 60% 墨色。
- **正文**：系统 sans（PingFang SC 等），16px / 1.7。
- **标签/元信息**：等宽 mono（SF Mono / JetBrains Mono），大写、letter-spacing 0.12–0.18em、11–12px。

## 版式母题（必须出现的设计语言）

1. **6px 粗墨线**：hero 下沿、区块顶部、`.sec-label` 后的延伸线。是这套风格最强的识别符。
2. **1px 细网格**：卡片、表格、阶梯条全用 hairline 围合，不用阴影、不用圆角（除小圆点 chip）。
3. **mono 编号标签**：`00 导论 · INTRODUCTION` 式小节标，编号用 accent 色。
4. **阶梯/进度条**：bordered rung 序列，终态 rung 填 accent。
5. **深浅交替**：浅色区块之间插入整段 ink 深色区块（深色上文字用 surface 色、强调用 accent-soft）。
6. **hover 反白**：可交互块 hover 时 invert 或 ink 底白字。
7. **大留白**：section padding 100px，内容 max-width 1120px，移动端收窄。
8. **滚动渐现**：`.rv` fade-up（opacity + translateY 28px，0.9s，逐级 delay）。打印/无 JS 时全部可见。

## 禁忌

- 不用渐变背景、不用卡片阴影、不用大圆角、不用 emoji 图标。
- 不用紫色/粉色渐变"AI 风"配色。
- 图标用 8–14px 实心圆点 chip，不用图标字体。

## PPT 映射

- 封面：墨底或浅底 + serif 900 大标题 + mono 标签行 + 6px 粗线。
- 章节页：accent 大编号 + serif 标题 + 6px 线。
- 内容页：hairline 网格表格、mono 标签、语义七色图例；每页一个观点。
- 字号阶梯：封面 44–96px / 章节 30–52px / 页标题 22–26px / 正文 13–16px / 标签 10–12px。
