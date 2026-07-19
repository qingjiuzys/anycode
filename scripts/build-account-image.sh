#!/usr/bin/env bash
# Local signed DMG → account-portal public/downloads → Docker image → ACR push.
#
# Prereqs:
#   - macOS + ~/.anycode/release.env (see scripts/release.env.example)
#   - deploy/account-service/.env with WECHAT_PAY_API_V3_KEY
#
# Usage:
#   ./scripts/build-account-image.sh              # build DMG (if needed) + push image
#   ./scripts/build-account-image.sh --skip-dmg   # image only (DMG already in public/downloads)
#   TAG=0.2.4 ./scripts/build-account-image.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
export ANYCODE_BUILD_TARGET=cloud
anycode_apply_build_target_exports

SKIP_DMG=0
for arg in "$@"; do
  case "$arg" in
    --skip-dmg) SKIP_DMG=1 ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $arg (try --skip-dmg)" >&2
      exit 1
      ;;
  esac
done

DOWNLOAD_DIR="$ROOT/crates/account-portal/public/downloads"
has_dmg() {
  compgen -G "$DOWNLOAD_DIR/anyCode_"*"_aarch64.dmg" >/dev/null 2>&1
}

if [[ "$SKIP_DMG" -eq 0 ]]; then
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "DMG build requires macOS. Use --skip-dmg if DMG is already staged." >&2
    exit 1
  fi
echo "==> signed desktop DMG (stages into account-portal/public/downloads)"
"$ROOT/scripts/build-account-portal.sh"
"$ROOT/scripts/release-desktop-local.sh"
elif ! has_dmg; then
  echo "No DMG in $DOWNLOAD_DIR — run without --skip-dmg on macOS first." >&2
  exit 1
fi

echo "==> portal downloads staged:"
ls -lh "$DOWNLOAD_DIR"/*.dmg 2>/dev/null || ls -lh "$DOWNLOAD_DIR"

echo "==> docker build + push (account API + portal + DMG baked in)"
exec "$ROOT/deploy/account-service/build-push.sh"
