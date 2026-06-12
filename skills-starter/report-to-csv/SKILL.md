---
name: report-to-csv
description: Extract Markdown tables or bullet lists into CSV for spreadsheets.
description_zh: 从 Markdown 表格或列表提取数据并导出为 CSV。
category: data
---

# report-to-csv

> **中文**：从 Markdown 表格或列表导出 CSV。  
> **English**: Extract Markdown tables or bullet lists into CSV for spreadsheets.

Use when the user wants spreadsheet-friendly output from a report or summary.

## Workflow

1. Locate source files with **Glob** / **FileRead** (Markdown, text, or agent output).
2. Prefer the bundled **`run`** script for the first Markdown table:
   - `Skill` tool → `report-to-csv` with args `path/to/report.md [out.csv]`
3. For multiple tables or custom columns, use **Bash** + **Edit** to refine CSV manually.
4. Validate row/column counts before delivering the path (WeChat / cron users appreciate a concrete file path).

## Output

- Default output: same basename as input with `.csv` extension.
- Use UTF-8. Quote fields that contain commas.
