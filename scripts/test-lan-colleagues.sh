#!/usr/bin/env bash
# Local two-peer smoke test for LAN colleague discovery + handoff protocol.
#
# Usage:
#   ./scripts/test-lan-colleagues.sh start    # start peer B + register on Desktop (peer A)
#   ./scripts/test-lan-colleagues.sh status   # health + peer list
#   ./scripts/test-lan-colleagues.sh stop     # stop peer B + remove dev peer entry
#   ./scripts/test-lan-colleagues.sh logs     # tail peer B log
#   ./scripts/test-lan-colleagues.sh probe    # curl 走一遍 handoff 协议（无需 UI）
#
# Peer A (AnyCode Desktop):  UI :43180  LAN :43181  (~/.anycode/lan)
# Peer B (this script):      UI :43221  LAN :43183  (/tmp/anycode-lan-peer-b)
#
# Same-machine note: mDNS cannot register two instances; peer B is injected via
# ~/.anycode/lan/dev_peers.json (read when you open「发现同事」).

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PEER_DIR="/tmp/anycode-lan-peer-b"
PEER_DB="/tmp/anycode-lan-peer-b/projects.db"
PEER_UI_PORT=43221
PEER_LAN_PORT=43183
PID_FILE="$PEER_DIR/peer-b.pid"
LOG_FILE="$PEER_DIR/peer-b.log"
DEV_PEERS_FILE="${HOME}/.anycode/lan/dev_peers.json"
SERVE_BIN="$ROOT/target/release-local/anycode-dashboard-serve"

mkdir -p "$PEER_DIR" "${HOME}/.anycode/lan"

write_peer_settings() {
  cat >"$PEER_DIR/settings.json" <<EOF
{
  "discovery_enabled": true,
  "display_name": "Test Colleague B",
  "lan_port": $PEER_LAN_PORT,
  "max_bundle_mb": 500
}
EOF
}

register_dev_peer() {
  local health
  health=$(curl -sf "http://127.0.0.1:${PEER_LAN_PORT}/api/lan/health") || return 1
  python3 - <<PY "$health"
import json, sys
from datetime import datetime, timezone
h = json.loads(sys.argv[1])
peer = {
  "instance_id": h["instance_id"],
  "device_name": h["device_name"],
  "host": "127.0.0.1",
  "lan_port": $PEER_LAN_PORT,
  "version": h.get("version", "0.2.4"),
  "last_seen": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
}
with open("$DEV_PEERS_FILE", "w") as f:
  json.dump([peer], f, indent=2)
print("dev_peers.json written for Desktop peer A")
PY
}

start_peer_b() {
  if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "Peer B already running (pid $(cat "$PID_FILE"))"
    register_dev_peer || true
    return 0
  fi

  if [[ ! -x "$SERVE_BIN" ]]; then
    echo "Building anycode-dashboard-serve (release-local)…"
    (cd "$ROOT" && cargo build -p anycode-dashboard --bin anycode-dashboard-serve --profile release-local)
  fi

  write_peer_settings
  export ANYCODE_LAN_DATA_DIR="$PEER_DIR"
  export ANYCODE_DASHBOARD_DB="$PEER_DB"
  export ANYCODE_DASHBOARD_TEST_AUTH_BYPASS=1

  nohup "$SERVE_BIN" \
    --host 127.0.0.1 \
    --port "$PEER_UI_PORT" \
    --db "$PEER_DB" \
    >"$LOG_FILE" 2>&1 &
  echo $! >"$PID_FILE"

  for _ in $(seq 1 15); do
    if curl -sf "http://127.0.0.1:${PEER_LAN_PORT}/api/lan/health" >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done

  register_dev_peer || { echo "Peer B LAN not ready — check $LOG_FILE"; return 1; }

  echo "Peer B started (pid $(cat "$PID_FILE"))"
  echo "  UI:  http://127.0.0.1:$PEER_UI_PORT"
  echo "  LAN: http://127.0.0.1:$PEER_LAN_PORT/api/lan/health"
  echo "  dev_peers: $DEV_PEERS_FILE"
}

stop_peer_b() {
  if [[ -f "$PID_FILE" ]]; then
    pid=$(cat "$PID_FILE")
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" || true
      echo "Stopped peer B (pid $pid)"
    fi
    rm -f "$PID_FILE"
  else
    echo "Peer B not running"
  fi
  rm -f "$DEV_PEERS_FILE"
  echo "Removed $DEV_PEERS_FILE"
}

probe_handoff() {
  echo "=== 1/4 LAN health ==="
  local health_a health_b
  health_a=$(curl -sf "http://127.0.0.1:43181/api/lan/health") || {
    echo "Peer A LAN (:43181) unreachable — start AnyCode Desktop first"
    return 1
  }
  health_b=$(curl -sf "http://127.0.0.1:${PEER_LAN_PORT}/api/lan/health") || {
    echo "Peer B LAN (:${PEER_LAN_PORT}) unreachable — run: $0 start"
    return 1
  }
  echo "Peer A: $(echo "$health_a" | python3 -c 'import json,sys; h=json.load(sys.stdin); print(h["device_name"], h["instance_id"])')"
  echo "Peer B: $(echo "$health_b" | python3 -c 'import json,sys; h=json.load(sys.stdin); print(h["device_name"], h["instance_id"])')"

  local handoff_id
  handoff_id="ho_probe_$(date +%s)"
  echo
  echo "=== 2/4 POST handoff/request → Peer B LAN ==="
  local req_resp
  req_resp=$(python3 - <<PY "$health_a" "$health_b" "$handoff_id"
import json, sys, urllib.request
health_a, health_b, hid = json.loads(sys.argv[1]), json.loads(sys.argv[2]), sys.argv[3]
body = {
  "id": hid,
  "kind": "project",
  "sender": {
    "instance_id": health_a["instance_id"],
    "device_name": health_a["device_name"],
    "host": "127.0.0.1",
    "lan_port": 43181,
  },
  "recipient": {
    "instance_id": health_b["instance_id"],
    "device_name": health_b["device_name"],
    "host": "127.0.0.1",
    "lan_port": $PEER_LAN_PORT,
  },
  "project_id": "probe-project",
  "project_name": "Probe Test Project",
}
data = json.dumps(body).encode()
req = urllib.request.Request(
  f"http://127.0.0.1:$PEER_LAN_PORT/api/lan/handoff/request",
  data=data,
  headers={"Content-Type": "application/json"},
  method="POST",
)
with urllib.request.urlopen(req, timeout=10) as r:
  print(r.read().decode())
PY
) || { echo "handoff/request failed"; return 1; }
  echo "$req_resp" | python3 -m json.tool

  echo
  echo "=== 3/4 GET incoming on Peer B UI ==="
  curl -sf "http://127.0.0.1:${PEER_UI_PORT}/api/lan/handoff/incoming" | python3 -m json.tool

  echo
  echo "=== 4/4 POST approve on Peer B UI ==="
  curl -sf -X POST "http://127.0.0.1:${PEER_UI_PORT}/api/lan/handoff/${handoff_id}/approve" \
    -H "Content-Type: application/json" \
    -d '{"target_root_path":"/tmp/anycode-lan-peer-b/imported"}' | python3 -m json.tool

  echo
  echo "=== status on Peer B LAN ==="
  curl -sf "http://127.0.0.1:${PEER_LAN_PORT}/api/lan/handoff/${handoff_id}/status" | python3 -m json.tool 2>/dev/null || echo "(status endpoint may require outgoing record on sender)"

  echo
  echo "Probe OK — 协议链路通。完整 bundle 上传需 Desktop UI 发起真实项目交接。"
}

status() {
  echo "=== Peer A — Desktop LAN (:43181) ==="
  curl -sf "http://127.0.0.1:43181/api/lan/health" | python3 -m json.tool 2>/dev/null || echo "not reachable (start AnyCode Desktop)"
  echo
  echo "=== Peer B — test instance (:${PEER_LAN_PORT}) ==="
  curl -sf "http://127.0.0.1:${PEER_LAN_PORT}/api/lan/health" | python3 -m json.tool 2>/dev/null || echo "not running — run: $0 start"
  echo
  echo "=== dev_peers.json (for Desktop UI) ==="
  if [[ -f "$DEV_PEERS_FILE" ]]; then cat "$DEV_PEERS_FILE"; else echo "(missing — run: $0 start)"; fi
  echo
  echo "=== UI test steps ==="
  cat <<EOF
1. AnyCode Desktop 保持运行（peer A）
2. 已执行 start 后，打开侧边栏 → 发现同事 → 应看到「Test Colleague B」
3. 右键 → 项目交接；在 http://127.0.0.1:$PEER_UI_PORT 侧边栏批准
4. 真机双端测试：两台 Mac 同 Wi‑Fi，无需 dev_peers.json，靠 mDNS 自动发现
5. 停止：$0 stop
EOF
}

cmd="${1:-status}"
case "$cmd" in
  start) start_peer_b; status ;;
  stop) stop_peer_b ;;
  status) status ;;
  logs) tail -f "$LOG_FILE" ;;
  probe) probe_handoff ;;
  *) echo "Usage: $0 {start|stop|status|logs|probe}"; exit 1 ;;
esac
