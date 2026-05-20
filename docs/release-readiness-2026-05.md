# Release Readiness 2026-05

This checklist tracks the production-grade line after the OpenClaw 5.19 parity
burst and the production readiness roadmap landing (`b86e583`).

## CI Verification (2026-05-20)

| Job | Run | Result |
|-----|-----|--------|
| CI (rust + docs) — production readiness | [GitHub Actions run 26143655616](https://github.com/qingjiuzys/anycode/actions/runs/26143655616) | **success** on `main` after `b86e583` |
| CI (rust + docs) — audit / cron allowlist (`c27f2e1`) | [GitHub Actions run 26144013132](https://github.com/qingjiuzys/anycode/actions/runs/26144013132) | **success** on `main` after `c27f2e1` |
| Eval harness step | `python3 scripts/eval/run.py --with-mock` in CI rust job | **pass** (8/8 scenarios) on both runs |

Local pre-push verification (same commit):

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets` | pass |
| `cargo test --workspace` | pass |
| `cargo build --release -p anycode` | pass |
| `python3 scripts/eval/run.py --with-mock` | pass |

## Required Checks

| Check | Command |
|-------|---------|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets` |
| Tests | `cargo test --workspace` |
| Release binary | `cargo build --release -p anycode` |
| Docs site | `cd docs-site && npm ci && npm run build` when `docs-site/` changed |
| Eval smoke | `python3 scripts/eval/run.py --with-mock` |

## Production Gates

- Tool calls are auditable via `~/.anycode/audit/tool-calls.jsonl`.
- Tool audit is queryable via `anycode audit tail`.
- Cron runs are queryable via `anycode cron runs`.
- Cron jobs support `tool_profile` including custom `allowlist`.
- Background nested agents write diagnostic state files.
- Memory evidence is indexed for file/search/fetch/MCP style tool results.
- `anycode doctor all` reports memory, channel, scheduler, and MCP hints.
- Structured CLI errors: `anycode doctor errors`; set `ANYCODE_ERRORS_JSON=1` on failure.

## Known Risks

| Risk | Mitigation |
|------|------------|
| Eval mock scenario depends on debug binary path in CI | CI builds release then runs eval against `target/debug/anycode` built during `cargo test`; keep eval step after workspace tests |
| Cron `allowlist` with `mcp__*` entries disables blanket MCP deny | Documented; use only for trusted monitoring jobs |
| Virtual scroll not implemented at runtime | Tier S/M synthetic tests only; large transcripts may still be slow |
| MCP silent reconnect intentionally disabled | ADR 007; use `anycode mcp status` + manual reconnect |
| GitHub Actions Node 20 deprecation warning on docs job | Bumped to `actions/checkout@v5` + `actions/setup-node@v5` (Node 24 runtime) |

## Rollback

1. **Revert release commit** on `main`:
   ```bash
   git revert b86e583
   git push origin main
   ```
2. **Disable CI eval step** if harness flakes: remove the `Eval harness` step from `.github/workflows/ci.yml`.
3. **Cron failure hooks**: unset `ANYCODE_CRON_FAILURE_SHELL` / `ANYCODE_CRON_FAILURE_WEBHOOK` on hosts running the scheduler.
4. **Tool audit / evidence logs**: safe to delete or rotate `~/.anycode/audit/tool-calls.jsonl` and `~/.anycode/memory/evidence.jsonl`; no runtime dependency.

## Known Non-Goals

- No HTTP daemon / Gateway revival.
- No dynamic plugin marketplace.
- No memory-wiki / dreaming full stack.
- No silent MCP auto-reconnect.

## Next Milestone (2026-06)

1. Stream UI aggregation by cron/session id in transcript dock.
2. Virtual scroll runtime (ADR 006) wired into stream REPL.
3. Expand eval harness with repository fixture tasks (SWE-bench-lite style).
