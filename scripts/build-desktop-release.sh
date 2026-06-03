#!/usr/bin/env bash
# Build anyCode desktop installer (Tauri) + release CLI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> cargo build --release -p anycode"
cargo build --release -p anycode

echo "==> stage bundled CLI for Tauri resources"
DESKTOP_BIN="$ROOT/apps/anycode-desktop/resources/bin"
mkdir -p "$DESKTOP_BIN"
cp "$ROOT/target/release/anycode" "$DESKTOP_BIN/anycode"
chmod +x "$DESKTOP_BIN/anycode"

echo "==> build dashboard UI"
"$ROOT/scripts/build-dashboard-ui.sh"

echo "==> generate desktop icons from assets/anycode-logo.png"
LOGO="$ROOT/apps/anycode-desktop/assets/anycode-logo.png"
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
cargo tauri build

echo "Done. Bundles under apps/anycode-desktop/target/release/bundle/"
echo "  DMG: apps/anycode-desktop/target/release/bundle/dmg/anyCode_*_aarch64.dmg"
echo "  App: apps/anycode-desktop/target/release/bundle/macos/anyCode.app"
