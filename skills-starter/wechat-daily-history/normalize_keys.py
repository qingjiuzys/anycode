#!/usr/bin/env python3
"""Normalize wechat-db-decrypt keys JSON for anycode sqlcipher_key_map backend."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HEX_KEY = re.compile(r"^[0-9a-fA-F]{32,192}$")
SKIP_KEYS = {"__salts__", "__metadata__", "__version__"}


def normalize(raw: dict) -> dict[str, str]:
    out: dict[str, str] = {}
    for k, v in raw.items():
        if not isinstance(k, str) or k in SKIP_KEYS or k.startswith("__"):
            continue
        if not isinstance(v, str):
            continue
        key = v.strip()
        if key.startswith("x'") and key.endswith("'"):
            key = key[2:-1]
        if not HEX_KEY.match(key):
            continue
        rel = k.replace("\\", "/").lstrip("/")
        if not rel.endswith(".db"):
            continue
        out[rel] = key
    return out


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: normalize_keys.py <input.json> <output.json>", file=sys.stderr)
        return 2
    src = Path(sys.argv[1])
    dst = Path(sys.argv[2])
    if not src.is_file():
        print(f"input not found: {src}", file=sys.stderr)
        return 1
    raw = json.loads(src.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        print("input must be a JSON object", file=sys.stderr)
        return 1
    out = normalize(raw)
    if not out:
        print("no database keys after normalization", file=sys.stderr)
        return 1
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    try:
        dst.chmod(0o600)
    except OSError:
        pass
    print(json.dumps({"ok": True, "count": len(out), "path": str(dst)}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
