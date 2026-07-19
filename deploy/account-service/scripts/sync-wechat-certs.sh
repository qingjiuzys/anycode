#!/usr/bin/env bash
# Sync WeChat Pay PEM files from qingjiu's zixun project (merchant 1703159450).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${WECHAT_CERT_SOURCE:-$HOME/workspace/business/zixun/server/src/main/resources/certificates}"
DEST="$ROOT/secrets"
mkdir -p "$DEST"
for f in apiclient_key.pem pub_key.pem; do
  cp "$SRC/$f" "$DEST/$f"
  echo "copied $f"
done
