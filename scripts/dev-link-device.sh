#!/usr/bin/env bash
# Complete device link without the removed `anycode` CLI — polls account API until linked.
#
# Usage:
#   ./scripts/dev-link-device.sh dev_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
#   ./scripts/dev-link-device.sh   # reads code from clipboard (macOS pbpaste)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
anycode_apply_build_target_exports

CODE="${1:-}"
if [[ -z "$CODE" ]] && command -v pbpaste >/dev/null 2>&1; then
  CLIP="$(pbpaste 2>/dev/null || true)"
  if [[ "$CLIP" =~ (dev_[a-f0-9-]+) ]]; then
    CODE="${BASH_REMATCH[1]}"
  fi
fi
if [[ -z "$CODE" ]]; then
  echo "Usage: $0 <device_code>" >&2
  echo "  device_code looks like dev_e523b4a8-ee9b-4455-b167-b0fa7dadda2b" >&2
  exit 1
fi

API="${ANYCODE_ACCOUNT_API_URL}"
SESSION_FILE="${HOME}/.anycode/credentials/cloud-session.json"

echo "Polling ${API}/api/v1/devices/link/poll for ${CODE} ..."
for _ in $(seq 1 60); do
  RESP="$(curl -s -w "\n%{http_code}" -X POST "${API}/api/v1/devices/link/poll" \
    -H 'Content-Type: application/json' \
    -d "{\"device_code\":\"${CODE}\"}")"
  BODY="$(echo "$RESP" | sed '$d')"
  STATUS="$(echo "$RESP" | tail -1)"
  if [[ "$STATUS" == "200" ]]; then
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
print('Linked:', out.get('user_email') or '(ok)')
print('Session:', sys.argv[2])
" "$BODY" "$SESSION_FILE"
    echo "Restart anyCode.app or reload workbench account page."
    exit 0
  fi
  if [[ "$STATUS" == "202" ]]; then
    printf "."
    sleep 2
    continue
  fi
  echo "Poll failed ($STATUS): $BODY" >&2
  echo "Approve the code in portal: ${API}/console/settings?code=${CODE}" >&2
  exit 1
done
echo "Timed out — approve device in browser settings first." >&2
exit 1
