#!/usr/bin/env bash
# Build account-portal SPA (anycode.work landing + console) into crates/account-portal/dist.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORTAL="$ROOT/crates/account-portal"
BRAND_MARK="$ROOT/brand/anycode-mark.svg"

if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to build account-portal" >&2
  exit 1
fi

if [[ -f "$BRAND_MARK" ]]; then
  cp -f "$BRAND_MARK" "$PORTAL/public/favicon.svg"
fi

cd "$PORTAL"
if [[ -f package-lock.json ]]; then
  npm ci
else
  npm install
fi
npm run build

test -f dist/index.html || {
  echo "account-portal build failed: dist/index.html missing" >&2
  exit 1
}

echo "Account portal built: $PORTAL/dist"
if command -v shasum >/dev/null 2>&1; then
  echo "dist hash: $(shasum -a 256 dist/index.html | awk '{print $1}')"
fi
