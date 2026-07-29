# ADR 016: Cloud A2A Team Handoff (Streaming Relay, No OSS)

## Status

Accepted (2026-07)

## Context

ADR 015 covers LAN colleague handoff (mDNS + direct HTTP upload on port 43181). Remote teammates cannot use LAN discovery or private-IP routes. Uploading bundles to object storage (OSS/S3) adds cost, compliance surface, and breaks the「数据不出端」product posture for project payloads.

We need a **cloud relay** on `anycode.work` that:

1. Reuses org identity from ADR 011 (device link, `/org/members`).
2. Preserves ADR 015 semantics: explicit approval, one-time stream token, `handoff_v1` bundle, no credentials in payload.
3. Transfers bundle bytes over a **long-lived streaming connection** — **no OSS**, relay holds bytes in memory only for the active session.
4. Aligns with [Google A2A](https://google.github.io/A2A/) concepts (Agent Card, Task lifecycle) for future third-party agents, without requiring full JSON-RPC in P1.

## Decision

### Architecture

```
┌─────────────┐   REST signaling    ┌──────────────────┐   REST signaling   ┌─────────────┐
│ Sender      │◄──────────────────►│ account-service  │◄──────────────────►│ Receiver    │
│ (Desktop)   │   WS/HTTP stream    │  A2A relay       │   WS/HTTP stream   │ (Desktop)   │
└─────────────┘   (chunk pipe)      └──────────────────┘   (chunk pipe)     └─────────────┘
       │                                    │                                      │
       └──────── handoff_v1 gzip tar ───────┴──── in-memory pipe only ──────────────┘
```

- **Signaling** (REST, Bearer auth): presence heartbeat, handoff request/approve/reject/status.
- **Transport** (long connection): WebSocket binary frames on `/api/v1/a2a/handoff/{id}/stream` (primary; auth via one-time `token` query — **not** Bearer). HTTP chunked GET/POST fallback for constrained proxies (P1.1).
- **Relay correctness**: chunks are **buffered with replay** so a late receiver still gets the full payload; empty frame = EOF (only via `publish_eof`). Sender marks `importing` after EOF; **receiver** marks `completed`.
- **Cloud bundle cap**: in-memory replay buffer defaults to **64 MiB** (aligned with Desktop cloud export). Larger projects use LAN or raise the shared constant later.
- **Storage**: MySQL stores session **metadata only** (parties, state, token hash + short-lived ephemeral stream token for status polls). Bundle bytes **never** written to DB or OSS; relay buffer is in-process memory with TTL and size cap.
- **LAN fast path** (ADR 015) remains; cloud path is selected when peer `transport: "cloud"`.

### Agent Card

Each Desktop publishes an [Agent Card](../a2a/agent-card.schema.json) on heartbeat:

- `instance_id`, `device_id`, `organization_id`, `capabilities` (`handoff.project`, `handoff.session`).
- Served at `GET /api/v1/a2a/agents/{instance_id}/card` (P2: well-known URL).

See `docs/a2a/agent-card.schema.json`.

### Task mapping (A2A ↔ anyCode)

See `docs/a2a/task-mapping.md`. Summary:

| A2A Task state | anyCode `HandoffState` |
|----------------|------------------------|
| `submitted` | `pending_approval` |
| `working` | `approved` / `uploading` / `importing` |
| `completed` | `completed` |
| `failed` | `failed` / `expired` |
| `canceled` | `rejected` |

### Security

- Same org (`organization_id`) required for handoff parties.
- Recipient must approve in Desktop UI before stream token is issued.
- Stream token: single-use, 5-minute TTL, passed as query param on WS upgrade (not logged in access logs when possible).
- Audit: `a2a_handoff_requested`, `approved`, `rejected`, `stream_started`, `completed`.
- P3: E2EE bundle wrapper (optional `handoff_v1+e2ee`).

### API surface (P1)

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/v1/a2a/presence/heartbeat` | Publish Agent Card + online status |
| GET | `/api/v1/a2a/team/peers` | Org members + online devices (Portal + Desktop) |
| POST | `/api/v1/a2a/handoff/request` | Create handoff task |
| GET | `/api/v1/a2a/handoff/incoming` | Pending incoming for this device |
| GET | `/api/v1/a2a/handoff/outgoing` | Outgoing status |
| POST | `/api/v1/a2a/handoff/{id}/approve` | Recipient approves |
| POST | `/api/v1/a2a/handoff/{id}/reject` | Recipient rejects |
| GET | `/api/v1/a2a/handoff/{id}/status` | Poll progress |
| WS | `/api/v1/a2a/handoff/{id}/stream?role=sender\|receiver&token=` | Binary chunk pipe |
| GET/POST | `/api/v1/a2a/handoff/{id}/stream/http` | Chunked HTTP fallback |

Desktop loopback proxy: `/api/cloud/a2a/*` forwards to account-service with cloud bearer token.

### Phasing

| Phase | Scope | Duration |
|-------|--------|----------|
| **P0** | This ADR + Agent Card schema + Task mapping | 1 week |
| **P1** | account-service relay + Portal「团队」+ Desktop cloud handoff (custom REST, no JSON-RPC) | 3–4 weeks |
| **P2** | JSON-RPC `tasks/send`, Agent Card registry, `A2A-Version` negotiation | 2–3 weeks |
| **P3** | E2EE bundle, third-party agents, SSO/RBAC | later |

### Non-goals (P1)

- OSS/S3 staging of bundles.
- Full A2A JSON-RPC server.
- Cross-org handoff.
- Persistent relay queue (offline recipient must come online within TTL).

## Consequences

- Single-replica relay is in-memory; multi-replica K8s requires sticky sessions or shared relay crate (future).
- Large bundles (>500MB default) need raised cap + longer stream timeout; sender backpressure if receiver slow.
- GitHub/deploy: no payment-style secrets in image; same pattern as ADR 016 WeChat runtime secrets.

## References

- ADR 011 Cloud Account Platform
- ADR 015 LAN Colleague Handoff
- [A2A Protocol Specification](https://google.github.io/A2A/)
- `docs/a2a/agent-card.schema.json`
- `docs/a2a/task-mapping.md`
