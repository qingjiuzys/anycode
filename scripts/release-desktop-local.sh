#!/usr/bin/env bash
# Build a signed + notarized macOS DMG locally and stage for anycode.work download.
#
# Prereqs:
#   - Developer ID Application cert in login keychain
#   - ~/.anycode/release.env (see scripts/release.env.example)
#
# Usage:
#   ./scripts/release-desktop-local.sh              # build + verify + stage
#   ./scripts/release-desktop-local.sh --upload     # also run ANYCODE_DOWNLOAD_UPLOAD
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

UPLOAD=0
if [[ "${1:-}" == "--upload" ]]; then
  UPLOAD=1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS only (signed DMG + notarization)." >&2
  exit 1
fi

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

# iching-style alias
if [[ -z "${APPLE_PASSWORD:-}" && -n "${APP_SPECIFIC_PASSWORD:-}" ]]; then
  export APPLE_PASSWORD="$APP_SPECIFIC_PASSWORD"
fi
if [[ -z "${APPLE_PASSWORD:-}" ]]; then
  echo "Set APPLE_PASSWORD (or APP_SPECIFIC_PASSWORD) in $RELEASE_ENV" >&2
  exit 1
fi

echo "Signing identity: $APPLE_SIGNING_IDENTITY"
echo "Team ID: $APPLE_TEAM_ID"

export ANYCODE_BUILD_TARGET=cloud
unset ANYCODE_DESKTOP_LOCAL_RELEASE
./scripts/build-desktop-release.sh

VERSION="$(awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f&&/^version/{gsub(/.*version = "/,""); gsub(/".*/,""); print; exit}' Cargo.toml)"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64) ARCH_TAG=aarch64 ;;
  x86_64) ARCH_TAG=x86_64 ;;
  *) ARCH_TAG="$ARCH" ;;
esac

DMG="$ROOT/target/release/bundle/dmg/anyCode_${VERSION}_${ARCH_TAG}.dmg"
APP="$ROOT/target/release/bundle/macos/anyCode.app"

if [[ ! -f "$DMG" ]]; then
  echo "DMG not found: $DMG" >&2
  exit 1
fi

echo "==> verify Gatekeeper / notarization"
if spctl -a -vv -t install "$APP" 2>&1 | tee /tmp/anycode-spctl.log | grep -qE 'accepted|Notarized'; then
  echo "Gatekeeper: OK"
else
  echo "WARNING: spctl did not report accepted/notarized — check /tmp/anycode-spctl.log" >&2
fi

DOWNLOAD_DIR="${ANYCODE_DOWNLOAD_DIR:-$ROOT/crates/account-portal/public/downloads}"
mkdir -p "$DOWNLOAD_DIR"

STAGED="$DOWNLOAD_DIR/anyCode_${VERSION}_${ARCH_TAG}.dmg"
LATEST="$DOWNLOAD_DIR/anyCode_latest_${ARCH_TAG}.dmg"
cp -f "$DMG" "$STAGED"
cp -f "$DMG" "$LATEST"

SHA="$(shasum -a 256 "$STAGED" | awk '{print $1}')"
echo "$SHA  $(basename "$STAGED")" >"$DOWNLOAD_DIR/SHA256SUMS.txt"

MANIFEST="$DOWNLOAD_DIR/latest.json"
python3 - "$MANIFEST" "$VERSION" "$ARCH_TAG" "$(basename "$STAGED")" "$SHA" <<'PY'
import json, sys
path, version, arch, filename, sha = sys.argv[1:6]
payload = {
    "version": version,
    "arch": arch,
    "filename": filename,
    "url": f"https://anycode.work/downloads/{filename}",
    "latest_url": f"https://anycode.work/downloads/anyCode_latest_{arch}.dmg",
    "sha256": sha,
}
with open(path, "w", encoding="utf-8") as f:
    json.dump(payload, f, indent=2)
    f.write("\n")
PY

echo ""
echo "Done."
echo "  DMG:     $STAGED"
echo "  Latest:  $LATEST"
echo "  SHA256:  $SHA"
echo "  URL:     https://anycode.work/downloads/$(basename "$STAGED")"
echo "  Latest:  https://anycode.work/downloads/anyCode_latest_${ARCH_TAG}.dmg"
echo ""
echo "Deploy: upload $DOWNLOAD_DIR/* to your portal static host, then redeploy account-portal."

if [[ "$UPLOAD" -eq 1 && -n "${ANYCODE_DOWNLOAD_UPLOAD:-}" ]]; then
  echo "==> ANYCODE_DOWNLOAD_UPLOAD"
  # shellcheck disable=SC2086
  eval "$ANYCODE_DOWNLOAD_UPLOAD"
fi
