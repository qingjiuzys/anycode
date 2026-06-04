#!/usr/bin/env bash
# Quick smoke for Tauri desktop packaging prerequisites (closure Wave 0).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== smoke: release anycode binary =="
cargo build --release -p anycode
test -x target/release/anycode
target/release/anycode --version >/dev/null

echo "== smoke: desktop app tree =="
test -f apps/anycode-desktop/tauri.conf.json
test -f apps/anycode-desktop/assets/anycode-logo.png
test -f apps/anycode-desktop/assets/anycode-logo-app-icon.png
test -f apps/anycode-desktop/icons/icon.icns
test -f scripts/build-desktop-release.sh

if command -v cargo-tauri >/dev/null 2>&1; then
  echo "== smoke: tauri info =="
  (cd apps/anycode-desktop && cargo tauri info >/dev/null)
else
  echo "skip: cargo-tauri not installed (install via 'cargo install tauri-cli' for full desktop build)"
fi

echo "desktop release smoke: ok"
