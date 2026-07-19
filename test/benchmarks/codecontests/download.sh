#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$BENCH_ROOT/data/codecontests-stratified"
CACHE="$BENCH_ROOT/data/cache"
ARCHIVE="$CACHE/codecontests-f1a9d3e.tar.gz"
PIN="f1a9d3e"
SHA256="e5f6789012345678abcdef9012345678abcdef9012345678abcdef1234567890"
URL="https://github.com/google-deepmind/code_contests/archive/refs/heads/main.tar.gz"

mkdir -p "$CACHE" "$DATA_DIR"
if [[ -f "$DATA_DIR/.pin_ok" ]] && [[ "$(cat "$DATA_DIR/.pin_ok")" == "$PIN" ]]; then
  echo "[codecontests] dataset already present ($PIN)"
  exit 0
fi
if [[ ! -f "$ARCHIVE" ]]; then
  echo "[codecontests] downloading $URL"
  curl -fsSL "$URL" -o "$ARCHIVE"
fi
# shellcheck source=../_common/verify_hash.sh
source "$BENCH_ROOT/_common/verify_hash.sh"
anycode_verify_sha256 "$SHA256" "$ARCHIVE"
rm -rf "$DATA_DIR"/*
tar -xzf "$ARCHIVE" -C "$DATA_DIR" --strip-components=1
echo "$PIN" > "$DATA_DIR/.pin_ok"
echo "[codecontests] ready at $DATA_DIR"
