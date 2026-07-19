#!/usr/bin/env bash
# Phase 1: workspace, project registration, skills, Python deps, dashboard.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HARNESS="$(cd "$(dirname "$0")" && pwd)"
BASE="${ANYCODE_E2E_BASE:-http://127.0.0.1:43180}"
WORKSPACE="${E2E_WORKSPACE:-${HOME}/.anycode/workspace/e2e-delivery}"
RUNTIME_HOME="${E2E_RUNTIME_HOME:-${WORKSPACE}-runtime-home}"

echo "==> ensure dashboard runtime binary"
DASH_BIN="${ANYCODE_DASHBOARD_BIN:-$ROOT/target/release/anycode-dashboard-serve}"
if [[ ! -x "$DASH_BIN" ]]; then
  (cd "$ROOT" && cargo build --release -p anycode-dashboard --bin anycode-dashboard-serve)
fi

echo "==> python office verify deps"
VENV="${HARNESS}/.venv"
if [[ ! -x "${VENV}/bin/python3" ]]; then
  python3 -m venv "$VENV"
fi
"${VENV}/bin/pip" install -q openpyxl python-docx python-pptx pypdf
echo "${VENV}/bin/python3" > "$HARNESS/out/python.txt"

echo "==> install skills-starter"
chmod +x "$ROOT/scripts/install-skills-starter.sh"
"$ROOT/scripts/install-skills-starter.sh" || true

echo "==> workspace layout"
rm -rf "$WORKSPACE/artifacts" "$WORKSPACE/out"
mkdir -p "$WORKSPACE"/{fixtures,artifacts,out}
cp -f "$HARNESS/shared/fixtures/"* "$WORKSPACE/fixtures/" 2>/dev/null || true
if [[ -d "$ROOT/scripts/eval/fixtures/bugfix-repo" ]]; then
  cp -rf "$ROOT/scripts/eval/fixtures/bugfix-repo" "$WORKSPACE/fixtures/bugfix-repo"
fi

echo "==> e2e-complex-repo (git seed for scenario 08)"
COMPLEX_REPO="$WORKSPACE/fixtures/e2e-complex-repo"
rm -rf "$COMPLEX_REPO"
cp -R "$HARNESS/shared/fixtures/e2e-complex-repo" "$COMPLEX_REPO"
(
  cd "$COMPLEX_REPO"
  git init -q
  git add -A
  git -c user.email="e2e@anycode.local" -c user.name="e2e-harness" commit -q -m "chore: seed sales-metrics with intentional bugs for e2e"
)
mkdir -p "$WORKSPACE/artifacts/executive_pack"

echo "==> e2e anycode config (turns=9999, security bypass)"
if [[ -n "${E2E_MODEL_PROFILE:-}" ]]; then
  node "$HARNESS/shared/utils/write_model_e2e_config.mjs" \
    "$E2E_MODEL_PROFILE" "$HARNESS/out/e2e-anycode.${E2E_MODEL_PROFILE}.config.json"
  cp -f \
    "$HARNESS/out/e2e-anycode.${E2E_MODEL_PROFILE}.config.json" \
    "$HARNESS/out/e2e-anycode.config.json"
else
  node "$HARNESS/shared/utils/write_e2e_config.mjs" "$HARNESS/out/e2e-anycode.config.json"
fi
mkdir -p "$WORKSPACE/.anycode"
cp -f "$HARNESS/out/e2e-anycode.config.json" "$WORKSPACE/.anycode/config.json"
mkdir -p "$RUNTIME_HOME/.anycode"
cp -f "$HARNESS/out/e2e-anycode.config.json" "$RUNTIME_HOME/.anycode/config.json"

echo "==> start dashboard"
pkill -f "anycode dashboard" 2>/dev/null || true
pkill -f "anycode-dashboard-serve" 2>/dev/null || true
sleep 1
nohup env \
  HOME="$RUNTIME_HOME" \
  ANYCODE_DASHBOARD_TEST_AUTH_BYPASS=1 \
  ANYCODE_DASHBOARD_RECORD=1 \
  ANYCODE_IGNORE_APPROVAL=1 \
  ANYCODE_MAX_AGENT_TURNS="${ANYCODE_MAX_AGENT_TURNS:-9999}" \
  ANYCODE_MAX_TOOL_CALLS="${ANYCODE_MAX_TOOL_CALLS:-50000}" \
  "$DASH_BIN" --host 127.0.0.1 --port 43180 >> /tmp/anycode-dashboard-e2e.log 2>&1 &
for _ in $(seq 1 60); do
  if curl -sf "${BASE}/api/health" >/dev/null 2>&1; then break; fi
  sleep 1
done
curl -sf "${BASE}/api/health" >/dev/null || { echo "dashboard not healthy" >&2; exit 1; }

echo "==> register e2e-delivery project"
E2E_WORKSPACE="$WORKSPACE" \
  E2E_PROJECT_NAME="${E2E_PROJECT_NAME:-e2e-delivery}" \
  node "$HARNESS/shared/utils/register_project.mjs"
