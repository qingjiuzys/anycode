#!/usr/bin/env python3
"""Summarize three-arm baseline metrics (low / Codex / enhanced) from JSON rows.

Input JSON: list of EvalArmMetrics-like objects or { "rows": [...] }.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCENARIOS = ROOT / "test" / "benchmarks" / "experience-baseline" / "scenarios.json"


def load_rows(path: Path) -> list[dict]:
    data = json.loads(path.read_text())
    if isinstance(data, dict) and "rows" in data:
        return data["rows"]
    if isinstance(data, list):
        return data
    raise SystemExit("expected list or {rows: [...]}")


def summarize(rows: list[dict]) -> dict:
    by_arm: dict[str, list[bool]] = defaultdict(list)
    for row in rows:
        by_arm[row.get("arm", "")].append(bool(row.get("passed")))
    rates = {
        arm: (sum(1 for p in vals if p) / len(vals) if vals else 0.0)
        for arm, vals in by_arm.items()
    }
    low = rates.get("low_model", 0.0)
    codex = rates.get("codex_reference", 0.0)
    enhanced = rates.get("low_model_enhanced", 0.0)
    return {
        "per_arm_pass_rate": rates,
        "enhanced_vs_low_delta": enhanced - low,
        "enhanced_vs_codex_delta": enhanced - codex,
        "meets_promotion_gate": (enhanced - low) >= 0.2 and (enhanced - codex) >= -0.15,
        "scenario_corpus": str(SCENARIOS.relative_to(ROOT)),
        "corpus_size": len(json.loads(SCENARIOS.read_text())),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("metrics_json", type=Path, nargs="?", help="EvalArmMetrics JSON")
    ap.add_argument("--print-corpus", action="store_true")
    args = ap.parse_args()
    if args.print_corpus:
        print(SCENARIOS.read_text())
        return 0
    if not args.metrics_json:
        ap.error("metrics_json required unless --print-corpus")
    summary = summarize(load_rows(args.metrics_json))
    print(json.dumps(summary, indent=2))
    return 0 if summary["meets_promotion_gate"] else 2


if __name__ == "__main__":
    sys.exit(main())
