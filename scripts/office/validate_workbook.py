#!/usr/bin/env python3
"""Validate workbook JSON / CSV for anycode-xlsx (FDE editorial)."""
from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path

FORBIDDEN = ("tbd", "lorem ipsum", "placeholder", "待填", "xxx")


def load_workbook(path: Path) -> dict:
    if path.suffix.lower() == ".json":
        return json.loads(path.read_text(encoding="utf-8"))
    if path.suffix.lower() == ".csv":
        with path.open(newline="", encoding="utf-8") as f:
            rows = [list(r) for r in csv.reader(f) if any(c.strip() for c in r)]
        return {
            "sheets": [
                {"name": "Detail", "rows": rows},
                {"name": "Summary", "rows": [["Metric", "Value"], ["Rows", str(max(0, len(rows) - 1))]]},
                {"name": "Notes", "rows": [["Note", "Derived from CSV"], ["Source", path.name]]},
            ]
        }
    raise ValueError(f"unsupported: {path.suffix}")


def validate_spec(spec: dict, path: Path, *, strict: bool) -> list[str]:
    issues: list[str] = []
    blob = json.dumps(spec, ensure_ascii=False).lower()
    for bad in FORBIDDEN:
        if bad in blob:
            issues.append(f"{path.name}: forbidden placeholder `{bad}`")

    sheets = spec.get("sheets") or []
    if strict and len(sheets) < 3:
        issues.append(f"{path.name}: need ≥3 sheets (Summary / Detail / …) — copy anycode-xlsx/templates/")
    data_rows = 0
    for sh in sheets:
        rows = sh.get("rows") or []
        if len(rows) < 2:
            issues.append(f"{path.name}: sheet `{sh.get('name', '?')}` needs header + ≥1 data row")
            continue
        header = rows[0]
        if not any(str(c).strip() for c in header):
            issues.append(f"{path.name}: sheet `{sh.get('name', '?')}` empty header")
        data_rows += len(rows) - 1
    if strict and data_rows < 6:
        issues.append(f"{path.name}: need ≥6 total data rows across sheets (concrete numbers)")
    return issues


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_workbook.py workbook.json|data.csv [anycode-xlsx]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    strict = len(sys.argv) > 2 and sys.argv[2] == "anycode-xlsx"
    if src.is_dir():
        files = sorted(list(src.glob("*.json")) + list(src.glob("*.csv")))
        files = [f for f in files if f.name != "workbook.json" or len(files) == 1]
        primary = src / "workbook.json"
        if primary.is_file():
            files = [primary]
    else:
        files = [src]
    all_issues: list[str] = []
    for f in files:
        try:
            spec = load_workbook(f)
        except (json.JSONDecodeError, ValueError) as e:
            all_issues.append(f"{f.name}: {e}")
            continue
        all_issues.extend(validate_spec(spec, f, strict=strict))
    if all_issues:
        for i in all_issues:
            print(f"WARN: {i}", file=sys.stderr)
        return 1
    print(f"OK: {len(files)} workbook(s) pass anycode-xlsx")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
