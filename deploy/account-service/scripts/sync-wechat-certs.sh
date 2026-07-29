#!/usr/bin/env bash
# Sync WeChat Pay PEM files from local merchant bundle into deploy/account-service/secrets/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${WECHAT_CERT_SOURCE:-$HOME/workspace/business/zixun/server/src/main/resources/certificates}"
DEST="$ROOT/secrets"
mkdir -p "$DEST"
for f in apiclient_key.pem pub_key.pem; do
  cp "$SRC/$f" "$DEST/$f"
  echo "copied $f"
done
