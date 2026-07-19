#!/usr/bin/env bash
# Deep-sign all Mach-O binaries inside anyCode.app (Playwright Chromium, ffmpeg, etc.)
set -euo pipefail

APP="${1:-}"
IDENTITY="${APPLE_SIGNING_IDENTITY:-}"

if [[ -z "$APP" || ! -d "$APP" ]]; then
  echo "usage: codesign-mac-app-deep.sh /path/to/anyCode.app" >&2
  exit 1
fi
if [[ -z "$IDENTITY" ]]; then
  IDENTITY="$(security find-identity -v -p codesigning | awk -F'"' '/Developer ID Application/ { print $2; exit }')"
  export APPLE_SIGNING_IDENTITY="$IDENTITY"
fi
if [[ -z "$IDENTITY" ]]; then
  echo "APPLE_SIGNING_IDENTITY not set" >&2
  exit 1
fi

echo "==> deep sign nested binaries in $APP"
xattr -cr "$APP" 2>/dev/null || true

is_macho() {
  file -b "$1" 2>/dev/null | grep -qE 'Mach-O|executable'
}

# Innermost files first (dylibs, helpers, executables)
while IFS= read -r -d '' f; do
  if is_macho "$f"; then
    codesign --force --options runtime --timestamp --sign "$IDENTITY" "$f" 2>/dev/null || true
  fi
done < <(find "$APP" -type f \( -name '*.dylib' -o -name '*.so' -o -perm -111 \) -print0)

# Nested .app bundles (Chrome for Testing, helpers) — deepest first
while IFS= read -r nested; do
  [[ "$nested" == "$APP" ]] && continue
  codesign --force --options runtime --timestamp --sign "$IDENTITY" "$nested" 2>/dev/null || true
done < <(find "$APP" -name '*.app' -type d | awk '{ print length, $0 }' | sort -rn | cut -d' ' -f2-)

codesign --force --options runtime --timestamp --sign "$IDENTITY" "$APP"
echo "==> verify codesign"
codesign --verify --deep --strict --verbose=2 "$APP"
echo "deep sign OK"
