# ADR 013: Cloud 5h call-quota pricing

## Status

Accepted (2026-06), amended 2026-06

## Context

Competitors (e.g. CodeBuddy) sell **feature gates** (unlimited tabs, edit prediction, preview, custom agents). anyCode keeps those **free locally** and charges for **hosted model gateway** access.

**「5 小时」** does **not** mean a 5-hour wall-clock pass. It means:

- A **rolling 5-hour window**
- **1000 model invocations** allowed per window
- When the window elapses, the **call counter resets** (next window starts)

## Decision

1. Plan **`cloud_5h`**: `calls_limit_per_window = 1000`, `quota_window_secs = 18000` (5h).

2. Entitlements columns: `quota_window_secs`, `calls_limit_per_window`, `calls_used_in_window`, `quota_window_started_at`.

3. **Enforcement**:
   - `gateway/authorize` → `quota::check_call_quota`
   - `gateway/usage` → `quota::record_model_call` (+ token accounting)

4. **Billing**: monthly prepaid (WeChat) for access to this tier; quota resets on schedule, not on payment.

5. **Pro/Team**: token-monthly quotas, no per-window call cap (`calls_limit_per_window = 0`).

## Consequences

- Portal copy:「1000 次 / 每 5 小时重置」
- API returns `calls_remaining`, `quota_resets_at` on authorize and entitlements bundle
- `pass_expires_at` on subscriptions is unused for this model (legacy column)

## Related

- [ADR 012](012-wechat-pay-prepaid-billing.md)
- `crates/account-service/src/quota.rs`
