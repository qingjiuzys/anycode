"""Security case validation in isolated temp workspaces with SARIF output."""

from __future__ import annotations

import json
import shutil
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

from .validators import (
    ToolFinding,
    ValidationReport,
    run_bandit,
    run_pylint,
    run_radon,
    run_ruff,
)


@dataclass
class SecurityValidationResult:
    case_id: str
    passed: bool
    mode: str
    sarif_paths: list[str] = field(default_factory=list)
    findings: list[dict[str, Any]] = field(default_factory=list)
    metrics: dict[str, Any] = field(default_factory=dict)
    error: str | None = None


def load_security_spec(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as f:
        return yaml.safe_load(f) or {}


def validate_security_case(
    spec_path: Path,
    sarif_dir: Path,
    *,
    workspace: Path | None = None,
) -> SecurityValidationResult:
    spec = load_security_spec(spec_path)
    case_id = str(spec.get("id", spec_path.stem))
    validation = spec.get("validation", {})
    mode = str(validation.get("mode", "must_detect"))
    tools = list(validation.get("tools", ["bandit", "ruff"]))
    sarif_dir.mkdir(parents=True, exist_ok=True)

    code_spec = spec.get("code_under_test", {})
    rel_path = str(code_spec.get("path", "generated.py"))
    content = code_spec.get("content", "")

    if workspace and (workspace / rel_path).exists():
        source_root = workspace
    elif content:
        tmp = Path(tempfile.mkdtemp(prefix=f"anycode-sec-{case_id}-"))
        try:
            target = tmp / rel_path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding="utf-8")
            return _analyze_tree(
                case_id=case_id,
                root=tmp,
                main_file=target,
                sarif_dir=sarif_dir,
                validation=validation,
                tools=tools,
                mode=mode,
            )
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    else:
        return SecurityValidationResult(
            case_id=case_id,
            passed=False,
            mode=mode,
            error="no code_under_test.content or workspace artifact",
        )

    main_file = source_root / rel_path
    return _analyze_tree(
        case_id=case_id,
        root=source_root,
        main_file=main_file,
        sarif_dir=sarif_dir,
        validation=validation,
        tools=tools,
        mode=mode,
    )


def _analyze_tree(
    *,
    case_id: str,
    root: Path,
    main_file: Path,
    sarif_dir: Path,
    validation: dict[str, Any],
    tools: list[str],
    mode: str,
) -> SecurityValidationResult:
    reports: list[ValidationReport] = []
    all_findings: list[ToolFinding] = []
    sarif_paths: list[str] = []

    if "bandit" in tools:
        report = run_bandit(root, sarif_dir, case_id)
        reports.append(report)
        all_findings.extend(report.findings)
        if report.sarif_path:
            sarif_paths.append(str(report.sarif_path))

    if "ruff" in tools:
        ruff_cfg = validation.get("ruff", {})
        report = run_ruff(root, sarif_dir, case_id, select=list(ruff_cfg.get("select", [])) or None)
        reports.append(report)
        all_findings.extend(report.findings)
        if report.sarif_path:
            sarif_paths.append(str(report.sarif_path))

    if "pylint" in tools and main_file.exists():
        report = run_pylint(main_file, sarif_dir, case_id)
        reports.append(report)
        all_findings.extend(report.findings)
        if report.sarif_path:
            sarif_paths.append(str(report.sarif_path))

    if "radon" in tools and main_file.exists():
        radon_cfg = validation.get("radon", {})
        max_cc = int(radon_cfg.get("max_complexity", 10))
        report = run_radon(main_file, max_complexity=max_cc)
        reports.append(report)
        all_findings.extend(report.findings)

    expected_rules = set(validation.get("expect_rules", []))
    bandit_cfg = validation.get("bandit", {})
    max_high = bandit_cfg.get("max_high_severity", None)

    if mode == "must_detect":
        if expected_rules:
            found_rules = {f.rule_id for f in all_findings}
            passed = bool(expected_rules & found_rules) or len(all_findings) > 0
        else:
            passed = len(all_findings) > 0
    elif mode == "must_be_clean":
        if max_high is not None:
            high = sum(1 for f in all_findings if f.severity in {"HIGH", "ERROR"})
            passed = high <= int(max_high)
        else:
            passed = len(all_findings) == 0
    else:
        passed = False

    findings_payload = [
        {
            "tool": f.tool,
            "rule_id": f.rule_id,
            "severity": f.severity,
            "message": f.message,
            "line": f.line,
        }
        for f in all_findings
    ]

    summary_path = sarif_dir / f"{case_id}.summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "case_id": case_id,
                "mode": mode,
                "passed": passed,
                "findings": findings_payload,
                "sarif": sarif_paths,
            },
            indent=2,
        ),
        encoding="utf-8",
    )

    return SecurityValidationResult(
        case_id=case_id,
        passed=passed,
        mode=mode,
        sarif_paths=sarif_paths,
        findings=findings_payload,
        metrics={
            "finding_count": len(all_findings),
            "tools": [r.tool for r in reports],
            "summary": str(summary_path),
        },
    )
