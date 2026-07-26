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

**Design-only** stage for commercial decks. Does **not** produce final `.pptx`.

## Style contract

Default visual language is **FDE Editorial** (`templates/*.html`, contract: `docs/design/fde-editorial-contract.md`, tokens: `brand-kits/fde-editorial/tokens.json`):

- Canvas `#f2f5f0`, ink `#231f20`, accent `#1400ff`; semantic 7-color legend for categories.
- Serif 900 display headlines (Songti SC) + mono uppercase micro-labels; 6px ink rules + 1px hairline grids.
- **Forbidden**: gradients, card shadows, large rounded corners, emoji icons.

Use `templates/lingqi/` (corporate blue) only when the user explicitly asks for that brand.

## Workflow

1. Create `slides/*.html` (1920×1080) from `templates/` — cover / section / two-column / metrics / content / closing. One argument per slide; title states the conclusion.
2. Run **`run`** via Skill: `slides/` `[brand_kit=fde-editorial]`
3. Outputs: `slide_manifest.json` + `evidence/slide-*.png` — inspect the PNGs yourself and fix overflow/empty areas before exporting.
4. Then invoke **`presentation-commercial-delivery`** to export editable native `.pptx`.
