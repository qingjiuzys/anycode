#!/usr/bin/env bash
# Build anycode-apple-media Swift CLI (macOS only).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/apps/anycode-desktop/native/anycode-apple-media"
OUT="$ROOT/apps/anycode-desktop/resources/bin/anycode-apple-media"
INSTALL_DIR="${HOME}/.anycode/bin"
INSTALL_OUT="${INSTALL_DIR}/anycode-apple-media"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "skip: anycode-apple-media requires macOS" >&2
  exit 0
fi

apple_media_needs_rebuild() {
  [[ "${ANYCODE_APPLE_MEDIA_FORCE:-}" == "1" ]] && return 0
  [[ ! -f "$OUT" ]] && return 0
  local out_mtime newest_src
  out_mtime="$(stat -f '%m' "$OUT")"
  newest_src="$(find "$PKG" \( -name '*.swift' -o -name Package.swift \) -exec stat -f '%m' {} \; 2>/dev/null | sort -n | tail -1)"
  [[ -z "$newest_src" || "$newest_src" -gt "$out_mtime" ]]
}

mkdir -p "$(dirname "$OUT")"

if ! apple_media_needs_rebuild; then
  echo "anycode-apple-media cache hit, skip swift build (set ANYCODE_APPLE_MEDIA_FORCE=1 to refresh)"
  exit 0
fi

cd "$PKG"
swift build -c release --product anycode-apple-media
BIN="$PKG/.build/release/anycode-apple-media"
cp "$BIN" "$OUT"
chmod +x "$OUT"
echo "Built $OUT"

mkdir -p "$INSTALL_DIR"
cp "$BIN" "$INSTALL_OUT"
chmod +x "$INSTALL_OUT"
echo "Installed $INSTALL_OUT (for CLI / WeChat bridge)"
