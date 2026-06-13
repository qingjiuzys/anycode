# Workbench IPC Contracts

Digital Workbench talks to live CLI sessions through small local contracts. Keep these stable unless the CLI and dashboard are updated in the same change.

## Session Registration

- The CLI recorder registers live sessions below the dashboard active-session directory.
- `ANYCODE_DASHBOARD_SESSION_ID` identifies the current dashboard session when the CLI is launched from workbench flows.
- Dashboard APIs should treat missing live registration as a normal offline session, not as an error.

## Approval Flow

- Web approvals are mediated by the dashboard approval IPC files.
- CLI approval callbacks remain the authority for allowing or denying a tool action.
- Dashboard state is a view of pending decisions; it must not bypass `SecurityLayer` approval semantics.

## Cancel Flow

- `POST /api/sessions/{session_id}/cancel` marks the dashboard session cancelled and writes a cooperative cancel request when a live CLI registration exists.
- CLI tail loops poll the cancel request and set the same cooperative cancel flag used by TTY and nested agent cancellation.
- Cancellation is best effort for blocking tools and in-flight provider HTTP bodies.

## Event Delivery

- CLI/runtime logging writes session output and structured trace sidecars.
- Dashboard ingest stores project/session events in SQLite.
- `notify`/SSE delivery is only a live update path; SQLite remains the durable source of truth.

## Naming

- Use `dashboard` in Rust module names and API internals.
- Use `Digital Workbench` in user-facing docs and UI copy.
- Use `notifications` for stored policies and `notify`/`event push` for transient live delivery.
