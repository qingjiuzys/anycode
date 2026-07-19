#!/usr/bin/env python3
"""Generate all shipped brand assets from brand/anycode-mark.svg."""
from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "brand/anycode-mark.svg"

SVG_TARGETS = (
    ROOT / "crates/account-portal/public/favicon.svg",
    ROOT / "crates/dashboard-ui/public/anycode-mark.svg",
)

PNG_TARGETS = {
    ROOT / "crates/account-portal/public/favicon.png": 64,
    ROOT / "crates/dashboard-ui/public/favicon.png": 64,
    ROOT / "crates/dashboard-ui/public/anycode-logo.png": 256,
    ROOT / "apps/anycode-desktop/assets/anycode-logo.png": 1024,
    ROOT / "apps/anycode-desktop/resources/dashboard-ui/anycode-logo.png": 256,
    ROOT / "apps/anycode-desktop/icons/32x32.png": 32,
    ROOT / "apps/anycode-desktop/icons/128x128.png": 128,
    ROOT / "apps/anycode-desktop/icons/128x128@2x.png": 256,
}


def rasterize() -> Image.Image:
    """Render with macOS Quick Look, then normalize through Pillow."""
    with tempfile.TemporaryDirectory(prefix="anycode-brand-") as tmp:
        subprocess.run(
            ["qlmanage", "-t", "-s", "1024", "-o", tmp, str(SOURCE)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        rendered = Path(tmp) / f"{SOURCE.name}.png"
        if not rendered.is_file():
            raise RuntimeError(f"Quick Look did not create {rendered}")
        return Image.open(rendered).convert("RGBA")


def resized(source: Image.Image, size: int) -> Image.Image:
    return source.resize((size, size), Image.Resampling.LANCZOS)


def main() -> None:
    if not SOURCE.is_file():
        raise SystemExit(f"missing canonical mark: {SOURCE}")

    image = rasterize()
    for target in SVG_TARGETS:
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(SOURCE, target)

    for target, size in PNG_TARGETS.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        resized(image, size).save(target, "PNG", optimize=True)

    icons = ROOT / "apps/anycode-desktop/icons"
    resized(image, 1024).save(icons / "icon.icns", "ICNS")
    resized(image, 256).save(
        icons / "icon.ico",
        "ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print(f"Generated {len(SVG_TARGETS) + len(PNG_TARGETS) + 2} assets from {SOURCE.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
