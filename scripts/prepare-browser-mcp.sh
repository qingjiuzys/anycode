#!/usr/bin/env bash
# Stage Playwright MCP + Chromium (+ optional Node runtime) for desktop bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/apps/anycode-desktop/browser-mcp"
STAGE="$ROOT/apps/anycode-desktop/resources/browser"

if [[ ! -f "$SRC/package.json" ]]; then
  echo "missing browser-mcp package: $SRC" >&2
  exit 1
fi

echo "==> prepare browser MCP bundle"
rm -rf "$STAGE"
mkdir -p "$STAGE/browsers"

cd "$SRC"
npm ci --omit=dev

PLAYWRIGHT_BROWSERS_PATH="$STAGE/browsers" npx playwright install chromium

cp package.json package-lock.json run.sh "$STAGE/"
chmod +x "$STAGE/run.sh"
cp -R node_modules "$STAGE/"

NODE_VER="${ANYCODE_BROWSER_NODE_VERSION:-22.16.0}"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64) NODE_ARCH="arm64" ;;
  x86_64) NODE_ARCH="x64" ;;
  *)
    echo "unsupported arch for bundled node: $ARCH (skipping node bundle)" >&2
    NODE_ARCH=""
    ;;
esac

if [[ -n "$NODE_ARCH" ]]; then
  NODE_TAR="node-v${NODE_VER}-${OS}-${NODE_ARCH}"
  TMP_NODE="$(mktemp -d)"
  curl -fsSL "https://nodejs.org/dist/v${NODE_VER}/${NODE_TAR}.tar.xz" | tar -xJ -C "$TMP_NODE"
  mv "$TMP_NODE/$NODE_TAR" "$STAGE/node"
  rm -rf "$TMP_NODE"
  echo "bundled node ${NODE_VER} (${OS}-${NODE_ARCH})"
fi

test -x "$STAGE/run.sh" || chmod +x "$STAGE/run.sh"
test -f "$STAGE/node_modules/@playwright/mcp/cli.js" || {
  echo "browser MCP cli missing after install" >&2
  exit 1
}

echo "browser MCP staged: $STAGE"
