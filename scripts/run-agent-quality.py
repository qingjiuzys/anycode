#!/usr/bin/env python3
"""Run agent-quality 2×2 ablation via Dashboard executor.

Each arm starts a DEDICATED anycode-dashboard-serve with ANYCODE_EVAL_* on the
server process (AgentRuntime reads the arm there — setting env on the runner
alone silently collapses all arms to the server's configuration).

Example:
  python3 scripts/run-agent-quality.py --models deepseek-v4-flash --split dev --reps 2
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
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


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--models", required=True, help="comma-separated model ids")
    ap.add_argument("--split", choices=["dev", "hidden", "challenge"], default="dev")
    ap.add_argument("--reps", type=int, default=2)
    ap.add_argument("--arms", default=",".join(ARMS))
    ap.add_argument("--port-base", type=int, default=43210)
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    out_dir = REPO / "test" / "benchmarks" / "agent-quality" / "results" / time.strftime(
        "%Y%m%d-%H%M%S"
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    selected = [a.strip() for a in args.arms.split(",") if a.strip()]
    manifest = {
        "split": args.split,
        "reps": args.reps,
        "arms": selected,
        "models": args.models,
        "shared": ["TaskCompiler", "GatePolicy", "CompletionGuard"],
    }
    (out_dir / "run-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    rows = []
    for idx, arm in enumerate(selected):
        exp, skills = ARMS[arm]
        port = args.port_base + idx
        print(f"== arm={arm} experience={exp} skills={skills} port={port} ==")
        env = os.environ.copy()
        env["ANYCODE_EVAL_MODE"] = "1"
        env["ANYCODE_EVAL_EXPERIENCE"] = exp
        env["ANYCODE_EVAL_SKILLS"] = skills
        env["ANYCODE_EVAL_ARM"] = arm
        env["ANYCODE_EVAL_DASHBOARD_URL"] = f"http://127.0.0.1:{port}"
        cmd = [
            sys.executable,
            str(REPO / "test" / "run.py"),
            "--profile",
            "agent-quality",
            "--models",
            args.models,
            "--repetitions",
            str(args.reps),
        ]
        if args.dry_run:
            print(" ", " ".join(cmd))
            continue
        db = Path(os.environ.get("TMPDIR", "/tmp")) / f"anycode-aq-{port}-{arm}.db"
        proc = start_dashboard(port, arm, exp, skills, db)
        try:
            wait_health(DashboardClient(f"http://127.0.0.1:{port}"))
            run = subprocess.run(cmd, cwd=REPO, env=env, check=False)
            rows.append({"arm": arm, "exit_code": run.returncode})
            if run.returncode != 0:
                print(f"arm {arm} failed with {run.returncode}", file=sys.stderr)
        finally:
            stop_dashboard(proc)

    (out_dir / "arm-exits.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out_dir}")
    print(
        "Summarize machine scores with:\n"
        f"  python3 scripts/summarize-agent-quality.py {out_dir}/rows.jsonl"
    )
    return 0 if all(r.get("exit_code", 0) == 0 for r in rows) or args.dry_run else 1


if __name__ == "__main__":
    raise SystemExit(main())
