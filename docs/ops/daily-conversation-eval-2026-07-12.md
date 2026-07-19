# Daily Conversation Eval — 2026-07-12

Cloud-only (Agnes + local account/gateway stack). Workbench: `http://127.0.0.1:43181` via `scripts/start-local-workbench.sh`.

## Executive summary

| Area | Result |
|------|--------|
| API scenario matrix (6/6) | **PASS** |
| Playwright core (Chromium) | **64 pass / 2 fail / 1 skip** |
| Live cloud smoke (`LIVE_E2E=1`) | **PASS** (pong in ~23s) |
| Gateway + session | **PASS** after `dev-auto-link.sh` |
| UI parity (index.html) | **MATCH** dist ↔ `/Applications/anyCode.app` |

**Overall: 6/7 daily matrix items PASS at API layer.** Artifacts API index returned 0 for all sessions (P2 UX — files exist on disk).

## Environment

```bash
./scripts/start-local-account.sh          # :43200
./scripts/start-local-gateway.sh start    # :43210
./scripts/dev-auto-link.sh                # refresh cloud session
./scripts/start-local-workbench.sh start  # :43181
export ANYCODE_IGNORE_APPROVAL=1          # default in workbench script
```

Agnes key: `~/.anycode/secrets/agnes.txt` (not in git).

## Scenario matrix

| Scenario | Status | Duration | Tools / Skill | On-disk artifact |
|----------|--------|----------|---------------|------------------|
| **Coding** | PASS | 75s | Glob, Grep | — |
| **PPT** | PASS | 72s | Bash, office-pptx Skill | `daily-brief.pptx` (35KB) |
| **PDF** | PASS | 69s | FileWrite, md-to-pdf Skill | `brief.pdf` (38KB) |
| **Image** | PASS | 48s | GenerateImage | Agnes CDN URL |
| **Video** | PASS | 147s | video-script Skill, GenerateImage | script + storyboard |
| **Skill trace** | PASS | 30s | doc-summary Skill | — |
| **Artifacts UI** | PARTIAL | — | — | Files on disk; `/api/sessions/.../artifacts` empty without scan |

Raw JSON: `test/out/daily-conversation-eval.json`

## Fixes applied (P0/P1)

| Issue | Fix |
|-------|-----|
| Embedded runtime ignored `ANYCODE_IGNORE_APPROVAL` | `env_ignore_approval()` in `bootstrap.rs` |
| Grep stuck on approval | Same + workbench already exports bypass |
| e2e-delivery overlay forced Ollama | `apply_project_overlays` + patch active chat → `cloud-auto` |
| Stale / revoked cloud tokens | `scripts/dev-auto-link.sh`, `scripts/refresh-cloud-session.sh` |
| `verify-cloud-e2e.sh` wrong session path | `credentials/cloud-session.json` + refresh on 401 |
| MCP registry missing `mcp` tool | Always register when `tools-mcp` enabled (prior) |
| Playwright reuse crashed seed | `DASHBOARD_E2E_REUSE=1` disables `webServer` in config |
| Broken `agents-tabs.spec.ts` | Restored market API test wrapper |
| No real conversation e2e | `e2e/conversation-live-smoke.spec.ts` (`LIVE_E2E=1`) |

## Playwright

```bash
cd crates/dashboard-ui
DASHBOARD_E2E_PORT=43181 DASHBOARD_E2E_REUSE=1 npx playwright test --project=chromium
LIVE_E2E=1 DASHBOARD_E2E_PORT=43181 DASHBOARD_E2E_REUSE=1 npx playwright test e2e/conversation-live-smoke.spec.ts
```

**Failures (P2, non-blocking):**

1. `agents-tabs` zh locale skill name — fixture skill row not present in live DB
2. `ui-interactions` `/setup` redirect — now routes via `conversations?cc=...` shell

## Parity

- `dist/index.html` SHA matches app bundle after `ANYCODE_DASHBOARD_UI_FORCE=1 ./scripts/sync-desktop-dev.sh`
- API `model_gateway_url` on :43181 → `http://127.0.0.1:43210`
- Desktop binary synced: `./scripts/sync-desktop-dev.sh --rust`

## Repro commands

```bash
# Full API matrix
python3 scripts/run-daily-conversation-eval.py

# Single scenario
SCENARIO_ONLY=pdf python3 scripts/run-daily-conversation-eval.py

# Quick pong smoke
python3 scripts/smoke-cloud-conversation.py

# Cloud stack verify
./scripts/verify-cloud-e2e.sh
```

## Remaining recommendations (P2+)

1. ~~**Artifacts index**~~ — **Done (2026-07-12 UX landing):** Skill/FileWrite paths indexed; session end auto `scan-artifacts`; Artifacts panel defaults to all scanned files
2. **Conversation UX** — anyCode label, `.agent-activity-line`, `.agent-status-line`, thinking preview; eval asserts `artifact_count >= 1` for pdf/ppt after scan
3. **Session token lifecycle** — rebuild embedded runtime when cloud session refreshes (avoid mid-turn 401 after token rotation)
4. **Playwright fixture mode** — keep `dashboard-e2e-server.sh` for CI; document `DASHBOARD_E2E_REUSE=1` for live stack
5. **e2e-delivery project** — default `models.active.chat` to `cloud-auto` in harness template, not Ollama mock
6. **Toast to Artifacts** — optional UX hint when tool produces files outside chat bubble
