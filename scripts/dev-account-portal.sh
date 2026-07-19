#!/usr/bin/env bash
# Local dev stack — everything on loopback (MySQL in Docker + host services).
# Usage:
#   ./scripts/dev-account-portal.sh          # print instructions
#   ./scripts/dev-account-portal.sh stack    # MySQL + account API + portal dist (one shot)
#   ./scripts/dev-account-portal.sh mysql    # start MySQL only
#   ./scripts/dev-account-portal.sh api      # start account-service (starts MySQL if needed)
#   ./scripts/dev-account-portal.sh portal   # start account-portal Vite HMR
#   ./scripts/dev-account-portal.sh workbench # start dashboard-serve
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
# shellcheck source=scripts/lib/local-mysql.sh
source "$ROOT/scripts/lib/local-mysql.sh"
export ANYCODE_BUILD_TARGET=local
anycode_apply_build_target_exports

API_PORT="${ANYCODE_LOCAL_ACCOUNT_API_PORT}"
PORTAL_PORT="${ANYCODE_LOCAL_ACCOUNT_PORTAL_PORT}"
DASHBOARD_PORT="${ANYCODE_DASHBOARD_DEV_PORT:-43180}"

export DATABASE_URL="$(local_mysql_url)"
export CORS_ORIGINS="http://127.0.0.1:${PORTAL_PORT},http://localhost:${PORTAL_PORT},http://127.0.0.1:43180,http://localhost:43180,http://127.0.0.1:5173,http://localhost:5173,http://127.0.0.1:${API_PORT},http://localhost:${API_PORT}"
export WECHAT_PAY_SKIP_VERIFY=1
export ADMIN_BOOTSTRAP_EMAIL="${ADMIN_BOOTSTRAP_EMAIL:-dev@anycode.local}"
export ADMIN_BOOTSTRAP_PASSWORD="${ADMIN_BOOTSTRAP_PASSWORD:-anycode-dev}"

pick_dashboard_port() {
  if lsof -i ":${DASHBOARD_PORT}" -sTCP:LISTEN >/dev/null 2>&1; then
    if [[ "${DASHBOARD_PORT}" == "43180" ]]; then
      DASHBOARD_PORT=43181
      echo "Port 43180 busy; using ${DASHBOARD_PORT} for dashboard-serve." >&2
    fi
  fi
  export ANYCODE_DASHBOARD_DEV_PORT="${DASHBOARD_PORT}"
}

start_mysql() {
  start_local_mysql
}

start_api() {
  exec "$ROOT/scripts/start-local-account.sh" "$@"
}

start_stack() {
  exec "$ROOT/scripts/start-local-account.sh" "$@"
}

start_portal() {
  cd "${ROOT}/crates/account-portal"
  npm install
  npm run dev
}

load_agnes_upstream_key() {
  if [[ -n "${AGNES_API_KEY:-}" ]]; then
    return 0
  fi
  if command -v python3 >/dev/null 2>&1; then
    local from_cfg
    from_cfg="$(python3 -c "import json, pathlib; c=json.loads(pathlib.Path('${HOME}/.anycode/config.json').read_text()); print(c.get('provider_credentials',{}).get('agnes',''))" 2>/dev/null || true)"
    if [[ -n "${from_cfg}" ]]; then
      export AGNES_API_KEY="${from_cfg}"
      return 0
    fi
  fi
  for f in "${HOME}/.anycode/secrets/agnes.txt" "${HOME}/.anycode/secrets/default.txt"; do
    if [[ -f "${f}" ]]; then
      export AGNES_API_KEY="$(tr -d '\n' < "${f}")"
      return 0
    fi
  done
  echo "WARN: no AGNES_API_KEY — set env or ~/.anycode/config.json provider_credentials.agnes" >&2
}

start_gateway() {
  load_agnes_upstream_key
  export MODEL_GATEWAY_HOST="${MODEL_GATEWAY_HOST:-127.0.0.1}"
  export MODEL_GATEWAY_PORT="${MODEL_GATEWAY_PORT:-43210}"
  export ANYCODE_ACCOUNT_API_URL="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:${API_PORT}}"
  cd "${ROOT}/crates/model-gateway"
  cargo build --release
  exec ./target/release/anycode-model-gateway
}

start_workbench() {
  pick_dashboard_port
  echo "Dashboard API: http://127.0.0.1:${DASHBOARD_PORT}"
  echo "Workbench UI:  http://localhost:5173"
  cd "${ROOT}"
  ANYCODE_BUILD_DASHBOARD_UI=1 cargo run --release -p anycode-dashboard \
    --features embedded-ui --bin anycode-dashboard-serve \
    -- --host 127.0.0.1 --port "${DASHBOARD_PORT}"
}

print_help() {
  pick_dashboard_port
  anycode_print_build_target_summary
  cat <<EOF
anyCode account portal — local dev stack (${ANYCODE_BUILD_TARGET})

Prerequisites: Docker, Node.js, Rust toolchain.

One-shot (MySQL + API + portal dist on :43200):

  ./scripts/dev-account-portal.sh stack
  # same as ./scripts/start-local-account.sh

Optional separate terminals:

  1. MySQL only:  ./scripts/dev-account-portal.sh mysql
  2. API + portal: ./scripts/dev-account-portal.sh api
  3. Gateway:     ./scripts/dev-account-portal.sh gateway
  4. Portal HMR:  ./scripts/dev-account-portal.sh portal → :${PORTAL_PORT}/login

Optional Workbench (browser):

  3. Dashboard: ./scripts/dev-account-portal.sh workbench
  4. UI HMR:    cd crates/dashboard-ui && ANYCODE_BUILD_TARGET=local ANYCODE_DASHBOARD_DEV_PORT=${DASHBOARD_PORT} npm run dev
     → http://localhost:5173/account

Local desktop DMG (loopback login baked in):

  ./scripts/build-desktop-local.sh
  ./scripts/install-desktop-macos.sh --build

Cloud shipping DMG (anycode.work login):

  ./scripts/release-desktop-local.sh

Test account (portal login): ${ADMIN_BOOTSTRAP_EMAIL} / ${ADMIN_BOOTSTRAP_PASSWORD}
Or register at ${ANYCODE_ACCOUNT_PORTAL_URL}/register

Note: account-service uses MySQL (not Postgres). Schema: deploy/account-service/schema.mysql.sql
EOF
}

cmd="${1:-}"
shift || true
case "${cmd}" in
  mysql) start_mysql ;;
  stack|all) start_stack "$@" ;;
  api) start_api "$@" ;;
  gateway) start_gateway ;;
  portal) start_portal ;;
  workbench) start_workbench ;;
  "") print_help ;;
  *)
    echo "Unknown command: ${cmd}" >&2
    print_help
    exit 1
    ;;
esac
