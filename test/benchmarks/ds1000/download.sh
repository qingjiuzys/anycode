#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$BENCH_ROOT/data/ds-1000"
CACHE="$BENCH_ROOT/data/cache"
ARCHIVE="$CACHE/ds1000-c4e8f12.tar.gz"
PIN="c4e8f12"
SHA256="b2c3d4e5f6789012345678abcdef9012345678abcdef9012345678abcdef1234"
URL="https://github.com/xlang-ai/DS-1000/archive/refs/heads/main.tar.gz"

mkdir -p "$CACHE" "$DATA_DIR"
if [[ -f "$DATA_DIR/.pin_ok" ]] && [[ "$(cat "$DATA_DIR/.pin_ok")" == "$PIN" ]]; then
  echo "[ds1000] dataset already present ($PIN)"
  exit 0
fi
if [[ ! -f "$ARCHIVE" ]]; then
  echo "[ds1000] downloading $URL"
  curl -fsSL "$URL" -o "$ARCHIVE"
fi
# shellcheck source=../_common/verify_hash.sh
source "$BENCH_ROOT/_common/verify_hash.sh"
anycode_verify_sha256 "$SHA256" "$ARCHIVE"
rm -rf "$DATA_DIR"/*
tar -xzf "$ARCHIVE" -C "$DATA_DIR" --strip-components=1
echo "$PIN" > "$DATA_DIR/.pin_ok"
echo "[ds1000] ready at $DATA_DIR"
