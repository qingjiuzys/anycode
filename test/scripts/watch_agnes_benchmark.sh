#!/usr/bin/env bash
# Watch Agnes benchmark until benchmark-scores.md is complete; auto-restart on exit.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUN_ID="${ANYCODE_BENCH_RUN_ID:-20260710-162747-agnes-bench}"
OUT="$ROOT/test/results/$RUN_ID"
LOG="$OUT/watchdog.log"
LOCK_DIR="$OUT/.run.lock"
INTERVAL="${ANYCODE_BENCH_WATCH_INTERVAL:-90}"
MAX_IDLE_ROUNDS="${ANYCODE_BENCH_MAX_IDLE_ROUNDS:-3}"

mkdir -p "$OUT"

is_complete() {
  local scores="$OUT/benchmark-scores.md"
  [[ -f "$scores" ]] || return 1
  [[ -f "$OUT/humaneval/humaneval-summary.json" ]] || return 1
  [[ -f "$OUT/mbpp/mbpp-summary.json" ]] || return 1
  "$ROOT/test/benchmarks/.venv-evalplus/bin/python" - <<PY
import json, sys
for ds in ("humaneval", "mbpp"):
    s = json.load(open(f"$OUT/{ds}/{ds}-summary.json"))
    if (
        s.get("model_pass_at_1_base") is None
        or s.get("valid_generation_count") != s.get("expected_task_count")
    ):
        sys.exit(1)
sys.exit(0)
PY
}

progress_line() {
  local he mbpp
  he=$(valid_sample_count "$OUT/humaneval/evalplus_results/humaneval/agnes-2.0-flash_openai_temp_0.0.jsonl")
  mbpp=0
  if [[ -f "$OUT/mbpp/evalplus_results/mbpp/agnes-2.0-flash_openai_temp_0.0.jsonl" ]]; then
    mbpp=$(valid_sample_count "$OUT/mbpp/evalplus_results/mbpp/agnes-2.0-flash_openai_temp_0.0.jsonl")
  fi
  echo "[$(date -Iseconds)] humaneval_valid=${he}/164 mbpp_valid=${mbpp}/378 running=$(run_is_active && echo 1 || echo 0)"
}

valid_sample_count() {
  local path="$1"
  [[ -f "$path" ]] || { echo 0; return; }
  "$ROOT/test/benchmarks/.venv-evalplus/bin/python" - "$path" <<'PY'
import json, sys
count = 0
for line in open(sys.argv[1], encoding="utf-8"):
    try:
        value = json.loads(line)
    except json.JSONDecodeError:
        continue
    if isinstance(value.get("solution"), str) and value["solution"].strip():
        count += 1
print(count)
PY
}

run_is_active() {
  local pid=""
  [[ -f "$LOCK_DIR/pid" ]] && pid="$(tr -dc '0-9' < "$LOCK_DIR/pid")"
  [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null
}

exec >>"$LOG" 2>&1
echo "==> watchdog start run_id=$RUN_ID interval=${INTERVAL}s"

idle=0
while ! is_complete; do
  progress_line
  if run_is_active; then
    idle=0
  else
    idle=$((idle + 1))
    echo "[$(date -Iseconds)] benchmark not running (idle=$idle); restarting..."
    ANYCODE_BENCH_RUN_ID="$RUN_ID" bash "$ROOT/test/scripts/run_agnes_benchmark.sh" &
    sleep 10
    if ! run_is_active; then
      echo "[$(date -Iseconds)] restart failed"
    fi
    if (( idle >= MAX_IDLE_ROUNDS )); then
      echo "[$(date -Iseconds)] too many idle restarts without progress; continuing anyway"
      idle=0
    fi
  fi
  sleep "$INTERVAL"
done

echo "[$(date -Iseconds)] COMPLETE"
cat "$OUT/benchmark-scores.md"
