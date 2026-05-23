# anycode-dashboard

Local Digital Workbench backend for anycode.

## Responsibilities

- SQLite schema, migrations, and typed store facade.
- Axum API and embedded static UI serving.
- Session/event ingest, replay, metrics, audit, and reports.
- Local control-plane glue for approvals, cancellation, gate runs, and triggered runs.
- Workbench governance views for skills, tools, services, security activity, and automation policies.

## Directory Map

- `src/api/`: router, auth middleware, app state, and domain handlers.
- `src/db/`: database facade, migrations, trusted-status helpers, and domain store modules.
- `src/observability/`: metrics, log parsing, ingest, and session replay.
- `src/governance/`: skills, service checks, automation policy, and security activity.
- `src/ipc/`: local approval and cancellation file contracts.
- `src/control/`: gate runner and project run trigger control paths.

## Validation

```bash
cargo test -p anycode-dashboard
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
```
