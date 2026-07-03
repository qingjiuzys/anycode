# ADR 003: App-native composition root

## Status

Accepted — **CLI retired** (2026-07)

## Context

anyCode originally shipped as a terminal-first `anycode` CLI. Desktop spawned `anyCode Workbench` and `anycode run` subprocesses. Dashboard duplicated runtime bootstrap in `chat_runtime/bootstrap.rs`.

## Decision

1. **`anycode-config`** — shared `config.json` schema and load/save (no clap/TUI).
2. **`anycode-bootstrap`** — shared `initialize_runtime` for dashboard, daemon, and channel bridges.
3. **`anycode-dashboard-ipc`** — file-based approval/question/cancel IPC (no dashboard ↔ bootstrap cycle).
4. **Desktop (`anycode-desktop`)** — embeds `anycode-dashboard::run_with_shutdown` in-process; embedded chat and UI triggers always use in-process `AgentRuntime`.
5. **`anycode-daemon`** — headless binary (`anycode-channel-bridge` crate): WeChat/Telegram/Discord bridges + built-in cron scheduler, all in-process.
6. **CLI removed** — no `anycode` binary in workspace or release artifacts.

## Consequences

- User-facing entry points: **Desktop** (GUI) and **daemon** (headless servers).
- No subprocess spawns of legacy CLI from dashboard or desktop.
- Channel + scheduler implementation lives in `anycode-channel-bridge`.
- Linux without GUI: run `anycode-daemon` + browser to dashboard HTTP.
