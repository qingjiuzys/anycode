#!/usr/bin/env python3
"""Build index.html deck viewer for a directory of 1920×1080 slide HTML files."""
from __future__ import annotations

import json
import sys
from pathlib import Path

INDEX_HTML = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root{{--bg:#0e0e10;--panel:#1a1a1f;--ink:#f2f5f0;--muted:rgba(242,245,240,.55);--accent:#1400ff}}
    *{{box-sizing:border-box}}
    body{{margin:0;font-family:"PingFang SC",system-ui,sans-serif;background:var(--bg);color:var(--ink);height:100vh;display:grid;grid-template-rows:auto 1fr auto}}
    header{{display:flex;align-items:center;gap:16px;padding:12px 20px;background:var(--panel);border-bottom:1px solid rgba(255,255,255,.08)}}
    header h1{{font-size:15px;font-weight:600;margin:0;flex:1}}
    header .meta{{font-size:12px;color:var(--muted);font-family:monospace}}
    main{{display:grid;grid-template-columns:220px 1fr;min-height:0}}
    nav{{overflow:auto;padding:12px;background:var(--panel);border-right:1px solid rgba(255,255,255,.08)}}
    nav a{{display:block;padding:8px 10px;margin-bottom:4px;font-size:13px;color:var(--muted);text-decoration:none;border:1px solid transparent}}
    nav a:hover,nav a.active{{color:var(--ink);border-color:rgba(20,0,255,.5);background:rgba(20,0,255,.12)}}
    nav a .idx{{color:var(--accent);font-family:monospace;margin-right:6px}}
    .stage{{display:flex;align-items:center;justify-content:center;padding:16px;min-height:0;background:#000}}
    .frame-wrap{{width:100%;max-width:1280px;aspect-ratio:16/9;border:1px solid rgba(255,255,255,.12);background:#111}}
    iframe{{width:100%;height:100%;border:0;display:block}}
    footer{{display:flex;align-items:center;justify-content:center;gap:12px;padding:10px;background:var(--panel);font-size:13px;color:var(--muted)}}
    footer button{{background:var(--accent);color:#fff;border:0;padding:8px 16px;font-size:13px;cursor:pointer}}
    footer button:disabled{{opacity:.35;cursor:default}}
    footer kbd{{font-family:monospace;background:rgba(255,255,255,.08);padding:2px 6px;border-radius:2px}}
  </style>
</head>
<body>
  <header>
    <h1>{title}</h1>
    <span class="meta">{count} slides · FDE Editorial HTML deck</span>
  </header>
  <main>
    <nav id="nav">{nav_links}</nav>
    <div class="stage"><div class="frame-wrap"><iframe id="frame" title="slide"></iframe></div></div>
  </main>
  <footer>
    <button id="prev" type="button">← 上一页</button>
    <span id="pos">1 / {count}</span>
    <button id="next" type="button">下一页 →</button>
    <span><kbd>←</kbd> <kbd>→</kbd> 翻页 · <kbd>F</kbd> 新标签打开当前页</span>
  </footer>
  <script>
    const slides = {slides_json};
    let i = 0;
    const frame = document.getElementById('frame');
    const pos = document.getElementById('pos');
    const nav = document.getElementById('nav');
    const links = [...nav.querySelectorAll('a')];
    function show(n) {{
      i = Math.max(0, Math.min(slides.length - 1, n));
      frame.src = slides[i];
      pos.textContent = (i + 1) + ' / ' + slides.length;
      links.forEach((a, j) => a.classList.toggle('active', j === i));
      document.getElementById('prev').disabled = i === 0;
      document.getElementById('next').disabled = i === slides.length - 1;
    }}
    document.getElementById('prev').onclick = () => show(i - 1);
    document.getElementById('next').onclick = () => show(i + 1);
    links.forEach((a, j) => a.addEventListener('click', e => {{ e.preventDefault(); show(j); }}));
    window.addEventListener('keydown', e => {{
      if (e.key === 'ArrowLeft') show(i - 1);
      if (e.key === 'ArrowRight') show(i + 1);
      if (e.key === 'f' || e.key === 'F') window.open(slides[i], '_blank');
    }});
    show(0);
  </script>
</body>
</html>
"""


def collect_slides(src: Path) -> list[Path]:
    if (src / "slides").is_dir():
        src = src / "slides"
    files = sorted(src.glob("*.html"))
    return [f for f in files if f.name.lower() != "index.html"]


def build_index(src: Path, out: Path | None = None) -> Path:
    slides = collect_slides(src)
    if len(slides) < 1:
        raise SystemExit(f"no slide HTML in {src}")
    deck_dir = slides[0].parent
    rel_slides = [s.name for s in slides]
    nav = "\n".join(
        f'<a href="{name}"><span class="idx">{i:02d}</span>{name}</a>'
        for i, name in enumerate(rel_slides, start=1)
    )
    title = deck_dir.name if deck_dir.name not in (".", "slides") else "Slide Deck"
    html = INDEX_HTML.format(
        title=title,
        count=len(rel_slides),
        nav_links=nav,
        slides_json=json.dumps(rel_slides, ensure_ascii=False),
    )
    dest = out or deck_dir / "index.html"
    dest.write_text(html, encoding="utf-8")
    return dest


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: build_slide_deck_index.py slides_dir [index.html]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else None
    dest = build_index(src, out)
    print(dest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
