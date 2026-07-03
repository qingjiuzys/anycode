#!/usr/bin/env bash
# Stage Chromium for native CDP browser (reuses prepare-browser-mcp download).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE="$ROOT/apps/anycode-desktop/resources/browser"
CHROMIUM_PATH_FILE="$STAGE/.chromium-path"
FINGERPRINT="$STAGE/.chromium-fingerprint"

find_chromium_binary() {
  local browsers="$STAGE/browsers"
  [[ -d "$browsers" ]] || return 1
  if [[ "$(uname -s)" == "Darwin" ]]; then
    local app
    app="$(find "$browsers" -name 'Chromium.app' -print -quit 2>/dev/null || true)"
    if [[ -n "$app" && -f "$app/Contents/MacOS/Chromium" ]]; then
      echo "$app/Contents/MacOS/Chromium"
      return 0
    fi
  fi
  local bin
  bin="$(find "$browsers" \( -name Chromium -o -name chrome -o -name 'Google Chrome for Testing' \) -type f -print -quit 2>/dev/null || true)"
  [[ -n "$bin" ]] && echo "$bin"
}

sha256_file() {
  local f="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$f" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    echo "sha256 unavailable" >&2
    exit 1
  fi
}

echo "==> prepare native Chromium (CDP)"
chmod +x "$ROOT/scripts/prepare-browser-mcp.sh"
"$ROOT/scripts/prepare-browser-mcp.sh"

CHROMIUM="$(find_chromium_binary || true)"
if [[ -z "$CHROMIUM" || ! -f "$CHROMIUM" ]]; then
  echo "Chromium binary not found under $STAGE/browsers" >&2
  exit 1
fi

printf '%s\n' "$CHROMIUM" >"$CHROMIUM_PATH_FILE"
printf 'exec=%s\n' "$(sha256_file "$CHROMIUM")" >"$FINGERPRINT"
echo "    Chromium: $CHROMIUM"
