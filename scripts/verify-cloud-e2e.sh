#!/usr/bin/env bash
# Smoke-check local cloud stack: account-service + model-gateway paths.
set -euo pipefail

API="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:43200}"
GW="${ANYCODE_MODEL_GATEWAY_URL:-http://127.0.0.1:43210}"

echo "== health =="
curl -fsS "$API/health" | head -c 200
echo

echo "== register/login skipped (use existing session) =="
if [[ ! -f "$HOME/.anycode/cloud-session.json" ]]; then
  echo "WARN: no cloud-session.json — link desktop account first"
  exit 0
fi

TOKEN=$(python3 -c "import json, pathlib; print(json.loads(pathlib.Path('$HOME/.anycode/cloud-session.json').read_text())['access_token'])")
echo "== models catalog =="
curl -fsS -H "Authorization: Bearer $TOKEN" "$API/api/v1/models/catalog" | head -c 400
echo

echo "== gateway models =="
curl -fsS -H "Authorization: Bearer $TOKEN" "$GW/v1/models" | head -c 400
echo
echo "OK"
