#!/usr/bin/env bash
# Refresh ~/.anycode/credentials/cloud-session.json via account-service device refresh.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
API="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:43200}"
SESSION_FILE="${HOME}/.anycode/credentials/cloud-session.json"
LEGACY_SESSION="${HOME}/.anycode/cloud-session.json"

if [[ ! -f "${SESSION_FILE}" && -f "${LEGACY_SESSION}" ]]; then
  SESSION_FILE="${LEGACY_SESSION}"
fi
if [[ ! -f "${SESSION_FILE}" ]]; then
  echo "No cloud session at ${SESSION_FILE} — link device first." >&2
  exit 1
fi

REFRESH="$(python3 -c "import json, pathlib; print(json.loads(pathlib.Path('${SESSION_FILE}').read_text()).get('refresh_token',''))")"
if [[ -z "${REFRESH}" ]]; then
  echo "Session file missing refresh_token — re-link device." >&2
  exit 1
fi

RESP="$(curl -s -w "\n%{http_code}" -X POST "${API}/api/v1/devices/refresh" \
  -H 'Content-Type: application/json' \
  -d "{\"refresh_token\":\"${REFRESH}\"}")"
BODY="$(echo "$RESP" | sed '$d')"
STATUS="$(echo "$RESP" | tail -1)"
if [[ "$STATUS" != "200" ]]; then
  echo "Refresh failed (${STATUS}): ${BODY}" >&2
  if [[ -x "${ROOT}/scripts/dev-auto-link.sh" ]]; then
    echo "Falling back to dev-auto-link.sh" >&2
    "${ROOT}/scripts/dev-auto-link.sh"
    exit 0
  fi
  exit 1
fi

python3 -c "
import json, sys
from pathlib import Path
data = json.loads(sys.argv[1])
out = {
    'access_token': data['access_token'],
    'refresh_token': data['refresh_token'],
    'user_email': data.get('user', {}).get('email'),
    'gateway_url': data.get('gateway_url'),
}
Path(sys.argv[2]).write_text(json.dumps(out, indent=2) + '\n')
print('Refreshed:', out.get('user_email') or '(ok)')
print('Gateway:', out.get('gateway_url') or '(default)')
" "$BODY" "$SESSION_FILE"
