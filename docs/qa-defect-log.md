# anyCode QA Defect Log

Audit started: 2026-06-12

| ID | Severity | Area | Description | Root cause | Status |
|----|----------|------|-------------|------------|--------|
| D001 | P0 | WeChat setup | Dashboard setup saves account to wrong path/fields | `crates/setup/src/wechat_ilink.rs` | closed |
| D002 | P0 | Quick auth | Dashboard presets drift from CLI choices | `crates/setup/src/quick_auth.rs` | closed |
| D003 | P1 | Agents UI | `summaryPaths` KPI hardcoded to `"2"` | `AgentsPage.tsx` | closed |
| D004 | P1 | Setup memory | `pipeline_http` without url/model silently falls back to hybrid | `handlers/setup.rs` | closed |
| D005 | P1 | Setup memory | Dashboard missing `pipeline_no_embedding` preset | `SetupWizardPage.tsx` | closed |
| D006 | P1 | Reports | URL params via `window.location` not router | `ReportsPage.tsx` | closed |
| D007 | P2 | API tests | New endpoints lack fixture coverage | `fixture_api.rs` | closed |
| D008 | P2 | Playwright | UI button matrix not automated | `e2e/` | closed |
| D009 | P0 | Routing | `/assets` SPA page conflicted with Vite static `/assets/*` mount | `api/mod.rs`, `embedded_ui.rs` | closed |

## Known limitations (not defects)

- Slack connector write sync
- Rerank model probe
- SSO/RBAC, Tauri, OAuth connector write
- Cron job edit/delete UI
