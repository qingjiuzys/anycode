#!/usr/bin/env bash
# Install anyCode.app to /Applications (macOS).
# Usage:
#   ./scripts/install-desktop-macos.sh           # use existing bundle
#   ./scripts/install-desktop-macos.sh --build   # build DMG first (local fast profile)
#   ./scripts/install-desktop-macos.sh --open    # open DMG for drag-to-Applications
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="anyCode.app"
BUNDLE_APP="$ROOT/target/release/bundle/macos/$APP_NAME"
DMG_GLOB="$ROOT/target/release/bundle/dmg/anyCode_*_aarch64.dmg"
INSTALL_DIR="/Applications"

do_build=0
do_open=0
for arg in "$@"; do
  case "$arg" in
    --build) do_build=1 ;;
    --open) do_open=1 ;;
    -h|--help)
      sed -n '2,6p' "$0"
      exit 0
      ;;
    *)
      echo "unknown arg: $arg" >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS only" >&2
  exit 1
fi

if [[ "$do_build" == "1" ]]; then
  echo "==> building desktop bundle (local target + fast profile)..."
  ANYCODE_BUILD_TARGET=local ANYCODE_DESKTOP_LOCAL_RELEASE=1 "$ROOT/scripts/build-desktop-release.sh"
fi

if [[ ! -d "$BUNDLE_APP" ]]; then
  echo "missing $BUNDLE_APP — run with --build or ./scripts/build-desktop-release.sh" >&2
  exit 1
fi

if [[ "$do_open" == "1" ]]; then
  DMG="$(ls -t $DMG_GLOB 2>/dev/null | head -1 || true)"
  if [[ -z "$DMG" || ! -f "$DMG" ]]; then
    echo "DMG not found; run with --build first" >&2
    exit 1
  fi
  echo "==> opening installer: $DMG"
  open "$DMG"
  echo "Drag anyCode to Applications in the Finder window."
  exit 0
fi

echo "==> installing to $INSTALL_DIR/$APP_NAME"
if [[ -d "$INSTALL_DIR/$APP_NAME" ]]; then
  rm -rf "$INSTALL_DIR/$APP_NAME"
fi
ditto "$BUNDLE_APP" "$INSTALL_DIR/$APP_NAME"
xattr -cr "$INSTALL_DIR/$APP_NAME" 2>/dev/null || true
echo "==> done: $INSTALL_DIR/$APP_NAME"
echo "Launch from Launchpad or: open -a anyCode"
