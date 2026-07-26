---
name: md-to-pdf
description: Convert Markdown reports to PDF for sharing (requires pandoc locally).
description_zh: 将 Markdown 报告转为 PDF 便于分享（需本地 pandoc）。
name_zh: Markdown 转 PDF
category: business
version: 1.1.0
mode: executable
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# md-to-pdf

> **English**: Convert Markdown reports to PDF for sharing (requires pandoc locally).
> **中文**：将 Markdown 报告转为 PDF（本地需 pandoc）。

## When to use

**Use when:**
- The user asks for a PDF deliverable from a Markdown file.
- The user needs a shareable document format from existing reports.

**Do not use when:**
- The user wants to convert non-Markdown formats (use pandoc directly).
- The task is to create a styled PDF from scratch with custom templates (suggest pandoc templates as a follow-up).
- The user wants cloud upload or sharing (this skill produces local files only).

## Workflow

1. Finalize the Markdown report under the project (e.g. `./reports/weekly-report.md`).
2. Run the bundled **`run`** script via the **Skill** tool:
   - args: `path/to/report.md [optional-output.pdf]`
3. The script attempts PDF engines in order of preference:
   - **LaTeX/dedicated engines**: `xelatex`, `lualatex`, `pdflatex`, `tectonic`, `wkhtmltopdf`, `weasyprint`, `prince` — first available wins.
   - **Chrome/Chromium fallback**: if no dedicated engine is found, render Pandoc HTML through headless Chrome and print to PDF.
   - **HTML fallback**: if no PDF engine or browser is available, write a same-basename `.html` file and inform the user.
4. Return the absolute output path in the final message (WeChat bridge will inline small `.md`/`.txt` or attach path hints).

## Quality contract

- Does not upload to cloud drives; local file only.
- The script exits with code 1 if `pandoc` is not installed — inform the user to install pandoc.
- If the output is HTML fallback, clearly tell the user that PDF export was not possible due to missing engines.
- Always return the absolute path of the output file.
- For styled PDFs, suggest pandoc templates in a follow-up task.

## Failure recovery

- **pandoc not installed**: the script exits with an error. Tell the user to install pandoc (`brew install pandoc` or distro package).
- **No PDF engine available**: the script produces an HTML fallback. Inform the user and suggest installing a PDF engine for native PDF output.
- **Chrome fallback produces empty PDF**: the script falls through to HTML fallback. Report this to the user.
- **Input file not found**: the script exits with an error. Ask the user to verify the file path.
