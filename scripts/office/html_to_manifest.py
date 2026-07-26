#!/usr/bin/env python3
"""Extract slide_manifest.json from branded HTML slides."""
from __future__ import annotations

import json
import re
import sys
from html import unescape
from pathlib import Path


def _strip(html: str) -> str:
    html = re.sub(r"<script[^>]*>.*?</script>", "", html, flags=re.S | re.I)
    html = re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.S | re.I)
    text = re.sub(r"<[^>]+>", " ", html)
    return unescape(re.sub(r"\s+", " ", text)).strip()


def _first(pattern: str, html: str) -> str:
    m = re.search(pattern, html, re.S | re.I)
    return unescape(m.group(1)).strip() if m else ""


def _slide_type(html: str, idx: int, total: int) -> str:
    m = re.search(r'data-type=["\']([^"\']+)["\']', html, re.I)
    if m:
        return m.group(1).lower()
    if idx == 0:
        return "cover"
    if idx == total - 1:
        return "closing"
    return "content"


def _bullets(html: str) -> list[dict]:
    out: list[dict] = []
    for block in re.findall(r"<li[^>]*>(.*?)</li>", html, re.S | re.I):
        stat = _first(r'class=["\'][^"\']*stat[^"\']*["\'][^>]*>([^<]+)', block) or _first(
            r'class=["\'][^"\']*number[^"\']*["\'][^>]*>([^<]+)', block
        )
        source = _first(r'class=["\'][^"\']*source[^"\']*["\'][^>]*>([^<]+)', block)
        text = _strip(re.sub(r"<span[^>]*class=[^>]*source[^>]*>.*", "", block, flags=re.S | re.I))
        text = re.sub(r"^\W+", "", text)
        if text:
            out.append({"stat": stat, "text": text, "source": source})
    return out


def _chips(html: str) -> list[dict]:
    chips: list[dict] = []
    for block in re.findall(r'class=["\'][^"\']*chip[^"\']*["\'][^>]*>(.*?)</div>', html, re.S | re.I):
        num = _first(r'class=["\'][^"\']*n[^"\']*["\'][^>]*>([^<]+)', block)
        txt = _first(r'class=["\'][^"\']*t[^"\']*["\'][^>]*>([^<]+)', block)
        if num or txt:
            chips.append({"number": num, "text": txt})
    return chips


def _stats(html: str) -> list[dict]:
    stats: list[dict] = []
    for card in re.findall(r'<div class="card"[^>]*>(.*?)</div>\s*</div>', html, re.S | re.I):
        number = _first(r'class=["\']number["\'][^>]*>([^<]+)', card)
        label = _first(r'class=["\']label["\'][^>]*>([^<]+)', card)
        detail = _first(r'class=["\']detail["\'][^>]*>([^<]+)', card)
        source = _first(r'class=["\']source["\'][^>]*>([^<]+)', card)
        if number or label:
            stats.append({"number": number, "label": label, "detail": detail, "source": source})
    if not stats:
        for li in _bullets(html):
            if li.get("stat"):
                stats.append(
                    {
                        "number": li["stat"],
                        "label": li["text"][:80],
                        "detail": "",
                        "source": li.get("source", ""),
                    }
                )
    return stats


def _agenda(html: str) -> list[str]:
    items: list[str] = []
    for block in re.findall(r"<li[^>]*>(.*?)</li>", html, re.S | re.I):
        t = _strip(re.sub(r'<span class="num">.*?</span>', "", block, flags=re.S))
        if t:
            items.append(t)
    return items


def _charts(html: str) -> list[dict]:
    charts: list[dict] = []
    for m in re.finditer(r'data-chart=["\'](\{.*?\})["\']', html, re.S | re.I):
        try:
            charts.append(json.loads(m.group(1)))
        except json.JSONDecodeError:
            pass
    for m in re.finditer(r'<script[^>]*type=["\']application/json["\'][^>]*id=["\']chart-spec["\'][^>]*>(.*?)</script>', html, re.S | re.I):
        try:
            spec = json.loads(m.group(1).strip())
            if isinstance(spec, list):
                charts.extend(spec)
            elif isinstance(spec, dict):
                charts.append(spec)
        except json.JSONDecodeError:
            pass
    return charts


def parse_slide(html: str, idx: int, total: int, source: str) -> dict:
    st = _slide_type(html, idx, total)
    slide: dict = {"type": st, "source_html": source}
    slide["title"] = _first(r"<h1[^>]*>(.*?)</h1>", html) or _first(r"<title[^>]*>(.*?)</title>", html)
    slide["subtitle"] = _first(r'class=["\']subtitle["\'][^>]*>(.*?)</', html)
    slide["meta"] = _first(r'class=["\']meta["\'][^>]*>(.*?)</', html)
    slide["footer_left"] = _first(r'class=["\']footer["\'][^>]*>\s*<span>([^<]+)', html)
    slide["footer_right"] = ""
    fm = re.search(r'class=["\']footer["\'][^>]*>.*?<span>[^<]+</span>\s*<span>([^<]+)', html, re.S | re.I)
    if fm:
        slide["footer_right"] = unescape(fm.group(1)).strip()
    slide["bullets"] = _bullets(html)
    slide["chips"] = _chips(html)
    slide["stats"] = _stats(html)
    slide["agenda"] = _agenda(html)
    slide["panel_title"] = _first(r'class=["\']panel["\'][^>]*>\s*<h2[^>]*>(.*?)</h2>', html)
    slide["panel_body"] = _first(r'class=["\']panel["\'][^>]*>.*?<p[^>]*>(.*?)</p>', html)
    slide["quote"] = _first(r'class=["\']quote["\'][^>]*>(.*?)</div>', html)
    slide["actions"] = _bullets(html) if st == "closing" else []
    if st == "closing" and not slide["actions"]:
        slide["actions"] = [{"text": b.get("text", ""), "source": b.get("source", "")} for b in slide["bullets"]]
    slide["charts"] = _charts(html)
    return slide


def collect_html(path: Path) -> list[Path]:
    if path.is_file():
        return [path]
    files = sorted(path.glob("*.html"))
    if not files and (path / "slides").is_dir():
        files = sorted((path / "slides").glob("*.html"))
    return files


def build_manifest(src: Path, brand_kit: str = "fde-editorial") -> dict:
    files = collect_html(src)
    if len(files) < 2:
        raise ValueError("need ≥2 slide HTML files")
    slides = [
        parse_slide(f.read_text(encoding="utf-8"), i, len(files), f.name)
        for i, f in enumerate(files)
    ]
    return {"brand_kit": brand_kit, "slides": slides}


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: html_to_manifest.py slides_dir [out.json] [brand_kit]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    out = (
        Path(sys.argv[2]).resolve()
        if len(sys.argv) > 2 and Path(sys.argv[2]).suffix == ".json"
        else src / "slide_manifest.json"
    )
    brand = sys.argv[3] if len(sys.argv) > 3 else "fde-editorial"
    manifest = build_manifest(src, brand_kit=brand)
    out.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
