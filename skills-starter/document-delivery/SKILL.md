---
name: document-delivery
description: Author structured reports and export editable DOCX with Decision/Action ownership.
description_zh: 撰写结构化报告并导出可编辑 DOCX（含 Decision/Action 责任人）。
name_zh: 文档交付
category: business
version: 1.1.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [document.author, document.export.docx]
priority: 110
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# document-delivery

Produce a real `.docx` in the workspace. Independent validators decide completion.

## Workflow

1. Clarify audience and purpose (once).
2. Outline Markdown: `# Summary` first, then `##` sections (metrics, incidents, changes, next steps).
3. Fill concrete facts; end every section with `Decision:` or `Action:` including owner and ISO date.
4. Keep the Markdown source under the workspace (`report.md` or similar).
5. Run the bundled **`run`** script via the **Skill** tool:
   - args: `report.md [optional-output.docx] [brand_kit]` (paths relative to the **project workspace**, or absolute)
   - `brand_kit` defaults to `fde-editorial` (standard anyCode editorial style: ink/serif/electric-blue, see `docs/design/fde-editorial-contract.md`); `lingqi` for the legacy corporate-blue brand, `gov-formal` for government reports.
6. Check: no empty sections, no TBD/lorem. Prefer real `.docx` — do not deliver flat PDF only.
7. If LibreOffice is available, smoke-open/convert to PDF for layout evidence; otherwise report the environment limit.

## Notes

- Requires `pip install python-docx` if missing.
- Skill `run` resolves relative paths against the project workspace (not the skill install dir).

## Failure recovery

- Missing Decision/Action → append as the last paragraph of each section, then re-run.
- Flat dump → rebuild H1/H2 hierarchy first.
- Do not self-declare completion.
