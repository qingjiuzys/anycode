---
name: anycode-xlsx
description: >-
  anyCode editorial Excel — COPY fde-editorial workbook JSON templates → branded .xlsx.
  Use for xlsx, excel, spreadsheet, 表格, 工作簿, 财务, sales, kpi.
description_zh: >-
  anyCode Excel：复制 workbook 模板 → validate → 导出 FDE 品牌 xlsx。
name_zh: anyCode Excel
category: business
version: 1.1.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [spreadsheet.author, spreadsheet.export.xlsx]
priority: 125
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# anycode-xlsx

**Excel 唯一正确路径** — 模板 `workbook.json` → validate → **可编辑 `.xlsx` 终稿**。

> 终稿是 **xlsx**，不是 JSON。JSON 是结构化数据源（方便 Agent 填数 + validate）；可选 `workbook.preview.html` 供 Workbench 快速看表。

## 禁止

- 禁止 TBD / placeholder 单元格
- 禁止 <3 sheet（Summary + Detail + 专题）
- 禁止只交 CSV/JSON 不交 xlsx
- 禁止跳过模板自造裸表

## 工作流

1. **Read** `components.md` + `templates/`
2. **Copy** 模板 → `workbook.json`
3. `run workbook.json [out.xlsx]` → validate → preview（可选）→ xlsx
4. 交付：**`.xlsx`**（必须）+ `workbook.json`（源）

## 模板

| 场景 | 模板 |
|------|------|
| 销售明细 | `workbook-sales.json` |
| 财务季报 | `workbook-finance.json` |
| 团队 KPI | `workbook-kpi.json` |

## 品牌

- `brand-kits/fde-editorial/xlsx/theme.json`（ink 表头 / accent 强调列）
