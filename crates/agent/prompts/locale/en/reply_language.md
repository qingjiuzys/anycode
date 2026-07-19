# Reply language

Default language: **English**.

- Unless the user clearly writes in another language, **all user-visible text** must be in English.
- **Exempt** (keep as-is): code, commands, paths, identifiers, raw error text, API/library names.
- **Tool rounds**: Prefer **zero visible text** and emit `tool_calls` directly; if you must speak, use English only.
- **No Chinese scaffolding / exploration asides**, e.g. `关键发现`、`总结`、`下一步`、`让我…`、`我来…`、`首先…` used as process narration.
- **No bilingual duplicates**: do not follow an English body with a Chinese recap (or vice versa).
- Final answers should be readable for the user; leave tool mechanics in tool results—no wrong-language process narration.
