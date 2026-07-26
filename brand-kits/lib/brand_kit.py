"""Shared brand-kit loader for Office delivery skills."""
from __future__ import annotations

import json
import os
import re
from pathlib import Path


def _hex_rgb(hex_color: str) -> tuple[int, int, int]:
    h = hex_color.lstrip("#")
    if len(h) == 8:
        h = h[2:]
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def brand_kits_root() -> Path:
    env = os.environ.get("ANYCODE_BRAND_KITS_DIR")
    if env:
        return Path(env)
    for base in [Path.cwd(), *Path.cwd().parents]:
        p = base / "brand-kits"
        if p.is_dir() and any(p.glob("*/tokens.json")):
            return p
    bundled = Path(__file__).resolve().parent.parent
    return bundled


def list_brand_kits() -> list[str]:
    root = brand_kits_root()
    out: list[str] = []
    for d in sorted(root.iterdir()):
        if d.is_dir() and not d.name.startswith("_") and (d / "tokens.json").is_file():
            out.append(d.name)
    return out


def find_brand_kit(name: str = "fde-editorial") -> Path:
    p = brand_kits_root() / name
    if (p / "tokens.json").is_file():
        return p
    skill_brand = Path(__file__).resolve().parent.parent / "lingqi"
    if name == "lingqi" and (skill_brand / "tokens.json").is_file():
        return skill_brand
    bundled = Path(__file__).resolve().parent / ".." / "lingqi"
    bundled = bundled.resolve()
    if name == "lingqi" and (bundled / "tokens.json").is_file():
        return bundled
    skill_install = Path.cwd() / "brand"
    if (skill_install / "tokens.json").is_file():
        return skill_install
    raise FileNotFoundError(f"brand kit `{name}` not found under {brand_kits_root()}")


def infer_brand_kit(text: str) -> str:
    t = text.lower()
    gov_hits = ("政府", "政务", "公文", "红头", "gov ", "government", "密级", "机关")
    edu_hits = ("教育", "教学", "课纲", "教案", "lesson plan", "school", "course")
    if any(h in t for h in gov_hits):
        return "gov-formal"
    if any(h in t for h in edu_hits):
        return "edu-clean"
    return "fde-editorial"


def load_tokens(kit_dir: Path | None = None, name: str = "fde-editorial") -> dict:
    kit = kit_dir or find_brand_kit(name)
    return json.loads((kit / "tokens.json").read_text(encoding="utf-8"))


def load_xlsx_theme(kit_dir: Path | None = None, name: str = "lingqi") -> dict:
    kit = kit_dir or find_brand_kit(name)
    return json.loads((kit / "xlsx" / "theme.json").read_text(encoding="utf-8"))


def load_pptx_layouts(kit_dir: Path | None = None, name: str = "lingqi") -> dict:
    kit = kit_dir or find_brand_kit(name)
    return json.loads((kit / "pptx" / "layouts.json").read_text(encoding="utf-8"))


def footer_text(tokens: dict, layouts: dict | None = None) -> str:
    if layouts and layouts.get("footer_text"):
        return str(layouts["footer_text"])
    meta = tokens.get("footer") or {}
    if meta.get("default_text"):
        return str(meta["default_text"])
    return f"{tokens.get('name', 'Document')} · Confidential"


def color_from_tokens(tokens: dict, key: str) -> tuple[int, int, int]:
    colors = tokens.get("colors") or {}
    val = colors.get(key)
    if val is None:
        # Allow raw hex as the key (e.g. "FFFFFF"), else fall back to dark gray.
        val = key if key.lstrip("#").strip() and all(c in "0123456789abcdefABCDEF" for c in key.lstrip("#")) else "#404040"
    return _hex_rgb(val)


def emit_artifact(path: Path, kind: str) -> None:
    print(path)
    title = path.name.replace('"', "")
    print(
        'ANYCODE_ARTIFACT:{"path":"%s","kind":"%s","title":"%s","inline":true}'
        % (path, kind, title)
    )
    sidecar = Path(str(path) + ".anycode-artifact.json")
    sidecar.write_text(
        '{"path":"%s","kind":"%s","title":"%s","inline":true}\n' % (path, kind, title),
        encoding="utf-8",
    )


def infer_scenario(text: str) -> str | None:
    t = text.lower()
    rules: list[tuple[str, tuple[str, ...]]] = [
        ("performance-review", ("述职", "performance review", "okr review", "年度考核")),
        ("education-lesson-plan", ("课纲", "教案", "lesson plan", "教学设计", "教学课件")),
        ("gov-briefing", ("政府", "政务", "gov briefing", "公文", "汇报材料")),
        ("finance-quarterly-review", ("财务", "预算", "quarterly review", "损益", "p&l", "cashflow")),
        ("med-aesthetic-proposal", ("医美", "med aesthetic", "美容", "诊所方案")),
        ("product-launch", ("产品发布", "product launch", "launch deck")),
        ("work-report", ("工作汇报", "weekly report", "周报", "月报", "ops report")),
    ]
    for sid, kws in rules:
        if any(kw in t for kw in kws):
            return sid
    return None


def scenarios_root() -> Path:
    for base in [Path.cwd(), *Path.cwd().parents]:
        p = base / "scenarios"
        if p.is_dir():
            return p
    return Path(__file__).resolve().parents[2] / "scenarios"


def load_scenario(scenario_id: str) -> dict:
    path = scenarios_root() / scenario_id / "manifest.json"
    if not path.is_file():
        raise FileNotFoundError(f"scenario `{scenario_id}` not found at {path}")
    return json.loads(path.read_text(encoding="utf-8"))
