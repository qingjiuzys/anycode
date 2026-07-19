"""P0/P1 risk requirement coverage gate."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class Requirement:
    id: str
    risk: str
    title: str
    cases: list[str]
    owner: str = "platform"
    due_version: str | None = None
    reason: str | None = None


def load_catalog() -> list[Requirement]:
    path = ROOT / "requirements" / "catalog.toml"
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    out: list[Requirement] = []
    for entry in data.get("requirements", []):
        out.append(
            Requirement(
                id=entry["id"],
                risk=entry["risk"],
                title=entry["title"],
                cases=list(entry.get("cases", [])),
                owner=entry.get("owner", "platform"),
                due_version=entry.get("due_version"),
                reason=entry.get("reason"),
            )
        )
    return out


def evaluate_coverage(passed_case_ids: set[str], executed_case_ids: set[str]) -> dict:
    reqs = load_catalog()
    p0p1 = [r for r in reqs if r.risk in {"P0", "P1"}]
    covered = []
    uncovered = []
    out_of_scope = []
    for req in p0p1:
        linked = [c for c in req.cases if c in executed_case_ids]
        if not linked:
            out_of_scope.append(req)
            continue
        if any(c in passed_case_ids for c in linked):
            covered.append(req)
        else:
            uncovered.append(req)
    in_scope = [r for r in p0p1 if any(c in executed_case_ids for c in r.cases)]
    rate = (len(covered) / len(in_scope) * 100) if in_scope else 100.0
    return {
        "total_requirements": len(in_scope),
        "covered_requirements": len(covered),
        "coverage_rate": rate,
        "out_of_scope_count": len(out_of_scope),
        "uncovered": [
            {
                "id": r.id,
                "risk": r.risk,
                "title": r.title,
                "owner": r.owner,
                "due_version": r.due_version,
                "reason": r.reason,
                "cases": r.cases,
            }
            for r in uncovered
        ],
        "gate_pass": rate >= 95.0
        and all(
            r.risk != "P0" or any(c in passed_case_ids for c in r.cases)
            for r in in_scope
            if r.cases
        ),
    }


def main() -> int:
    """Validate catalog loads and report baseline coverage (no cases executed)."""
    reqs = load_catalog()
    p0p1 = [r for r in reqs if r.risk in {"P0", "P1"}]
    print(f"catalog: {len(reqs)} requirements ({len(p0p1)} P0/P1)")
    for req in p0p1:
        cases = ", ".join(req.cases) if req.cases else "(none)"
        print(f"  [{req.risk}] {req.id}: {req.title} -> {cases}")
    if len(p0p1) < 10:
        print("WARN: fewer than 10 P0/P1 requirements — expand catalog.toml")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
