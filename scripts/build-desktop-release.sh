#!/usr/bin/env bash
# Build anyCode desktop installer (Tauri) + release CLI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> sync workspace version to dashboard-ui / desktop manifests"
chmod +x "$ROOT/scripts/sync-workspace-version.sh"
"$ROOT/scripts/sync-workspace-version.sh"

echo "==> build dashboard UI (must run before CLI — embedded-ui bakes dist/)"
"$ROOT/scripts/build-dashboard-ui.sh"

echo "==> cargo build --release -p anycode (embedded-ui + tools-mcp + media-local on macOS)"
CLI_FEATURES="embedded-ui,tools-mcp,knowledge-embeddings"
if [[ "$(uname -s)" == "Darwin" ]]; then
  CLI_FEATURES="${CLI_FEATURES},media-local"
fi
cargo build --release -p anycode --features "$CLI_FEATURES"

echo "==> build Apple native media helper (macOS STT/OCR)"
chmod +x "$ROOT/scripts/build-apple-media-cli.sh"
"$ROOT/scripts/build-apple-media-cli.sh"

echo "==> prepare bundled browser MCP (Playwright + Chromium)"
chmod +x "$ROOT/scripts/prepare-browser-mcp.sh"
"$ROOT/scripts/prepare-browser-mcp.sh"

echo "==> stage bundled CLI + project templates for Tauri resources"
DESKTOP_BIN="$ROOT/apps/anycode-desktop/resources/bin"
DESKTOP_TPL="$ROOT/apps/anycode-desktop/resources/project-templates"
mkdir -p "$DESKTOP_BIN"
if [[ -f "$ROOT/target/release/anycode.exe" ]]; then
  cp "$ROOT/target/release/anycode.exe" "$DESKTOP_BIN/anycode.exe"
  chmod +x "$DESKTOP_BIN/anycode.exe"
else
  cp "$ROOT/target/release/anycode" "$DESKTOP_BIN/anycode"
  chmod +x "$DESKTOP_BIN/anycode"
fi
rm -rf "$DESKTOP_TPL"
cp -R "$ROOT/project-templates" "$DESKTOP_TPL"
DESKTOP_UI="$ROOT/apps/anycode-desktop/resources/dashboard-ui"
rm -rf "$DESKTOP_UI"
cp -R "$ROOT/crates/dashboard-ui/dist" "$DESKTOP_UI"
test -f "$DESKTOP_UI/index.html" || {
  echo "missing dashboard-ui dist for desktop bundle" >&2
  exit 1
}

echo "==> prepare desktop app icon (crop + scale)"
ICON_VENV="$ROOT/scripts/.venv-icon"
if [[ "$(uname -s)" == "Darwin" ]]; then
  if [[ -x "$ICON_VENV/bin/python" ]]; then
    ICON_PY="$ICON_VENV/bin/python"
  elif [[ -x "$ICON_VENV/Scripts/python.exe" ]]; then
    ICON_PY="$ICON_VENV/Scripts/python.exe"
  else
    python3 -m venv "$ICON_VENV"
    if [[ -x "$ICON_VENV/bin/python" ]]; then
      ICON_PY="$ICON_VENV/bin/python"
    else
      ICON_PY="$ICON_VENV/Scripts/python.exe"
    fi
    "$ICON_PY" -m pip install -q pillow
  fi
  "$ICON_PY" "$ROOT/scripts/prepare-desktop-icon.py"
fi

echo "==> generate desktop icons from assets/anycode-logo-app-icon.png"
LOGO="$ROOT/apps/anycode-desktop/assets/anycode-logo-app-icon.png"
if [[ ! -f "$LOGO" ]]; then
  echo "missing desktop logo: $LOGO" >&2
  exit 1
fi
if ! command -v cargo-tauri >/dev/null 2>&1; then
  echo "installing cargo-tauri CLI..."
  cargo install tauri-cli --version "^2" --locked
fi
(cd "$ROOT/apps/anycode-desktop" && cargo tauri icon "$LOGO")

echo "==> cargo tauri build (apps/anycode-desktop)"
cd "$ROOT/apps/anycode-desktop"
# Ad-hoc sign when no Developer ID is configured (CI / local unsigned builds).
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
cargo tauri build

echo "Done. Bundles under apps/anycode-desktop/target/release/bundle/"
echo "  DMG: apps/anycode-desktop/target/release/bundle/dmg/anyCode_*_aarch64.dmg"
echo "  App: apps/anycode-desktop/target/release/bundle/macos/anyCode.app"
