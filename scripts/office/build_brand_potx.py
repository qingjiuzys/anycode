#!/usr/bin/env python3
"""Generate brand template.potx for any brand kit."""
from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "brand-kits" / "lib"))
sys.path.insert(0, str(REPO / "scripts" / "office"))

from brand_kit import find_brand_kit, load_tokens  # noqa: E402
from html_to_manifest import build_manifest  # noqa: E402
from pptx_slide_builder import build_deck  # noqa: E402


def main() -> int:
    brand = sys.argv[1] if len(sys.argv) > 1 else "fde-editorial"
    kit = find_brand_kit(brand)
    tokens = load_tokens(kit)
    template_dir = kit / "pptx"
    template_dir.mkdir(parents=True, exist_ok=True)
    out = template_dir / "template.potx"

    seed = REPO / "skills-starter" / "presentation-design" / "templates"
    sample_manifest = {
        "brand_kit": brand,
        "slides": [
            {"type": "cover", "title": "Title", "subtitle": "Subtitle", "meta": "Briefing", "chips": [{"number": "42%", "text": "Metric"}], "footer_left": tokens.get("name", brand), "footer_right": ""},
            {"type": "section", "title": "Section", "subtitle": "Overview", "agenda": ["A", "B", "C", "D"], "footer_left": tokens.get("name", brand), "footer_right": "02"},
            {"type": "content", "title": "Topic", "bullets": [{"stat": "37%", "text": "Point one", "source": "Source"}], "panel_title": "Impact", "panel_body": "Notes", "footer_left": tokens.get("name", brand), "footer_right": "03"},
            {"type": "metrics", "title": "Metrics", "stats": [{"number": "42%", "label": "L1", "detail": "D", "source": "S"}] * 6, "footer_left": tokens.get("name", brand), "footer_right": "04"},
            {"type": "closing", "title": "Next", "subtitle": "Summary", "actions": [{"text": "Action 1"}], "meta": "contact@example.com", "footer_left": tokens.get("name", brand), "footer_right": "End"},
        ],
    }
    if seed.is_dir() and len(list(seed.glob("*.html"))) >= 2:
        try:
            sample_manifest = build_manifest(seed, brand_kit=brand)
        except ValueError:
            pass

    prs = build_deck(sample_manifest, tokens)
    tmp = template_dir / "template.pptx"
    prs.save(str(tmp))
    tmp.replace(out)
    meta = {"brand_kit": brand, "layouts": ["cover", "section", "content", "metrics", "closing"], "path": str(out)}
    (template_dir / "template.meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
