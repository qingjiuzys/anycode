"""Static analysis and deterministic validators for eval cases."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass
class ValidationResult:
    ok: bool
    message: str
    details: dict[str, Any] | None = None


def validate_file_exists(path: Path, pattern: str | None = None) -> ValidationResult:
    if not path.exists():
        return ValidationResult(False, f"missing file: {path}")
    if pattern and path.is_file():
        content = path.read_text(encoding="utf-8", errors="replace")
        if not re.search(pattern, content):
            return ValidationResult(False, f"pattern not found in {path}", {"pattern": pattern})
    return ValidationResult(True, "ok")


def validate_json_path(payload: Any, path: str, expected: Any | None = None) -> ValidationResult:
    current = payload
    for part in path.split("."):
        if part.endswith("]"):
            key, idx_raw = part[:-1].split("[")
            if key:
                if not isinstance(current, dict) or key not in current:
                    return ValidationResult(False, f"missing key {key} at {path}")
                current = current[key]
            idx = int(idx_raw)
            if not isinstance(current, list) or idx >= len(current):
                return ValidationResult(False, f"index {idx} out of range at {path}")
            current = current[idx]
        else:
            if not isinstance(current, dict) or part not in current:
                return ValidationResult(False, f"missing path segment {part} in {path}")
            current = current[part]
    if expected is not None and current != expected:
        return ValidationResult(False, f"expected {expected!r}, got {current!r}", {"path": path})
    return ValidationResult(True, "ok", {"value": current, "path": path})


def validate_trace_event(trace: dict[str, Any], event_type: str) -> ValidationResult:
    events = trace.get("events", [])
    types = {e.get("event_type") for e in events if isinstance(e, dict)}
    if event_type in types:
        return ValidationResult(True, "ok", {"event_type": event_type})
    return ValidationResult(False, f"trace missing event_type={event_type}", {"seen": sorted(types)})


def validate_cargo_tests(crate: str, filter_expr: str | None = None) -> ValidationResult:
    cmd = f"cargo test -p {crate}"
    if filter_expr:
        cmd += f" {filter_expr}"
    proc = subprocess.run(cmd, shell=True, capture_output=True, text=True, check=False)
    if proc.returncode == 0:
        return ValidationResult(True, "ok")
    return ValidationResult(False, (proc.stderr or proc.stdout)[-2000:])


def load_json_file(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


@dataclass
class ToolFinding:
    tool: str
    rule_id: str
    message: str
    severity: str
    line: int | None = None


@dataclass
class ValidationReport:
    tool: str
    ok: bool
    findings: list[ToolFinding] = field(default_factory=list)
    sarif_path: Path | None = None
    stdout: str = ""
    stderr: str = ""
    metrics: dict[str, Any] = field(default_factory=dict)


def _run(cmd: list[str], cwd: Path, timeout: int = 120) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def run_bandit(target_dir: Path, sarif_dir: Path, case_id: str) -> ValidationReport:
    sarif_root = sarif_dir.resolve()
    sarif_root.mkdir(parents=True, exist_ok=True)
    sarif_path = sarif_root / f"{case_id}.bandit.sarif"
    json_path = sarif_root / f"{case_id}.bandit.json"
    target = target_dir.resolve()
    proc = _run(
        [
            sys.executable,
            "-m",
            "bandit",
            "-r",
            str(target),
            "-f",
            "json",
            "-o",
            str(json_path),
            "-q",
        ],
        cwd=target,
    )
    findings = _parse_bandit_json(json_path) if json_path.exists() else []
    if findings:
        _write_minimal_sarif(sarif_path, findings, tool_name="bandit")
    high = [f for f in findings if f.severity in {"HIGH", "MEDIUM"}]
    return ValidationReport(
        tool="bandit",
        ok=proc.returncode in {0, 1},
        findings=findings,
        sarif_path=sarif_path if sarif_path.exists() else None,
        stdout=proc.stdout,
        stderr=proc.stderr,
        metrics={"high_or_medium": len(high)},
    )


def run_ruff(target_dir: Path, sarif_dir: Path, case_id: str, select: list[str] | None = None) -> ValidationReport:
    sarif_root = sarif_dir.resolve()
    sarif_root.mkdir(parents=True, exist_ok=True)
    sarif_path = sarif_root / f"{case_id}.ruff.sarif"
    target = target_dir.resolve()
    cmd = [
        sys.executable,
        "-m",
        "ruff",
        "check",
        str(target),
        "--output-format",
        "sarif",
        "--output-file",
        str(sarif_path),
    ]
    if select:
        cmd.extend(["--select", ",".join(select)])
    proc = _run(cmd, cwd=target)
    findings = _parse_generic_sarif(sarif_path, "ruff") if sarif_path.exists() else []
    return ValidationReport(
        tool="ruff",
        ok=proc.returncode in {0, 1},
        findings=findings,
        sarif_path=sarif_path if sarif_path.exists() else None,
        stdout=proc.stdout,
        stderr=proc.stderr,
        metrics={"violations": len(findings)},
    )


def run_pylint(target_file: Path, sarif_dir: Path, case_id: str) -> ValidationReport:
    sarif_root = sarif_dir.resolve()
    sarif_root.mkdir(parents=True, exist_ok=True)
    sarif_path = sarif_root / f"{case_id}.pylint.sarif"
    source = target_file.resolve()
    proc = _run(
        [
            sys.executable,
            "-m",
            "pylint",
            str(source),
            "--output-format=json",
            "--score=no",
            "--reports=no",
        ],
        cwd=source.parent,
    )
    findings = _parse_pylint_json(proc.stdout)
    if findings:
        _write_minimal_sarif(sarif_path, findings, tool_name="pylint")
    return ValidationReport(
        tool="pylint",
        ok=True,
        findings=findings,
        sarif_path=sarif_path if sarif_path.exists() else None,
        stdout=proc.stdout,
        stderr=proc.stderr,
        metrics={"violations": len(findings)},
    )


def run_radon(target_file: Path, max_complexity: int = 10) -> ValidationReport:
    source = target_file.resolve()
    proc = _run(
        [sys.executable, "-m", "radon", "cc", "-j", str(source)],
        cwd=source.parent,
    )
    metrics: dict[str, Any] = {"max_complexity": 0, "blocks": []}
    findings: list[ToolFinding] = []
    if proc.stdout.strip():
        try:
            data = json.loads(proc.stdout)
            for path, blocks in data.items():
                for block in blocks:
                    complexity = int(block.get("complexity", 0))
                    metrics["blocks"].append({"name": block.get("name"), "complexity": complexity})
                    metrics["max_complexity"] = max(metrics["max_complexity"], complexity)
                    if complexity > max_complexity:
                        findings.append(
                            ToolFinding(
                                tool="radon",
                                rule_id="C901",
                                message=f"{block.get('name')} complexity {complexity} > {max_complexity}",
                                severity="MEDIUM",
                                line=block.get("lineno"),
                            )
                        )
        except json.JSONDecodeError:
            pass
    return ValidationReport(
        tool="radon",
        ok=True,
        findings=findings,
        metrics=metrics,
    )


def _parse_bandit_json(path: Path) -> list[ToolFinding]:
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return []
    out: list[ToolFinding] = []
    for item in data.get("results", []):
        out.append(
            ToolFinding(
                tool="bandit",
                rule_id=str(item.get("test_id", "unknown")),
                message=str(item.get("issue_text", "")),
                severity=str(item.get("issue_severity", "MEDIUM")).upper(),
                line=item.get("line_number"),
            )
        )
    return out


def _parse_bandit_sarif(path: Path) -> list[ToolFinding]:
    return _parse_generic_sarif(path, "bandit")


def _parse_generic_sarif(path: Path, tool: str) -> list[ToolFinding]:
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return []
    out: list[ToolFinding] = []
    for run in data.get("runs", []):
        for result in run.get("results", []):
            rule_id = result.get("ruleId", "unknown")
            level = result.get("level", "warning")
            message = result.get("message", {}).get("text", "")
            line = None
            locations = result.get("locations") or []
            if locations:
                region = locations[0].get("physicalLocation", {}).get("region", {})
                line = region.get("startLine")
            out.append(
                ToolFinding(
                    tool=tool,
                    rule_id=str(rule_id),
                    message=message,
                    severity=level.upper(),
                    line=line,
                )
            )
    return out


def _parse_pylint_json(stdout: str) -> list[ToolFinding]:
    if not stdout.strip():
        return []
    try:
        items = json.loads(stdout)
    except json.JSONDecodeError:
        return []
    out: list[ToolFinding] = []
    for item in items:
        out.append(
            ToolFinding(
                tool="pylint",
                rule_id=str(item.get("message-id", item.get("symbol", "unknown"))),
                message=str(item.get("message", "")),
                severity=str(item.get("type", "warning")).upper(),
                line=item.get("line"),
            )
        )
    return out


def _write_minimal_sarif(path: Path, findings: list[ToolFinding], tool_name: str) -> None:
    results = []
    for finding in findings:
        result: dict[str, Any] = {
            "ruleId": finding.rule_id,
            "level": finding.severity.lower() if finding.severity else "warning",
            "message": {"text": finding.message},
        }
        if finding.line:
            result["locations"] = [
                {
                    "physicalLocation": {
                        "region": {"startLine": finding.line},
                    }
                }
            ]
        results.append(result)
    doc = {
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{"tool": {"driver": {"name": tool_name}}, "results": results}],
    }
    path.write_text(json.dumps(doc, indent=2), encoding="utf-8")
