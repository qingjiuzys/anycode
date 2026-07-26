#!/usr/bin/env python3
"""Validate slide HTML content density against brand-kit layouts.json contracts."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "brand-kits" / "lingqi" / "lib"))
from brand_kit import find_brand_kit, load_pptx_layouts  # noqa: E402


def slide_type(html: str) -> str:
    m = re.search(r'data-type=["\']([^"\']+)["\']', html, re.I)
    return (m.group(1).lower() if m else "content").lower()


def count_stats(html: str) -> int:
    return len(re.findall(r'class=["\'][^"\']*\bstat\b|class=["\'][^"\']*\bnumber\b', html, re.I))


def count_list_items(html: str) -> int:
    return len(re.findall(r"<li\b", html, re.I))


def count_panels(html: str) -> int:
    return len(
        re.findall(
            r'class=["\'][^"\']*\b(panel|quote|chip|card|cta-box|contact|side)\b',
            html,
            re.I,
        )
    )


def validate_file(path: Path, layouts: dict) -> list[str]:
    html = path.read_text(encoding="utf-8")
    st = slide_type(html)
    rules = (layouts.get("content_density") or {}).get(st) or {}
    if not rules:
        return []
    issues: list[str] = []
    li = count_list_items(html)
    stats = count_stats(html)
    panels = count_panels(html)
    if li < rules.get("min_list_items", 0):
        issues.append(f"{path.name}: type={st} needs ≥{rules['min_list_items']} list items, found {li}")
    if stats < rules.get("min_stat_spans", 0):
        issues.append(f"{path.name}: type={st} needs ≥{rules['min_stat_spans']} stat/number blocks, found {stats}")
    if panels < rules.get("min_side_panels", 0):
        issues.append(f"{path.name}: type={st} needs ≥{rules['min_side_panels']} panels/cards, found {panels}")
    return issues


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_slide_html.py slides_dir [brand_kit]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    kit = find_brand_kit(sys.argv[2] if len(sys.argv) > 2 else "lingqi")
    layouts = load_pptx_layouts(kit)
    files = sorted(src.glob("*.html"))
    if not files and (src / "slides").is_dir():
        files = sorted((src / "slides").glob("*.html"))
    all_issues: list[str] = []
    for f in files:
        all_issues.extend(validate_file(f, layouts))
    if all_issues:
        for i in all_issues:
            print(f"WARN: {i}", file=sys.stderr)
        return 1
    print(f"OK: {len(files)} slides pass content density")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
