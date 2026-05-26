#!/usr/bin/env bash
# Post-deploy smoke for Digital Workbench (local/single-machine production).
set -euo pipefail

HOST="${ANYCODE_DASHBOARD_HOST:-127.0.0.1}"
PORT="${ANYCODE_DASHBOARD_PORT:-43180}"
BASE="http://${HOST}:${PORT}"

echo "== anyCode dashboard post-deploy smoke =="
echo "target: ${BASE}"

curl -sf "${BASE}/api/health" | grep -q '"ok"' && echo "✓ health"
curl -sf "${BASE}/api/bootstrap" | grep -q 'workbench_phase' && echo "✓ bootstrap"
curl -sf "${BASE}/api/settings/doctor" | grep -q '"checks"' && echo "✓ doctor"
curl -sf "${BASE}/api/security/approvals/summary" | grep -q 'pending_total' && echo "✓ approval summary"
curl -sf "${BASE}/api/overview" | grep -q 'sessions_total' && echo "✓ overview"

echo "All smoke checks passed."
