#!/usr/bin/env bash
# Compile desktop shell only (release-local, no DMG/Tauri bundle). ~1–2min incremental.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${ANYCODE_DESKTOP_PROFILE:-release-local}"
export ANYCODE_DESKTOP_SKIP_UI_STAGE=1

"$ROOT/scripts/build-dashboard-ui.sh"

echo "==> cargo build --profile $PROFILE -p anycode-desktop"
cargo build --profile "$PROFILE" --manifest-path "$ROOT/apps/anycode-desktop/Cargo.toml"

echo "Binary: $ROOT/apps/anycode-desktop/target/$PROFILE/anycode-desktop"
echo "Install: ./scripts/sync-desktop-dev.sh --ui-only   # refresh UI in /Applications"
echo "         ./scripts/sync-desktop-dev.sh --rust      # UI + binary"
