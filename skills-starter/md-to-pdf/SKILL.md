---
name: md-to-pdf
description: Convert Markdown reports to PDF for sharing (requires pandoc locally).
---

# md-to-pdf

Use when the user asks for a PDF deliverable from Markdown.

## Workflow

1. Finalize the Markdown report under the project (e.g. `./reports/weekly-report.md`).
2. Run the bundled **`run`** script via the **Skill** tool:
   - args: `path/to/report.md [optional-output.pdf]`
3. If `pandoc` is missing, tell the user to install it (`brew install pandoc` / distro package) or offer HTML export instead.
4. Return the absolute PDF path in the final message (WeChat bridge will inline small `.md`/`.txt` or attach path hints).

## Notes

- Does not upload to cloud drives; local file only.
- For styled PDFs, suggest pandoc templates in a follow-up task.
