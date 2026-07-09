---
name: office-pptx
description: Generate PowerPoint (.pptx) decks from outlines using python-pptx.
description_zh: 用 python-pptx 根据大纲生成 PowerPoint（.pptx）演示文稿。
category: business
---

# office-pptx

> **中文**：根据大纲生成 `.pptx` 幻灯片（需本地 `python-pptx`）。  
> **English**: Generate `.pptx` slide decks from an outline (requires local `python-pptx`).

## Workflow

1. Confirm topic, audience, and slide count (typically 8–12).
2. Write a slide outline as Markdown (`outline.md`) with one `##` heading per slide.
3. Run the bundled **`run`** script via the **Skill** tool:
   - args: `outline.md [optional-output.pptx]`
4. Review the output path; each `##` becomes one title + bullet slide.
5. Return the absolute `.pptx` path in the final message.

## Notes

- Requires `pip install python-pptx` if missing.
- Prefer simple title + bullets; avoid embedded images unless user supplies assets.
- Output is a local file under the project workspace.
