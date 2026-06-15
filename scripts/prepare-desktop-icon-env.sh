#!/usr/bin/env bash
# Ensure Python venv with Pillow for prepare-desktop-icon.py (macOS only).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    exit 0
  fi
  return 0 2>/dev/null || exit 0
fi

ICON_VENV="$ROOT/scripts/.venv-icon"
if [[ -x "$ICON_VENV/bin/python" ]]; then
  ICON_PY="$ICON_VENV/bin/python"
elif [[ -x "$ICON_VENV/Scripts/python.exe" ]]; then
  ICON_PY="$ICON_VENV/Scripts/python.exe"
else
  python3 -m venv "$ICON_VENV"
  if [[ -x "$ICON_VENV/bin/python" ]]; then
    ICON_PY="$ICON_VENV/bin/python"
  else
    ICON_PY="$ICON_VENV/Scripts/python.exe"
  fi
  "$ICON_PY" -m pip install -q pillow
fi

export ICON_PY
