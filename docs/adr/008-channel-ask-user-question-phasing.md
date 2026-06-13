# ADR 008: Channel AskUserQuestion — phasing

Status: **Proposed** (2026-04). **Update 2026-04-20:** Slices **(1)** task-local + broker scaffolding and **(2)** **Telegram** inline keyboard MVP are **implemented** (`crates/cli/src/tg_ask.rs`, `tg.rs` poll + `callback_query`).

## Context

- In-process hosts already implement **`AskUserQuestion`** (TUI, stream REPL, etc.).
- IM channels (WeChat, Telegram, Discord) need structured choices without elevating a **public** trait before a second real host exists.
- Spike: [`docs/ops/channel-ask-user-question-spike.md`](../ops/channel-ask-user-question-spike.md).

## Implementation slices (ordered)

1. **`pub(crate)` host enum** (or extension of existing internal host type) that can represent `Telegram` / `Discord` / `WeChatText` with a single `ask_user_question` entry using **async** message send + **timeout** + **correlation id** stored in session state.
2. **One channel first** (recommended: **Telegram** inline keyboard + callback query, or **Discord** button interaction) with **text fallback** (`reply 1` / `reply a`) documented in system append.
3. **Mutual exclusion** with tool approval pending: at most one of `WaitingPermission` vs `WaitingQuestion` per chat (see spike §「与 PermissionBroker 的关系」).
4. **WeChat**: stay on **y/n**-style approval for tools; optional **numeric / letter** option replies for AskUserQuestion only after slice (2) proves the pattern.
5. **Documentation + Fluent** for channel-specific prompts; no user-facing JSON schema change.

## Out of scope (this ADR)

- Public stable trait for third-party channel plugins.
- Full parity with desktop TUI widgets (sliders, free text fields).

## Consequences

- Code may add `pub(crate)` types and channel-specific modules under `crates/cli/src/` without ADR amendment.
- Promoting any type to **`pub`** across the workspace boundary requires a new ADR or **Accepted** bump on this document.
