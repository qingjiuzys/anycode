#!/usr/bin/env bash
set -euo pipefail
MODELS="${1:-}"
OUTPUT_DIR="${2:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=../_common/sandbox.sh
source "$BENCH_ROOT/_common/sandbox.sh"

ADAPTER="ds1000"
IMAGE="anycode/bench-ds1000:c4e8f12"
DATA_DIR="$BENCH_ROOT/data/ds-1000"

mkdir -p "${OUTPUT_DIR:-/tmp}"

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build -t "$IMAGE" -f "$SCRIPT_DIR/Dockerfile" "$SCRIPT_DIR"
fi

if [[ -z "$MODELS" ]]; then
  echo "[${ADAPTER}] preflight ok (no models — skipping generation)"
  echo '{"adapter":"'"$ADAPTER"'","status":"preflight_ok"}' > "${OUTPUT_DIR:-/tmp}/${ADAPTER}.json"
  exit 0
fi

mkdir -p "$OUTPUT_DIR"
anycode_bench_run_sandboxed \
  -v "$DATA_DIR:/data:ro" \
  -v "$OUTPUT_DIR:/out:rw" \
  "$IMAGE" \
  bash -lc 'python3 /opt/ds1000/run_eval.py --data /data --out /out'
