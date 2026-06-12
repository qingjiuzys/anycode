---
name: md-to-pdf
description: Convert Markdown reports to PDF for sharing (requires pandoc locally).
description_zh: 将 Markdown 报告转为 PDF 便于分享（需本地 pandoc）。
category: business
---

# md-to-pdf

> **中文**：将 Markdown 报告转为 PDF（本地需 pandoc）。  
> **English**: Convert Markdown reports to PDF for sharing (requires pandoc locally).

Use when the user asks for a PDF deliverable from Markdown.

## Workflow

1. Finalize the Markdown report under the project (e.g. `./reports/weekly-report.md`).
2. Run the bundled **`run`** script via the **Skill** tool:
   - args: `path/to/report.md [optional-output.pdf]`
3. If a PDF engine is available (`xelatex`, `lualatex`, `pdflatex`, `tectonic`,
   `wkhtmltopdf`, `weasyprint`, or `prince`), the script writes the requested PDF.
4. If no dedicated PDF engine is installed but Chrome/Chromium is available, the
   script renders Pandoc HTML through headless Chrome and writes the requested PDF.
5. If no PDF engine or Chrome/Chromium is installed but `pandoc` is available, the script writes a
   same-basename `.html` fallback and prints that path. Tell the user the export
   is HTML because the local PDF engine is missing.
6. Return the absolute output path in the final message (WeChat bridge will inline small `.md`/`.txt` or attach path hints).

## Notes

- Does not upload to cloud drives; local file only.
- For styled PDFs, suggest pandoc templates in a follow-up task.
