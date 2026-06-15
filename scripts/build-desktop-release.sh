#!/usr/bin/env bash
# Build anyCode desktop installer (Tauri) + release CLI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export ROOT
cd "$ROOT"

BUNDLE_DIR="$ROOT/target/release/bundle"

step() {
  local label="$1"
  shift
  local start=$SECONDS
  echo "==> $label"
  "$@"
  local elapsed=$((SECONDS - start))
  echo "    (${elapsed}s)"
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

desktop_icon_fingerprint() {
  local logo_src="$ROOT/apps/anycode-desktop/assets/anycode-logo.png"
  local logo="$ROOT/apps/anycode-desktop/assets/anycode-logo-app-icon.png"
  printf 'src=%s\nicon=%s\n' "$(sha256_file "$logo_src")" "$(sha256_file "$logo")"
}

desktop_icon_cache_hit() {
  [[ "${ANYCODE_DESKTOP_ICON_FORCE:-}" == "1" ]] && return 1
  local fp="$ROOT/apps/anycode-desktop/icons/.icon-fingerprint"
  local icns="$ROOT/apps/anycode-desktop/icons/icon.icns"
  local logo="$ROOT/apps/anycode-desktop/assets/anycode-logo-app-icon.png"
  [[ -f "$fp" && -f "$icns" && -f "$logo" ]] || return 1
  [[ "$(desktop_icon_fingerprint)" == "$(cat "$fp")" ]]
}

write_desktop_icon_fingerprint() {
  desktop_icon_fingerprint >"$ROOT/apps/anycode-desktop/icons/.icon-fingerprint"
}

expected_stage_fingerprint() {
  local cli="$ROOT/target/release/anycode"
  [[ -f "$cli" ]] || cli="$ROOT/target/release/anycode.exe"
  printf 'cli=%s\nui=%s\n' "$(sha256_file "$cli")" "$(sha256_file "$ROOT/crates/dashboard-ui/dist/index.html")"
}

stage_cache_hit() {
  [[ "${ANYCODE_DESKTOP_STAGE_FORCE:-}" == "1" ]] && return 1
  local fp="$ROOT/apps/anycode-desktop/resources/.stage-fingerprint"
  local staged_cli="$ROOT/apps/anycode-desktop/resources/bin/anycode"
  [[ -f "$fp" && -f "$staged_cli" ]] || return 1
  [[ -f "$ROOT/apps/anycode-desktop/resources/dashboard-ui/index.html" ]] || return 1
  [[ "$(expected_stage_fingerprint)" == "$(cat "$fp")" ]]
}

write_stage_fingerprint() {
  expected_stage_fingerprint >"$ROOT/apps/anycode-desktop/resources/.stage-fingerprint"
}

TAURI_PROFILE="release"
if [[ "${ANYCODE_DESKTOP_LOCAL_RELEASE:-}" == "1" ]]; then
  TAURI_PROFILE="release-local"
fi

BUILD_START=$SECONDS
chmod +x "$ROOT/scripts/sync-workspace-version.sh"
chmod +x "$ROOT/scripts/build-apple-media-cli.sh"
chmod +x "$ROOT/scripts/prepare-browser-mcp.sh"
chmod +x "$ROOT/scripts/prepare-desktop-icon-env.sh"

step "sync workspace version to dashboard-ui / desktop manifests" \
  "$ROOT/scripts/sync-workspace-version.sh"

step "build dashboard UI (must run before CLI — embedded-ui bakes dist/)" \
  "$ROOT/scripts/build-dashboard-ui.sh"

CLI_FEATURES="embedded-ui,tools-mcp,knowledge-embeddings"
if [[ "$(uname -s)" == "Darwin" ]]; then
  CLI_FEATURES="${CLI_FEATURES},media-local"
fi

PARALLEL_START=$SECONDS
echo "==> cargo build anycode + parallel sidecar prep"
echo "    features: $CLI_FEATURES"
cargo build --release -p anycode --features "$CLI_FEATURES" &
CARGO_PID=$!
"$ROOT/scripts/build-apple-media-cli.sh" &
APPLE_PID=$!
"$ROOT/scripts/prepare-browser-mcp.sh" &
BROWSER_PID=$!
CARGO_STATUS=0
APPLE_STATUS=0
BROWSER_STATUS=0
wait "$CARGO_PID" || CARGO_STATUS=$?
wait "$APPLE_PID" || APPLE_STATUS=$?
wait "$BROWSER_PID" || BROWSER_STATUS=$?
if [[ "$CARGO_STATUS" -ne 0 || "$APPLE_STATUS" -ne 0 || "$BROWSER_STATUS" -ne 0 ]]; then
  echo "parallel build failed (cargo=$CARGO_STATUS apple=$APPLE_STATUS browser=$BROWSER_STATUS)" >&2
  exit 1
fi
echo "    ($((SECONDS - PARALLEL_START))s)"

if stage_cache_hit; then
  echo "==> stage resources cache hit, skip copy (set ANYCODE_DESKTOP_STAGE_FORCE=1 to refresh)"
  echo "    (0s)"
else
  STAGE_START=$SECONDS
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
write_stage_fingerprint
echo "    ($((SECONDS - STAGE_START))s)"
fi

if [[ "$TAURI_PROFILE" == "release-local" ]]; then
  echo "==> using release-local profile for desktop (faster local DMG; unset ANYCODE_DESKTOP_LOCAL_RELEASE for shipping LTO build)"
fi

if desktop_icon_cache_hit; then
  echo "==> desktop icons cache hit, skip icon prep (set ANYCODE_DESKTOP_ICON_FORCE=1 to refresh)"
  echo "    (0s)"
else
  ICON_START=$SECONDS
  echo "==> prepare desktop app icon (crop + scale)"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    # shellcheck source=scripts/prepare-desktop-icon-env.sh
    source "$ROOT/scripts/prepare-desktop-icon-env.sh"
    "$ICON_PY" "$ROOT/scripts/prepare-desktop-icon.py"
  fi

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
  write_desktop_icon_fingerprint
  echo "    ($((SECONDS - ICON_START))s)"
fi

step "cargo tauri build (apps/anycode-desktop, profile=$TAURI_PROFILE)" bash -ec '
  cd "$ROOT/apps/anycode-desktop"
  export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
  if [[ -z "${APPLE_ID:-}" || -z "${APPLE_PASSWORD:-}" || -z "${APPLE_TEAM_ID:-}" ]]; then
    unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
  fi
  cargo tauri build -- --profile "'"$TAURI_PROFILE"'"
'

TOTAL=$((SECONDS - BUILD_START))
echo "Done in ${TOTAL}s. Bundles under ${BUNDLE_DIR}/"
echo "  DMG: ${BUNDLE_DIR}/dmg/anyCode_*_aarch64.dmg"
echo "  App: ${BUNDLE_DIR}/macos/anyCode.app"
