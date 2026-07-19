# ADR 012: WeChat Pay prepaid billing (v3.0)

## Status

Accepted (2026-06)

## Context

[ADR 011](011-cloud-account-platform.md) chose Stripe Checkout for initial cloud billing with "WeChat Pay later". Product direction for v3.0:

1. **Real production billing** for the China market via **WeChat Pay API v3** (Native 扫码).
2. **Dual payment channels**: WeChat default domestically; Stripe retained for international users.
3. **Prepaid first**: monthly/yearly prepaid packages; **委托代扣** (contract deduction / auto-renew) deferred to a follow-up.

Stripe integration remains subscription-based. WeChat uses prepaid period extension on `subscriptions.period_end`.

## Decision

1. **`payment_orders` table** records pending/paid checkout attempts (`provider`, `out_trade_no`, `code_url`, amounts).

2. **`billing_wechat` module** (`crates/account-service/src/billing_wechat.rs`):
   - Native pay: `POST /v3/pay/transactions/native` → `code_url` QR
   - Notify: `POST /api/v1/billing/webhooks/wechat` with APIv3 AES-GCM decrypt
   - On `TRANSACTION.SUCCESS`, extend plan period via `billing::activate_prepaid_period`

3. **Unified checkout API**: `POST /api/v1/billing/checkout` with `{ plan, provider: "wechat"|"stripe", cycle: "monthly"|"yearly" }`.

4. **Expiry**: WeChat prepaid subscriptions downgrade to `free` when `period_end` passes (`billing::refresh_subscription_status`).

5. **Portal**: provider toggle, WeChat QR modal with order polling (`GET /api/v1/billing/orders/:id`).

6. **Environment** (see `deploy/account-service/README.md`):
   - `WECHAT_PAY_APP_ID`, `WECHAT_PAY_MCH_ID`, `WECHAT_PAY_SERIAL_NO`
   - `WECHAT_PAY_PRIVATE_KEY` or `WECHAT_PAY_PRIVATE_KEY_PATH`
   - `WECHAT_PAY_API_V3_KEY`, `WECHAT_PAY_NOTIFY_URL` (or `ACCOUNT_PUBLIC_URL`)
   - `WECHAT_PAY_PLATFORM_CERT` for production notify signature verification
   - `WECHAT_PRICE_PRO_MONTHLY_CENTS`, `WECHAT_PRICE_TEAM_MONTHLY_CENTS`

## Consequences

- Invoices gain optional `amount_cny` and `payment_order_id`.
- `subscriptions.payment_provider` and `prepaid_until` track prepaid state.
- Mock upgrade remains for dev without merchant credentials.
- Contract deduction (auto-renew) is a future ADR; no schema blocker.

## Related

- [ADR 011](011-cloud-account-platform.md)
- `crates/account-service/migrations/003_wechat_pay.sql`
