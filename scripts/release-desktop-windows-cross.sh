#!/usr/bin/env bash
# Cross-compile Windows NSIS installer from macOS/Linux via cargo-xwin, then stage.
#
# Prereqs (once): ./scripts/setup-windows-cross.sh
#
# Notes:
#   - Produces NSIS .exe only (MSI/WiX still requires a Windows host).
#   - Experimental per Tauri docs; prefer a Windows host when available.
#   - Omits local ONNX/knowledge features (same as Mac Intel cross).
#   - No Authenticode signing in this path.
#
# Usage:
#   ./scripts/release-desktop-windows-cross.sh
#   ./scripts/release-desktop-windows-cross.sh --skip-build   # stage existing NSIS only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SKIP_BUILD=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    -h|--help)
      sed -n '2,18p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

# Force Windows target (do not inherit a leftover Mac ANYCODE_TAURI_TARGET).
export ANYCODE_TAURI_TARGET="x86_64-pc-windows-msvc"
export ANYCODE_DESKTOP_SKIP_BROWSER=1
export XWIN_CACHE_DIR="${XWIN_CACHE_DIR:-$HOME/.cache/cargo-xwin}"
# Avoid leftover Apple signing env steering tauri into a macOS bundle path.
unset APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID || true

if [[ -d /opt/homebrew/opt/llvm/bin ]]; then
  export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
elif [[ -d /usr/local/opt/llvm/bin ]]; then
  export PATH="/usr/local/opt/llvm/bin:$PATH"
fi

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  if ! command -v cargo-xwin >/dev/null 2>&1 || ! command -v makensis >/dev/null 2>&1; then
    echo "Missing tooling. Run: ./scripts/setup-windows-cross.sh" >&2
    exit 1
  fi
  chmod +x "$ROOT/scripts/build-desktop-release.sh"
  "$ROOT/scripts/build-desktop-release.sh"
fi

chmod +x "$ROOT/scripts/stage-desktop-windows.sh"
"$ROOT/scripts/stage-desktop-windows.sh"

echo
echo "Staged under crates/account-portal/public/downloads/"
echo "Deploy: TAG=\$(grep '^version' Cargo.toml | head -1 | sed 's/.*\"\\(.*\\)\".*/\\1/') ./scripts/build-account-image.sh --skip-dmg"
