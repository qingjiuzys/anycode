#!/bin/bash
# 将本机 Cursor 接入 DeepSeek（deepseek-v4-flash）
# 用法：
#   ./cursor-deepseek-setup.sh            # 直接写入配置（Cursor 需已退出）
#   ./cursor-deepseek-setup.sh --wait     # 等待 Cursor 完全退出后写入
set -euo pipefail

STORAGE="$HOME/Library/Application Support/Cursor/User/globalStorage/storage.json"
KEYFILE="$HOME/.anycode/secrets/deepseek.txt"
BASE_URL="https://api.deepseek.com"
MODEL="deepseek-v4-flash"

if [[ "${1:-}" == "--wait" ]]; then
  echo "[1/3] 等待 Cursor 完全退出..."
  while pgrep -x "Cursor" >/dev/null 2>&1 || pgrep -f "Cursor Helper (Renderer)" >/dev/null 2>&1; do
    sleep 2
  done
  echo "[1/3] Cursor 已退出"
else
  if pgrep -x "Cursor" >/dev/null 2>&1; then
    echo "错误：Cursor 正在运行。请先 Cmd+Q 完全退出 Cursor，或用 --wait 参数。"
    exit 1
  fi
fi

if [[ ! -f "$KEYFILE" ]]; then
  echo "错误：找不到 DeepSeek key：$KEYFILE"
  exit 1
fi

cp "$STORAGE" "$STORAGE.bak-$(date +%Y%m%d-%H%M%S)"

python3 - "$STORAGE" "$KEYFILE" "$BASE_URL" "$MODEL" <<'EOF'
import json, sys
p, kf, base, model = sys.argv[1:5]
key = open(kf).read().strip()
d = json.load(open(p))
d["useOpenAIKey"] = True
d["openAIBaseUrl"] = base
ai = d.get("aiSettings") or {}
added = ai.get("userAddedModels") or []
if model not in added:
    added.append(model)
ai["userAddedModels"] = added
en = ai.get("modelOverrideEnabled") or []
if model not in en:
    en.append(model)
ai["modelOverrideEnabled"] = en
d["aiSettings"] = ai
auth = d.get("cursorAuth") or {}
auth["openAIKey"] = key
d["cursorAuth"] = auth
json.dump(d, open(p, "w"), indent=1, ensure_ascii=False)
print("写入完成：useOpenAIKey=true, openAIBaseUrl=%s, userAddedModels=%s" % (base, model))
EOF

echo "[3/3] 完成。重新打开 Cursor 即可在模型列表中选择 deepseek-v4-flash。"