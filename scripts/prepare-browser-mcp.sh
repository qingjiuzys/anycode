#!/usr/bin/env bash
# Stage Playwright MCP + Chromium (+ optional Node runtime) for desktop bundle.
# Reuses apps/anycode-desktop/resources/browser/ when lockfile + platform unchanged.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/apps/anycode-desktop/browser-mcp"
STAGE="$ROOT/apps/anycode-desktop/resources/browser"
FINGERPRINT="$STAGE/.bundle-fingerprint"
LOCKFILE="$SRC/package-lock.json"

if [[ ! -f "$SRC/package.json" ]]; then
  echo "missing browser-mcp package: $SRC" >&2
  exit 1
fi

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

sha256_file() {
  local f="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$f" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    echo "sha256 unavailable" >&2
    exit 1
  fi
}

expected_fingerprint() {
  local lock_hash
  lock_hash="$(sha256_file "$LOCKFILE")"
  printf 'lock=%s\nnode=%s\nos=%s\narch=%s\n' "$lock_hash" "$NODE_VER" "$OS" "$ARCH"
}

browser_bundle_complete() {
  [[ -f "$STAGE/run.sh" ]] \
    && [[ -f "$STAGE/node_modules/@playwright/mcp/cli.js" ]] \
    && [[ -d "$STAGE/browsers" ]] \
    && [[ -n "$(find "$STAGE/browsers" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]]
}

cache_hit() {
  [[ "${ANYCODE_BROWSER_MCP_FORCE:-}" == "1" ]] && return 1
  browser_bundle_complete || return 1
  [[ -f "$FINGERPRINT" ]] || return 1
  [[ "$(expected_fingerprint)" == "$(cat "$FINGERPRINT")" ]]
}

write_fingerprint() {
  expected_fingerprint >"$FINGERPRINT"
}

fetch_node_tarball() {
  local node_tar="$1"
  local cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/anycode"
  local cache_file="$cache_dir/${node_tar}.tar.xz"
  mkdir -p "$cache_dir"
  if [[ -f "$cache_file" ]]; then
    echo "using cached node tarball: $cache_file"
    cp "$cache_file" "$2"
    return 0
  fi
  curl -fsSL "https://nodejs.org/dist/v${NODE_VER}/${node_tar}.tar.xz" -o "$2"
  cp "$2" "$cache_file"
}

echo "==> prepare browser MCP bundle"

if cache_hit; then
  echo "browser MCP cache hit, skip download (set ANYCODE_BROWSER_MCP_FORCE=1 to refresh)"
  exit 0
fi

if [[ "${ANYCODE_BROWSER_MCP_FORCE:-}" == "1" ]]; then
  echo "browser MCP cache miss: ANYCODE_BROWSER_MCP_FORCE=1"
elif ! browser_bundle_complete; then
  echo "browser MCP cache miss: staged bundle incomplete (first build or resources/browser removed)"
elif [[ ! -f "$FINGERPRINT" ]]; then
  echo "browser MCP cache miss: missing $FINGERPRINT"
else
  echo "browser MCP cache miss: lockfile, node version, or platform changed"
  echo "  expected: $(expected_fingerprint)"
  echo "  stored:   $(cat "$FINGERPRINT")"
fi

rm -rf "$STAGE"
mkdir -p "$STAGE/browsers"

cd "$SRC"
npm ci --omit=dev

PLAYWRIGHT_BROWSERS_PATH="$STAGE/browsers" npx playwright install chromium

cp package.json package-lock.json run.sh "$STAGE/"
chmod +x "$STAGE/run.sh"
cp -R node_modules "$STAGE/"

if [[ -n "$NODE_ARCH" ]]; then
  NODE_TAR="node-v${NODE_VER}-${OS}-${NODE_ARCH}"
  TMP_NODE="$(mktemp -d)"
  TMP_XZ="$TMP_NODE/${NODE_TAR}.tar.xz"
  fetch_node_tarball "$NODE_TAR" "$TMP_XZ"
  tar -xJ -f "$TMP_XZ" -C "$TMP_NODE"
  mv "$TMP_NODE/$NODE_TAR" "$STAGE/node"
  rm -rf "$TMP_NODE"
  echo "bundled node ${NODE_VER} (${OS}-${NODE_ARCH})"
fi

test -x "$STAGE/run.sh" || chmod +x "$STAGE/run.sh"
test -f "$STAGE/node_modules/@playwright/mcp/cli.js" || {
  echo "browser MCP cli missing after install" >&2
  exit 1
}

write_fingerprint
echo "browser MCP staged: $STAGE"
