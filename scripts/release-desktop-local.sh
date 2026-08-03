#!/usr/bin/env bash
# Build a signed + notarized macOS DMG locally and stage for anycode.work download.
#
# Prereqs:
#   - Developer ID Application cert in login keychain
#   - ~/.anycode/release.env (see scripts/release.env.example)
#
# Usage:
#   ./scripts/release-desktop-local.sh                 # host arch (usually aarch64)
#   ./scripts/release-desktop-local.sh --arch aarch64
#   ./scripts/release-desktop-local.sh --arch x86_64   # cross from Apple Silicon
#   ./scripts/release-desktop-local.sh --upload        # also run ANYCODE_DOWNLOAD_UPLOAD
#   ./scripts/release-desktop-local.sh --skip-build    # stage existing DMG only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

UPLOAD=0
SKIP_BUILD=0
ARCH_TAG=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --upload) UPLOAD=1; shift ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --arch)
      ARCH_TAG="${2:-}"
      shift 2
      ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS only (signed DMG + notarization)." >&2
  exit 1
fi

HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  arm64) HOST_TAG=aarch64 ;;
  x86_64) HOST_TAG=x86_64 ;;
  *) HOST_TAG="$HOST_ARCH" ;;
esac

if [[ -z "$ARCH_TAG" ]]; then
  ARCH_TAG="$HOST_TAG"
fi
case "$ARCH_TAG" in
  aarch64|arm64) ARCH_TAG=aarch64; RUST_TARGET=aarch64-apple-darwin ;;
  x86_64|amd64) ARCH_TAG=x86_64; RUST_TARGET=x86_64-apple-darwin ;;
  *)
    echo "Unsupported --arch $ARCH_TAG (use aarch64 or x86_64)" >&2
    exit 1
    ;;
esac

RELEASE_ENV="${ANYCODE_RELEASE_ENV:-$HOME/.anycode/release.env}"
if [[ -f "$RELEASE_ENV" ]]; then
  # shellcheck source=/dev/null
  source "$RELEASE_ENV"
  echo "Loaded $RELEASE_ENV"
else
  echo "Missing $RELEASE_ENV — copy scripts/release.env.example and fill Apple credentials." >&2
  exit 1
fi

if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  APPLE_SIGNING_IDENTITY="$(security find-identity -v -p codesigning | awk -F'"' '/Developer ID Application/ { print $2; exit }')"
  export APPLE_SIGNING_IDENTITY
fi

for var in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_TEAM_ID; do
  if [[ -z "${!var:-}" ]]; then
    echo "Set $var in $RELEASE_ENV" >&2
    exit 1
  fi
done

if [[ -z "${APPLE_PASSWORD:-}" && -n "${APP_SPECIFIC_PASSWORD:-}" ]]; then
  export APPLE_PASSWORD="$APP_SPECIFIC_PASSWORD"
fi
if [[ -z "${APPLE_PASSWORD:-}" ]]; then
  echo "Set APPLE_PASSWORD (or APP_SPECIFIC_PASSWORD) in $RELEASE_ENV" >&2
  exit 1
fi

echo "Signing identity: $APPLE_SIGNING_IDENTITY"
echo "Team ID: $APPLE_TEAM_ID"
echo "Arch: $ARCH_TAG (rust target $RUST_TARGET)"

VERSION="$(awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f&&/^version/{gsub(/.*version = "/,""); gsub(/".*/,""); print; exit}' Cargo.toml)"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  if [[ "$ARCH_TAG" != "$HOST_TAG" ]]; then
    rustup target add "$RUST_TARGET" >/dev/null
  fi
  export ANYCODE_BUILD_TARGET=cloud
  export ANYCODE_TAURI_TARGET="$RUST_TARGET"
  export ANYCODE_DMG_ARCH_TAG="$ARCH_TAG"
  unset ANYCODE_DESKTOP_LOCAL_RELEASE
  ./scripts/build-desktop-release.sh
fi

# Prefer target-triple bundle path for cross builds
DMG=""
APP=""
for candidate in \
  "$ROOT/target/${RUST_TARGET}/release/bundle/dmg/anyCode_${VERSION}_${ARCH_TAG}.dmg" \
  "$ROOT/target/release/bundle/dmg/anyCode_${VERSION}_${ARCH_TAG}.dmg"
do
  if [[ -f "$candidate" ]]; then
    DMG="$candidate"
    break
  fi
done

for candidate in \
  "$ROOT/target/${RUST_TARGET}/release/bundle/macos/anyCode.app" \
  "$ROOT/target/release/bundle/macos/anyCode.app"
do
  if [[ -d "$candidate" ]]; then
    APP="$candidate"
    break
  fi
done

if [[ -z "$DMG" || ! -f "$DMG" ]]; then
  echo "DMG not found for ${VERSION}/${ARCH_TAG}" >&2
  ls -la "$ROOT/target/release/bundle/dmg" 2>/dev/null || true
  ls -la "$ROOT/target/${RUST_TARGET}/release/bundle/dmg" 2>/dev/null || true
  exit 1
fi

echo "==> verify Gatekeeper / notarization"
if [[ -n "$APP" && -d "$APP" ]]; then
  if spctl -a -vv -t install "$APP" 2>&1 | tee /tmp/anycode-spctl.log | grep -qE 'accepted|Notarized'; then
    echo "Gatekeeper: OK"
  else
    echo "WARNING: spctl did not report accepted/notarized — check /tmp/anycode-spctl.log" >&2
  fi
fi

DOWNLOAD_DIR="${ANYCODE_DOWNLOAD_DIR:-$ROOT/crates/account-portal/public/downloads}"
mkdir -p "$DOWNLOAD_DIR"

STAGED="$DOWNLOAD_DIR/anyCode_${VERSION}_${ARCH_TAG}.dmg"
LATEST="$DOWNLOAD_DIR/anyCode_latest_${ARCH_TAG}.dmg"
cp -f "$DMG" "$STAGED"
cp -f "$DMG" "$LATEST"

python3 "$ROOT/scripts/lib/regen-downloads-manifest.py" "$DOWNLOAD_DIR"

echo ""
echo "Done."
echo "  DMG:     $STAGED"
echo "  Latest:  $LATEST"
echo "  Manifest: $DOWNLOAD_DIR/releases.json"
echo "  URL:     https://anycode.work/downloads/$(basename "$STAGED")"
echo ""

if [[ "$UPLOAD" -eq 1 && -n "${ANYCODE_DOWNLOAD_UPLOAD:-}" ]]; then
  echo "==> ANYCODE_DOWNLOAD_UPLOAD"
  # shellcheck disable=SC2086
  eval "$ANYCODE_DOWNLOAD_UPLOAD"
fi
