---
name: web-design-delivery
description: Design and implement a self-contained HTML landing page with visual craft, then verify structure and contrast.
description_zh: 设计并实现自包含 HTML 落地页（视觉层次 + 对比度），并做结构验收。
name_zh: 网页设计交付
category: quality
version: 1.0.0
mode: documentation
approval: writes-workspace
channel_capabilities: [files, artifacts]
provides_capabilities: [web.implement, web.preview]
priority: 100
platforms: [darwin, linux]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# web-design-delivery

Produce a **real HTML file** in the project workspace. Do **not** claim the task is complete yourself — independent validators decide.

## Workflow

1. Reuse known visual preferences (do not re-ask colors if already known).
2. Default visual language is **FDE Editorial** (contract: `docs/design/fde-editorial-contract.md`, tokens: `brand-kits/fde-editorial/tokens.json`): light canvas `#f2f5f0`, ink `#231f20`, electric-blue accent `#1400ff`; serif 900 display headlines (Songti SC) + mono uppercase micro-labels; 6px ink rules, 1px hairline grids, generous whitespace, one dark ink section for contrast. Forbidden: gradients, card shadows, big rounded corners, emoji icons, Inter/Roboto.
3. Compose a rich hero: mono meta-label row, serif display headline (bold + thin secondary line), one statement sentence with an accent `<strong>`, **and** one structural anchor (ladder / rule list / hairline grid panel).
4. Write semantic HTML/CSS to a single file (e.g. `index.html`). Include HTML comments with approximate contrast ratios for body text and CTA.
5. Verify locally:
   - exactly one `H1`
   - no gradient / box-shadow / border-radius > 8px
   - palette stays within the contract tokens (accent used sparingly, not as background wash)
   - no markdown fences
   - `<meta name="viewport">` + responsive `@media` for ~375 / 768 / 1440
   - file path is absolute when reporting
6. Capture screenshots when a browser tool is available (`evidence/viewport-375.png`, `768`, `1440`). If browser/fonts unavailable, report the environment limit — do not invent screenshots.
7. If a verification diagnostic is returned, fix only failed gates; do not regress passed gates.

## Output contract

- Pure HTML document on disk (not a fenced code block in chat).
- Serif display type + sans body + mono labels; avoid Inter/Roboto.
- Keep visual hierarchy — never flatten to a bare centered block just to satisfy a checklist.

## Failure recovery

- Missing contrast comments → add HTML comments with approximate ratios.
- Flattened layout → restore ladder/rule-list/grid structural anchor.
- Off-contract styling (gradients, shadows, purple AI-wash) → replace with FDE Editorial tokens.
- Do **not** return `task_completed=true`; wait for independent verification.
