#!/usr/bin/env python3
"""HTML slides + manifest + evidence PNGs (no PPTX export)."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: run slides_dir [brand_kit]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    brand = sys.argv[2] if len(sys.argv) > 2 else "fde-editorial"

    manifest_py = REPO / "scripts" / "office" / "html_to_manifest.py"
    proc = subprocess.run(
        [sys.executable, str(manifest_py), str(src), str(src / "slide_manifest.json")],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr or proc.stdout)
        return proc.returncode
    print(proc.stdout.strip())

    validator = REPO / "scripts" / "office" / "validate_slide_html.py"
    if validator.is_file():
        subprocess.run([sys.executable, str(validator), str(src), brand], check=False)

    screenshot = REPO / "scripts" / "office" / "screenshot_slide_html.py"
    ev = src / "evidence"
    ev.mkdir(parents=True, exist_ok=True)
    slides = sorted(src.glob("*.html"))
    if not slides and (src / "slides").is_dir():
        slides = sorted((src / "slides").glob("*.html"))
    for i, hf in enumerate(slides, start=1):
        png = ev / f"slide-{i:02d}.png"
        subprocess.run([sys.executable, str(screenshot), str(hf), str(png)], check=False)
        print(png)
    print(str(src / "slide_manifest.json"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
