---
name: report-to-csv
description: Extract Markdown tables or bullet lists into CSV for spreadsheets.
description_zh: 从 Markdown 表格或列表提取数据并导出为 CSV。
name_zh: 报表转 CSV
category: data
version: 1.1.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
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

## Quality contract

- Preserve the source column order and do not silently drop malformed rows.
- Report source table count, exported row count, column count, and output path.
- Empty input produces a clear error rather than an empty success artifact.

## Failure recovery

- When no Markdown table exists, show the detected headings/lists and ask which structure should become rows.
- For inconsistent columns, write a diagnostic report and keep the source unchanged.
