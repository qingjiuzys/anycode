---
name: anycode-docx
description: >-
  anyCode editorial Word — COPY fde-editorial MD templates → HTML preview → editable .docx.
  Use for docx, word, report, 文档, 周报, 月报, 述职, 工作汇报, briefing.
description_zh: >-
  anyCode Word 文档：复制 MD 模板 → HTML 预览 → 导出可编辑 docx（FDE Editorial）。
name_zh: anyCode Word
category: business
version: 1.1.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [document.author, document.export.docx]
priority: 125
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# anycode-docx

**Word 唯一正确路径** — 模板 Markdown → **HTML 预览** → **可编辑 `.docx` 终稿**。

## 为什么接 HTML

- **MD**：Agent 写内容、validate、可 diff（写作层）
- **HTML preview**：Workbench 浏览器评审，FDE 样式即时可见（预览层）
- **DOCX**：对外交付、同事用 Word 改（终稿层）

三者一条链，`run` 一次跑完。HTML **不是**替代 docx，是预览加速评审。

## 禁止

- 禁止 TBD / lorem / placeholder
- 禁止跳过模板自造扁平 bullet
- 禁止无 `Decision:`/`Action:` + 负责人 + 日期
- 禁止只交 MD 或只交 preview 不交 docx

## 工作流

1. **Read** `components.md` + `templates/`
2. **Copy** 模板 → `report.md`，只改内容
3. `run report.md [out.docx]` → validate → `report.preview.html` → `report.docx`
4. 交付：**`.docx`**（必须）+ `report.preview.html`（预览）+ `report.md`（源）

## 模板

| 场景 | 模板 |
|------|------|
| 周报 / 月报 | `work-report.md` |
| 述职 / OKR | `performance-review.md` |
| 短简报 | `briefing-note.md` |

## 视觉

- preview.html 对齐 `docs/design/fde-editorial-contract.md`
- docx 走 `document-delivery` + `brand-kits/fde-editorial`
