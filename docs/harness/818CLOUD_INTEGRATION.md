# 818cloud native integration

Authoritative source: qingshan-opc/818cloud at e8c9b49eafd12a69cf2007e59cd25b3d9b52683e, product_sso.rs and platform/sso.rs.

## Existing and confirmed

The server exposes /api/v2/sso/authorize, /token, /introspect and /revoke under the SSO prefix. Authorization uses exact registered callback, state and S256. Confidential client authentication belongs to the product server. Introspection returns active/iss/sub/aud/context/tenant/scope, with organization/member/tenant/grant versions in enterprise context. It is not an advertised OIDC implementation.

## Added here

A server-only typed introspection client, strict identity/context validation, a mandatory ProductAcl interface, and an idempotent usage-fact DTO. No product database, tenant mapping query or wallet operation is invented.

## Still required

Implement ProductAcl against actual AnyCode project/user/tenant storage. Re-introspect/re-authorize before each consequential operation, not only initial login. Reject wrong audience, revoked grant and scope mismatch. Enterprise account authority does not imply desktop device consent.

Reuse product_sso gateway for web flows. Desktop pairing, opaque short-lived device credentials, keychain storage, broker transport and revocation are PROPOSED, not endpoints claimed to exist. Keep client secret on server only. No client_secret in Tauri configuration, JS bundle, skill, logs or model context.

Implement transactional usage outbox and existing-wallet settlement after agreeing a real platform API. Current UsageReceipt is not sufficient for financial billing reconciliation. Never duplicate provider usage at both parent aggregate and child detail levels.

This bundle does not change 818cloud source, production infrastructure, users, wallets or orders. Apply no destructive migration. Tenant/product separation must remain consistent with FDE, EOS and other products rather than merging their business models into AnyCode.
