#!/usr/bin/env python3
"""Verify scenario-corpus counts, schema, and manifest loader integration."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from runner.manifest import load_all_suite_cases  # noqa: E402

try:
    import yaml
except ImportError:
    print("FAIL: PyYAML not installed (pip install -r test/requirements.txt)")
    sys.exit(1)

REQUIRED = {
    "browser": {"min": 12, "complex_min": 0},
    "skills": {"min": 44, "complex_min": 0},  # 14 contracts + 30 A/B
    "office": {"min": 60, "complex_min": 36},
    "coding": {"min": 30, "complex_min": 20},  # complex + long
    "interaction": {"min": 19, "complex_min": 0},  # 1 + 12 + 6
    "robustness": {"min": 50, "complex_min": 0},
    "security": {"min": 40, "complex_min": 0},
}

SCENARIO_SCHEMA_SUITES = {"browser", "skills", "office", "coding", "interaction", "robustness"}
SECURITY_REQUIRED_FIELDS = {"id", "risk", "code_under_test", "validation"}

REQUIRED_FIELDS = {"id", "tier", "risk", "prompts", "expected", "requires", "timeout_seconds"}
ASSERTION_VALIDATORS = {
    "file_exists",
    "file_contains",
    "file_json_keys",
    "file_line_changed",
    "dom_contains",
    "dom_count",
    "dom_attribute",
    "dom_has_class",
    "dom_not_contains",
    "pytest_pass",
    "cargo_test_pass",
    "npm_test_pass",
    "typecheck_pass",
    "http_get_json",
    "skill_frontmatter_valid",
    "skill_vet_pass",
    "skill_invoked",
    "docx_paragraph_contains",
    "docx_styles_valid",
    "xlsx_cell_equals",
    "xlsx_formula_eval",
    "pptx_slide_count",
    "pptx_title_contains",
    "cross_format_refs_consistent",
    "markdown_heading",
    "agent_asked_clarification",
    "fail_fast_reported",
    "no_destructive_commands",
    "no_secret_leak",
}

FIXTURE_PATHS = [
    "fixtures/browser/site/index.html",
    "fixtures/projects/flask-starter/app.py",
    "fixtures/projects/cross-file-mini/project-01",
    "fixtures/projects/large-repo/src/handlers.rs",
    "fixtures/skills/ab-fixtures/notes.md",
]


def count_yaml_files(suite: str) -> int:
    d = ROOT / "cases" / suite
    return len(list(d.glob("*.yaml"))) if d.exists() else 0


def validate_yaml(path: Path) -> list[str]:
    errors: list[str] = []
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        return [f"{path.name}: not a mapping"]
    missing = REQUIRED_FIELDS - set(data.keys())
    if missing:
        errors.append(f"{path.name}: missing fields {sorted(missing)}")
    expected = data.get("expected", {})
    assertions = expected.get("assertions", []) if isinstance(expected, dict) else []
    if not assertions:
        errors.append(f"{path.name}: expected.assertions empty")
    for i, assertion in enumerate(assertions):
        if not isinstance(assertion, dict):
            errors.append(f"{path.name}: assertion[{i}] not a dict")
            continue
        validator = assertion.get("validator")
        if validator not in ASSERTION_VALIDATORS:
            errors.append(f"{path.name}: unknown validator {validator!r}")
    requires = data.get("requires", {})
    if not isinstance(requires, dict):
        errors.append(f"{path.name}: requires must be dict")
    return errors


def validate_security_yaml(path: Path) -> list[str]:
    errors: list[str] = []
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        return [f"{path.name}: not a mapping"]
    missing = SECURITY_REQUIRED_FIELDS - set(data.keys())
    if missing:
        errors.append(f"{path.name}: missing fields {sorted(missing)}")
    validation = data.get("validation", {})
    if not isinstance(validation, dict):
        errors.append(f"{path.name}: validation must be dict")
    elif not validation.get("mode"):
        errors.append(f"{path.name}: validation.mode required")
    code = data.get("code_under_test", {})
    if not isinstance(code, dict) or not code.get("content"):
        errors.append(f"{path.name}: code_under_test.content required")
    return errors


def main() -> int:
    failures: list[str] = []
    suites = load_all_suite_cases()

    print("=== Scenario corpus verification ===\n")
    total_cases = 0
    total_yaml = 0

    for suite, req in REQUIRED.items():
        loaded = suites.get(suite, [])
        yaml_count = count_yaml_files(suite)
        total_cases += len(loaded)
        total_yaml += yaml_count
        complex_count = sum(
            1 for c in loaded if c.meta.get("complexity") in ("complex", "long")
        )
        status = "OK" if len(loaded) >= req["min"] else "FAIL"
        print(
            f"[{status}] {suite}: index={len(loaded)} yaml={yaml_count} "
            f"(min {req['min']}, complex {complex_count}, min_complex {req['complex_min']})"
        )
        if len(loaded) < req["min"]:
            failures.append(f"{suite}: expected >={req['min']} cases, got {len(loaded)}")
        if complex_count < req["complex_min"]:
            failures.append(f"{suite}: expected >={req['complex_min']} complex, got {complex_count}")

    print(f"\nTotal cases (manifest): {total_cases}")
    print(f"Total yaml files: {total_yaml}")

    # Office bucket breakdown
    office_dir = ROOT / "cases" / "office"
    if office_dir.exists():
        buckets = {"text-md": 0, "docx": 0, "xlsx": 0, "pptx": 0, "cross-format": 0}
        for p in office_dir.glob("*.yaml"):
            name = p.stem
            for key in buckets:
                if key.replace("-", "_") in name:
                    buckets[key] += 1
        print("\nOffice buckets:", buckets)

    # Schema validation sample (all files)
    schema_errors = 0
    for suite_dir in (ROOT / "cases").glob("*"):
        if not suite_dir.is_dir():
            continue
        for ypath in suite_dir.glob("*.yaml"):
            if suite_dir.name == "security":
                errs = validate_security_yaml(ypath)
            elif suite_dir.name in SCENARIO_SCHEMA_SUITES:
                errs = validate_yaml(ypath)
            else:
                continue
            schema_errors += len(errs)
            failures.extend(errs[:3])  # cap per-file noise in summary

    print(f"\nSchema errors: {schema_errors}")

    # Fixtures
    print("\nFixtures:")
    for rel in FIXTURE_PATHS:
        p = ROOT / rel
        ok = p.exists()
        print(f"  [{'OK' if ok else 'FAIL'}] {rel}")
        if not ok:
            failures.append(f"missing fixture: {rel}")

    # Large repo line count
    handlers = ROOT / "fixtures/projects/large-repo/src/handlers.rs"
    if handlers.exists():
        lines = len(handlers.read_text(encoding="utf-8").splitlines())
        print(f"\nLarge-repo handlers.rs lines: {lines}")
        if lines < 4500:
            failures.append(f"large-repo handlers.rs has {lines} lines, expected ~5000")

    if failures:
        print(f"\n{len(failures)} failure(s):")
        for f in failures[:20]:
            print(f"  - {f}")
        if len(failures) > 20:
            print(f"  ... and {len(failures) - 20} more")
        return 1

    print("\nAll scenario-corpus checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
