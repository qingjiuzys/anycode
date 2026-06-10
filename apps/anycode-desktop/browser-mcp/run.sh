#!/usr/bin/env bash
# Launch bundled @playwright/mcp (headless) with packaged Chromium.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CLI="${ROOT}/node_modules/@playwright/mcp/cli.js"

if [[ ! -f "$CLI" ]]; then
  echo "anycode browser-mcp: missing ${CLI}" >&2
  exit 1
fi

NODE_BIN=""
if [[ -x "${ROOT}/node/bin/node" ]]; then
  NODE_BIN="${ROOT}/node/bin/node"
elif command -v node >/dev/null 2>&1; then
  NODE_BIN="$(command -v node)"
else
  echo "anycode browser-mcp: node runtime not found (bundle node/ or install Node 18+)" >&2
  exit 1
fi

export PLAYWRIGHT_BROWSERS_PATH="${ROOT}/browsers"
export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS="${PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS:-true}"

exec "$NODE_BIN" "$CLI" --headless "$@"
