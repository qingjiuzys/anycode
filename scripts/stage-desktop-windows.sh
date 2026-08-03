#!/usr/bin/env bash
# Stage Windows MSI/NSIS installers into account-portal/public/downloads.
# Run on Windows after ./scripts/build-desktop-release.sh (Git Bash / MSYS).
#
# Usage:
#   ./scripts/stage-desktop-windows.sh
#   ./scripts/stage-desktop-windows.sh /path/to/anyCode_0.40.0_x64_en-US.msi
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f&&/^version/{gsub(/.*version = "/,""); gsub(/".*/,""); print; exit}' Cargo.toml)"
DOWNLOAD_DIR="${ANYCODE_DOWNLOAD_DIR:-$ROOT/crates/account-portal/public/downloads}"
mkdir -p "$DOWNLOAD_DIR"

pick_artifact() {
  local kind="$1"
  local found=""
  if [[ $# -ge 2 && -n "${2:-}" && -f "$2" ]]; then
    echo "$2"
    return 0
  fi
  case "$kind" in
    msi)
      found="$(ls -1 "$ROOT"/target/release/bundle/msi/*.msi 2>/dev/null | head -1 || true)"
      ;;
    nsis)
      found="$(ls -1 "$ROOT"/target/release/bundle/nsis/*.exe 2>/dev/null | head -1 || true)"
      ;;
  esac
  if [[ -n "$found" && -f "$found" ]]; then
    echo "$found"
    return 0
  fi
  return 1
}

STAGED_ANY=0

if MSI="$(pick_artifact msi "${1:-}")"; then
  DEST="$DOWNLOAD_DIR/anyCode_${VERSION}_x64.msi"
  cp -f "$MSI" "$DEST"
  cp -f "$MSI" "$DOWNLOAD_DIR/anyCode_latest_x64.msi"
  echo "Staged MSI: $DEST"
  STAGED_ANY=1
fi

if EXE="$(pick_artifact nsis "${2:-}")"; then
  DEST="$DOWNLOAD_DIR/anyCode_${VERSION}_x64.exe"
  cp -f "$EXE" "$DEST"
  cp -f "$EXE" "$DOWNLOAD_DIR/anyCode_latest_x64.exe"
  echo "Staged NSIS: $DEST"
  STAGED_ANY=1
fi

if [[ "$STAGED_ANY" -eq 0 ]]; then
  echo "No Windows MSI/NSIS found under target/release/bundle/{msi,nsis}." >&2
  echo "Build on Windows first: ./scripts/build-desktop-release.sh" >&2
  exit 1
fi

python3 "$ROOT/scripts/lib/regen-downloads-manifest.py" "$DOWNLOAD_DIR"
echo "Updated $DOWNLOAD_DIR/releases.json"
echo "Sync this downloads/ tree back to the Mac host before build-account-image.sh if needed."
