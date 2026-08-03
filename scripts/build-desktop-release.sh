#!/usr/bin/env bash
# Build anyCode desktop installer (Tauri) + embedded dashboard.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export ROOT
cd "$ROOT"

# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
anycode_apply_build_target_exports
anycode_print_build_target_summary

if [[ "$(uname -s)" == "Darwin" ]]; then
  export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-10.15}"
  export CMAKE_OSX_DEPLOYMENT_TARGET="${CMAKE_OSX_DEPLOYMENT_TARGET:-10.15}"
fi

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
  printf 'ui=%s\ntarget=%s\n' \
    "$(sha256_file "$ROOT/crates/dashboard-ui/dist/index.html")" \
    "${ANYCODE_BUILD_TARGET:-cloud}"
}

stage_cache_hit() {
  [[ "${ANYCODE_DESKTOP_STAGE_FORCE:-}" == "1" ]] && return 1
  local fp="$ROOT/apps/anycode-desktop/resources/.stage-fingerprint"
  [[ -f "$fp" ]] || return 1
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

# --bundles is platform-specific (tauri rejects bundle types for other OSes).
BUNDLES="deb,appimage"
case "$(uname -s)" in
  # Prefer app-only on macOS; DMG is produced by package-desktop-dmg.sh after notarization.
  # Tauri's bundled bundle_dmg.sh is flaky under cross / target-triple layouts.
  Darwin)  BUNDLES="app" ;;
  MINGW*|MSYS*|CYGWIN*) BUNDLES="msi,nsis" ;;
esac

BUILD_START=$SECONDS
chmod +x "$ROOT/scripts/sync-workspace-version.sh"
chmod +x "$ROOT/scripts/build-apple-media-cli.sh"
chmod +x "$ROOT/scripts/prepare-browser-mcp.sh"
chmod +x "$ROOT/scripts/prepare-chromium.sh"
chmod +x "$ROOT/scripts/prepare-desktop-icon-env.sh"
chmod +x "$ROOT/scripts/codesign-mac-app-deep.sh"
chmod +x "$ROOT/scripts/notarize-mac-app.sh"

REL_ENV="${ANYCODE_RELEASE_ENV:-$HOME/.anycode/release.env}"
SIGNING_RELEASE=0
if [[ -f "$REL_ENV" ]]; then
  # shellcheck source=/dev/null
  source "$REL_ENV"
fi
if [[ "$(uname -s)" == "Darwin" && -n "${APPLE_SIGNING_IDENTITY:-}" && "${APPLE_SIGNING_IDENTITY}" != "-" && -n "${APPLE_ID:-}" ]]; then
  SIGNING_RELEASE=1
fi
SKIP_BROWSER=0
if [[ "${ANYCODE_DESKTOP_SKIP_BROWSER:-}" == "1" || "$SIGNING_RELEASE" -eq 1 ]]; then
  SKIP_BROWSER=1
fi
if [[ "$SKIP_BROWSER" -eq 1 ]]; then
  echo "==> skip bundled Chromium (notarized release; browser installs on first use)"
  # tauri.conf.json declares resources/browser/ as a bundled resource; tauri
  # validates the path exists at build time even when we skip the download.
  mkdir -p "$ROOT/apps/anycode-desktop/resources/browser"
fi

step "sync workspace version to dashboard-ui / desktop manifests" \
  "$ROOT/scripts/sync-workspace-version.sh"

step "build dashboard UI (must run before desktop — embedded-ui bakes dist/)" \
  "$ROOT/scripts/build-dashboard-ui.sh"

DASHBOARD_FEATURES="embedded-ui,tools-browser"
HOST_IS_DARWIN=0
[[ "$(uname -s)" == "Darwin" ]] && HOST_IS_DARWIN=1
CROSS_MAC=0
if [[ "$HOST_IS_DARWIN" -eq 1 && -n "${ANYCODE_TAURI_TARGET:-}" ]]; then
  case "$(uname -m)-${ANYCODE_TAURI_TARGET}" in
    arm64-x86_64-apple-darwin|x86_64-aarch64-apple-darwin) CROSS_MAC=1 ;;
  esac
fi
if [[ "$HOST_IS_DARWIN" -eq 1 && "$CROSS_MAC" -eq 0 ]]; then
  # macOS TTS is provided by anycode-apple-media. Avoid bundling Piper's
  # espeak compiler/data while retaining local embeddings and STT.
  DASHBOARD_FEATURES="${DASHBOARD_FEATURES},knowledge-embeddings,embedding-local,stt-local"
elif [[ "$HOST_IS_DARWIN" -eq 1 && "$CROSS_MAC" -eq 1 ]]; then
  echo "==> cross-mac build: omit knowledge-embeddings/local-ml (ort-sys host-only prebuilts)"
else
  DASHBOARD_FEATURES="${DASHBOARD_FEATURES},knowledge-embeddings"
fi

PARALLEL_START=$SECONDS
echo "==> cargo build dashboard + parallel sidecar prep"
echo "    features: $DASHBOARD_FEATURES"
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode-dashboard --features "$DASHBOARD_FEATURES" &
CARGO_PID=$!
"$ROOT/scripts/build-apple-media-cli.sh" &
APPLE_PID=$!
BROWSER_PID=""
if [[ "$SKIP_BROWSER" -eq 0 ]]; then
  "$ROOT/scripts/prepare-chromium.sh" &
  BROWSER_PID=$!
fi
CARGO_STATUS=0
APPLE_STATUS=0
BROWSER_STATUS=0
wait "$CARGO_PID" || CARGO_STATUS=$?
wait "$APPLE_PID" || APPLE_STATUS=$?
if [[ -n "$BROWSER_PID" ]]; then
  wait "$BROWSER_PID" || BROWSER_STATUS=$?
fi
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
  echo "==> stage project templates + dashboard UI for Tauri resources"
  DESKTOP_TPL="$ROOT/apps/anycode-desktop/resources/project-templates"
  rm -rf "$DESKTOP_TPL"
  cp -R "$ROOT/project-templates" "$DESKTOP_TPL"
  DESKTOP_STARTER="$ROOT/apps/anycode-desktop/resources/skills-starter"
  rm -rf "$DESKTOP_STARTER"
  cp -R "$ROOT/skills-starter" "$DESKTOP_STARTER"
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

anycode_write_account_endpoints_manifest \
  "$ROOT/apps/anycode-desktop/resources/account-endpoints.json"

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

TAURI_TARGET="${ANYCODE_TAURI_TARGET:-}"
if [[ -n "$TAURI_TARGET" ]]; then
  echo "==> tauri target override: $TAURI_TARGET"
fi

CARGO_EXTRA_ARGS=()
if [[ "${CROSS_MAC:-0}" -eq 1 ]]; then
  CARGO_EXTRA_ARGS+=(--no-default-features)
fi

step "cargo tauri build (apps/anycode-desktop, profile=$TAURI_PROFILE)" bash -c '
  cd "$1"
  export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    echo "Signing in Tauri; notarization deferred until after bundle"
    unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
  elif [[ -z "${APPLE_ID:-}" || -z "${APPLE_PASSWORD:-}" || -z "${APPLE_TEAM_ID:-}" ]]; then
    unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
  fi
  if [[ "$2" -eq 1 ]]; then
    rm -rf "$3/apps/anycode-desktop/resources/browser/browsers"
  fi
  TARGET_FLAG=()
  if [[ -n "${6:-}" ]]; then
    TARGET_FLAG=(--target "$6")
  fi
  EXTRA=()
  if [[ "${7:-}" == "1" ]]; then
    EXTRA=(--no-default-features)
  fi
  exec cargo tauri build --bundles "$4" "${TARGET_FLAG[@]}" -- --profile "$5" "${EXTRA[@]}"
' _ "$ROOT/apps/anycode-desktop" "$SKIP_BROWSER" "$ROOT" "$BUNDLES" "$TAURI_PROFILE" "$TAURI_TARGET" "$CROSS_MAC"

if [[ -n "$TAURI_TARGET" ]]; then
  DESKTOP_APP_BUNDLE="$ROOT/target/${TAURI_TARGET}/${TAURI_PROFILE}/bundle/macos/anyCode.app"
  if [[ ! -d "$DESKTOP_APP_BUNDLE" ]]; then
    DESKTOP_APP_BUNDLE="$ROOT/target/${TAURI_TARGET}/release/bundle/macos/anyCode.app"
  fi
  SIGN_APP_BUNDLE="$DESKTOP_APP_BUNDLE"
  BUNDLE_DIR="$ROOT/target/${TAURI_TARGET}/release/bundle"
else
  DESKTOP_APP_BUNDLE="$ROOT/target/${TAURI_PROFILE}/bundle/macos/anyCode.app"
  if [[ ! -d "$DESKTOP_APP_BUNDLE" ]]; then
    DESKTOP_APP_BUNDLE="$ROOT/target/release/bundle/macos/anyCode.app"
  fi
  if [[ -d "$DESKTOP_APP_BUNDLE" && "$DESKTOP_APP_BUNDLE" != "$ROOT/target/release/bundle/macos/anyCode.app" ]]; then
    step "sync desktop .app into target/release/bundle for signing/DMG" bash -ec "
      rm -rf '$ROOT/target/release/bundle/macos/anyCode.app'
      mkdir -p '$ROOT/target/release/bundle/macos'
      ditto '$DESKTOP_APP_BUNDLE' '$ROOT/target/release/bundle/macos/anyCode.app'
    "
  fi
  SIGN_APP_BUNDLE="$ROOT/target/release/bundle/macos/anyCode.app"
fi

if [[ "$(uname -s)" == "Darwin" && -n "${APPLE_SIGNING_IDENTITY:-}" && "${APPLE_SIGNING_IDENTITY}" != "-" ]]; then
  APP_BUNDLE="$SIGN_APP_BUNDLE"
  REL_ENV="${ANYCODE_RELEASE_ENV:-$HOME/.anycode/release.env}"
  if [[ -f "$REL_ENV" ]]; then
    # shellcheck source=/dev/null
    source "$REL_ENV"
  fi
  if [[ -d "$APP_BUNDLE" && -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    step "deep-sign bundled binaries (resources/bin, native modules)" "$ROOT/scripts/codesign-mac-app-deep.sh" "$APP_BUNDLE"
    step "notarize + staple" "$ROOT/scripts/notarize-mac-app.sh" "$APP_BUNDLE"
  fi
fi

if [[ "$(uname -s)" == "Darwin" ]]; then
  step "repackage DMG with Applications drag target" "$ROOT/scripts/package-desktop-dmg.sh"
fi

TOTAL=$((SECONDS - BUILD_START))
echo "Done in ${TOTAL}s. Bundles under ${BUNDLE_DIR}/"
echo "  DMG: ${BUNDLE_DIR}/dmg/anyCode_*.dmg"
echo "  App: ${SIGN_APP_BUNDLE:-${BUNDLE_DIR}/macos/anyCode.app}"
