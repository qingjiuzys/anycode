#!/usr/bin/env python3
"""Minimal anyCode production eval harness.

The harness is intentionally credential-free. It checks deterministic CLI
scenarios first; model-backed repository tasks can be appended later without
changing the result schema.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BIN = os.environ.get("ANYCODE_EVAL_BIN", str(ROOT / "target" / "debug" / "anycode"))


@dataclass
class Scenario:
    id: str
    command: list[str]
    expect: str


SCENARIOS = [
    Scenario("help", [BIN, "--help"], "anyCode"),
    Scenario("status-json", [BIN, "status", "--json"], "model"),
    Scenario("doctor-all", [BIN, "doctor", "all", "--json"], "memory.backend"),
    Scenario("doctor-errors", [BIN, "doctor", "errors", "--json"], "eval.scenario_failed"),
    Scenario("mcp-status", [BIN, "mcp", "status", "--json"], "policy.reconnect"),
    Scenario("cron-runs", [BIN, "cron", "runs", "--limit", "1", "--json"], "["),
    Scenario("memory-doctor", [BIN, "memory", "doctor", "--json"], "memory.backend"),
]


def run_one(s: Scenario) -> dict:
    with tempfile.TemporaryDirectory(prefix="anycode-eval-home-") as home:
        env = os.environ.copy()
        env["HOME"] = home
        p = subprocess.run(
            s.command,
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
        )
    out = p.stdout + p.stderr
    return {
        **asdict(s),
        "status": "pass" if p.returncode == 0 and s.expect in out else "fail",
        "exit_code": p.returncode,
        "stdout_tail": p.stdout[-500:],
        "stderr_tail": p.stderr[-500:],
    }


def main() -> int:
    if "--list" in sys.argv:
        print(json.dumps([asdict(s) for s in SCENARIOS], indent=2))
        return 0
    with_mock = "--with-mock" in sys.argv
    rows = [run_one(s) for s in SCENARIOS]
    if with_mock:
        mock_cmd = [BIN, "eval", "run", "--mock", "--json"]
        env = os.environ.copy()
        with tempfile.TemporaryDirectory(prefix="anycode-eval-home-") as home:
            env["HOME"] = home
            p = subprocess.run(
                mock_cmd,
                cwd=ROOT,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=120,
            )
        combined = p.stdout + p.stderr
        rows.append(
            {
                "id": "mock-fixture-run",
                "command": mock_cmd,
                "expect": "MOCK_EVAL",
                "status": "pass"
                if p.returncode == 0 and "MOCK_EVAL" in combined
                else "fail",
                "exit_code": p.returncode,
                "stdout_tail": p.stdout[-500:],
                "stderr_tail": p.stderr[-500:],
            }
        )
    print(json.dumps(rows, indent=2))
    return 0 if all(r["status"] == "pass" for r in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
