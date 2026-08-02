---
name: presentation-commercial-delivery
description: Export editable native PPTX from slide_manifest/HTML via brand potx fill (commercial deliverable).
description_zh: 从 slide_manifest/HTML 填充品牌母版，导出可编辑原生 PPTX（商用终稿）。
name_zh: 幻灯片商用导出
category: office
version: 1.0.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [presentation.export.pptx]
priority: 130
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# presentation-commercial-delivery

**Commercial deliverable path** — native editable OOXML via `fill_potx.py`.

## Workflow

1. Ensure `slides/*.html` and/or `slide_manifest.json` exist (from `anycode-ppt`).
2. Run **`run`**: `slides/` or `slide_manifest.json` `[output.pptx]` `[brand_kit=fde-editorial]`
   - `fde-editorial` = standard anyCode editorial style (contract: `docs/design/fde-editorial-contract.md`); `lingqi` / `gov-formal` for their respective brands.
3. Produces editable `.pptx` — native text shapes and native `c:chart` charts, not full-slide raster images.

## Content density

| Slide type | Minimum |
|------------|---------|
| cover | 3 value chips + meta |
| section | 4 agenda items |
| content | 5 bullets + side panel |
| metrics | 6 stat cards |
| closing | 4 actions + contact |
