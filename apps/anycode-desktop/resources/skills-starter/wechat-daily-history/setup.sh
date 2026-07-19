#!/usr/bin/env bash
# Auto-detect WeChat db_storage, extract SQLCipher keys, merge ~/.anycode/config.json.
# Default: sqlcipher_key_map (direct local read via sqlcipher CLI, no HTTP port).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STATE_DIR="${ANYCODE_WECHAT_HISTORY_STATE:-$HOME/.anycode/wechat-history}"
VENDOR_DIR="$STATE_DIR/vendor/wechat-db-decrypt"
VENDOR_REPO="${ANYCODE_WECHAT_DECRYPT_REPO:-https://github.com/Thearas/wechat-db-decrypt-macos.git}"
CONFIG_PATH="${ANYCODE_CONFIG:-$HOME/.anycode/config.json}"
HTTP_ENDPOINT="${ANYCODE_WECHAT_HTTP:-http://127.0.0.1:5030}"
DEFAULT_KEY_OUT="$STATE_DIR/wechat_keys.json"
NORMALIZE_PY="$SCRIPT_DIR/normalize_keys.py"
TODAY="$(date +%Y-%m-%d)"
EXTRACT_ERROR=""

mkdir -p "$STATE_DIR"

usage() {
  cat <<EOF
usage: setup.sh [install|setup|status|ensure|extract-keys|start|stop]

  install      brew install sqlcipher/llvm; clone wechat-db-decrypt vendor.
  setup        install + ensure (recommended one-shot).
  status       Detect WeChat, keys, sqlcipher, SIP hint; print JSON.
  ensure       Write config; extract keys if missing (default).
  extract-keys Memory-scan keys → ${DEFAULT_KEY_OUT} (WeChat must be running).
  start/stop   Optional legacy chatlog HTTP only.

Direct local read: sqlcipher_key_map + wechat_keys.json — no open port.
iLink 微信扫码绑定机器人通道，与本脚本无关。
EOF
}

chatlog_health_ok() {
  local url="${HTTP_ENDPOINT%/}/api/v1/chatlog?time=${TODAY}&format=json&limit=1"
  curl -sf --max-time 3 "$url" >/dev/null 2>&1
}

find_sqlcipher_bin() {
  local p
  for p in sqlcipher /opt/homebrew/bin/sqlcipher /opt/homebrew/opt/sqlcipher/bin/sqlcipher /usr/local/bin/sqlcipher; do
    if [[ "$p" == */* ]]; then
      [[ -x "$p" ]] && printf '%s' "$p" && return 0
    elif command -v "$p" >/dev/null 2>&1; then
      command -v "$p"
      return 0
    fi
  done
  return 1
}

sip_enabled_hint() {
  if ! command -v csrutil >/dev/null 2>&1; then
    echo "unknown"
    return 0
  fi
  if csrutil status 2>/dev/null | grep -qi "disabled"; then
    echo "disabled"
  else
    echo "enabled"
  fi
}

wechat_running() {
  pgrep -x WeChat >/dev/null 2>&1
}

find_chatlog_bin() {
  local name cand dir
  for name in chatlog chatlog-bot chatlog-server chatlog-bot-server; do
    if cand="$(command -v "$name" 2>/dev/null)" && [[ -n "$cand" && -x "$cand" ]]; then
      printf '%s' "$cand"
      return 0
    fi
  done
  for dir in \
    "$HOME/.local/bin" "$HOME/bin" "$HOME/go/bin" "$HOME/.chatlog" \
    "/opt/homebrew/bin" "/usr/local/bin"; do
    for name in chatlog chatlog-bot chatlog-server; do
      cand="$dir/$name"
      if [[ -x "$cand" ]]; then
        printf '%s' "$cand"
        return 0
      fi
    done
  done
  return 1
}

find_wechat_db_storage() {
  python3 - <<'PY'
from pathlib import Path
root = Path.home() / "Library/Containers/com.tencent.xinWeChat/Data/Documents/xwechat_files"
best = None
best_mtime = 0.0
if root.is_dir():
    for p in root.glob("wxid_*/db_storage"):
        if p.is_dir():
            mtime = p.stat().st_mtime
            if mtime > best_mtime:
                best_mtime = mtime
                best = p
if best:
    print(best)
PY
}

find_key_map() {
  local p
  for p in \
    "$DEFAULT_KEY_OUT" \
    "$HOME/wechat_keys.json" \
    "$HOME/.chatlog/wechat_keys.json" \
    "$STATE_DIR/wechat_keys.json" \
    "$HOME/Documents/wechat_keys.json"; do
    if [[ -f "$p" ]]; then
      printf '%s' "$p"
      return 0
    fi
  done
  return 1
}

config_prefers_http() {
  python3 - "$CONFIG_PATH" <<'PY'
import json, sys
from pathlib import Path
p = Path(sys.argv[1])
if not p.is_file():
    print("false")
    raise SystemExit
cfg = json.loads(p.read_text(encoding="utf-8"))
wh = cfg.get("wechatHistory") or {}
print("true" if wh.get("backend") == "chatlog_http" else "false")
PY
}

collect_state() {
  CHATLOG_BIN="$(find_chatlog_bin 2>/dev/null || true)"
  SQLCIPHER_BIN="$(find_sqlcipher_bin 2>/dev/null || true)"
  DATA_DIR="$(find_wechat_db_storage 2>/dev/null || true)"
  KEY_MAP="$(find_key_map 2>/dev/null || true)"
  WECHAT_RUNNING=false
  wechat_running && WECHAT_RUNNING=true
  SIP_HINT="$(sip_enabled_hint)"
  VENDOR_READY=false
  [[ -f "$VENDOR_DIR/find_key_memscan.py" ]] && VENDOR_READY=true
  HTTP_OK=false
  CHATLOG_RUNNING=false
  chatlog_health_ok && HTTP_OK=true
  if [[ -f "$STATE_DIR/chatlog.pid" ]]; then
    local pid
    pid="$(cat "$STATE_DIR/chatlog.pid" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      CHATLOG_RUNNING=true
    fi
  fi
}

json_detect() {
  python3 - "$CHATLOG_BIN" "$SQLCIPHER_BIN" "$DATA_DIR" "$KEY_MAP" "$HTTP_ENDPOINT" "$HTTP_OK" "$CHATLOG_RUNNING" "$WECHAT_RUNNING" "$SIP_HINT" "$VENDOR_READY" <<'PY'
import json, os, sys
(chatlog_bin, sqlcipher_bin, data_dir, key_map, http_endpoint,
 http_ok, running, wechat_running, sip_hint, vendor_ready) = sys.argv[1:11]
print(json.dumps({
    "chatlog_bin": chatlog_bin or None,
    "sqlcipher_bin": sqlcipher_bin or None,
    "data_dir": data_dir or None,
    "key_map_path": key_map or None,
    "http_endpoint": http_endpoint,
    "http_ok": http_ok == "true",
    "chatlog_running": running == "true",
    "wechat_running": wechat_running == "true",
    "sip_status": sip_hint,
    "vendor_ready": vendor_ready == "true",
    "wechat_container_present": os.path.isdir(os.path.expanduser("~/Library/Containers/com.tencent.xinWeChat")),
    "note": "iLink QR bind is for WeChat bot channel, not local DB history.",
}, ensure_ascii=False))
PY
}

merge_config() {
  local data_dir="$1" key_map="$2" backend="$3"
  python3 - "$CONFIG_PATH" "$backend" "$HTTP_ENDPOINT" "$data_dir" "$key_map" <<'PY'
import json, os, stat, sys
from pathlib import Path

cfg_path = Path(sys.argv[1])
backend = sys.argv[2]
http_endpoint = sys.argv[3]
data_dir = sys.argv[4] or None
key_map = sys.argv[5] or None

cfg_path.parent.mkdir(parents=True, exist_ok=True)
cfg = json.loads(cfg_path.read_text(encoding="utf-8")) if cfg_path.is_file() else {}
wh = cfg.setdefault("wechatHistory", {})
wh.update({
    "enabled": True,
    "backend": backend,
    "httpEndpoint": http_endpoint,
    "defaultTimezone": wh.get("defaultTimezone") or "Asia/Shanghai",
    "maxRowsPerQuery": wh.get("maxRowsPerQuery") or 500,
})
if data_dir:
    wh["dataDir"] = data_dir
if key_map:
    wh["keyMapPath"] = key_map
text = json.dumps(cfg, indent=2, ensure_ascii=False) + "\n"
cfg_path.write_text(text, encoding="utf-8")
try:
    os.chmod(cfg_path, stat.S_IRUSR | stat.S_IWUSR)
except OSError:
    pass
print(cfg_path)
PY
}

ensure_vendor() {
  if [[ -f "$VENDOR_DIR/find_key_memscan.py" ]]; then
    return 0
  fi
  mkdir -p "$(dirname "$VENDOR_DIR")"
  echo "Cloning wechat-db-decrypt-macos into $VENDOR_DIR ..."
  git clone --depth 1 "$VENDOR_REPO" "$VENDOR_DIR"
}

find_lldb_python_path() {
  local lldb_bin py
  for lldb_bin in \
    /Applications/Xcode.app/Contents/Developer/usr/bin/lldb \
    /opt/homebrew/opt/llvm/bin/lldb \
    "$(command -v lldb 2>/dev/null || true)"; do
    [[ -n "$lldb_bin" && -x "$lldb_bin" ]] || continue
    py="$("$lldb_bin" -P 2>/dev/null || true)"
    if [[ -n "$py" && -d "$py/lldb" ]]; then
      printf '%s' "$py"
      return 0
    fi
  done
  return 1
}

find_memscan_python() {
  local v
  for v in /usr/bin/python3 /Applications/Xcode.app/Contents/Developer/usr/bin/python3 python3.9 python3; do
    if command -v "$v" >/dev/null 2>&1; then
      if "$v" -c 'import sys; raise SystemExit(0 if sys.version_info[:2] <= (3, 9) else 1)' 2>/dev/null; then
        command -v "$v" 2>/dev/null || printf '%s' "$v"
        return 0
      fi
    fi
  done
  command -v python3
}

run_memscan_once() {
  local tmpdir="$1" lldb_py="$2" py="$3" use_sudo="$4"
  if [[ "$use_sudo" == "1" && $EUID -ne 0 ]]; then
    (cd "$tmpdir" && sudo -E env "PYTHONPATH=$lldb_py" "$py" "$VENDOR_DIR/find_key_memscan.py")
  else
    (cd "$tmpdir" && env "PYTHONPATH=$lldb_py" "$py" "$VENDOR_DIR/find_key_memscan.py")
  fi
}

install_deps() {
  local log="$STATE_DIR/install.log"
  : >"$log"
  echo "==> $(date -Iseconds 2>/dev/null || date) install" >>"$log"
  if command -v brew >/dev/null 2>&1; then
    if ! find_sqlcipher_bin >/dev/null 2>&1; then
      echo "==> brew install sqlcipher" | tee -a "$log" >&2
      brew install sqlcipher >>"$log" 2>&1 || echo "brew install sqlcipher failed; see $log" | tee -a "$log" >&2
    fi
    if ! command -v lldb >/dev/null 2>&1; then
      echo "==> brew install llvm (lldb)" | tee -a "$log" >&2
      brew install llvm >>"$log" 2>&1 || echo "brew install llvm failed; see $log" | tee -a "$log" >&2
    fi
  else
    echo "Homebrew not found; ensure sqlcipher and lldb are on PATH manually." | tee -a "$log" >&2
  fi
  if ! ensure_vendor >>"$log" 2>&1; then
    echo "vendor install failed; see $log" >&2
    return 1
  fi
  chmod +x "$NORMALIZE_PY" 2>/dev/null || true
  collect_state
  if [[ -z "$SQLCIPHER_BIN" ]]; then
    echo "warning: sqlcipher not found; run: brew install sqlcipher" >&2
  fi
  json_detect
  return 0
}

extract_keys_chatlog() {
  local bin="$1"
  local log="$STATE_DIR/extract-keys.log"
  local out="$DEFAULT_KEY_OUT"
  local try_specs=(
    "key export -o $out"
    "keys export $out"
    "key extract --output $out"
    "extract-key -o $out"
    "key --output $out"
    "keys --output $out"
    "key export --output $out"
  )
  echo "==> chatlog fallback via $bin" >>"$log"
  local spec
  for spec in "${try_specs[@]}"; do
    echo "==> trying: $bin $spec" >>"$log"
    if bash -lc "$bin $spec" >>"$log" 2>&1 && [[ -f "$out" && -s "$out" ]]; then
      chmod 600 "$out" 2>/dev/null || true
      return 0
    fi
  done
  for p in "$HOME/wechat_keys.json" "$HOME/.chatlog/wechat_keys.json" "$PWD/wechat_keys.json" "$PWD/keys.json"; do
    if [[ -f "$p" && -s "$p" ]]; then
      cp "$p" "$out"
      chmod 600 "$out" 2>/dev/null || true
      return 0
    fi
  done
  return 1
}

extract_keys_memscan() {
  local log="$STATE_DIR/extract-keys.log"
  local out="$DEFAULT_KEY_OUT"
  local raw="$STATE_DIR/keys.raw.json"
  EXTRACT_ERROR=""

  if ! wechat_running; then
    EXTRACT_ERROR=wechat_not_running
    echo "WeChat is not running." >>"$log"
    return 1
  fi

  ensure_vendor || { EXTRACT_ERROR=vendor_install_failed; return 1; }

  local lldb_py py
  lldb_py="$(find_lldb_python_path 2>/dev/null || true)"
  py="$(find_memscan_python 2>/dev/null || true)"
  if [[ -z "$lldb_py" ]]; then
    EXTRACT_ERROR=lldb_not_installed
    echo "lldb Python not found; install Xcode CLT or: brew install llvm" >>"$log"
    return 1
  fi
  if [[ -z "$py" ]]; then
    EXTRACT_ERROR=lldb_not_installed
    echo "python3 not found for memscan" >>"$log"
    return 1
  fi

  echo "==> $(date -Iseconds 2>/dev/null || date) memscan extract (py=$py)" >>"$log"
  local tmpdir
  tmpdir="$(mktemp -d)"
  if ! run_memscan_once "$tmpdir" "$lldb_py" "$py" 0 >>"$log" 2>&1; then
    echo "==> memscan without sudo failed; retrying with sudo" >>"$log"
    if ! run_memscan_once "$tmpdir" "$lldb_py" "$py" 1 >>"$log" 2>&1; then
      if grep -Eiq "password|sudo|incorrect password|a terminal is required" "$log"; then
        EXTRACT_ERROR=sudo_denied
      elif grep -Eiq "SIP|csrutil|not allowed|Operation not permitted|Error attaching" "$log"; then
        EXTRACT_ERROR=sip_blocks_memory_scan
      else
        EXTRACT_ERROR=wechat_keys_extract_failed
      fi
      rm -rf "$tmpdir"
      return 1
    fi
  fi
  if [[ ! -f "$tmpdir/wechat_keys.json" ]]; then
    EXTRACT_ERROR=wechat_keys_extract_failed
    rm -rf "$tmpdir"
    return 1
  fi
  cp "$tmpdir/wechat_keys.json" "$raw"
  rm -rf "$tmpdir"

  if ! python3 "$NORMALIZE_PY" "$raw" "$out" >>"$log" 2>&1; then
    EXTRACT_ERROR=wechat_keys_extract_failed
    return 1
  fi
  return 0
}

extract_keys() {
  local log="$STATE_DIR/extract-keys.log"
  : >"$log"
  if extract_keys_memscan; then
    return 0
  fi
  if [[ -n "${CHATLOG_BIN:-}" ]] && extract_keys_chatlog "$CHATLOG_BIN"; then
    local raw="$STATE_DIR/keys.raw.chatlog.json"
    cp "$DEFAULT_KEY_OUT" "$raw" 2>/dev/null || true
    python3 "$NORMALIZE_PY" "$DEFAULT_KEY_OUT" "$DEFAULT_KEY_OUT" >>"$log" 2>&1 || true
    return 0
  fi
  return 1
}

extract_error_hint() {
  case "${EXTRACT_ERROR:-}" in
    wechat_not_running)
      echo "请先打开并登录 Mac 微信，保持 WeChat 进程运行后重试。"
      ;;
    sip_blocks_memory_scan)
      echo "macOS SIP 阻止内存扫描。请重启进恢复模式执行 csrutil disable，完成后再 csrutil enable。详见 vendor README: $VENDOR_REPO"
      ;;
    sudo_denied)
      echo "密钥提取需要管理员权限 attach 微信进程，请在提示时输入 sudo 密码。"
      ;;
    lldb_not_installed|vendor_install_failed)
      echo "运行 setup.sh install 安装 llvm/sqlcipher 与 vendor 工具。"
      ;;
    *)
      echo "查看 ${STATE_DIR}/extract-keys.log；确认微信已登录。可运行: setup.sh install && setup.sh extract-keys"
      ;;
  esac
}

verify_keys_smoke() {
  local log="$STATE_DIR/verify-keys.log"
  local keys="$1"
  [[ -f "$keys" ]] || return 1
  : >"$log"
  if [[ -f "$VENDOR_DIR/verify_keys.py" ]] && command -v python3 >/dev/null 2>&1; then
    if python3 "$VENDOR_DIR/verify_keys.py" --keys "$keys" >>"$log" 2>&1; then
      return 0
    fi
  fi
  local sqlc db key
  sqlc="$(find_sqlcipher_bin 2>/dev/null || true)"
  [[ -n "$sqlc" && -n "${DATA_DIR:-}" ]] || return 1
  db="$(find "$DATA_DIR" -name 'message_0.db' 2>/dev/null | head -1 || true)"
  [[ -n "$db" ]] || return 0
  key="$(python3 - "$keys" <<'PY'
import json, sys
from pathlib import Path
raw = json.loads(Path(sys.argv[1]).read_text())
for k, v in raw.items():
    if k.endswith("message_0.db") or k.endswith("message/message_0.db"):
        print(v)
        break
PY
)"
  [[ -n "$key" ]] || return 1
  printf "PRAGMA key = \"x'%s'\";\nPRAGMA cipher_page_size = 4096;\nPRAGMA kdf_iter = 256000;\nSELECT count(*) FROM sqlite_master;\n" "$key" \
    | "$sqlc" "$db" >>"$log" 2>&1
}

start_chatlog() {
  local bin="$1"
  local log="$STATE_DIR/chatlog.log"
  local pidfile="$STATE_DIR/chatlog.pid"
  local host="${HTTP_ENDPOINT#*://}"
  host="${host%%/*}"
  local port="${host##*:}"
  port="${port:-5030}"
  host="${host%%:*}"
  if [[ -f "$pidfile" ]]; then
    local oldpid
    oldpid="$(cat "$pidfile" 2>/dev/null || true)"
    if [[ -n "$oldpid" ]] && kill -0 "$oldpid" 2>/dev/null && chatlog_health_ok; then
      return 0
    fi
    [[ -n "$oldpid" ]] && kill "$oldpid" 2>/dev/null || true
  fi
  local base try_cmds=() cmd
  base="$(basename "$bin")"
  if [[ "$base" == *bot* ]]; then
    try_cmds=("$bin server --http ${host}:${port}" "$bin server --port ${port}" "$bin server")
  else
    try_cmds=("$bin server --http ${host}:${port}" "$bin server --addr ${host}:${port}" "$bin server" "$bin serve")
  fi
  for cmd in "${try_cmds[@]}"; do
    echo "==> $(date -Iseconds 2>/dev/null || date) trying: $cmd" >>"$log"
    nohup bash -lc "$cmd" >>"$log" 2>&1 &
    echo $! >"$pidfile"
    sleep 2
    if chatlog_health_ok; then
      return 0
    fi
    kill "$(cat "$pidfile")" 2>/dev/null || true
  done
  return 1
}

stop_chatlog() {
  local pidfile="$STATE_DIR/chatlog.pid"
  [[ -f "$pidfile" ]] || return 0
  local pid
  pid="$(cat "$pidfile" 2>/dev/null || true)"
  [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  rm -f "$pidfile"
}

print_result() {
  local ok="$1" backend="$2" cfg_path="$3" error="${4:-}" hint="${5:-}"
  local detect
  detect="$(json_detect)"
  python3 - "$ok" "$backend" "$cfg_path" "$error" "$hint" "$detect" "$STATE_DIR" <<'PY'
import json, sys
ok, backend, cfg_path, error, hint, detect_json, state_dir = sys.argv[1:8]
out = {
    "ok": ok == "true",
    "config_path": cfg_path or None,
    "backend": backend or None,
    "detect": json.loads(detect_json),
}
if error:
    out["error"] = error
if hint:
    out["hint"] = hint
if not out["ok"]:
    if out.get("error") in ("wechat_keys_not_found", "wechat_keys_extract_failed", "sip_blocks_memory_scan"):
        out["extract_log"] = f"{state_dir}/extract-keys.log"
    if out["detect"].get("chatlog_bin") and not out["detect"].get("http_ok"):
        out["log"] = f"{state_dir}/chatlog.log"
print(json.dumps(out, ensure_ascii=False, indent=2))
PY
}

want_http_mode() {
  [[ "${ANYCODE_WECHAT_HTTP:-}" == "1" ]] || [[ "$(config_prefers_http)" == "true" ]]
}

finish_success() {
  local backend="$1" cfg_path="$2"
  if ! verify_keys_smoke "$KEY_MAP"; then
    print_result false "$backend" "$cfg_path" "keys_extracted_but_db_verify_failed" \
      "密钥已写入但解密验证失败。请重启微信后运行 setup.sh extract-keys。"
    exit 1
  fi
  print_result true "$backend" "$cfg_path" "" \
    "已配置 ${backend}；QueryWeChatHistory 可直接读本地加密库，无需 HTTP 端口。"
  exit 0
}

cmd="${1:-ensure}"
case "$cmd" in
  -h|--help|help) usage; exit 0 ;;
  install)
    install_deps
    exit $?
    ;;
  setup)
    install_deps || exit 1
    cmd=ensure
    ;;
  status)
    collect_state
    json_detect
    exit 0
    ;;
  stop)
    stop_chatlog
    collect_state
    json_detect
    exit 0
    ;;
  extract-keys)
    collect_state
    if ! extract_keys; then
      print_result false "" "" "${EXTRACT_ERROR:-wechat_keys_extract_failed}" "$(extract_error_hint)"
      exit 1
    fi
    KEY_MAP="$(find_key_map 2>/dev/null || true)"
    collect_state
    if [[ -n "$KEY_MAP" ]] && ! verify_keys_smoke "$KEY_MAP"; then
      print_result false "" "" "keys_extracted_but_db_verify_failed" \
        "密钥已提取但 verify 失败；重启微信后重试 extract-keys。"
      exit 1
    fi
    json_detect
    exit 0
    ;;
esac

collect_state

if [[ -z "$DATA_DIR" ]]; then
  print_result false "" "" "wechat_db_not_found" "未找到本机微信 db_storage。请确认 macOS 微信已登录。"
  exit 1
fi

if [[ -z "$KEY_MAP" && "$cmd" == "ensure" ]]; then
  extract_keys || true
  KEY_MAP="$(find_key_map 2>/dev/null || true)"
fi

if [[ -n "$KEY_MAP" ]]; then
  BACKEND="sqlcipher_key_map"
  CFG_OUT="$(merge_config "$DATA_DIR" "$KEY_MAP" "$BACKEND")"
  finish_success "$BACKEND" "$CFG_OUT"
fi

if want_http_mode && [[ -n "$CHATLOG_BIN" ]]; then
  BACKEND="chatlog_http"
  CFG_OUT="$(merge_config "$DATA_DIR" "" "$BACKEND")"
  if [[ "$HTTP_OK" != "true" && ( "$cmd" == "ensure" || "$cmd" == "start" ) ]]; then
    start_chatlog "$CHATLOG_BIN" && HTTP_OK=true || true
  fi
  if [[ "$HTTP_OK" == "true" ]]; then
    print_result true "$BACKEND" "$CFG_OUT" "" ""
    exit 0
  fi
  print_result false "$BACKEND" "$CFG_OUT" "chatlog_http_unreachable" \
    "chatlog HTTP 未响应。建议运行: setup.sh setup（直接本地，无需端口）。"
  exit 1
fi

BACKEND="sqlcipher_key_map"
CFG_OUT="$(merge_config "$DATA_DIR" "" "$BACKEND")"
print_result false "$BACKEND" "$CFG_OUT" "wechat_keys_not_found" \
  "运行一键配置: setup.sh setup 或 anycode wechat history setup（需微信已登录；可能需临时关 SIP）。"
exit 1
