#!/usr/bin/env python3
"""Run EvalPlus codegen + evaluate for Agnes (OpenAI-compatible backend)."""
from __future__ import annotations

import argparse
import json
import os
import sys
from collections.abc import Iterable
from pathlib import Path
from typing import Any

HTTP_ERROR_MARKERS = (
    "http error",
    "http status",
    "status code",
    "error code",
    "rate limit",
    "bad gateway",
    "service unavailable",
)


def sample_path(root: Path, dataset: str, model: str) -> Path:
    identifier = model.strip("./").replace("/", "--")
    return root / "evalplus_results" / dataset / f"{identifier}_openai_temp_0.0.jsonl"


def classify_record(record: Any) -> str | None:
    if not isinstance(record, dict) or not record.get("task_id"):
        return "malformed"
    solution = record.get("solution")
    if not isinstance(solution, str) or not solution.strip():
        return "empty"
    low = solution.lower()
    if any(marker in low for marker in HTTP_ERROR_MARKERS):
        return "http_error"
    return None


def clean_resume_samples(path: Path) -> dict[str, int]:
    stats = {"valid": 0, "empty": 0, "http_error": 0, "malformed": 0}
    if not path.is_file():
        return stats
    valid: list[dict[str, Any]] = []
    invalid: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            record = {"raw": line}
        reason = classify_record(record)
        if reason is None:
            valid.append(record)
            stats["valid"] += 1
        else:
            stats[reason] += 1
            invalid.append({"line": line_number, "failure_reason": reason, "record": record})
    if invalid:
        archive = path.with_suffix(".invalid.jsonl")
        with archive.open("a", encoding="utf-8") as f:
            for record in invalid:
                f.write(json.dumps(record, ensure_ascii=False) + "\n")
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text("".join(json.dumps(record, ensure_ascii=False) + "\n" for record in valid), encoding="utf-8")
    tmp.replace(path)
    return stats


def update_failure_stats(path: Path, delta: dict[str, int]) -> dict[str, int]:
    totals = {"empty": 0, "http_error": 0, "malformed": 0}
    if path.is_file():
        try:
            saved = json.loads(path.read_text(encoding="utf-8"))
            for key in totals:
                totals[key] = int(saved.get(key, 0))
        except (json.JSONDecodeError, OSError, TypeError, ValueError):
            pass
    for key in totals:
        totals[key] += int(delta.get(key, 0))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(totals, indent=2) + "\n", encoding="utf-8")
    return totals


def expected_task_count(dataset: str, id_range: Iterable[int] | None, mini: bool) -> int | None:
    if mini:
        return None
    total = 164 if dataset == "humaneval" else 378
    if id_range is None:
        return total
    low, high = id_range
    return max(0, min(total, high) - max(0, low))


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--dataset", choices=["humaneval", "mbpp"], required=True)
    p.add_argument("--model", default="agnes-2.0-flash")
    p.add_argument("--base-url", default="https://apihub.agnes-ai.com/v1")
    p.add_argument("--root", type=Path, required=True)
    p.add_argument("--id-range", nargs=2, type=int, metavar=("LOW", "HIGH"))
    p.add_argument("--mini", action="store_true")
    return p.parse_args()


def main() -> int:
    args = parse_args()
    if not os.environ.get("OPENAI_API_KEY"):
        print("OPENAI_API_KEY not set", file=sys.stderr)
        return 1
    os.environ.setdefault("EVALPLUS_MAX_MEMORY_BYTES", "-1")

    from evalplus.codegen import run_codegen
    from evalplus.evaluate import evaluate

    args.root.mkdir(parents=True, exist_ok=True)
    os.chdir(args.root)
    target_samples = sample_path(args.root, args.dataset, args.model)
    failure_stats_path = target_samples.with_suffix(".generation-stats.json")
    resume_stats = clean_resume_samples(target_samples)
    failure_stats = update_failure_stats(failure_stats_path, resume_stats)
    if sum(resume_stats.values()):
        print(f"[resume] {json.dumps(resume_stats, sort_keys=True)}")

    model_kwargs = {
        "model": args.model,
        "backend": "openai",
        "base_url": args.base_url,
        "dataset": args.dataset,
        "greedy": True,
        "root": str(args.root / "evalplus_results"),
        "resume": True,
        "jsonl_fmt": True,
    }
    if args.id_range:
        model_kwargs["id_range"] = list(args.id_range)
    if args.mini:
        model_kwargs["mini"] = True

    print(f"[codegen] dataset={args.dataset} model={args.model}")
    try:
        samples = run_codegen(**model_kwargs)
    except Exception as exc:
        low = str(exc).lower()
        reason = "http_error" if any(marker in low for marker in HTTP_ERROR_MARKERS) else "malformed"
        failure_stats = update_failure_stats(failure_stats_path, {reason: 1})
        failure_summary = {
            "dataset": args.dataset,
            "model": args.model,
            "base_url": args.base_url,
            "samples": str(target_samples),
            "eval_results": None,
            "expected_task_count": expected_task_count(args.dataset, args.id_range, args.mini),
            "valid_generation_count": resume_stats["valid"],
            "empty_sample_count": failure_stats["empty"],
            "http_error_count": failure_stats["http_error"],
            "invalid_sample_count": sum(failure_stats.values()),
            "generation_error": str(exc),
        }
        (args.root / f"{args.dataset}-summary.json").write_text(
            json.dumps(failure_summary, indent=2) + "\n",
            encoding="utf-8",
        )
        raise
    print(f"[codegen] samples={samples}")
    final_stats = clean_resume_samples(Path(samples))
    failure_stats = update_failure_stats(failure_stats_path, final_stats)
    valid_generation_count = final_stats["valid"]

    eval_kwargs = {
        "dataset": args.dataset,
        "samples": samples,
        "parallel": max(1, (os.cpu_count() or 4) // 2),
        "i_just_wanna_run": True,
    }
    if args.mini:
        eval_kwargs["mini"] = True

    result_path = Path(str(samples).replace(".jsonl", "_eval_results.json"))
    if result_path.is_file():
        print(f"[evaluate] reusing existing results={result_path}")
    else:
        print(f"[evaluate] dataset={args.dataset}")
        evaluate(**eval_kwargs)
    if not result_path.is_file():
        # evalplus may write next to jsonl
        alt = args.root / f"{args.dataset}_eval_results.json"
        result_path = alt if alt.is_file() else result_path

    summary = {
        "dataset": args.dataset,
        "model": args.model,
        "base_url": args.base_url,
        "samples": str(samples),
        "eval_results": str(result_path) if result_path.is_file() else None,
        "expected_task_count": expected_task_count(args.dataset, args.id_range, args.mini),
        "valid_generation_count": valid_generation_count,
        "empty_sample_count": failure_stats["empty"],
        "http_error_count": failure_stats["http_error"],
        "invalid_sample_count": sum(failure_stats.values()),
    }
    if result_path.is_file():
        data = json.loads(result_path.read_text())
        summary["task_count"] = len(data.get("eval", {}))
        # Recompute pass@1 from eval block
        from evalplus.eval import PASS, estimate_pass_at_k
        import numpy as np

        total = np.array([len(r) for r in data["eval"].values()])
        base_correct = np.array(
            [sum(1 for r in res if r.get("base_status") == PASS) for res in data["eval"].values()]
        )
        plus_correct = np.array(
            [
                sum(1 for r in res if r.get("base_status") == PASS and r.get("plus_status") == PASS)
                for res in data["eval"].values()
            ]
        )
        if len(total) and total.min() >= 1:
            summary["model_pass_at_1_base"] = float(estimate_pass_at_k(total, base_correct, 1).mean())
            summary["model_pass_at_1_plus"] = float(estimate_pass_at_k(total, plus_correct, 1).mean())
        summary["date"] = data.get("date")
        summary["hash"] = data.get("hash")

    out_json = args.root / f"{args.dataset}-summary.json"
    out_json.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
