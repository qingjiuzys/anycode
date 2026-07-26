---
name: spreadsheet-delivery
description: Author tabular workbooks and export real XLSX with headers and concrete rows.
description_zh: 编写表格工作簿并导出真实 XLSX（表头 + 具体数据行）。
name_zh: 表格交付
category: business
version: 1.0.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [spreadsheet.author, spreadsheet.export.xlsx]
priority: 110
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# spreadsheet-delivery

Produce a real `.xlsx` workbook. Independent validators decide completion.

## Workflow

1. Confirm sheet names, columns, and units (currency, percent, dates).
2. Write a CSV or Markdown table source under the workspace (`sales.csv` or `table.md`).
3. Require a header row plus ≥1 data row; no TBD / lorem / placeholder cells.
4. Prefer formulas only when they are simple and documented in a README note.
5. Run the bundled **`run`** script via the **Skill** tool:
   - args: `sales.csv [optional-output.xlsx]` **or** `table.md [optional-output.xlsx]`
6. Spot-check: column names are stable; numeric cells are numbers not strings when possible.
7. Return the absolute `.xlsx` path. Do **not** self-declare verification complete.

## Notes

- Requires `pip install openpyxl` if missing.
- CSV: first line is header; comma-separated.
- Markdown: a GitHub-style pipe table is accepted.

## Failure recovery

- Missing header → add a clear header row and re-run.
- Empty sheet → add concrete sample/real rows before export.
- If `openpyxl` is missing, report the dependency and keep the CSV/MD source.
