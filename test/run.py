#!/usr/bin/env python3
"""anyCode eval runner — assembly layer over runtime-native EvalResult JSON traces."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent

if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


def _prefer_venv_python() -> None:
    venv_root = ROOT / ".venv"
    venv_python = venv_root / "bin" / "python"
    if not venv_python.exists():
        return
    if os.environ.get("VIRTUAL_ENV") == str(venv_root.resolve()):
        return
    env = os.environ.copy()
    env["VIRTUAL_ENV"] = str(venv_root.resolve())
    env["PATH"] = f"{venv_root / 'bin'}:{env.get('PATH', '')}"
    os.execve(str(venv_python), [str(venv_python), *sys.argv], env)


_prefer_venv_python()

from runner.doctor import print_doctor_report, run_doctor  # noqa: E402
from runner.executor import CaseResult, execute_case, new_run_context, write_results  # noqa: E402
from runner.manifest import filter_cases, load_profile  # noqa: E402
from runner.model_probe import probe_models, write_model_reports  # noqa: E402
from runner.reporter import write_junit, write_summary  # noqa: E402
from runner.risk_gate import evaluate_coverage  # noqa: E402
from runner.dashboard_client import DashboardClient  # noqa: E402


def parse_models(raw: str | None) -> list[str]:
    if not raw:
        return []
    return [part.strip() for part in raw.split(",") if part.strip()]


def shard_cases(cases: list, shard: str | None) -> list:
    if not shard or "/" not in shard:
        return cases
    index_s, total_s = shard.split("/", 1)
    index = int(index_s)
    total = int(total_s)
    return [case for i, case in enumerate(cases) if i % total == index]


def run_coverage(output_dir: Path) -> dict:
    script = REPO / "scripts" / "test-coverage.sh"
    if not script.exists():
        return {"error": "scripts/test-coverage.sh missing"}
    coverage_dir = output_dir / "coverage"
    coverage_dir.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [str(script), str(coverage_dir)],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    )
    snapshot = {
        "exit_code": proc.returncode,
        "stdout_tail": (proc.stdout or "")[-2000:],
        "stderr_tail": (proc.stderr or "")[-2000:],
    }
    summary_path = coverage_dir / "summary.json"
    if summary_path.exists():
        snapshot["summary"] = json.loads(summary_path.read_text(encoding="utf-8"))
    (coverage_dir / "runner.log").write_text(
        (proc.stdout or "") + "\n" + (proc.stderr or ""),
        encoding="utf-8",
    )
    return snapshot


def run_profile(
    profile: str,
    models: list[str],
    *,
    coverage: bool,
    shard: str | None,
    repetitions: int = 1,
) -> int:
    if profile == "live-model" and not models:
        print("live-model profile requires --models", file=sys.stderr)
        return 2
    spec = load_profile(profile)
    cases = filter_cases(spec.cases, profile, models or None, spec.tier)
    cases = shard_cases(cases, shard)
    if not cases:
        print(f"No cases selected for profile={profile}", file=sys.stderr)
        return 2

    ctx = new_run_context(profile, models)
    ctx.repetitions = repetitions
    print(f"run_id={ctx.run_id} profile={profile} cases={len(cases)}")

    model_probes = []
    if models:
        try:
            client = DashboardClient()
            model_probes = probe_models(client, models)
            write_model_reports(ctx.results_dir, model_probes)
        except Exception as exc:  # noqa: BLE001
            print(f"model probe warning: {exc}", file=sys.stderr)

    results: list[CaseResult] = []
    for attempt in range(1, repetitions + 1):
        ctx.attempt = attempt
        if repetitions > 1:
            print(f"\nAttempt {attempt}/{repetitions}")
        for case in cases:
            print(f"  -> {case.id} ({case.runner})")
            result = execute_case(case, ctx, model_probes=model_probes or None)
            results.append(result)
            print(f"     {result.status} ({result.duration_seconds:.1f}s)")

    write_results(ctx, results)
    write_junit(ctx, results)

    passed_ids = {r.id for r in results if r.status == "passed"}
    executed_ids = {r.id for r in results if r.status != "skipped"}
    risk = evaluate_coverage(passed_ids, executed_ids)

    coverage_snapshot = None
    if coverage:
        coverage_snapshot = run_coverage(ctx.results_dir)

    write_summary(ctx, results, coverage={"risk_gate": risk, "coverage": coverage_snapshot})
    (ctx.results_dir / "risk_gate.json").write_text(
        json.dumps(risk, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    print(f"\nResults: {ctx.results_dir}")
    print(
        f"pass={sum(1 for r in results if r.status == 'passed')} "
        f"fail={sum(1 for r in results if r.status == 'failed')} "
        f"skip={sum(1 for r in results if r.status == 'skipped')}"
    )
    print(f"P0/P1 coverage: {risk['coverage_rate']:.1f}% gate={'PASS' if risk['gate_pass'] else 'FAIL'}")

    failed = any(r.status == "failed" for r in results)
    if not risk["gate_pass"]:
        return 1
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="anyCode eval runner")
    parser.add_argument("command", nargs="?", choices=["doctor"], help="environment checks")
    parser.add_argument(
        "--profile",
        choices=["smoke", "release-candidate", "full", "fixture-ci", "live-model"],
    )
    parser.add_argument("--models", help="comma-separated model aliases (local-1b,agnes,cloud-auto)")
    parser.add_argument("--coverage", action="store_true", help="collect llvm-cov + vitest coverage")
    parser.add_argument("--shard", help="shard index/total, e.g. 0/4")
    parser.add_argument(
        "--repetitions",
        type=int,
        help="repeat each selected case (live-model defaults to 3)",
    )
    args = parser.parse_args(argv)

    if args.command == "doctor":
        return print_doctor_report(run_doctor())

    if not args.profile:
        parser.error(
            "provide --profile smoke|release-candidate|full|fixture-ci|live-model or command doctor"
        )

    repetitions = (
        args.repetitions
        if args.repetitions is not None
        else (3 if args.profile == "live-model" else 1)
    )
    if repetitions < 1:
        parser.error("--repetitions must be >= 1")

    return run_profile(
        args.profile,
        parse_models(args.models),
        coverage=args.coverage,
        shard=args.shard,
        repetitions=repetitions,
    )


if __name__ == "__main__":
    raise SystemExit(main())
