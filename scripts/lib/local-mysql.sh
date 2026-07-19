#!/usr/bin/env bash
# Start local MySQL 8 for account-service (Docker).
#
# Usage:
#   source "$ROOT/scripts/lib/local-mysql.sh"
#   start_local_mysql
set -euo pipefail

: "${ROOT:?ROOT must be set before sourcing local-mysql.sh}"

LOCAL_MYSQL_CONTAINER="${LOCAL_MYSQL_CONTAINER:-anycode-account-mysql}"
LOCAL_MYSQL_PORT="${LOCAL_MYSQL_PORT:-3307}"
LOCAL_MYSQL_ROOT_PASSWORD="${LOCAL_MYSQL_ROOT_PASSWORD:-anycode}"
LOCAL_MYSQL_DATABASE="${LOCAL_MYSQL_DATABASE:-anycode}"
LOCAL_MYSQL_USER="${LOCAL_MYSQL_USER:-anycode}"
LOCAL_MYSQL_PASSWORD="${LOCAL_MYSQL_PASSWORD:-anycode}"
LOCAL_MYSQL_SCHEMA="${LOCAL_MYSQL_SCHEMA:-$ROOT/deploy/account-service/schema.mysql.sql}"

local_mysql_url() {
  echo "mysql://${LOCAL_MYSQL_USER}:${LOCAL_MYSQL_PASSWORD}@127.0.0.1:${LOCAL_MYSQL_PORT}/${LOCAL_MYSQL_DATABASE}"
}

local_mysql_url_for_docker() {
  echo "mysql://${LOCAL_MYSQL_USER}:${LOCAL_MYSQL_PASSWORD}@host.docker.internal:${LOCAL_MYSQL_PORT}/${LOCAL_MYSQL_DATABASE}"
}

start_local_mysql() {
  if ! command -v docker >/dev/null 2>&1; then
    echo "Docker is required for local MySQL (scripts/lib/local-mysql.sh)" >&2
    return 1
  fi
  if docker ps --format '{{.Names}}' | grep -qx "${LOCAL_MYSQL_CONTAINER}"; then
    echo "MySQL already running (${LOCAL_MYSQL_CONTAINER} on :${LOCAL_MYSQL_PORT})"
    return 0
  fi
  docker rm -f "${LOCAL_MYSQL_CONTAINER}" >/dev/null 2>&1 || true
  echo "==> starting ${LOCAL_MYSQL_CONTAINER} (mysql:8.0 on :${LOCAL_MYSQL_PORT})"
  docker run -d --name "${LOCAL_MYSQL_CONTAINER}" \
    -e "MYSQL_ROOT_PASSWORD=${LOCAL_MYSQL_ROOT_PASSWORD}" \
    -e "MYSQL_DATABASE=${LOCAL_MYSQL_DATABASE}" \
    -e "MYSQL_USER=${LOCAL_MYSQL_USER}" \
    -e "MYSQL_PASSWORD=${LOCAL_MYSQL_PASSWORD}" \
    -p "${LOCAL_MYSQL_PORT}:3306" \
    mysql:8.0 >/dev/null
  echo "Waiting for MySQL..."
  for _ in $(seq 1 45); do
    if docker exec "${LOCAL_MYSQL_CONTAINER}" mysqladmin ping -h127.0.0.1 \
      -uroot -p"${LOCAL_MYSQL_ROOT_PASSWORD}" --silent 2>/dev/null; then
      break
    fi
    sleep 2
  done
  if [[ ! -f "$LOCAL_MYSQL_SCHEMA" ]]; then
    echo "Missing schema: $LOCAL_MYSQL_SCHEMA" >&2
    return 1
  fi
  docker exec -i "${LOCAL_MYSQL_CONTAINER}" mysql -uroot -p"${LOCAL_MYSQL_ROOT_PASSWORD}" \
    "${LOCAL_MYSQL_DATABASE}" <"$LOCAL_MYSQL_SCHEMA"
  echo "MySQL ready: 127.0.0.1:${LOCAL_MYSQL_PORT} (db=${LOCAL_MYSQL_DATABASE})"
}
