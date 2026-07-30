---
name: office-pptx
description: Generate PowerPoint (.pptx) decks from outlines using python-pptx with narrative and visual checks.
description_zh: 用 python-pptx 根据大纲生成 PowerPoint，并做叙事/视觉检查。
name_zh: PPT 生成
category: business
version: 1.2.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [presentation.export.pptx]
priority: 50
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# office-pptx

> **中文**：根据大纲生成 `.pptx` 幻灯片（需本地 `python-pptx`）。  
> **English**: Generate `.pptx` slide decks from an outline (requires local `python-pptx`).

## Workflow

1. Confirm topic, audience, and slide count/order constraints.
2. Prefer narrative arc: Title → Problem → Metric → Plan → Risks → Ask (or task-specified order).
3. Write a slide outline as Markdown (`outline.md`) with one `##` heading per slide.
4. Every non-title bullet should include a concrete number **or** named owner/date. No TBD / Competitor X.
5. Run the bundled **`run`** script via the **Skill** tool:
   - args: `outline.md [optional-output.pptx]` (paths relative to the **project workspace**, or absolute)
6. Validate outline (≥2 slides, non-empty titles) and that `.pptx` exists and is non-empty.
7. Spot-check density: avoid walls of text; prefer 3–5 bullets per slide.
8. When LibreOffice/`soffice` is available, render slide thumbnails under `evidence/slide-*.png` and fix overflow/low-contrast pages. If LibreOffice is missing, keep the `.pptx` + outline and report the environment limit.
9. Return the absolute `.pptx` path and final slide count. Do **not** self-declare verification complete.

## Notes

- Requires `pip install python-pptx` if missing.
- Prefer simple title + bullets; avoid embedded images unless user supplies assets.
- Output is a local file under the project workspace.
- Skill `run` resolves relative paths against the project workspace (not the skill install dir).

## Failure recovery

- If `python-pptx` is missing, report the exact dependency and keep the completed outline for retry.
- If generation fails, do not leave a partial `.pptx`; return the outline path and error summary.
- Duplicate JSON keys in intermediate outlines are forbidden — rebuild from an in-memory structure.
- This starter creates structured decks; do not promise custom art-direction unless a template is supplied.
