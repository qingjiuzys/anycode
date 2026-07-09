#!/usr/bin/env python3
"""Crop logo padding, scale artwork, and emit transparent rounded PNG app icons."""
from __future__ import annotations

import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("Pillow required: python3 -m pip install pillow", file=sys.stderr)
    sys.exit(1)

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "apps/anycode-desktop/assets/anycode-logo.png"
OUT = ROOT / "apps/anycode-desktop/assets/anycode-logo-app-icon.png"
OUT_UI = ROOT / "crates/dashboard-ui/src/assets/anycode-logo-app-icon.png"
OUT_BRAND = ROOT / "crates/dashboard-ui/src/assets/brand-icon.png"

# Fraction of canvas used by artwork (macOS dock reads better with ~8% margin).
FILL_RATIO = 0.92
# Corner radius as a fraction of output size (~macOS squircle feel).
CORNER_RATIO = 0.22


def is_background(r: int, g: int, b: int, a: int) -> bool:
    if a < 16:
        return True
    if r > 235 and g > 235 and b > 235:
        return True
    # Ignore faint grey watermarks / compression noise.
    spread = max(r, g, b) - min(r, g, b)
    return spread < 28 and min(r, g, b) > 190


def content_bbox(im: Image.Image) -> tuple[int, int, int, int]:
    px = im.convert("RGBA")
    w, h = px.size
    min_x, min_y = w, h
    max_x, max_y = 0, 0
    data = px.load()
    for y in range(h):
        for x in range(w):
            r, g, b, a = data[x, y]
            if is_background(r, g, b, a):
                continue
            min_x = min(min_x, x)
            min_y = min(min_y, y)
            max_x = max(max_x, x)
            max_y = max(max_y, y)
    if max_x <= min_x or max_y <= min_y:
        return 0, 0, w, h
    pad = max(8, int(min(w, h) * 0.01))
    return (
        max(0, min_x - pad),
        max(0, min_y - pad),
        min(w, max_x + pad + 1),
        min(h, max_y + pad + 1),
    )


def rounded_alpha_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def apply_rounded_corners(im: Image.Image, radius: int) -> Image.Image:
    im = im.convert("RGBA")
    w, h = im.size
    mask = rounded_alpha_mask(w, radius)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    out.paste(im, (0, 0), mask)
    return out


def build_icon(size: int) -> Image.Image:
    im = Image.open(SRC).convert("RGBA")
    box = content_bbox(im)
    cropped = im.crop(box)

    target = int(size * FILL_RATIO)
    cw, ch = cropped.size
    scale = min(target / cw, target / ch)
    nw, nh = max(1, int(cw * scale)), max(1, int(ch * scale))
    resized = cropped.resize((nw, nh), Image.Resampling.LANCZOS)

    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ox = (size - nw) // 2
    oy = (size - nh) // 2
    canvas.paste(resized, (ox, oy), resized)
    radius = max(8, int(size * CORNER_RATIO))
    return apply_rounded_corners(canvas, radius)


def main() -> None:
    if not SRC.is_file():
        print(f"missing source: {SRC}", file=sys.stderr)
        sys.exit(1)

    app_icon = build_icon(1024)
    brand_icon = build_icon(128)

    app_icon.save(OUT, format="PNG", optimize=True)
    OUT_UI.parent.mkdir(parents=True, exist_ok=True)
    app_icon.save(OUT_UI, format="PNG", optimize=True)
    brand_icon.save(OUT_BRAND, format="PNG", optimize=True)
    print(
        f"wrote transparent rounded icons:\n"
        f"  {OUT}\n"
        f"  {OUT_UI}\n"
        f"  {OUT_BRAND}"
    )


if __name__ == "__main__":
    main()
