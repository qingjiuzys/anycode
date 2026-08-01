#!/usr/bin/env bash
# Apply runtime secrets for anycode.work account-service (WeChat Pay + DB).
# Requires: kubectl context → dis-cloud (or pass namespace arg), deploy/account-service/.env
#
# Usage:
#   ./scripts/apply-k8s-account-secrets.sh dis-cloud
#   ./scripts/apply-k8s-wechat-secret.sh dis-cloud   # PEM volume only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NS="${1:-dis-cloud}"
ENV_FILE="${ENV_FILE:-$ROOT/.env}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing $ENV_FILE — copy env.example and fill WeChat + DATABASE_URL" >&2
  exit 1
fi

set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

required=(DATABASE_URL WECHAT_PAY_API_V3_KEY WECHAT_PAY_APP_ID WECHAT_PAY_MCH_ID WECHAT_PAY_SERIAL_NO)
for key in "${required[@]}"; do
  if [[ -z "${!key:-}" ]]; then
    echo "Missing $key in $ENV_FILE" >&2
    exit 1
  fi
done

echo "==> secret/anycode-account-secrets (namespace $NS)"
kubectl create secret generic anycode-account-secrets \
  -n "$NS" \
  --from-literal=DATABASE_URL="$DATABASE_URL" \
  --from-literal=WECHAT_PAY_API_V3_KEY="$WECHAT_PAY_API_V3_KEY" \
  --from-literal=WECHAT_PAY_APP_ID="$WECHAT_PAY_APP_ID" \
  --from-literal=WECHAT_PAY_MCH_ID="$WECHAT_PAY_MCH_ID" \
  --from-literal=WECHAT_PAY_SERIAL_NO="$WECHAT_PAY_SERIAL_NO" \
  ${SMTP_PASSWORD:+--from-literal=SMTP_PASSWORD="$SMTP_PASSWORD"} \
  ${IDENTITY_ENCRYPTION_SECRET:+--from-literal=IDENTITY_ENCRYPTION_SECRET="$IDENTITY_ENCRYPTION_SECRET"} \
  ${AUDIT_ENCRYPTION_SECRET:+--from-literal=AUDIT_ENCRYPTION_SECRET="$AUDIT_ENCRYPTION_SECRET"} \
  --dry-run=client -o yaml | kubectl apply -f -

echo "==> secret/anycode-wechat-certs"
"$ROOT/scripts/create-k8s-wechat-secret.sh" "$NS"

DEPLOY="${ANYCODE_K8S_DEPLOYMENT:-anycode-account}"
if ! kubectl get deploy "$DEPLOY" -n "$NS" >/dev/null 2>&1; then
  DEPLOY="anycode"
fi
echo "==> rollout restart deployment/$DEPLOY"
kubectl rollout restart "deployment/$DEPLOY" -n "$NS"
kubectl rollout status "deployment/$DEPLOY" -n "$NS" --timeout=180s

echo "==> verify https://anycode.work/health"
curl -sS https://anycode.work/health | python3 -m json.tool | rg 'wechat_pay' || true
