# anycode Digital Workbench UI

React + TypeScript + Vite + TanStack Router + TanStack Query + React Flow + ECharts.

## Development

```bash
# Terminal 1 — API (default http://127.0.0.1:43180)
anycode dashboard

# Terminal 2 — UI with /api proxy
cd crates/dashboard-ui
npm install
npm run dev
```

Open http://localhost:5173

## Runtime recording

`anycode run` / `goal` append to `~/.anycode/projects.db` when `ANYCODE_DASHBOARD_RECORD` is enabled (default on).

- Structured lines in `~/.anycode/tasks/<id>/output.log`
- Sidecar `events.jsonl` for tagged lines (`[task_start]`, `[tool_call_*]`, `[gate]`, …)
- Goal acceptance gates are logged live during `GoalEngine` verification

## Production static files

```bash
cd crates/dashboard-ui && npm ci && npm run build
anycode dashboard --open
```

If `crates/dashboard-ui/dist/index.html` exists, `anycode dashboard` serves it automatically.
Override with `--static-dir` or `ANYCODE_DASHBOARD_STATIC`.

From repo root:

```bash
./scripts/build-dashboard-ui.sh
```

## Status & V3 planning

**V1 MVP + V2 slices A–D are complete** (2026-05).

| Doc | Purpose |
|-----|---------|
| [`docs/digital-workbench-STATUS.md`](../../docs/digital-workbench-STATUS.md) | One-page ship checklist |
| [`docs/digital-workbench-next-steps-zh.md`](../../docs/digital-workbench-next-steps-zh.md) | **Start V3 planning here** (中文) |
| [`docs/digital-workbench-next-steps.md`](../../docs/digital-workbench-next-steps.md) | V3 tiers + sample roadmap (EN) |

User docs: [docs-site/guide/dashboard-planning.md](../../docs-site/guide/dashboard-planning.md)

Tests: `npm test` · `npm run test:e2e` (11 specs) · `cargo test -p anycode-dashboard`
