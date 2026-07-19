#!/usr/bin/env bash
# Agnes industry benchmark: HumanEval+ and MBPP+ via EvalPlus 0.3.1 (OpenAI-compatible API).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VENV="${ANYCODE_EVALPLUS_VENV:-$ROOT/test/benchmarks/.venv-evalplus}"
RUN_ID="${ANYCODE_BENCH_RUN_ID:-$(date +%Y%m%d-%H%M%S)-agnes-bench}"
OUT="$ROOT/test/results/$RUN_ID"
LOG="$OUT/run.log"
MODEL="${AGNES_BENCH_MODEL:-agnes-2.0-flash}"
BASE_URL="${AGNES_BENCH_BASE_URL:-https://apihub.agnes-ai.com/v1}"
DATASETS="${AGNES_BENCH_DATASETS:-humaneval,mbpp}"
BENCH_KEY_FILE="${ANYCODE_BENCH_KEY_FILE:-$HOME/.anycode/bench-subscription.key}"

mkdir -p "$OUT"
LOCK_DIR="$OUT/.run.lock"

acquire_run_lock() {
  if mkdir "$LOCK_DIR" 2>/dev/null; then
    printf '%s\n' "$$" > "$LOCK_DIR/pid"
    trap 'rm -rf "$LOCK_DIR"' EXIT INT TERM
    return
  fi
  local owner=""
  [[ -f "$LOCK_DIR/pid" ]] && owner="$(tr -dc '0-9' < "$LOCK_DIR/pid")"
  if [[ -n "$owner" ]] && kill -0 "$owner" 2>/dev/null; then
    echo "benchmark run_id=$RUN_ID already running as pid=$owner" >&2
    exit 3
  fi
  echo "removing stale benchmark lock for run_id=$RUN_ID" >&2
  rm -rf "$LOCK_DIR"
  mkdir "$LOCK_DIR"
  printf '%s\n' "$$" > "$LOCK_DIR/pid"
  trap 'rm -rf "$LOCK_DIR"' EXIT INT TERM
}

acquire_run_lock

resolve_api_key() {
  if [[ -n "${ANYCODE_BENCH_API_KEY:-}" ]]; then
    printf '%s' "$ANYCODE_BENCH_API_KEY"
    return
  fi
  if [[ -f "$BENCH_KEY_FILE" ]]; then
    tr -d '\n' < "$BENCH_KEY_FILE"
    return
  fi
  "$VENV/bin/python" - <<'PY'
import json, pathlib, sys
c = json.loads(pathlib.Path.home().joinpath(".anycode/config.json").read_text())
key = c.get("provider_credentials", {}).get("agnes") or c.get("api_key")
if not key:
    sys.exit("no API key: set ANYCODE_BENCH_API_KEY or ~/.anycode/bench-subscription.key")
print(key, end="")
PY
}

# Skip datasets that already have scored summaries (resume across restarts).
pending_datasets() {
  local all="$1" ds summary has_score
  local out=()
  IFS=',' read -r -a parts <<< "$all"
  for ds in "${parts[@]}"; do
    ds="$(echo "$ds" | xargs)"
    [[ -n "$ds" ]] || continue
    summary="$OUT/$ds/${ds}-summary.json"
    if [[ -f "$summary" ]] && "$VENV/bin/python" - <<PY 2>/dev/null
import json, sys
s=json.load(open("$summary"))
expected=s.get("expected_task_count")
complete=(
    s.get("model_pass_at_1_base") is not None
    and s.get("valid_generation_count") == expected
)
sys.exit(0 if complete else 1)
PY
    then
      echo "[skip] $ds already scored" >&2
      continue
    fi
    out+=("$ds")
  done
  if ((${#out[@]} == 0)); then
    echo ""
  else
    IFS=','; echo "${out[*]}"
  fi
}

exec > >(tee -a "$LOG") 2>&1

echo "==> Agnes benchmark run_id=$RUN_ID model=$MODEL"

if [[ ! -x "$VENV/bin/python" ]]; then
  python3.11 -m venv "$VENV"
fi
if ! "$VENV/bin/python" -c "import evalplus" 2>/dev/null; then
  echo "==> installing evalplus==0.3.1"
  "$VENV/bin/pip" install -q "evalplus==0.3.1"
fi

KEY="$(resolve_api_key)"
export OPENAI_API_KEY="$KEY"
export EVALPLUS_MAX_MEMORY_BYTES="${EVALPLUS_MAX_MEMORY_BYTES:--1}"

PENDING="$(pending_datasets "$DATASETS")"
if [[ -z "$PENDING" ]]; then
  echo "==> all datasets already scored; regenerating summary"
else
  DATASETS="$PENDING"
fi

IFS=',' read -r -a DS_ARR <<< "$DATASETS"
for dataset in "${DS_ARR[@]}"; do
  dataset="$(echo "$dataset" | xargs)"
  [[ -n "$dataset" ]] || continue
  echo ""
  echo "======== dataset=$dataset ========"
  ds_out="$OUT/$dataset"
  mkdir -p "$ds_out"
  ANYCODE_BENCH_ROOT="$ds_out" "$VENV/bin/python" "$ROOT/test/scripts/run_agnes_evalplus.py" \
    --dataset "$dataset" \
    --model "$MODEL" \
    --base-url "$BASE_URL" \
    --root "$ds_out" \
    2>&1 | tee "$ds_out/eval.log"
done

"$VENV/bin/python" "$ROOT/test/scripts/summarize_agnes_benchmark.py" --out "$OUT" --model "$MODEL"
echo "==> done: $OUT/benchmark-scores.md"
