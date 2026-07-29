#!/usr/bin/env bash
# Create/update K8s Secret with WeChat Pay PEM files (runtime mount at /app/wechat-certs).
# Usage:
#   ./scripts/create-k8s-wechat-secret.sh dis-cloud
#   WECHAT_CERT_DIR=./secrets ./scripts/create-k8s-wechat-secret.sh dis-cloud
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NS="${1:-dis-cloud}"
CERT_DIR="${WECHAT_CERT_DIR:-$ROOT/secrets}"

for f in apiclient_key.pem pub_key.pem; do
  if [[ ! -f "$CERT_DIR/$f" ]]; then
    echo "Missing $CERT_DIR/$f — run: ./scripts/sync-wechat-certs.sh" >&2
    exit 1
  fi
done

kubectl create secret generic anycode-wechat-certs \
  -n "$NS" \
  --from-file=apiclient_key.pem="$CERT_DIR/apiclient_key.pem" \
  --from-file=pub_key.pem="$CERT_DIR/pub_key.pem" \
  --dry-run=client -o yaml | kubectl apply -f -

echo "OK: secret/anycode-wechat-certs in namespace $NS"
