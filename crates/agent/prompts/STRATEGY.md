# Agent prompt language strategy

## Layers

| Layer | What | Language |
|-------|------|----------|
| **core** | System, Tone, Agent loop, Browser, Plan, … | English only (one protocol source of truth) |
| **locale** | Reply-language section + ephemeral reminder | One directory per language tag (`zh`, `en`, …) |
| **dynamic** | cwd / OS / date, tool list, skills, agent description | Placeholders in files; filled by Rust |

Do **not** mix Chinese and English inside `core/`. Do **not** duplicate the same user-visible answer in two languages.

## Locale file checklist (`locale/<tag>/`)

Each language directory must include:

1. **`reply_language.md`** — full `# Reply language` section (default language, exemptions, tool-round rules, forbidden scaffolding, no bilingual duplicate).
2. **`ephemeral_reminder.md`** — one short line injected every LLM call (same rules, compressed).

Keep **zh** and **en** structurally parallel (same bullet topics) so behavior stays symmetric when editing.

## Runtime wiring

UI / API `lang` → `ChatTurnContext.reply_language` (or `ANYCODE_REPLY_LANG`) →
`prompt_catalog::resolve_locale_tag` → load `locale/<tag>/…`.

Unknown tags: no reply-language section / no ephemeral reminder (current behavior).

Fluent (`crates/locale`) owns **UI / channel copy**, not this agent system stack.

## Adding a language

1. Copy `locale/en/` → `locale/<tag>/` and translate those files (keep the same bullet topics).
2. Register `<tag>` in `crates/agent/src/prompt_catalog.rs` (`resolve_locale_tag` + locale map).
3. Do **not** rewrite `core/*`.

## Follow-ups (out of this directory for now)

WeChat bridge copy, overview briefing templates, and compact-body prompts should migrate under the same pattern later (per-crate `prompts/` or extensions here).
