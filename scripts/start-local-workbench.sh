#!/usr/bin/env bash
# Start local Workbench (dashboard-serve + embedded UI) for dev/e2e.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
anycode_apply_build_target_exports

PORT="${ANYCODE_DASHBOARD_DEV_PORT:-43181}"
HOST="${ANYCODE_DASHBOARD_HOST:-127.0.0.1}"
PROFILE="${ANYCODE_DASHBOARD_PROFILE:-release-local}"
PID_FILE="${TMPDIR:-/tmp}/anycode-workbench-${PORT}.pid"
LOG_FILE="${TMPDIR:-/tmp}/anycode-workbench-${PORT}.log"
BIN="$ROOT/target/${PROFILE}/anycode-dashboard-serve"

stop_workbench() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      sleep 0.5
    fi
    rm -f "$PID_FILE"
  fi
  if lsof -ti ":${PORT}" >/dev/null 2>&1; then
    lsof -ti ":${PORT}" | xargs kill 2>/dev/null || true
    sleep 0.5
  fi
}

start_workbench() {
  export ANYCODE_ACCOUNT_API_URL="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:43200}"
  export ANYCODE_ACCOUNT_PORTAL_URL="${ANYCODE_ACCOUNT_PORTAL_URL:-http://127.0.0.1:43200}"
  export ANYCODE_MODEL_GATEWAY_URL="${ANYCODE_MODEL_GATEWAY_URL:-http://127.0.0.1:43210}"
  export ANYCODE_DASHBOARD_EMBEDDED_DESKTOP="${ANYCODE_DASHBOARD_EMBEDDED_DESKTOP:-1}"
  export ANYCODE_IGNORE_APPROVAL="${ANYCODE_IGNORE_APPROVAL:-1}"

  if [[ ! -x "$BIN" ]]; then
    echo "==> building anycode-dashboard-serve (profile=$PROFILE)"
    cargo build --profile "$PROFILE" -p anycode-dashboard --bin anycode-dashboard-serve
  fi

  "$ROOT/scripts/start-local-gateway.sh" start

  stop_workbench
  nohup "$BIN" --host "$HOST" --port "$PORT" >>"$LOG_FILE" 2>&1 &
  echo $! >"$PID_FILE"
  for _ in $(seq 1 30); do
    if curl -sf "http://${HOST}:${PORT}/api/health" >/dev/null 2>&1; then
      echo "workbench ready: http://${HOST}:${PORT}/ (pid $(cat "$PID_FILE"), log $LOG_FILE)"
      return 0
    fi
    sleep 0.5
  done
  echo "workbench failed to start — see $LOG_FILE" >&2
  tail -30 "$LOG_FILE" >&2 || true
  return 1
}

case "${1:-start}" in
  start) start_workbench ;;
  stop) stop_workbench; echo "stopped" ;;
  restart) stop_workbench; start_workbench ;;
  status)
    if curl -sf "http://${HOST}:${PORT}/api/health" >/dev/null 2>&1; then
      echo "up http://${HOST}:${PORT}/"
    else
      echo "down"
      exit 1
    fi
    ;;
  *)
    echo "Usage: $0 [start|stop|restart|status]" >&2
    exit 1
    ;;
esac
