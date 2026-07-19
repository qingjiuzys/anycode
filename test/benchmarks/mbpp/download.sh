#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$BENCH_ROOT/data/evalplus-mbpp"
CACHE="$BENCH_ROOT/data/cache"
ARCHIVE="$CACHE/mbpp-v0.3.1.tar.gz"
PIN="v0.3.1"
SHA256="490f08728db1da8a73df89ada48f8ad0270fbd667895838348e64506095a2d49"
URL="https://github.com/evalplus/evalplus/archive/refs/tags/v0.3.1.tar.gz"

mkdir -p "$CACHE" "$DATA_DIR"
if [[ -f "$DATA_DIR/.pin_ok" ]] && [[ "$(cat "$DATA_DIR/.pin_ok")" == "$PIN" ]]; then
  echo "[mbpp] dataset already present ($PIN)"
  exit 0
fi
if [[ ! -f "$ARCHIVE" ]]; then
  echo "[mbpp] downloading $URL"
  curl -fsSL "$URL" -o "$ARCHIVE"
fi
# shellcheck source=../_common/verify_hash.sh
source "$BENCH_ROOT/_common/verify_hash.sh"
anycode_verify_sha256 "$SHA256" "$ARCHIVE"
rm -rf "$DATA_DIR"/*
tar -xzf "$ARCHIVE" -C "$DATA_DIR" --strip-components=1
echo "$PIN" > "$DATA_DIR/.pin_ok"
echo "[mbpp] ready at $DATA_DIR"
