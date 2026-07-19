"""Environment and dependency checks."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent


@dataclass
class CheckResult:
    name: str
    ok: bool
    detail: str


def run_doctor() -> list[CheckResult]:
    checks: list[CheckResult] = []
    checks.append(_check_python())
    checks.append(_check_eval_python_deps())
    checks.append(_check_command("cargo", ["--version"]))
    checks.append(_check_command("node", ["--version"]))
    checks.append(_check_command("npm", ["--version"]))
    checks.append(_check_path(REPO / "Cargo.toml", "workspace Cargo.toml"))
    checks.append(_check_path(ROOT / "manifests" / "smoke.toml", "smoke manifest"))
    checks.append(_check_path(ROOT / "requirements" / "catalog.toml", "P0/P1 catalog"))
    checks.append(_check_dashboard_binary())
    checks.append(_check_optional("docker", ["--version"], "Docker (benchmark sandbox)"))
    checks.append(_check_optional("chromium", ["--version"], "Chromium (browser smoke)"))
    checks.append(_check_env_script("ANYCODE_DASHBOARD_TEST_AUTH_BYPASS", "scripts/dashboard-e2e-server.sh"))
    return checks


def _check_python() -> CheckResult:
    version = sys.version_info
    ok = version >= (3, 11)
    return CheckResult(
        "python",
        ok,
        f"{version.major}.{version.minor}.{version.micro}"
        + ("" if ok else " (need >= 3.11)"),
    )


def _check_eval_python_deps() -> CheckResult:
    missing: list[str] = []
    for module in ("bandit", "ruff", "pylint", "radon", "yaml"):
        try:
            __import__(module if module != "yaml" else "yaml")
        except ImportError:
            missing.append(module)
    if missing:
        venv = ROOT / ".venv" / "bin" / "python"
        hint = f"pip install -r test/requirements.txt"
        if venv.exists():
            hint = f"{venv} -m pip install -r test/requirements.txt (or run via test/run.py)"
        return CheckResult("eval-python-deps", False, f"missing: {', '.join(missing)} — {hint}")
    return CheckResult("eval-python-deps", True, "bandit, ruff, pylint, radon, pyyaml")


def _check_command(name: str, args: list[str]) -> CheckResult:
    path = shutil.which(name)
    if not path:
        return CheckResult(name, False, "not found in PATH")
    try:
        out = subprocess.run(
            [path, *args],
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
        detail = (out.stdout or out.stderr).strip().splitlines()[0] if out.stdout or out.stderr else path
        return CheckResult(name, out.returncode == 0, detail)
    except Exception as exc:  # noqa: BLE001
        return CheckResult(name, False, str(exc))


def _check_optional(name: str, args: list[str], label: str) -> CheckResult:
    result = _check_command(name, args)
    if not result.ok:
        result.detail = f"optional: {label} — {result.detail}"
    return CheckResult(result.name, True, result.detail)


def _check_path(path: Path, label: str) -> CheckResult:
    return CheckResult(label, path.exists(), str(path))


def _check_dashboard_binary() -> CheckResult:
    bin_path = REPO / "target" / "release" / "anycode-dashboard-serve"
    if bin_path.exists():
        return CheckResult("anycode-dashboard-serve", True, str(bin_path))
    return CheckResult(
        "anycode-dashboard-serve",
        False,
        "missing — run: ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode-dashboard --features embedded-ui --bin anycode-dashboard-serve",
    )


def _check_env_script(name: str, script_rel: str) -> CheckResult:
    script = REPO / script_rel
    if script.exists() and name in script.read_text(encoding="utf-8"):
        return CheckResult(name, True, f"exported in {script_rel}")
    value = os.environ.get(name)
    if value:
        return CheckResult(name, True, f"{value} (environment)")
    return CheckResult(name, True, f"not set locally — expected in {script_rel}")


def print_doctor_report(checks: list[CheckResult]) -> int:
    failed = 0
    for check in checks:
        status = "OK" if check.ok else "FAIL"
        print(f"[{status}] {check.name}: {check.detail}")
        if not check.ok:
            failed += 1
    if failed:
        print(f"\n{failed} required check(s) failed.")
        return 1
    print("\nAll required checks passed.")
    return 0
