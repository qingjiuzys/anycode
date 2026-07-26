#!/usr/bin/env python3
"""Render ECharts spec to PNG via Playwright (evidence / PPTX placeholder)."""
from __future__ import annotations

import json
import sys
from pathlib import Path

HTML_TMPL = """<!DOCTYPE html>
<html><head><meta charset="utf-8"/>
<script src="https://cdn.jsdelivr.net/npm/echarts@5/dist/echarts.min.js"></script>
<style>html,body{margin:0;width:1280px;height:720px;background:#fff}</style>
</head><body><div id="c" style="width:1280px;height:720px"></div>
<script>
const spec = __SPEC__;
const chart = echarts.init(document.getElementById('c'));
chart.setOption(spec);
</script></body></html>"""


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: render_chart_png.py chart_spec.json output.png", file=sys.stderr)
        return 1
    spec_path = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2]).resolve()
    spec = json.loads(spec_path.read_text(encoding="utf-8"))
    if "option" in spec:
        option = spec["option"]
    elif "categories" in spec:
        option = {
            "title": {"text": spec.get("title", "")},
            "xAxis": {"type": "category", "data": spec.get("categories", [])},
            "yAxis": {"type": "value"},
            "series": [
                {"type": spec.get("type", "bar"), "data": s.get("data", s.get("values", []))}
                for s in (spec.get("series") or [])
            ],
        }
    else:
        option = spec
    html = HTML_TMPL.replace("__SPEC__", json.dumps(option, ensure_ascii=False))
    out.parent.mkdir(parents=True, exist_ok=True)
    tmp_html = out.with_suffix(".render.html")
    tmp_html.write_text(html, encoding="utf-8")
    try:
        from playwright.sync_api import sync_playwright

        with sync_playwright() as p:
            browser = p.chromium.launch()
            page = browser.new_page(viewport={"width": 1280, "height": 720})
            page.goto(tmp_html.as_uri(), wait_until="networkidle")
            page.wait_for_timeout(800)
            page.screenshot(path=str(out), full_page=True)
            browser.close()
    except Exception as exc:  # noqa: BLE001
        print(f"WARN: Playwright render failed ({exc}); writing stub PNG via PIL", file=sys.stderr)
        try:
            from PIL import Image, ImageDraw

            img = Image.new("RGB", (1280, 720), color=(238, 243, 248))
            draw = ImageDraw.Draw(img)
            draw.text((40, 40), spec.get("title", "Chart"), fill=(27, 58, 92))
            img.save(out)
        except ImportError:
            out.write_bytes(b"")
            return 1
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
