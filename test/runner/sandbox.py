"""Isolated execution for untrusted generated code and benchmarks."""

from __future__ import annotations

import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SandboxResult:
    ok: bool
    exit_code: int
    stdout: str
    stderr: str
    duration_seconds: float
    mode: str


def docker_available() -> bool:
    return shutil.which("docker") is not None


def run_in_docker(
    command: list[str],
    *,
    workspace: Path,
    image: str = "python:3.11-slim",
    timeout_seconds: int = 120,
    memory: str = "512m",
    cpus: str = "1",
) -> SandboxResult:
    started = time.time()
    if not docker_available():
        return SandboxResult(
            ok=False,
            exit_code=127,
            stdout="",
            stderr="docker not available",
            duration_seconds=time.time() - started,
            mode="docker-missing",
        )
    mount = workspace.resolve()
    proc = subprocess.run(
        [
            "docker",
            "run",
            "--rm",
            "--network=none",
            "--read-only",
            "--tmpfs",
            "/tmp:rw,noexec,nosize=64m",
            "--memory",
            memory,
            "--cpus",
            cpus,
            "-v",
            f"{mount}:/workspace:ro",
            "-w",
            "/workspace",
            image,
            *command,
        ],
        capture_output=True,
        text=True,
        timeout=timeout_seconds,
        check=False,
    )
    return SandboxResult(
        ok=proc.returncode == 0,
        exit_code=proc.returncode,
        stdout=proc.stdout or "",
        stderr=proc.stderr or "",
        duration_seconds=time.time() - started,
        mode="docker",
    )


def run_local_checked(command: str, *, cwd: Path, timeout_seconds: int = 60) -> SandboxResult:
    """Fallback for trusted fixture validation only."""
    started = time.time()
    proc = subprocess.run(
        command,
        shell=True,
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=timeout_seconds,
        check=False,
    )
    return SandboxResult(
        ok=proc.returncode == 0,
        exit_code=proc.returncode,
        stdout=proc.stdout or "",
        stderr=proc.stderr or "",
        duration_seconds=time.time() - started,
        mode="local-trusted",
    )
