#!/usr/bin/env python3
"""Screenshot a fixed 1920×1080 slide HTML file to PNG (Playwright Chromium)."""
from __future__ import annotations

import sys
from pathlib import Path

WIDTH = 1920
HEIGHT = 1080


def screenshot_html(html_path: Path, png_path: Path, *, width: int = WIDTH, height: int = HEIGHT) -> None:
    from playwright.sync_api import sync_playwright

    html_path = html_path.resolve()
    png_path.parent.mkdir(parents=True, exist_ok=True)
    url = html_path.as_uri()
    with sync_playwright() as pw:
        browser = pw.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": width, "height": height, "deviceScaleFactor": 1})
        page.goto(url, wait_until="networkidle")
        page.wait_for_timeout(150)
        page.screenshot(path=str(png_path), clip={"x": 0, "y": 0, "width": width, "height": height})
        browser.close()


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: screenshot_slide_html.py slide.html out.png", file=sys.stderr)
        return 1
    screenshot_html(Path(sys.argv[1]), Path(sys.argv[2]))
    print(Path(sys.argv[2]).resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
