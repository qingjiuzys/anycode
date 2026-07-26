"""Execute individual cases and aggregate results."""

from __future__ import annotations

import json
import os
import subprocess
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from .assertion_runner import run_assertions
from .cargo_runner import run_cargo_case, run_command, tail_output
from .dashboard_client import DashboardClient
from .manifest import CaseSpec, ROOT
from .model_probe import filter_cases_for_probe, ModelProbe
from .trajectory_gate import evaluate_trajectory
from .validators import validate_trace_event

REPO = ROOT.parent


@dataclass
class CaseResult:
    id: str
    suite: str
    risk: str
    status: str
    duration_seconds: float
    runner: str
    error: str | None = None
    metrics: dict[str, Any] = field(default_factory=dict)
    artifacts: list[str] = field(default_factory=list)


@dataclass
class RunContext:
    run_id: str
    profile: str
    models: list[str]
    results_dir: Path
    started_at: float
    attempt: int = 1
    repetitions: int = 1


def new_run_context(profile: str, models: list[str] | None) -> RunContext:
    run_id = time.strftime("%Y%m%d-%H%M%S") + "-" + uuid.uuid4().hex[:8]
    results_dir = ROOT / "results" / run_id
    results_dir.mkdir(parents=True, exist_ok=True)
    return RunContext(
        run_id=run_id,
        profile=profile,
        models=list(models or []),
        results_dir=results_dir,
        started_at=time.time(),
    )


def execute_case(
    case: CaseSpec,
    ctx: RunContext,
    *,
    model_probes: list[ModelProbe] | None = None,
) -> CaseResult:
    started = time.time()
    if model_probes:
        na = filter_cases_for_probe(case.suite, model_probes)
        if na == "na":
            return CaseResult(
                id=case.id,
                suite=case.suite,
                risk=case.risk,
                status="skipped",
                duration_seconds=0.0,
                runner=case.runner,
                error="capability N/A (tools=false for local 1B)",
                metrics={"capability": "na"},
            )
    try:
        if case.runner == "cargo":
            return _run_cargo(case, started)
        if case.runner == "npm":
            return _run_npm(case, started)
        if case.runner == "playwright":
            return _run_playwright(case, started)
        if case.runner == "dashboard":
            return _run_dashboard(case, started, ctx)
        if case.runner == "static":
            return _run_static(case, started)
        if case.runner == "security":
            return _run_security(case, started, ctx)
        if case.runner == "benchmark":
            return _run_benchmark(case, started, ctx)
        raise ValueError(f"unknown runner: {case.runner}")
    except Exception as exc:  # noqa: BLE001
        return CaseResult(
            id=case.id,
            suite=case.suite,
            risk=case.risk,
            status="failed",
            duration_seconds=time.time() - started,
            runner=case.runner,
            error=str(exc),
        )


def _run_cargo(case: CaseSpec, started: float) -> CaseResult:
    result = run_cargo_case(case)
    status = "passed" if result.exit_code == 0 else "failed"
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status=status,
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=None if status == "passed" else tail_output(result),
        metrics={"exit_code": result.exit_code},
    )


def _run_npm(case: CaseSpec, started: float) -> CaseResult:
    workdir = case.meta.get("workdir", "crates/dashboard-ui")
    command = case.command or "npm test"
    result = run_command(command, cwd=REPO / workdir, timeout_seconds=case.timeout_seconds)
    status = "passed" if result.exit_code == 0 else "failed"
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status=status,
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=None if status == "passed" else tail_output(result),
        metrics={"exit_code": result.exit_code},
    )


def _run_playwright(case: CaseSpec, started: float) -> CaseResult:
    spec = case.meta.get("spec", "")
    command = case.command or f"npm run test:e2e -- {spec}".strip()
    result = run_command(
        command,
        cwd=REPO / "crates/dashboard-ui",
        timeout_seconds=case.timeout_seconds,
        env={"CI": "1"},
    )
    status = "passed" if result.exit_code == 0 else "failed"
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status=status,
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=None if status == "passed" else tail_output(result),
        metrics={"exit_code": result.exit_code},
    )


def _run_dashboard(case: CaseSpec, started: float, ctx: RunContext) -> CaseResult:
    requires = case.requires
    if requires.get("llm") == "live" and not ctx.models:
        return CaseResult(
            id=case.id,
            suite=case.suite,
            risk=case.risk,
            status="skipped",
            duration_seconds=time.time() - started,
            runner=case.runner,
            error="live LLM required — pass --models",
        )
    client = DashboardClient()
    if not client.health():
        raise RuntimeError("dashboard not healthy — start scripts/dashboard-e2e-server.sh or workbench")
    workspace = ROOT / "workspaces" / ctx.run_id / f"attempt-{ctx.attempt}" / case.id
    workspace.mkdir(parents=True, exist_ok=True)
    fixture = case.meta.get("fixture")
    fixture_root: Path | None = None
    if fixture:
        fixture_root = ROOT / "fixtures" / fixture
        if not fixture_root.exists():
            raise FileNotFoundError(f"fixture missing: {fixture_root}")
        subprocess.run(["cp", "-R", str(fixture_root) + "/.", str(workspace)], check=True)
    project_id = client.create_project(str(workspace.resolve()), f"eval-{case.id}")
    session_id = client.create_session(project_id, case.id)
    prompts = case.meta.get("prompts")
    if not isinstance(prompts, list) or not prompts:
        prompts = [case.meta.get("prompt", "Reply with OK.")]
    normalized_prompts = [
        prompt if isinstance(prompt, str) else str(prompt.get("content", ""))
        for prompt in prompts
        if isinstance(prompt, (str, dict))
    ]
    if not normalized_prompts or any(not prompt.strip() for prompt in normalized_prompts):
        raise ValueError("scenario prompts must contain non-empty text")

    turn_statuses: list[str] = []
    per_turn_timeout = max(1.0, float(case.timeout_seconds) / len(normalized_prompts))
    for turn, prompt in enumerate(normalized_prompts, start=1):
        status_code, payload = client.send_message(
            session_id,
            prompt,
            agent=case.meta.get("agent"),
            skills=case.meta.get("skills"),
            timeout=per_turn_timeout,
        )
        if status_code >= 400:
            raise RuntimeError(f"message turn {turn} failed ({status_code}): {payload}")
        try:
            final = client.wait_session_done(session_id, timeout=per_turn_timeout)
        except TimeoutError:
            client.cancel_session(session_id)
            raise
        turn_statuses.append(final)
        if final != "completed":
            break

    usage = client.get_usage(session_id)
    trace = client.get_trace(session_id)
    replay = client.get_replay(session_id)
    passed = len(turn_statuses) == len(normalized_prompts) and all(
        status == "completed" for status in turn_statuses
    )
    if case.meta.get("expect_trace_event") and isinstance(trace, dict):
        validation = validate_trace_event(trace, case.meta["expect_trace_event"])
        if not validation.ok:
            passed = False

    expected = case.meta.get("expected", {})
    assertions = expected.get("assertions", []) if isinstance(expected, dict) else []
    assertion_report = run_assertions(
        assertions if isinstance(assertions, list) else [],
        workspace=workspace,
        fixture_root=fixture_root,
        trace=trace,
        replay=replay,
    )
    passed = passed and assertion_report.ok

    assertion_names = {
        str(assertion.get("validator"))
        for assertion in assertions
        if isinstance(assertion, dict)
    }
    zero_tool_is_expected = bool(
        assertion_names
        & {
            "agent_asked_clarification",
            "fail_fast_reported",
            "no_destructive_commands",
            "no_secret_leak",
        }
    )
    trajectory = evaluate_trajectory(
        trace,
        replay=replay,
        required_tools=list(case.requires.get("tools", [])),
        policy=case.meta.get("trajectory") if isinstance(case.meta.get("trajectory"), dict) else None,
        allow_zero_tools=zero_tool_is_expected,
    )
    passed = passed and trajectory.ok

    trace_events = trace.get("events", []) if isinstance(trace, dict) else []
    case_artifact = (
        ctx.results_dir / "traces" / f"attempt-{ctx.attempt}" / f"{case.id}.json"
    )
    case_artifact.parent.mkdir(parents=True, exist_ok=True)
    case_artifact.write_text(
        json.dumps(
            {
                "case_id": case.id,
                "session_id": session_id,
                "prompts": normalized_prompts,
                "turn_statuses": turn_statuses,
                "trace": trace,
                "replay": replay,
                "assertions": assertion_report.results,
                "trajectory": {
                    "ok": trajectory.ok,
                    "violations": trajectory.violations,
                    "metrics": trajectory.metrics,
                },
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    errors: list[str] = []
    if not all(status == "completed" for status in turn_statuses):
        errors.append(f"turn statuses: {turn_statuses}")
    errors.extend(
        f"{item['validator']}: {item['message']}"
        for item in assertion_report.results
        if not item["ok"]
    )
    errors.extend(f"trajectory: {violation}" for violation in trajectory.violations)
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status="passed" if passed else "failed",
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=None if passed else "; ".join(errors) or "scenario validation failed",
        metrics={
            "eval_mode": "live-model",
            "attempt": ctx.attempt,
            "repetitions": ctx.repetitions,
            "eval_arm": os.environ.get("ANYCODE_EVAL_ARM")
            or {
                ("1", "1"): "experience_skill",
                ("1", "0"): "experience_only",
                ("0", "1"): "skill_only",
                ("0", "0"): "baseline",
            }.get(
                (
                    os.environ.get("ANYCODE_EVAL_EXPERIENCE", "1"),
                    os.environ.get("ANYCODE_EVAL_SKILLS", "1"),
                ),
                "production",
            ),
            "eval_experience": os.environ.get("ANYCODE_EVAL_EXPERIENCE"),
            "eval_skills": os.environ.get("ANYCODE_EVAL_SKILLS"),
            "eval_mode_flag": os.environ.get("ANYCODE_EVAL_MODE"),
            "session_status": turn_statuses[-1] if turn_statuses else "not_started",
            "prompt_turns": len(normalized_prompts),
            "completed_turns": sum(status == "completed" for status in turn_statuses),
            "usage": usage,
            "trace_events": len(trace_events) if isinstance(trace_events, list) else 0,
            "assertions_total": len(assertion_report.results),
            "assertions_passed": sum(item["ok"] for item in assertion_report.results),
            **trajectory.metrics,
        },
        artifacts=[str(workspace), str(case_artifact)],
    )


def _run_static(case: CaseSpec, started: float) -> CaseResult:
    command = case.command
    if not command:
        raise ValueError("static runner requires command")
    result = run_command(command, timeout_seconds=case.timeout_seconds)
    status = "passed" if result.exit_code == 0 else "failed"
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status=status,
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=None if status == "passed" else tail_output(result),
        metrics={"eval_mode": case.meta.get("eval_mode", "fixture-ci")},
    )


def _run_security(case: CaseSpec, started: float, ctx: RunContext) -> CaseResult:
    from .security_validator import validate_security_case

    spec_path = case.path or ROOT / "cases" / case.suite / f"{case.id}.yaml"
    if not spec_path.exists():
        raise FileNotFoundError(f"security spec missing: {spec_path}")
    sarif_dir = ctx.results_dir / "sarif"
    workspace: Path | None = None
    if case.requires.get("llm") == "live" and ctx.models:
        dash = _run_dashboard(case, started, ctx)
        if dash.artifacts:
            workspace = Path(dash.artifacts[0])
        if dash.status != "passed" and workspace is None:
            return CaseResult(
                id=case.id,
                suite=case.suite,
                risk=case.risk,
                status="failed",
                duration_seconds=time.time() - started,
                runner=case.runner,
                error=dash.error or "dashboard generation failed",
            )
    result = validate_security_case(spec_path, sarif_dir, workspace=workspace)
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status="passed" if result.passed else "failed",
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=result.error if not result.passed else None,
        metrics={
            "mode": result.mode,
            "finding_count": len(result.findings),
            **result.metrics,
        },
        artifacts=result.sarif_paths,
    )


def _run_benchmark(case: CaseSpec, started: float, ctx: RunContext) -> CaseResult:
    adapter = case.meta.get("adapter")
    if not adapter:
        raise ValueError("benchmark runner requires meta.adapter")
    script = ROOT / "benchmarks" / adapter / "run_adapter.sh"
    if not script.exists():
        return CaseResult(
            id=case.id,
            suite=case.suite,
            risk=case.risk,
            status="skipped",
            duration_seconds=time.time() - started,
            runner=case.runner,
            error=f"adapter not installed: {adapter}",
        )
    models = ",".join(ctx.models) if ctx.models else ""
    proc = subprocess.run(
        [str(script), models, str(ctx.results_dir)],
        capture_output=True,
        text=True,
        timeout=case.timeout_seconds,
        check=False,
    )
    benchmark_status = "executed"
    for result_path in ctx.results_dir.glob("*.json"):
        try:
            payload = json.loads(result_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        if payload.get("adapter") == adapter and payload.get("status") in {"preflight_ok", "stub_executed"}:
            benchmark_status = payload["status"]
            break
    status = "passed" if proc.returncode == 0 else "failed"
    if benchmark_status in {"preflight_ok", "stub_executed"}:
        status = "skipped"
    return CaseResult(
        id=case.id,
        suite=case.suite,
        risk=case.risk,
        status=status,
        duration_seconds=time.time() - started,
        runner=case.runner,
        error=(
            f"benchmark {benchmark_status}; no model score produced"
            if status == "skipped"
            else None if status == "passed"
            else (proc.stderr or proc.stdout)[-4000:]
        ),
        metrics={"benchmark_status": benchmark_status, "exit_code": proc.returncode},
    )


def write_results(ctx: RunContext, results: list[CaseResult]) -> None:
    jsonl_path = ctx.results_dir / "cases.jsonl"
    with jsonl_path.open("w", encoding="utf-8") as f:
        for result in results:
            f.write(json.dumps(asdict(result), ensure_ascii=False) + "\n")
