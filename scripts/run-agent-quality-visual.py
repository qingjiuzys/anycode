#!/usr/bin/env python3
"""Real 4-arm visual landing run via Dashboard (env on server process).

Starts anycode-dashboard-serve per arm with ANYCODE_EVAL_* so AgentRuntime
actually switches Experience/Skill. Saves HTML artifacts for side-by-side review.

Example:
  python3 scripts/run-agent-quality-visual.py --model deepseek-v4-flash
"""
from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
import uuid
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
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

LANDING_PROMPT = (
    "Build a self-contained HTML landing page for anyCode CLI in FDE Editorial style: "
    "light canvas #f2f5f0, ink #231f20, electric-blue accent #1400ff, one H1 in serif 900 "
    "(Songti SC), mono uppercase meta labels, 6px ink rule, hairline grid section, lede, "
    "primary CTA, secondary link, contrast comments, no gradients/shadows/rounded cards. "
    "Write index.html in the project workspace. Do not wrap the file in markdown fences."
)






def find_html(workspace: Path) -> list[Path]:
    out = []
    for p in workspace.rglob("*.html"):
        if p.is_file() and "node_modules" not in p.parts:
            out.append(p)
    return sorted(out, key=lambda p: p.stat().st_mtime, reverse=True)


def run_arm(
    *,
    arm: str,
    exp: str,
    skills: str,
    port: int,
    out_dir: Path,
    timeout: float,
) -> dict:
    db = Path(os.environ.get("TMPDIR", "/tmp")) / f"anycode-aq-visual-{port}-{arm}.db"
    proc = start_dashboard(port, arm, exp, skills, db)
    client = DashboardClient(f"http://127.0.0.1:{port}")
    row: dict = {"arm": arm, "experience": exp, "skills": skills}
    try:
        wait_health(client, timeout=120)
        workspace = out_dir / "workspaces" / arm
        workspace.mkdir(parents=True, exist_ok=True)
        project_id = client.create_project(str(workspace.resolve()), f"aq-visual-{arm}")
        session_id = client.create_session(project_id, f"landing-{arm}")
        row["session_id"] = session_id
        status, payload = client.send_message(session_id, LANDING_PROMPT, timeout=timeout)
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
        htmls = find_html(workspace)
        artifacts_dir = out_dir / "artifacts" / arm
        artifacts_dir.mkdir(parents=True, exist_ok=True)
        copied = []
        for i, src in enumerate(htmls[:3]):
            dest = artifacts_dir / (src.name if i == 0 else f"{i}-{src.name}")
            dest.write_bytes(src.read_bytes())
            copied.append(str(dest.relative_to(out_dir)))
        # Prefer canonical name for primary
        if copied:
            primary = artifacts_dir / "index.html"
            if not primary.exists():
                primary.write_bytes((out_dir / copied[0]).read_bytes())
            row["primary_html"] = str(primary.relative_to(out_dir))
        row["artifacts"] = copied
        row["ok"] = final == "completed" and bool(copied)
        # Keep a short trace excerpt
        (artifacts_dir / "trace.json").write_text(
            json.dumps(trace, ensure_ascii=False, indent=2)[:200_000] + "\n",
            encoding="utf-8",
        )
        return row
    finally:
        stop_dashboard(proc)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=43199)
    ap.add_argument("--timeout", type=float, default=900)
    ap.add_argument(
        "--arms",
        default=",".join(ARMS),
        help="comma-separated arms",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=None,
    )
    args = ap.parse_args()
    stamp = time.strftime("%Y%m%d-%H%M%S")
    out_dir = args.out or (
        REPO / "test" / "benchmarks" / "agent-quality" / "results" / f"visual-{stamp}"
    )
    out_dir.mkdir(parents=True, exist_ok=True)

    selected = [a.strip() for a in args.arms.split(",") if a.strip()]
    rows = []
    for arm in selected:
        if arm not in ARMS:
            raise SystemExit(f"unknown arm: {arm}")
        exp, skills = ARMS[arm]
        print(f"\n== arm={arm} experience={exp} skills={skills} ==", flush=True)
        t0 = time.time()
        try:
            row = run_arm(
                arm=arm,
                exp=exp,
                skills=skills,
                port=args.port,
                out_dir=out_dir,
                timeout=args.timeout,
            )
        except Exception as exc:  # noqa: BLE001
            row = {"arm": arm, "ok": False, "error": str(exc)}
            print(f"FAILED: {exc}", flush=True)
        row["duration_s"] = round(time.time() - t0, 1)
        rows.append(row)
        print(
            f"   ok={row.get('ok')} status={row.get('session_status')} "
            f"html={row.get('primary_html')} ({row['duration_s']}s)",
            flush=True,
        )

    summary = {
        "run_id": out_dir.name,
        "prompt": LANDING_PROMPT,
        "model_note": "uses ~/.anycode/config.json active chat model",
        "rows": rows,
    }
    (out_dir / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    # Convenience index for browser open
    links = []
    for r in rows:
        html = r.get("primary_html")
        if html:
            links.append(
                f'<li><a href="{html}">{r["arm"]}</a> '
                f'ok={r.get("ok")} {r.get("duration_s")}s</li>'
            )
    (out_dir / "index.html").write_text(
        "<!DOCTYPE html><meta charset=utf-8><title>AQ visual arms</title>"
        "<h1>Agent quality — landing arms</h1><ul>"
        + "\n".join(links)
        + "</ul><p>Open each link to compare visuals.</p>\n",
        encoding="utf-8",
    )
    print(f"\nWrote {out_dir}")
    print(f"Open: {out_dir / 'index.html'}")
    return 0 if all(r.get("ok") for r in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
