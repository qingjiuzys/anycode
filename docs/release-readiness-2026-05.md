# Release Readiness 2026-05

This checklist tracks the production-grade line after the OpenClaw 5.19 parity
burst.

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
- Cron runs are queryable via `anycode cron runs`.
- Background nested agents write diagnostic state files.
- Memory evidence is indexed for file/search/fetch/MCP style tool results.
- `anycode doctor all` reports memory, channel, scheduler, and MCP hints.
- Structured CLI errors: `anycode doctor errors`; set `ANYCODE_ERRORS_JSON=1` on failure.

## Known Non-Goals

- No HTTP daemon / Gateway revival.
- No dynamic plugin marketplace.
- No memory-wiki / dreaming full stack.
- No silent MCP auto-reconnect.

