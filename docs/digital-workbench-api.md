# Digital Workbench API Contract

Base URL for local V1:

```text
http://127.0.0.1:<dashboard_port>
```

All write/control endpoints are out of scope for the first implementation slice unless explicitly marked safe.

## Authentication

V1 local mode:

- Trusted local mode allowed only when bound to `127.0.0.1`.
- API token is required if binding to any non-loopback address.

Future enterprise mode:

- Cookie session for UI.
- Bearer token for API.
- OIDC / SSO optional.

## Endpoints

### Health

```http
GET /api/health
```

Response:

```json
{
  "ok": true,
  "version": "0.2.0",
  "db_path": "~/.anycode/projects.db",
  "mode": "local"
}
```

### Projects

```http
GET /api/projects
```

Response:

```json
{
  "projects": [
    {
      "id": "proj_01",
      "name": "test-apps 自动化交付",
      "root_path": "/path/to/workspace",
      "status": "active",
      "trust_score": 0.62,
      "sessions_count": 12,
      "artifacts_count": 96,
      "updated_at": "2026-05-21T10:00:00Z"
    }
  ]
}
```

```http
GET /api/projects/:project_id
```

### Sessions

```http
GET /api/projects/:project_id/sessions?limit=50
```

Response:

```json
{
  "sessions": [
    {
      "id": "sess_01",
      "kind": "goal",
      "task_id": "b727e70c-2804-4c5c-b609-694e265e8900",
      "title": "Provider 修复",
      "status": "running",
      "trusted_status": "unverified",
      "agent_type": "goal",
      "model": "gemma-4-31b-it",
      "started_at": "2026-05-21T04:09:05Z"
    }
  ]
}
```

```http
GET /api/sessions/:session_id
```

```http
GET /api/sessions/:session_id/usage
```

Response: `{ "usage": TokenUsageStats, "by_model": ModelUsageRow[] }` (session-scoped, all LLM events).

```http
POST /api/sessions/:session_id/cancel
```

Marks a `running` session as `cancelled` in the dashboard DB. When the session has a live CLI registration (`~/.anycode/dashboard/active/{session_id}.json`), also writes a cooperative cancel request for the recorder tail loop. Returns `409` if not running.

Response: `{ "ok": true, "session_id": "...", "live_signal": true|false }`

### Security activity (historical log)

```http
GET /api/security/activity?limit=50&project_id=<optional>
```

Response:

```json
{
  "summary": {
    "denied_total": 2,
    "pending_total": 1,
    "read_only": false,
    "note": "Historical log from output.log. Live pending approvals appear in the Security inbox above.",
    "recent": [
      {
        "id": "evt_…",
        "project_id": "proj_…",
        "project_name": "my-app",
        "session_id": "sess_…",
        "event_type": "tool_denied",
        "severity": "warn",
        "title": "Bash denied",
        "tool_name": "Bash",
        "reason": "User denied",
        "occurred_at": "2026-05-22T12:00:00Z"
      }
    ]
  }
}
```

Events are ingested from `[tool_denied]` and `[tool_approval_pending]` lines in `output.log`.

### Web tool approval inbox (live)

```http
GET /api/security/approvals/pending?limit=20
POST /api/security/approvals/:approval_id/respond
```

List response:

```json
{
  "pending": [
    {
      "approval_id": "apr_…",
      "session_id": "sess_…",
      "tool": "Bash",
      "input_preview": "{ … }",
      "created_at": "2026-05-22T12:00:00Z",
      "status": "pending"
    }
  ],
  "web_enabled": true,
  "respond_allowed": true
}
```

Respond body: `{ "decision": "allow_once" | "allow_tool" | "deny" }`

Loopback-only unless `ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE=1`. Disable with `ANYCODE_DASHBOARD_WEB_APPROVAL=0`. CLI sets `ANYCODE_DASHBOARD_SESSION_ID` while the dashboard recorder is active.

Filter by session: `GET .../pending?session_id=sess_…`

Summary for badges: `GET /api/security/approvals/summary` → `{ pending_total, by_session: [{ session_id, count }] }`

Resolved approvals are logged as `[tool_approval_resolved] name=…` in `output.log` and appear in security activity.

### UI trigger run

```http
POST /api/projects/:project_id/runs/trigger
GET /api/projects/:project_id/runs/triggers?limit=10
```

Trigger body:

```json
{
  "prompt": "Fix the failing unit test in src/foo.rs",
  "kind": "run",
  "goal": "optional when kind=goal",
  "agent": "general"
}
```

Response: `{ "trigger": TriggerRunResult }` with `pid`, `command_preview`, `log_path`. Spawns detached `anycode run -I -C {root}`. Loopback-only unless `ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE=1`.

### Events

```http
GET /api/sessions/:session_id/events?after=<event_id>&limit=200
```

```http
GET /api/projects/:project_id/events/stream
Accept: text/event-stream
```

SSE event:

```text
event: project_event
data: {"id":"evt_01","event_type":"tool_call_end","title":"Bash finished","severity":"info"}
```

### Gates

```http
GET /api/projects/:project_id/gates
GET /api/sessions/:session_id/gates
```

Response:

```json
{
  "gates": [
    {
      "id": "gate_01",
      "name": "flutter test",
      "status": "failed",
      "required": true,
      "output_excerpt": "ProviderNotFoundException...",
      "session_id": "sess_01"
    }
  ]
}
```

### Artifacts

```http
GET /api/projects/:project_id/artifacts?kind=file
```

Response:

```json
{
  "artifacts": [
    {
      "id": "art_01",
      "path": "test/app-02/lib/main.dart",
      "kind": "file",
      "title": "main.dart",
      "trust_level": "needs_verify",
      "verified_by_gate_id": null
    }
  ]
}
```

### Agents and Skills

```http
GET /api/projects/:project_id/agents
GET /api/skills
GET /api/projects/:project_id/skills
```

### Settings

```http
GET /api/settings/services
```

Response:

```json
{
  "services": [
    {
      "name": "dashboard",
      "host": "127.0.0.1",
      "port": 43180,
      "status": "running",
      "auth_mode": "local"
    }
  ]
}
```

```http
GET /api/settings/database
```

### Reports

```http
GET /api/projects/:project_id/report?format=json
GET /api/projects/:project_id/report?format=markdown
GET /api/sessions/:session_id/report?format=json
GET /api/sessions/:session_id/report?format=markdown
```

Response (`format=json`):

```json
{
  "report": {
    "scope": "project",
    "id": "proj_01",
    "title": "anycode",
    "generated_at": "2026-05-22T00:00:00Z",
    "trusted_status": "blocked",
    "markdown": "# anycode Digital Workbench Report...",
    "summary": {
      "sessions": 12,
      "events": 320,
      "failed_gates": 2,
      "artifacts": 18
    },
    "source_counts": {
      "sessions": 12,
      "events": 320,
      "gates": 4,
      "artifacts": 18
    }
  }
}
```

`format=markdown` returns the same envelope with `markdown` populated (JSON body, not `text/markdown`).

Generating a report writes a low-risk audit event (`project_report_generated` / `session_report_generated`).

### Audit

```http
GET /api/audit/events?project_id=&action=&risk=&limit=100
```

Response:

```json
{
  "events": [
    {
      "id": "audit_...",
      "project_id": "proj_...",
      "session_id": null,
      "actor": "local",
      "action": "project_report_generated",
      "risk": "low",
      "detail": {},
      "created_at": "2026-05-22T00:00:00Z"
    }
  ]
}
```

Recorded actions (V1): `dashboard_started`, `project_reindex_requested`, `skills_rescan_requested`, `project_report_generated`, `session_report_generated`.

### Policies

```http
GET /api/settings/policies
```

Response:

```json
{
  "policy": {
    "mode": "local_trusted",
    "host_binding": "127.0.0.1:43180",
    "remote_access_allowed": false,
    "write_actions_allowed": false,
    "safe_actions": ["reindex", "report_export", "skills_rescan", "tool_approval"],
    "blocked_actions": ["edit_files", "delete_files", "git_push", "deploy", "stop_task"]
  }
}
```

### Data health

```http
GET /api/settings/data-health
GET /api/projects/:project_id/data-health
```

Response:

```json
{
  "health": {
    "status": "warn",
    "db_path": "/Users/.../.anycode/projects.db",
    "db_size_bytes": 1048576,
    "generated_at": "2026-05-22T00:00:00Z",
    "checks": [
      {
        "id": "missing_project_root",
        "name": "Project root exists",
        "status": "warn",
        "message": "Project root no longer exists",
        "count": 1,
        "project_id": "proj_..."
      }
    ]
  }
}
```

Read-only diagnostics; no auto-repair.

## Event Envelope

All structured runtime events should follow this shape:

```json
{
  "id": "evt_...",
  "project_id": "proj_...",
  "session_id": "sess_...",
  "task_id": "uuid",
  "agent_id": "agent_...",
  "event_type": "tool_call_end",
  "severity": "info",
  "title": "Bash finished",
  "body": "flutter test failed",
  "payload": {
    "turn": 4,
    "tool_name": "Bash",
    "elapsed_ms": 2312,
    "error": "ProviderNotFoundException"
  },
  "occurred_at": "2026-05-21T10:00:00Z"
}
```

## V1 Safe Control Endpoints

Optional and safe:

```http
POST /api/settings/services/dashboard/open-browser
POST /api/projects/:project_id/reindex
POST /api/skills
GET /api/projects/:project_id/report
GET /api/sessions/:session_id/report
```

Report export and reindex/skills rescan are audited in `auth_events` (`source=dashboard`).

Not in V1:

- Stop task
- Approve tool
- Edit file
- Push/deploy
- Delete asset

## V2 endpoints (2026-05)

### Usage export

```http
GET /api/metrics/usage/export?days=7
GET /api/metrics/usage/export?days=7&project_id=proj_01
```

Returns `text/csv` attachment.

### Per-project usage

```http
GET /api/projects/:project_id/usage?days=7
```

### Gate runner

```http
GET /api/projects/:project_id/gates/presets
POST /api/projects/:project_id/gates/execute
POST /api/projects/:project_id/gates/execute/stream
```

Execute body:

```json
{ "preset_id": "cargo_test", "required": false }
```

or `{ "command": "cargo test", "name": "custom" }`.

Stream endpoint returns `text/event-stream` with JSON payloads: `{ "type": "line", "line": "..." }`, `{ "type": "done", "result": GateExecuteResult }`.

Results persist to `gates` (via `manual_gate` session) and `project_events` (`gate_executed`).

### V3 metrics (2026-05)

```http
GET /api/metrics/usage?days=7          # includes by_model[]
GET /api/metrics/kpi/saved-hours?days=7
GET /api/sessions/:session_id/usage
POST /api/sessions/:session_id/cancel
GET /api/settings/connectors/:id/linear/issues
```

### GitHub connector preview

```http
GET /api/settings/connectors/:connector_id/github/issues
```

Requires connector `source_type=github` and `config.repo`. Token from config or `GITHUB_TOKEN` / `ANYCODE_GITHUB_TOKEN`.

### Blocked threshold alert

On dashboard startup, when `sessions_blocked > ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD` (default 0), emits at most one `blocked_threshold_exceeded` audit/notification per hour.
