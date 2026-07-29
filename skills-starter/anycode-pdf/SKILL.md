---
name: anycode-pdf
description: >-
  anyCode editorial PDF — COPY fde-editorial MD templates → HTML preview → print-quality PDF.
  Distilled from Kimi PDF HTML route + FDE contract. Use for pdf, report pdf, 报告, 论文, 导出 pdf.
description_zh: >-
  anyCode PDF：复制 MD 模板 → HTML 预览 → 高保真 PDF（FDE Editorial，DeepSeek 友好工作流）。
name_zh: anyCode PDF
category: business
version: 1.0.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [document.author, document.export.pdf]
priority: 125
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# anycode-pdf

**PDF 唯一正确路径** — 模板 Markdown → **HTML 预览** → **`.pdf` 终稿**（Playwright 打印，FDE Editorial）。

> 提炼自 Kimi PDF HTML 路线：Paged 语义由固定 CSS + 打印页边距实现；**默认不走 LaTeX**（除非用户明确要求 .tex）。

## DeepSeek 执行要点

Read `../_shared/deepseek-office.md`，然后：

1. **Copy** 最接近的 `templates/*.md` → `report.md`
2. 只改正文与数据；保留 `#` 结构、表格、Decision/Action 行
3. **`run report.md [out.pdf]`** — validate → preview.html → pdf
4. 交付：**`.pdf`**（必须）+ `report.preview.html` + `report.md`

## 禁止

- 禁止 TBD / lorem / placeholder
- 禁止跳过模板写裸 Markdown
- 禁止只交 HTML 不交 PDF（除非 Playwright 不可用且已在回复中说明）
- 禁止编造统计数据（不确定 → 先搜索）

## 模板

| 场景 | 模板 |
|------|------|
| 周报 / 月报 | `work-report.md` |
| 正式报告 / 方案 | `formal-report.md` |
| 短简报 | `briefing-note.md` |

## 引用与语言

- 中文文档：参考文献用 GB/T 7714 标识 `[J][M][D]`
- 英文文档：APA
- 输出语言与用户请求一致

## 视觉

- preview / PDF 对齐 `docs/design/fde-editorial-contract.md`
- 封面需有 FDE 结构（标题 + meta 行），禁止纯白无样式封面
