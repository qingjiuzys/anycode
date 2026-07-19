"""JUnit and Markdown report generation."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

from .executor import CaseResult, RunContext


def _usage_number(result: CaseResult, key: str) -> float:
    usage = result.metrics.get("usage", {})
    if not isinstance(usage, dict):
        return 0.0
    summary = usage.get("usage", usage)
    if not isinstance(summary, dict):
        return 0.0
    value = summary.get(key, 0)
    return float(value) if isinstance(value, (int, float)) else 0.0


def write_junit(ctx: RunContext, results: list[CaseResult]) -> Path:
    testsuites = ET.Element("testsuites")
    suite_el = ET.SubElement(
        testsuites,
        "testsuite",
        name=f"anycode-eval-{ctx.profile}",
        tests=str(len(results)),
        failures=str(sum(1 for r in results if r.status == "failed")),
        skipped=str(sum(1 for r in results if r.status == "skipped")),
    )
    for result in results:
        case = ET.SubElement(suite_el, "testcase", classname=result.suite, name=result.id, time=f"{result.duration_seconds:.3f}")
        if result.status == "failed":
            failure = ET.SubElement(case, "failure", message=result.error or "failed")
            failure.text = result.error or "failed"
        elif result.status == "skipped":
            ET.SubElement(case, "skipped")
    path = ctx.results_dir / "junit.xml"
    ET.ElementTree(testsuites).write(path, encoding="utf-8", xml_declaration=True)
    return path


def write_summary(ctx: RunContext, results: list[CaseResult], coverage: dict[str, Any] | None = None) -> Path:
    total = len(results)
    passed = sum(1 for r in results if r.status == "passed")
    failed = sum(1 for r in results if r.status == "failed")
    skipped = sum(1 for r in results if r.status == "skipped")
    p0 = [r for r in results if r.risk == "P0"]
    p0_failed = [r for r in p0 if r.status == "failed"]
    probes = [r for r in results if r.id.startswith("model-probe-")]
    benchmarks = [r for r in results if r.runner == "benchmark"]
    agent_tasks = [r for r in results if r.runner == "dashboard" and r not in probes]
    product_cases = [r for r in results if r not in probes and r not in benchmarks and r not in agent_tasks]

    def score(rows: list[CaseResult]) -> str:
        executed = [r for r in rows if r.status != "skipped"]
        if not executed:
            return "N/A"
        return f"{sum(r.status == 'passed' for r in executed) / len(executed) * 100:.1f}%"

    probe_status = (
        f"passed={sum(r.status == 'passed' for r in probes)}, "
        f"failed={sum(r.status == 'failed' for r in probes)}, "
        f"skipped={sum(r.status == 'skipped' for r in probes)}"
        if probes
        else "not run"
    )
    lines = [
        f"# anyCode Eval — {ctx.profile}",
        "",
        f"- run_id: `{ctx.run_id}`",
        f"- models: `{', '.join(ctx.models) or 'none'}`",
        f"- total: {total} | passed: {passed} | failed: {failed} | skipped: {skipped}",
        f"- product_pass_rate: {score(product_cases)}",
        f"- agent_task_score: {score(agent_tasks)}",
        "- model_pass_at_1: N/A (reported only by completed benchmark adapters)",
        f"- probe_status: {probe_status}",
        "",
    ]
    agent_results = [result for result in results if result.metrics.get("eval_mode") == "live-model"]
    if agent_results:
        task_successes = sum(
            result.metrics.get("completed_turns") == result.metrics.get("prompt_turns")
            and result.metrics.get("assertions_passed") == result.metrics.get("assertions_total")
            for result in agent_results
        )
        compliant = sum(bool(result.metrics.get("trajectory_compliant")) for result in agent_results)
        tool_calls = sum(int(result.metrics.get("tool_calls", 0)) for result in agent_results)
        tool_errors = sum(int(result.metrics.get("tool_errors", 0)) for result in agent_results)
        retries = sum(int(result.metrics.get("retry_count", 0)) for result in agent_results)
        total_tokens = sum(_usage_number(result, "total_tokens") for result in agent_results)
        estimated_cost_cny = sum(
            _usage_number(result, "estimated_cost_cny") for result in agent_results
        )
        repetitions = max(int(result.metrics.get("repetitions", 1)) for result in agent_results)
        lines.extend(
            [
                "## Agent benchmark metrics",
                f"- repetitions: {repetitions}",
                f"- task_success_rate: {task_successes / len(agent_results) * 100:.1f}%",
                f"- trajectory_compliance_rate: {compliant / len(agent_results) * 100:.1f}%",
                f"- tool_error_rate: {(tool_errors / tool_calls * 100) if tool_calls else 0:.1f}%",
                f"- tool_calls: {tool_calls} | tool_errors: {tool_errors} | retries: {retries}",
                f"- total_tokens: {int(total_tokens)} | estimated_cost_cny: {estimated_cost_cny:.6f}",
                "",
            ]
        )
    if p0:
        lines.append(f"- P0: {len(p0)} cases, {len(p0_failed)} failed")
        if p0_failed:
            lines.append("")
            lines.append("## P0 failures")
            for result in p0_failed:
                lines.append(f"- `{result.id}` ({result.suite}): {result.error}")
    if failed:
        lines.append("")
        lines.append("## Failures")
        for result in results:
            if result.status == "failed":
                lines.append(f"- `{result.id}` [{result.suite}]: {result.error}")
    if coverage:
        lines.append("")
        lines.append("## Coverage snapshot")
        lines.append("```json")
        lines.append(json.dumps(coverage, indent=2, ensure_ascii=False))
        lines.append("```")
    path = ctx.results_dir / "summary.md"
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    _write_category_reports(
        ctx,
        product_cases=product_cases,
        benchmarks=benchmarks,
        probes=probes,
        agent_tasks=agent_tasks,
    )
    return path


def _write_category_reports(
    ctx: RunContext,
    *,
    product_cases: list[CaseResult],
    benchmarks: list[CaseResult],
    probes: list[CaseResult],
    agent_tasks: list[CaseResult],
) -> None:
    """Keep product, naked-model, and Agent E2E evidence in separate artifacts."""

    def status_lines(rows: list[CaseResult]) -> list[str]:
        if not rows:
            return ["- No cases executed in this category."]
        return [
            f"- `{row.id}`: {row.status}"
            + (f" — {row.error}" if row.error else "")
            for row in rows
        ]

    product_executed = [row for row in product_cases if row.status != "skipped"]
    product_rate = (
        sum(row.status == "passed" for row in product_executed) / len(product_executed) * 100
        if product_executed
        else None
    )
    product = [
        "# Product regression report",
        "",
        f"- run_id: `{ctx.run_id}`",
        f"- product_pass_rate: {product_rate:.1f}%" if product_rate is not None else "- product_pass_rate: N/A",
        "",
        *status_lines(product_cases),
        "",
    ]
    (ctx.results_dir / "product-regression.md").write_text("\n".join(product), encoding="utf-8")

    model = [
        "# Model benchmark report",
        "",
        f"- run_id: `{ctx.run_id}`",
        "- model_pass_at_1 is only valid when an adapter produced a scored dataset.",
        "",
        "## Benchmark adapters",
        *status_lines(benchmarks),
        "",
        "## Availability probes",
        *status_lines(probes),
        "",
    ]
    (ctx.results_dir / "model-benchmark.md").write_text("\n".join(model), encoding="utf-8")

    live_rows = [
        row for row in agent_tasks if row.metrics.get("eval_mode") == "live-model"
    ]
    successful = sum(row.status == "passed" for row in live_rows)
    compliant = sum(bool(row.metrics.get("trajectory_compliant")) for row in live_rows)
    agent = [
        "# Agent E2E report",
        "",
        f"- run_id: `{ctx.run_id}`",
        f"- models: `{', '.join(ctx.models) or 'none'}`",
        f"- executed_tasks: {len(live_rows)}",
        f"- task_success_rate: {successful / len(live_rows) * 100:.1f}%"
        if live_rows
        else "- task_success_rate: N/A",
        f"- trajectory_compliance_rate: {compliant / len(live_rows) * 100:.1f}%"
        if live_rows
        else "- trajectory_compliance_rate: N/A",
        "",
        *status_lines(agent_tasks),
        "",
    ]
    (ctx.results_dir / "agent-e2e.md").write_text("\n".join(agent), encoding="utf-8")
