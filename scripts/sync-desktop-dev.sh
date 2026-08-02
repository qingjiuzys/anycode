#!/usr/bin/env bash
# Fast desktop dev loop — default UI-only (~15s). Avoid full LTO release (~5min).
#
# Usage:
#   ./scripts/sync-desktop-dev.sh              # UI sync → /Applications/anyCode.app
#   ./scripts/sync-desktop-dev.sh --rust     # UI + incremental Rust (release-local ~1–2min)
#   ./scripts/sync-desktop-dev.sh --ui-only   # never compile Rust
#
# Env:
#   ANYCODE_DESKTOP_PROFILE=release-local   # default; use "release" for shipping binary
#   ANYCODE_DESKTOP_INSTALL=/path/anyCode.app
#   ANYCODE_DESKTOP_OPEN=0                  # skip relaunch
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${ANYCODE_DESKTOP_PROFILE:-release-local}"
INSTALL="${ANYCODE_DESKTOP_INSTALL:-/Applications/anyCode.app}"
MANIFEST="$ROOT/apps/anycode-desktop/Cargo.toml"
DESKTOP_TARGET="$ROOT/apps/anycode-desktop/target"
BIN="$DESKTOP_TARGET/$PROFILE/anycode-desktop"
UI_SRC="$ROOT/crates/dashboard-ui/dist"
UI_DEST="$INSTALL/Contents/Resources/resources/dashboard-ui"

FORCE_RUST=0
UI_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --rust) FORCE_RUST=1 ;;
    --ui-only) UI_ONLY=1 ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *)
      echo "unknown arg: $arg (try --rust or --ui-only)" >&2
      exit 1
      ;;
  esac
done

step() {
  local label="$1"
  shift
  local start=$SECONDS
  echo "==> $label"
  "$@"
  echo "    ($((SECONDS - start))s)"
}

rust_sources_newer_than_binary() {
  [[ -x "$BIN" ]] || return 0
  local marker
  marker="$(find \
    "$ROOT/apps/anycode-desktop/src" \
    "$ROOT/apps/anycode-desktop/build.rs" \
    "$ROOT/crates/dashboard/src" \
    "$ROOT/crates/dashboard/migrations" \
    "$ROOT/crates/dashboard/build.rs" \
    "$ROOT/crates/setup/src" \
    -type f -newer "$BIN" 2>/dev/null | head -1 || true)"
  [[ -n "$marker" ]]
}

BUILD_START=$SECONDS

step "dashboard UI (cached vite build)" "$ROOT/scripts/build-dashboard-ui.sh"

if [[ ! -d "$INSTALL/Contents/MacOS" ]]; then
  echo "missing $INSTALL — build once: ./scripts/build-desktop-local.sh or --rust after a full bundle exists" >&2
  exit 1
fi

step "sync UI into app bundle" bash -ec "
  mkdir -p \"$(dirname "$UI_DEST")\"
  rsync -a --delete \"$UI_SRC/\" \"$UI_DEST/\"
"

# macOS 26 Tahoe wraps legacy .icns in a light squircle ("icon jail"). Syncing the
# icns + applying a Finder custom icon escapes that container without Assets.car.
ICNS_SRC="$ROOT/apps/anycode-desktop/icons/icon.icns"
ICNS_DEST="$INSTALL/Contents/Resources/icon.icns"
if [[ -f "$ICNS_SRC" ]]; then
  step "sync Dock icon (Tahoe)" bash -ec "
    cp -f \"$ICNS_SRC\" \"$ICNS_DEST\"
    if command -v swift >/dev/null 2>&1; then
      swift -e '
        import AppKit
        let app = \"$INSTALL\"
        let icns = \"$ICNS_DEST\"
        guard let img = NSImage(contentsOfFile: icns) else { fatalError(\"icns\") }
        _ = NSWorkspace.shared.setIcon(img, forFile: app, options: [])
      '
    fi
  "
fi

NEED_RUST=0
if [[ "$FORCE_RUST" == "1" ]]; then
  NEED_RUST=1
elif [[ "$UI_ONLY" == "1" ]]; then
  NEED_RUST=0
elif [[ ! -x "$BIN" ]]; then
  NEED_RUST=1
elif rust_sources_newer_than_binary; then
  NEED_RUST=1
fi

if [[ "$NEED_RUST" == "1" ]]; then
  step "cargo build desktop (profile=$PROFILE, skip UI stage)" bash -ec "
    export ANYCODE_DESKTOP_SKIP_UI_STAGE=1
    touch \"$ROOT/apps/anycode-desktop/src/main.rs\"
    cargo build --profile \"$PROFILE\" --manifest-path \"$MANIFEST\"
  "
  step "install binary" bash -ec "
    cp \"$BIN\" \"$INSTALL/Contents/MacOS/anycode-desktop\"
    if ! strings \"$BIN\" | rg -q 'skill name zh'; then
      echo 'ERROR: desktop binary missing migration 14 (skill name zh) — rebuild failed' >&2
      exit 1
    fi
  "
else
  echo "==> skip Rust build (UI-only; use --rust if you changed backend code)"
  echo "    (0s)"
fi

if [[ "${ANYCODE_DESKTOP_OPEN:-1}" != "0" ]]; then
  killall anycode-desktop 2>/dev/null || true
  sleep 0.5
  open -a "$INSTALL"
fi

echo "Done in $((SECONDS - BUILD_START))s."
echo "  App:  $INSTALL"
echo "  UI:   $UI_DEST"
if [[ -x "$INSTALL/Contents/MacOS/anycode-desktop" ]]; then
  echo "  Bin:  $INSTALL/Contents/MacOS/anycode-desktop"
fi
