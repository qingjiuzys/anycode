#!/usr/bin/env bash
# Register a fake *online* cloud teammate for Desktop「发现同事 → 云端团队」.
#
# Uses your existing ~/.anycode/credentials/cloud-session.json (same org).
# Creates a second linked device if needed, then sends A2A presence heartbeats
# so the colleague graph shows a real peer (not demo preview).
#
# Usage:
#   ./scripts/register-cloud-colleague.sh start [--name "林晓"]
#   ./scripts/register-cloud-colleague.sh status
#   ./scripts/register-cloud-colleague.sh stop
#
# After start: reload「发现同事」— right-click the circle → 项目/会话交接.
# Optional full receive path: start with AUTO_APPROVE=1 (auto-approves incoming handoffs).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/build-target.sh
source "$ROOT/scripts/lib/build-target.sh"
anycode_apply_build_target_exports

SESSION_FILE="${HOME}/.anycode/credentials/cloud-session.json"
STATE_DIR="${HOME}/.anycode/lan"
STATE_FILE="${STATE_DIR}/dev-cloud-colleague.json"
PID_FILE="${STATE_DIR}/dev-cloud-colleague.pid"
LOG_FILE="${STATE_DIR}/dev-cloud-colleague.log"

API="${ANYCODE_ACCOUNT_API_URL:-https://anycode.work}"
API="${API%/}"
INTERVAL="${HEARTBEAT_INTERVAL_SECS:-30}"
DISPLAY_NAME="${COLLEAGUE_NAME:-林晓 (测试同事)}"
AUTO_APPROVE="${AUTO_APPROVE:-0}"

die() {
  echo "error: $*" >&2
  exit 1
}

load_token() {
  [[ -f "$SESSION_FILE" ]] || die "missing $SESSION_FILE — link cloud account in Desktop first"
  if [[ -n "${ANYCODE_CLOUD_ACCESS_TOKEN:-}" ]]; then
    echo "$ANYCODE_CLOUD_ACCESS_TOKEN"
    return 0
  fi
  python3 - <<'PY' "$SESSION_FILE"
import json, sys
from pathlib import Path
data = json.loads(Path(sys.argv[1]).read_text())
tok = (data.get("access_token") or "").strip()
if not tok:
    raise SystemExit("cloud-session.json has no access_token")
print(tok)
PY
}

refresh_token_if_needed() {
  TOKEN="$(load_token)"
  if api_get "/api/v1/auth/me" >/dev/null 2>&1; then
    return 0
  fi
  echo "access token expired — refreshing…" >&2
  python3 - <<'PY' "$SESSION_FILE" "$API" || return 1
import json, sys, urllib.request, urllib.error
from pathlib import Path

session_path, api = sys.argv[1], sys.argv[2].rstrip("/")
data = json.loads(Path(session_path).read_text())
refresh = (data.get("refresh_token") or "").strip()
if not refresh:
    raise SystemExit("no refresh_token in cloud-session.json")

body = json.dumps({"refresh_token": refresh}).encode()
req = urllib.request.Request(
    f"{api}/api/v1/devices/refresh",
    data=body,
    method="POST",
    headers={"Content-Type": "application/json"},
)
try:
    with urllib.request.urlopen(req, timeout=30) as resp:
        out = json.loads(resp.read().decode())
except urllib.error.HTTPError as e:
    raise SystemExit(
        f"refresh failed ({e.code}). Re-link cloud in Desktop: Settings → Cloud → Sign in"
    )
data["access_token"] = out["access_token"]
if out.get("refresh_token"):
    data["refresh_token"] = out["refresh_token"]
Path(session_path).write_text(json.dumps(data, indent=2) + "\n")
print("refreshed cloud session", file=sys.stderr)
PY
  TOKEN="$(load_token)"
  api_get "/api/v1/auth/me" >/dev/null 2>&1 || die "cloud session invalid — re-link in Desktop (Settings → Cloud)"
}

api_get() {
  local path="$1"
  curl -sf "${API}${path}" -H "Authorization: Bearer ${TOKEN}"
}

api_post() {
  local path="$1" body="$2"
  curl -sf "${API}${path}" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d "$body"
}

ensure_colleague_device() {
  if [[ -f "$STATE_FILE" ]]; then
    local saved_id
    saved_id="$(python3 -c "import json; print(json.load(open('$STATE_FILE')).get('device_id',''))")"
    if [[ -n "$saved_id" ]]; then
      echo "$saved_id"
      return 0
    fi
  fi

  python3 - <<'PY' "$API" "$TOKEN" "$DISPLAY_NAME" "$STATE_FILE"
import json, sys, urllib.request, uuid

api, token, display_name, state_path = sys.argv[1:5]

def req(method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    r = urllib.request.Request(
        f"{api}{path}",
        data=data,
        method=method,
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(r, timeout=30) as resp:
        return json.loads(resp.read().decode())

me = req("GET", "/api/v1/auth/me")
user = me.get("user") or {}
org_id = user.get("organization_id") or ""
user_id = user.get("id") or ""
if not org_id or not user_id:
    raise SystemExit("auth/me missing organization_id")

devices = req("GET", "/api/v1/devices").get("devices") or []
session_device = None
# Prefer a linked device that is not the most recently seen desktop (use oldest spare).
active = [d for d in devices if not d.get("revoked")]
if len(active) >= 2:
    spare = active[-1]
    out = {
        "device_id": spare["id"],
        "device_name": spare.get("device_name") or display_name,
        "instance_id": f"devcol_{uuid.uuid4()}",
        "organization_id": org_id,
        "user_id": user_id,
        "display_name": display_name,
    }
    json.dump(out, open(state_path, "w"), indent=2)
    print(spare["id"])
    raise SystemExit(0)

link = req("POST", "/api/v1/devices/link/start", {"device_name": display_name})
code = link["device_code"]
req("POST", "/api/v1/devices/link/approve", {"device_code": code})

poll = None
for _ in range(30):
    try:
        r = urllib.request.Request(
            f"{api}/api/v1/devices/link/poll",
            data=json.dumps({"device_code": code}).encode(),
            method="POST",
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(r, timeout=30) as resp:
            if resp.status == 200:
                poll = json.loads(resp.read().decode())
                break
    except urllib.error.HTTPError as e:
        if e.code == 202:
            import time
            time.sleep(2)
            continue
        raise
if not poll:
    raise SystemExit("device link poll timed out")

device_id = poll["device_id"]
out = {
    "device_id": device_id,
    "device_name": display_name,
    "instance_id": f"devcol_{uuid.uuid4()}",
    "organization_id": org_id,
    "user_id": user_id,
    "display_name": display_name,
}
json.dump(out, open(state_path, "w"), indent=2)
print(device_id)
PY
}

heartbeat_once() {
  python3 - <<'PY' "$API" "$TOKEN" "$STATE_FILE"
import json, sys, urllib.request

api, token, state_path = sys.argv[1:4]
state = json.load(open(state_path))
card = {
    "schema_version": "anycode_agent_card_v1",
    "instance_id": state["instance_id"],
    "device_id": state["device_id"],
    "organization_id": state["organization_id"],
    "user_id": state["user_id"],
    "name": state.get("device_name") or state.get("display_name") or "Test Colleague",
    "transport": "cloud",
    "version": "0.3.0-dev",
    "capabilities": ["handoff.project", "handoff.session", "streaming.relay"],
}
body = json.dumps({"agent_card": card}).encode()
req = urllib.request.Request(
    f"{api}/api/v1/a2a/presence/heartbeat",
    data=body,
    method="POST",
    headers={
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    },
)
with urllib.request.urlopen(req, timeout=30) as resp:
    resp.read()
print("heartbeat ok", state["device_name"], state["instance_id"][:12])
PY
}

poll_incoming_once() {
  [[ "$AUTO_APPROVE" == "1" ]] || return 0
  local device_id
  device_id="$(python3 -c "import json; print(json.load(open('$STATE_FILE'))['device_id'])")"
  local incoming
  incoming="$(api_get "/api/v1/a2a/handoff/incoming?device_id=${device_id}")" || return 0
  python3 - <<'PY' "$incoming" "$API" "$TOKEN" "$device_id"
import json, sys, urllib.request

incoming, api, token, device_id = sys.argv[1:5]
data = json.loads(incoming)
items = data.get("incoming") or []
if not items:
    raise SystemExit(0)
for task in items:
    hid = task.get("id")
    if not hid:
        continue
    body = json.dumps({"recipient_device_id": device_id, "target_project_id": "imported-handoff"}).encode()
    req = urllib.request.Request(
        f"{api}/api/v1/a2a/handoff/{hid}/approve",
        data=body,
        method="POST",
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            print("auto-approved handoff", hid, resp.read().decode()[:120])
    except Exception as e:
        print("approve failed", hid, e)
PY
}

run_loop() {
  mkdir -p "$STATE_DIR"
  while true; do
    refresh_token_if_needed || {
      echo "cloud auth failed — sleeping 60s ($(date -u +%H:%M:%S))" >&2
      sleep 60
      continue
    }
    heartbeat_once || echo "heartbeat failed $(date -u +%H:%M:%S)" >&2
    poll_incoming_once || true
    sleep "$INTERVAL"
  done
}

cmd_start() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --name)
        DISPLAY_NAME="$2"
        shift 2
        ;;
      *)
        die "unknown start arg: $1"
        ;;
    esac
  done

  if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "Already running (pid $(cat "$PID_FILE"))"
    cmd_status
    return 0
  fi

  mkdir -p "$STATE_DIR"
  refresh_token_if_needed
  local device_id
  device_id="$(ensure_colleague_device)"
  heartbeat_once

  : >"$LOG_FILE"
  nohup "$0" _loop >>"$LOG_FILE" 2>&1 &
  echo $! >"$PID_FILE"
  echo "Cloud colleague registered — heartbeats every ${INTERVAL}s (TTL 90s)"
  echo "  API:    $API"
  echo "  state:  $STATE_FILE"
  echo "  log:    $LOG_FILE"
  sleep 0.5
  echo "Started cloud colleague daemon (pid $(cat "$PID_FILE"))"
  cmd_status
}

cmd_stop() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" || true
      echo "Stopped pid $pid"
    fi
    rm -f "$PID_FILE"
  else
    echo "Not running"
  fi
}

cmd_status() {
  echo "=== cloud session ==="
  if [[ -f "$SESSION_FILE" ]]; then
    python3 -c "import json; d=json.load(open('$SESSION_FILE')); print('  email:', d.get('user_email','?'))"
  else
    echo "  (no session)"
  fi
  echo "=== colleague state ==="
  if [[ -f "$STATE_FILE" ]]; then cat "$STATE_FILE"; else echo "  (none — run start)"; fi
  echo "=== daemon ==="
  if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "  running pid $(cat "$PID_FILE")"
  else
    echo "  not running"
  fi
  if [[ -f "$LOG_FILE" ]]; then
    echo "=== last log lines ==="
    tail -n 3 "$LOG_FILE" || true
  fi
  if [[ -f "$SESSION_FILE" ]]; then
    if refresh_token_if_needed 2>/dev/null; then
      echo "=== team peers (account API) ==="
      api_get "/api/v1/a2a/team/peers" | python3 -m json.tool 2>/dev/null || echo "  (fetch failed)"
    else
      echo "=== team peers ==="
      echo "  (cloud session expired — re-link in Desktop Settings → Cloud, then re-run start)"
    fi
  fi
  cat <<EOF

Desktop: 侧边栏 → 发现同事 → 云端团队
  应看到真实在线同事（非「界面预览」横幅），右键圆圈 → 项目/会话交接。
  停止: $0 stop
EOF
}

case "${1:-status}" in
  start) shift; cmd_start "$@" ;;
  stop) cmd_stop ;;
  status) cmd_status ;;
  _loop) run_loop ;;
  *)
    echo "Usage: $0 {start [--name \"林晓 (测试同事)\"]|stop|status}" >&2
    exit 1
    ;;
esac
