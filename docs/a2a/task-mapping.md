# A2A Task ↔ anyCode Handoff Mapping

Reference for [ADR 016](../adr/016-cloud-a2a-team-handoff.md). P1 uses anyCode REST + WebSocket stream; P2 adds JSON-RPC `tasks/send` compatibility.

## Task kinds

| anyCode `HandoffKind` | A2A `Task.metadata.handoff_kind` | Bundle scope |
|----------------------|----------------------------------|--------------|
| `project` | `anycode.handoff.project` | Full workspace + project sessions/memories/artifacts |
| `session` | `anycode.handoff.session` | Single session + artifacts; optional target project |

## State machine

| anyCode `HandoffState` | A2A Task `status` | Notes |
|------------------------|---------------------|-------|
| `pending_approval` | `submitted` | Waiting recipient UI approve |
| `approved` | `working` | Stream token issued |
| `uploading` | `working` | Sender pumping bundle via relay |
| `importing` | `working` | Receiver importing `handoff_v1` |
| `completed` | `completed` | Audit + cleanup |
| `rejected` | `canceled` | User rejected |
| `failed` | `failed` | I/O or import error |
| `expired` | `failed` | TTL exceeded (`metadata.reason=expired`) |

## REST (P1) ↔ JSON-RPC (P2)

| P1 anyCode endpoint | P2 A2A JSON-RPC method | Direction |
|---------------------|------------------------|-----------|
| `POST /a2a/handoff/request` | `tasks/send` | Client → server |
| `GET /a2a/handoff/{id}/status` | `tasks/get` | Poll |
| `POST /a2a/handoff/{id}/approve` | `tasks/cancel` (inverse flow) | Recipient |
| `WS /a2a/handoff/{id}/stream` | `tasks/sendSubscribe` (artifact stream) | Bidirectional bytes |

## Message / artifact mapping

| Payload | A2A artifact | anyCode field |
|---------|--------------|---------------|
| Bundle manifest JSON | `Artifact` `mimeType: application/json` | `BundleManifest` |
| gzip tar bytes | `Artifact` `mimeType: application/gzip` | stream body |
| Progress | `TaskStatusUpdateEvent` | `progress_pct` 0–100 |

## Agent Card capabilities

| Capability | Required for |
|------------|--------------|
| `handoff.project` | Project handoff wizard |
| `handoff.session` | Session handoff wizard |
| `streaming.relay` | Cloud WS relay (P1) |
| `tasks.send` | JSON-RPC tasks (P2) |

## Headers (P2)

| Header | Value |
|--------|-------|
| `A2A-Version` | `0.1` (negotiate in P2) |
| `Authorization` | `Bearer <cloud session>` |
| `X-Anycode-Instance-Id` | Sender/recipient `instance_id` |

## Error codes

| HTTP | anyCode code | A2A mapping |
|------|--------------|-------------|
| 401 | `unauthorized` | `-32001` |
| 403 | `forbidden_org` | `-32003` |
| 404 | `handoff_not_found` | `-32004` |
| 409 | `invalid_state` | `-32009` |
| 413 | `bundle_too_large` | `-32010` |
| 410 | `stream_expired` | `-32011` |
