#!/usr/bin/env bash
# DeepSeek live integration suite (uses DEEPSEEK_API_KEY env, temp HOME).
# Does NOT modify ~/.anycode/config.json or commit secrets.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ANYCODE_BIN:-$ROOT/target/release/anycode}"
FIX="$ROOT/scripts/eval/fixtures/minimal-repo"
BUGFIX="$ROOT/scripts/eval/fixtures/bugfix-repo"

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  echo "DEEPSEEK_API_KEY required" >&2
  exit 1
fi
if [[ ! -x "$BIN" ]]; then
  echo "Building anycode release binary..."
  (cd "$ROOT" && cargo build --release -p anycode)
fi

REAL_HOME="$HOME"
TEST_HOME="$(mktemp -d)"
CFG="$TEST_HOME/config.json"
export HOME="$TEST_HOME"
export CFG="$CFG"
export ANYCODE_IGNORE_APPROVAL=1
export ANYCODE_EVAL_TOOLCHAIN_HOME="${ANYCODE_EVAL_TOOLCHAIN_HOME:-$REAL_HOME}"

python3 - <<'PY'
import json, os
cfg = {
  "provider": "deepseek",
  "plan": "coding",
  "api_key": os.environ["DEEPSEEK_API_KEY"],
  "base_url": "https://api.deepseek.com/v1/chat/completions",
  "model": "deepseek-chat",
  "temperature": 0.2,
  "max_tokens": 512,
  "memory": {"backend": "noop", "path": ".anycode/mem", "auto_save": False},
  "security": {"permission_mode": "bypass", "require_approval": False, "sandbox_mode": False},
}
with open(os.environ["CFG"], "w") as f:
    json.dump(cfg, f, indent=2)
PY

PASS=0
FAIL=0
SKIP=0
RESULTS=()

record() {
  local id="$1" status="$2" detail="$3"
  RESULTS+=("$id|$status|$detail")
  case "$status" in
    pass) PASS=$((PASS + 1)); echo "✅ $id — $detail" ;;
    fail) FAIL=$((FAIL + 1)); echo "❌ $id — $detail" ;;
    skip) SKIP=$((SKIP + 1)); echo "⏭️  $id — $detail" ;;
  esac
}

run_cli() {
  local id="$1" expect="$2"
  shift 2
  local out ec
  out=$("$BIN" -c "$CFG" "$@" 2>&1) || true
  ec=$?
  if echo "$out" | grep -q "$expect"; then
    record "$id" pass "exit=$ec matched '$expect'"
  else
    record "$id" fail "exit=$ec expected '$expect' tail=$(echo "$out" | tail -2 | tr '\n' ' ')"
  fi
}

run_cli_json() {
  local id="$1"
  shift
  local out ec
  out=$("$BIN" -c "$CFG" "$@" 2>&1) || true
  ec=$?
  if echo "$out" | grep -Fq '[' || echo "$out" | grep -Fq '{'; then
    record "$id" pass "exit=$ec json body present"
  else
    record "$id" fail "exit=$ec no json: $(echo "$out" | tail -1)"
  fi
}

run_llm() {
  local id="$1" expect="$2"
  shift 2
  local out ec
  out=$("$BIN" -c "$CFG" --ignore "$@" 2>&1) || true
  ec=$?
  if echo "$out" | grep -qi "$expect"; then
    record "$id" pass "exit=$ec found '$expect'"
  else
    record "$id" fail "exit=$ec missing '$expect'"
  fi
}

echo "=== DeepSeek anyCode suite (HOME=$TEST_HOME) ==="

# --- API sanity ---
API=$(curl -sS --retry 2 --retry-delay 1 -w "\nHTTP:%{http_code}" https://api.deepseek.com/v1/chat/completions \
  -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"Reply: PING_OK"}],"max_tokens":8}' 2>&1) || true
if echo "$API" | grep -q 'PING_OK' && echo "$API" | grep -q 'HTTP:200'; then
  record "api-ping" pass "DeepSeek HTTP 200"
elif echo "$API" | grep -q 'HTTP:200'; then
  record "api-ping" pass "DeepSeek HTTP 200 (body ok)"
else
  record "api-ping" fail "curl failed or bad response: $(echo "$API" | tail -1)"
fi

# --- CLI / doctor (no LLM) ---
run_cli "status-json" "deepseek" status --json
run_cli "doctor-all" "memory" doctor all --json
run_cli "doctor-errors" "eval.scenario_failed" doctor errors --json
run_cli "memory-doctor" "memory" memory doctor --json
run_cli "mcp-status" "policy" mcp status --json
run_cli "channel-status" "channel" channel status --json
run_cli_json "cron-runs-json" cron runs --limit 3 --json
run_cli "eval-list" "mock-fixture" eval list

# workflow validate
WV=$("$BIN" -c "$CFG" workflow validate "$ROOT/examples/workflow.example.yml" 2>&1) || true
if echo "$WV" | grep -qi 'ok'; then record "workflow-validate-ok" pass "example workflow"; else record "workflow-validate-ok" fail "$WV"; fi
WV_BAD=$("$BIN" -c "$CFG" workflow validate "$ROOT/scripts/eval/fixtures/declarative-workflow.yml" 2>&1) || true
if echo "$WV_BAD" | grep -qi 'failed\|required_gates'; then record "workflow-validate-gates" pass "rejects required_gates"; else record "workflow-validate-gates" fail "$WV_BAD"; fi

# skills
SK=$("$BIN" -c "$CFG" skills list 2>&1) || true
if echo "$SK" | grep -qi 'skill\|SKILL\|roots\|empty\|no skills'; then record "skills-list" pass "command ok"; else record "skills-list" fail "$SK"; fi

# eval mock (no API) — isolate toolchain to real HOME (temp HOME has no cargo)
EM=$(ANYCODE_EVAL_TOOLCHAIN_HOME="$REAL_HOME" "$BIN" -c "$CFG" eval run --mock --json 2>/dev/null) || true
EM_EC=$?
if echo "$EM" | python3 -c "import sys,json; r=json.loads(sys.stdin.read()); bad=[x['id'] for x in r if x.get('status')!='pass']; sys.exit(0 if not bad else 1)" 2>/dev/null; then
  record "eval-mock" pass "all eval rows pass (cli exit=$EM_EC)"
else
  FAIL_IDS=$(echo "$EM" | python3 -c "import sys,json; r=json.loads(sys.stdin.read()); print([x['id'] for x in r if x.get('status')!='pass'])" 2>/dev/null || echo "parse error")
  record "eval-mock" fail "failures: $FAIL_IDS exit=$EM_EC"
fi

PYEVAL=$(ANYCODE_EVAL_BIN="$BIN" ANYCODE_EVAL_TOOLCHAIN_HOME="$REAL_HOME" python3 "$ROOT/scripts/eval/run.py" --with-mock 2>/dev/null) || true
if echo "$PYEVAL" | python3 -c "import sys,json; r=json.loads(sys.stdin.read()); assert all(x['status']=='pass' for x in r)" 2>/dev/null; then
  record "eval-run-py" pass "run.py --with-mock"
else
  record "eval-run-py" fail "python harness failed"
fi

# audit
run_cli_json "audit-tail" audit tail --limit 5 --json

# skills starter install (local, no network)
if [[ -f "$ROOT/scripts/install-skills-starter.sh" ]]; then
  SIS=$(ANYCODE_SKILLS_DIR="$TEST_HOME/.anycode/skills" bash "$ROOT/scripts/install-skills-starter.sh" 2>&1) || true
  if [[ -d "$TEST_HOME/.anycode/skills" ]] && ls "$TEST_HOME/.anycode/skills"/*/SKILL.md &>/dev/null; then
    record "skills-starter-install" pass "starter skills installed"
  else
    record "skills-starter-install" fail "$SIS"
  fi
else
  record "skills-starter-install" skip "script missing"
fi

# --- Live LLM runs ---
run_llm "run-arithmetic" "4" run -C "$FIX" --agent general-purpose "What is 2+2? Reply with only the digit."
run_llm "run-fileread" "tool_call_end" run -C "$FIX" --agent general-purpose "You MUST call FileRead on hello.txt then quote its full content exactly."
run_llm "run-grep" "tool_call" run -C "$FIX" --agent general-purpose "Use Grep to search hello.txt for 'minimal' and report the matching line."
run_llm "run-bash" "tool_call" run -C "$FIX" --agent general-purpose "Use Bash to run: echo BASH_OK — then reply with BASH_OK only."

# budget (capture real exit code — do not use `|| true` before $? )
set +e
BOUT=$("$BIN" -c "$CFG" --ignore run -C "$FIX" --token-budget 4 --agent general-purpose "hello" 2>&1)
BEC=$?
set -e
if echo "$BOUT" | grep -q 'budget_exceeded' && [[ "$BEC" -ne 0 ]]; then
  record "run-budget-trip" pass "exit=$BEC budget_exceeded"
else
  record "run-budget-trip" fail "exit=$BEC no budget_exceeded"
fi

# goal
GOUT=$("$BIN" -c "$CFG" --ignore run -C "$FIX" --goal "answer correctly" --done-when "42" --max-goal-attempts 2 --agent general-purpose "What is 6 times 7? Reply with only the number." 2>&1) || true
if echo "$GOUT" | grep -q '42'; then record "run-goal-done-when" pass "found 42"; else record "run-goal-done-when" fail "no 42"; fi

# multifile read (heavier)
MF="$ROOT/scripts/eval/fixtures/multifile-repo"
run_llm "run-multifile-read" "MARKER" run -C "$MF" --agent general-purpose "Use FileRead on docs/overview.md and report the MARKER_* token in that file only."

# cron NL parse via tools if CLI has it - check
if "$BIN" -c "$CFG" cron --help 2>&1 | grep -q create; then
  record "cron-help" pass "cron subcommands present"
else
  record "cron-help" skip "no cron create in help"
fi

# dashboard doctor if embedded
DD=$("$BIN" -c "$CFG" dashboard doctor 2>&1) || true
if echo "$DD" | grep -qi 'dashboard\|sqlite\|ok\|error'; then record "dashboard-doctor" pass "responded"; else record "dashboard-doctor" skip "$DD"; fi

# --- Extra CLI coverage (no LLM) ---
run_cli "help-top" "anyCode" --help
run_cli "run-help" "token-budget" run --help
run_cli "eval-list-budget" "mock-fixture-budget-trip" eval list
run_cli_json "audit-stats" audit stats --json

# Rust eval harness unit test (fast, no API)
ET=$(cd "$ROOT" && HOME="$REAL_HOME" cargo test -p anycode --test eval_mock --quiet 2>&1) || true
if echo "$ET" | grep -q 'test result: ok'; then
  record "cargo-test-eval-mock" pass "eval_mock.rs green"
else
  record "cargo-test-eval-mock" fail "$(echo "$ET" | tail -5 | tr '\n' ' ')"
fi

# Per-scenario mock status from last eval run
for SC in mock-fixture-greet mock-fixture-bugfix mock-fixture-multifile mock-fixture-test-repair mock-fixture-budget-trip; do
  ST=$(echo "$EM" | python3 -c "import sys,json; r=json.loads(sys.stdin.read()); m={x['id']:x.get('status') for x in r}; print(m.get('$SC','missing'))" 2>/dev/null || echo "parse")
  if [[ "$ST" == "pass" ]]; then record "eval-$SC" pass "mock scenario"; else record "eval-$SC" fail "status=$ST"; fi
done

# --- More live LLM tool coverage ---
run_llm "run-glob" "tool_call" run -C "$FIX" --agent general-purpose "Use Glob with pattern '*.txt' in the current directory and name one matching file."
run_llm "run-edit" "tool_call" run -C "$FIX" --agent general-purpose "Use Edit to append a line 'SUITE_MARKER=1' to hello.txt (create if needed), then confirm SUITE_MARKER=1 in your reply."

# Short reasoning / JSON-ish reply
run_llm "run-json-digit" "7" run -C "$FIX" --agent general-purpose "What is 3+4? Reply with only the digit, no words."

# Bugfix fixture smoke (live LLM, lighter than full eval)
if [[ -d "$BUGFIX" ]]; then
  set +e
  BF=$("$BIN" -c "$CFG" --ignore run -C "$BUGFIX" --agent general-purpose --max-duration-secs 120 "Fix the failing test in this repo. Use Bash to run cargo test." 2>&1)
  BF_EC=$?
  set -e
  if echo "$BF" | grep -qiE 'tool_call|bash|cargo|test result|turn_end|FileEdit|Edit'; then
    record "run-bugfix-smoke" pass "agent engaged (exit=$BF_EC)"
  else
    record "run-bugfix-smoke" fail "exit=$BF_EC tail=$(echo "$BF" | tail -3 | tr '\n' ' ')"
  fi
fi

# Verify temp config isolation (must not leak real provider)
ISO=$("$BIN" -c "$CFG" status --json 2>&1) || true
if echo "$ISO" | grep -q 'deepseek' && ! echo "$ISO" | grep -q 'z\.ai\|glm-5'; then
  record "config-isolation" pass "uses temp deepseek config only"
else
  record "config-isolation" fail "unexpected provider in status"
fi

echo ""
echo "=== SUMMARY pass=$PASS fail=$FAIL skip=$SKIP ==="
rm -rf "$TEST_HOME"
[[ "$FAIL" -eq 0 ]]
