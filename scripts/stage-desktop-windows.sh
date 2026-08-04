#!/usr/bin/env bash
# Stage Windows MSI/NSIS installers into account-portal/public/downloads.
#
# Sources (first hit wins):
#   - explicit path args
#   - native Windows build: target/release/bundle/{msi,nsis}/
#   - Mac/Linux cross: target/x86_64-pc-windows-msvc/release/bundle/nsis/
#
# Usage:
#   ./scripts/stage-desktop-windows.sh
#   ./scripts/stage-desktop-windows.sh /path/to/anyCode_0.40.0_x64_en-US.msi
#   ./scripts/stage-desktop-windows.sh '' /path/to/anyCode_0.40.0_x64-setup.exe
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f&&/^version/{gsub(/.*version = "/,""); gsub(/".*/,""); print; exit}' Cargo.toml)"
DOWNLOAD_DIR="${ANYCODE_DOWNLOAD_DIR:-$ROOT/crates/account-portal/public/downloads}"
WIN_TARGET="${ANYCODE_TAURI_TARGET:-x86_64-pc-windows-msvc}"
mkdir -p "$DOWNLOAD_DIR"

pick_first() {
  local f
  for f in "$@"; do
    if [[ -n "$f" && -f "$f" ]]; then
      echo "$f"
      return 0
    fi
  done
  return 1
}

pick_artifact() {
  local kind="$1"
  local found=""
  if [[ $# -ge 2 && -n "${2:-}" && -f "$2" ]]; then
    echo "$2"
    return 0
  fi
  case "$kind" in
    msi)
      found="$(pick_first \
        "$(ls -1 "$ROOT"/target/release/bundle/msi/*.msi 2>/dev/null | head -1 || true)" \
        "$(ls -1 "$ROOT"/target/"$WIN_TARGET"/release/bundle/msi/*.msi 2>/dev/null | head -1 || true)" \
        || true)"
      ;;
    nsis)
      found="$(pick_first \
        "$(ls -1 "$ROOT"/target/release/bundle/nsis/*.exe 2>/dev/null | head -1 || true)" \
        "$(ls -1 "$ROOT"/target/"$WIN_TARGET"/release/bundle/nsis/*.exe 2>/dev/null | head -1 || true)" \
        || true)"
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
  echo "No Windows MSI/NSIS found under:" >&2
  echo "  target/release/bundle/{msi,nsis}" >&2
  echo "  target/${WIN_TARGET}/release/bundle/nsis" >&2
  echo "Build: ./scripts/release-desktop-windows-cross.sh  (Mac/Linux NSIS)" >&2
  echo "   or: ./scripts/build-desktop-release.sh           (on Windows)" >&2
  exit 1
fi

python3 "$ROOT/scripts/lib/regen-downloads-manifest.py" "$DOWNLOAD_DIR"
echo "Updated $DOWNLOAD_DIR/releases.json"
echo "Sync this downloads/ tree back to the Mac host before build-account-image.sh if needed."
