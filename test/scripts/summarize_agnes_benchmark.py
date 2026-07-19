#!/usr/bin/env python3
"""Aggregate Agnes EvalPlus summaries into benchmark-scores.md."""
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--model", default="agnes-2.0-flash")
    return p.parse_args()


def main() -> int:
    args = parse_args()
    rows = []
    for name, title, tasks in [
        ("humaneval", "HumanEval+ (EvalPlus)", 164),
        ("mbpp", "MBPP+ (EvalPlus)", 378),
    ]:
        p = args.out / name / f"{name}-summary.json"
        if not p.is_file():
            rows.append((title, name, tasks, "—", "—", "—", "—", "—", "not run"))
            continue
        s = json.loads(p.read_text())
        base = s.get("model_pass_at_1_base")
        plus = s.get("model_pass_at_1_plus")
        n = s.get("task_count", "—")
        valid = s.get("valid_generation_count", "—")
        empty = s.get("empty_sample_count", "—")
        http_errors = s.get("http_error_count", "—")
        status = (
            "ok"
            if base is not None and valid == s.get("expected_task_count")
            else "incomplete"
        )
        rows.append(
            (
                title,
                name,
                tasks,
                f"{base * 100:.1f}%" if base is not None else "—",
                f"{plus * 100:.1f}%" if plus is not None else "—",
                str(n),
                str(valid),
                f"empty={empty}, http={http_errors}",
                status,
            )
        )

    md = args.out / "benchmark-scores.md"
    lines = [
        f"# Agnes 行业 Benchmark 实测 — {args.model}",
        "",
        f"生成时间：{datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}",
        "",
        "| 数据集 | Adapter | 题量 | model_pass_at_1 (base) | model_pass_at_1 (+) | 评测题数 | 有效生成 | 失败统计 | 状态 |",
        "|--------|---------|------|------------------------|---------------------|----------|----------|----------|------|",
    ]
    for row in rows:
        lines.append(
            f"| {row[0]} | `{row[1]}` | {row[2]} | {row[3]} | {row[4]} | "
            f"{row[5]} | {row[6]} | {row[7]} | {row[8]} |"
        )
    lines += [
        "",
        "## 说明",
        "",
        "- **base**：原始 HumanEval/MBPP 测试集通过率",
        "- **(+)**：EvalPlus 增强测试集通过率（更严格）",
        "- 评测工具：EvalPlus v0.3.1，`--backend openai`，greedy decoding",
        f"- 产物目录：`{args.out}`",
    ]
    md.write_text("\n".join(lines) + "\n")
    print(md)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
