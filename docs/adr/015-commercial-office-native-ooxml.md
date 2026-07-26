# ADR 015: Commercial Office — Native Editable OOXML

## Status

Accepted (2026-07-23)

## Context

Office delivery had two competing paths:

1. **Raster PPTX** — HTML → Playwright screenshot → full-slide `<p:pic>` images. Looks OK in preview but **not editable** in PowerPoint.
2. **Sparse text-box PPTX** — python-pptx rebuild without brand master. Editable but visually poor.

Customers require **commercial-grade deliverables**: native OOXML files they can edit in PowerPoint, Word, or Excel without re-authoring from screenshots.

## Decision

Freeze a **three-layer** model:

| Layer | Role | Deliverable? |
|-------|------|--------------|
| **Authoring** | 1920×1080 HTML + `slide_manifest.json` | Workspace intermediate only |
| **Export** | lingqi potx/dotx + structured fill → native OOXML | **Only formal deliverable** |
| **Evidence** | Playwright `evidence/slide-*.png` | Gate / blind review only |

### PPT

- `presentation-design` (capability: `presentation.author`) — HTML, manifest, evidence PNGs.
- `presentation-commercial-delivery` (capability: `presentation.export.pptx`, priority 130) — `fill_potx.py` → editable `.pptx`.
- `presentation-html-delivery` — **deprecated**; wrapper calls design + commercial.
- `office-pptx` — fallback only (priority 50).

### DOCX / XLSX

- `document-delivery` — MD → branded `.docx` with header/footer + Heading styles; auto-builds `template.dotx` when missing.
- `spreadsheet-delivery` — ≥3 sheets (Summary, Detail, Pricing) with lingqi theme.

### Gates (P1)

| Validator | Rule |
|-----------|------|
| `office.pptx_editable` | Real `a:t` text (≥120 chars deck-wide); **reject** raster-only decks |
| `office.pptx_density` | ≥5 native `<p:sp>`/slide average; pics do not count |
| `office.pptx_render_thumbs` | Evidence PNGs from HTML (not PIL placeholders) |
| `office.docx_commercial` | Header/footer + Heading styles |
| `office.xlsx_style` | ≥3 worksheets + branded header fill |

### Eval promotion

- Primary metric: `editable_commercial_score` (includes `pptx_editable` pass/fail).
- vs baseline `office-20260722-223000`: delta ≥ +15 **and** all editable gates pass.
- HTML evidence blind review is **supplementary**; cannot override P1 machine failures.

## Consequences

- `scripts/office/html_slides_to_pptx.py` delegates to `fill_potx.py`; no longer produces raster `pitch.pptx` as primary output.
- TaskCompiler and Experience card `office.pptx-briefing` enforce design → commercial export workflow.
- Skill router selects `presentation-commercial-delivery` over `office-pptx` for `presentation.export.pptx`.
- Complex charts/speaker notes may still invoke Anthropic `pptx`/`docx`/`xlsx` skills, but **must** start from lingqi brand templates — never empty `Presentation()`.

## Out of scope (v1)

- 100MB photo-heavy decks; image slots use placeholders.
- Complex animations / embedded video.
- Full anthropics skill orchestration in Rust runtime (planned follow-up).

## References

- Plan: Commercial Office Route (frozen 2026-07-23)
- Scripts: `scripts/office/build_lingqi_potx.py`, `html_to_manifest.py`, `fill_potx.py`
- Skills: `skills-starter/presentation-design`, `presentation-commercial-delivery`
- Ops: `docs/ops/agent-quality-promotion.md`
