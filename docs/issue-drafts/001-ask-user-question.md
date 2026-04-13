# GitHub issue draft: AskUserQuestion host-mediated selection

**Tracked issue:** https://github.com/qingjiuzys/anycode/issues/3 (created from this draft).

To recreate or update: `gh issue create --title "feat: AskUserQuestion host-mediated selection (REPL + TUI)" --body-file docs/issue-drafts/001-ask-user-question.md` (or paste the section below into the web UI).

---

## Summary

Replace the **first-option fallback** for **`AskUserQuestion`** with real **host-mediated** selection in both **stream REPL** and **fullscreen TUI**, aligned with existing **approval** UX patterns (`SecurityLayer` / prompts).

## Background

- Listed as **§3 Next** in [`docs/roadmap.md`](../roadmap.md) (stream REPL + fullscreen TUI + TTY dialoguer are implemented; channel bridges remain headless — see Non-goals).
- **Shipped behavior:** without an attached host, the tool returns **`status: unsupported_host`** (no silent first-option pick).

## Acceptance criteria

1. When the model invokes **AskUserQuestion** with multiple options, the user can **pick an option** (or cancel / timeout with defined behavior) in:
   - **Inline stream REPL** (`tasks_repl` / stream ratatui path).
   - **Fullscreen TUI** (`tui/run/event` or equivalent).
2. **No silent first-option default** in normal interactive TTY sessions; if stdin is non-interactive, behavior is **documented** (error or deterministic fallback) and tested.
3. **Consistency** with approval dialogs where reasonable (keyboard focus, cancel key, i18n hooks if existing tools use `tr!`).
4. **Tests**: at least one unit or integration test covering parse + selection plumbing (mock host if needed).

## Non-goals (this issue)

- Full **MCP OAuth** GUI parity.
- **Channel-native UX** (WeChat / Telegram / Discord cards or inline keyboards for **AskUserQuestion**): track separately; today those processes do not attach **`AskUserQuestionHost`** and correctly get **`unsupported_host`**.

## References

- [`docs/roadmap.md`](../roadmap.md) §3  
- `anycode-tools`: AskUserQuestion implementation and catalog description  
- `crates/security`: approval flow for UX alignment
