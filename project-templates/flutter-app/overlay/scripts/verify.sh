#!/usr/bin/env bash
# Optional gate helper for Dashboard "project verify" — assumes Flutter is already on PATH.
# Agent environment setup is NOT defined here; see skill flutter-bootstrap.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v flutter >/dev/null 2>&1 || {
  echo "verify.sh: flutter not on PATH (Agent should install SDK first)" >&2
  exit 1
}
flutter pub get
flutter analyze
flutter test
if [[ "${REQUIRE_IOS_BUILD:-0}" == "1" ]]; then
  export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
  flutter build ios --simulator --no-codesign
fi
echo "verify.sh: passed"
