#!/usr/bin/env bash
# Local account stack: MySQL (Docker) + account-service (host cargo) + portal dist on :43200.
#
# Usage:
#   ./scripts/start-local-account.sh              # MySQL + portal build + API (default)
#   ./scripts/start-local-account.sh --no-build   # skip portal rebuild
#   ./scripts/start-local-account.sh --remote-db    # use deploy/account-service/.env DATABASE_URL
#   ./scripts/start-local-account.sh --background  # API as background daemon (macOS may be flaky)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
# shellcheck source=scripts/lib/local-mysql.sh
source "$ROOT/scripts/lib/local-mysql.sh"
export ANYCODE_BUILD_TARGET=local
anycode_apply_build_target_exports

CONTAINER=anycode-account-local
PID_FILE="${TMPDIR:-/tmp}/anycode-account-local.pid"
LEGACY_IMAGE="${ANYCODE_ACCOUNT_IMAGE:-registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:0.2.3}"
PORTAL_DIST="$ROOT/crates/account-portal/dist"
OPS_DIST="$ROOT/crates/ops-portal/dist"
ENV_FILE="$ROOT/deploy/account-service/.env"
ACCOUNT_CRATE="$ROOT/crates/account-service"
ACCOUNT_BIN="$ACCOUNT_CRATE/target/release/anycode-account"
ACCOUNT_LOG="${TMPDIR:-/tmp}/anycode-account-local.log"
RUNTIME_IMAGE="${ANYCODE_ACCOUNT_RUNTIME_IMAGE:-debian:bookworm-slim}"

NO_BUILD=0
USE_DOCKER_API=0
USE_REMOTE_DB=0
BACKGROUND=0
for arg in "$@"; do
  case "$arg" in
    --no-build) NO_BUILD=1 ;;
    --docker-api|--docker) USE_DOCKER_API=1 ;;
    --remote-db) USE_REMOTE_DB=1 ;;
    --background|-d) BACKGROUND=1 ;;
    -h|--help)
      sed -n '2,9p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $arg" >&2
      exit 1
      ;;
  esac
done

if [[ "$NO_BUILD" -eq 0 ]]; then
  "$ROOT/scripts/build-account-portal.sh"
fi

if [[ ! -f "$PORTAL_DIST/index.html" ]]; then
  echo "Missing $PORTAL_DIST/index.html — run without --no-build first." >&2
  exit 1
fi

if [[ "$USE_REMOTE_DB" -eq 1 ]]; then
  if [[ -f "$ENV_FILE" ]]; then
    # shellcheck source=/dev/null
    source "$ENV_FILE"
  fi
  if [[ -z "${DATABASE_URL:-}" ]]; then
    echo "DATABASE_URL is required with --remote-db (set env or $ENV_FILE)" >&2
    exit 1
  fi
  echo "Using remote DATABASE_URL from env/.env"
else
  start_local_mysql
  if [[ "$USE_DOCKER_API" -eq 1 ]]; then
    DATABASE_URL="$(local_mysql_url_for_docker)"
  else
    DATABASE_URL="$(local_mysql_url)"
  fi
fi

IDENTITY_ENCRYPTION_SECRET="${IDENTITY_ENCRYPTION_SECRET:-anycode-local-identity-dev-secret}"
ADMIN_BOOTSTRAP_EMAIL="${ADMIN_BOOTSTRAP_EMAIL:-dev@anycode.local}"
ADMIN_BOOTSTRAP_PASSWORD="${ADMIN_BOOTSTRAP_PASSWORD:-anycode-dev}"
CORS_ORIGINS="http://127.0.0.1:43180,http://localhost:43180,http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT},http://localhost:${ANYCODE_LOCAL_ACCOUNT_API_PORT}"

stop_host_api() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      for _ in $(seq 1 10); do
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.5
      done
    fi
    rm -f "$PID_FILE"
  fi
  if lsof -ti ":${ANYCODE_LOCAL_ACCOUNT_API_PORT}" >/dev/null 2>&1; then
    lsof -ti ":${ANYCODE_LOCAL_ACCOUNT_API_PORT}" | xargs kill 2>/dev/null || true
  fi
}

stop_docker_api() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}

wait_for_health() {
  for _ in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "account-service did not become healthy on :${ANYCODE_LOCAL_ACCOUNT_API_PORT}" >&2
  docker logs "$CONTAINER" 2>&1 | tail -n 40 >&2 || true
  if [[ -f "$ACCOUNT_LOG" ]]; then
    echo "--- host API log ---" >&2
    tail -n 40 "$ACCOUNT_LOG" >&2 || true
  fi
  return 1
}

build_host_api() {
  echo "==> building account-service (host cargo release)"
  (cd "$ACCOUNT_CRATE" && cargo build --release)
  test -x "$ACCOUNT_BIN"
}

start_host_api() {
  stop_host_api
  stop_docker_api
  build_host_api
  local api_env=(
    "DATABASE_URL=$DATABASE_URL"
    "ACCOUNT_SERVICE_HOST=127.0.0.1"
    "ACCOUNT_SERVICE_PORT=${ANYCODE_LOCAL_ACCOUNT_API_PORT}"
    "ACCOUNT_PORTAL_DIR=$PORTAL_DIST"
    "ACCOUNT_PORTAL_URL=${ANYCODE_ACCOUNT_API_URL}"
    "ACCOUNT_PUBLIC_URL=${ANYCODE_ACCOUNT_API_URL}"
    "OPS_PORTAL_DIR=$OPS_DIST"
    "CORS_ORIGINS=$CORS_ORIGINS"
    "IDENTITY_ENCRYPTION_SECRET=$IDENTITY_ENCRYPTION_SECRET"
    "ADMIN_BOOTSTRAP_EMAIL=$ADMIN_BOOTSTRAP_EMAIL"
    "ADMIN_BOOTSTRAP_PASSWORD=$ADMIN_BOOTSTRAP_PASSWORD"
    "WECHAT_PAY_SKIP_VERIFY=1"
  )
  if [[ "$BACKGROUND" -eq 1 ]]; then
    echo "==> starting anycode-account in background (:${ANYCODE_LOCAL_ACCOUNT_API_PORT})"
    : >"$ACCOUNT_LOG"
    nohup env "${api_env[@]}" "$ACCOUNT_BIN" >>"$ACCOUNT_LOG" 2>&1 </dev/null &
    echo $! >"$PID_FILE"
    disown -h $! 2>/dev/null || true
    return 0
  fi
  echo "==> starting anycode-account (foreground — Ctrl+C to stop)"
  echo "    MySQL :${LOCAL_MYSQL_PORT} | Portal ${ANYCODE_ACCOUNT_API_URL}"
  echo "    Login ${ADMIN_BOOTSTRAP_EMAIL} / ${ADMIN_BOOTSTRAP_PASSWORD}"
  exec env "${api_env[@]}" "$ACCOUNT_BIN"
}

start_docker_api() {
  stop_host_api
  stop_docker_api
  build_host_api
  if ! docker image inspect "$RUNTIME_IMAGE" >/dev/null 2>&1; then
    docker pull "$RUNTIME_IMAGE" >/dev/null
  fi
  echo "==> starting $CONTAINER (host binary in $RUNTIME_IMAGE)"
  docker run --rm -d --name "$CONTAINER" \
    --add-host=host.docker.internal:host-gateway \
    -p "${ANYCODE_LOCAL_ACCOUNT_API_PORT}:8080" \
    -v "$PORTAL_DIST:/app/portal:ro" \
    -v "$ACCOUNT_BIN:/usr/local/bin/anycode-account:ro" \
    -e "DATABASE_URL=${DATABASE_URL}" \
    -e "ACCOUNT_SERVICE_HOST=0.0.0.0" \
    -e "ACCOUNT_SERVICE_PORT=8080" \
    -e "ACCOUNT_PORTAL_DIR=/app/portal" \
    -e "ACCOUNT_PORTAL_URL=${ANYCODE_ACCOUNT_API_URL}" \
    -e "ACCOUNT_PUBLIC_URL=${ANYCODE_ACCOUNT_API_URL}" \
    -e "CORS_ORIGINS=${CORS_ORIGINS}" \
    -e "IDENTITY_ENCRYPTION_SECRET=${IDENTITY_ENCRYPTION_SECRET}" \
    -e "ADMIN_BOOTSTRAP_EMAIL=${ADMIN_BOOTSTRAP_EMAIL}" \
    -e "ADMIN_BOOTSTRAP_PASSWORD=${ADMIN_BOOTSTRAP_PASSWORD}" \
    -e "WECHAT_PAY_SKIP_VERIFY=1" \
    "$RUNTIME_IMAGE" \
    /usr/local/bin/anycode-account >/dev/null
}

if [[ "$USE_DOCKER_API" -eq 1 ]]; then
  start_docker_api
  wait_for_health
  curl -s "http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}/health" | python3 -m json.tool 2>/dev/null || true
  echo ""
  echo "Stack (local):"
  echo "  MySQL:   127.0.0.1:${LOCAL_MYSQL_PORT}  (${LOCAL_MYSQL_CONTAINER})"
  echo "  Portal:  http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}/"
  echo "  Login:   http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}/login"
  echo "  API:     docker:${CONTAINER}"
  echo ""
  echo "Portal login: ${ADMIN_BOOTSTRAP_EMAIL} / ${ADMIN_BOOTSTRAP_PASSWORD}"
  exit 0
fi

if [[ "$BACKGROUND" -eq 1 ]]; then
  start_host_api
  wait_for_health
  curl -s "http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}/health" | python3 -m json.tool 2>/dev/null || true
  echo ""
  echo "Stack (local, background):"
  echo "  MySQL:   127.0.0.1:${LOCAL_MYSQL_PORT}  (${LOCAL_MYSQL_CONTAINER})"
  echo "  Portal:  http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}/"
  echo "  API pid: $(cat "$PID_FILE" 2>/dev/null || echo n/a)"
  echo "  API log: $ACCOUNT_LOG"
  echo "  Portal login: ${ADMIN_BOOTSTRAP_EMAIL} / ${ADMIN_BOOTSTRAP_PASSWORD}"
  exit 0
fi

start_host_api
