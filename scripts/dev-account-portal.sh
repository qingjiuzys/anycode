#!/usr/bin/env bash
# Local dev stack for anycode.work/login (account-portal + account-service).
# Usage:
#   ./scripts/dev-account-portal.sh          # print instructions + ensure deps
#   ./scripts/dev-account-portal.sh mysql    # start MySQL only
#   ./scripts/dev-account-portal.sh api      # start account-service (needs MySQL)
#   ./scripts/dev-account-portal.sh portal   # start account-portal Vite
#   ./scripts/dev-account-portal.sh workbench # start dashboard-serve + dashboard-ui vite
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MYSQL_CONTAINER=anycode-account-mysql
MYSQL_PORT=3307
API_PORT=43200
PORTAL_PORT=43201
DASHBOARD_PORT="${ANYCODE_DASHBOARD_DEV_PORT:-43180}"

export DATABASE_URL="mysql://anycode:anycode@127.0.0.1:${MYSQL_PORT}/anycode"
export ACCOUNT_PORTAL_URL="http://127.0.0.1:${PORTAL_PORT}"
export ACCOUNT_PUBLIC_URL="http://127.0.0.1:${API_PORT}"
export CORS_ORIGINS="http://127.0.0.1:${PORTAL_PORT},http://localhost:${PORTAL_PORT},http://127.0.0.1:43180,http://localhost:43180,http://127.0.0.1:5173,http://localhost:5173,http://127.0.0.1:${API_PORT},http://localhost:${API_PORT}"
export WECHAT_PAY_SKIP_VERIFY=1
export ADMIN_BOOTSTRAP_EMAIL="${ADMIN_BOOTSTRAP_EMAIL:-dev@anycode.local}"
export ADMIN_BOOTSTRAP_PASSWORD="${ADMIN_BOOTSTRAP_PASSWORD:-anycode-dev}"
export ANYCODE_ACCOUNT_API_URL="http://127.0.0.1:${API_PORT}"
export ANYCODE_ACCOUNT_PORTAL_URL="http://127.0.0.1:${PORTAL_PORT}"

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
  if docker ps --format '{{.Names}}' | grep -qx "${MYSQL_CONTAINER}"; then
    echo "MySQL already running (${MYSQL_CONTAINER} on :${MYSQL_PORT})"
    return
  fi
  docker rm -f "${MYSQL_CONTAINER}" >/dev/null 2>&1 || true
  docker run -d --name "${MYSQL_CONTAINER}" \
    -e MYSQL_ROOT_PASSWORD=anycode \
    -e MYSQL_DATABASE=anycode \
    -e MYSQL_USER=anycode \
    -e MYSQL_PASSWORD=anycode \
    -p "${MYSQL_PORT}:3306" \
    mysql:8.0
  echo "Waiting for MySQL..."
  for _ in $(seq 1 45); do
    if docker exec "${MYSQL_CONTAINER}" mysqladmin ping -h127.0.0.1 -uroot -panycode --silent 2>/dev/null; then
      break
    fi
    sleep 2
  done
  docker exec -i "${MYSQL_CONTAINER}" mysql -uroot -panycode anycode \
    < "${ROOT}/deploy/account-service/schema.mysql.sql"
  echo "MySQL ready on 127.0.0.1:${MYSQL_PORT}"
}

start_api() {
  start_mysql
  cd "${ROOT}/crates/account-service"
  cargo run -p anycode-account-service
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
  cat <<EOF
anycode.work/login — local dev stack

Prerequisites: Docker, Node.js, Rust toolchain.

Run in separate terminals:

  1. API:       ./scripts/dev-account-portal.sh api
  2. Gateway:  ./scripts/dev-account-portal.sh gateway
  3. Portal UI: ./scripts/dev-account-portal.sh portal
     → http://127.0.0.1:${PORTAL_PORT}/login

Optional Workbench (browser):

  3. Dashboard: ./scripts/dev-account-portal.sh workbench
  4. UI HMR:    cd crates/dashboard-ui && ANYCODE_DASHBOARD_DEV_PORT=${DASHBOARD_PORT} npm run dev
     → http://localhost:5173/account

Desktop (stop any process on :43180 first):

  export ANYCODE_ACCOUNT_API_URL=http://127.0.0.1:${API_PORT}
  export ANYCODE_ACCOUNT_PORTAL_URL=http://127.0.0.1:${PORTAL_PORT}
  ./scripts/build-dashboard-ui.sh
  cd apps/anycode-desktop && cargo tauri dev

Test account (bootstrap): ${ADMIN_BOOTSTRAP_EMAIL} / ${ADMIN_BOOTSTRAP_PASSWORD}
Or register at http://127.0.0.1:${PORTAL_PORT}/register

Note: account-service uses MySQL (not Postgres). Schema: deploy/account-service/schema.mysql.sql
EOF
}

cmd="${1:-}"
case "${cmd}" in
  mysql) start_mysql ;;
  api) start_api ;;
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
