#!/usr/bin/env python3
"""Render HTML to PDF via Playwright print (FDE editorial reports)."""
from __future__ import annotations

import sys
from pathlib import Path


def html_to_pdf(html_path: Path, pdf_path: Path) -> None:
    from playwright.sync_api import sync_playwright

    html_path = html_path.resolve()
    pdf_path.parent.mkdir(parents=True, exist_ok=True)
    url = html_path.as_uri()
    with sync_playwright() as pw:
        browser = pw.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": 920, "height": 1200})
        page.goto(url, wait_until="networkidle")
        page.wait_for_timeout(200)
        page.pdf(
            path=str(pdf_path),
            format="A4",
            print_background=True,
            margin={"top": "18mm", "bottom": "18mm", "left": "16mm", "right": "16mm"},
        )
        browser.close()


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: html_to_pdf.py input.html output.pdf", file=sys.stderr)
        return 1
    html_path = Path(sys.argv[1])
    pdf_path = Path(sys.argv[2])
    if not html_path.is_file():
        print(f"missing html: {html_path}", file=sys.stderr)
        return 1
    try:
        html_to_pdf(html_path, pdf_path)
    except Exception as exc:
        print(f"pdf render failed: {exc}", file=sys.stderr)
        return 1
    if not pdf_path.is_file() or pdf_path.stat().st_size < 512:
        print("pdf output empty or too small", file=sys.stderr)
        return 1
    print(pdf_path.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
