#!/usr/bin/env python3
"""Validate structured report Markdown for anycode-docx (FDE editorial)."""
from __future__ import annotations

import re
import sys
from pathlib import Path

FORBIDDEN = ("tbd", "lorem ipsum", "placeholder", "待补充", "xxx")

HEADING_RE = re.compile(r"^(#{1,3})\s+(.+)$")
DECISION_ACTION_RE = re.compile(
    r"^(Decision|Action|决策|行动)[:：]\s*.+",
    re.I,
)


def validate(path: Path, *, strict: bool = True) -> list[str]:
    text = path.read_text(encoding="utf-8")
    issues: list[str] = []
    lower = text.lower()
    for bad in FORBIDDEN:
        if bad in lower:
            issues.append(f"{path.name}: forbidden placeholder `{bad}`")

    h1 = h2 = 0
    decision_action = 0
    for line in text.splitlines():
        hm = HEADING_RE.match(line.strip())
        if hm:
            lvl = len(hm.group(1))
            if lvl == 1:
                h1 += 1
            elif lvl == 2:
                h2 += 1
        if DECISION_ACTION_RE.match(line.strip()):
            decision_action += 1

    if h1 < 1:
        issues.append(f"{path.name}: need ≥1 H1 title")
    if strict and h2 < 2:
        issues.append(f"{path.name}: need ≥2 H2 sections (copy anycode-docx/templates/)")
    if strict and decision_action < 1:
        issues.append(
            f"{path.name}: need ≥1 Decision:/Action: (or 决策:/行动:) with owner + date"
        )
    if len(text.strip()) < 200:
        issues.append(f"{path.name}: report too short — fill concrete facts")
    return issues


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_report_md.py report.md [anycode-docx]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    strict = len(sys.argv) > 2 and sys.argv[2] in ("anycode-docx", "anycode-pdf")
    if src.is_dir():
        files = sorted(src.glob("*.md"))
        files = [f for f in files if f.name.lower() not in ("readme.md", "components.md")]
    else:
        files = [src]
    all_issues: list[str] = []
    for f in files:
        all_issues.extend(validate(f, strict=strict))
    if all_issues:
        for i in all_issues:
            print(f"WARN: {i}", file=sys.stderr)
        return 1
    mode = f"{sys.argv[2]} strict" if strict and len(sys.argv) > 2 else "report md"
    print(f"OK: {len(files)} file(s) pass {mode}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
