# Dashboard control plane (V3 Week 3–10)

**Scope:** Approved local control-plane actions from the Web UI: session cancel, UI trigger run, and interactive tool approval. These update SQLite/audit state and, when noted, signal a live `anycode` CLI via file IPC.

## Session cancel (`POST /api/sessions/{id}/cancel`)

| Aspect | Behavior |
|--------|----------|
| **Effect** | Sets `status = cancelled`, `ended_at = now` when session was `running` |
| **Live CLI** | When `~/.anycode/dashboard/active/{session_id}.json` exists (recorder registered), writes `~/.anycode/dashboard/cancel/{session_id}.json`; CLI tail loops poll every ~300ms and set the cooperative cancel flag |
| **Stale runs** | No active registration → DB-only cancel (`live_signal: false`) |
| **Audit** | `session_cancelled` in audit + project event with `live_signal` in payload |
| **Trust** | Refreshes trusted status; may trigger automation policies |
| **UI** | Cancel button; alert when `live_signal: true` |

Response: `{ "ok": true, "session_id": "...", "live_signal": true|false }`

Use DB-only cancel for orphaned `running` rows. Live signal stops cooperative runs at turn/tool boundaries (same as Ctrl+C).

## Gate runner `required` flag

Manual gate runs from the project page can set `required: true` in `POST .../gates/execute`. Failed required gates block delivery trust (same as ingested gates).

## Gate streaming (`POST .../gates/execute/stream`)

Returns SSE events while the shell command runs: `line` chunks then `done` with `GateExecuteResult`. UI gate runner uses this for live log tail.

## Connector doctor probes

`GET /api/settings/doctor` runs optional reachability checks for enabled GitHub/Linear connectors (5s timeout). Missing API tokens → `warn`; failed probe → `error`.

## Not implemented (Tier 2+)

- Browser visual gates · SSO/RBAC · Connector OAuth/write-back

**Shipped separately:** macOS Tauri desktop v0.1 (`apps/anycode-desktop`) wraps embedded Workbench via dashboard sidecar — not a second runtime.

See [digital-workbench-permissions.md](digital-workbench-permissions.md) and [digital-workbench-next-steps.md](../workbench/digital-workbench-next-steps.md).

## Web tool approval (V3 Week 8)

| Aspect | Behavior |
|--------|----------|
| **Pending file** | `~/.anycode/dashboard/approvals/pending/{approval_id}.json` when CLI registers pending approval |
| **Response file** | `~/.anycode/dashboard/approvals/responses/{approval_id}.json` written by dashboard API |
| **CLI poll** | `WorkbenchApprovalCallback` polls every ~400ms when `ANYCODE_DASHBOARD_SESSION_ID` is set |
| **API** | `GET /api/security/approvals/pending`, `POST /api/security/approvals/{id}/respond` |
| **Decisions** | `allow_once`, `allow_tool`, `deny` |
| **Binding** | Loopback-only unless `ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE=1`; disable with `ANYCODE_DASHBOARD_WEB_APPROVAL=0` |
| **Audit** | `tool_approval_responded` |

Stream REPL / run / goal set `ANYCODE_DASHBOARD_SESSION_ID` while the dashboard recorder is active.

**V3 Week 9:** Session detail shows a scoped inbox when `status=running`. Home running table shows per-session pending badges. `GET /api/security/approvals/summary` powers counts. Approved tools log `[tool_approval_resolved]` for the activity timeline.

**V3 Week 10:** Conversations page adds **Needs approval** filter chip, session list badges, inline approval inbox in the chat pane, and a Home banner linking to `/conversations?filter=needs_approval`. Thread view surfaces `tool_approval_*` / `tool_denied` events.

## UI trigger run (`POST /api/projects/{id}/runs/trigger`)

Spawns a **detached** `anycode run -I -C {project_root}` subprocess (goal mode adds `--goal`). Loopback-only unless `ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE=1`. Disable with `ANYCODE_DASHBOARD_TRIGGER_RUN=0`. Logs: `~/.anycode/dashboard/triggers/{trigger_id}.log`. Dashboard recorder picks up the session automatically.

## Security activity (`GET /api/security/activity`)

Historical observability for `tool_denied` and `tool_approval_pending` events ingested from `output.log`. Live pending approvals use the Web inbox below.
