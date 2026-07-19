#!/usr/bin/env bash
# Submit anyCode.app to Apple notarization and staple ticket.
set -euo pipefail

APP="${1:-}"
if [[ -z "$APP" || ! -d "$APP" ]]; then
  echo "usage: notarize-mac-app.sh /path/to/anyCode.app" >&2
  exit 1
fi

for var in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  if [[ -z "${!var:-}" ]]; then
    echo "Set $var for notarization" >&2
    exit 1
  fi
done

ZIP="$(mktemp -t anycode-notarize.XXXXXX.zip)"
trap 'rm -f "$ZIP"' EXIT

echo "==> zip for notarization (ditto, no __MACOSX)"
ditto -c -k --keepParent "$APP" "$ZIP"

echo "==> notarytool submit"
SUBMIT_OUT="$(mktemp)"
if ! xcrun notarytool submit "$ZIP" \
  --apple-id "$APPLE_ID" \
  --password "$APPLE_PASSWORD" \
  --team-id "$APPLE_TEAM_ID" \
  --wait 2>&1 | tee "$SUBMIT_OUT"; then
  echo "notarytool submit failed" >&2
  exit 1
fi
if grep -q "status: Invalid" "$SUBMIT_OUT"; then
  echo "notarization Invalid — fetch log with: xcrun notarytool log <id> ..." >&2
  exit 1
fi
rm -f "$SUBMIT_OUT"

echo "==> staple"
if [[ -d /Applications/Xcode.app/Contents/Developer ]]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
xcrun stapler staple "$APP"
spctl -a -vv -t install "$APP" 2>&1 | tee /tmp/anycode-spctl.log || true
echo "notarization OK"
