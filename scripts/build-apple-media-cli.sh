#!/usr/bin/env bash
# Build anycode-apple-media Swift CLI (macOS only).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/apps/anycode-desktop/native/anycode-apple-media"
OUT="$ROOT/apps/anycode-desktop/resources/bin/anycode-apple-media"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "skip: anycode-apple-media requires macOS" >&2
  exit 0
fi

mkdir -p "$(dirname "$OUT")"
cd "$PKG"
swift build -c release --product anycode-apple-media
BIN="$PKG/.build/release/anycode-apple-media"
cp "$BIN" "$OUT"
chmod +x "$OUT"
echo "Built $OUT"
