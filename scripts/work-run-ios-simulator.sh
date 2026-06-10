#!/usr/bin/env bash
# Run one or all work-run Flutter apps on the booted iOS Simulator.
# Prereq: DEVELOPER_DIR, CocoaPods, iOS platform in Xcode (26.5 matching Xcode.app).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

DEVICE="${FLUTTER_IOS_DEVICE:-}"
if [[ -z "$DEVICE" ]]; then
  DEVICE="$(xcrun simctl list devices booted | grep -oE '[A-F0-9-]{36}' | head -1 || true)"
fi
if [[ -z "$DEVICE" ]]; then
  echo "Booting iPhone 17 Pro (iOS 26.3 runtime)..."
  xcrun simctl boot "iPhone 17 Pro" 2>/dev/null || true
  open -a Simulator
  sleep 3
  DEVICE="$(xcrun simctl list devices booted | grep -oE '[A-F0-9-]{36}' | head -1)"
fi
if [[ -z "$DEVICE" ]]; then
  echo "No booted simulator. Open Simulator.app and boot a device."
  exit 1
fi

APPS="${*:-test-flutter-01 test-flutter-02 test-flutter-03}"
echo "Using simulator: $DEVICE"
echo "Apps: $APPS"
echo "Tip: export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer"

for d in $APPS; do
  echo ""
  echo ">>> flutter run -d $DEVICE in $d"
  (cd "$ROOT/$d" && flutter pub get && flutter run -d "$DEVICE") &
done
wait
