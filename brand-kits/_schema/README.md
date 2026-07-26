# Brand-kit schema

Each brand kit lives at `brand-kits/{id}/` with:

| File | Required | Purpose |
|------|----------|---------|
| `tokens.json` | yes | Colors, fonts, spacing, slide size |
| `pptx/layouts.json` | yes | Slide type density rules + footer text |
| `pptx/template.potx` | generated | Run `python3 scripts/office/build_brand_potx.py {id}` |
| `docx/template.dotx` | generated | Run `python3 scripts/office/build_brand_dotx.py {id}` |
| `xlsx/theme.json` | yes | Header fills, sheet names, status colors |

Discovery: `ANYCODE_BRAND_KITS_DIR/{id}/tokens.json` or repo `brand-kits/{id}/tokens.json`.

Built-in kits: `lingqi` (enterprise), `gov-formal` (government), `edu-clean` (education).
