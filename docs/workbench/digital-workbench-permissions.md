# Digital Workbench Permissions and Automation Policy

**0.3 scope:** roles and enterprise mode definitions support the **web account console** (org/member/RBAC shell). **Agent execution policy** (tool approval, run trigger) remains CLI-first; web control-plane actions are loopback-dev only unless explicitly reopened in a later ADR.

## Modes

### Local Trusted Mode

Allowed only when:

- Host is `127.0.0.1` or `localhost`.
- User explicitly starts `anycode dashboard`.
- No remote access token is issued.

Purpose:

- Fast local development.
- Read local project state.
- View events, gates, artifacts.

### Local Authenticated Mode

Required when:

- User enables login.
- API token is created.
- Dashboard is accessed beyond one browser session.

### Enterprise Mode

Required when:

- Host is not loopback.
- Multiple users exist.
- External connectors are enabled.
- Audit and RBAC are required.

## Roles

- `Owner`
  - Full project and organization control.
- `Admin`
  - Manage users, services, settings, connectors.
- `Maintainer`
  - Run automations, approve medium-risk actions.
- `Developer`
  - Start sessions, inspect assets, request approvals.
- `Viewer`
  - Read-only.
- `Auditor`
  - Read-only plus audit export.

## Action Risk Matrix

| Action | Risk | Default V1 | Approval |
| --- | --- | --- | --- |
| View projects | Low | Allowed | No |
| View sessions/events | Low | Allowed | No |
| View artifacts in project root | Low | Allowed | No |
| Index project files | Low | Allowed | No |
| Read files outside project | Medium | Blocked | Yes |
| Run tests/build commands | Medium | Allowed from agent runtime; **V2:** manual gate runner from project detail (non-required gates, audited) | Policy-based |
| Start new goal/workflow | Medium | Later | Optional |
| Edit files | High | Not from dashboard V1 | Yes |
| Delete files | High | Blocked | Always |
| Git commit | High | Not from dashboard V1 | Yes |
| Git push | Critical | Blocked V1 | Always |
| Deploy/release | Critical | Blocked V1 | Always |
| External API mutation | Critical | Blocked V1 | Always |
| External API read (GitHub issues) | Low | **V2 POC** read-only | No |

## V1 Dashboard Controls

V1 dashboard is mostly read-only. **V2 adds** manual gate execution and GitHub issues preview (read-only).

Allowed:

- Open browser.
- Refresh/reindex project.
- Export report.
- Copy command/path.
- Toggle local UI settings.
- **V2:** Export token usage CSV.
- **V2:** Run verification gate presets in project workspace (audited; non-required gates).
- **V2:** Preview GitHub open issues for configured connector.

Not allowed:

- Approve tools.
- Modify files.
- Stop/kill tasks.
- Push/deploy.
- Delete assets.

Rationale:

The dashboard should first become the trusted observation layer. Control actions can be added after audit, auth, and approval UX are stable.

## Automation Policies

### Retry Policy

Fields:

- `max_attempts`
- `unlimited`
- `backoff_ms`
- `retry_on`
- `stop_on`

Examples:

- Retry on test failure.
- Retry on transient LLM failure.
- Stop on missing credentials.
- Stop on destructive approval denial.

### Model Fallback Policy

Fields:

- `primary_model`
- `fallback_models`
- `quota_error_patterns`
- `cooldown_seconds`

Behavior:

- Detect quota/rate-limit errors.
- Switch to next configured model.
- Record fallback event.
- Show fallback in dashboard.

### Gate Policy

Fields:

- `required_gates`
- `trusted_completion_requires_all`
- `allow_readme_marker_only`
- `browser_verification_required`

Default:

- README marker alone is not trusted.
- Required gates must pass.
- Failed gates lower project trust score.

### Asset Policy

Fields:

- `allowed_roots`
- `external_sources`
- `max_file_bytes`
- `index_binary_files`
- `redaction_rules`

Default:

- Project root readable.
- Outside root requires approval.
- Large files summarized, not directly injected.

## Audit Events

### Dashboard V1 (local, low-risk)

Recorded in `auth_events` with `source=dashboard`:

- `dashboard_started`
- `project_reindex_requested`
- `skills_rescan_requested`
- `project_report_generated`
- `session_report_generated`

Actor is always `local`; no login/RBAC in V1.

### Enterprise (future)

Must also record:

- Login success/failure.
- Password reset.
- API token creation/deletion.
- Service start/stop.
- Port change.
- Asset source authorization.
- External connector authorization.
- Gate override.
- Any high/critical action approval.

## Enterprise Requirements Before Remote Access

Do not support remote dashboard binding until these exist:

- Authenticated sessions.
- API token support.
- CSRF protection for cookie flows.
- Audit logging.
- RBAC.
- Explicit host/port config.
- Warning when binding to `0.0.0.0`.
