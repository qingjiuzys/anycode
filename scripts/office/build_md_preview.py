#!/usr/bin/env python3
"""Render FDE editorial HTML preview from structured report Markdown."""
from __future__ import annotations

import html
import re
import sys
from pathlib import Path

HEADING_RE = re.compile(r"^(#{1,3})\s+(.+)$")
BULLET_RE = re.compile(r"^[-*]\s+(.+)$")
TABLE_SEP = re.compile(r"^\|?[\s:-]+\|[\s|:-]+\|?$")
DA_RE = re.compile(r"^(Decision|Action|决策|行动)[:：]", re.I)

CSS = """
:root{--bg:#f2f5f0;--ink:#231f20;--accent:#1400ff;--ink-60:rgba(35,31,32,.72);--ink-08:rgba(35,31,32,.08);
  --serif:"Songti SC","Noto Serif SC",serif;--sans:"PingFang SC",sans-serif;--mono:"JetBrains Mono",monospace}
*{box-sizing:border-box}
body{margin:0;font-family:var(--sans);background:var(--bg);color:var(--ink);line-height:1.7;font-size:16px}
.wrap{max-width:920px;margin:0 auto;padding:48px 32px 80px}
.sec-label{font-family:var(--mono);font-size:12px;letter-spacing:.16em;text-transform:uppercase;color:var(--ink-60);
  display:flex;align-items:center;gap:12px;margin:48px 0 20px}
.sec-label::after{content:"";flex:1;height:4px;background:var(--ink);max-width:320px}
h1{font-family:var(--serif);font-weight:900;font-size:42px;line-height:1.2;margin:0 0 24px;border-bottom:6px solid var(--ink);padding-bottom:16px}
h2{font-family:var(--serif);font-weight:900;font-size:26px;margin:32px 0 12px;color:var(--ink)}
h3{font-family:var(--serif);font-weight:700;font-size:18px;margin:20px 0 8px}
p{margin:0 0 14px;color:var(--ink)}
ul{margin:0 0 16px;padding-left:0;list-style:none;border:1px solid var(--ink-08)}
li{padding:10px 14px;border-bottom:1px solid var(--ink-08)}
li:last-child{border-bottom:none}
.da{background:var(--ink);color:var(--bg);padding:14px 18px;margin:16px 0;font-family:var(--mono);font-size:14px;border-left:6px solid var(--accent)}
.da b{color:var(--accent)}
table{width:100%;border-collapse:collapse;margin:16px 0;border:1px solid var(--ink)}
th,td{border:1px solid var(--ink);padding:10px 12px;text-align:left;font-size:14px}
th{background:var(--ink);color:var(--bg);font-family:var(--mono);font-size:12px;letter-spacing:.08em}
.meta{font-family:var(--mono);font-size:12px;color:var(--ink-60);margin-bottom:32px}
footer{margin-top:48px;padding-top:16px;border-top:1px solid var(--ink);font-family:var(--mono);font-size:11px;color:var(--ink-60)}
"""


def md_to_html(text: str, title: str) -> str:
    parts: list[str] = []
    in_table = False
    table_rows: list[list[str]] = []

    def flush_table():
        nonlocal in_table, table_rows
        if not table_rows:
            return
        parts.append("<table><thead><tr>")
        for c in table_rows[0]:
            parts.append(f"<th>{html.escape(c)}</th>")
        parts.append("</tr></thead><tbody>")
        for row in table_rows[1:]:
            parts.append("<tr>")
            for c in row:
                parts.append(f"<td>{html.escape(c)}</td>")
            parts.append("</tr>")
        parts.append("</tbody></table>")
        table_rows = []
        in_table = False

    for raw in text.splitlines():
        line = raw.rstrip()
        if line.strip().startswith("|") and "|" in line:
            cells = [c.strip() for c in line.strip("|").split("|")]
            if TABLE_SEP.match(line.replace(" ", "")):
                continue
            if not in_table:
                in_table = True
            table_rows.append(cells)
            continue
        if in_table:
            flush_table()
        if not line.strip():
            continue
        hm = HEADING_RE.match(line.strip())
        if hm:
            lvl = len(hm.group(1))
            t = html.escape(hm.group(2).strip())
            if lvl == 1:
                parts.append(f"<h1>{t}</h1>")
            elif lvl == 2:
                parts.append(f'<div class="sec-label">SECTION</div><h2>{t}</h2>')
            else:
                parts.append(f"<h3>{t}</h3>")
            continue
        bm = BULLET_RE.match(line.strip())
        if bm:
            parts.append(f"<ul><li>{html.escape(bm.group(1))}</li></ul>")
            continue
        body = html.escape(line.strip())
        if DA_RE.match(line.strip()):
            label = line.split(":", 1)[0].split("：", 1)[0]
            rest = line.split(":", 1)[-1] if ":" in line else line.split("：", 1)[-1]
            parts.append(f'<div class="da"><b>{html.escape(label.strip())}:</b> {html.escape(rest.strip())}</div>')
        else:
            parts.append(f"<p>{body}</p>")
    if in_table:
        flush_table()

    body_html = "\n".join(parts)
    return f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{html.escape(title)}</title>
  <style>{CSS}</style>
</head>
<body>
  <div class="wrap">
    <div class="meta">FDE Editorial · anycode-docx preview</div>
    {body_html}
    <footer>Source: Markdown report · edit .md and re-run anycode-docx/run</footer>
  </div>
</body>
</html>
"""


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: build_md_preview.py report.md [preview.html]", file=sys.stderr)
        return 1
    md = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else md.with_suffix(".preview.html")
    text = md.read_text(encoding="utf-8")
    title = md.stem.replace("-", " ").replace("_", " ")
    for line in text.splitlines():
        hm = HEADING_RE.match(line.strip())
        if hm and len(hm.group(1)) == 1:
            title = hm.group(2).strip()
            break
    out.write_text(md_to_html(text, title), encoding="utf-8")
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
