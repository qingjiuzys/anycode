#!/usr/bin/env bash
# Work-run Flutter gates — aligned with dashboard gate_runner (pubspec.yaml):
#   flutter analyze, flutter test
# Plus Goal-engine checks: GOAL_ACCEPTANCE_OK + widget_test tap/pumpAndSettle.
# Optional iOS simulator smoke: flutter build ios --simulator (needs Xcode iOS platform).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
REQUIRE_IOS="${REQUIRE_IOS_BUILD:-1}"

if [[ ! -x "$DEVELOPER_DIR/usr/bin/xcodebuild" ]]; then
  echo "ERROR: Xcode not at DEVELOPER_DIR=$DEVELOPER_DIR"
  echo "  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer"
  exit 1
fi

fail=0
ios_fail_msg=""

for d in test-flutter-01 test-flutter-02 test-flutter-03; do
  dir="$ROOT/$d"
  echo ""
  echo "======== $d ========"

  if ! grep -qx 'GOAL_ACCEPTANCE_OK' "$dir/README.md" 2>/dev/null; then
    echo "FAIL: README.md must contain exact line GOAL_ACCEPTANCE_OK"
    fail=1
  fi
  wt="$dir/test/widget_test.dart"
  if ! grep -qE 'tester\.tap|\.tap\(find' "$wt" || ! grep -q 'pumpAndSettle' "$wt"; then
    echo "FAIL: $wt needs tester.tap + pumpAndSettle (goal_engine.rs)"
    fail=1
  fi

  (
    cd "$dir"
    flutter pub get
    echo "-- gate: flutter analyze --"
    flutter analyze
    echo "-- gate: flutter test --"
    flutter test --reporter expanded
  ) || { fail=1; continue; }

  if [[ "$REQUIRE_IOS" == "1" && -d "$dir/ios" ]]; then
    echo "-- gate: flutter build ios --simulator --"
    if ! flutter build ios --simulator --no-codesign 2>&1; then
      ios_fail_msg="iOS simulator build failed (often: Xcode > Settings > Components > install iOS 26.5 platform; simulator runtime may be 26.3 while SDK is 26.5)."
      fail=1
    fi
  fi
done

echo ""
if [[ "$fail" -ne 0 ]]; then
  echo "GATE SUMMARY: FAILED"
  [[ -n "$ios_fail_msg" ]] && echo "iOS: $ios_fail_msg"
  echo "Passed locally if only analyze+test matter: REQUIRE_IOS_BUILD=0 $0"
  exit 1
fi
if [[ "$REQUIRE_IOS" == "1" ]]; then
  echo "GATE SUMMARY: PASSED (analyze, test, goal markers, iOS simulator build)"
else
  echo "GATE SUMMARY: PASSED (analyze, test, goal markers) — iOS build skipped (REQUIRE_IOS_BUILD=0)"
fi
