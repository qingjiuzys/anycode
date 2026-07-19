#!/usr/bin/env bash
# Complex eval: e2e-delivery 08 + benchmark dataset download + model probes.
# Models: agnes (project ~/.anycode keys), local-1b (SGLang MiniCPM5-1B + minicpm5 parser).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HARNESS="$ROOT/test/e2e-delivery-chain"
RUN_ID="$(date +%Y%m%d-%H%M%S)-complex"
OUT="$ROOT/test/results/$RUN_ID"
LOG="$OUT/run.log"
mkdir -p "$OUT"

exec > >(tee -a "$LOG") 2>&1

echo "==> complex model eval run_id=$RUN_ID"

# --- Phase 0: prerequisites ---
LOCAL_1B_BACKEND="${LOCAL_1B_BACKEND:-sglang}"
if [[ "$LOCAL_1B_BACKEND" == "sglang" ]]; then
  chmod +x "$ROOT/test/scripts/ensure_sglang_minicpm5.sh"
  "$ROOT/test/scripts/ensure_sglang_minicpm5.sh"
else
  command -v ollama >/dev/null || { echo "ollama not found"; exit 1; }
  curl -sf http://127.0.0.1:11434/api/tags >/dev/null || { echo "ollama API not up — run: ollama serve"; exit 1; }
  if ! curl -sf http://127.0.0.1:11434/api/tags | grep -qi 'minicpm5-1b-e2e'; then
    echo "==> creating ollama minicpm5-1b-e2e (num_ctx=32768 for agent harness)"
    cat > /tmp/anycode-Modelfile.minicpm5-e2e <<'EOF'
FROM minicpm5-1b:latest
PARAMETER num_ctx 32768
EOF
    ollama create minicpm5-1b-e2e -f /tmp/anycode-Modelfile.minicpm5-e2e || echo "WARN: minicpm5-1b-e2e create failed"
  fi
fi

python3 -c "import json,pathlib; c=json.loads(pathlib.Path.home().joinpath('.anycode/config.json').read_text()); assert c.get('provider_credentials',{}).get('agnes') or c.get('api_key'), 'agnes key missing'" \
  || { echo "agnes key missing in ~/.anycode/config.json"; exit 1; }

if [[ ! -x "$ROOT/target/release/anycode-dashboard-serve" ]]; then
  (cd "$ROOT" && cargo build --release -p anycode-dashboard --bin anycode-dashboard-serve)
fi

# --- Phase 1: benchmark dataset downloads ---
BENCH_ROOT="$ROOT/test/benchmarks"
if [[ "${COMPLEX_EVAL_ONLY:-0}" != "1" ]]; then
  export ANYCODE_BENCH_ALLOW_BOOTSTRAP=1
  for adapter in humaneval mbpp multipl-e ds1000 canitedit codexglue codecontests; do
    if [[ -x "$BENCH_ROOT/$adapter/download.sh" ]]; then
      echo "==> downloading benchmark: $adapter"
      "$BENCH_ROOT/$adapter/download.sh" || echo "WARN: $adapter download failed (continuing)"
    fi
  done
fi

# --- Phase 2: prepare e2e harness ---
cd "$HARNESS"

run_complex_for_profile() {
  local profile="$1"
  local timeout_ms="${2:-7200000}"
  local profile_out="$OUT/complex-$profile"
  local workspace="${HOME}/.anycode/workspace/e2e-delivery/$RUN_ID/$profile"
  echo ""
  echo "======== COMPLEX 08 profile=$profile timeout_ms=$timeout_ms ========"
  export E2E_MODEL_PROFILE="$profile"
  export E2E_EVAL_RUN_ID="$RUN_ID-$profile"
  export E2E_SESSION_TIMEOUT_MS="$timeout_ms"
  export E2E_WORKSPACE="$workspace"
  export E2E_PROJECT_NAME="e2e-delivery-$RUN_ID-$profile"

  rm -rf "$profile_out"
  mkdir -p "$profile_out"
  rm -f \
    "$HARNESS/out/08-complex-delivery.json" \
    "$HARNESS/out/08-verify.json" \
    "$HARNESS/out/08-complex-delivery.brief.md" \
    "$HARNESS/out/.e2e-artifact-owner.json"

  E2E_WORKSPACE="$workspace" E2E_PROJECT_NAME="$E2E_PROJECT_NAME" SKIP_RESET=1 bash bootstrap.sh
  export SKIP_BOOTSTRAP=1
  export SKIP_RESET=1
  node "$HARNESS/shared/utils/write_model_e2e_config.mjs" "$profile" "$HARNESS/out/e2e-anycode.$profile.config.json"
  cp -f "$HARNESS/out/e2e-anycode.$profile.config.json" "$HARNESS/out/e2e-anycode.config.json"
  cp -f "$HARNESS/out/e2e-anycode.$profile.config.json" "$workspace/.anycode/config.json"

  if COMPLEX_ONLY=1 node run_all.mjs; then
    python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("score", 0))' \
      "$HARNESS/out/08-verify.json" > "$profile_out/agent_task_score.txt"
    echo "PASS complex-$profile" | tee "$profile_out/status.txt"
    node generate_complex_audit.mjs
    cp -f "$HARNESS/out/08-verify.json" "$profile_out/" 2>/dev/null || true
    cp -f "$HARNESS/out/08-complex-delivery.json" "$profile_out/" 2>/dev/null || true
    cp -f "$HARNESS/out/.e2e-artifact-owner.json" "$profile_out/" 2>/dev/null || true
    cp -f "$HARNESS/out/"*complex* "$profile_out/" 2>/dev/null || true
    return 0
  else
    python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("score", 0))' \
      "$HARNESS/out/08-verify.json" > "$profile_out/agent_task_score.txt" 2>/dev/null || echo 0 > "$profile_out/agent_task_score.txt"
    echo "FAIL complex-$profile" | tee "$profile_out/status.txt"
    cp -f "$HARNESS/out/08-verify.json" "$profile_out/" 2>/dev/null || true
    cp -f "$HARNESS/out/08-complex-delivery.json" "$profile_out/" 2>/dev/null || true
    cp -f "$HARNESS/out/.e2e-artifact-owner.json" "$profile_out/" 2>/dev/null || true
    return 1
  fi
}

AGNES_OK=0
LOCAL_OK=0

# --- Phase 3: Agnes complex delivery (primary) ---
if run_complex_for_profile "agnes" 7200000; then AGNES_OK=1; fi

# --- Phase 4: Local 1B via SGLang (shorter budget) ---
if run_complex_for_profile "local-1b" 3600000; then LOCAL_OK=1; fi
AGNES_SCORE="$(cat "$OUT/complex-agnes/agent_task_score.txt" 2>/dev/null || echo 0)"
LOCAL_SCORE="$(cat "$OUT/complex-local-1b/agent_task_score.txt" 2>/dev/null || echo 0)"

# --- Phase 5: benchmark adapter preflight + agnes stub run ---
if [[ "${COMPLEX_EVAL_ONLY:-0}" != "1" ]]; then
  BENCH_OUT="$OUT/benchmarks"
  mkdir -p "$BENCH_OUT"
  for adapter in humaneval mbpp; do
    echo "==> benchmark adapter: $adapter"
    "$BENCH_ROOT/$adapter/run_adapter.sh" "agnes" "$BENCH_OUT/$adapter" || true
  done
fi

# --- Phase 6: dashboard model probes (if dashboard up) ---
if [[ "${COMPLEX_EVAL_ONLY:-0}" != "1" ]] && curl -sf http://127.0.0.1:43180/api/health >/dev/null 2>&1; then
  export ANYCODE_EVAL_DASHBOARD_URL=http://127.0.0.1:43180
  python3 "$ROOT/test/run.py" --profile full --models local-1b,agnes 2>&1 | tee "$OUT/model-probes.log" || true
fi

# --- Summary ---
{
  echo "# Complex model eval — $RUN_ID"
  echo ""
  echo "| Model | agent_task_score (Complex 08) |"
  echo "|-------|-------------------------------|"
  echo "| agnes | ${AGNES_SCORE}/100 ($([[ $AGNES_OK -eq 1 ]] && echo PASS || echo FAIL)) |"
  echo "| local-1b (${LOCAL_1B_BACKEND}) | ${LOCAL_SCORE}/100 ($([[ $LOCAL_OK -eq 1 ]] && echo PASS || echo FAIL)) |"
  echo ""
  echo "Log: $LOG"
  echo "Artifacts: $OUT"
} | tee "$OUT/summary.md"

if [[ $AGNES_OK -eq 1 ]]; then
  exit 0
fi
exit 1
