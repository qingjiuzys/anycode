#!/usr/bin/env python3
"""Build a branded multi-sheet .xlsx from CSV/MD (lingqi theme)."""
from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path

try:
    from openpyxl import Workbook
    from openpyxl.styles import Alignment, Font, PatternFill
except ImportError:
    print("ERROR: openpyxl not installed. Run: pip install openpyxl", file=sys.stderr)
    sys.exit(2)

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "brand-kits" / "lib"))
from brand_kit import emit_artifact, find_brand_kit, load_xlsx_theme  # noqa: E402


def parse_markdown_table(text: str) -> list[list[str]]:
    rows: list[list[str]] = []
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if cells and all(re.fullmatch(r":?-{3,}:?", c or "") for c in cells):
            continue
        if cells:
            rows.append(cells)
    return rows


def load_rows(path: Path) -> list[list[str]]:
    if path.suffix.lower() == ".csv":
        with path.open(newline="", encoding="utf-8") as f:
            return [list(r) for r in csv.reader(f) if any(c.strip() for c in r)]
    text = path.read_text(encoding="utf-8")
    rows = parse_markdown_table(text)
    if not rows:
        for line in text.splitlines():
            if "\t" in line:
                rows.append([c.strip() for c in line.split("\t")])
            elif "," in line and not line.startswith("#"):
                rows.append([c.strip() for c in line.split(",")])
    return rows


def coerce(cell):
    if isinstance(cell, (int, float)):
        return cell
    s = str(cell).strip()
    if re.fullmatch(r"-?\d+", s):
        return int(s)
    if re.fullmatch(r"-?\d+\.\d+", s):
        return float(s)
    return s


def style_sheet(ws, theme: dict, rows: list[list[str]]):
    header = theme.get("header") or {}
    fill_hex = header.get("fill", "1B3A5C")
    font_hex = header.get("font_color", "FFFFFF")
    header_fill = PatternFill("solid", fgColor=fill_hex)
    header_font = Font(bold=True, color=font_hex, name=header.get("font_name", "Arial"))
    if not rows:
        return
    for c_i, val in enumerate(rows[0], start=1):
        cell = ws.cell(row=1, column=c_i, value=val)
        cell.fill = header_fill
        cell.font = header_font
        cell.alignment = Alignment(horizontal="center", vertical="center")
    for r_i, row in enumerate(rows[1:], start=2):
        for c_i, val in enumerate(row, start=1):
            ws.cell(row=r_i, column=c_i, value=coerce(val))
    ws.freeze_panes = "A2"


def add_native_charts(wb, spec: dict) -> None:
    charts = spec.get("charts") or []
    if not charts:
        return
    try:
        from openpyxl.chart import BarChart, Reference
    except ImportError:
        return
    ws = wb.create_sheet(title="Charts")
    row = 1
    for ch in charts:
        ws.cell(row=row, column=1, value=ch.get("title", "Chart"))
        row += 1
        categories = ch.get("categories") or []
        series_list = ch.get("series") or []
        if not categories or not series_list:
            continue
        header = ["Category"] + [s.get("name", f"S{i+1}") for i, s in enumerate(series_list)]
        for c_i, h in enumerate(header, start=1):
            ws.cell(row=row, column=c_i, value=h)
        row += 1
        start = row
        for cat_i, cat in enumerate(categories):
            ws.cell(row=row, column=1, value=cat)
            for s_i, s in enumerate(series_list, start=2):
                vals = s.get("data") or s.get("values") or []
                val = vals[cat_i] if cat_i < len(vals) else 0
                ws.cell(row=row, column=s_i, value=val)
            row += 1
        chart = BarChart()
        chart.title = ch.get("title", "Chart")
        data = Reference(ws, min_col=2, min_row=start - 1, max_row=row - 1, max_col=1 + len(series_list))
        cats = Reference(ws, min_col=1, min_row=start, max_row=row - 1)
        chart.add_data(data, titles_from_data=True)
        chart.set_categories(cats)
        ws.add_chart(chart, f"{chr(65 + len(header) + 1)}2")
        row += 2


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: run table.csv|table.md|workbook.json [output.xlsx] [brand_kit]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    out_path = (
        Path(sys.argv[2]).resolve()
        if len(sys.argv) > 2
        else src.with_suffix(".xlsx")
    )
    brand = sys.argv[3] if len(sys.argv) > 3 else "fde-editorial"
    theme = load_xlsx_theme(find_brand_kit(brand))

    wb = Workbook()
    wb.remove(wb.active)

    if src.suffix.lower() == ".json":
        spec = json.loads(src.read_text(encoding="utf-8"))
        sheets = spec.get("sheets") or []
        if len(sheets) < 3:
            print("ERROR: workbook.json needs ≥3 sheets for commercial path", file=sys.stderr)
            return 1
        for sh in sheets:
            name = sh.get("name", "Sheet")
            rows = sh.get("rows") or []
            ws = wb.create_sheet(title=name[:31])
            style_sheet(ws, theme, rows)
        add_native_charts(wb, spec)
    else:
        rows = load_rows(src)
        if len(rows) < 2:
            print("ERROR: need header + ≥1 data row", file=sys.stderr)
            return 1
        blob = " ".join(" ".join(r) for r in rows).lower()
        for bad in ("tbd", "lorem ipsum", "placeholder"):
            if bad in blob:
                print(f"ERROR: forbidden placeholder `{bad}`", file=sys.stderr)
                return 1
        names = theme.get("defaults", {}).get("sheet_names", ["Summary", "Detail", "Pricing"])
        ws1 = wb.create_sheet(title=names[0][:31])
        style_sheet(ws1, theme, rows)
        ws2 = wb.create_sheet(title=names[1][:31])
        summary_rows = [
            ["Metric", "Value"],
            ["Rows", str(len(rows) - 1)],
            ["Columns", str(len(rows[0]))],
            ["Total Units", str(sum(int(r[2]) for r in rows[1:] if len(r) > 2 and str(r[2]).replace("-", "").isdigit()))],
        ]
        style_sheet(ws2, theme, summary_rows)
        ws3 = wb.create_sheet(title=names[2][:31])
        pricing_header = theme.get("pricing_header") or theme.get("header") or {}
        # Derive the third sheet from the input data (per-column numeric
        # totals) — never ship hardcoded fake pricing tiers to users.
        third_rows = [["Column", "Numeric Total", "Status"]]
        header = rows[0]
        for ci, col_name in enumerate(header):
            total = 0.0
            seen = 0
            for r in rows[1:]:
                if ci < len(r):
                    cell = str(r[ci]).replace(",", "").strip()
                    try:
                        total += float(cell)
                        seen += 1
                    except ValueError:
                        continue
            if seen:
                third_rows.append([str(col_name), f"{total:g}", "Active"])
        if len(third_rows) == 1:
            third_rows.append(["(no numeric columns)", "—", "Review"])
        style_sheet(ws3, {**theme, "header": pricing_header}, third_rows)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    wb.save(str(out_path))
    emit_artifact(out_path, "spreadsheet")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
