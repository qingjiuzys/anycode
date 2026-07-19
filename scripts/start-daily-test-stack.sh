#!/usr/bin/env bash
# One-shot local stack for daily cloud conversation testing (no local LLM).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> account + mysql"
"$ROOT/scripts/start-local-account.sh" >/dev/null 2>&1 &
for _ in $(seq 1 30); do
  curl -sf http://127.0.0.1:43200/health >/dev/null 2>&1 && break
  sleep 1
done

echo "==> model gateway :43210"
"$ROOT/scripts/start-local-gateway.sh" start

echo "==> cloud session"
if ! "$ROOT/scripts/verify-cloud-e2e.sh" >/dev/null 2>&1; then
  "$ROOT/scripts/dev-auto-link.sh"
fi

echo "==> patch config to cloud-auto"
python3 <<'PY'
import json
from pathlib import Path
p = Path.home() / ".anycode/config.json"
c = json.loads(p.read_text())
c["model"] = "auto"
models = c.setdefault("models", {})
models.setdefault("active", {})["chat"] = "cloud-auto"
models.setdefault("chat", {}).update({
    "provider": "anycode_cloud",
    "model": "auto",
    "base_url": "http://127.0.0.1:43210/v1/chat/completions",
})
p.write_text(json.dumps(c, indent=2) + "\n")
print("active chat -> cloud-auto")
PY

echo ""
echo "Stack ready. Start anyCode.app (gateway must be up BEFORE app launch):"
echo "  open -a anyCode"
echo "  curl http://127.0.0.1:43180/api/cloud/gateway-test"
echo ""
echo "Run daily API matrix:"
echo "  ANYCODE_E2E_BASE=http://127.0.0.1:43180 python3 scripts/run-daily-conversation-eval.py"
