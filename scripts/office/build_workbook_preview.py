#!/usr/bin/env python3
"""Render FDE editorial HTML preview from workbook JSON."""
from __future__ import annotations

import html
import json
import sys
from pathlib import Path

CSS = """
:root{--bg:#f2f5f0;--ink:#231f20;--accent:#1400ff;--ink-60:rgba(35,31,32,.72);
  --serif:"Songti SC","Noto Serif SC",serif;--sans:"PingFang SC",sans-serif;--mono:"JetBrains Mono",monospace}
body{margin:0;font-family:var(--sans);background:var(--bg);color:var(--ink);padding:40px 32px 64px}
h1{font-family:var(--serif);font-weight:900;font-size:36px;margin:0 0 8px;border-bottom:6px solid var(--ink);padding-bottom:12px}
.meta{font-family:var(--mono);font-size:12px;color:var(--ink-60);margin-bottom:32px}
.sheet{margin-bottom:40px}
.sheet h2{font-family:var(--mono);font-size:13px;letter-spacing:.14em;text-transform:uppercase;color:var(--accent);margin:0 0 12px}
table{width:100%;max-width:1100px;border-collapse:collapse;border:1px solid var(--ink);margin-bottom:8px}
th,td{border:1px solid var(--ink);padding:8px 12px;font-size:14px;text-align:left}
th{background:var(--ink);color:#f2f5f0;font-family:var(--mono);font-size:11px;letter-spacing:.06em}
tr:nth-child(even) td{background:rgba(255,255,255,.35)}
footer{font-family:var(--mono);font-size:11px;color:var(--ink-60);margin-top:32px;border-top:1px solid var(--ink);padding-top:12px}
"""


def sheet_html(name: str, rows: list[list]) -> str:
    if not rows:
        return ""
    parts = [f'<section class="sheet"><h2>{html.escape(name)}</h2><table>']
    parts.append("<thead><tr>" + "".join(f"<th>{html.escape(str(c))}</th>" for c in rows[0]) + "</tr></thead>")
    parts.append("<tbody>")
    for row in rows[1:]:
        parts.append("<tr>" + "".join(f"<td>{html.escape(str(c))}</td>" for c in row) + "</tr>")
    parts.append("</tbody></table></section>")
    return "".join(parts)


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: build_workbook_preview.py workbook.json [preview.html]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else src.with_suffix(".preview.html")
    spec = json.loads(src.read_text(encoding="utf-8"))
    title = spec.get("title") or src.stem.replace("-", " ").replace("_", " ")
    sheets = spec.get("sheets") or []
    body = "".join(sheet_html(sh.get("name", "Sheet"), sh.get("rows") or []) for sh in sheets)
    page = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>{html.escape(title)}</title><style>{CSS}</style></head>
<body>
<h1>{html.escape(title)}</h1>
<div class="meta">FDE Editorial · anycode-xlsx preview · {len(sheets)} sheet(s)</div>
{body}
<footer>Source: workbook.json · edit JSON and re-run anycode-xlsx/run</footer>
</body></html>"""
    out.write_text(page, encoding="utf-8")
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
