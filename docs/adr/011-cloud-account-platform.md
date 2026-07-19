# ADR 011: Cloud account platform, device link, and hybrid model gateway

## Status

Accepted (2026-06)

## Context

anyCode 0.3 initially shipped a **local Workbench `/account` shell** plus a headless `anycode-account` REST API on port 43200. Product intent requires:

1. A **hosted cloud web portal** (login, plans, billing, model marketplace) — not only API endpoints.
2. **Cloud-first identity**: users sign in on the portal; the desktop/CLI app links via device authorization, not embedded email/password forms in the local UI.
3. **Hybrid models**: Free tier remains BYOK; paid tiers unlock **anyCode-hosted inference** via a model gateway with quota metering.
4. **Real subscription billing** (Stripe; WeChat Pay later).

[ADR 003](003-http-daemon-deprecated.md) deprecates an in-CLI HTTP task daemon. A **separate cloud model-gateway service** is not that daemon: it proxies LLM inference only; Agent tool execution stays local.

## Decision

1. **Three cloud deployables** (local dev ports in parentheses):
   - `anycode-account` — account API + serves Account Portal static UI (`:43200`)
   - `anycode-model-gateway` — OpenAI-compatible inference proxy (`:43210`)
   - PostgreSQL — entitlements, billing, device links, usage events

2. **Identity flow**: RFC 8628–style device link (`POST /api/v1/devices/link/start` → `anycode://link?code=…` → poll). Session stored in `~/.anycode/cloud-session.json` on the client.

3. **Local Workbench `/account`**: read-only summary + link to cloud portal; no inline cloud register/login.

4. **Hybrid LLM**: new provider id `anycode-cloud` in `crates/llm` routes to model-gateway with cloud API key or device session token.

5. **Billing**: Stripe Checkout + webhooks update `subscriptions`; mock upgrade retained for dev without Stripe keys.

## Consequences

- [WorkBuddy comparison](../comparisons/workbuddy-comparison-2026-06.md) **Skip** on built-in Credits is superseded for **paid cloud inference** only; BYOK remains for free tier.
- `crates/account-service/` and `deploy/account-service/` are versioned in git.
- New crate `crates/account-portal/` (SPA) and `crates/model-gateway/`.
- Desktop registers `anycode://` URL scheme for device link callbacks.

## Related

- [digital-workbench-api.md](../workbench/digital-workbench-api.md)
- [roadmap.md](../roadmap.md) §3.5 (updated)
- [003-http-daemon-deprecated.md](003-http-daemon-deprecated.md) — still no in-CLI task HTTP API
