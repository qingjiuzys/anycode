#!/usr/bin/env bash
set -euo pipefail
MODELS="${1:-}"
OUTPUT_DIR="${2:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=../_common/sandbox.sh
source "$BENCH_ROOT/_common/sandbox.sh"

ADAPTER="multipl-e"
IMAGE="anycode/bench-multipl-e:7e843e0"
DATA_DIR="$BENCH_ROOT/data/multipl-e"

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
  bash -lc 'python3 /opt/multipl-e/run_benchmark.py --languages python,rust,typescript,go,java --out /out'
