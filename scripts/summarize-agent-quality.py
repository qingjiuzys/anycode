#!/usr/bin/env python3
"""Summarize agent-quality 2×2 ablation results (task-clustered bootstrap + Holm).

Expects JSONL or JSON array of rows:
  {task_id, arm, quality, final_success, cost, latency_ms, environment_failure?}

Arms: baseline / experience_only / skill_only / experience_skill
"""
from __future__ import annotations

import argparse
import json
import random
from collections import defaultdict
from pathlib import Path


ARMS = ("baseline", "experience_only", "skill_only", "experience_skill")
PRIMARY = (
    "Q11-Q10_skill_vs_experience_only",
    "Q11-Q01_experience_vs_skill_only",
)


def load_rows(path: Path) -> list[dict]:
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".jsonl":
        return [json.loads(line) for line in text.splitlines() if line.strip()]
    data = json.loads(text)
    if isinstance(data, dict) and "rows" in data:
        return list(data["rows"])
    return list(data)


def mean(xs: list[float]) -> float:
    return sum(xs) / len(xs) if xs else float("nan")


def task_arm_means(rows: list[dict]) -> dict[str, dict[str, float]]:
    buckets: dict[str, dict[str, list[float]]] = defaultdict(lambda: defaultdict(list))
    for r in rows:
        if r.get("environment_failure"):
            continue
        buckets[r["task_id"]][r["arm"]].append(float(r["quality"]))
    return {
        tid: {arm: mean(vals) for arm, vals in arms.items()}
        for tid, arms in buckets.items()
    }


def contrast(task_means: dict[str, dict[str, float]], a: str, b: str) -> list[float]:
    out = []
    for means in task_means.values():
        if a in means and b in means:
            out.append(means[a] - means[b])
    return out


def bootstrap_ci(deltas: list[float], n: int = 10_000, seed: int = 0) -> tuple[float, float, float]:
    if not deltas:
        return float("nan"), float("nan"), float("nan")
    rng = random.Random(seed)
    samples = []
    for _ in range(n):
        draw = [deltas[rng.randrange(len(deltas))] for _ in deltas]
        samples.append(mean(draw))
    samples.sort()
    lo = samples[int(0.025 * (n - 1))]
    hi = samples[int(0.975 * (n - 1))]
    return mean(deltas), lo, hi


def holm_reject_null(ci_lower_gt_0: list[bool], alpha: float = 0.05) -> list[bool]:
    """Approximate Holm: rank by 'hardest' (False first), require sequential pass.

    For bootstrap CI tests we only have pass/fail of lo>0; treat failures as
    p≈alpha and successes as p≈0, then apply Holm ordering.
    """
    indexed = sorted(enumerate(ci_lower_gt_0), key=lambda x: (x[1], x[0]))
    # Failures first; once a failure appears after adjusted threshold, stop.
    out = [False] * len(ci_lower_gt_0)
    m = len(ci_lower_gt_0)
    for rank, (idx, ok) in enumerate(indexed):
        # effective alpha = alpha / (m - rank)
        # If ok (CI>0), treat as rejected null → True
        if ok:
            out[idx] = True
        else:
            # remaining including this cannot reject
            for _, (j, _) in enumerate(indexed[rank:], start=rank):
                out[j] = False
            break
    # If all ok, all True
    if all(ci_lower_gt_0):
        return [True] * m
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("results", type=Path)
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument(
        "--promotion-thresholds",
        type=Path,
        default=Path("test/benchmarks/agent-quality/manifest.json"),
    )
    args = ap.parse_args()
    rows = load_rows(args.results)
    means = task_arm_means(rows)
    contrasts = {
        "Q11-Q10_skill_vs_experience_only": contrast(means, "experience_skill", "experience_only"),
        "Q11-Q01_experience_vs_skill_only": contrast(means, "experience_skill", "skill_only"),
        "Q10-Q00_experience_main": contrast(means, "experience_only", "baseline"),
        "Q01-Q00_skill_main": contrast(means, "skill_only", "baseline"),
    }
    report: dict = {"tasks": len(means), "contrasts": {}}
    primary_flags = []
    for name, deltas in contrasts.items():
        est, lo, hi = bootstrap_ci(deltas)
        flag = lo > 0 if lo == lo else False
        report["contrasts"][name] = {
            "n_tasks": len(deltas),
            "estimate": est,
            "ci95": [lo, hi],
            "ci_lower_gt_0": flag,
        }
        if name in PRIMARY:
            primary_flags.append(flag)

    inter = []
    for m in means.values():
        if all(a in m for a in ARMS):
            inter.append(
                (m["experience_skill"] - m["skill_only"])
                - (m["experience_only"] - m["baseline"])
            )
    est, lo, hi = bootstrap_ci(inter)
    report["contrasts"]["interaction"] = {
        "n_tasks": len(inter),
        "estimate": est,
        "ci95": [lo, hi],
    }

    holm = holm_reject_null(primary_flags)
    report["holm_primary"] = {
        PRIMARY[i]: holm[i] if i < len(holm) else False for i in range(len(PRIMARY))
    }

    thresholds = {}
    if args.promotion_thresholds.exists():
        thresholds = json.loads(args.promotion_thresholds.read_text(encoding="utf-8")).get(
            "scoring", {}
        ).get("promotion", {})
    vs_exp = thresholds.get("vs_experience_only_min_delta", 8)
    vs_sk = thresholds.get("vs_skill_only_min_delta", 3)

    c_skill = report["contrasts"]["Q11-Q10_skill_vs_experience_only"]
    c_exp = report["contrasts"]["Q11-Q01_experience_vs_skill_only"]
    skill_ok = (
        c_skill["estimate"] >= vs_exp
        and c_skill["ci_lower_gt_0"]
        and report["holm_primary"].get(PRIMARY[0], False)
    )
    exp_ok = (
        c_exp["estimate"] >= vs_sk
        and c_exp["ci_lower_gt_0"]
        and report["holm_primary"].get(PRIMARY[1], False)
    )
    report["promotion_thresholds"] = {
        "vs_experience_only_min_delta": vs_exp,
        "vs_skill_only_min_delta": vs_sk,
    }
    report["recommendation"] = (
        "experience_skill"
        if skill_ok and exp_ok
        else "skill_only"
        if skill_ok and not exp_ok
        else "inconclusive"
    )
    report["lab_note"] = (
        "Synthetic/dev rows are for harness validation only — not hidden promotion evidence."
    )

    text = json.dumps(report, indent=2)
    print(text)
    if args.out:
        args.out.write_text(text + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
