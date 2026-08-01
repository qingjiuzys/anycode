#!/usr/bin/env python3
"""Real office (DOCX/PPTX/XLSX) 4-arm eval via Dashboard (env on server process).

Mirrors run-agent-quality-visual.py: restarts anycode-dashboard-serve per arm with
ANYCODE_EVAL_* so AgentRuntime switches Experience/Skill. Saves office artifacts and
scores commercial visual features vs legacy baseline.

Example:
  python3 scripts/run-agent-quality-office.py --arms baseline,experience_skill
  python3 scripts/run-agent-quality-office.py --scenes docx,pptx,xlsx,pptx_commercial
"""
from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = (
    REPO / "test" / "benchmarks" / "agent-quality" / "results" / "office-20260722-223000"
)
sys.path.insert(0, str(REPO / "scripts" / "lib"))
sys.path.insert(0, str(REPO / "test"))
from dashboard_server import start_dashboard, stop_dashboard, wait_health  # noqa: E402
from runner.dashboard_client import DashboardClient  # noqa: E402

ARMS = {
    "baseline": ("0", "0"),
    "experience_only": ("1", "0"),
    "skill_only": ("0", "1"),
    "experience_skill": ("1", "1"),
}

SCENES: dict[str, dict] = {
    "docx": {
        "exts": [".docx"],
        "prompt": (
            "Audience: engineering leads. Do not ask clarifying questions — assume and proceed. "
            "Write a weekly ops report for anyCode platform (week of 2026-07-14). "
            "Include Markdown source then export a real report.docx under the project workspace. "
            "Structure: # Summary, then ## Metrics, ## Incidents, ## Changes, ## Next steps. "
            "End every section with Decision: or Action: including a named owner and ISO date. "
            "Use concrete numbers (no TBD/lorem). Prefer the anycode-docx Skill run script."
        ),
    },
    "pptx": {
        "exts": [".pptx"],
        "prompt": (
            "Audience: executives. Do not ask clarifying questions — assume and proceed. "
            "Create a 6-slide product pitch PPTX for anyCode under the project workspace: "
            "Title, Problem, Metric, Plan, Risks, Ask. Every non-title bullet must include a "
            "concrete number OR named owner/date. No TBD / Competitor X. Use "
            "anycode-ppt Skill for slides/*.html + slide_manifest.json + evidence PNGs, "
            "then presentation-commercial-delivery to export editable pitch.pptx (not raster)."
        ),
    },
    "pptx_commercial": {
        "exts": [".pptx"],
        "prompt": (
            "Audience: enterprise buyers. Do not ask clarifying questions — assume and proceed. "
            "Deliver a commercial-grade 8-slide anyCode pitch deck under the project workspace. "
            "MUST use anycode-ppt + presentation-commercial-delivery Skills with lingqi brand-kit and COPY template structure: "
            "slides/01-cover.html (3 value chips), slides/02-section.html (agenda), "
            "slides/03-problem.html (two-column: 5 bullets + side panel + quote), "
            "slides/04-metrics.html (6 stat cards), slides/05-plan.html (5 timeline items + owners), "
            "slides/06-architecture.html (two-column), slides/07-risks.html (5 risks + mitigations panel), "
            "slides/08-closing.html (4 action items + contact). "
            "Each content slide must fill 1920×1080 — no sparse 3-bullet slides. "
            "Export pitch-commercial.pptx (editable native OOXML) and evidence/slide-*.png via commercial Skill run."
        ),
    },
    "xlsx": {
        "exts": [".xlsx"],
        "prompt": (
            "Audience: finance ops. Do not ask clarifying questions — assume and proceed. "
            "Create a June 2026 sales workbook as a real .xlsx under the project workspace. "
            "Columns: Region, Product, Units, Revenue. Include a header row plus at least 6 "
            "concrete data rows (APAC/EMEA/AMER). No TBD/lorem. Prefer anycode-xlsx "
            "Skill with lingqi brand-kit: ≥3 sheets (Summary + Detail + Pricing), branded header fill, "
            "frozen top row. Save sales-june.xlsx."
        ),
    },
}


def pptx_shape_stats(path: Path) -> dict:
    slides = 0
    shapes = 0
    pics = 0
    text_len = 0
    raster_slides = 0
    with zipfile.ZipFile(path) as zf:
        for name in zf.namelist():
            if name.startswith("ppt/slides/slide") and name.endswith(".xml"):
                slides += 1
                xml = zf.read(name).decode("utf-8", errors="replace")
                sp = xml.count("<p:sp")
                pic = xml.count("<p:pic")
                shapes += sp
                pics += pic
                slide_text = 0
                for part in xml.split("<a:t")[1:]:
                    end = part.find("</a:t>")
                    if end >= 0:
                        slide_text += len(part[:end].lstrip(">").strip())
                text_len += slide_text
                if pic >= 1 and sp <= 2 and slide_text < 20:
                    raster_slides += 1
    avg = (shapes / slides) if slides else 0.0
    editable = text_len >= 120 and not (
        pics >= slides and slides >= 2 and raster_slides >= max(slides - 1, 1)
    )
    return {
        "slides": slides,
        "shapes": shapes,
        "pics": pics,
        "text_chars": text_len,
        "raster_slides": raster_slides,
        "shapes_per_slide": round(avg, 2),
        "pptx_editable": editable,
    }


def docx_commercial_features(path: Path) -> dict:
    with zipfile.ZipFile(path) as zf:
        names = set(zf.namelist())
        has_header = any(n.startswith("word/header") for n in names)
        has_footer = any(n.startswith("word/footer") for n in names)
        styles = ""
        if "word/styles.xml" in names:
            styles = zf.read("word/styles.xml").decode("utf-8", errors="replace").lower()
    return {
        "has_header": has_header,
        "has_footer": has_footer,
        "has_heading_styles": "heading" in styles or "title" in styles,
    }


def xlsx_style_features(path: Path) -> dict:
    with zipfile.ZipFile(path) as zf:
        sheets = sum(
            1
            for n in zf.namelist()
            if n.startswith("xl/worksheets/sheet") and n.endswith(".xml")
        )
        styles = ""
        if "xl/styles.xml" in zf.namelist():
            styles = zf.read("xl/styles.xml").decode("utf-8", errors="replace").lower()
    return {
        "sheet_count": sheets,
        "has_brand_fill": "patternfill" in styles or "fgcolor" in styles,
    }


def score_artifact(path: Path, scene: str) -> dict:
    ext = path.suffix.lower()
    score: dict = {"path": str(path), "bytes": path.stat().st_size}
    if ext == ".pptx":
        score.update(pptx_shape_stats(path))
        ws = path.parent
        evidence = ws / "evidence"
        thumbs = sorted(evidence.glob("slide-*.png")) if evidence.is_dir() else []
        score["render_thumbs"] = len(thumbs)
        score["commercial_score"] = min(
            100,
            int(score.get("shapes_per_slide", 0) * 12)
            + (20 if score.get("render_thumbs", 0) >= 2 else 0)
            + (10 if score["bytes"] > 50_000 else 0),
        )
        score["editable_commercial_score"] = (
            score["commercial_score"]
            + (40 if score.get("pptx_editable") else -100)
            + (10 if score.get("text_chars", 0) >= 200 else 0)
        )
    elif ext == ".docx":
        feats = docx_commercial_features(path)
        score.update(feats)
        score["commercial_score"] = (
            (30 if feats["has_header"] else 0)
            + (30 if feats["has_footer"] else 0)
            + (40 if feats["has_heading_styles"] else 0)
        )
    elif ext == ".xlsx":
        feats = xlsx_style_features(path)
        score.update(feats)
        score["commercial_score"] = (
            (50 if feats["sheet_count"] >= 3 else feats["sheet_count"] * 15)
            + (50 if feats["has_brand_fill"] else 0)
        )
    else:
        score["commercial_score"] = 0
    score["scene"] = scene
    return score


def load_baseline_scores(baseline_dir: Path, scene: str) -> dict | None:
    if not baseline_dir.is_dir():
        return None
    arm_dir = baseline_dir / "artifacts" / scene / "baseline"
    if not arm_dir.is_dir():
        return None
    for ext in SCENES.get(scene, {}).get("exts", []):
        for p in arm_dir.glob(f"*{ext}"):
            if p.is_file():
                return score_artifact(p, scene)
    return None


def promotion_verdict(row: dict, baseline: dict | None) -> dict:
    cur = row.get("quality") or {}
    if not baseline:
        return {"status": "no_baseline", "delta": None}
    delta = cur.get("editable_commercial_score", cur.get("commercial_score", 0)) - baseline.get(
        "editable_commercial_score", baseline.get("commercial_score", 0)
    )
    promoted = delta >= 15 and row.get("ok")
    if row.get("scene", "").startswith("pptx"):
        promoted = promoted and cur.get("shapes_per_slide", 0) >= 5
        promoted = promoted and cur.get("render_thumbs", 0) >= 1
        promoted = promoted and cur.get("pptx_editable") is True
    if row.get("scene") == "docx":
        promoted = promoted and cur.get("has_header") and cur.get("has_footer")
    if row.get("scene") == "xlsx":
        promoted = promoted and cur.get("sheet_count", 0) >= 3
    return {"status": "promoted" if promoted else "hold", "delta": delta}






def find_office(workspace: Path, exts: list[str]) -> list[Path]:
    out: list[Path] = []
    for ext in exts:
        for p in workspace.rglob(f"*{ext}"):
            if p.is_file() and "node_modules" not in p.parts:
                out.append(p)
    return sorted(out, key=lambda p: p.stat().st_mtime, reverse=True)


def run_case(
    *,
    scene: str,
    arm: str,
    exp: str,
    skills: str,
    port: int,
    out_dir: Path,
    timeout: float,
) -> dict:
    spec = SCENES[scene]
    db = Path(os.environ.get("TMPDIR", "/tmp")) / f"anycode-aq-office-{port}-{scene}-{arm}.db"
    proc = start_dashboard(port, f"{scene}-{arm}", exp, skills, db)
    client = DashboardClient(f"http://127.0.0.1:{port}")
    row: dict = {"scene": scene, "arm": arm, "experience": exp, "skills": skills}
    try:
        wait_health(client, timeout=120)
        workspace = out_dir / "workspaces" / scene / arm
        workspace.mkdir(parents=True, exist_ok=True)
        project_id = client.create_project(str(workspace.resolve()), f"aq-office-{scene}-{arm}")
        session_id = client.create_session(project_id, f"{scene}-{arm}")
        row["session_id"] = session_id
        status, payload = client.send_message(session_id, spec["prompt"], timeout=timeout)
        row["message_status"] = status
        if status >= 400:
            row["error"] = payload
            row["ok"] = False
            return row
        final = client.wait_session_done(session_id, timeout=timeout)
        row["session_status"] = final
        usage = client.get_usage(session_id)
        trace = client.get_trace(session_id)
        row["usage"] = usage
        files = find_office(workspace, spec["exts"])
        artifacts_dir = out_dir / "artifacts" / scene / arm
        artifacts_dir.mkdir(parents=True, exist_ok=True)
        copied = []
        for i, src in enumerate(files[:3]):
            dest = artifacts_dir / (src.name if i == 0 else f"{i}-{src.name}")
            dest.write_bytes(src.read_bytes())
            copied.append(str(dest.relative_to(out_dir)))
        if copied:
            row["primary"] = copied[0]
        row["artifacts"] = copied
        primary_path = artifacts_dir / Path(copied[0]).name if copied else None
        if primary_path and primary_path.is_file():
            ev_src = workspace / "evidence"
            if ev_src.is_dir():
                ev_dest = artifacts_dir / "evidence"
                ev_dest.mkdir(parents=True, exist_ok=True)
                for png in ev_src.glob("*.png"):
                    dest = ev_dest / png.name
                    dest.write_bytes(png.read_bytes())
                    row.setdefault("render_evidence", []).append(
                        str(dest.relative_to(out_dir))
                    )
            row["quality"] = score_artifact(primary_path, scene)
        row["ok"] = final == "completed" and bool(copied)
        # Gate / skill markers from trace
        trace_text = json.dumps(trace, ensure_ascii=False)
        row["trace_markers"] = {
            k: (k in trace_text)
            for k in (
                "gate_plan_created",
                "skill_resolved",
                "repair_requested",
                "verification_finished",
            )
        }
        (artifacts_dir / "trace.json").write_text(
            json.dumps(trace, ensure_ascii=False, indent=2)[:200_000] + "\n",
            encoding="utf-8",
        )
        return row
    finally:
        stop_dashboard(proc)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=43201)
    ap.add_argument("--timeout", type=float, default=900)
    ap.add_argument("--arms", default=",".join(ARMS))
    ap.add_argument("--scenes", default="docx,pptx,xlsx")
    ap.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()
    stamp = time.strftime("%Y%m%d-%H%M%S")
    out_dir = args.out or (
        REPO / "test" / "benchmarks" / "agent-quality" / "results" / f"office-{stamp}"
    )
    out_dir.mkdir(parents=True, exist_ok=True)

    selected_arms = [a.strip() for a in args.arms.split(",") if a.strip()]
    selected_scenes = [s.strip() for s in args.scenes.split(",") if s.strip()]
    for s in selected_scenes:
        if s not in SCENES:
            raise SystemExit(f"unknown scene: {s}")
    for a in selected_arms:
        if a not in ARMS:
            raise SystemExit(f"unknown arm: {a}")

    rows = []
    for scene in selected_scenes:
        for arm in selected_arms:
            exp, skills = ARMS[arm]
            print(f"\n== scene={scene} arm={arm} experience={exp} skills={skills} ==", flush=True)
            t0 = time.time()
            try:
                row = run_case(
                    scene=scene,
                    arm=arm,
                    exp=exp,
                    skills=skills,
                    port=args.port,
                    out_dir=out_dir,
                    timeout=args.timeout,
                )
            except Exception as exc:  # noqa: BLE001
                row = {"scene": scene, "arm": arm, "ok": False, "error": str(exc)}
                print(f"FAILED: {exc}", flush=True)
            row["duration_s"] = round(time.time() - t0, 1)
            baseline = load_baseline_scores(args.baseline, scene)
            row["baseline_quality"] = baseline
            row["promotion"] = promotion_verdict(row, baseline)
            rows.append(row)
            print(
                f"   ok={row.get('ok')} status={row.get('session_status')} "
                f"file={row.get('primary')} q={row.get('quality', {}).get('commercial_score')} "
                f"promo={row.get('promotion', {}).get('status')} ({row['duration_s']}s) "
                f"markers={row.get('trace_markers')}",
                flush=True,
            )

    summary = {
        "run_id": out_dir.name,
        "baseline_run": str(args.baseline) if args.baseline.is_dir() else None,
        "promotion_rules": {
            "min_commercial_delta": 15,
            "pptx_min_shapes_per_slide": 5,
            "pptx_min_render_thumbs": 1,
            "docx_requires_header_footer": True,
            "xlsx_min_sheets": 3,
        },
        "scenes": {k: SCENES[k]["prompt"] for k in selected_scenes},
        "model_note": "uses ~/.anycode/config.json active chat model",
        "rows": rows,
    }
    (out_dir / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    quality_rows = [
        {
            "scene": r.get("scene"),
            "arm": r.get("arm"),
            "ok": r.get("ok"),
            "quality": r.get("quality"),
            "baseline_quality": r.get("baseline_quality"),
            "promotion": r.get("promotion"),
        }
        for r in rows
    ]
    (out_dir / "quality-score.json").write_text(
        json.dumps(quality_rows, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    lines = ["<!DOCTYPE html><meta charset=utf-8><title>AQ office arms</title>",
             "<h1>Agent quality — office arms</h1><ul>"]
    for r in rows:
        primary = r.get("primary")
        label = f"{r.get('scene')}/{r.get('arm')}"
        if primary:
            lines.append(
                f'<li><a href="{primary}">{label}</a> '
                f'ok={r.get("ok")} {r.get("duration_s")}s</li>'
            )
        else:
            lines.append(f"<li>{label} ok={r.get('ok')} err={r.get('error')}</li>")
    lines.append("</ul><p>Open artifacts locally (docx/pptx/xlsx).</p>\n")
    (out_dir / "index.html").write_text("\n".join(lines), encoding="utf-8")
    print(f"\nWrote {out_dir}")
    print(f"Open: {out_dir / 'index.html'}")
    return 0 if all(r.get("ok") for r in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
