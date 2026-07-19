"""Cargo and static command execution helpers."""

from __future__ import annotations

import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

from .manifest import CaseSpec

REPO = Path(__file__).resolve().parents[2]


@dataclass
class CommandResult:
    exit_code: int
    stdout: str
    stderr: str
    duration_seconds: float


def run_command(
    command: str,
    *,
    cwd: Path | None = None,
    timeout_seconds: int = 300,
    env: dict[str, str] | None = None,
) -> CommandResult:
    started = time.time()
    proc = subprocess.run(
        command,
        shell=True,
        cwd=cwd or REPO,
        capture_output=True,
        text=True,
        timeout=timeout_seconds,
        env={**os.environ, **(env or {})},
        check=False,
    )
    return CommandResult(
        exit_code=proc.returncode,
        stdout=proc.stdout or "",
        stderr=proc.stderr or "",
        duration_seconds=time.time() - started,
    )


def run_cargo_case(case: CaseSpec) -> CommandResult:
    command = case.command or "cargo test --workspace"
    build_ui = case.meta.get("build_dashboard_ui", False)
    workdir = case.meta.get("workdir")
    cwd = REPO / workdir if workdir else REPO
    env: dict[str, str] = {}
    if build_ui:
        env["ANYCODE_BUILD_DASHBOARD_UI"] = "1"
    return run_command(command, cwd=cwd, timeout_seconds=case.timeout_seconds, env=env)


def tail_output(result: CommandResult, limit: int = 4000) -> str:
    blob = result.stderr or result.stdout
    if len(blob) <= limit:
        return blob
    return blob[-limit:]
