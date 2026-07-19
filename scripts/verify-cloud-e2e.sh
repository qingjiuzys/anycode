#!/usr/bin/env bash
# Smoke-check local cloud stack: account-service + model-gateway paths.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
API="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:43200}"
GW="${ANYCODE_MODEL_GATEWAY_URL:-http://127.0.0.1:43210}"

echo "== health =="
curl -fsS "$API/health" | head -c 200
echo

echo "== register/login skipped (use existing session) =="
SESSION_FILE="${HOME}/.anycode/credentials/cloud-session.json"
LEGACY_SESSION="${HOME}/.anycode/cloud-session.json"
if [[ ! -f "${SESSION_FILE}" && -f "${LEGACY_SESSION}" ]]; then
  SESSION_FILE="${LEGACY_SESSION}"
fi
if [[ ! -f "${SESSION_FILE}" ]]; then
  echo "WARN: no cloud session — link desktop account first (${HOME}/.anycode/credentials/cloud-session.json)"
  exit 0
fi

TOKEN=$(python3 -c "import json, pathlib; print(json.loads(pathlib.Path('${SESSION_FILE}').read_text())['access_token'])")
echo "== models catalog =="
CATALOG_STATUS=$(curl -s -o /tmp/anycode-catalog.json -w "%{http_code}" -H "Authorization: Bearer $TOKEN" "$API/api/v1/models/catalog")
if [[ "$CATALOG_STATUS" == "401" ]] && [[ -x "${ROOT}/scripts/refresh-cloud-session.sh" ]]; then
  echo "token expired — refreshing session"
  "${ROOT}/scripts/refresh-cloud-session.sh" || true
  TOKEN=$(python3 -c "import json, pathlib; print(json.loads(pathlib.Path('${SESSION_FILE}').read_text())['access_token'])")
  CATALOG_STATUS=$(curl -s -o /tmp/anycode-catalog.json -w "%{http_code}" -H "Authorization: Bearer $TOKEN" "$API/api/v1/models/catalog")
fi
if [[ "$CATALOG_STATUS" != "200" ]]; then
  curl -fsS -H "Authorization: Bearer $TOKEN" "$API/api/v1/models/catalog" | head -c 400 || true
  echo
  exit 1
fi
head -c 400 /tmp/anycode-catalog.json
echo

echo "== gateway models =="
curl -fsS -H "Authorization: Bearer $TOKEN" "$GW/v1/models" | head -c 400
echo
echo "OK"
