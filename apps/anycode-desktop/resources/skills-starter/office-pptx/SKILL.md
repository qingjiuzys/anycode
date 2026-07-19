---
name: office-pptx
description: Generate PowerPoint (.pptx) decks from outlines using python-pptx.
description_zh: 用 python-pptx 根据大纲生成 PowerPoint（.pptx）演示文稿。
name_zh: PPT 生成
category: business
version: 1.1.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# office-pptx

> **中文**：根据大纲生成 `.pptx` 幻灯片（需本地 `python-pptx`）。  
> **English**: Generate `.pptx` slide decks from an outline (requires local `python-pptx`).

## Workflow

1. Confirm topic, audience, and slide count (typically 8–12).
2. Write a slide outline as Markdown (`outline.md`) with one `##` heading per slide.
3. Run the bundled **`run`** script via the **Skill** tool:
   - args: `outline.md [optional-output.pptx]` (paths relative to the **project workspace**, or absolute)
4. Validate that the outline has at least 2 slides and that every slide has a non-empty title.
5. Run the skill and verify that the resulting `.pptx` exists and is non-empty.
6. Return the absolute `.pptx` path and the final slide count.

## Notes

- Requires `pip install python-pptx` if missing.
- Prefer simple title + bullets; avoid embedded images unless user supplies assets.
- Output is a local file under the project workspace.
- Skill `run` resolves relative paths against the project workspace (not the skill install dir); use absolute paths when unsure.

## Failure recovery

- If `python-pptx` is missing, report the exact dependency and keep the completed outline for retry.
- If generation fails, do not leave a partial `.pptx`; return the outline path and error summary.
- This starter creates structured decks, not fully art-directed presentations; do not promise custom visual design unless a template is supplied.
