# Eval Harness (removed)

The **`anycode eval`** CLI subcommand and **`scripts/eval/`** harness were **removed** with the terminal CLI retirement.

## Use instead

- **`cargo test --workspace`** — primary CI validation (mock LLM integration tests in crate test suites).
- **Workbench** — manual smoke: `/setup`, short chat, automations, channel bridges via `anycode-daemon`.
- **DeepSeek / live API checks** — configure provider in Workbench Settings, send a one-line test message (no dedicated eval script).

Historical design notes below are kept for context only.

---

## Former design (historical)

Production changes were validated by repeatable scenarios:

| Class | Purpose |
|-------|---------|
| CLI smoke | Help/status/doctor commands without provider credentials. |
| Automation | Cron ledger reads when empty. |
| Mock fixture | Scripted mock LLM + fixture repos. |

CI previously ran `python3 scripts/eval/run.py --with-mock` after building the release binary. That job is **gone**; use `cargo test --workspace`.
