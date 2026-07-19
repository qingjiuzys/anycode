#!/usr/bin/env python3
"""Crop logo padding and emit full-bleed opaque PNG app icons (no white margin).

macOS applies its own squircle mask. Pre-rounding + transparent margins leave a
visible light ring in the Dock (smaller purple square inside the app tile).
Fill the canvas edge-to-edge with brand purple instead.
"""
from __future__ import annotations

import sys
from collections import deque
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

# Brand gradient stops from brand/anycode-mark.svg
BRAND_TOP = (120, 104, 255)  # #7868FF
BRAND_BOTTOM = (81, 68, 232)  # #5144E8
# Slight overfill so anti-aliased logo edges never leave a fringe.
COVER_OVERFILL = 1.04


def is_exterior_white(r: int, g: int, b: int, a: int) -> bool:
    """True for transparent / near-white canvas outside the mark (not the A glyph)."""
    if a < 16:
        return True
    if r > 235 and g > 235 and b > 235:
        return True
    spread = max(r, g, b) - min(r, g, b)
    return spread < 28 and min(r, g, b) > 190


def is_mark_pixel(r: int, g: int, b: int, a: int) -> bool:
    """Any non-exterior pixel (purple fill, white A, lavender accent)."""
    return not is_exterior_white(r, g, b, a)


def content_bbox(im: Image.Image) -> tuple[int, int, int, int]:
    px = im.convert("RGBA")
    w, h = px.size
    min_x, min_y = w, h
    max_x, max_y = 0, 0
    data = px.load()
    for y in range(h):
        for x in range(w):
            r, g, b, a = data[x, y]
            if not is_mark_pixel(r, g, b, a):
                continue
            min_x = min(min_x, x)
            min_y = min(min_y, y)
            max_x = max(max_x, x)
            max_y = max(max_y, y)
    if max_x <= min_x or max_y <= min_y:
        return 0, 0, w, h
    return min_x, min_y, max_x + 1, max_y + 1


def clear_exterior_white(im: Image.Image) -> Image.Image:
    """Flood-fill from corners: only connected exterior white → transparent.

    Preserves the solid white 'A' (interior white not connected to corners).
    """
    im = im.convert("RGBA")
    w, h = im.size
    pixels = im.load()
    visited = [[False] * w for _ in range(h)]
    q: deque[tuple[int, int]] = deque()

    def try_seed(x: int, y: int) -> None:
        if not (0 <= x < w and 0 <= y < h):
            return
        r, g, b, a = pixels[x, y]
        if is_exterior_white(r, g, b, a) and not visited[y][x]:
            visited[y][x] = True
            q.append((x, y))

    for x, y in ((0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)):
        try_seed(x, y)

    while q:
        x, y = q.popleft()
        pixels[x, y] = (0, 0, 0, 0)
        for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
            if 0 <= nx < w and 0 <= ny < h and not visited[ny][nx]:
                r, g, b, a = pixels[nx, ny]
                if is_exterior_white(r, g, b, a):
                    visited[ny][nx] = True
                    q.append((nx, ny))
    return im


def brand_gradient(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size))
    px = img.load()
    for y in range(size):
        t = y / max(1, size - 1)
        r = int(BRAND_TOP[0] + (BRAND_BOTTOM[0] - BRAND_TOP[0]) * t)
        g = int(BRAND_TOP[1] + (BRAND_BOTTOM[1] - BRAND_TOP[1]) * t)
        b = int(BRAND_TOP[2] + (BRAND_BOTTOM[2] - BRAND_TOP[2]) * t)
        row = (r, g, b, 255)
        for x in range(size):
            px[x, y] = row
    return img


def flatten_on_brand(im: Image.Image, size: int) -> Image.Image:
    base = brand_gradient(size)
    composed = Image.alpha_composite(base, im.convert("RGBA"))
    rgb = composed.convert("RGB")
    out = Image.new("RGBA", (size, size))
    out.paste(rgb, (0, 0))
    return out


def build_app_icon(size: int) -> Image.Image:
    """Full-bleed opaque icon for .icns / Dock."""
    im = Image.open(SRC).convert("RGBA")
    box = content_bbox(im)
    cropped = clear_exterior_white(im.crop(box))
    cw, ch = cropped.size
    scale = max(size / cw, size / ch) * COVER_OVERFILL
    nw, nh = max(1, int(cw * scale)), max(1, int(ch * scale))
    resized = cropped.resize((nw, nh), Image.Resampling.LANCZOS)

    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ox = (size - nw) // 2
    oy = (size - nh) // 2
    layer.paste(resized, (ox, oy), resized)
    return flatten_on_brand(layer, size)


def rounded_brand_icon(size: int) -> Image.Image:
    """Small UI mark: full brand fill with soft corners (no white corners)."""
    base = build_app_icon(size)
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    radius = max(8, int(size * 0.22))
    draw.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(base, (0, 0), mask)
    return out


def main() -> None:
    if not SRC.is_file():
        print(f"missing source: {SRC}", file=sys.stderr)
        sys.exit(1)

    app_icon = build_app_icon(1024)
    brand_icon = rounded_brand_icon(128)

    app_icon.save(OUT, format="PNG", optimize=True)
    OUT_UI.parent.mkdir(parents=True, exist_ok=True)
    app_icon.save(OUT_UI, format="PNG", optimize=True)
    brand_icon.save(OUT_BRAND, format="PNG", optimize=True)

    tl = app_icon.getpixel((0, 0))
    if tl[3] < 255 or (tl[0] > 235 and tl[1] > 235 and tl[2] > 235):
        print(f"bad corner pixel: {tl}", file=sys.stderr)
        sys.exit(1)
    bright = sum(
        1
        for y in range(200, 800, 4)
        for x in range(200, 800, 4)
        if app_icon.getpixel((x, y))[0] > 230
        and app_icon.getpixel((x, y))[1] > 230
    )
    if bright < 500:
        print(f"missing white A glyph (bright samples={bright})", file=sys.stderr)
        sys.exit(1)

    print(
        f"wrote full-bleed opaque icons:\n"
        f"  {OUT}\n"
        f"  {OUT_UI}\n"
        f"  {OUT_BRAND}"
    )


if __name__ == "__main__":
    main()
