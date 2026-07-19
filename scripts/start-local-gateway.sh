#!/usr/bin/env bash
# Start local model-gateway on :43210 (background). Requires account-service on :43200.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
anycode_apply_build_target_exports

GW_PORT="${MODEL_GATEWAY_PORT:-43210}"
GW_HOST="${MODEL_GATEWAY_HOST:-127.0.0.1}"
PID_FILE="${TMPDIR:-/tmp}/anycode-model-gateway.pid"
LOG_FILE="${TMPDIR:-/tmp}/anycode-model-gateway.log"
GW_BIN="$ROOT/crates/model-gateway/target/release/anycode-model-gateway"

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
  echo "WARN: no AGNES_API_KEY — set env or ~/.anycode/secrets/agnes.txt" >&2
  return 1
}

stop_gateway() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      sleep 0.5
    fi
    rm -f "$PID_FILE"
  fi
  if lsof -ti ":${GW_PORT}" >/dev/null 2>&1; then
    lsof -ti ":${GW_PORT}" | xargs kill 2>/dev/null || true
    sleep 0.5
  fi
}

start_gateway() {
  load_agnes_upstream_key || true
  export ANYCODE_ACCOUNT_API_URL="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:43200}"
  export MODEL_GATEWAY_HOST="${GW_HOST}"
  export MODEL_GATEWAY_PORT="${GW_PORT}"

  if [[ ! -x "$GW_BIN" ]]; then
    echo "==> building anycode-model-gateway"
    (cd "$ROOT/crates/model-gateway" && cargo build --release)
  fi

  stop_gateway
  nohup "$GW_BIN" >>"$LOG_FILE" 2>&1 &
  echo $! >"$PID_FILE"
  for _ in $(seq 1 20); do
    if curl -sf "http://${GW_HOST}:${GW_PORT}/health" >/dev/null 2>&1; then
      echo "model-gateway ready: http://${GW_HOST}:${GW_PORT}/ (pid $(cat "$PID_FILE"), log $LOG_FILE)"
      return 0
    fi
    sleep 0.5
  done
  echo "model-gateway failed to become healthy — see $LOG_FILE" >&2
  tail -20 "$LOG_FILE" >&2 || true
  return 1
}

case "${1:-start}" in
  start) start_gateway ;;
  stop) stop_gateway; echo "stopped" ;;
  restart) stop_gateway; start_gateway ;;
  status)
    if curl -sf "http://${GW_HOST}:${GW_PORT}/health" >/dev/null 2>&1; then
      echo "up http://${GW_HOST}:${GW_PORT}/"
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
