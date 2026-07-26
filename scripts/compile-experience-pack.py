#!/usr/bin/env python3
"""Compile an offline experience pack from teacher trajectories (JSONL).

Does NOT call cloud APIs; expects validated trajectories produced elsewhere.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def distill(traj: dict) -> dict:
    return {
        "id": f"distill.{traj.get('id', 'unknown')}",
        "title": f"Distilled: {traj.get('id', 'unknown')}",
        "family": traj.get("family", "general"),
        "applicable_when": [str(traj.get("prompt", ""))[:80]],
        "task_breakdown": traj.get("notes", []),
        "tool_order": traj.get("tool_order", []),
        "key_checks": [n for n in traj.get("notes", []) if "check" in n.lower()],
        "common_failures": [],
        "recovery": ["replay failed gate with evidence"],
        "examples": [traj.get("prompt", "")],
        "model_compat": ["weak_local"],
        "regression_score": float(traj.get("low_model_replay_gain", 0)),
        "version": "0.1.0",
    }


def sign(payload: bytes, secret: str) -> str:
    h = hashlib.sha256()
    h.update(secret.encode())
    h.update(payload)
    return h.hexdigest()[:16]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--trajectories", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--id", default="lab-pack")
    ap.add_argument("--version", default="0.1.0")
    ap.add_argument("--min-gain", type=float, default=0.1)
    ap.add_argument("--secret", default="")
    args = ap.parse_args()

    cards = []
    for line in args.trajectories.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        traj = json.loads(line)
        if not traj.get("passed_gates"):
            continue
        if float(traj.get("low_model_replay_gain", 0)) < args.min_gain:
            continue
        cards.append(distill(traj))

    pack = {
        "meta": {
            "id": args.id,
            "version": args.version,
            "model_compat": ["weak_local", "*"],
            "regression_score": (
                sum(c["regression_score"] for c in cards) / len(cards) if cards else 0.0
            ),
            "created_at": datetime.now(timezone.utc).isoformat(),
            "signature_hex": "",
            "signer": "",
        },
        "cards": cards,
    }
    body = json.dumps(
        {"id": pack["meta"]["id"], "version": pack["meta"]["version"], "cards": cards},
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode()
    if args.secret:
        pack["meta"]["signature_hex"] = sign(body, args.secret)
        pack["meta"]["signer"] = "compile-experience-pack.py"

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(pack, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {args.out} cards={len(cards)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
