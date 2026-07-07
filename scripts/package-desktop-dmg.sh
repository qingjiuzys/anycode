#!/usr/bin/env bash
# Repackage anyCode.app into a DMG with Applications drag target (no Rust rebuild).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/target/release/bundle/macos/anyCode.app"
OUT_DIR="$ROOT/target/release/bundle/dmg"
VERSION="$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64) ARCH_TAG=aarch64 ;;
  x86_64) ARCH_TAG=x86_64 ;;
  *) ARCH_TAG="$ARCH" ;;
esac
DMG="$OUT_DIR/anyCode_${VERSION}_${ARCH_TAG}.dmg"
STAGE="$(mktemp -d)"

cleanup() { rm -rf "$STAGE"; }
trap cleanup EXIT

if [[ ! -d "$APP" ]]; then
  echo "missing $APP — run ./scripts/build-desktop-release.sh first" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
ditto "$APP" "$STAGE/anyCode.app"
ln -s /Applications "$STAGE/Applications"

TMP_DMG="$OUT_DIR/.anyCode-packaging.dmg"
rm -f "$TMP_DMG" "$DMG"
hdiutil create -volname "anyCode" -srcfolder "$STAGE" -ov -format UDRW -fs HFS+ "$TMP_DMG" >/dev/null
hdiutil convert "$TMP_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG" >/dev/null
rm -f "$TMP_DMG"

echo "DMG: $DMG"
echo "Open with: open \"$DMG\""
