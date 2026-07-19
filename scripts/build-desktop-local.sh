#!/usr/bin/env bash
# Local developer DMG: loopback account login (127.0.0.1:43200/43201), not anycode.work.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export ROOT

export ANYCODE_BUILD_TARGET=local
export ANYCODE_DESKTOP_LOCAL_RELEASE=1

exec "$ROOT/scripts/build-desktop-release.sh"
