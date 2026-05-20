#!/usr/bin/env python3
"""Minimal anyCode production eval harness.

Scenarios are loaded from `scripts/eval/scenarios.json` (shared with `anycode eval`).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).resolve().parent / "scenarios.json"
BIN = os.environ.get("ANYCODE_EVAL_BIN", str(ROOT / "target" / "debug" / "anycode"))


@dataclass
class Scenario:
    id: str
    command: list[str]
    expect: str


def load_manifest() -> dict:
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def cli_scenarios() -> list[Scenario]:
    rows = []
    for row in load_manifest()["cli_scenarios"]:
        rows.append(
            Scenario(
                id=row["id"],
                command=[BIN, *row["command"]],
                expect=row["expect"],
            )
        )
    return rows


def mock_fixture_ids() -> list[str]:
    return [row["id"] for row in load_manifest()["mock_fixtures"]]


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
        "id": s.id,
        "command": s.command,
        "expect": s.expect,
        "status": "pass" if p.returncode == 0 and s.expect in out else "fail",
        "exit_code": p.returncode,
        "stdout_tail": p.stdout[-500:],
        "stderr_tail": p.stderr[-500:],
    }


def main() -> int:
    if "--list" in sys.argv:
        manifest = load_manifest()
        print(json.dumps(manifest, indent=2))
        return 0
    with_mock = "--with-mock" in sys.argv
    rows = [run_one(s) for s in cli_scenarios()]
    if with_mock:
        mock_cmd = [BIN, "eval", "run", "--mock", "--json"]
        env = os.environ.copy()
        if "ANYCODE_EVAL_TOOLCHAIN_HOME" not in env and "HOME" in env:
            env["ANYCODE_EVAL_TOOLCHAIN_HOME"] = env["HOME"]
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
        mock_ids = mock_fixture_ids()
        rows.append(
            {
                "id": "mock-fixture-scenarios",
                "command": mock_cmd,
                "expect": "MOCK_EVAL",
                "status": "pass"
                if p.returncode == 0
                and "MOCK_EVAL" in combined
                and all(mid in combined for mid in mock_ids)
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
