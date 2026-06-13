# Digital Workbench API Contract

Base URL for local V1:

```text
http://127.0.0.1:<dashboard_port>
```

All write/control endpoints are out of scope for the first implementation slice unless explicitly marked safe.

## Authentication

**0.3 product direction:** the Workbench web UI is an **account console** (login, plan, usage, billing, API, enterprise settings). It is **not** the primary surface for operating Agent runs in 0.3 — execution stays in CLI / local runtime.

V1 local mode:

- Trusted local mode allowed only when bound to `127.0.0.1`.
- API token is required if binding to any non-loopback address.
- `/api/auth/login`, `/api/auth/logout`, `/api/auth/me` support session UI (see `LoginPage`).

0.3 targets (planned):

- Cookie session for UI on non-loopback hosts.
- Bearer token for API; API key CRUD in admin UI.
- Subscription/entitlement records (mock OK before payment integration).

Future enterprise mode:

- OIDC / SSO optional (design in 0.3-E; full IdP later).
- RBAC per [`digital-workbench-permissions.md`](digital-workbench-permissions.md).

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

`trust_score` is `0.0`–`1.0` when the project has scorable activity (sessions, gates, or artifacts); it equals `readiness_score / 100` from `/api/projects/:id/metrics`. The field is omitted or `null` when the project is registered but has no sessions, gates, or artifacts yet.

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

Response: `{ "trigger": TriggerRunResult }` with `pid`, `command_preview`, `log_path`, and `sandbox_note`. Spawns detached `anycode run -C {root}` with dashboard recording enabled; sensitive tools use the Web approval inbox when approval is required. Loopback-only unless `ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE=1`.

### Start conversation (WebChat REPL)

```http
POST /api/projects/:project_id/conversations/start
```

Creates a **pending** session in SQLite, then spawns a long-lived non-TTY `anycode` line REPL (`ANYCODE_DASHBOARD_SESSION_STICKY=1`) bound to the session. Messages are sent on stdin; events are recorded to SQLite via the dashboard recorder.

Request body:

```json
{
  "title": "Optional session name — defaults to first 120 chars of prompt",
  "prompt": "Fix the failing unit test in src/foo.rs",
  "kind": "run",
  "goal": "optional when kind=goal (validated but WebChat uses repl mode)",
  "agent": "general-purpose",
  "skills": ["optional-skill-id"]
}
```

Response: `{ "session": SessionDetail, "chat": WebChatSendResult }` where `chat` includes `pid`, `log_path`, `started_at`, and `queued`. On spawn failure the session is marked `failed` and the handler returns an error with `session_id`.

Use this from the Conversations page. The legacy `POST .../runs/trigger` path remains for detached one-shot runs; it does not use the WebChat hub.

### Follow-up message (existing session)

```http
POST /api/sessions/:session_id/message
```

Send a follow-up prompt to an existing WebChat session (or spawn the REPL on first message for sessions created via `POST /api/sessions`).

Request body:

```json
{
  "prompt": "Now add a regression test",
  "agent": "optional override for next message only",
  "skills": ["optional-skill-id"]
}
```

Response: `{ "ok": true, "session_id": "...", "chat": WebChatSendResult }`. Changing `agent` updates the session row and **evicts** the cached REPL (terminates the old process) so the next send respawns with the new `--agent`.

Same loopback / `ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE` policy as conversation start.

### Events

```http
GET /api/sessions/:session_id/events?after=<event_id>&limit=200
```

```http
GET /api/sessions/:session_id/trace
GET /api/sessions/:session_id/execution-log?offset=0&limit=200
```

**Data layering (session SSOT):**

| Layer | Store | Contents |
|-------|--------|----------|
| Index | SQLite `projects.db` | projects, sessions (`title`, `prompt_preview`, unique `task_id`), index events (`user_prompt`, `assistant_response`, `task_*`, `gate`, `budget_*`, approvals) |
| Stack detail | `~/.anycode/tasks/{task_id}/output.log` | turn / LLM / tool_call trace; read on demand via `execution-log` |

Trace response: `{ "trace": { "schema_version": 1, "session_id": "...", "source": "output.log"|"database", "events": [...] } }`. Prefers `output.log` when `task_id` is set; falls back to DB for legacy rows.

Execution-log response: `{ "execution_log": { "offset", "next_offset", "has_more", "lines": [{ "line_no", "raw", "event_type", ... }] } }`.

`POST /api/projects/scan` syncs workspace paths and skills only (no bulk log import).

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
GET /api/projects/:project_id/report?format=json&lang=zh|en
GET /api/projects/:project_id/report?format=markdown&lang=zh|en
GET /api/sessions/:session_id/report?format=json&lang=zh|en
GET /api/sessions/:session_id/report?format=markdown&lang=zh|en
```

Query:

- `lang` — `zh` or `en` (default `en`). Controls localized titles, verdict text, and Markdown export strings.
- `format` — `json` (default) or `markdown` (same JSON envelope; not raw `text/markdown`).

Response (`format=json`):

```json
{
  "report": {
    "scope": "project",
    "id": "proj_01",
    "title": "anycode 数字工作台报告",
    "lang": "zh",
    "generated_at": "2026-05-22T00:00:00Z",
    "trusted_status": "unverified",
    "markdown": "# anycode 数字工作台报告...",
    "highlights": {
      "trust_verified": 2,
      "trust_unverified": 5,
      "trust_blocked": 0,
      "failures_unique": 1,
      "verdict": "部分会话未验证，交付前请复核。"
    },
    "sessions_recent": [],
    "sessions_imported_count": 12,
    "failure_groups": [{ "title": "Bash 失败", "event_type": "tool_error", "count": 6, "last_at": "...", "session_id": "sess_..." }],
    "gates": [],
    "artifacts": [],
    "events_sample_limit": 50,
    "summary": { "sessions": 12, "events": 320, "failed_gates": 0, "artifacts": 18 },
    "source_counts": { "sessions": 12, "events": 320, "gates": 4, "artifacts": 18 }
  }
}
```

Project reports omit empty gate/artifact sections from `markdown`. Recent sessions exclude `Imported task …` rows (count in `sessions_imported_count`). Failures are grouped by `(title, event_type)`.

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

### Doctor

```http
GET /api/settings/doctor
```

Response:

```json
{
  "doctor": {
    "status": "ok",
    "generated_at": "2026-05-27T12:00:00Z",
    "checks": [
      { "id": "llm_config_exists", "status": "ok", "message": "config.json found at ..." },
      { "id": "llm_api_key", "status": "ok", "message": "Primary api_key is configured" }
    ],
    "next_steps": []
  }
}
```

Includes infrastructure checks (DB, UI dist, loopback, MCP env) plus LLM checks (`llm_config_exists`, `llm_api_key`, `llm_google_fallback` when applicable) and optional connector reachability probes.

### Model catalog & LLM config

```http
GET /api/settings/model-catalog
POST /api/settings/model-catalog/refresh
GET /api/settings/models
PUT /api/settings/models
POST /api/settings/models/{model_id}/enable
POST /api/settings/models/{model_id}/test
```

**Catalog** response: `{ "providers", "zai_models", "google_models", "capabilities", "cache_meta", ... }` — static presets plus optional cached remote lists (no secrets).

**Refresh** body: `{ "provider": "openai", "base_url": "https://..." }` (optional). Writes cache under `~/.anycode/catalog-cache/`.

**Models registry** (`GET /api/settings/models`): `{ "active": { "chat": "model-id", ... }, "items": [ ... ], "model_fallback": ... }`.

**Registry patch** (`PUT /api/settings/models`): `{ "items": [...], "active": { "chat": "id" }, "delete_ids": ["old-id"] }` — merge-safe.

**Enable** body: `{ "capabilities": ["chat", "embedding"] }`.

**Test model** body: `{ "capability": "chat", "draft": { ...ConfiguredModel } }` — probes draft or saved profile without requiring a prior save.

```http
GET /api/settings/llm
```

Response (masked secrets):

```json
{
  "config_present": true,
  "provider": "google",
  "model": "gemini-2.0-flash",
  "api_key": { "configured": true, "preview": "sk-…" },
  "model_fallback": { "provider": "anthropic", "model": "claude-sonnet-4-20250514", "on": "geo" },
  "models": {},
  "routing_agents": {},
  "registry": { "active": {}, "items": [] }
}
```

```http
PUT /api/settings/llm
```

Body (all fields optional): `provider`, `model`, `plan`, `base_url`, `api_key`, `provider_credentials`, `fallback_provider`, `fallback_model`, `fallback_on`, `routing_agents`, `routing_agents_delete`, `models`. Patches `~/.anycode/config.json` with deep merge for `models.*` and returns `{ "ok": true, "config_path": "...", "model_fallback": ... }`.

```http
POST /api/settings/llm
```

Body:

```json
{ "capability": "chat" }
```

Runs a short LLM probe for the given capability (`chat`, `vision`, `embedding`, `stt`, `tts`, `image`, `video`). Response: `{ "ok": true, "message": "..." }` or `{ "ok": false, "error": "..." }` with HTTP 400 on failure.

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
