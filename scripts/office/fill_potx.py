#!/usr/bin/env python3
"""Fill lingqi brand template → editable native .pptx from slide_manifest.json or HTML dir."""
from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "brand-kits" / "lib"))
sys.path.insert(0, str(REPO / "scripts" / "office"))

from brand_kit import emit_artifact, find_brand_kit, load_tokens  # noqa: E402
from html_to_manifest import build_manifest  # noqa: E402
from pptx_slide_builder import build_deck  # noqa: E402


def ensure_evidence_pngs(workspace: Path) -> None:
    """Regenerate stale/missing HTML evidence before pptx export."""
    screenshot = REPO / "scripts" / "office" / "screenshot_slide_html.py"
    if not screenshot.is_file():
        return
    slides_dir = workspace / "slides" if (workspace / "slides").is_dir() else workspace
    slides = sorted(slides_dir.glob("*.html"))
    if not slides:
        return
    ev = workspace / "evidence"
    ev.mkdir(parents=True, exist_ok=True)
    import subprocess

    for i, hf in enumerate(slides, start=1):
        png = ev / f"slide-{i:02d}.png"
        if not png.is_file() or png.stat().st_mtime < hf.stat().st_mtime:
            subprocess.run([sys.executable, str(screenshot), str(hf), str(png)], check=False)


def attach_visual_assets(manifest: dict, workspace: Path) -> None:
    """Wire evidence PNGs (HTML screenshots) into manifest so pptx matches designed visuals."""

    def evidence_png(index: int) -> tuple[str, Path] | None:
        for base in (workspace, workspace / "slides"):
            png = base / "evidence" / f"slide-{index:02d}.png"
            if png.is_file():
                rel = png.relative_to(workspace)
                return str(rel), png
        return None

    slides_dir = workspace / "slides" if (workspace / "slides").is_dir() else workspace
    for i, slide in enumerate(manifest.get("slides") or []):
        hit = evidence_png(i + 1)
        if hit:
            slide["visual_png"] = hit[0]
        src = slide.get("source_html")
        if src:
            html_path = slides_dir / src
            if not html_path.is_file():
                html_path = workspace / src
            if html_path.is_file():
                html = html_path.read_text(encoding="utf-8")
                from html_to_manifest import _has_dense_visual, _images  # noqa: WPS433

                slide["has_visual"] = slide.get("has_visual") or _has_dense_visual(html)
                if not slide.get("images"):
                    slide["images"] = _images(html)
        if slide.get("visual_png"):
            slide["layout"] = "visual_full"


def load_manifest(src: Path, brand: str = "fde-editorial") -> dict:
    if src.is_file() and src.suffix == ".json":
        return json.loads(src.read_text(encoding="utf-8"))
    manifest_path = src / "slide_manifest.json"
    if manifest_path.is_file():
        return json.loads(manifest_path.read_text(encoding="utf-8"))
    return build_manifest(src, brand_kit=brand)


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: fill_potx.py slides_dir|slide_manifest.json [output.pptx] [brand_kit]", file=sys.stderr)
        return 1
    src = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else Path.cwd() / "pitch.pptx"
    brand = sys.argv[3] if len(sys.argv) > 3 else "fde-editorial"

    kit = find_brand_kit(brand)
    tokens = load_tokens(kit)
    potx = kit / "pptx" / "template.potx"
    if not potx.is_file():
        import subprocess

        subprocess.run([sys.executable, str(REPO / "scripts" / "office" / "build_brand_potx.py"), brand], check=True)

    manifest = load_manifest(src, brand)
    if manifest.get("brand_kit") in (None, "lingqi") and brand != "lingqi":
        manifest["brand_kit"] = brand
    ws = src if src.is_dir() else src.parent
    ensure_evidence_pngs(ws)
    attach_visual_assets(manifest, ws)
    charts_dir = ws / "charts"
    renderer = REPO / "scripts" / "office" / "render_chart_png.py"
    for i, slide in enumerate(manifest.get("slides") or []):
        for j, chart in enumerate(slide.get("charts") or []):
            # Native-editable charts (categories + series present) are built by
            # pptx_slide_builder._add_chart; only rasterize charts that lack data.
            if chart.get("categories") and chart.get("series"):
                continue
            png_rel = chart.get("png") or f"charts/slide-{i+1:02d}-chart-{j+1}.png"
            png_path = ws / png_rel
            if not png_path.is_file() and renderer.is_file():
                charts_dir.mkdir(parents=True, exist_ok=True)
                spec_tmp = charts_dir / f"slide-{i+1:02d}-chart-{j+1}.json"
                spec_tmp.write_text(json.dumps(chart, ensure_ascii=False, indent=2), encoding="utf-8")
                import subprocess

                subprocess.run([sys.executable, str(renderer), str(spec_tmp), str(png_path)], check=False)
            # Only point at the PNG when it actually exists — otherwise the
            # builder silently drops the chart entirely.
            if png_path.is_file():
                chart["png"] = str(Path(png_rel))
            else:
                print(f"WARN: chart render failed or unavailable: {png_rel}", file=sys.stderr)
    prs = build_deck(manifest, tokens, workspace=ws)
    out.parent.mkdir(parents=True, exist_ok=True)
    prs.save(str(out))
    emit_artifact(out, "presentation")

    manifest_out = out.parent / "slide_manifest.json"
    if not manifest_out.is_file():
        manifest_out.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    renderer = REPO / "scripts" / "office" / "render_pptx_evidence.py"
    screenshot = REPO / "scripts" / "office" / "screenshot_slide_html.py"
    ev = out.parent / "evidence"
    slides_dir = src if src.is_dir() else src.parent
    if slides_dir.is_dir() and any(slides_dir.glob("*.html")):
        import subprocess

        ev.mkdir(parents=True, exist_ok=True)
        for i, hf in enumerate(sorted(slides_dir.glob("*.html")), start=1):
            png = ev / f"slide-{i:02d}.png"
            subprocess.run([sys.executable, str(screenshot), str(hf), str(png)], check=False)
    elif renderer.is_file():
        import subprocess

        subprocess.run([sys.executable, str(renderer), str(out), str(ev)], check=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
