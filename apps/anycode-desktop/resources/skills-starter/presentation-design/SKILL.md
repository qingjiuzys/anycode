---
name: presentation-design
description: Author dense branded slide HTML (1920x1080) + slide_manifest.json + evidence PNGs.
description_zh: 撰写高密度品牌 slide HTML + manifest + 验收缩略图（不导出 PPTX）。
name_zh: 幻灯片设计
category: business
version: 1.0.0
mode: executable
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [presentation.author]
priority: 120
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# presentation-design

**Design-only** stage. Does **not** produce final `.pptx`.

> **General PPT tasks:** use **`anycode-ppt`** skill instead (priority 125). It owns FDE Editorial templates + validate + export.
> **Do not** use this skill alone to author slides from scratch.

## Style contract

Default visual language is **FDE Editorial** only (`brand-kits/fde-editorial/tokens.json`, contract: `docs/design/fde-editorial-contract.md`):

- Canvas `#f2f5f0`, ink `#231f20`, accent `#1400ff`
- Serif 900 headlines + mono sec-label + 6px ink rules
- **Forbidden**: gradients, shadows, large border-radius, lingqi blue/green theme

`templates/lingqi/` is **deprecated for default use** — only when user explicitly names lingqi brand.

## Workflow

1. Slides must already exist from **`anycode-ppt` templates** (copy-first, not invent CSS)
2. Run **`run`**: `slides/` `fde-editorial` → manifest + evidence
3. Then **`presentation-commercial-delivery`** → editable `.pptx`
