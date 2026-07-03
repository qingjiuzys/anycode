# anycode Digital Workbench UI

React + TypeScript + Vite + TanStack Router + TanStack Query + React Flow + ECharts.

## Development

**Option A — Desktop (recommended):**

```bash
cd apps/anycode-desktop && cargo tauri dev
```

Open http://127.0.0.1:43180

**Option B — UI dev server + API:**

```bash
# Terminal 1 — API
ANYCODE_BUILD_DASHBOARD_UI=1 cargo run --release -p anycode-dashboard --features embedded-ui --bin anycode-dashboard-serve

# Terminal 2 — Vite proxy
cd crates/dashboard-ui && npm install && npm run dev
```

Open http://localhost:5173

## Runtime recording

Workbench sessions append to `~/.anycode/projects.db` when dashboard recording is enabled.

## Production static files

```bash
./scripts/build-dashboard-ui.sh
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode-desktop
```

Embedded UI is bundled in the desktop app; headless API-only: `anycode-dashboard-serve` with `--features embedded-ui`.
