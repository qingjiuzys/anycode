#!/usr/bin/env bash
# Ensure MiniCPM5-1B is served via SGLang with the official minicpm5 tool-call parser.
# See: https://github.com/OpenBMB/MiniCPM/blob/main/docs/deployment/sglang.md
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PORT="${SGLANG_PORT:-30000}"
HOST="${SGLANG_HOST:-127.0.0.1}"
MODEL_PATH="${SGLANG_MODEL_PATH:-openbmb/MiniCPM5-1B}"
SERVED_NAME="${SGLANG_SERVED_MODEL:-MiniCPM5-1B}"
CTX_LEN="${SGLANG_CTX_LEN:-32768}"
MEM_FRAC="${SGLANG_MEM_FRAC:-0.85}"
TOOL_PARSER="${SGLANG_TOOL_PARSER:-minicpm5}"
VENV="${ANYCODE_SGLANG_VENV:-$ROOT/test/e2e-delivery-chain/.venv-sglang}"
LOG="${ANYCODE_SGLANG_LOG:-/tmp/anycode-sglang-minicpm5.log}"
PID_FILE="${ANYCODE_SGLANG_PID_FILE:-/tmp/anycode-sglang-minicpm5.pid}"

if [[ -n "${SGLANG_BASE_URL:-}" ]]; then
  BASE_URL="${SGLANG_BASE_URL%/}"
  HEALTH_URL="${BASE_URL%/v1/chat/completions}"
  HEALTH_URL="${HEALTH_URL%/v1}"
  HEALTH_URL="${HEALTH_URL}/health"
else
  HEALTH_URL="http://${HOST}:${PORT}/health"
fi

wait_healthy() {
  local tries="${1:-120}"
  for _ in $(seq 1 "$tries"); do
    if curl -sf "$HEALTH_URL" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  return 1
}

if wait_healthy 3; then
  echo "[ensure_sglang] healthy: $HEALTH_URL"
  exit 0
fi

if [[ -n "${SGLANG_BASE_URL:-}" ]]; then
  echo "[ensure_sglang] remote SGLANG_BASE_URL set but health check failed: $HEALTH_URL" >&2
  exit 1
fi

if ! command -v python3 >/dev/null; then
  echo "[ensure_sglang] python3 not found" >&2
  exit 1
fi

if [[ ! -d "$VENV" ]]; then
  echo "[ensure_sglang] creating venv at $VENV"
  python3 -m venv "$VENV"
fi
# shellcheck disable=SC1091
source "$VENV/bin/activate"

if ! python -c "import torch; exit(0 if torch.cuda.is_available() else 1)" >/dev/null 2>&1; then
  if ! python -c "import torch" >/dev/null 2>&1; then
    echo "[ensure_sglang] installing torch to probe CUDA..."
    pip install -q torch
  fi
  if ! python -c "import torch; exit(0 if torch.cuda.is_available() else 1)" >/dev/null 2>&1; then
    echo "[ensure_sglang] ERROR: CUDA not available — SGLang MiniCPM5-1B needs an NVIDIA GPU." >&2
    echo "[ensure_sglang] Remote: export SGLANG_BASE_URL=http://<gpu-host>:30000/v1/chat/completions" >&2
    echo "[ensure_sglang] Fallback: export LOCAL_1B_BACKEND=ollama" >&2
    exit 1
  fi
fi

if ! python -c "import sglang" >/dev/null 2>&1; then
  echo "[ensure_sglang] installing SGLang from main (minicpm5 parser not in pip release yet)"
  pip install -q -U pip wheel
  if ! pip install -q "git+https://github.com/sgl-project/sglang.git@main#subdirectory=python"; then
    echo "[ensure_sglang] git install failed — retry after network fix or pre-install:" >&2
    echo "  pip install \"git+https://github.com/sgl-project/sglang.git@main#subdirectory=python\"" >&2
    exit 1
  fi
fi

DTYPE="${SGLANG_DTYPE:-bfloat16}"

export VLLM_WORKER_MULTIPROC_METHOD="${VLLM_WORKER_MULTIPROC_METHOD:-spawn}"
export SGLANG_ALLOW_OVERWRITE_LONGER_CONTEXT_LEN="${SGLANG_ALLOW_OVERWRITE_LONGER_CONTEXT_LEN:-1}"
export SGLANG_DISABLE_CUDNN_CHECK="${SGLANG_DISABLE_CUDNN_CHECK:-1}"

if [[ -f "$PID_FILE" ]]; then
  old_pid="$(cat "$PID_FILE" 2>/dev/null || true)"
  if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
    echo "[ensure_sglang] server already starting (pid=$old_pid), waiting..."
    if wait_healthy 180; then
      echo "[ensure_sglang] healthy after wait"
      exit 0
    fi
    echo "[ensure_sglang] stale pid $old_pid — restarting" >&2
    kill "$old_pid" 2>/dev/null || true
  fi
fi

echo "[ensure_sglang] launching server model=$MODEL_PATH port=$PORT parser=$TOOL_PARSER ctx=$CTX_LEN"
nohup python -m sglang.launch_server \
  --model-path "$MODEL_PATH" \
  --served-model-name "$SERVED_NAME" \
  --dtype "$DTYPE" \
  --context-length "$CTX_LEN" \
  --mem-fraction-static "$MEM_FRAC" \
  --tool-call-parser "$TOOL_PARSER" \
  --host "$HOST" \
  --port "$PORT" \
  >>"$LOG" 2>&1 &
echo $! >"$PID_FILE"

if wait_healthy 180; then
  echo "[ensure_sglang] ready http://${HOST}:${PORT}/v1/chat/completions (log: $LOG)"
  exit 0
fi

echo "[ensure_sglang] server failed to become healthy — tail $LOG:" >&2
tail -40 "$LOG" >&2 || true
exit 1
