# Upstream 账号池（DeepSeek / Agnes）

平台代付用的厂商 API Key 存在 `upstream_accounts` / `upstream_account_keys`（加密），**不是**用户在控制台创建的 Cloud API Key。

## 维护入口

1. **Ops Portal**（推荐）：部署后打开 ops 控制台 →「上游账号池」→ 选择 Provider（`deepseek` / `agnes`）→ 粘贴 API Key。
2. **Admin API**：`POST /api/v1/admin/upstream-accounts`，body 含 `provider_id`、`name`、`api_key`、可选 `base_url`。

环境变量：

- `UPSTREAM_KEY_ENCRYPTION_SECRET` — 加密密钥（必填）
- `DEEPSEEK_API_BASE_URL` — 默认 `https://api.deepseek.com`
- `AGNES_API_BASE_URL` — Agnes 默认 URL

## 本地演示注入 DeepSeek Key

```bash
# 先起 stack
./scripts/dev-account-portal.sh stack

# 用 admin token 创建（替换 KEY 与 TOKEN）
curl -sS -X POST "http://127.0.0.1:43200/api/v1/admin/upstream-accounts" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"provider_id":"deepseek","name":"deepseek-primary","api_key":"sk-..."}'
```

用户侧 Cloud API Key：官网 `/console/api` 或桌面「账号」面板。

**切勿把厂商 Key 提交进 git。**
