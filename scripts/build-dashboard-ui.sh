#!/usr/bin/env bash
# Build Digital Workbench static UI into crates/dashboard-ui/dist
# Run before release or: anycode dashboard (auto-serves dist when present)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UI="$ROOT/crates/dashboard-ui"

if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to build dashboard-ui" >&2
  exit 1
fi

cd "$UI"
if [[ -f package-lock.json ]]; then
  npm ci
else
  npm install
fi
npm run build

if [[ ! -f dist/index.html ]]; then
  echo "build failed: dist/index.html missing" >&2
  exit 1
fi

echo "Dashboard UI built: $UI/dist"
if command -v shasum >/dev/null 2>&1; then
  echo "dist hash: $(shasum -a 256 dist/index.html | cut -d' ' -f1)"
fi
echo "Next:"
echo "  cargo fmt --all -- --check"
echo "  cargo test -p anycode-dashboard"
echo "  cargo build --release -p anycode"
echo "Run: anycode dashboard --open"
echo "Doctor: anycode dashboard doctor"
