# Digital Workbench — Control Plane Closure Report

**Date:** 2026-05 · **Final phase:** `v3_week10` · **Status:** Closed for local loopback use

## Frozen scope

This workstream is **complete**. No further Week 11+ control-plane features are in scope unless explicitly replanned.

### Shipped (V3 Week 3–10)

| Capability | Entry points |
|------------|--------------|
| Live cooperative cancel | `POST /api/sessions/{id}/cancel`, `CancelSessionButton` |
| UI trigger run / goal | `POST /api/projects/{id}/runs/trigger`, `ProjectTriggerRunPanel` |
| Web tool approval (file IPC) | `GET/POST /api/security/approvals/*`, `SecurityApprovalInbox` |
| Session-scoped inbox | Session detail when `status=running` |
| Conversations workflow | `?filter=needs_approval`, inline inbox, thread events |
| Security activity log | `GET /api/security/activity`, ingested from `output.log` |

### Explicitly deferred

- Connector OAuth / write actions
- Pending-approval notification policies
- SSO / RBAC
- Tauri desktop shell
- Browser visual gates

## Security boundaries

| Action | Default | Remote override |
|--------|---------|-----------------|
| Web approval respond | Loopback only | `ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE=1` |
| UI trigger run | Loopback only | `ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE=1` |
| Disable Web approval | — | `ANYCODE_DASHBOARD_WEB_APPROVAL=0` |
| Disable UI trigger | — | `ANYCODE_DASHBOARD_TRIGGER_RUN=0` |

State directories under `~/.anycode/dashboard/` (or `ANYCODE_DASHBOARD_STATE_DIR`):

- `active/` — live CLI session registration (cancel)
- `cancel/` — cooperative cancel requests
- `approvals/pending/` — pending tool approvals
- `approvals/responses/` — Web approval decisions
- `triggers/` — UI trigger run logs

Audit actions recorded: `session_cancelled`, `project_run_triggered`, `tool_approval_responded`.

## Closure fixes applied

| Area | Fix |
|------|-----|
| `tasks_run.rs` | Clear `ANYCODE_DASHBOARD_SESSION_ID` when task execution returns `Err` |
| `dashboard_record.rs` | Clear session env when REPL finish lacks `task_id` |
| `workbench_approval.rs` | Clear pending file when TUI wins select race or falls through |
| `approval_ipc` / `cancel_ipc` tests | Shared env lock via `test_util::lock_state_dir_env()` |
| Docs | Updated control-plane scope, STATUS counts, next-steps, this report |

## Manual acceptance scenario

```bash
anycode dashboard --open
# another terminal
anycode repl
# trigger a sensitive tool (e.g. Bash)
```

Expected:

1. Home banner shows pending approval count with link to Conversations.
2. Conversations **Needs approval** filter lists the session.
3. Inline inbox in chat pane supports allow once / allow tool / deny.
4. Session detail scoped inbox works for the same session.
5. CLI continues or denies per decision.
6. Resolved/denied events appear in Security activity after log ingest.

## Residual risks (accepted for local MVP)

| Risk | Mitigation |
|------|------------|
| File IPC stale files if CLI crashes | Pending cleared on Web respond, timeout (30 min), or TUI decision; manual cleanup under `~/.anycode/dashboard/` |
| Non-loopback binding | Requires explicit remote override env vars; doctor warns |
| UI trigger uses `-I` (headless approvals) | Documented in UI copy; not equivalent to interactive REPL approval path |
| Concurrent TUI + Web approval | `tokio::select!` — first response wins; pending file cleared |
| Project filter + needs_approval | Client-side filter on running sessions using summary API |

## Suggested PR split (when committing)

1. Dashboard IPC + API (`approval_ipc`, `cancel_ipc`, `task_trigger`, handlers)
2. CLI integration (`workbench_approval`, `dashboard_record`, `tasks_run`, recorder)
3. Dashboard UI (inbox, Conversations, Home badges)
4. Docs + tests + closure report

## Verification checklist

Run before merge/commit:

```bash
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Record results in this section when executing closure verification.

### Closure verification (2026-05-23)

| Check | Result |
|-------|--------|
| `cargo test -p anycode-dashboard` | **59 passed** (57 unit + 1 fixture + 1 recorder e2e) |
| `cd crates/dashboard-ui && npm test` | **1 passed** |
| `cd crates/dashboard-ui && npm run test:e2e` | **28 passed** |
| `ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui` | **OK** |
| `cargo fmt --all -- --check` | **OK** (after `cargo fmt --all`) |
| `cargo clippy --workspace --all-targets` | **OK** (pre-existing warnings only) |
| `cargo test --workspace` | **OK** |
