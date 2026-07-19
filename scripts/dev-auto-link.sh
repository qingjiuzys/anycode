#!/usr/bin/env bash
# Auto device-link for local dev (login + approve + poll) without browser.
set -euo pipefail

API="${ANYCODE_ACCOUNT_API_URL:-http://127.0.0.1:43200}"
EMAIL="${ADMIN_BOOTSTRAP_EMAIL:-dev@anycode.local}"
PASSWORD="${ADMIN_BOOTSTRAP_PASSWORD:-anycode-dev}"
SESSION_FILE="${HOME}/.anycode/credentials/cloud-session.json"

LOGIN="$(curl -s -X POST "${API}/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}")"
TOKEN="$(python3 -c "import json,sys; d=json.loads(sys.argv[1]); print(d.get('access_token') or d.get('token',''))" "$LOGIN")"

START="$(curl -s -X POST "${API}/api/v1/devices/link/start" \
  -H 'Content-Type: application/json' \
  -d '{"device_name":"eval-auto"}')"
CODE="$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['device_code'])" "$START")"

curl -s -X POST "${API}/api/v1/devices/link/approve" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "{\"device_code\":\"${CODE}\"}" >/dev/null

POLL="$(curl -s -X POST "${API}/api/v1/devices/link/poll" \
  -H 'Content-Type: application/json' \
  -d "{\"device_code\":\"${CODE}\"}")"

mkdir -p "$(dirname "$SESSION_FILE")"
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
print('Linked:', out.get('user_email'))
print('Gateway:', out.get('gateway_url'))
" "$POLL" "$SESSION_FILE"
