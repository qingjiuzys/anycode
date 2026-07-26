#!/usr/bin/env python3
"""DEPRECATED: use fill_potx.py for commercial editable PPTX. This wrapper delegates."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: html_slides_to_pptx.py slides_dir [output.pptx] [brand_kit]", file=sys.stderr)
        print("NOTE: delegates to fill_potx.py (native editable export)", file=sys.stderr)
        return 1
    filler = REPO / "scripts" / "office" / "fill_potx.py"
    proc = subprocess.run([sys.executable, str(filler), *sys.argv[1:]], capture_output=True, text=True)
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr or proc.stdout)
        return proc.returncode
    print(proc.stdout.strip())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
