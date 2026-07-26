"""Load TOML manifests and case definitions."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None  # type: ignore[assignment]


ROOT = Path(__file__).resolve().parents[1]


@dataclass
class CaseSpec:
    id: str
    suite: str
    tier: list[str]
    risk: str
    runner: str
    command: str | None = None
    requires: dict[str, Any] = field(default_factory=dict)
    models: list[str] = field(default_factory=list)
    timeout_seconds: int = 300
    path: Path | None = None
    meta: dict[str, Any] = field(default_factory=dict)


@dataclass
class ProfileSpec:
    id: str
    max_duration_minutes: int
    tier: list[str]
    suites: list[str]
    cases: list[CaseSpec]


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as f:
        return tomllib.load(f)


def load_profile(profile_id: str) -> ProfileSpec:
    path = ROOT / "manifests" / f"{profile_id}.toml"
    data = load_toml(path)
    profile = data["profile"]
    cases: list[CaseSpec] = []
    for entry in profile.get("cases", []):
        cases.append(_case_from_dict(entry))
    for suite in profile.get("suites", []):
        cases.extend(_load_suite_cases(suite, profile.get("tier", [profile_id])))
    return ProfileSpec(
        id=profile_id,
        max_duration_minutes=int(profile.get("max_duration_minutes", 60)),
        tier=list(profile.get("tier", [profile_id])),
        suites=list(profile.get("suites", [])),
        cases=cases,
    )


def _case_from_dict(entry: dict[str, Any]) -> CaseSpec:
    return CaseSpec(
        id=entry["id"],
        suite=entry.get("suite", "misc"),
        tier=list(entry.get("tier", [])),
        risk=entry.get("risk", "P2"),
        runner=entry.get("runner", "cargo"),
        command=entry.get("command"),
        requires=dict(entry.get("requires", {})),
        models=list(entry.get("models", [])),
        timeout_seconds=int(entry.get("timeout_seconds", 300)),
        meta=dict(entry.get("meta", {})),
    )


def _load_yaml_case(path: Path) -> dict[str, Any]:
    if yaml is None:
        raise RuntimeError("PyYAML required to load case definitions: pip install pyyaml")
    with path.open(encoding="utf-8") as f:
        data = yaml.safe_load(f)
    if not isinstance(data, dict):
        raise ValueError(f"case yaml must be a mapping: {path}")
    return data


def _merge_case_yaml(case: CaseSpec, yaml_data: dict[str, Any]) -> None:
    """Merge yaml fields into CaseSpec; yaml wins for overlapping meta keys."""
    if yaml_data.get("tier"):
        case.tier = list(yaml_data["tier"])
    if yaml_data.get("risk"):
        case.risk = str(yaml_data["risk"])
    if yaml_data.get("runner"):
        case.runner = str(yaml_data["runner"])
    if yaml_data.get("timeout_seconds") is not None:
        case.timeout_seconds = int(yaml_data["timeout_seconds"])
    if yaml_data.get("requires"):
        case.requires = dict(yaml_data["requires"])
    if yaml_data.get("models"):
        case.models = list(yaml_data["models"])
    meta = dict(case.meta)
    for key in ("prompts", "prompt", "fixture", "expected", "complexity", "meta"):
        if key in yaml_data and yaml_data[key] is not None:
            meta[key] = yaml_data[key]
    if "prompts" in meta and "prompt" not in meta:
        prompts = meta["prompts"]
        if isinstance(prompts, list) and prompts:
            meta["prompt"] = prompts[0] if isinstance(prompts[0], str) else prompts[0].get("content", "")
    nested = yaml_data.get("meta")
    if isinstance(nested, dict):
        meta.update(nested)
    case.meta = meta


def _load_suite_cases(suite: str, tier: list[str]) -> list[CaseSpec]:
    suite_dir = ROOT / "cases" / suite
    index = suite_dir / "index.toml"
    if not index.exists():
        return []
    data = load_toml(index)
    out: list[CaseSpec] = []
    for entry in data.get("cases", []):
        case = _case_from_dict(entry)
        if not case.tier:
            case.tier = tier
        case.suite = suite
        yaml_path = suite_dir / f"{case.id}.yaml"
        case.path = yaml_path if yaml_path.exists() else None
        if case.path is not None:
            _merge_case_yaml(case, _load_yaml_case(yaml_path))
        out.append(case)
    return out


def load_all_suite_cases() -> dict[str, list[CaseSpec]]:
    cases_root = ROOT / "cases"
    if not cases_root.exists():
        return {}
    suites: dict[str, list[CaseSpec]] = {}
    for suite_dir in sorted(cases_root.iterdir()):
        if suite_dir.is_dir() and (suite_dir / "index.toml").exists():
            suites[suite_dir.name] = _load_suite_cases(suite_dir.name, ["all"])
    return suites


def filter_cases(
    cases: list[CaseSpec],
    profile_id: str,
    models: list[str] | None,
    profile_tiers: list[str] | None = None,
    *,
    offline: bool = False,
) -> list[CaseSpec]:
    selected_tiers = set(profile_tiers or [profile_id])
    filtered = [
        c
        for c in cases
        if not c.tier or bool(selected_tiers.intersection(c.tier)) or "all" in c.tier
    ]
    if offline:
        # Offline mode is the default for all test runs: skip anything that
        # needs a live LLM or external network (ANYCODE_OFFLINE=0 to opt out).
        filtered = [
            c
            for c in filtered
            if c.requires.get("llm") != "live" and not c.requires.get("network")
        ]
    if models:
        model_set = {m.strip() for m in models}
        filtered = [
            c
            for c in filtered
            if not c.models or any(m in model_set for m in c.models) or c.runner != "dashboard"
        ]
    return filtered
