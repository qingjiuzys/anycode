"""Shared dashboard server lifecycle for agent-quality runners.

Every eval arm needs ANYCODE_EVAL_* set on the SERVER process (AgentRuntime
reads CompileArmFlags::from_eval_env there) — setting it on the runner process
only mislabels arms. Start one dashboard per arm with this helper.
"""
from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "test"))
from runner.dashboard_client import DashboardClient  # noqa: E402


def wait_health(client: DashboardClient, timeout: float = 90) -> None:
    deadline = time.time() + timeout
    last_err: Exception | None = None
    while time.time() < deadline:
        try:
            if client.health():
                return
        except Exception as exc:  # noqa: BLE001
            last_err = exc
        time.sleep(0.5)
    raise RuntimeError(f"dashboard not healthy at {client.base_url}: {last_err}")


def resolve_dashboard_bin() -> Path:
    candidates = [
        REPO / "target" / "release-local" / "anycode-dashboard-serve",
        REPO / "target" / "release" / "anycode-dashboard-serve",
        REPO / "target" / "debug" / "anycode-dashboard-serve",
    ]
    for p in candidates:
        if p.exists():
            return p
    raise FileNotFoundError(
        "anycode-dashboard-serve not built; run:\n"
        "  cargo build --profile release-local -p anycode-dashboard "
        "--features embedded-ui,tools-browser --bin anycode-dashboard-serve"
    )


def start_dashboard(
    port: int,
    arm: str,
    exp: str,
    skills: str,
    db: Path,
    extra_env: dict | None = None,
) -> subprocess.Popen:
    bin_path = resolve_dashboard_bin()
    for suffix in ("", "-wal", "-shm"):
        p = Path(str(db) + suffix) if suffix else db
        if p.exists():
            p.unlink()
    env = os.environ.copy()
    env.update(
        {
            "ANYCODE_DASHBOARD_DB": str(db),
            "ANYCODE_DASHBOARD_RECORD": "0",
            "ANYCODE_DASHBOARD_TEST_AUTH_BYPASS": "1",
            "ANYCODE_IGNORE_APPROVAL": "1",
            "ANYCODE_EVAL_MODE": "1",
            "ANYCODE_EVAL_EXPERIENCE": exp,
            "ANYCODE_EVAL_SKILLS": skills,
            "ANYCODE_EVAL_ARM": arm,
        }
    )
    if extra_env:
        env.update(extra_env)
    log_path = Path(os.environ.get("TMPDIR", "/tmp")) / f"anycode-aq-{port}-{arm}.log"
    log_f = open(log_path, "w", encoding="utf-8")
    proc = subprocess.Popen(
        [str(bin_path), "--host", "127.0.0.1", "--port", str(port), "--db", str(db)],
        cwd=str(REPO),
        env=env,
        stdout=log_f,
        stderr=subprocess.STDOUT,
    )
    proc._aq_log = log_f  # type: ignore[attr-defined]
    proc._aq_log_path = log_path  # type: ignore[attr-defined]
    return proc


def stop_dashboard(proc: subprocess.Popen) -> None:
    log_f = getattr(proc, "_aq_log", None)
    if proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
    if log_f is not None:
        try:
            log_f.close()
        except Exception:  # noqa: BLE001
            pass
    path = getattr(proc, "_aq_log_path", None)
    if path and Path(path).exists() and proc.returncode not in (0, -signal.SIGTERM, -15):
        tail = Path(path).read_text(encoding="utf-8", errors="replace")[-2000:]
        print(f"dashboard exit={proc.returncode} log_tail:\n{tail}", flush=True)
