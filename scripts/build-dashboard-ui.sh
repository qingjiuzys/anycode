#!/usr/bin/env bash
# Build Digital Workbench static UI into crates/dashboard-ui/dist
# Run before release or: anycode dashboard (auto-serves dist when present)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UI="$ROOT/crates/dashboard-ui"
NPM_FP="$UI/.npm-fingerprint"
DIST_FP="$UI/.dist-fingerprint"
LOCKFILE="$UI/package-lock.json"

if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to build dashboard-ui" >&2
  exit 1
fi

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

src_tree_signature() {
  if command -v git >/dev/null 2>&1 && git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$ROOT" ls-files 'crates/dashboard-ui/src' 'crates/dashboard-ui/vite.config.ts' \
      'crates/dashboard-ui/tsconfig.json' 'crates/dashboard-ui/tsconfig.app.json' \
      'crates/dashboard-ui/index.html' 2>/dev/null | sort | while read -r f; do
      [[ -f "$ROOT/$f" ]] && sha256_file "$ROOT/$f"
    done | shasum -a 256 | awk '{print $1}'
    return
  fi
  find "$UI/src" "$UI/vite.config.ts" "$UI/tsconfig.json" "$UI/index.html" -type f 2>/dev/null \
    | sort | while read -r f; do
    if stat -f '%m:%z' "$f" >/dev/null 2>&1; then
      stat -f '%m:%z' "$f"
    else
      stat -c '%Y:%s' "$f"
    fi
  done | shasum -a 256 | awk '{print $1}'
}

expected_npm_fingerprint() {
  if [[ -f "$LOCKFILE" ]]; then
    printf 'lock=%s\n' "$(sha256_file "$LOCKFILE")"
  else
    printf 'lock=none\n'
  fi
}

expected_dist_fingerprint() {
  printf 'lock=%s\nsrc=%s\n' "$(sha256_file "$LOCKFILE")" "$(src_tree_signature)"
}

npm_cache_hit() {
  [[ "${ANYCODE_DASHBOARD_UI_FORCE:-}" == "1" ]] && return 1
  [[ -d "$UI/node_modules" ]] || return 1
  [[ -f "$NPM_FP" ]] || return 1
  [[ "$(expected_npm_fingerprint)" == "$(cat "$NPM_FP")" ]]
}

dist_cache_hit() {
  [[ "${ANYCODE_DASHBOARD_UI_FORCE:-}" == "1" ]] && return 1
  [[ -f "$UI/dist/index.html" ]] || return 1
  [[ -f "$DIST_FP" ]] || return 1
  [[ "$(expected_dist_fingerprint)" == "$(cat "$DIST_FP")" ]]
}

write_npm_fingerprint() {
  expected_npm_fingerprint >"$NPM_FP"
}

write_dist_fingerprint() {
  expected_dist_fingerprint >"$DIST_FP"
}

cd "$UI"
if npm_cache_hit; then
  echo "dashboard-ui npm cache hit, skip npm ci (set ANYCODE_DASHBOARD_UI_FORCE=1 to refresh)"
else
  if [[ -f package-lock.json ]]; then
    npm ci
  else
    npm install
  fi
  write_npm_fingerprint
fi

"$ROOT/scripts/sync-workspace-version.sh"

if dist_cache_hit; then
  echo "dashboard-ui dist cache hit, skip vite build"
else
  npm run build
  write_dist_fingerprint
fi

if [[ ! -f dist/index.html ]]; then
  echo "build failed: dist/index.html missing" >&2
  exit 1
fi

echo "Dashboard UI built: $UI/dist"
if command -v shasum >/dev/null 2>&1; then
  echo "dist hash: $(shasum -a 256 dist/index.html | cut -d' ' -f1)"
fi
