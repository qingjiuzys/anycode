#!/usr/bin/env python3
"""Validate slide HTML content density against brand-kit layouts + anycode-ppt visual rules."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "brand-kits" / "lib"))
from brand_kit import find_brand_kit, load_pptx_layouts  # noqa: E402

DENSE_VISUAL = (
    "ladder",
    "layer-stack",
    "layer-stack-4",
    "agent-cycle",
    "evo-grid",
    "duo",
    "trio",
    "metrics",
    "timeline",
    "checklist",
    "diagram-box",
    "quote",
)

# FDE Editorial anti-patterns (lingqi / generic corporate deck slop)
FORBIDDEN_STYLE = (
    (r"linear-gradient", "gradient backgrounds forbidden (use flat #f2f5f0 / #231f20)"),
    (r"box-shadow", "card shadows forbidden"),
    (r"border-radius:\s*(?:1[0-9]|[2-9]\d)", "large border-radius forbidden (FDE uses hairline boxes)"),
    (r"#1[Bb]3[Aa]5[Cc]", "lingqi navy #1B3A5C forbidden — use fde-editorial tokens"),
    (r"#00[Bb]050", "lingqi green #00B050 forbidden — accent is #1400ff"),
    (r"\blingqi\b", "lingqi brand/footer forbidden unless user explicitly requested lingqi"),
)


def validate_fde_editorial_style(path: Path, html: str) -> list[str]:
    issues: list[str] = []
    for pat, msg in FORBIDDEN_STYLE:
        if re.search(pat, html, re.I):
            issues.append(f"{path.name}: {msg}")
    if not re.search(r"#f2f5f0|var\(--bg\)|--ink:\s*#231f20", html, re.I):
        issues.append(
            f"{path.name}: missing FDE canvas tokens (#f2f5f0 / --bg / --ink) — copy anycode-ppt/templates/"
        )
    if not re.search(
        r"sec-label|class=[\"'][^\"']*\bladder\b|layer-stack|Songti|Noto Serif",
        html,
        re.I,
    ):
        issues.append(
            f"{path.name}: missing FDE editorial markers (sec-label / ladder / layer-stack / serif title)"
        )
    return issues


def slide_type(html: str) -> str:
    m = re.search(r'data-type=["\']([^"\']+)["\']', html, re.I)
    return (m.group(1).lower() if m else "content").lower()


def count_stats(html: str) -> int:
    return len(re.findall(r'class=["\'][^"\']*\bstat\b|class=["\'][^"\']*\bnumber\b', html, re.I))


def count_list_items(html: str) -> int:
    li = len(re.findall(r"<li\b", html, re.I))
    agenda = len(re.findall(r'class=["\'][^"\']*\bitem\b', html, re.I))
    return li + agenda


def count_panels(html: str) -> int:
    return len(
        re.findall(
            r'class=["\'][^"\']*\b(panel|quote|chip|card|cta-box|contact|side|agenda|item|rung|layer-card|cycle-node|cycle-core|mile|check-item|stat)\b',
            html,
            re.I,
        )
    )


def has_dense_visual(html: str) -> bool:
    if re.search(r"<img\b", html, re.I):
        return True
    return any(re.search(rf'class=["\'][^"\']*\b{pat}\b', html, re.I) for pat in DENSE_VISUAL)


def validate_dense_visual(path: Path, html: str) -> list[str]:
    st = slide_type(html)
    if st in ("cover", "section", "closing"):
        if st == "cover" and not (has_dense_visual(html) or "tag-row" in html or "ladder" in html):
            return [f"{path.name}: cover needs ladder or tag-row"]
        if st == "section" and count_list_items(html) < 2:
            return [f"{path.name}: section needs ≥2 agenda items"]
        if st == "closing" and not has_dense_visual(html):
            return [f"{path.name}: closing needs trio/card summary blocks"]
        return []
    if not has_dense_visual(html):
        return [
            f"{path.name}: content slide missing editorial visual "
            f"(need one of: {', '.join(DENSE_VISUAL)})"
        ]
    return []


def validate_file(path: Path, layouts: dict, *, dense_mode: bool) -> list[str]:
    html = path.read_text(encoding="utf-8")
    issues: list[str] = []
    if dense_mode:
        issues.extend(validate_dense_visual(path, html))
        issues.extend(validate_fde_editorial_style(path, html))
    st = slide_type(html)
    rules = (layouts.get("content_density") or {}).get(st) or {}
    if rules:
        li = count_list_items(html)
        stats = count_stats(html)
        panels = count_panels(html)
        skip_lists = dense_mode and (
            (st == "content" and has_dense_visual(html))
            or (st == "closing" and ("trio" in html or panels >= 3))
        )
        if not skip_lists and li < rules.get("min_list_items", 0):
            issues.append(f"{path.name}: type={st} needs ≥{rules['min_list_items']} list items, found {li}")
        if stats < rules.get("min_stat_spans", 0):
            issues.append(
                f"{path.name}: type={st} needs ≥{rules['min_stat_spans']} stat/number blocks, found {stats}"
            )
        if panels < rules.get("min_side_panels", 0):
            issues.append(f"{path.name}: type={st} needs ≥{rules['min_side_panels']} panels/cards, found {panels}")
    return issues


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_slide_html.py slides_dir [brand_kit|anycode-ppt]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    kit_arg = sys.argv[2] if len(sys.argv) > 2 else "fde-editorial"
    dense_mode = kit_arg == "anycode-ppt"
    brand_name = "fde-editorial" if dense_mode else kit_arg
    kit = find_brand_kit(brand_name)
    layouts = load_pptx_layouts(kit)
    files = sorted(src.glob("*.html"))
    if not files and (src / "slides").is_dir():
        files = sorted((src / "slides").glob("*.html"))
    all_issues: list[str] = []
    for f in files:
        all_issues.extend(validate_file(f, layouts, dense_mode=dense_mode))
    if all_issues:
        for i in all_issues:
            print(f"WARN: {i}", file=sys.stderr)
        return 1
    mode = "anycode-ppt visual + density" if dense_mode else "content density"
    print(f"OK: {len(files)} slides pass {mode}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
