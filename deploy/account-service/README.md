# anyCode Account Cloud Service

Central account, subscription, billing, model marketplace, and device link API for v0.3+ cloud platform. See [ADR 011](../../docs/adr/011-cloud-account-platform.md).

## Agnes Cloud Gateway + 账号池

迁移（已有库执行一次）：

```bash
mysql -h HOST -u USER -p anycode < crates/account-service/migrations/006_upstream_account_pool.sql
```

关键环境变量：

```bash
export ANYCODE_MODEL_GATEWAY_URL=https://gateway.818cloud.com
export UPSTREAM_KEY_ENCRYPTION_SECRET=<32+ char secret>
export AGNES_API_BASE_URL=https://apihub.agnes-ai.com/v1/chat/completions
export ADMIN_BOOTSTRAP_EMAIL=ops@example.com
export ADMIN_BOOTSTRAP_PASSWORD=<initial-password>
export OPS_PORTAL_DIR="$(pwd)/crates/ops-portal/dist"
```

构建运营平台：

```bash
cd crates/ops-portal && npm ci && npm run build
```

访问 **/ops/**（由 account-service 托管）管理 Agnes 账号池、模型目录与健康事件。本地 CLI 使用 `anycode_cloud` + `anycode auth login`，默认模型 `agnes-chat`。

## Quick start (local)

From repository root:

```bash
# PostgreSQL
docker compose -f deploy/account-service/docker-compose.yml up postgres -d

# Build Account Portal UI
cd crates/account-portal && npm ci && npm run build

# Account API + Portal (serves dist at http://127.0.0.1:43200/)
export DATABASE_URL=postgres://anycode:anycode@127.0.0.1:5432/anycode_account
export ACCOUNT_PORTAL_DIR="$(pwd)/crates/account-portal/dist"
export ACCOUNT_PORTAL_URL=http://127.0.0.1:43200
cargo run -p anycode-account-service

# Model gateway (optional; needs upstream API keys)
export ANYCODE_ACCOUNT_API_URL=http://127.0.0.1:43200
export ZAI_API_KEY=...
cargo run -p anycode-model-gateway
```

Open **http://127.0.0.1:43200/** — cloud portal (not a bare API 404).

## Deploy to anycode.work (image includes portal + DMG)

One command on **macOS** (signed DMG → portal `public/downloads` → Docker image → ACR):

```bash
./scripts/build-account-image.sh
# or pin tag: TAG=0.2.4 ./scripts/build-account-image.sh
```

Then roll K8s:

```bash
kubectl set image deployment/anycode-account anycode-account=registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:0.2.4
kubectl rollout status deployment/anycode-account
```

Download URLs (same deployment, no extra upload):

- https://anycode.work/downloads/anyCode_latest_aarch64.dmg
- https://anycode.work/downloads/latest.json

DMG-only refresh without rebuilding API image: still run full image build (portal stage embeds DMG).

See [docs/ops/desktop-release-local.md](../../docs/ops/desktop-release-local.md).

1. Sign in on the portal → **设备** → **打开 anyCode 桌面应用**
2. Desktop receives `anycode://link?code=...` or run: `anycode auth link --code <code>`
3. Session stored in `~/.anycode/credentials/cloud-session.json`
4. Workbench `/account` reads linked session via `GET /api/cloud/session`

## Stripe (optional, international)

```bash
export STRIPE_SECRET_KEY=sk_test_...
export STRIPE_WEBHOOK_SECRET=whsec_...
export STRIPE_PRICE_PRO=price_...
export STRIPE_PRICE_TEAM=price_...
```

Portal **Plans** → **Stripe** creates a Checkout Session (recurring subscription).

## WeChat Pay (v3.0, China — prepaid)

### 回调地址（填到微信商户平台）

| 用途 | URL |
|------|-----|
| **支付结果通知（必配）** | `https://anycode.work/api/v1/billing/webhooks/wechat` |
| 账号门户 | `https://anycode.work` |

微信要求 **HTTPS** 且公网可达；`http://anycode.work` 仅作跳转，回调必须用 `https://`。

在 [微信支付商户平台](https://pay.weixin.qq.com/) → 产品中心 → 开发配置 → **Native 支付** / **支付回调 URL** 填入上表 notify 地址。

### 本地配置

```bash
cd deploy/account-service
cp env.example .env
# 编辑 .env：WECHAT_PAY_API_V3_KEY、WECHAT_PAY_APP_ID、WECHAT_PAY_MCH_ID、WECHAT_PAY_SERIAL_NO
# （勿提交 .env；值从微信商户平台获取）

./scripts/sync-wechat-certs.sh   # → secrets/apiclient_key.pem + pub_key.pem（gitignore）
```

`.env` 需包含（示例键名，见 `env.example`）：

- `WECHAT_PAY_APP_ID`
- `WECHAT_PAY_MCH_ID`
- `WECHAT_PAY_SERIAL_NO`
- `WECHAT_PAY_API_V3_KEY`
- 私钥 / 平台公钥 → `secrets/apiclient_key.pem`、`secrets/pub_key.pem`（**仅本地 / K8s Secret，不进镜像**）

Docker 启动：

```bash
cd deploy/account-service
docker compose up --build -d
```

反向代理需把 `https://anycode.work` 转到 `account-service:43200`（含 `/api/v1/billing/webhooks/wechat`）。

Dev without TLS (local notify testing only):

```bash
export WECHAT_PAY_SKIP_VERIFY=1
```

See [ADR 012](../../docs/adr/012-wechat-pay-prepaid-billing.md).

## Workbench integration

```bash
export ANYCODE_ACCOUNT_API_URL=http://127.0.0.1:43200
export ANYCODE_ACCOUNT_PORTAL_URL=http://127.0.0.1:43200
export ANYCODE_MODEL_GATEWAY_URL=http://127.0.0.1:43210
anycode dashboard --open
```

Local `/account` links to the cloud portal; use `anycode auth login` for CLI.

## Docker image (Aliyun ACR)

Account API + Account Portal SPA in one image. Container listens on **8080**; map K8s/Ingress **80 → 8080**.

```bash
# Login once (Aliyun ACR)
docker login registry.cn-zhangjiakou.aliyuncs.com

# Build & push
chmod +x deploy/account-service/build-push.sh
./deploy/account-service/build-push.sh
# → registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:0.2.3
# → registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:latest
```

K8s reference: [`k8s/postgres.yaml`](k8s/postgres.yaml), [`k8s/secret.yaml`](k8s/secret.yaml), [`k8s/deployment.yaml`](k8s/deployment.yaml), [`k8s/ingress.yaml`](k8s/ingress.yaml)

**K8s quick start (`dis-cloud` namespace)**

```bash
NS=dis-cloud

# 1) Postgres (skip if you already have RDS / external Postgres)
kubectl apply -f deploy/account-service/k8s/postgres.yaml -n $NS
kubectl wait --for=condition=ready pod -l app=anycode-postgres -n $NS --timeout=120s

# 2) Secrets — fill secret.yaml locally OR use kubectl (never commit real values)
kubectl apply -f deploy/account-service/k8s/secret.yaml -n $NS   # template only; edit first

# 2b) WeChat PEM volume secret (runtime mount /app/wechat-certs)
chmod +x deploy/account-service/scripts/create-k8s-wechat-secret.sh
./deploy/account-service/scripts/create-k8s-wechat-secret.sh $NS

# 2c) One-shot: sync .env secrets + WeChat PEMs + rollout (requires deploy/account-service/.env)
chmod +x deploy/account-service/scripts/apply-k8s-account-secrets.sh
./deploy/account-service/scripts/apply-k8s-account-secrets.sh $NS

# 3) App + Service
kubectl apply -f deploy/account-service/k8s/deployment.yaml -n $NS

# 4) Ingress (TLS for anycode.work)
kubectl apply -f deploy/account-service/k8s/ingress.yaml -n $NS
```

If your Deployment is named `anycode` (not `anycode-account`), ensure the container has:

```yaml
envFrom:
  - secretRef:
      name: anycode-account-env
```

One-liner to create/update the Secret only:

```bash
kubectl create secret generic anycode-account-env \
  --from-literal=DATABASE_URL='postgres://anycode:anycode@anycode-postgres:5432/anycode_account' \
  -n dis-cloud --dry-run=client -o yaml | kubectl apply -f -
kubectl rollout restart deployment/anycode -n dis-cloud
```

**Deploy checklist**

1. Set `DATABASE_URL`, `WECHAT_PAY_API_V3_KEY`, `WECHAT_PAY_APP_ID`, `WECHAT_PAY_MCH_ID`, `WECHAT_PAY_SERIAL_NO` in Secret `anycode-account-secrets`.
2. Create Secret `anycode-wechat-certs` with PEM files (`create-k8s-wechat-secret.sh`).
3. TLS: apply Ingress with cert for `anycode.work` — app does not terminate SSL.
4. WeChat notify URL: `https://anycode.work/api/v1/billing/webhooks/wechat` (set in merchant platform).
5. Verify: `curl -s https://anycode.work/health | jq .wechat_pay_configured` → `true`.

**ImagePullBackOff / `image ... not found`**

- Rebuild with `./deploy/account-service/build-push.sh` (forces `linux/amd64`, disables buildx attestation for ACR compatibility).
- Verify manifest has layers (not an empty OCI index): `docker manifest inspect registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:0.2.3` — should list `layers` with sizes.
- Verify pull: `docker pull --platform linux/amd64 registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:0.2.3`
- ACR console: if **镜像 ID / 大小** are blank, the push was likely an incompatible OCI attestation index; re-run `build-push.sh` and refresh the console.
- **Private ACR**: Aliyun often returns “not found” when the node is not logged in. Create pull secret in `dis-cloud` and uncomment `imagePullSecrets` in `deployment.yaml`:

```bash
kubectl create secret docker-registry aliyun-acr \
  --docker-server=registry.cn-zhangjiakou.aliyuncs.com \
  --docker-username=<acr-user> --docker-password=<acr-password> \
  -n dis-cloud
kubectl rollout restart deployment/anycode-account -n dis-cloud
```

## Docker (local compose)

```bash
docker compose -f deploy/account-service/docker-compose.yml up --build
```

Services: Postgres `:5432`, account `:43200`, model-gateway `:43210`.
